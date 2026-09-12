# 100,000-trace report benchmark

Generate the deterministic 100,000-trace workload, render the selected report
formats, and record wall-clock milliseconds. Pass trace count and duration to
`report_benchmark::measure_report_duration` to obtain traces/sec. Always record
format (HTML/JSON), privacy profile, frozen watermark, CPU/RAM, and whether
artifacts were written to disk.

The helper rejects zero traces or zero duration. It normalizes a measurement; it
does not claim the report met the design target until the full renderer/query
runner has completed.
