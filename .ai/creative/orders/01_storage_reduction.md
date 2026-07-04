# Order 01 — Storage Reduction (schema-only)

## Target Files
- `create_database.py` (only file modified)

## Safety & Equivalence Zone
- DO NOT touch `src/**` — CSV byte output and `COPY_STAR`/`COPY_PLANET` column lists stay identical. Postgres `COPY ... (col list) FROM STDIN CSV` maps by name, so table column order/types may change freely as long as names survive and values parse.
- Verified in-range (do not second-guess): `star_index` 0..63, `type` 1..5, `spectr` -4..3, planet `index`/`orbiting`/`satellites`/`theme_id`/`water_item` all « 32767. All FLOAT columns hold f32-sourced data → `REAL` is lossless (Rust `{}` prints shortest-roundtrip f32 repr).
- **KEEP all 42 vein columns (`min_*`,`max_*`,`estimate_*`) and 14 `ore_*` columns as `INT`** — values reach ~9.4e8, exceeds SMALLINT.
- Do not alter `parse_rule.py`/`rules.py` — numeric comparisons work unchanged on narrower types.

## Implementation Plan
1. **stars table:**
   - `id INT UNIQUE PRIMARY KEY` → `id INT PRIMARY KEY` (drops redundant second unique index).
   - `start_dist`, `luminosity`: `FLOAT` → `REAL`.
   - `star_index`, `type`, `spectr`: `INT` → `SMALLINT`.
   - Keep `seed INT`, `dyson_radius INT`, `ore_* INT`.
   - Declare columns widest→narrowest: INT/REAL group, then SMALLINT group.
2. **planets table:**
   - `sun_distance`, `temperature`, `gas_h`, `gas_d`, `gas_i`: `FLOAT` → `REAL`.
   - `index`, `orbiting`, `water_item`, `satellites`, `theme_id`: `INT` → `SMALLINT`.
   - Order: `star_id` + 42 vein INTs + 5 REALs first, then 5 SMALLINTs, then the 3 BOOLs (`gas_giant`, `inside_ds`, `tidal_lock`) last. Keep `UNIQUE(star_id, index)`.
3. **UNLOGGED load:** `CREATE UNLOGGED TABLE` for both tables (regenerable data; halves write amplification). Add comment: run `ALTER TABLE ... SET LOGGED;` post-load if durability wanted.
4. **Indexes after load:** move all `CREATE INDEX` calls into a new function `create_indexes()`; add `argparse` with `--indexes` flag → runs only `create_indexes()`; default (no flag) runs only `create_schema()` (tables, no indexes). Delete `index("stars", "id")` (duplicates PK).
5. Remove the dead `dist_cols` variable + `#TODO` string (it injects a literal comment via `c()` — currently no-op, keep it no-op or delete).
6. Python 3.12 note: keep `'\n'.join(...)` expressions out of f-string braces (hoist to variables) so the script runs on ≥3.10.

## Validation Criteria
```powershell
python -c "import ast; ast.parse(open('create_database.py').read())"
python create_database.py            # against a scratch/local PG
python create_database.py --indexes
```
- SQL checks: `\d stars` / `\d planets` show SMALLINT/REAL as specified; `SELECT COUNT(*) FROM pg_indexes WHERE tablename='stars' AND indexname='idx_stars_id';` → 0.
- End-to-end: run Rust worker for seeds 0..100; assert `SELECT COUNT(*) FROM stars` = 6400 and no COPY errors.
