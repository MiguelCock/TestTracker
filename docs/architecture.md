# Architecture

TestTracker is a local-first desktop application for planning, executing, reviewing, and reporting
penetration-testing engagements. It is a native Rust application built with **GPUI** and
**gpui-kit**; there is no webview, JavaScript runtime, or remote content.

## Status

Implemented foundations:

- Encrypted local vault (Argon2id + SQLCipher).
- Local profile with an Ed25519 signing identity.
- Engagements with application summary and scope assets.
- Global template library with custom templates and curated/imported bundles.
- Unified Tests & Findings workbench with the Probability × Impact severity matrix and OWASP/CWE
  fields.
- Encrypted virtual Files workspace for Postman, Burp, HAR, Procmon, OpenAPI, GraphQL, Nmap, logs,
  screenshots, and other pentest artifacts.
- One Plan/Build agent with read-only virtual-file tools and approved change proposals.
- Document generation (PDF via Typst, DOCX via docx-rs, XLSX via rust_xlsxwriter) with encrypted
  document storage.
- AI provider connections (Ollama, OpenAI-compatible, Anthropic, Azure OpenAI) with encrypted
  credentials, per-engagement policy, redaction preview, one-time consent, and audit logging.
- Plan and Build agent modes with scoped context, validated proposals, and explicit apply approval
  for summaries, tests, test approaches, and findings.
- Optional local MCP server bridge with a loopback-only Streamable HTTP endpoint, bearer-token
  authentication, read tools, draft-only proposal tools, and in-app approval.
- Engagement assistant with auto-selected validated actions, finding improvement, focused
  test/finding/file context, and text extraction from engagement files.
- Curated test templates and bundles with validated JSON import/export.
- Encrypted Files workspace with Postman, Burp XML, HAR, Procmon, OpenAPI, GraphQL, Nmap, and text
  parsers.
- Read-only agent file tools with virtual paths, per-call permission prompts, revocable
  always-allow rules, and an audited bounded tool loop.
- Styled PDF, DOCX, and XLSX report output with cover pages and severity/status color treatment.
- Signed read-only WebAssembly plugins with a trusted-publisher store, capability declarations, and
  a fuel-/memory-limited runtime.
- Passphrase-encrypted engagement bundles with signed manifests, hash verification, and logical
  import that remaps identifiers.
- Append-only, hash-chained audit events with verification, shown in Settings → Activity.
- Auto-lock and session expiry.
- OpenCode Plan/Build agent pack sharing the MCP tool surface.
- Native file open/save dialogs and clipboard integration.

Planned next: MCP client/tool calling, plugin process runners and UI contributions, full-vault
backup and multi-device sync, and multi-profile separation of duties.

Operational references: [`operations.md`](operations.md) for backup/restore and rollback,
[`migration-policy.md`](migration-policy.md) for schema and cutover gates,
[`information-architecture.md`](information-architecture.md) for the target screen map, and
[`visual-baseline.md`](visual-baseline.md) for known layout defects.

## Layers and trust boundaries

```
GPUI app (crates/app) — native Rust UI
   │ direct service calls / background tasks
testtracker-core (crates/core) — all trust decisions
   ├─ domain rules, validation, RBAC
   ├─ SQLCipher database + migrations
   ├─ crypto, vault, identity, audit
   ├─ AI gateway
   ├─ MCP bridge (loopback HTTP, token, approval queue)
   ├─ WASM plugin host
   ├─ reference-file parsers
   └─ document renderers (PDF, DOCX, XLSX)
        │
        ├─ OS keychain / passphrase-derived keys
        ├─ encrypted engagement files and generated-document store
        ├─ allowlisted AI provider APIs
        └─ encrypted engagement bundles
```

The UI is in-process Rust. It never opens the database, reads `vault.json`, or decrypts provider,
vault, or signing keys; those fields are `pub(crate)` in `core`. The crate boundary prevents
accidental access, but it is not an OS hostile-user boundary. If that isolation is required, split
`core` into a separate process with a narrow IPC surface.

## Key hierarchy

| Key                              | Purpose                         | Protection                                                   |
| -------------------------------- | ------------------------------- | ------------------------------------------------------------ |
| Passphrase                       | Unlocks the vault               | Never stored; Argon2id with 64 MiB, 3 iterations             |
| Master key                       | Root of the local key hierarchy | Encrypted in `vault.json` by the passphrase-derived key      |
| Database key                     | SQLCipher key                   | Derived from the master key with BLAKE3 context separation   |
| Profile signing key              | Ed25519 approvals and sign-offs | Private key encrypted with the master key; public key stored |
| Engagement content key (planned) | Evidence and bundle encryption  | Wrapped by the master key; deleting it crypto-shreds content |

## Local-first trust model

A single device owns one encrypted store. Multiple local profiles are planned; the current build
creates one administrator profile. RBAC prevents accidental actions and structures workflow, but the
device owner can always read the local database. Cryptographic sign-offs and the audit chain make
changes verifiable when engagements are exported and shared.

True hostile-user isolation requires the future self-hosted server. Do not claim local RBAC provides
it.

## Data flow

1. A GPUI view calls a function in `testtracker_core::services::*` through `Backend::call`.
2. The service checks the session, auto-lock state, and role.
3. Input is validated against explicit limits.
4. The mutation runs in a SQLite transaction.
5. An audit event is appended to the same transaction.
6. The transaction commits and a DTO is returned to the view on the GPUI main thread.

Read operations also require an unlocked session and update the session activity timestamp. Long
database, document, import, and network work runs on the tokio runtime or GPUI background executor;
`render` never blocks on the session mutex.

## Directory layout

```
crates/core/           Rust core, no GUI dependencies
  src/domain/          models, enums, severity, RBAC
  src/db/              migrations, repositories, seed templates
  src/security/        crypto, vault, identity, audit
  src/ai/              providers, redaction, agent, assistant, reference tools
  src/mcp/             loopback MCP bridge, token auth, tools, approvals
  src/plugins/         signed WASM packages and runtime
  src/bundles/         encrypted engagement bundles
  src/imports/         assessment extraction and drafts
  src/documents/       PDF, DOCX, XLSX renderers
  src/services/        validation + transaction + audit service layer
integrations/opencode/ Plan/Build agents and MCP configuration
  src/state.rs         process state and unlocked session
crates/app/            GPUI application
  src/main.rs          application entry point
  src/backend.rs       tokio runtime bridge and error mapping
  src/app.rs           root view, screens, dialogs, session state
  src/ui.rs            reusable native widgets
  src/theme.rs         colors and typography tokens
  src/format.rs        labels and date/byte formatting
docs/                  architecture and security documentation
reference/             ignored, local-only source material
```

## Commands

```bash
cargo run -p testtracker-app
cargo build --release -p testtracker-app
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit
```

## Security invariants

- **Core-only trust**: the UI cannot open SQLite or read vault/provider/signing keys.
- **Audit every mutation**: the audit event is written in the same transaction as the data change.
- **Server-side severity**: probability/impact mapping is authoritative in `domain/severity.rs`.
- **Bounded parsing**: documents, bundles, plugins, and engagement files are untrusted; validate
  size, format, hashes, IDs, paths, and text limits before use.
- **Explicit network consent**: AI prompts require a redacted preview and one-time consent; policy is
  checked again on send.
- **No auto-apply**: agent, plugin, and MCP output is a proposal until the user applies it.
- **Plan is read-only**: the Plan agent has no write tools; Build proposals require explicit apply.
- **Loopback-only MCP**: bearer token, Host/Origin validation, no direct write tools, stop on lock.
- **No remote content**: the app loads no web content and has no shell/HTTP plugin surface.
- **Native file validation**: dialogs choose paths, but core validates extension, size, and content
  before storage or execution.
