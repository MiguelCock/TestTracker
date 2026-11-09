# Data model

SQLite with SQLCipher. All identifiers are UUIDv7 strings; timestamps are RFC 3339 UTC strings.
Migrations are versioned with `PRAGMA user_version`. Foreign keys are enabled.

## Tables

| Table                        | Purpose                                                                |
| ---------------------------- | ---------------------------------------------------------------------- |
| `profiles`                   | Local identities, role, Ed25519 public key, encrypted private key      |
| `engagements`                | Project metadata, scope summary, attack surface, status, version       |
| `engagement_members`         | Engagement-to-profile role assignments                                 |
| `scope_assets`               | URLs, hosts, APIs, cloud accounts, and in/out-of-scope flags           |
| `risks`                      | Scoping risks with likelihood, impact, and stored severity             |
| `assumptions`                | Engagement assumptions                                                 |
| `contacts`                   | Client and engagement contacts                                         |
| `test_case_templates`        | Reusable, domain-specific test cases (web, API, mobile, etc.)          |
| `test_template_bundles`      | Curated or imported template packs                                     |
| `test_template_bundle_items` | Templates and stable source IDs in each bundle                         |
| `test_cases`                 | Engagement test execution records                                      |
| `findings`                   | Structured findings with severity, OWASP/CWE, remediation, retest      |
| `finding_test_cases`         | Many-to-many links between findings and tests                          |
| `finding_assets`             | Links between findings and scope assets (future UI)                    |
| `evidence`                   | Evidence metadata and encrypted blob references                        |
| `evidence_blobs`             | Encrypted blob locations and nonces                                    |
| `snapshots`                  | Immutable engagement snapshots with BLAKE3 content hashes              |
| `generated_documents`        | Generated PDF/DOCX/XLSX artifacts, encrypted paths, nonces, and hashes |
| `documents`                  | Editable report content (future)                                       |
| `document_versions`          | Immutable editable-document versions (future)                          |
| `approvals`                  | Reviewer decisions, snapshot links, and Ed25519 signatures             |
| `ai_providers`               | AI provider metadata and encrypted API keys                            |
| `ai_policies`                | Per-engagement AI policy (off, local only, approved cloud)             |
| `audit_events`               | Append-only hash-chained audit log                                     |
| `settings`                   | Local key/value settings                                               |

## Invariants

- A finding is a record. “No finding” is represented by the absence of a linked finding, never a
  boolean.
- `test_cases.status`, `test_cases.result`, linked findings, and evidence are independent fields. A
  passed test can still have a result narrative; a failed test may have no finding.
- `findings.severity` is derived server-side from `probability` and `impact_score`.
- Every mutation commits with an audit event in the same transaction.
- Engagement deletion cascades to its child records; the audit trail remains.
- `reference/` and generated evidence never enter the repository.

## Severity matrix

Both probability and impact are integers from 0 to 9. Each is bucketed:

| Values | Bucket |
| ------ | ------ |
| 0–2    | Low    |
| 3–5    | Medium |
| 6–9    | High   |

The matrix is:

| Impact bucket \ Probability bucket | Low           | Medium | High     |
| ---------------------------------- | ------------- | ------ | -------- |
| Low                                | Informational | Low    | Medium   |
| Medium                             | Low           | Medium | High     |
| High                               | Medium        | High   | Critical |

This matches the legacy findings register. Rust is authoritative; the frontend mirrors it for
preview only.

## Audit chain

Each `audit_events` row contains `seq`, event data, `prev_hash`, and `hash`.

```
hash = BLAKE3(prev_hash || canonical_json(seq, id, actor, action, entity, metadata, created_at))
```

The first row uses 64 zeroes as `prev_hash`. Verification recomputes the entire chain. Any edit,
deletion, or reordering breaks verification. Audit events are written in the same transaction as
the mutation they describe.

## Versioning and approvals

Engagements carry a monotonically increasing `version`. A snapshot stores a point-in-time copy of
the engagement, scope, tests, findings, and evidence metadata with a BLAKE3 content hash.

Plugin packages are signed by trusted publishers and installed under the app data directory. The
WASM file hash is stored in `plugins` and verified before every execution. Plugin output is not
persisted; only an audit event with the output hash, fuel usage, and log count is written.

AI provider API keys are encrypted with the master key and a provider-specific AAD. AI prompt
previews and one-time consent tokens live only in memory, are cleared on vault lock, and are never
written to the database. AI audit events contain hashes and metadata, not message content.

Generated documents are stored as encrypted files under the app data directory with a BLAKE3
content hash and a per-engagement key. Download decrypts and verifies the hash before returning
bytes to the UI.

An approval signature covers the approval ID, engagement ID, snapshot ID, decision, timestamp, and
snapshot content hash. Verification recomputes the message and checks the reviewer's Ed25519 public
key, so any change to the snapshot or decision invalidates the signature.

Generated reports are immutable artifacts stored in `generated_documents`; editable report content
and its version history remain planned.
