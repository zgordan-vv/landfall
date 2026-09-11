# `project_trace` worker

The projection worker consumes a bounded Tokio channel, processes one
`trace_id` job at a time, and exits when its cancellation token fires or the
queue closes. The processor is injected as a callback, keeping queue
orchestration independent from the reducer and repository implementation that
will be added next.
