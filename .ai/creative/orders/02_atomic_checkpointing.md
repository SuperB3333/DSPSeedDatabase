# Order 02 — Atomic Checkpointing + Correct Resume

## Target Files
- `src/misc.rs` (checkpoint read/write functions)
- `src/main.rs` (resume logic, checkpoint loop, worker launch)

## Safety & Equivalence Zone
- FROZEN: `src/algorithm/**`, `src/generate_csv.rs`. `worker_thread` per-seed body (`gen_formatted` → `send`) must remain byte-identical; only the `Range<i32>` fed to each worker may change.
- Invariant A (no skips): every seed in `[START_SEED, END_SEED)` is committed exactly once across all runs.
- Invariant B (no duplicates): resume must never re-COPY a seed already in the DB (stars.id PK would abort whole batches).
- Verified bug being fixed: current resume is a permanent no-op — all 32 `PROGRESS_WORKERS` slots are written as `progress - max_buffer`; unused slots yield `-max_buffer`, `.min()` picks them, offset clamps to 0. Also a naive global `start_seed` shift would violate Invariant B because chunks are contiguous and independent.

## Implementation Plan
1. **Checkpoint format v2** (atomic write already exists — keep tmp+fsync+rename in `write_checkpoint_atomic`, change payload):
   ```
   v2 <start_seed> <end_seed> <worker_count>
   <chunk_start> <chunk_end> <watermark>     # one line per worker, only worker_count lines
   ```
   `watermark` = highest seed+1 in that chunk guaranteed committed.
2. **Watermark computation** (monitor loop, main.rs): for worker i with chunk `cs..ce`:
   `watermark = (cs + generated_i - max_buffer).clamp(cs, ce)` where `max_buffer = channel_size + commit_count * writer_count`. Conservative by construction: in-flight seeds ≤ max_buffer, so everything below watermark is committed.
3. **Clean-completion checkpoint:** after ALL commit threads join (everything committed), write checkpoint with `watermark = chunk_end` for every worker.
4. **Resume (startup):**
   - Parse v2 file. If header `(start,end,workers)` ≠ current env config → `log_error!` and exit(1) with message "checkpoint mismatch; delete <file> or restore config" (never guess). Missing/unparseable file → fresh start (`log_debug!`).
   - If all watermarks == chunk_end → `log_info!("all work done")`, return.
   - Per-worker resume range: `watermark_i .. chunk_end_i` (do NOT re-split; reuse recorded chunks).
   - **Duplicate purge (Invariant B):** before spawning threads, open one PG connection and for each worker delete the conservatively-rewound window:
     ```sql
     DELETE FROM planets WHERE star_id >= w*100 AND star_id < ce*100;
     DELETE FROM stars   WHERE id      >= w*100 AND id      < ce*100;
     ```
     (`w` = watermark, `ce` = chunk_end; star_id = seed*100 + index, stride 100, index<64 — range math is exact.) Skip entirely in fresh-start path.
5. **Cadence:** move checkpoint write out of the 100ms UI tick — write every ~5s (tick counter `% 50`).
6. Fresh start still uses `split_chunks(start..end, worker_count)` unchanged, then records those chunks in every checkpoint write.

## Validation Criteria
```powershell
cargo check; cargo build --release
```
- Run 1: seeds 0..2000, kill process (`Stop-Process`) mid-run. Inspect checkpoint: v2 header + N worker lines, watermarks within chunk bounds.
- Run 2 (same env): resumes; completes. Then:
  `SELECT seed, COUNT(*) FROM stars GROUP BY seed HAVING COUNT(*) <> 64;` → 0 rows.
  `SELECT COUNT(DISTINCT seed) FROM stars;` → 2000 (no skips), `SELECT COUNT(*) FROM stars;` → 128000 (no dups).
- Run 3 (immediately again): exits with "all work done", DB counts unchanged.
- Mismatch test: change WORKER_THREADS, rerun → clean error exit, no DB writes.
- Kill during write: no truncated checkpoint ever observed (`.tmp` may remain; ignored by reader).
