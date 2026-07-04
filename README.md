# DSPSeedDatabase

A Rust worker that generates Dyson Sphere Program galaxy data for every seed in a range and bulk-COPYs the results into PostgreSQL. Python handles schema creation (`create_database.py`) and a WebSocket query server (`server/server.py`).

## Build

```bash
cargo build --release
```

Docker image built via the `Dockerfile` in the repo root.

## Configuration

All configuration is via environment variables. The Rust worker reads these at startup; the Python scripts use the same PG_* names for connection defaults.

### Seed range & workers

| Variable | Default | Constraint | Notes |
|----------|---------|------------|-------|
| `START_SEED` | `0` | must be `< END_SEED` | First seed to generate (inclusive). |
| `END_SEED` | `10000` | — | Last seed to generate (exclusive). |
| `WORKER_THREADS` | `8` | `>= 1`, `< 32`, `< END_SEED` | Number of generation threads. |
| `WRITER_THREADS` | `4` | — | Number of Postgres COPY consumer threads. |
| `COMMIT_COUNT` | `1000` | — | Rows per COPY transaction batch. |
| `CHANNEL_SIZE` | `1000` | — | Bounded channel capacity between workers and writers. |

### Connection

| Variable | Default | Notes |
|----------|---------|-------|
| `PG_USER` | `postgres` | |
| `PG_PASS` | `rootpassword` | |
| `PG_NETLOC` | `localhost` | |
| `PG_PORT` | `5432` | |
| `PG_DBNAME` | `dsp` | |

### Modes

| Variable | Default | Notes |
|----------|---------|-------|
| `BENCHMARK` | `0` | Set to `1` for pure-generation throughput mode — skips checkpointing and Postgres entirely. |
| `NO_TUI` | `0` | Set to `1` to disable the interactive TUI progress display (useful in Docker/CI). |
| `LOG_LEVEL` | `info` | `error`, `warn`, `info`, or `debug`. |
| `CHECKPOINT_FILE` | `checkpoints.txt` | Path to the v2 checkpoint file. |

## Database setup & run order

```bash
# 1. Start Postgres
docker compose -f compose/compose.postgres.yaml up -d

# 2. Create schema
python create_database.py

# 3. Run the generator
docker compose -f compose/compose.yaml up
```

After loading all seeds, create indexes (run once):

```bash
python create_database.py --indexes
```

### Schema summary

- **stars** — 1 row per star per seed. Primary key `id = seed * 100 + star_index`. Columns: `seed`, `start_dist`, `luminosity`, `dyson_radius`, `star_index`, `type`, `spectr`, plus 22 ore columns (`ore_iron`, `ore_copper`, …).
- **planets** — 1 row per planet, FK `star_id` referencing `stars.id`. 14 ores × 3 columns each (`min_<ore>`, `max_<ore>`, `estimate_<ore>`). Gas giants and absent veins use `-1` sentinel values. Additional columns: `sun_distance`, `temperature`, `gas_h`, `gas_d`, `gas_i`, `index`, `orbiting`, `water_item`, `satellites`, `theme_id`, `gas_giant`, `inside_ds`, `tidal_lock`.

## Checkpointing & resume

The generator writes a v2 checkpoint file every ~5 seconds using atomic writes (write to `.tmp`, `fsync`, rename over the real file).

**Resume behavior:**

- On startup the generator reads the checkpoint. If the file is missing or unparseable, it starts fresh.
- If the header (`START_SEED`, `END_SEED`, `WORKER_THREADS`) in the checkpoint does not match the current config, the process exits with an error — change the config back or delete the checkpoint.
- If all worker watermarks are at their chunk ends, the run is already complete and exits immediately.
- Otherwise, each worker resumes from its recorded watermark (the conservatively-rewound committed boundary). Before spawning, the generator DELETEs `stars` and `planets` rows in the rewound window (`star_id >= watermark * 100`) to purge duplicates from the partial commit.
- After all workers finish and writers drain, a clean-completion checkpoint is written with watermarks at chunk ends.

**Invalidation:** Changing `START_SEED`, `END_SEED`, or `WORKER_THREADS` between runs causes a header mismatch and a forced exit.

## Benchmark mode

```bash
BENCHMARK=1 ./target/release/dsp_seed_finder
```

Benchmark mode skips all Postgres and checkpoint I/O. Workers generate seeds normally but the channel consumers are `bench_sink_thread` instances that drain and discard the CSV payload while counting bytes. The final log line reports total MB and throughput:

```
benchmark: generated 10000 seeds in 3.14s (3185 seeds/sec), 123.45 MB (39.32 MB/s)
```

## Seed scoring

`score_seeds.py` ranks seeds by a user-weighted sum of per-seed metrics (read-only, never writes to the database).

```bash
# Default: rank by ore_iron weight=1, top 25
python score_seeds.py

# Custom weights, top 50
python score_seeds.py --weight ore_oil=2 --weight ore_diamond=1.5 --top 50

# Filter to a seed range
python score_seeds.py --seed-range 1000 5000

# Export to CSV
python score_seeds.py --csv results.csv

# Inspect generated SQL without hitting the database
python score_seeds.py --explain --weight ore_iron=1
```

**Available metrics:** `ore_iron`, `ore_copper`, `ore_silicium`, `ore_titanium`, `ore_stone`, `ore_coal`, `ore_oil`, `ore_fireice`, `ore_diamond`, `ore_fractal`, `ore_crysrub`, `ore_grat`, `ore_bamboo`, `ore_mag`, `luminosity`, `max_luminosity`, `dyson_radius`, `gas_giants`, `tidal_locked`, `planets_inside_ds`, `oceans`.

**CLI flags:**

| Flag | Description |
|------|-------------|
| `--host` | DB host (env `PG_NETLOC`, default `localhost`) |
| `--port` | DB port (env `PG_PORT`, default `5432`) |
| `--user` | DB user (env `PG_USER`, default `postgres`) |
| `--pass` | DB password (env `PG_PASS`, default `rootpassword`) |
| `--dbname` | DB name (env `PG_DBNAME`, default `dsp`) |
| `--top N` | Number of top seeds to return (default 25) |
| `--seed-range LO HI` | Pre-filter seeds |
| `--csv PATH` | Write results to CSV |
| `--explain` | Print generated SQL and exit |
| `--weight NAME=FLOAT` | Repeatable; set metric weight |

## Query server

`server/server.py` runs a WebSocket server on `127.0.0.1:62879` that accepts JSON messages with a `type` field:

- **Find** — query the database with filter rules; returns `Result` messages and a final `Done`.
- **Generate** — generate a galaxy on the fly for a given seed.
- **Stop** — close the connection.

## Troubleshooting

- **PK violation (`duplicate key value violates unique constraint "stars_pkey"`)** — indicates overlapping seed ranges across runs. Ensure `START_SEED`/`END_SEED` don't overlap with previously loaded data, or let the checkpoint resume handle it.
- **Stale checkpoint** — if config changed since the last run, delete `checkpoints.txt` and start fresh.
- **TUI garbage in `docker logs`** — set `NO_TUI=1` to disable the interactive progress display; the generator falls back to stderr log lines.
- **Writer channel stall panic (`recv_timeout reached`)** — a writer thread received no data for over 1 second. Check Postgres connectivity and load. This panic is by design to surface deadlocks early.
- **Container restart loop** — the app exits on completion; if `restart: unless-stopped` is set in compose, Docker will relaunch it and hit PK violations. Use `restart: "no"` or `on-failure`.

## Known issues

See `.ai/generation-review-2026-07-02.md` for the full known-issues register and implementation status.
