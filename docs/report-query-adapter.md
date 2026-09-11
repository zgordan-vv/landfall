# Report query adapter

`document_from_query_rows()` is the bridge between database/query projections
and the existing R0 `ReportDocument`. The query layer supplies bounded rows and
their derived counts; the adapter reuses `TraceReport::from_state` so report
tokens remain identical to the offline flow. The returned `ReportSnapshot`
records the frozen projection watermark, making the exported report's freshness
and reproducibility explicit.
