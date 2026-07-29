# Docker Compose Files

Run all commands in this document from the repository root.

## Full Stack

`compose/compose_full.yaml` uses this startup sequence:

1. Docker starts PostgreSQL.
2. Docker waits for the PostgreSQL health check to pass.
3. The generator creates the schema.
4. The generator writes the configured seeds.
5. PostgREST starts after the generator exits with code `0`.

Set the PostgreSQL password:

```bash
export POSTGRES_PASSWORD='replace-this-password'
```

Start the full stack:

```bash
docker compose -f compose/compose_full.yaml up -d --wait
```

The default range contains seeds from 0 to 99. The application programming
interface (API) listens on `127.0.0.1:3000`.

## PostgreSQL Only

`compose/compose.postgres.yaml` starts PostgreSQL. PostgreSQL stores data in
`data/`. PostgreSQL listens on `127.0.0.1:5432` by default.

Set the PostgreSQL password:

```bash
export POSTGRES_PASSWORD='replace-this-password'
```

Start PostgreSQL:

```bash
docker compose -f compose/compose.postgres.yaml up -d --wait
```

Set `POSTGRES_PORT` to use a different host port.

## Generator Only

`compose/compose.yaml` starts one finite generator job. The generator connects
to a PostgreSQL server on the Docker host.

Set the PostgreSQL password:

```bash
export POSTGRES_PASSWORD='replace-this-password'
```

Start the generator:

```bash
docker compose -f compose/compose.yaml up
```

Set `START_SEED` and `END_SEED` before the command to select a range.

## Stop Services

```bash
docker compose -f compose/compose_full.yaml down --remove-orphans
```

The `docker compose down` command does not remove the bind-mounted database
files in `data/`.
