# Memory

## Project
DSPSeedDatabase is a tool for bulk-scanning and storing galaxy seeds for the game *Dyson Sphere Program* (DSP). A Rust binary (`dsp_seed_finder`) generates galaxy data for a range of seeds in parallel and bulk-inserts it into a PostgreSQL database via `COPY`. A Python layer handles DB schema creation (`create_database.py`), a WebSocket query server (`server/server.py`), and utility scripts. The system is containerised: the Rust worker ships as a Docker image and runs alongside a Postgres instance via Docker Compose.

## Discoveries
Blueprint: `- **[Category]** Short fact. Details. (Source: file/context)`

- **Architecture** Rust binary is the hot path: worker threads generate CSV rows, commit threads bulk-insert via `COPY` into Postgres. Python is used only for schema setup and the query/serve layer. (Source: src/main.rs, create_database.py)
- **Build** Rust crate name `dsp_seed_finder`, edition 2021. Build: `cargo build --release`. Docker multi-stage build produces a scratch-based image. (Source: Cargo.toml, Dockerfile)
- **Config** Runtime config via env vars: `START_SEED`, `END_SEED` (default 0–10000, max 10M), `WORKER_THREADS` (default 8), `WRITER_THREADS` (default 4), `COMMIT_COUNT` (default 1000), `CHANNEL_SIZE` (default 1000), `CHECKPOINT_FILE`, `PG_USER/PG_PASS/PG_PORT/PG_DBNAME/PG_NETLOC`. (Source: src/main.rs, compose/compose.yaml)
- **Architecture** DB schema: two tables — `stars` (one row per star per seed) and `planets` (one row per planet, FK `star_id`). Ore columns for 14 vein types. (Source: create_database.py, misc.py)
- **Dependency** Rust deps: `crossbeam-channel`, `once_cell`, `serde`, `postgres`, `crossterm`. Python deps: `psycopg2`, `websockets`. (Source: Cargo.toml, server/server.py)
- **Architecture** WebSocket server (`server/server.py`, port 62879) supports three message types: `Find` (SQL query via `parse_rule`), `Generate` (run galaxy gen on-the-fly via `dsp_generator`), `Stop`. (Source: server/server.py)
- **Tooling** Docker Compose files in `compose/`: `compose.yaml` (worker only, Postgres external via `host.docker.internal`), `compose.postgres.yaml` (worker + Postgres 16 container, data volume `../data`). Default image tag `toti330/seedfinder:0.2.0`. (Source: compose/)
- **Gotcha** `compose.postgres.yaml` still sets `PG_NETLOC: host.docker.internal` — should be `postgres` (the service name) when Postgres runs as a sibling container. (Source: compose/compose.postgres.yaml)
- **Gotcha** `compose.md` documents two compose variants that don't exist yet: `compose.monitoring.yaml` (Prometheus + Grafana) and `compose.full-stack.yaml` (all-in-one). (Source: compose/compose.md)
- **Convention** `local.md` is gitignored (listed in `.gitignore` as `.ai/local.md`). `memory.md` is not gitignored — it is shared. (Source: .gitignore)
- **Gotcha** No test suite found. No `tests/` directory, no `#[cfg(test)]` detected in a shallow pass. (Source: directory listing)
