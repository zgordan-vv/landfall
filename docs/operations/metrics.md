# Prometheus metrics

`GET /metrics` exposes Prometheus text format and is deliberately outside the
customer API token middleware. Publish it only on the internal data network or
through an authenticated monitoring proxy; do not expose it to the public
internet.

The endpoint obtains a fresh, aggregate-only PostgreSQL snapshot at scrape time:

- `landfall_database_up` and `landfall_process_ready`;
- project, environment, and enabled RPC-route counts;
- durable jobs by `ready`, `running`, and `dead_letter` state;
- events received during the previous 24 hours.

It contains no project names, IDs, endpoints, token data, trace IDs, or event
payloads. A database failure returns HTTP `503` together with
`landfall_database_up 0`, so a monitoring system can distinguish an unavailable
scrape from an empty queue.

Example internal scrape configuration:

```yaml
scrape_configs:
  - job_name: landfall-server
    metrics_path: /metrics
    static_configs:
      - targets: ["server:8080"]
```

The repository already provides this configuration and operational alert rules
under [`deploy/monitoring/`](../../deploy/monitoring/). See
[monitoring and alerts](monitoring-and-alerts.md) for startup, routing, and
incident response.
