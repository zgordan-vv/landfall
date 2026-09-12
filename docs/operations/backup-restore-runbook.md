# Backup, restore, and incident runbook

1. Stop writers and record the current projection watermark.
2. Create a database backup and store its `BackupManifest` (ID, schema,
   byte count, SHA-256) separately from the payload:

   ```bash
   BACKUP_FILE=./backups/landfall-$(date +%Y%m%d-%H%M%S).dump \
   ./scripts/backup-postgres.sh
   ```

   The script uses custom `pg_dump` format and writes a companion `.manifest`.
3. For restore, provision an isolated PostgreSQL instance and run migrations.
4. Validate schema, byte count, and checksum before importing any bytes.
5. Restore only after an explicit destructive guard:

   ```bash
   CONFIRM_RESTORE=YES BACKUP_FILE=./backups/landfall.dump \
   ./scripts/restore-postgres.sh
   ```

6. Restore raw events first, then re-run projections and compare the resulting
   watermark and report checksums with the incident snapshot.
7. Re-enable workers only after health and privacy checks pass.

The manifest validation fails closed on schema, size, or checksum mismatch.
