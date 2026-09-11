# Confirmation-wait capture

`measureConfirmationWait()` records how long the application waited for a
commitment. A resolved operation is `commitment_reached`; rejected operations
default to `failed` and can be classified as `timeout` or `cancelled` with an
explicit mapper. Durations use the same monotonic nanosecond contract as
signing measurement and original errors remain available.
