# Order 06 — Project Documentation

## Target Files
- `README.md` (currently one line — expand)

## Safety & Equivalence Zone
- Docs only. No code, config, or compose files may be modified.
- Every documented option MUST exist in code — verify each env var against `src/main.rs`/`src/misc.rs`/`src/logging.rs` before writing it. Do not document features from orders 02–05 unless their code is actually present in the working tree at execution time (check first; omit missing ones).

## Implementation Plan
Write README.md with these sections (concise, table-driven, no marketing prose):
1. **Overview** — Rust worker generates DSP galaxy data per seed and bulk-COPYs into Postgres; Python handles schema + WebSocket query server. One paragraph.
2. **Build** — `cargo build --release`; Docker image via `Dockerfile`.
3. **Configuration** — table of env vars with defaults and constraints (from code, not memory): `START_SEED` 0, `END_SEED` 10000, `WORKER_THREADS` 8 (max 31), `WRITER_THREADS` 4, `COMMIT_COUNT` 1000, `CHANNEL_SIZE` 1000, `CHECKPOINT_FILE` checkpoints.txt, `LOG_LEVEL` info (error|warn|info|debug), `PG_USER/PG_PASS/PG_NETLOC/PG_PORT/PG_DBNAME` (postgres/rootpassword/localhost/5432/dsp). Add `NO_TUI`/`BENCHMARK` only if present in code.
4. **Database setup & run order** — the confirmed 3-step sequence:
   1) `docker compose -f compose/compose.postgres.yaml up -d`
   2) `python create_database.py` (note `--indexes` post-load step if Order 01 landed)
   3) `docker compose -f compose/compose.yaml up`
   Plus schema summary: `stars` (1 row/star/seed, `id = seed*100 + star_index`), `planets` (FK `star_id`, 14 ores × min/max/estimate, `-1` sentinels for absent veins/gas giants).
5. **Checkpointing & resume** — describe the actual implemented behavior (read the code first): atomic tmp+fsync+rename writes; v2 per-worker watermark format and resume/purge semantics if Order 02 landed, otherwise current conservative-offset behavior. State plainly which config changes invalidate a checkpoint.
6. **Benchmark mode** — `BENCHMARK=1` usage + what it skips (only if Order 04 landed).
7. **Seed scoring** — `score_seeds.py` usage examples: `--weight ore_oil=2 --top 10` (only if Order 05 landed).
8. **Query server** — one short paragraph: `server/server.py`, port 62879, message types Find/Generate/Stop.
9. **Troubleshooting** — bullet list:
   - `stars.id` PK violation → overlapping seed range re-run; fix START_SEED/checkpoint or purge range.
   - Worker exits instantly with "all work done" → stale checkpoint file; delete it.
   - Garbage/escape codes in `docker logs` → TUI in non-TTY; set `NO_TUI=1` (if implemented).
   - Writer panic "channel stall detected" → generation slower than 1s lull tolerance or DB down mid-run.
   - Container restart loop re-inserting seeds → set `restart: "no"` in compose.
10. Link `.ai/generation-review-2026-07-02.md` for the full known-issues register.

## Validation Criteria
- Every env var in the README exists in `src/` (grep each name; zero misses).
- Every command runs as written (`cargo build --release` compiles; python steps syntax-check).
- No documented flag/feature lacks a corresponding code path in the current tree.
- README renders without broken markdown (`# ` headers, closed tables/code fences).
