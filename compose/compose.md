# compose
This folder contains Docker Compose configurations for running project services
individually.

#### Variations
- **compose.yaml**
Runs only the `seedfinder` container. It is configured to connect to an
individually managed PostgreSQL instance at `host.docker.internal:5432` using
database `dsp`, user `postgres`, and password `rootpassword`.

- **compose.postgres.yaml**
Runs PostgreSQL and PostgREST. PostgreSQL is available for individual connections from
local tools or from `seedfinder` in `compose.yaml`. It publishes PostgreSQL on localhost
port `5432`, creates database `dsp`, and persists data in `../data`. PostgREST is
exposed on port `3000` and provides a RESTful API over the `public` schema.

#### Usage
- Start PostgreSQL + PostgREST:
  `docker compose -f compose.postgres.yaml up -d`

- Start seedfinder only:
  `docker compose -f compose.yaml up -d`

- Connect locally (PostgreSQL):
  `postgresql://postgres:rootpassword@localhost:5432/dsp`

- PostgREST API:
  `http://localhost:3000`
