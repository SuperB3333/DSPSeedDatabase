CREATE USER postgrest_api WITH PASSWORD 'postgrest_api_password';

ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO postgrest_api;
