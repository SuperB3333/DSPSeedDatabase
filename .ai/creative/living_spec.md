# Living Spec — Architect State File

**Role:** Lead Systems Architect. No app-code edits by architect; all changes delegated via `.ai/creative/orders/`.
**Last update:** 2026-07-03 — ALL 6 orders generated. Architecture phase complete; awaiting execution (by subagents or user) + validation reports.

## Baseline Project State (verified against live code)

- **Pipeline:** Rust binary `dsp_seed_finder` (src/main.rs) spawns N worker threads calling
  `gen_formatted(seed, 64, 1.0)` (src/generate_csv.rs) → `(stars_csv, planets_csv)` strings →
  crossbeam bounded channel → M commit threads bulk-`COPY` into Postgres in one txn per batch.
- **Generation core:** `src/algorithm/**` (worldgen + data). Deterministic port of game RNG.
  UNTOUCHABLE for equivalence (zero-tolerance policy applies).
- **CSV contract:** `gen_formatted` emits stars (22 cols) + planets (56 cols); column order fixed by
  `COPY_STAR` / `COPY_PLANET` in src/misc.rs and schema in create_database.py.
- **star_id = star.index + seed*100** (stride 100, hardcoded in generate_csv.rs:18).
- **Config:** env vars START_SEED, END_SEED, WORKER_THREADS(<32), WRITER_THREADS, COMMIT_COUNT,
  CHANNEL_SIZE, CHECKPOINT_FILE, LOG_LEVEL, PG_*. Parsed via macros env_int!/env_str!.
- **Logging:** src/logging.rs already exists — LOG_LEVEL env, stderr output, atomic level check,
  macros log_error!/warn!/info!/debug!. Goal 3 is mostly built; order covers the remaining gap
  (TUI vs non-TTY, progress-to-stderr fallback).
- **Checkpointing:** `write_checkpoint_atomic` (src/misc.rs:31) already does tmp+fsync+rename.
  **BUG (verified by trace):** resume is a permanent no-op — checkpoint writes all 32
  PROGRESS_WORKERS slots as `progress - max_buffer`; unused slots (id >= worker_count) write
  `-max_buffer`; resume takes `.min()` → always `-max_buffer` → offset `(min+max_buffer).max(0)=0`.
  Even if fixed naively, advancing global start_seed and re-splitting chunks causes duplicate PK
  violations (workers own contiguous independent ranges). Order 02 redesigns resume.
- **TUI:** metrics.rs writes progress bars to stdout via crossterm alt-screen; raw mode toggled
  every 100ms tick; percent underflow panic risk at >100% (`100 - percent` repeat).
- **Metrics loop:** main.rs:171-188 — sps hardcoded -1.0; checkpoint written every 100ms.
- **DB schema:** stars (id PK + UNIQUE dup, FLOAT8 cols holding f32 data), planets (no PK, 42 INT
  vein cols min/max/estimate ×14 ores, UNIQUE(star_id,index) present in create_database.py:68).
  Indexes created BEFORE load. Gas giants emit 42×`-1` sentinels.
- **Prior deep review exists:** `.ai/generation-review-2026-07-02.md` — 40+ findings with owner
  decisions (§9). Orders below cross-reference it; equivalence-relevant items re-verified.
- **Scale target:** 10M seeds → 640M stars, 2.24B planets, ~3TB index-dominated.
- **Query layer:** server/server.py (WebSocket) + parse_rule.py/rules.py build SQL against
  stars/planets. Rules use `-1` sentinels implicitly? — verified: rules compare with numeric
  operators; sentinel→NULL migration is flagged OPTIONAL/high-risk in Order 01.
- **No test suite. CI builds Windows debug (false signal).**

## Order Index (all in .ai/creative/orders/)

| # | File | Status | Risk |
|---|------|--------|------|
| 1 | 01_storage_reduction.md | GENERATED | Low (schema-only; CSV untouched) |
| 2 | 02_atomic_checkpointing.md | GENERATED | Medium (resume logic; gen untouched) |
| 3 | 03_console_logging.md | GENERATED | Low (module exists; gap-fill only) |
| 4 | 04_benchmark_mode.md | GENERATED | Low (additive flag) |
| 5 | 05_seed_scoring.md | GENERATED | Low (new read-only script) |
| 6 | 06_documentation.md | GENERATED | None (docs only) |

## Equivalence Rulings (architect sign-off)

- `src/algorithm/**` — NOT modified by any order. All orders declare it a frozen zone.
- `gen_formatted` CSV byte-output — frozen in every order. Orders 02/03/04 touch only main.rs
  orchestration around it; Order 01 touches only Postgres DDL (CSV parses identically into
  narrower column types: all emitted values verified in-range — star_index<64<SMALLINT-max,
  type 1..5, spectr -4..3, planet index/orbiting/satellites/theme_id/water_item all «32767,
  floats are f32-sourced → REAL lossless; vein 42 cols stay INT, max ~9.4e8 > SMALLINT).
- Seed-generation algorithm refactors: NONE authorized. P1 (name-gen removal) and P2/P3 from the
  prior review were considered and EXCLUDED from these orders — P1/P2 touch create_galaxy /
  star_planets internals; equivalence is provable but they are not required by the 6 goals, so
  per zero-tolerance policy they stay out of scope.

## Pending / Follow-ups

### Execution validation records (orchestrator)

- **Order 01 — DONE.** Files changed: `create_database.py` only (target match confirmed via
  `git diff --stat`). Validation: `ast.parse` syntax check PASS. All 78 COPY column names
  (22 star + 56 planet from `src/misc.rs`) verified present in new schema. Frozen zone
  (`src/**`) untouched. Schema matches plan: id PK (redundant UNIQUE dropped), REAL/SMALLINT
  narrowing, UNLOGGED tables, `create_indexes()` + `--indexes` argparse flag, dead
  `dist_cols`/`#TODO`/`c()` removed. SKIPPED (no local Postgres / psycopg2 not installed):
  `python create_database.py`, `--indexes` run, `\d` type inspection, pg_indexes count,
  Rust end-to-end seed run. Deviations: none.

- Order 04 DONE: `BENCHMARK=1` env flag; sink-thread swap at spawn time (worker/commit bodies
  frozen); skips checkpoint read+write and all PG access; BENCH_BYTES counter doubles as a
  determinism fingerprint; final `benchmark:` log line with seeds/sec + MB/s. main.rs only.
- Order 05 (seed scoring): standalone Python script (e.g. `score_seeds.py`) using psycopg2, reads
  stars/planets, computes configurable score (SQL aggregate), ranks seeds. Read-only; reuse
  parse_rule/rules if convenient but do not modify them.
- Order 06 (documentation): README expansion — build, env-var table, 3-step compose workflow
  (postgres compose → create_database.py → worker compose), checkpoint/resume semantics (v2 format
  from Order 02), benchmark + scoring usage, troubleshooting (PK violations, checkpoint mismatch,
  TUI garbage in docker logs → NO_TUI).
- After each order's execution: record validation results + drift here.
- Turn protocol (budget): write exactly one order per session, update this file, stop.
