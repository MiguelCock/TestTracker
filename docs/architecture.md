# Architecture

TestTracker is a local-first desktop application for planning, executing, reviewing, and reporting
penetration-testing engagements. It is built with Tauri 2 (Rust) and React 19 (TypeScript).

## Status

Implemented foundations:

- Encrypted local vault (Argon2id + SQLCipher).
- Local profile with an Ed25519 signing identity.
- Engagements, scope assets, risks, assumptions, contacts.
- Test-case tracker with reusable domain templates.
- Structured findings with the Probability × Impact severity matrix and OWASP/CWE fields.
- Encrypted evidence store with per-engagement key derivation.
- Immutable engagement snapshots and Ed25519-signed reviewer approvals.
- Document generation (PDF via Typst, DOCX via docx-rs, XLSX via rust_xlsxwriter) with encrypted
  document storage.
- AI provider connections (Ollama, OpenAI-compatible, Anthropic, Azure OpenAI) with encrypted
  credentials, per-engagement policy, redaction preview, one-time consent, and audit logging.
- Server-side agent skills with scoped context, validated JSON proposals, and explicit apply
  approval for summaries, assumptions, tests, test approaches, and findings.
- Optional local MCP server bridge with a loopback-only Streamable HTTP endpoint, bearer-token
  authentication, read tools, draft-only proposal tools, and in-app approval.
- Signed read-only WebAssembly plugins with a trusted-publisher store, capability declarations, and
  a fuel-/memory-limited runtime.
- Passphrase-encrypted engagement bundles with signed manifests, hash verification, and logical
  import that remaps identifiers.
- Append-only, hash-chained audit events with verification.
- Auto-lock and session expiry.

Planned next: MCP client/tool calling, plugin process runners and UI contributions, full-vault
backup and multi-device sync, and multi-profile separation of duties.

## Layers and trust boundaries

```
React webview (untrusted)
   │ typed IPC commands / events
Tauri core (Rust) — all trust decisions
   ├─ domain rules, validation, RBAC
   ├─ SQLCipher database + migrations
   ├─ crypto, vault, identity, audit
   ├─ AI gateway
   ├─ MCP bridge (loopback HTTP, token, approval queue)
   ├─ WASM plugin host
   └─ document renderers (planned)
        │
        ├─ OS keychain / passphrase-derived keys
        ├─ encrypted evidence and generated-document store
        ├─ allowlisted AI provider APIs
        └─ encrypted engagement bundles
```

The webview never receives the master key, database key, provider credentials, or signing private
key. The frontend only sees DTOs and session metadata. All mutations validate input and permissions
in Rust.

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

1. The React UI calls a typed Tauri command.
2. Rust checks the session, auto-lock state, and role.
3. Input is validated against explicit limits.
4. The mutation runs in a SQLite transaction.
5. An audit event is appended to the same transaction.
6. The transaction commits and a DTO is returned.

Read operations also require an unlocked session and update the session activity timestamp.

## Directory layout

```
src/                 React application
  components/        UI primitives and layout
  features/          auth, dashboard, engagements, settings
  hooks/             session and auto-lock
  lib/               IPC, API, labels, formatting, severity
src-tauri/           Rust core
  src/commands/      thin IPC command layer
  src/db/            migrations, repositories, seed templates
  src/domain/        models, enums, severity, RBAC
  src/security/      crypto, vault, identity, audit
  src/mcp/           local MCP bridge, token auth, tools, approvals
  src/state.rs       process state and unlocked session
docs/                architecture and security documentation
reference/           ignored, local-only source material
```

## Commands

```bash
bun install
bun run dev            # Vite only
bun run tauri dev      # desktop application
bun run check          # lint + typecheck + tests
bun run build          # production frontend build
cd src-tauri && cargo test
cd src-tauri && cargo clippy --all-targets -- -D warnings
```

## Design decisions

- **Passphrase-first vault** instead of an OS keychain dependency. It works on every platform and
  keeps the recovery model explicit. OS keychain integration can be added as an optional unlock
  accelerator without changing the vault format.
- **SQLCipher via rusqlite** rather than an unencrypted SQLite file with application-level
  encryption. This protects indexes, WAL files, and temporary structures.
- **Hash-chained audit in SQLite** rather than an external log. It commits atomically with the data
  and can be verified offline.
- **Structured findings** rather than a boolean. Legacy `Finding = No` means “no linked finding”.
- **Server-side severity calculation** so clients cannot submit an inconsistent severity.
- **Native document renderers** instead of external office tooling: Typst for PDF, docx-rs for DOCX,
  and rust_xlsxwriter for XLSX. Output is encrypted before it is written to disk.
- **Server-side AI consent**: the redacted prompt is stored in memory with a one-time token; the
  frontend cannot send an arbitrary payload after preview. Policy and provider existence are
  re-checked at send time.
- **Local MCP bridge with no direct writes**: the MCP server binds to loopback, requires a bearer
  token, validates Host/Origin, and exposes only read tools plus proposals. Proposals are memory
  only until the user approves them in the desktop UI.
- **Host-based locality**: local-only policy is decided by the provider URL host, not by provider
  type, so a remote Ollama endpoint still requires approved-cloud policy.
- **Signed WASM plugins**: packages are Ed25519-signed over a BLAKE3 digest, installed only from
  trusted publishers, hash-checked before each run, and executed with no WASI, filesystem, or
  network imports. Plugins return suggestions; only the tester can apply them.
- **Encrypted bundles**: a passphrase-derived key (Argon2id) encrypts a ZIP payload with
  XChaCha20-Poly1305. The manifest is Ed25519-signed and lists BLAKE3 hashes for every file. Import
  creates a new engagement and remaps all identifiers; it never merges into existing data.
- **No remote content in the webview** and a strict CSP. Outbound network calls are explicit,
  allowlisted AI provider requests from Rust. The optional MCP bridge is an inbound loopback-only
  listener with token authentication, not a webview network permission.
- **Linux WebKit workaround**: `main.rs` sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` before Tauri
  initializes unless the user overrides it. This avoids a known WebKitGTK + Wayland/NVIDIA freeze.
  It is the only `unsafe` block in the application.
