# Job claim/recovery benchmark

The benchmark model uses a monotonic test clock. A worker claims a pending job
with a finite lease; a second worker cannot claim it while the lease is valid.
When the lease expires, recovery returns the job to `pending`, increments the
next claim's attempt count, and permits another worker to continue. Completion
clears the lease and is not recoverable.

Run `cargo test -p landfall-server job_recovery`. Record worker count, lease
duration, claim latency, recovered jobs, and attempts in an integration run;
the unit test verifies state transitions only.
