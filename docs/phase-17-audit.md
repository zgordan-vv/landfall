# Phase 17 documentation audit

Phase 17 establishes a consistent information architecture and upgrades the
highest-traffic entry points: quick start, architecture overview, Kit and
manual SDK guides, privacy/metrics/diagnostics guides, operations index, API
reference, troubleshooting, benchmark methodology, security limitations, and
Upwork case study.

Remaining leaf files are intentionally classified by directory and should be
reviewed against [documentation-standard.md](documentation-standard.md) when
their corresponding feature changes. Short contract notes are acceptable for
internal invariants, but any file linked as a user-facing guide must include a
procedure, example, expected evidence, and limitation.

The current API reference is honest about the registered P0 routes; generated
or planned query modules are not presented as live endpoints. This prevents
documentation from promising functionality that the runtime router does not
yet expose.
