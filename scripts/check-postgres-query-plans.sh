#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

compose=(bash scripts/compose.sh exec -T postgres psql -U landfall -d landfall -X -v ON_ERROR_STOP=1)
day="$(date -u +%Y-%m-%d)"
next_day="$(date -u -j -v+1d -f '%Y-%m-%d' "$day" +%Y-%m-%d 2>/dev/null || date -u -d "$day + 1 day" +%Y-%m-%d)"
suffix="$(date -u +%Y%m%d)"
partition="raw_events_${suffix}"

"${compose[@]}" -c "CREATE TABLE IF NOT EXISTS telemetry.${partition} PARTITION OF telemetry.raw_events FOR VALUES FROM ('${day}') TO ('${next_day}')" >/dev/null
trap '"${compose[@]}" -c "DROP TABLE IF EXISTS telemetry.${partition}" >/dev/null' EXIT

trace_plan="$("${compose[@]}" -c "SET enable_seqscan=off; EXPLAIN SELECT trace_id FROM reporting.traces WHERE environment_id = '018f2d8e-7b3a-7c01-8a01-000000000002' AND updated_at >= now() - interval '1 day' AND updated_at < now();")"
event_plan="$("${compose[@]}" -c "SET enable_seqscan=off; EXPLAIN SELECT event_id FROM telemetry.raw_events WHERE trace_id = '018f2d8e-7b3a-7c01-8a01-000000000003' AND occurred_at >= now() - interval '1 day' AND occurred_at < now();")"

grep -q 'traces_environment_updated_idx' <<<"$trace_plan"
grep -q 'raw_events_trace_time_idx' <<<"$event_plan"
printf 'Trace and bounded raw-event query plans use the intended indexes.\n'
