#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

prometheus_image="prom/prometheus:v3.5.0@sha256:63805ebb8d2b3920190daf1cb14a60871b16fd38bed42b857a3182bc621f4996"

docker run --rm --entrypoint promtool \
    --volume "$repository_root/deploy/monitoring:/etc/prometheus:ro" \
    "$prometheus_image" \
    check config /etc/prometheus/prometheus.yml

bash scripts/compose.sh --profile monitoring config --quiet

printf 'Prometheus configuration and monitoring Compose profile are valid.\n'
