# SDK configuration

`validateConfig` accepts only HTTPS collectors (or localhost development),
validates bounded batch/buffer sizes, normalizes the URL, and returns an
`Object.freeze`d snapshot. `LandfallSdk` retains that snapshot rather than
reading mutable caller options during telemetry execution.
