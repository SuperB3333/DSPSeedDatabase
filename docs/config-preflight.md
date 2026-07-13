# Configuration Preflight

Set `PRINT_CONFIG=1` to print the resolved runtime configuration and exit successfully before the program attempts a PostgreSQL connection, reads checkpoints, spawns threads, or modifies files.

The output includes the seed range, worker and writer counts, commit and channel sizes, benchmark state, checkpoint path, database user/host/port/database, and `LOG_LEVEL`. The database password is always printed as `redacted`.

Normal and preflight invocations reject `WORKER_THREADS=0`, `WRITER_THREADS=0`, and `COMMIT_COUNT=0` before work begins.
