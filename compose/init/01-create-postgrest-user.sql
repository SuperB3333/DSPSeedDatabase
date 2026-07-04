CREATE USER postgrest_api WITH PASSWORD 'postgrest_api_password';

GRANT USAGE ON SCHEMA public TO postgrest_api;
GRANT SELECT ON stars TO postgrest_api;
GRANT SELECT ON planets TO postgrest_api;
