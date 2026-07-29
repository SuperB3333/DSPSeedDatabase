# DSP Seed Finder

DSP Seed Finder generates galaxy data for Dyson Sphere Program (DSP). A Rust
program writes the data to PostgreSQL. PostgREST supplies a read-only
application programming interface (API) that uses Hypertext Transfer Protocol
(HTTP).

## Services

- `seedfinder` generates stars, planets, gases, and vein amounts.
- `postgres` stores the generated data.
- `api` gives read-only access to the `stars` and `planets` tables.

The generator processes the configured seed range. The generator then stops.

## Requirements

Install these tools:

- Docker Engine
- Docker Compose
- `curl` for the API check

The release platform is Linux on a 64-bit x86 processor.

## Start The Full Stack

1. Set a PostgreSQL password.

   ```bash
   export POSTGRES_PASSWORD='replace-this-password'
   ```

2. Start the full stack with the default 100-seed range.

   ```bash
   docker compose -f compose/compose_full.yaml up -d --wait
   ```

3. Make sure that the generator exit code is `0`.

   ```bash
   docker inspect seedfinder --format '{{.State.ExitCode}}'
   ```

4. Read one star from the API.

   ```bash
   curl --fail 'http://127.0.0.1:3000/stars?select=seed,star_index&limit=1'
   ```

Set `END_SEED` to the first seed that the generator does not process.

```bash
END_SEED=1000 docker compose -f compose/compose_full.yaml up -d --wait
```

## Start Services Independently

Set the PostgreSQL password:

```bash
export POSTGRES_PASSWORD='replace-this-password'
```

Start PostgreSQL:

```bash
docker compose -f compose/compose.postgres.yaml up -d --wait
```

Start the generator after the PostgreSQL health check passes:

```bash
POSTGRES_PASSWORD="$POSTGRES_PASSWORD" \
docker compose -f compose/compose.yaml up
```

The standalone generator uses Linux host networking. It connects to PostgreSQL
at `127.0.0.1:5432`.

## Configuration

The generator reads these environment variables:

| Variable | Executable default | Function |
| --- | ---: | --- |
| `START_SEED` | `0` | Set the first seed. |
| `END_SEED` | `10000` | Set the first seed that the generator does not process. |
| `WORKER_THREADS` | Adaptive | Override the generator worker count. |
| `WRITER_THREADS` | `1` | Set the PostgreSQL writer count. |
| `COMMIT_COUNT` | `64` | Set the maximum seeds in one transaction. |
| `CHANNEL_SIZE` | `64` | Set the completed-seed queue capacity. |
| `BENCHMARK` | Off | Discard generated output. |
| `NO_TUI` | Off | Disable the terminal user interface (TUI). |
| `PG_NETLOC` | `localhost` | Set the PostgreSQL host. |
| `PG_PORT` | `5432` | Set the PostgreSQL port. |
| `PG_DBNAME` | `dsp` | Set the PostgreSQL database. |
| `PG_USER` | `postgres` | Set the PostgreSQL user. |
| `PG_PASS` | `rootpassword` | Set the PostgreSQL password. |

The adaptive worker count is the smaller value of the seed count and 32. Set
`WORKER_THREADS` to select a different worker count.

The full-stack Compose file sets `END_SEED` to `100`. The standalone generator
Compose file sets `END_SEED` to `1000`. Each generator Compose file enables
`NO_TUI`.

## Build And Test

Build the release image:

```bash
docker build \
  --build-arg IMAGE_VERSION=0.7.0 \
  --build-arg VCS_REF="$(git rev-parse HEAD)" \
  --tag dsp-seed-finder:0.7.0 .
```

Run the Rust tests in a container:

```bash
docker run --rm \
  --mount type=bind,source="$PWD/inserter",target=/src,readonly \
  --workdir /src \
  rust:1.97.0 \
  cargo test --release --locked
```

## Performance

The controlled test used one worker. The test measured the paired change in
throughput. The median gain was 38.0% for version 0.7.0. Median throughput
increased from 2.649 seeds/s to 3.668 seeds/s. The bootstrap interval was 32.0%
to 39.5% at 95% confidence.

For the method, raw data, discarded experiments, and scaling results, refer to
[`docs/performance.md`](docs/performance.md).

## Determinism

A determinism test ran the same version 0.7.0 executable two times. The test
processed seeds from 0 to 999. The `cmp` tool found no different bytes. Each
file contained 50,069,759 bytes.

```text
d188c78a555c95f2188db893a8c9890ec2a50848c9b8826f6e72c7104b3b7acd
```

The value is a 256-bit Secure Hash Algorithm (SHA-256) digest. The result is
valid for the tested executable. GNU and musl math libraries can give different
planet values.

## Data Storage

The database uses PostgreSQL `UNLOGGED` tables. PostgreSQL does not restore data
from these tables after a crash. Use logged tables if crash recovery is
necessary.

The database files are in `data/`.

1. Make sure that the current directory is the repository root.

   ```bash
   test -f Dockerfile
   ```

2. Stop all services.

   ```bash
   docker compose -f compose/compose_full.yaml down --remove-orphans
   ```

3. Remove the database directory.

   ```bash
   sudo rm -rf -- data
   ```

4. Create an empty database directory.

   ```bash
   mkdir data
   ```

## Release Image

The release image is `toti330/dsp_seed_finder:0.7.0`.

The published image has this digest:

```text
sha256:58d968aa4a6661b7b203a6716bd0de7c5a39763d688410b1d600d617513181ca
```

Examine the remote image:

```bash
docker buildx imagetools inspect toti330/dsp_seed_finder:0.7.0
```
