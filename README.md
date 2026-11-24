# TestTracker

TestTracker is a local-first desktop application for planning, executing, reviewing, and reporting
penetration-testing engagements. It replaces spreadsheet workflows with a structured, encrypted
workspace for engagements, scope, tests, findings, files, and audit history.

Built with **Rust + GPUI** (gpui-kit) as a native desktop application. There is no webview, no
remote content, and no JavaScript UI runtime.

## Features

- **Encrypted local vault** — Argon2id passphrase derivation and a SQLCipher database.
- **Engagements** — project metadata, scope assets, attack surface, risks, assumptions, contacts,
  status, and versioning.
- **Test tracker** — a large reusable template library with curated bundles, cross-team JSON
  import/export, status, results, and notes.
- **Findings** — structured records with server-computed Probability × Impact severity, OWASP/CWE
  references, remediation, retest status, owner, and linked test cases.
- **Encrypted files workspace** — virtual folders for Postman collections, Burp exports, Procmon
  captures, logs, screenshots, and other pentest artifacts. Files are encrypted at rest and exposed
  to the agent as a read-only directory.
- **Template library** — browse seeded templates, create custom templates, manage bundles, and
  apply them to engagements.
- **Document generation** — styled test plan, test-case tracker, findings register, executive
  summary, and technical report as PDF, DOCX, or XLSX. Generated files are encrypted at rest.
- **Plan/Build agent** — one policy-controlled agent with a read-only Plan mode and a Build mode
  that turns work into explicit change proposals. It reads engagement context, tests, findings, and
  the virtual file tree through read-only tools. Every write needs explicit approval. Local-only is
  the default.
- **Assessment import** — create a new engagement from an XLSX/XLSM tracker, DOCX, PDF, Markdown,
  JSON, or text assessment. Tracker spreadsheets are parsed deterministically; other formats can use
  AI to produce a reviewable draft.
- **Local MCP bridge** — opt-in, loopback-only Model Context Protocol server with a bearer token,
  read tools, draft-only proposal tools, and in-app approval. It stops when the vault locks.
- **Signed plugins** — read-only WebAssembly modules from trusted publishers, executed in a fuel-
  and memory-limited sandbox.
- **Encrypted bundles** — passphrase-encrypted `.ttbundle` export/import for whole engagements,
  including engagement files and generated documents, with signed manifests and hash verification.
- **Audit history** — append-only hash chain with tamper detection.
- **Auto-lock** — keys are cleared from memory after inactivity.

## Requirements

- Rust 1.100+ (the repository pins it in `rust-toolchain.toml`)
- Linux build dependencies:
  `build-essential pkg-config libssl-dev libasound2-dev libdbus-1-dev libfontconfig-dev
  libglib2.0-dev libva-dev libvulkan1 libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev
  libx11-xcb-dev libxcb1-dev libxcb-randr0-dev libxcb-shape0-dev libxcb-xfixes0-dev libzstd-dev`
- A Vulkan-capable GPU driver, plus Wayland or X11. GPUI renders natively; there is no WebKit or
  WebView2 dependency.
- macOS and Windows use the platform Rust toolchain and GPU stack.

## Install and run

```bash
cargo run -p testtracker-app          # development build
cargo build --release -p testtracker-app
./target/release/testtracker
```

The application data directory is:

- Linux: `~/.local/share/com.testtracker.desktop`
- macOS: `~/Library/Application Support/com.testtracker.desktop`
- Windows: `%APPDATA%\com.testtracker.desktop`

Override it for testing with `TESTTRACKER_DATA_DIR=/tmp/tt-data`.

## First run

1. Start TestTracker.
2. Create a display name and a passphrase of at least 12 characters.
3. Store the passphrase safely. It cannot be recovered; losing it means losing access to the vault.
4. The app creates an encrypted database and a local Administrator profile with an Ed25519 identity
   key.

The vault is local to this device. Nothing is uploaded.

## Using TestTracker

### Dashboard

Shows active engagements, open findings, test completion, and recent engagements.

### Engagements

Create an engagement from **New engagement**. Set the project name, client code, status, dates,
application summary, attack surface, scope, risks, assumptions, and contacts. Scope and context can
be refined with **Edit scope & context** (pipe-separated lines).

### Tests & Findings

The tracker and findings register share one workbench:

- Tests are the primary rows; each row shows status, category, linked finding count, and actions.
- Expand a test to see its linked findings and add a new finding directly to it.
- Standalone findings appear in a separate section.
- Shared search, test-status, and severity filters.
- Reorder tests with the up/down controls when no filters are active.

A passed test is not a finding. Keep test status/result and structured findings separate.

### Files

Files is the engagement's encrypted artifact workspace:

- Virtual folders such as `postman/`, `burp/`, `procmon/`, `logs/`, and `notes/`.
- Upload, preview extracted text, download, and delete.
- Supports Postman, Burp XML, HAR, Procmon CSV/XML, OpenAPI, GraphQL, Nmap XML, JSON, CSV, XML,
  YAML, Markdown, logs, text, images, PDFs, and other binary artifacts (metadata-only when text
  extraction is unavailable).
- The agent reads files by virtual path with read-only directory/search tools.

Legacy evidence records are migrated into Files automatically on unlock.

### Agent

The Agent tab is the single AI surface:

- **Plan** reads engagement context and files, then produces a plan without applying changes.
- **Build** turns approved work into proposals with before/after previews. Nothing is applied until
  you approve it.
- Provider credentials, engagement AI policy, redaction preview, and consent apply to both modes.
- Files in context are shown in the side panel; all engagement files are available by default.
- Build proposals can create or update tests, findings, scope, and the application summary.

### Templates

The global Templates page lets you:

- Browse and search seeded templates by domain and category.
- Create, edit, and delete custom templates.
- Apply a template to a selected engagement.
- Manage bundles: apply, import, export, and delete.
- Import a template bundle JSON from another team.

### Documents

Generate styled PDF, DOCX, and XLSX reports. Reports now cover the application summary, scope, tests,
and findings; evidence and review sections were removed.

### MCP bridge

Start the loopback bridge from **MCP**. Reveal or regenerate the bearer token, review draft proposals,
and approve or reject them in the app. The bridge stops when the vault locks.

### Bundles and imports

- **Export bundle** on an engagement creates a passphrase-encrypted `.ttbundle`.
- **Import bundle** on the Engagements page creates a new engagement with new identifiers.
- **Import assessment** extracts XLSX/DOCX/PDF/text documents into a reviewable draft.

## Security model

- The UI is native Rust, and all database, key, and provider access lives in `testtracker-core`.
  View code calls the service layer; it does not open SQLite or read vault keys.
- All validation, authorization, and severity calculations happen in Rust.
- The database is encrypted with SQLCipher; engagement files and generated documents are encrypted
  with per-engagement keys.
- AI provider keys are encrypted with the vault key and only decrypted in memory for an approved
  request.
- AI prompts require a redaction preview and a server-side one-time consent; cloud providers are
  blocked unless the engagement policy allows them.
- Agent and MCP actions return validated proposals and cannot write until the user applies them.
- Engagement files are encrypted at rest, engagement-scoped, parsed by bounded Rust parsers, and
  readable only through explicitly approved read-only tools.
- Image files are stored in Files; text extraction is metadata-only and images are never sent to a
  provider without an explicit request.
- The MCP bridge binds only to loopback, requires a bearer token, validates Host/Origin, exposes no
  direct write tools, and stops on vault lock.
- Assessment import treats document text as untrusted and only writes after draft approval.
- Plugins are signed by trusted publishers, verified on install, hash-checked before every run, and
  executed with no filesystem, network, or WASI access.
- Bundles use Argon2id-derived keys and XChaCha20-Poly1305 encryption; manifests are Ed25519-signed
  and all files are hash-verified on import.
- The audit log is append-only and hash-chained.
- No telemetry. AI network calls are explicit, redacted, policy-controlled, and audited. Plugins have
  no network access.

Local-first limitation: a device owner can read the local database. Local roles prevent mistakes and
structure review, but they are not hostile-user isolation. The append-only audit chain makes changes
verifiable locally and when engagement data is exported.

## Data, backups, and reset

The application data directory contains `vault.json`, the encrypted database, encrypted engagement
files, generated documents, and installed plugins. Back up the entire directory while the app is
closed; `vault.json` and `testtracker.db` must always be restored together.

See [`docs/operations.md`](docs/operations.md) for the backup, verification, restore, and rollback
procedure, and [`docs/migration-policy.md`](docs/migration-policy.md) for the forward-only schema
policy. To reset, close the app and delete the directory. **This destroys the vault and all
files.**

## Linux notes

GPUI uses native Wayland/X11 windowing. If a window does not appear, verify that a Vulkan driver is
installed (`vulkaninfo`) and try X11:

```bash
WAYLAND_DISPLAY='' cargo run -p testtracker-app
```

The old WebKitGTK DMABUF workaround is no longer used.

## Development

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit
cargo build --release -p testtracker-app

# Migration fixture and copied-data-directory rehearsal
./scripts/migration-rehearsal.sh

# Plugin signing tool
cargo run -p testtracker-core --bin ttplugin -- keygen publisher.key
cargo run -p testtracker-core --bin ttplugin -- sign \
  examples/plugins/scope-helper publisher.key scope-helper.ttplugin
```

The first build compiles bundled SQLCipher, vendored OpenSSL, Typst, and GPUI; it can take several
minutes. CI runs formatting, clippy, tests, a release build, dependency audit, and secret scanning on
Linux. Release artifacts are currently unsigned.

## Repository layout

```
crates/core/    domain, database, security, AI, MCP, plugins, bundles, imports, documents, services
crates/app/     GPUI application, theme, components, views
crates/core/src/bin/ttplugin.rs   plugin signing CLI
docs/           architecture, threat model, data model, plugin SDK, AI policy, MCP bridge
integrations/opencode/  OpenCode Plan/Build agents and MCP config
examples/       plugin examples
reference/      ignored local-only source material
```
