# Order 04 — Benchmark Mode (pure-generation throughput)

## Target Files
- `src/main.rs` (only file modified)

## Safety & Equivalence Zone
- FROZEN: `src/algorithm/**`, `src/generate_csv.rs`, `src/misc.rs`, `commit_thread` body, `worker_thread` body. Workers must run the exact same code path — benchmark isolates CPU by swapping the channel *consumer*, never the producer.
- Invariant: with `BENCHMARK` unset/`0`, the binary's behavior is byte-for-byte identical to today (no reordering of connects, checkpoint reads, or thread spawns in the normal path).
- Benchmark mode must not read, write, or delete: Postgres (no `Client::connect`, no Order-02 duplicate purge) and the checkpoint file (no resume read, no periodic write) — a bench run must never corrupt real-run state.

## Implementation Plan
1. **Flag:** `let benchmark = env_str!("BENCHMARK", "0") == "1";` next to the other config reads. `log_info!("benchmark mode: DB and checkpointing disabled")` when set.
2. **Skip state I/O:** wrap the checkpoint-resume block and the in-loop checkpoint write (Order 02 versions if already applied, else current ones) in `if !benchmark { ... }`. Skip `get_db_str()` usage (build `conf` only in normal mode).
3. **Sink thread:** add alongside `commit_thread`:
   ```rust
   static BENCH_BYTES: AtomicU64 = AtomicU64::new(0);
   fn bench_sink_thread(rec: Receiver<(String, String)>) {
       while let Ok((s, p)) = rec.recv() {
           BENCH_BYTES.fetch_add((s.len() + p.len()) as u64, Relaxed);
           COMMITTED_SEEDS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
       }
   }
   ```
   Plain `recv()` (no timeout): thread exits on `Disconnected` after main drops `entry_sender` — same shutdown path as writers. `COMMITTED_SEEDS` increments keep the TUI/progress (Order 03) working unmodified.
4. **Spawn switch:** in the writer-spawn loop, `if benchmark { spawn bench_sink_thread(rx) } else { spawn commit_thread(rx, conf) }`. Keep `writer_count` sinks (harmless; keeps loop structure identical).
5. **Report:** in the final `log_info!("done: ...")`, when benchmark, append MB generated and MB/s from `BENCH_BYTES`. Label the line `benchmark:` instead of `done:` so results are greppable.
6. Imports: `std::sync::atomic::AtomicU64`. No new crates.

## Validation Criteria
```powershell
cargo check; cargo build --release
$env:BENCHMARK="1"; $env:END_SEED="2000"; .\target\release\dsp_seed_finder.exe 2>bench.log
```
- Runs to completion with NO Postgres reachable; `bench.log` shows config, progress, and final `benchmark: ... seeds/sec ... MB/s`.
- `checkpoints.txt` (or `CHECKPOINT_FILE`) is not created/modified: compare `LastWriteTime` before/after.
- Determinism probe: two consecutive bench runs report identical total bytes (generation is deterministic; byte count acts as a cheap output fingerprint).
- Normal-mode regression: unset `BENCHMARK`, run seeds 0..100 against local PG → rows land, checkpoint written, behavior unchanged.
