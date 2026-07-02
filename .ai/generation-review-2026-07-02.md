# DSPSeedDatabase — Generation Pipeline Code Review

**Date:** 2026-07-02
**Scope:** Generation phase only (`src/`, `create_database.py`, `Dockerfile`, `compose/`). Query layer out of scope except for schema-drift risk.
**Status of clarifying questions:** All resolved by owner — decisions incorporated below (see §9).
**Target scale (confirmed):** Full 10M seeds → 640M star rows, 2.24B planet rows, ~3 TB with indexes.

---

## 1. Executive Summary

| # | Impact | Issue |
|---|--------|-------|
| 1 | **Critical** | Stars and their planets are committed on two independent Postgres connections/transactions — a crash between `scpy.finish()` and `pcpy.finish()` orphans star rows without planets (or vice versa), silently corrupting the dataset. |
| 2 | **Critical** | `commit_thread` breaks on `recv_timeout` error (Timeout *and* Disconnected combined in one `Err(_)` arm) — any >1s lull permanently kills a writer; if all writers die while seeds remain in the channel, workers block forever in `send()` and the process deadlocks silently. |
| 3 | **Critical** | `FROM scratch` with a dynamically-linked (glibc) binary: the container won't start — needs a musl static build or a non-scratch base image. |
| 4 | **Critical** | `restart: unless-stopped` on a finite batch job that exits on completion: the container restart-loops indefinitely, re-inserting duplicate seeds; the duplicate `stars.id` PK violation then aborts every COPY batch. |
| 5 | **Critical** | No automated crash recovery: the checkpoint file is written 10×/sec but **never read** on startup; a crash requires manual `START_SEED` adjustment, and since worker chunks are contiguous and independent, a single `START_SEED` cannot correctly resume all workers anyway. |

**Pre-generation blockers (decided, must be implemented before the 10M run):**
- Fix split-transaction atomicity (L1).
- Implement `satellite_count` + `orbiting` columns (L6) — retrofitting requires a full regeneration.
- Make `STAR_COUNT` env-configurable and derive/validate the `star_id` stride (L7).
- Create indexes after load, not before (S1).

---

## 2. Logic Errors

### L1 — [Critical] Stars/Planets committed in separate transactions
**File:** `src/main.rs:46-67`
`commit_thread` opens two separate `Client` connections — `star_client` and `planet_client` — with independent `copy_in..finish` cycles. A crash, panic, or DB error between `scpy.finish()` and `pcpy.finish()` commits stars with no matching planets (or vice versa), with no FK to catch it and no retry.
**Fix:** Single `Client`, both COPY streams inside one explicit transaction (`client.transaction()`), commit once.

### L2 — [Critical] `recv_timeout` Err arm conflates Timeout and Disconnected
**File:** `src/main.rs:51-54`
`Err(_) => break` treats a momentary 1s lull the same as channel closure. Any stall >1s permanently kills a writer; if all 4 die mid-run, the bounded channel fills, workers block in `send().unwrap()` forever → silent infinite hang (monitor loop never sees workers finish).
**Load-bearing quirk:** the timeout is currently the *actual* shutdown mechanism — `main` holds its `entry_sender` alive through the monitor loop, so writers never see `Disconnected` naturally.
**Fix (two parts):**
1. `Err(RecvTimeoutError::Disconnected) => break, Err(RecvTimeoutError::Timeout) => continue`.
2. Keep `drop(entry_sender)` before writer joins (current position works); preferably move it right after spawning to remove the fragile ordering coupling.

### L3 — [Critical] `restart: unless-stopped` on a finite job
**File:** `compose/compose.yaml:13`
Worker exits on completion → Docker relaunches → re-COPY of same range → `stars.id` PK violation → crash → restart loop.
**Fix:** `restart: "no"` (or `on-failure` with a cap).

### L4 — [High] No resume-from-checkpoint; checkpoint is write-only
**File:** `src/main.rs:116-136`
Checkpoint is written every 100ms but never read at startup; resume is manual via `START_SEED`. Worker chunks are contiguous and independent, so one global `START_SEED` can only resume from the slowest worker → re-generates committed seeds → PK violations.
**Fix:** Read checkpoint at startup for per-worker offsets, or derive the durable watermark from the DB (`SELECT MAX(seed) FROM stars` per chunk range) — the DB value is the only *guaranteed*-committed watermark.

### L5 — [High] Final partial batch never counted in `COMMITTED_SEEDS`
**File:** `src/main.rs:57-58, 66-67`
`fetch_add` only fires in the `i % commit_count == 0` branch; the post-loop flush commits up to `commit_count-1` seeds per writer without counting them (up to ~4k undercount with defaults).
**Fix:** After final `finish()`, `COMMITTED_SEEDS.fetch_add(i % config.1, SeqCst)`.

### L6 — [High] [KEEP & IMPLEMENT — owner decision] `satellite_count` hardcoded -1; `orbiting` column never written
**File:** `src/generate_csv.rs:36-38`; `create_database.py:46`; `src/misc.rs:4`
- `satellite_count = -1 //todo implement` for every gas giant. The real count is derivable at CSV-write time: count planets whose `orbit_around` (`planet.rs:20`, `has_orbit_around()` `planet.rs:150`) points at that gas giant. `star_planets.rs:323-387` already tracks this during generation.
- `orbiting INT` (comment: "Index of gas giant, -1 if orbit around sun") exists in the schema but is absent from `COPY_PLANET` → NULL forever. It mirrors `planet.orbit_around` exactly.
**Owner decision:** Both fields are required. Must be implemented **before** the 10M run — retrofitting requires full regeneration.
**Implementation plan (documented, not yet implemented):**
1. In `gen_formatted`, after `get_planets()`, build a per-star map: gas-giant index → moon count (single pass over planets checking `orbit_around`).
2. Emit real `satellite_count` for gas giants (0 if no moons), keep 0 for non-gas planets.
3. Add `orbiting` to the planets CSV row: the orbited gas giant's planet index, or -1 when orbiting the star (source: `planet.orbit_around` → its `.index`).
4. Add `orbiting` to `COPY_PLANET` in `src/misc.rs` (column already exists in schema; keep spelling in sync).
5. Keep `create_database.py:46` as-is; consider `SMALLINT` (see S-section).

### L7 — [High — escalated, owner decision] `STAR_COUNT` must become configurable; `star_id` stride is a fragile magic number
**File:** `src/main.rs:27` (`const STAR_COUNT: usize = 64`); `src/generate_csv.rs:18` (`seed * 100`)
`star_id = star.index + seed * 100`. Stride 100 is hardcoded, undocumented, and not linked to `STAR_COUNT`. Safe today (64 ≤ 100; max id 999,999,963 < i32::MAX), but the moment a configurable star count exceeds 100, ids collide across seeds and silently corrupt the PK.
**Owner decision:** `STAR_COUNT` shall be dynamic/configurable (env var like the rest of the config).
**Implementation plan (documented, not yet implemented):**
1. Add `STAR_COUNT` env var (`env_int!("STAR_COUNT", 64)`), pass through to `gen_formatted` (already takes `star_count` as a parameter — only `main.rs` needs the change).
2. Replace magic `100` with a named `STAR_ID_STRIDE` constant; runtime-validate `star_count <= STAR_ID_STRIDE` with a clear error at startup (compile-time assert no longer possible once dynamic).
3. Overflow guard: `max_seed * STRIDE + star_count` must stay < `i32::MAX` → with stride 100 the seed cap is ~21.4M; validate `END_SEED` against this.
4. Also update capacity hints in `generate_csv.rs:13-14` (already parameterized) and `REC_MULTIPLIER` handling if it ever becomes configurable too.
5. Preferable long-term alternative: composite key `(seed, star_index)` — see Idea I1. If star_count may ever exceed 100, adopt the composite key **before** generation instead of enlarging the stride.
6. Note: `assert!(worker_count < 32)` in `main.rs:84` and the fixed `PROGRESS_WORKERS: [AtomicI32; 32]` are related hardcoded limits; document or lift together.
7. DB note: `stars.star_index` range grows with configurable star_count; still fits SMALLINT (≤32767) for any sane value.

### L8 — [Medium] Planets table has no unique constraint; duplicate rows on any restart
**File:** `create_database.py:42-64`
No PK / `UNIQUE(star_id, index)` on `planets`. Any overlapping re-run silently duplicates planet rows (stars are protected by their PK; planets are not).
**Fix:** Add `UNIQUE (star_id, index)` (or composite PK), pair with a staging-table + `INSERT ... ON CONFLICT DO NOTHING` load path for idempotent resume.

### L9 — [Medium] `create_database.py` is a SyntaxError on Python < 3.12
**File:** `create_database.py:37, 38, 60-62`
Backslashes (`'\n'.join(...)`) inside f-string `{...}` expressions require PEP 701 (Python 3.12+). No `pyproject.toml`/`.python-version` declares this.
**Fix:** Declare `requires-python = ">=3.12"` or hoist the joins into local variables before the f-string.

### L10 — [Medium/latent] Small-`star_count` arithmetic panics in `generate_stars`
**File:** `src/algorithm/worldgen/galaxy_gen.rs:143-146, 170`
`num12 = (num11 - 1) / num8` can be 0 for small star counts → `index % num12` divide-by-zero panic; `num9/num10/num11` are unchecked `usize` subtractions that underflow for tiny counts. Unreachable at 64, **but becomes live once STAR_COUNT is configurable (L7)** — add guards as part of that work. Also `star.rs:93` divides by `star_count - 1` → NaN at star_count=1.
**Fix:** Validate a sane minimum (e.g. `star_count >= 8`) at config parse time; add saturating/guarded math.

### L11 — [Low] `DspRandom` plain subtraction panics in debug builds
**File:** `src/algorithm/data/random.rs:16, 48` (line 24 already uses `wrapping_sub`)
Debug builds panic on overflow where release wraps (matching .NET). The shipped release binary is correct; debug/test builds are broken for many seeds, and CI builds debug (see D8).
**Fix:** `wrapping_sub` at lines 16 and 48; `seed.wrapping_abs()` at line 10 (i32::MIN edge).

### Verified non-issues (checked, no action)
- `ORES[1..15]` ordering exactly matches `COPY_STAR`/`COPY_PLANET`/`misc.py` column order — no swapped ore columns.
- `SpectrType` transmute (`star.rs:283`) is safe: class factor clamped to [-4.0, 2.0], all discriminants valid. Star type +1 mapping matches `misc.py` exactly.
- `VeinType` transmute (`planet.rs:590`): loop range 1..=14 maps to valid discriminants only.
- Gas id panic in `generate_csv.rs:50` can never fire: all `gas_items` in `THEME_PROTOS` are ⊆ {1120, 1121, 1011}.
- `lazy_getter` early-returns still cache (closure semantics) — no recompute bug.
- `get_theme()` returns cached `&'static ThemeProto` — no per-planet deep clone.
- Decimation loop in `generate_temp_poses` is O(n) (reverse-order removes), not O(n²) as it first appears.
- `Vein::min()/max()` i32 products peak ~9.4e8 at multiplier 1.0 — no overflow (would overflow at multiplier ≳2.3; keep INT columns, not SMALLINT).
- COPY column lists match schema exactly (22/22 stars, 55/56 planets — the missing one is `orbiting`, see L6).

---

## 3. Performance & Optimization

### P1 — [High] Name generation runs per-seed but is never stored
**File:** `src/algorithm/worldgen/galaxy_gen.rs:196-202`; `name_gen.rs`
`random_name` (up to 256 attempts, fresh String allocations, O(n) dedup scan) runs for all 64 stars per seed; `generate_csv.rs` has no name column. 640M wasted calls at 10M seeds.
**Fix:** Delete the `random_name` call + `names` vec from `create_galaxy` (or feature-gate). Note: `name` field is only consumed by the serde Serialize path, which is dead code (P5).

### P2 — [Medium] `get_avg_vein` makes 14 separate cold passes per star
**File:** `src/algorithm/data/star_planets.rs:60-93`
Each vein type's first call does a full planet×vein scan (results memoized after). One combined pass building all 14 sums would be ~14× cheaper cold.
**Fix:** Single-pass `precompute_avg_veins()` populating the whole map.

### P3 — [Medium] `push_str(format!(...).as_str())` allocates a temp String per CSV row segment
**File:** `src/generate_csv.rs:20-96`
~Billions of short-lived allocations at 10M-seed scale.
**Fix:** `use std::fmt::Write; write!(stars, "...")` formats directly into the existing buffer.

### P4 — [Medium] ~350 MB worst-case in-flight buffering with defaults
Channel (1000 × ~70 KB ≈ 70 MB) + 4 writers × 1000 msgs COPY buffering (≈280 MB). No container mem_limit.
**Fix:** `COMMIT_COUNT=100` drops writer buffering to ~28 MB with negligible throughput cost; or bound by bytes.

### P5 — [Low] serde is confirmed dead code — safe to remove entirely (owner-confirmed)
**File:** `Cargo.toml:17`; Serialize impls across `src/algorithm/data/*`
Owner confirms: no JSON dependency exists, no internal serialization calls; the pipeline hand-builds CSV. All `Serialize`/`Deserialize` derives and manual impls (`star.rs:391`, `planet.rs:641`, `star_planets.rs:401`, plus the `avg_vein_amounts` HashMap field kept "for serde") can be removed with zero production impact.
**Fix:** Remove the serde dependency + derives + manual impls + `star_planets.rs:19` serde-only field. Cuts compile time and removes the `rc` feature entirely.

### P6 — [Low] Missing capacity hints for `tmp_poses`/`tmp_drunk`
**File:** `galaxy_gen.rs:20, 53` — max size known (`target_count * actual_iter_count`); use `Vec::with_capacity`.

### P7 — [Nit] Release profile: add `codegen-units = 1`, `panic = "abort"`
**File:** `Cargo.toml:6-8` — small perf/binary-size wins for a single-binary image.

---

## 4. Robustness & Edge Cases

### R1 — [Critical] Dockerfile produces a non-starting container
**File:** `Dockerfile:1, 6-7`
glibc-linked binary from `rust:latest` copied into `FROM scratch` → no `ld.so`/glibc → exec fails.
**Fix:** Build `--target x86_64-unknown-linux-musl` (static) for scratch, or use `debian:bookworm-slim`/distroless-cc runtime. (`sslmode=disable` means no CA-cert requirement.)

### R2 — [Critical] No SIGTERM handling; `docker stop` aborts in-flight COPY
**File:** `Dockerfile:8`; compose
Binary is PID 1 with no handler → SIGTERM ignored → SIGKILL after 10s → current batches lost.
**Fix:** `init: true` in compose; a shared `AtomicBool` shutdown flag checked per seed in workers, drain + flush in writers; raise `stop_grace_period`.

### R3 — [High] Writer panics cascade to deadlock rather than clean failure
**File:** `src/main.rs:55-62`
`.unwrap()/.expect()` on DB calls panics a writer; remaining receivers keep the channel alive; workers can block on a full channel; main joins writers only after workers finish → hang.
**Fix:** `commit_thread` returns `Result`; on failure, signal shutdown (drop remaining receivers / set flag) so workers unwind.

### R4 — [High] Checkpoint file opened without `.truncate(true)` — stale-byte corruption
**File:** `src/main.rs:118-121`
Shorter write over longer content leaves trailing garbage ("999" over "1000" → "9990"-style values).
**Fix:** Add `.truncate(true)` (one line).

### R5 — [Medium] Checkpoint value `progress - max_buffer` is negative early in the run
**File:** `src/main.rs:124-127` — garbage offsets until 5000 seeds done. **Fix:** clamp with `.max(0)`.
Also note: `PROGRESS_WORKERS` counts *generated*, not *committed* — `max_buffer` is a heuristic, not a guarantee; the DB watermark (L4 fix) is the reliable source.

### R6 — [Medium] Progress bar `usize` underflow panics at ~100%
**File:** `src/metrics.rs:20-25`
`(committed + queue)` can exceed `goal` → `"░".repeat(100 - cpu_percent)` underflow-panics at the very end of a successful run.
**Fix:** `.min(100)` on both percent values (one line each).

### R7 — [Low] One bad seed kills a worker thread
**File:** `src/main.rs:36` — `gen_formatted(...).expect(...)`. At 10M seeds, rare edge cases become near-certain.
**Fix:** log + skip failed seeds; count failures in a metric.

### R8 — [Low] `elapsed.as_secs()` integer math → Inf seeds/sec for sub-second runs
**File:** `src/main.rs:151` — **Fix:** `as_secs_f64()`.

---

## 5. Docker / Environment

### D1/D2 — [Critical] See R1 (scratch image) and L3 (restart policy).

### D3 — [High] No `.dockerignore`
**File:** `Dockerfile:3` — `COPY . .` ships `target/`, `.git/`, `data/`, `flamegraph.svg`, Python files into every build context.
**Fix:** Add `.dockerignore`: `target/`, `.git/`, `data/`, `*.svg`, `*.py`, `*.md`, `compose/`, `archive/`, `.ai/`.

### D4 — [High] No cargo dependency-cache layer
**File:** `Dockerfile:3-4` — any source edit rebuilds all deps.
**Fix:** cargo-chef, or copy `Cargo.toml`/`Cargo.lock` + dummy `main.rs` build first, then copy `src/`.

### D5 — [Low — reclassified per owner workflow] Two-compose split is intentional
Owner runs `compose.postgres.yaml` (postgres only, port 5432 published) first, then `create_database.py` from the host, then `compose.yaml` (worker via `host.docker.internal`). This works as designed.
Remaining notes:
- Ordering is manual and undocumented → document the 3-step sequence in README (see DX10); a `depends_on` can't span separate compose projects.
- The stale `memory.md` gotcha claiming `compose.postgres.yaml` mis-sets `PG_NETLOC` is inaccurate (that file has no worker service) — correct the memory entry.
- Optional hardening: a small wait-for-postgres retry loop in the Rust binary at startup (currently `Client::connect(...).unwrap()` in each writer dies instantly if postgres isn't up yet — with `restart: no` per L3, a too-early start would need a manual relaunch).

### D6 — [Medium] No resource limits / healthcheck on either service
**Fix:** `deploy.resources.limits` on the worker; `pg_isready` healthcheck on postgres (useful even standalone, for the manual ordering).

### D7 — [Medium] crossterm TUI in a detached container
**File:** `src/main.rs:112-114`, `metrics.rs` — raw mode + alternate screen against a pipe → escape-code garbage in `docker logs`.
**Fix:** Gate on `stdout().is_terminal()` or `NO_TUI=1` env; fall back to one plain progress line per interval.

### D8 — [Low] CI builds Windows debug; ships Linux release
**File:** `.github/workflows/rust.yml` — `windows-latest`, `cargo build` (debug, no tests/clippy). Debug builds panic on the `random.rs` wrapping paths (L11), so CI is a false signal and never builds the shipped artifact.
**Fix:** `ubuntu-latest`, `cargo build --release --target x86_64-unknown-linux-musl`, add `cargo clippy`; add a determinism smoke test (see I2).

### D9 — [Nit] `version: "1"` in compose.yaml is obsolete — remove the key.

---

## 6. Developer Experience / Usability

### DX1 — [High] seeds/sec is hardcoded `-1.0` — dead metric
**File:** `src/main.rs:134` — **Fix (one line):** pass `COMMITTED_SEEDS.load(Relaxed) as f32 / start.elapsed().as_secs_f32()`.

### DX2/DX3 — See R4 (truncate) and L4 (resume). Highest-value DX+robustness pair.

### DX4 — [Medium] Checkpoint rewritten 10×/sec, coupled to the UI tick
**File:** `src/main.rs:117-128` — **Fix:** separate ~5s cadence (or every N seeds).

### DX5 — [Medium] `assert!` config validation has no context
**File:** `src/main.rs:82-84` — **Fix:** add messages with the offending values. When STAR_COUNT becomes configurable (L7), extend validation: `star_count <= STAR_ID_STRIDE`, `star_count >= 8`, seed-cap check.

### DX6 — [Medium] Blank screen during writer drain after workers finish
**File:** `src/main.rs:136-144` — TUI torn down while writers may flush for minutes.
**Fix:** post-worker "flushing N remaining" phase using `entry_reciever.len()`.

### DX7 — [Medium] No `DRY_RUN` mode
**Fix:** `DRY_RUN=1` → skip DB connect/COPY, drain channel to a byte counter. Enables generation benchmarking and CI smoke tests without Postgres.

### DX8 — [Low] `.gitignore:14` malformed: `flamegraph.svg### Rust template` (missing newline) — pattern matches nothing; the 893 KB SVG is tracked. **Fix:** newline; `git rm --cached flamegraph.svg`.

### DX9 — [Low] `checkpoints.txt` and `data/` missing from `.gitignore`.

### DX10 — [Low] README is one line
Document: build, env vars (incl. new `STAR_COUNT`), and the confirmed 3-step run order — (1) `docker compose -f compose/compose.postgres.yaml up -d`, (2) `python create_database.py`, (3) `docker compose -f compose/compose.yaml up`.

### DX11 — [Nit] Duplicate agent memory: both `./memory.md` and `.ai/memory.md` exist with diverging content; `.github/copilot-instructions.md` points at `.ai/memory.md`, `.ai/agent-instructions.md` at `memory.md`. Consolidate to one.

---

## 7. Storage & DB Size Notes (10M-seed scale confirmed)

Scale basis: 640M star rows, 2.24B planet rows. Raw table data ≈ 620–700 GB; the 3 TB projection is therefore index-dominated → index strategy is the biggest lever.

### S1 — [Critical] Indexes created before bulk COPY
**File:** `create_database.py:66-83`
Per-row index maintenance + WAL during a 2.9B-row load: 5–20× slower and bloated indexes.
**Fix:** Create tables bare → COPY everything → `CREATE INDEX` afterwards (optionally with raised `maintenance_work_mem` and parallel workers). Single highest-ROI change.

### S2 — [High] `idx_stars_id` duplicates the PK index
**File:** `create_database.py:71` — drop `index("stars", "id")`. Also `id INT UNIQUE PRIMARY KEY` (line 27) creates a *second* redundant unique index → use plain `PRIMARY KEY` (S6).

### S3 — [High] FLOAT8 → REAL for all f32-sourced columns
**File:** `create_database.py:30, 32, 49, 53, 56-58`
`start_dist`, `luminosity`, `sun_distance`, `temperature`, `gas_h/d/i` are `FLOAT` (8B) holding f32 data.
**Impact at 10M seeds:** ~4B saved × (2 cols × 640M + 5 cols × 2.24B) ≈ **50 GB** table data, plus smaller indexes on `luminosity`/`temperature`.

### S4 — [High] Load into UNLOGGED tables
WAL for 2.9B rows provides no value for regenerable data. `CREATE UNLOGGED TABLE`, then `ALTER TABLE ... SET LOGGED` post-load if desired. Roughly halves write amplification. Also set `wal_compression = on`.

### S5 — [resolved by L6] `orbiting` will be populated, not dropped
Column stays; add to COPY + CSV (see L6 plan). Type note: `orbiting` and `satellites` fit `SMALLINT`.

### S6 — [Medium] SMALLINT candidates
`stars`: `star_index`, `type`, `spectr` (2.6B → saves ~4 GB). `planets`: `index`, `satellites`, `theme_id`, `water_item`, `orbiting` (≈10 B/row × 2.24B ≈ 20 GB, before alignment effects). **Keep the 42 vein columns INT** — verified max ~9.4e8 exceeds SMALLINT by far. Order columns widest→narrowest (8B, 4B, 2B, then the 3 BOOLs last) to minimise alignment padding.

### S7 — [Medium] Low-selectivity B-trees: `gas_giant` (bool), `type` (5 values), `spectr` (8)
At 2.24B rows, `idx_planets_gas_giant` alone is tens of GB for near-zero planner value.
**Fix:** drop, or use partial indexes (`WHERE gas_giant`) matched to actual query patterns; decide at post-load index-creation time (S1 makes this easy to iterate).

### S8 — [Medium] Partition `planets` (and optionally `stars`) by seed range
2.24B-row monolith ⇒ `PARTITION BY RANGE (star_id)` (star_id encodes seed×stride, so ranges align with seed ranges). Enables parallel COPY, per-partition index builds, pruning, and cheap re-generation of a seed range (drop partition + regen — also the cleanest resume/repair primitive). Must be decided **before** generation; retrofit = full rewrite.

### S9 — [Low→High at 3 TB] Filesystem compression is the biggest single lever
Fixed-width numeric rows compress 3–5×. ZFS/btrfs with zstd/lz4 under the tablespace turns ~3 TB into ~0.7–1 TB with zero schema changes. Given the stated "too large to give away" concern, this is the cheapest path to a shareable dataset.

---

## 8. Optional Ideas

### I1 — Composite `(seed, star_index)` key instead of computed `star_id`
Eliminates the stride invariant entirely (relevant now that STAR_COUNT becomes configurable — L7), self-documents, and makes per-seed queries/partition pruning natural. Planets: `(seed, star_index, planet_index)`. Decide before the 10M run.

### I2 — Determinism smoke test in CI
Generate seeds 0..100 with `DRY_RUN`, hash the CSV output, compare against a checked-in golden hash. Catches RNG/formula regressions (the whole dataset's value rests on game-exact determinism) and gives CI a real signal (fixes D8's false green).

### I3 — Minimal Prometheus text endpoint
`compose.md` already plans Prometheus/Grafana. A bare `TcpListener` thread serving `seeds_generated_total`, `seeds_committed_total`, `channel_depth`, `seeds_per_second` (~50 lines, no new deps) makes long 10M-seed runs observable without the TUI.

---

## 9. Resolved Questions (owner decisions, 2026-07-02)

| Q | Decision | Effect on findings |
|---|----------|--------------------|
| Q1: 3 TB scope | 3 TB is for the full 10M seeds | Storage math consistent (raw ≈ 0.7 TB + indexes); S1/S7/S8/S9 priorities confirmed; partitioning decision is pre-generation |
| Q2: compose workflow | Intentional split: postgres compose first → `create_database.py` → worker compose; `host.docker.internal` is by design | D5 downgraded to documentation + wait-for-DB hardening; fix stale memory.md gotcha |
| Q3: STAR_COUNT | Make dynamic/configurable via env | L7 escalated to High with implementation plan; L10 guards become required; DX5 validation extended |
| Q4: satellites/orbiting | Keep both; must be populated | L6 escalated to pre-generation blocker with implementation plan; S5 resolved as "populate, don't drop" |
| Q5: serde | Confirmed dead code, zero production impact | P5: full removal is safe (dep + derives + manual impls + serde-only fields) |

## 10. Suggested Implementation Order

1. **Correctness blockers (before any large run):** L1 single-txn commit, L2 recv_timeout fix (+sender drop), L3 restart policy, R1 Docker base image, L6 satellites/orbiting, L7 configurable STAR_COUNT + stride guard (or I1 composite key), L8 planets uniqueness, S8 partitioning decision.
2. **Load performance:** S1 indexes-after-load, S4 UNLOGGED, S3/S6 column types (must precede generation), P1 remove name-gen, P3 write! formatting.
3. **Robustness:** R2 SIGTERM drain, R3 error propagation, R4 truncate, R6 percent clamp, R7 skip bad seeds, L4 resume-from-DB-watermark.
4. **DX polish:** DX1 real seeds/sec, D7 non-TTY logging, DX7 DRY_RUN, D3/D4 Docker build hygiene, D8 CI overhaul + I2 golden test, DX10 README.
