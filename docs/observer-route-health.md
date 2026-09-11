# Observer route health and data-quality gaps

`RouteHealthRegistry` aggregates request, success, failure, and rate-limit
counters independently for each RPC route. `DataQualityGap` names evidence
limitations such as no status, unavailable RPC, malformed provider response,
or unsupported transaction version. These signals are kept separate from the
transaction outcome so a provider outage is not misreported as a dropped
transaction.
