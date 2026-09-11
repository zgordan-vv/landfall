# SDK shutdown flush

`LandfallSdk.flush()` accepts the pending-flush operation and a timeout. It
returns `flushed`, `timed_out`, or `failed` and always settles within the
configured bound (apart from the small scheduling overhead). A failed flush
increments the transport-failure counter and invokes the existing telemetry
error callback. A timeout is reported as a result and does not throw.
