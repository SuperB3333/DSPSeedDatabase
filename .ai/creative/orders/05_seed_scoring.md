# Order 05 — Score-Based Seed Searching

## Target Files
- `score_seeds.py` (NEW, repo root — only file created; nothing else modified)

## Safety & Equivalence Zone
- FROZEN: `src/**` (all Rust), `create_database.py`, `parse_rule.py`, `rules.py`, `server/**`. Zero generation-logic changes; this is a read-only DB consumer.
- Script must be strictly read-only: single connection, `SELECT` only; open with `conn.set_session(readonly=True)` as a hard guard.
- Sentinel rule (critical for score correctness): gas giants carry `-1` in all 42 vein columns; rocky planets without a vein carry `-1` in that ore's min/max/estimate. Always aggregate with `GREATEST(estimate_x, 0)` (or filter `estimate_x > 0`) so sentinels can't subtract from scores.
- SQL values must be parameterized (`%s`), never interpolated — weights are user input.

## Implementation Plan
1. **Config/CLI** (argparse):
   - DB flags mirroring Rust env names: `--host/--port/--user/--pass/--dbname` with env-var fallbacks `PG_NETLOC/PG_PORT/PG_USER/PG_PASS/PG_DBNAME`, then the same defaults as `src/misc.rs` (`localhost/5432/postgres/rootpassword/dsp`).
   - `--top N` (default 25), `--seed-range LO HI` (optional pre-filter `s.seed >= LO AND s.seed < HI`), `--csv PATH` (optional output file), `--explain` (print the generated SQL and exit).
2. **Metric registry** — dict `METRICS: name -> (sql_expr, description)`, one aggregate per seed. Initial set (all derived from existing columns; use `misc.py`'s `veins` list to generate the 14 ore metrics):
   - `ore_<vein>`: `SUM(GREATEST(p.estimate_<vein>, 0))` per seed (14 entries, generated in a loop).
   - `luminosity`: `SUM(s.luminosity)` over the seed's stars.
   - `max_luminosity`: `MAX(s.luminosity)`.
   - `dyson_radius`: `MAX(s.dyson_radius)`.
   - `gas_giants`: `COUNT(*) FILTER (WHERE p.gas_giant)`.
   - `tidal_locked`: `COUNT(*) FILTER (WHERE p.tidal_lock)`.
   - `planets_inside_ds`: `COUNT(*) FILTER (WHERE p.inside_ds)`.
   - `oceans`: `COUNT(*) FILTER (WHERE p.water_item = 1000)` (water worlds).
3. **Weights:** repeatable flag `--weight NAME=FLOAT` (e.g. `--weight ore_oil=2 --weight gas_giants=0.5`). Score = `SUM(weight_i * metric_i)`. Unknown NAME → exit(2) listing valid metrics. No weights given → default `--weight ore_iron=1` and print a hint.
4. **Query shape** (one round-trip; per-star aggregates first to avoid double-counting star columns across the planet join):
   ```sql
   WITH per_star AS (
     SELECT s.seed, s.luminosity, s.dyson_radius,
            <planet aggregates: SUM/COUNT over p.*>
     FROM stars s JOIN planets p ON p.star_id = s.id
     [WHERE s.seed >= %s AND s.seed < %s]
     GROUP BY s.id, s.seed, s.luminosity, s.dyson_radius
   )
   SELECT seed, <weighted sum of seed-level aggregates> AS score,
          <each requested metric as its own column>
   FROM per_star GROUP BY seed ORDER BY score DESC LIMIT %s;
   ```
   Metric SQL fragments come from the trusted registry; only numeric weights/bounds/limit are parameters.
5. **Output:** aligned table to stdout (`seed`, `score`, one column per requested metric); `--csv` writes the same rows via `csv.writer`. Row count + elapsed time to stderr.
6. Keep it dependency-light: `psycopg2`, stdlib only (matches existing scripts). Guard body with `if __name__ == "__main__":`.

## Validation Criteria
```powershell
python -c "import ast; ast.parse(open('score_seeds.py').read())"
python score_seeds.py --explain --weight ore_oil=2 --weight gas_giants=1   # SQL prints, no DB needed
python score_seeds.py --top 10 --weight ore_iron=1 --seed-range 0 100      # against seeded local DB
```
- Logic checks: scores are non-negative when all weights ≥ 0 (sentinel guard works); `--top 10` returns ≤ 10 distinct seeds; unknown metric name exits 2 with the metric list.
- Cross-check one seed by hand: `SELECT SUM(GREATEST(p.estimate_iron,0)) FROM planets p JOIN stars s ON s.id=p.star_id WHERE s.seed=<X>;` equals the script's `ore_iron` column for seed X.
- Read-only proof: run as a user with only SELECT grants (or confirm `set_session(readonly=True)` present) — script completes.
- `git status` shows exactly one new file: `score_seeds.py` (order file aside).
