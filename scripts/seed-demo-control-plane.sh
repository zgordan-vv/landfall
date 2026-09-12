#!/usr/bin/env bash
set -euo pipefail

service="${LANDFALL_DB_SERVICE:-postgres}"
db_user="${LANDFALL_POSTGRES_USER:-landfall}"
db_name="${LANDFALL_POSTGRES_DB:-landfall}"
project_id="0198ef00-0000-7000-8000-000000000100"
environment_id="0198ef00-0000-7000-8000-000000000200"

bash scripts/compose.sh exec -T "$service" psql --username="$db_user" --dbname="$db_name" \
  -v ON_ERROR_STOP=1 \
  -v project_id="$project_id" -v environment_id="$environment_id" <<'SQL'
INSERT INTO control.projects (project_id, name)
VALUES (:'project_id'::uuid, 'local-demo')
ON CONFLICT (project_id) DO NOTHING;
INSERT INTO control.environments (environment_id, project_id, name, cluster)
VALUES (:'environment_id'::uuid, :'project_id'::uuid, 'local', 'devnet')
ON CONFLICT (environment_id) DO NOTHING;
SQL
printf 'Seeded project_id=%s environment_id=%s\n' "$project_id" "$environment_id"
