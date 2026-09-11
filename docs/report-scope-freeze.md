# Frozen report scope

`FrozenReportScope` captures the cohort identity, projection watermark, event
schema version, and all core semantic versions before report work is queued.
Every query in the job can therefore use one consistent snapshot, and the
result can be reproduced or compared later. Empty cohort or schema identities
are rejected at the boundary.
