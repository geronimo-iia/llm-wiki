# Decision: ops::ingest::ingest Naming — Leave As-Is

## Context

`ops::ingest::ingest` stutters internally (`ingest::ingest(...)`),
but external call sites read fine as `ops::ingest(...)`.

→ [analysis prompt](../prompts/rename-ops-ingest.md)

## Decision

**Leave as-is.** The stutter is 1 internal line. The external API
matches the pattern of other ops functions (`ops::search`,
`ops::list`, `ops::content_read`). Renaming 6+ call sites for
1 line of internal clarity is not worth the churn.
