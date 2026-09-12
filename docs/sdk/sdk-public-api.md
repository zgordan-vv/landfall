# TypeScript SDK public API

The first SDK slice exposes `LandfallSdk.startTrace()` and a lightweight
`TraceContext.emit()`. Configuration requires an explicit collector URL; only
HTTPS (or localhost development) is accepted. Telemetry failures use a bounded
typed error callback and are deliberately isolated from customer transaction
control flow.
