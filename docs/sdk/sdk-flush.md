# SDK shutdown flush

`LandfallSdk.flush()` posts the currently buffered events to `/v1/ingest` and
returns `flushed`, `timed_out`, or `failed`. An optional pending-flush operation
is retained for backward compatibility. The call always settles within the
configured bound (apart from small scheduling overhead). A failed flush
increments the transport-failure counter, invokes the telemetry-error callback,
and restores its events for a later retry. A timeout is reported as a result and
does not throw.
