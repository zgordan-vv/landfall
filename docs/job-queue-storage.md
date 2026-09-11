# Job queue storage

`work.jobs` stores durable asynchronous work. Workers claim ready rows using
the indexed `available_at`/lease fields, perform external work outside the
claim transaction, then mark completion or retry explicitly. A partial unique
index prevents duplicate `ready` or `running` jobs with the same type and
dedupe key while allowing historical completed rows.
