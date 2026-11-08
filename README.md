# TestTracker

TestTracker is a local-first desktop application for planning, executing, reviewing, and reporting
penetration-testing engagements. It replaces spreadsheet workflows with a structured, encrypted
workspace for engagements, scope, test cases, findings, evidence, review sign-offs, and audit
history.

Built with Tauri 2, Rust, React 19, and TypeScript.

## Features

- **Encrypted local vault** — Argon2id passphrase derivation and a SQLCipher database.
- **Engagements** — project metadata, scope assets, attack surface, risks, assumptions, contacts,
  status, and versioning.
- **Test tracker** — reusable domain templates (web, API, mobile, thick client, container, cloud,
  Windows, SAP, AI, BI), status, results, and notes.
- **Findings** — structured records with server-computed Probability × Impact severity, OWASP/CWE
  references, remediation, retest status, owner, and linked test cases.
- **Encrypted evidence** — screenshots and files encrypted per engagement with integrity hashes,
  previews, downloads, and links to findings/tests.
- **Review and sign-off** — immutable snapshots and Ed25519-signed reviewer decisions verified
  locally.
- **Document generation** — test plan, test-case tracker, findings register, executive summary, and
  technical report as PDF, DOCX, or XLSX. Generated files are encrypted at rest.
- **AI assistant and agent skills** — provider connections, per-engagement policy, redaction
  preview, one-time consent, and audit logging. Agent skills draft summaries, assumptions, tests,
  test approaches, and findings with explicit apply approval. Local-only is the default.
- **Signed plugins** — read-only WebAssembly modules from trusted publishers, executed in a
  fuel- and memory-limited sandbox with declared capabilities.
- **Encrypted bundles** — passphrase-encrypted `.ttbundle` export/import for whole engagements,
  including evidence and generated documents, with signed manifests and hash verification.
- **Audit history** — append-only hash chain with tamper detection.
- **Dashboard** — open findings by severity, test completion, blocked tests, and recent
  engagements.
- **Auto-lock** — keys are cleared from memory after inactivity.

## Requirements

- **Bun 1.3+** (package manager and scripts)
- **Rust 1.95+**
- **Linux build dependencies**: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `librsvg2-dev`,
  `libayatana-appindicator3-dev`, `libxdo-dev`, `libssl-dev`, `patchelf`, `build-essential`,
  `pkg-config`
- macOS and Windows use the standard Tauri prerequisites.

## Install and run

```bash
bun install
bun run tauri dev
```

Build a release:

```bash
bun run tauri build
```

## First run

1. Start TestTracker.
2. Create a display name and a passphrase of at least 12 characters.
3. Store the passphrase safely. It cannot be recovered; losing it means losing access to the local
   vault.
4. The app creates an encrypted database and a local Administrator profile with an Ed25519 identity
   key.

The vault is local to this device. Nothing is uploaded.

## Using TestTracker

### Dashboard

Shows active engagements, open findings, high/critical exposure, test completion, blocked tests, and
recent engagements.

### Engagements

Create an engagement from **New engagement**. Fill in:

- Project name and optional client code (use short codes rather than full client names).
- Status and test dates.
- Application summary and attack surface.
- Scope assets (URL, host, IP, API, repository, cloud account, mobile app, thick client, other).
- Risks with likelihood and impact (0–9); severity is calculated automatically.
- Assumptions and contacts.

Open an engagement to access its tabs.

### Test tracker

- **From templates** adds reusable test cases by domain.
- **Custom test** creates a blank test case.
- Change status inline, edit the approach, result, and notes, and search/filter large trackers.
- Reorder tests with the up/down controls (filters must be cleared while reordering).
- The current user is recorded as the assignee automatically; there is no manual assignee field.
- Keep status, result, evidence, and linked findings separate. A passed test is not a finding.

### Findings

- Create a finding with probability and impact; the server computes severity using the approved
  matrix.
- Add OWASP/CWE references, remediation, owner, retest status, and linked test cases.
- Use severity consistently: Informational, Low, Medium, High, Critical.

### Evidence

- Attach files up to 8 MB. Content is encrypted before it is written to disk.
- Add a caption and optionally link the evidence to a finding or test case.
- Preview images and text, download the decrypted file, or delete the evidence.
- Evidence is stored under the app data directory, never in the repository.

### AI assistant

- Add providers in **Settings → AI providers**: Ollama/local, OpenAI-compatible, Anthropic, or Azure
  OpenAI. Credentials are encrypted and never returned to the interface.
- Set the per-engagement **AI policy**: Off, Local models only (default), or Approved cloud
  providers.
- Open the **AI** tab, choose a provider and purpose, paste the context you intend to share, and
  select **Preview redactions**.
- Review the exact redacted payload. Sending requires a fresh, single-use consent that expires in
  five minutes.
- Responses are drafts and are not saved to the engagement. The audit log stores provider, model,
  purpose, prompt/response hashes, token usage, and redaction count — not message content.
- Agent skills are available on the engagement overview (summary, attack surface, assumptions), the
  test tracker (**Generate tests**, and **AI draft** per test for approach or finding). Each skill
  previews the server-built context and requires an explicit **Apply**. See `docs/ai-agent.md`.
- Redaction is best-effort. Local-only permits localhost endpoints, which may themselves proxy to a
  cloud service; use a trusted local setup.
- Consumer AI subscriptions generally do not expose API access. Use official APIs, cloud accounts,
  or local models.
- **Ollama troubleshooting:** run `ollama pull llama3.2` (or another model) before selecting it.
  Verify with `ollama list`. In the provider editor, **Load models** lists installed models. An
  HTTP 404 at `/api/chat` means the model is not installed; the app now shows the provider error.

### Plugins

- Add publisher Ed25519 public keys in **Settings → Plugins**.
- Install a signed `.ttplugin` package. Packages signed by an untrusted publisher are rejected.
- Open an engagement's **Plugins** tab, choose an enabled plugin, and run it.
- Suggested test cases can be added with one click; suggested findings are display-only.
- Plugins never receive evidence content and cannot write to the database. The WASM file hash is
  verified before each run.
- See `examples/plugins/scope-helper/` and `docs/plugin-sdk.md` for the package format, ABI, and
  signing CLI.

### Encrypted bundles

- **Export bundle** on an engagement creates a `.ttbundle` encrypted with a passphrase you choose.
- Bundles include scope, risks, assumptions, contacts, test cases, findings, evidence, and
  optionally generated documents.
- **Import bundle** on the Engagements page creates a new engagement with new identifiers and
  remapped test/finding/evidence links. Existing data is not modified.
- Import verifies the encrypted envelope, file hashes, manifest signature, and record limits.
- The manifest signature is self-signed and shown as a fingerprint. It proves integrity, not the
  sender's identity; confirm the fingerprint out of band.
- Snapshots, approvals, audit history, AI data, and plugins are not included in bundles.
- Limits: 25 MB of bundle content, 40 MB bundle file. Passphrases cannot be recovered.
- Full format and security details: `docs/bundles.md`.

### Review and sign-off

- **Create snapshot** captures the engagement, scope, risks, assumptions, contacts, tests, findings,
  and evidence metadata at the current version.
- **Sign off** records an approval, change request, or rejection against a snapshot.
- Signatures are verified against the snapshot hash; editing the database invalidates them.
- In the current single-profile build, the local Administrator can sign its own snapshot. Multiple
  local profiles and separation of duties are planned.

### Documents

- Generate a **test plan**, **test-case tracker**, **findings register**, **executive summary**, or
  **technical report** from the current engagement data.
- Choose **PDF** or **DOCX** for reports. **XLSX** is available for the test-case tracker and
  findings register.
- Generated files are encrypted in the app data directory. Download decrypts them locally.
- PDF rendering uses embedded fonts; content in scripts not covered by those fonts may not render.
- Deleting a document removes its encrypted file; deleting an engagement removes all of its
  generated documents.

### Audit

- Every mutation writes an audit event in the same database transaction.
- **Verify chain** recomputes the hash chain and reports tampering.
- Events include actor, action, entity, timestamp, and metadata.

### Locking

Use the lock button in the sidebar to clear keys from memory. The app also auto-locks after 15
minutes of inactivity.

## Security model

- The React webview never receives the master key, database key, or signing private key.
- All validation, authorization, and severity calculations happen in Rust.
- The database is encrypted with SQLCipher; evidence blobs and generated documents are encrypted with per-engagement keys.
- AI provider keys are encrypted with the vault key and only decrypted in memory for an approved request.
- AI prompts require a redaction preview and a server-side one-time consent; cloud providers are blocked unless the engagement policy allows them.
- Agent skills receive only scoped engagement context, return validated JSON, and cannot write until the user applies the proposal.
- Plugins are signed by trusted publishers, verified on install, hash-checked before every run, and executed with no filesystem, network, or WASI access.
- Bundles use Argon2id-derived keys and XChaCha20-Poly1305 encryption; manifests are Ed25519-signed and all files are hash-verified on import.
- The audit log is append-only and hash-chained.
- The Tauri capability set is minimal; there are no shell or HTTP webview plugins, no remote
  content, and a strict Content Security Policy.
- No telemetry. AI network calls are explicit, redacted, policy-controlled, and audited. Plugins have no network access.

Local-first limitation: a device owner can read the local database. Local roles prevent mistakes and
structure review, but they are not hostile-user isolation. Cryptographic sign-offs and the audit
chain make changes verifiable when data is exported in future versions.

## Data, backups, and reset

TestTracker stores its data in the platform application data directory:

- Linux: `~/.local/share/com.testtracker.desktop`
- macOS: `~/Library/Application Support/com.testtracker.desktop`
- Windows: `%APPDATA%\com.testtracker.desktop`

The directory contains `vault.json`, the encrypted database, encrypted evidence, generated documents,
installed plugins, and WebKit cache files. Engagement bundles are user-selected downloads and are not
stored in the app data directory. Back up the entire directory while the app is closed. A built-in encrypted backup workflow is
planned.

To reset the app, close it and delete that directory. **This destroys the vault and all evidence.**

## Linux troubleshooting

TestTracker sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` on Linux before WebKit initializes. This avoids
a known WebKitGTK + Wayland/NVIDIA freeze where the window stops repainting and accepting input.

To opt out or test another workaround:

```bash
WEBKIT_DISABLE_DMABUF_RENDERER=0 bun run tauri dev
WEBKIT_DISABLE_COMPOSITING_MODE=1 bun run tauri dev
GDK_BACKEND=x11 bun run tauri dev
```

If the interface still freezes, close the app, remove `~/.cache/fontconfig`, run `fc-cache -f`, and
restart. This resolves a separate WebKitGTK fontconfig loop reported on some Linux systems.

## Development

```bash
bun run dev              # Vite only
bun run tauri dev        # desktop app
bun run check            # ESLint + TypeScript + Vitest
bun run build            # typecheck + production frontend build
bun run test             # Vitest
bun run lint
bun run typecheck
bun run format:check

cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml --check

# Plugin signing tool
cargo run --manifest-path src-tauri/Cargo.toml --bin ttplugin -- keygen publisher.key
cargo run --manifest-path src-tauri/Cargo.toml --bin ttplugin -- sign \
  examples/plugins/scope-helper publisher.key scope-helper.ttplugin
```

The first Rust build compiles bundled SQLCipher, vendored OpenSSL, and Typst, and can take several minutes.

CI runs frontend checks, Rust formatting/clippy/tests, and a dependency audit on Linux.

## Release hardening

Local verification:

```bash
bun run check
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo audit --file src-tauri/Cargo.lock
bun audit --audit-level=high
bun run tauri build --debug --no-bundle
```

CI runs frontend checks, Rust formatting/clippy/tests, a full Tauri debug build, dependency audits,
and a verified-secret scan. The release workflow builds Linux, Windows, and macOS bundles and
uploads them as workflow artifacts on `v*` tags.

**Signing status:** release artifacts are currently unsigned and no updater endpoint is configured.
Do not distribute public builds until code signing, updater signing, and release-key management are
in place. Signing keys must be stored in CI secrets, never committed. See `SECURITY.md` and
`.cargo/audit.toml` for the dependency-audit exceptions and rationale.

## Project layout

```
src/                 React UI (features, components, hooks, lib)
src-tauri/src/
  bundles/           encrypted engagement bundle format, payload, and import/export
  commands/          Tauri IPC commands
  db/                schema, migrations, repositories
  domain/            models, severity, RBAC, signing
  plugins/           manifest, package signing, WASM runtime
  security/          crypto, vault, identity, audit
  bin/ttplugin.rs    plugin signing CLI
  state.rs           session, auto-lock, AI consent
docs/                architecture, threat model, data model, plugin SDK, AI policy
reference/           ignored, local-only source material
```

## Roadmap

- Full-vault encrypted backup/restore and optional multi-device sync.
- Multiple local profiles and separation of duties.
- Multi-step AI tool loops, MCP integration, streaming, and optional encrypted prompt retention.
- Plugin process runners, schema-driven UI contributions, and a plugin registry.

## Confidentiality

This repository must not contain client-identifying material, assessment evidence, generated
reports, exported bundles, databases, or credentials. `reference/` is git-ignored and local-only.
Use synthetic data in tests and examples.
