# Memory

## Project
Dyson Sphere Program (DSP) seed database for generating, storing, and querying game seeds. Currently in generation/logging phase with Rust backend for high-performance seed creation and PostgreSQL storage. Query functionality will be implemented later.

## Discoveries
- **[Architecture]** Hybrid system: Rust (src/) for heavy generation/DB writes, Python (server/) for the query API and schema management. (Source: src/main.rs, server/server.py)
- **[Build]** Rust component lives under `inserter/`; Docker builds it by copying `inserter/` into `/app` and compiling the `dsp_seed_finder` binary for `x86_64-unknown-linux-musl`. (Source: inserter/Cargo.toml, Dockerfile)
- **[Config]** Rust workers configured via env vars: `START_SEED`, `END_SEED`, `WORKER_THREADS`, `COMMIT_COUNT`, `CHANNEL_SIZE`, `CHECKPOINT_FILE`. (Source: src/main.rs)
- **[Dependency]** Python relies on `psycopg2` for DB and `websockets` for API. Rust uses `crossbeam-channel` for task distribution. (Source: Cargo.toml, server/server.py)
- **[Tooling]** Docker Compose files in `compose/` support SQLite, PostgreSQL, and Monitoring (Prometheus/Grafana) stacks. (Source: compose/compose.md)
- **[Convention]** DB schema (stars/planets) and extensive indexing are managed by `create_database.py`. (Source: create_database.py)
- **[Gotcha]** `server/server.py` references a `dsp_generator` module not found in the root file list, possibly a missing component or name mismatch. (Source: server/server.py)
- **[Tooling]** Rust backend includes a TUI-style progress monitor using `crossterm`. (Source: src/metrics.rs)
- **[Pattern]** Rule-to-SQL conversion is implemented via a hierarchical class structure in `rules.py`. (Source: rules.py)
