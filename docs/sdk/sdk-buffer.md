# SDK bounded buffer

`EventBuffer` stores events in FIFO order up to a configured capacity. `push`
returns `false` on overflow instead of allocating more memory; callers can
increment a drop counter and notify their callback. `drain` removes a bounded
batch for transport. `restoreFront` returns an unsent batch to the front, ahead
of events captured while it was in flight. `LandfallSdk` uses that operation
automatically after a failed collector response.
