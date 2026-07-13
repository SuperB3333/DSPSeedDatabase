# Pipeline Auto-Tuning

`tools/tune_pipeline.py` independently sweeps worker threads, writer threads, and commit counts. It measures external wall time with `time.perf_counter()`, runs each candidate 3 times by default, and uses median seeds/second. The recommendation is the smallest candidate at or above `95.00%` of the peak median; the output also reports the efficient ceiling.

```bash
# Worker-only: executes workloads but changes 0 configuration files.
python3 tools/tune_pipeline.py --test worker --test-only

# Preview a Docker writer sweep: 0 workloads, 0 database writes, 0 files changed.
python3 tools/tune_pipeline.py --test writer --dry-run \
  --command "docker compose -f compose/compose.yaml run --rm {docker_env} seedfinder"

# Run writer and commit sweeps against a dedicated database range.
python3 tools/tune_pipeline.py --test writer --test commit \
  --allow-db-writes --start-seed 8000000 --apply-to tuning.env
```

Worker sweeps force `BENCHMARK=1`. Writer and commit sweeps force `BENCHMARK=0`, require `--allow-db-writes` and an explicit `--start-seed`, and leave inserted rows in PostgreSQL. Use a dedicated database or unused seed range.

`--test-only` prevents configuration writes. `--dry-run` executes 0 workloads and changes 0 files. `--apply-to ENV_FILE` offers a separate confirmation prompt for every recommendation; rejecting one setting leaves it unchanged and continues with the remaining settings. The `{docker_env}` command token expands to Docker `-e NAME=VALUE` arguments.
