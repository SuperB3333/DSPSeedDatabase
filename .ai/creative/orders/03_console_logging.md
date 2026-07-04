# Order 03 — Console Logging & stderr Progress (gap-fill)

## Target Files
- `src/metrics.rs`
- `src/main.rs` (monitor loop + TUI setup/teardown only)

## Safety & Equivalence Zone
- FROZEN: `src/algorithm/**`, `src/generate_csv.rs`, `worker_thread`/`commit_thread` bodies, `src/logging.rs` core (LOG_LEVEL parsing, level constants, macros — already correct; do not redesign).
- Nothing in this order runs in the per-seed hot loop; generation output must be bit-identical.
- Preserve: log lines go to stderr only; TUI data goes to stdout only. Never mix.

## Implementation Plan
1. **TUI gating** (`main.rs`): add `let tui = std::io::stdout().is_terminal() && env_str!("NO_TUI", "0") != "1";` (`use std::io::IsTerminal;`, stable since 1.70). Wrap `EnterAlternateScreen`/`Hide`/`Show`/`LeaveAlternateScreen` and the `write_metrics` call in `if tui`.
2. **stderr progress fallback** (`metrics.rs`): new fn
   `pub fn log_progress(sps: f32, goal: i32, queue: i32)` emitting ONE plain line via `log_info!`:
   `progress: committed=X/GOAL (P%), in-flight=Q, seeds/sec=S`.
   Called from the monitor loop every ~5s (tick counter `% 50`) when `!tui`; never when TUI is active.
3. **Real seeds/sec** (`main.rs` loop): replace hardcoded `-1.0` with
   `COMMITTED_SEEDS.load(Relaxed) as f32 / start.elapsed().as_secs_f32().max(1e-6)`.
   Pass to both `write_metrics` and `log_progress`.
4. **Underflow clamp** (`metrics.rs`): `let cpu_percent = ...min(100);` same for `db_percent` — fixes `"░".repeat(100 - p)` panic when committed+queue > goal.
5. **Raw-mode churn** (`metrics.rs`): move `enable_raw_mode`/`disable_raw_mode` out of `write_metrics` — call once at TUI setup / teardown in `main.rs` (inside the `if tui` blocks). `write_metrics` keeps only MoveTo/Clear/writeln. Drop the `for _ in 0..4 { Clear(All) }` loop to a single `Clear(All)`.
6. **Final summary**: the existing end-of-run `log_info!("done: ...")` already lands on stderr — keep, no change.

## Validation Criteria
```powershell
cargo check; cargo build --release
```
- TTY run (small range, no DB needed if Order 04 exists; else local PG): TUI renders as before, no panic at 100%.
- Redirected run: `$env:NO_TUI="1"; .\target\release\dsp_seed_finder.exe 2>err.log 1>out.log` → `out.log` empty of escape codes; `err.log` has `[info] progress: ...` lines with nonzero seeds/sec.
- `$env:LOG_LEVEL="error"` → progress lines suppressed, errors still emitted.
- Logic check: no call added inside `worker_thread`'s per-seed loop (grep diff of main.rs lines 34–41 → unchanged).
