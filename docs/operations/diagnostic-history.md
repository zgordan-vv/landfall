# Diagnostic history

`append_diagnostics` writes versioned diagnostic rows and their evidence links
in one transaction. Reprojection appends a newer `rule_set_version`; readers
select the newest applicable claim while older results remain available for
audit and comparison. Evidence links are insert-only and duplicate-safe.
