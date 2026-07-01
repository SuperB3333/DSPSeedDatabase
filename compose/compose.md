# compose
This folder contains different Docker compose run configurations 
for the project that you can choose from.

#### Variations
- **compose.yaml**
The default compose file. Sets up only the seedfinder container with default configs.

- **compose.postgres.yaml**
Replaces the default SQLite backend with PostgreSQL. Adds a Postgres container
and configures seedfinder to use it for persistent storage.

- **compose.monitoring.yaml**
Adds Prometheus + Grafana containers. Seedfinder exposes metrics on a
configurable port; Prometheus scrapes them and Grafana provides dashboards.

- **compose.full-stack.yaml**
Combines compose.yaml, compose.postgres.yaml, and compose.monitoring.yaml
into a single all-in-one stack. Requires the most resources but gives you
the full feature set.