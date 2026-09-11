# Partition retention worker

`plan_retention()` computes a cutoff date and deterministically lists daily raw
event partitions older than that cutoff. It supports `dry_run`, which reports
the destructive set without dropping anything. A zero-day retention policy is
rejected, preventing accidental full deletion; the executor can pass the plan
to the existing storage partition-drop primitives only after explicit policy
approval.
