# Migration and rollback policy

This policy governs schema changes and data cutover for the engagement-simplification backlog. It is
normative for E2, E3, E8, and E10.

## Versioning

- Schema versions use `PRAGMA user_version` and the ordered `MIGRATIONS` registry in
  `crates/core/src/db/schema.rs`.
- Migrations are forward-only. There are no down migrations.
- The build refuses a database whose `user_version` is newer than `LATEST_VERSION`; an older binary
  must not attempt to seed or modify a newer schema.
- The oldest supported source schema is version 1. A fresh database starts at version 0.
- Never renumber, rewrite, or delete a shipped migration. Add a new migration to the end and update
  `LATEST_VERSION` through the registry length.
- Every migration runs inside a transaction. A failed step rolls back and leaves the database at
  the last successful version.

## Preconditions

All of the following must be true before a migration that changes or removes user data:

1. The application is closed.
2. A full data-directory backup exists outside the repository and is checksum-verified.
3. The backup has been opened successfully in the current build.
4. The audit chain verifies in the backup.
5. Free disk space is at least twice the data-directory size.
6. The pre-drop backup is retained until post-cutover verification passes.
7. The owner has signed the cutover checklist.

See `docs/operations.md` for the backup, restore, and verification commands.

## Migration phases

| Phase | Content | Data loss | Gate |
| --- | --- | --- | --- |
| Additive | New tables/columns, backfills, new indexes | None | Normal review; migration fixture green |
| Copy | Evidence → Files, documents → Files, template ownership | None; source retained | Hash/count verification and idempotent rerun |
| Destructive | Drop legacy tables and columns | Yes, one-way | All readers removed; UI/report/MCP/agent checks; signed backup; owner sign-off |
| Cleanup | Remove dead services, DTOs, routes, and tests | None | Destructive migration already verified |

Destructive SQL must not share a release step with the code that stops reading the old tables.
Ship the additive/copy work first, verify it, then ship the drop in a separate change.

## Tables and data

### Drop only after the gate

| Table | Replacement | Conditions |
| --- | --- | --- |
| `evidence` | `engagement_files` | Every evidence row migrated and hash-verified; Files UI owns the workflow; MCP/report tools no longer read evidence |
| `evidence_blobs` | `engagement_files` paths | Same as above; no remaining blob references |
| `snapshots` | Audit chain and document history | Review UI and services removed; bundle signing does not depend on snapshots |
| `approvals` | Audit chain | Review and sign-off flows removed; Ed25519 identity retained for bundles |
| `risks` | Engagement application summary / scope | UI, DTOs, reports, MCP context, and agent prompts no longer read or write them |
| `assumptions` | Engagement application summary | Same as risks |
| `contacts` | Engagement metadata | Same as risks |

### Preserve

- `profiles` and encrypted signing keys (required for bundles).
- `engagements`, `scope_assets`, `test_cases`, `findings`, and their links.
- `test_case_templates`, `test_template_bundles`, and bundle items.
- `documents`, `document_versions`, `generated_documents`.
- `engagement_files`, `ai_tool_permissions`.
- `ai_providers`, `ai_policies`.
- `plugins` and publisher trust.
- `audit_events` and the hash chain.
- `settings`.

Dropping a table must not reset `user_version` or rewrite audit rows. The drop itself is audited
through the normal migration/cutover record.

## Verification gates

A destructive migration is not complete until all checks pass on a **copy**:

- `user_version == LATEST_VERSION`.
- `PRAGMA integrity_check` returns `ok`.
- `PRAGMA foreign_key_check` returns no rows.
- `audit::verify_chain` returns true.
- Preserved table row counts match the pre-migration backup.
- Migrated Files counts and sampled plaintext hashes match the evidence source.
- The vault unlocks and the engagement list matches the backup.
- Reports generate in all supported formats.
- MCP and agent context contain no removed fields.

The automated fixture in `crates/core/src/db/migration_tests.rs` covers schema shape, backfills,
foreign keys, integrity, and the audit chain. It must be extended for every new migration,
especially the destructive one.

## Failure handling

- Stop. Do not retry on the only copy.
- Keep the failed copy and its logs; do not delete evidence of the failure.
- Restore from the verified backup into a fresh directory and reproduce with the migration fixture.
- If a migration fails partway, the transaction for that step is rolled back; the database remains
  at the last successful version. Retrying resumes from that version.
- If verification fails after a destructive step, restore the whole pre-migration directory. Do not
  attempt to reverse individual statements.

## Rollback

- Application rollback: check out the `pre-simplify` baseline tag and rebuild.
- Data rollback: restore the entire pre-migration data directory.
- A newer database is not expected to open in an older binary.
- Audit events created after the backup are lost on restore. Record that loss in the cutover notes.
- There is no supported partial rollback that keeps post-migration writes.

## Sign-off

Record the following in the cutover issue or a local notes file next to the backup, never in a
tracked file with client data:

```text
cutover: <epic/issue>
baseline tag: pre-simplify
backup path: <local path>
backup manifest: <SHA256SUMS path and hash>
source schema version: <n>
target schema version: <n>
fixture run: <CI URL or local result>
audit chain: verified <timestamp>
restore drill: passed <timestamp>
owner: <name>
decision: go / no-go
```

## Retention

Keep the pre-migration backup until TT-124 (post-cutover backup/restore verification) passes and
the owner signs off. Do not delete backups automatically. Backups contain client data even when the
vault and database are encrypted; store them on an encrypted volume with restrictive permissions.
