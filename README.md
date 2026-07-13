# DSP Seed Finder

A high-performance pipeline for generating, indexing, and searching Dyson Sphere Program (DSP) galaxy seeds. It replaces the trial-and-error of seed hunting with a multi-threaded Rust generator and a flexible Python-based SQL search engine.

## Core Capabilities

- **High-Speed Generation:** Multi-threaded Rust implementation of the DSP galaxy generation algorithm, capable of processing thousands of seeds per second.
- **SQL-Powered Search:** Exports seed data to PostgreSQL for complex queries that the game's UI cannot perform (e.g., "Find a seed with 6 O-stars and a tidally locked planet inside the Dyson radius").
- **Resume-Safe:** Atomic checkpointing ensures that long-running generation jobs (e.g., processing 10 million seeds) can be paused and resumed without data corruption or duplication.
- **Flexible Scoring:** A Python scoring engine ranks seeds based on weighted metrics like total luminosity, rare ore availability, or "inside-sphere" planet counts.

## Quick Start

### 1. Infrastructure
The fastest way to get the database ready is via Docker Compose:
```bash
docker-compose -f compose/compose.postgres.yaml up -d
```

### 2. Schema Setup
Initialize the database schema (requires `psycopg2`):
```bash
python3 create_database.py
```

### 3. Generate Seeds
Build and run the generator. This will populate the database with seeds in the specified range.
```bash
# Configuration via environment variables
START_SEED=0 END_SEED=100000 WORKER_THREADS=16 cargo run --release
```

### 4. Search & Score
Find the best seeds based on your own criteria:
```bash
# Example: Rank seeds by O-star luminosity and Silicon availability
python3 score_seeds.py --weight luminosity=1.5 --weight ore_silicium=1.0 --top 10
```

## Architecture

### The Pipeline
1.  **Generation (`src/`)**: A Rust crate that replicates the game's internal generation logic. It bypasses all rendering/UI and uses `crossbeam-channel` to pipe generated CSV data from worker threads to database writer threads.
2.  **Ingestion**: Uses PostgreSQL `COPY FROM STDIN` for maximum throughput. Tables are created as `UNLOGGED` by default to minimize Write-Ahead Log (WAL) overhead.
3.  **Search (`rules.py`, `score_seeds.py`)**: A Python layer that compiles high-level search criteria into optimized SQL.

### Database Schema
Data is split into two primary tables:
- **`stars`**: High-level system data (luminosity, type, star-level aggregated ores).
- **`planets`**: Detailed planet-level data (vein counts, tidal locking, orbital radius, gas rates).

**Note on Sentinels:** To handle the variance in planet types, the database uses `-1` as a sentinel value for ores that do not exist on a given planet (e.g., iron on a gas giant). The scoring engine uses `GREATEST(estimate, 0)` to ensure these sentinels never subtract from a seed's score.

## Configuration

The Rust generator is configured via environment variables:
- `START_SEED` / `END_SEED`: The range of seeds to process.
- `WORKER_THREADS`: Number of threads generating galaxy data.
- `WRITER_THREADS`: Number of threads pushing data to Postgres.
- `CHECKPOINT_FILE`: Path to the progress tracking file (default: `checkpoints.txt`).
- `BENCHMARK=1`: Disables DB writes to test pure generation throughput.
- `DIAGNOSTICS=1`: Prints worker, channel, COPY, commit, payload-byte, and actual batch-size measurements after the run. Stage durations are aggregate thread time and can exceed wall-clock time when threads overlap.

## Sharp Edges

- **Checkpoint Invariants:** The generator bails out if it detects a mismatch between the current config and an existing `checkpoints.txt`. If you change your seed range or worker count, you must delete the old checkpoint file.
- **UNLOGGED Tables:** For speed, tables are `UNLOGGED`. If the database crashes, data may be lost. Run `ALTER TABLE ... SET LOGGED` if you require durability.
- **Database Load:** The generator can easily saturate a standard Postgres installation. If you see "channel stall" warnings, try increasing `COMMIT_COUNT` or decreasing `WORKER_THREADS`.

## Making Your First Change

The shortest path to a meaningful change is adding a new metric to the scoring engine:
1.  Open `score_seeds.py`.
2.  Add a new entry to the `_build_metrics` dictionary (e.g., a filter for specific planet themes).
3.  Run the script with your new metric: `--weight your_metric=1.0`.
