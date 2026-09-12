# Metric summary

`summarize_metric()` returns numerator, denominator, explicit exclusion counts,
and a metric-definition version. In-flight and unknown traces never enter the
terminal denominator, preventing percentages from overstating reliability.

## Verification

Run `cargo test -p landfall-server metrics_summary`. A zero denominator is
valid and means there is not enough terminal evidence, not a zero-success rate.
