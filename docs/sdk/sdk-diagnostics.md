# SDK diagnostics

`LandfallSdk.health` exposes aggregate `droppedEvents` and
`transportFailures` counters. Applications can receive immutable snapshots
through `onHealthChange`; callback exceptions are swallowed so diagnostics
never change transaction behavior. Counters are updated explicitly by the
buffer and transport layers through `recordDroppedEvents` and
`recordTransportFailure`.
