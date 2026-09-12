# Compose profiles

`docker-compose.yml` keeps the default `postgres` service for existing local
workflows and provides isolated profile services for repeatable demos:

```bash
# Disposable demo database
LANDFALL_POSTGRES_DB=landfall_demo \
LANDFALL_POSTGRES_USER=landfall \
LANDFALL_POSTGRES_PASSWORD=demo-only \
docker compose --profile demo up -d postgres-demo

# Production-like database volume
LANDFALL_POSTGRES_DB=landfall \
LANDFALL_POSTGRES_USER=landfall \
LANDFALL_POSTGRES_PASSWORD=local-secret \
docker compose --profile production up -d postgres-production
```

The services use the same pinned PostgreSQL image and healthcheck but separate
named volumes. This prevents a demo reset or fixture load from changing the
production-like dataset. The profile services currently provision PostgreSQL;
application wiring and migrations remain explicit follow-up commands.

Validate the expanded model without starting containers:

```bash
LANDFALL_POSTGRES_DB=landfall LANDFALL_POSTGRES_USER=landfall \
LANDFALL_POSTGRES_PASSWORD=local docker compose --profile demo config --quiet
```
