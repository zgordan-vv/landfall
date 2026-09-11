# Metric summary

`summarize_metric()` returns numerator, denominator, explicit exclusion counts,
and a metric-definition version. In-flight and unknown traces never enter the
terminal denominator, preventing percentages from overstating reliability.
