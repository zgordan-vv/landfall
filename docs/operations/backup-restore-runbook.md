# PostgreSQL backup and restore

The database contains the durable event ledger, deduplication state, jobs, and
read models. A backup is useful only if its checksum, archive structure, and
restore procedure have all been tested. Landfall uses PostgreSQL custom-format
archives because they can be verified with `pg_restore` and support selective
recovery when necessary.

## Create and verify a backup

Run backups from a host that can use the deployment's Compose configuration.
The destination must be a new path: the script refuses to overwrite either a
dump or its manifest.

```bash
BACKUP_FILE=./backups/landfall-$(date -u +%Y%m%dT%H%M%SZ).dump \
  ./scripts/backup-postgres.sh
```

The command creates a `.dump` plus a `.manifest` beside it. The manifest records
the format version, byte count, SHA-256, PostgreSQL server version, source
service, and UTC creation time. Both files use owner-only permissions. The
backup command immediately verifies the byte count, checksum, and custom archive
directory before reporting success.

Validate a backup again before copying or restoring it:

```bash
BACKUP_FILE=./backups/landfall-20260913T120000Z.dump \
  ./scripts/verify-postgres-backup.sh
```

Store the dump and manifest together in encrypted, access-controlled storage.
The checksum detects accidental corruption; it does not encrypt the archive or
make it safe to share publicly.

## Restore into an isolated database

Restore uses `pg_restore --clean --if-exists`, which can delete and replace
objects in its target database. Stop writers and workers first. Test a restore
only against a new, isolated PostgreSQL service — never the production service.

The target database is intentionally required as an explicit variable, avoiding
an unnoticed default database becoming the destructive target.

```bash
CONFIRM_RESTORE=YES \
LANDFALL_DB_SERVICE=postgres-restore \
LANDFALL_RESTORE_DATABASE=landfall_restore \
BACKUP_FILE=./backups/landfall-20260913T120000Z.dump \
  ./scripts/restore-postgres.sh
```

For a recovery Compose file, supply it explicitly to verification, backup, and
restore:

```bash
LANDFALL_COMPOSE_FILE=./restore.compose.yml \
LANDFALL_COMPOSE_PROJECT=landfall-restore \
  ./scripts/verify-postgres-backup.sh
```

Normal deployments need none of these overrides; they use `scripts/compose.sh`.

## Verify recovery before accepting traffic

1. Run application migrations; an old dump is not necessarily the current schema.
2. Compare expected raw-event counts, projection watermarks, and report
   checksums with the incident snapshot.
3. Run read-only health, privacy, and smoke checks against the recovery stack.
4. Re-enable workers only after recovered data and projections are understood.

## Automated recovery exercise

Run the disposable end-to-end exercise whenever these scripts, the PostgreSQL
image, or backup policy changes:

```bash
./scripts/test-backup-restore.sh
```

It starts two temporary PostgreSQL services, inserts a marker row into the
source, creates and validates a backup, restores it into the second service,
and asserts that the marker is present. It removes containers, volumes, and the
temporary archive on exit; the normal Landfall database is never touched.
