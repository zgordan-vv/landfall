# PostgreSQL release image

`docker-compose.yml` использует `postgres:18.6-bookworm` с immutable digest.
Тег показывает читаемую current-minor версию, а digest гарантирует, что
повторный запуск не подтянет другой image с тем же тегом.

## Проверка

```bash
LANDFALL_POSTGRES_DB=landfall \
LANDFALL_POSTGRES_USER=landfall \
LANDFALL_POSTGRES_PASSWORD=local \
docker compose config --quiet
docker image inspect \
  'postgres:18.6-bookworm@sha256:1c59e2c3c818eaa0f0628f695b36e7c9e362d6b219b36a54a32df645cbd7e1af'
```

Перед release нужно выбрать актуальный minor PostgreSQL 18, проверить миграции
и security advisories, затем обновить тег и digest одним reviewable commit.
Нельзя менять только тег или удалять digest: это нарушит воспроизводимость
demo и production-like окружения.
