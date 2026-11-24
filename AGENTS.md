# AGENTS.md

## Repo purpose

Build **TestTracker**, an internal application that standardizes how penetration-testing engagements
are planned, executed, tracked, reviewed, and reported. It replaces manual spreadsheets with
engagements, scope, reusable test cases, structured findings, encrypted files, audit history,
versioning, and PDF/DOCX/XLSX exports.

## Current state

Native Rust desktop app built with **GPUI** and **gpui-kit**. No Tauri, no React, no webview, no
JavaScript UI runtime. Implemented: vault/key management, profiles, engagements with scope and
application summary, unified Tests & Findings, custom test templates and curated bundles, encrypted
virtual Files workspace, audit hash chain and Activity view, auto-lock, styled document generation,
AI providers, one Plan/Build agent with read-only file tools and approved change proposals, OpenCode
agent pack, signed plugins, encrypted bundles, assessment import, local MCP server bridge, and native
file dialogs. Planned: MCP client/tool calling, encrypted multi-profile separation.

Use **Cargo** only. There is no npm/yarn/pnpm/Bun toolchain.

## Commands

```bash
cargo run -p testtracker-app                  # desktop app
cargo build --release -p testtracker-app
cargo check --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p testtracker-core severity       # filter
cargo audit
cargo build -p testtracker-app                # compile-check the UI
./scripts/migration-rehearsal.sh              # migration + copied-data-dir rehearsal

# Plugin signing CLI
cargo run -p testtracker-core --bin ttplugin -- keygen publisher.key
cargo run -p testtracker-core --bin ttplugin -- sign examples/plugins/scope-helper publisher.key scope-helper.ttplugin
```

First Rust build is slow: bundled SQLCipher + vendored OpenSSL, Typst, and GPUI. Linux needs
`build-essential pkg-config libssl-dev libasound2-dev libdbus-1-dev libfontconfig-dev libglib2.0-dev
libva-dev libvulkan1 libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev
libxcb1-dev libxcb-randr0-dev libxcb-shape0-dev libxcb-xfixes0-dev libzstd-dev`, plus a Vulkan driver and Wayland/X11.

Windows: the vendored OpenSSL build requires Perl. Git for Windows ships one at `C:\Program Files\Git\usr\bin\perl.exe`; add that directory to `PATH` before `cargo check`/`cargo test`/`tauri dev` when `perl` is not already available.

## Architecture rules

- Workspace layout: `crates/core` is GUI-free; `crates/app` is the GPUI binary. Only `core` touches
  the database, vault, keys, AI network calls, MCP server, plugins, bundles, imports, and document
  renderers.
- View code calls `testtracker_core::services::*` through `Backend::call`; it must never open
  `rusqlite`, read `vault.json`, decrypt keys, or hold a session lock in `render`. Do not block the
  GPUI main thread with database, CPU, or network work.
- `AppState`, `UnlockedSession`, master/database/provider/signing keys, and the SQLCipher connection
  stay `pub(crate)` in `core`. Expose new operations through services, not by widening state.
- Every core operation now lives in `crates/core/src/services/`; keep validation, RBAC,
  transactions, and audit writes there. Every mutation appends an audit event in the same SQLite
  transaction via `security/audit.rs`. Never add a mutation path that skips it.
- Severity is computed in Rust from probability/impact (`domain/severity.rs`). Do not duplicate the
  matrix in the UI as authoritative logic.
- Findings are records, never booleans. Legacy `Finding = No` means "no finding linked". Keep
  `status`, `result`, files, and linked findings separate.
- Use parameterized SQL only. Migrations are versioned in `crates/core/src/db/schema.rs` via
  `PRAGMA user_version`.
- Keep `#![deny(unsafe_code)]` in app/core. Do not add `unsafe` for UI convenience.
- GPUI is pre-1.0. Keep `gpui-kit` and GPUI versions pinned; do not mix the crates.io `gpui` crate
  with gpui-kit's re-exported GPUI. Use `gpui_kit::gpui::*` and `gpui_kit::component::*`.
- Native file dialogs, save dialogs, and clipboard access happen in the app; validate paths, sizes,
  and extensions in `core` services before storing or executing anything.
- AI provider API keys are encrypted with the master key and never returned to the UI. Prompts
  require a server-held redacted preview and one-time consent; never add a path that sends an
  arbitrary UI payload directly to a provider. Default policy is local-only; enforce policy on
  preview and again on send.
- The agent may select only server-approved actions. Validate action type, target ownership,
  payload shape, and policy before storing a proposal. Never auto-apply assistant or agent output;
  image input requires a provider explicitly marked vision-capable and an explicit file action.
- Imported template bundles and engagement files are untrusted data. Validate format, size, content
  hash, IDs, domains, and text limits; remap identifiers; never execute or render imported HTML.
  Files are encrypted at rest, scoped to one engagement, parsed in bounded Rust parsers, and exposed
  only through read-only virtual-path tools requiring explicit permission.
- Plugins are untrusted. Verify the publisher signature at install, store and verify the WASM hash
  before execution, and keep the runtime free of WASI/filesystem/network imports. Plugin output is
  untrusted; never auto-apply it. Do not add arbitrary JavaScript/HTML plugin UI.
- The agent has one Plan mode and one Build mode. Plan tools are read-only. Build tools may only
  create `AgentChange`/proposal records; never apply them without explicit user action. Files are
  read-only to the agent; no shell, arbitrary filesystem paths, or file writes.
- The MCP server is opt-in, binds only to loopback, requires a bearer token, and exposes no direct
  write tools. All MCP proposals require in-app approval. Never log the token or full tool payloads.
  Stop the server and clear proposals on lock; do not extend the session from MCP calls.
- Bundles are passphrase-encrypted with Argon2id + XChaCha20-Poly1305. Never weaken the KDF bounds,
  skip manifest signature verification, or import without checking every file hash and remapping
  identifiers. Bundles must create new records, never merge into existing engagements.
- The UI is in-process Rust and therefore trusted. Preserve defense-in-depth with the core/app crate
  boundary and by keeping secrets out of view structs. Do not claim the UI is an OS security
  boundary. If hostile-user isolation is ever required, split `core` into a separate process.
- Data directory: `~/.local/share/com.testtracker.desktop` on Linux (identifier-based). Do not
  change it without a migration; existing vaults live there.
- Do not enable an updater or distribution signing without real keys and an endpoint. Release
  artifacts are currently unsigned.

Key files: `crates/core/src/security/` (crypto, vault, identity, audit),
`crates/core/src/state.rs` (session/auto-lock), `crates/core/src/mcp/` (local MCP bridge),
`crates/core/src/db/` (schema/repos), `crates/core/src/services/` (IPC-free service layer),
`crates/core/src/ai/`, `crates/app/src/app.rs` (root view), `crates/app/src/backend.rs`
(tokio/GPUI bridge), `crates/app/src/ui.rs` (widgets), `docs/` (architecture, threat model,
data model, plugin SDK, AI policy, MCP bridge, template library, operations, migration policy,
information architecture, visual baseline).

## Confidential material

- `reference/` is git-ignored and contains client-identifying and offensive-security source
  documents. Never commit it, never use `git add -f`, and never copy its contents into tracked files,
  tests, fixtures, commit messages, docs, or logs.
- Never commit evidence, generated reports, exports, bundles, databases, or `.env` files.
  `.gitignore` covers `reference/`, `evidence/`, `reports/`, `exports/`, `*.ttbundle`, `*.ttplugin`,
  `vault.json`, and `*.db*`.
- Use synthetic data in tests and examples. Do not name clients in code or docs.
- A fresh clone will not contain `reference/`; that is expected, not missing requirements.
- New sensitive files at the repo root are only partially covered by `.gitignore`; move them into
  `reference/` immediately.

## Reference inputs (read-only)

`reference/` holds the legacy tracker, findings register (macro-enabled), per-domain checklists,
methodology, engagement lifecycle, report-review criteria, and tools inventory. Never edit these.
List the folder locally for exact filenames; do not enumerate client filenames in tracked files.

Reading formats: `unzip -p file.xlsx xl/sharedStrings.xml`, `pdftotext -layout file.pdf -`,
`soffice --headless --convert-to ...`. Not installed: `openpyxl`, `python-docx`, `pandas`,
`pdfplumber`.

## Testing and runtime gotchas

- Rust unit tests cover crypto, vault, identity, audit chain, severity, RBAC, AI redaction/provider
  parsing/URL policy/agent proposal validation/assistant action parsing/image preparation/reference
  file parsers/tool permissions, plugin package signing/ZIP safety/WASM fuel/memory/export checks,
  bundle encryption/ZIP validation, MCP token/auth/approval/proposal behavior, template bundle
  integrity, file migration, migration fixtures (v1→latest, v9→latest, idempotency), copied data
  directory restore, and a full encrypted-DB flow (`crates/core/src/test_flow.rs`). Add tests for
  every new security or domain rule.
- UI tests use `gpui-kit`'s test support (`#[gpui_kit::test]`) where practical. Keep UI tests in
  `crates/app`; do not depend on a real vault or `reference/`.
- Before finishing a task run `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and
  `cargo audit`.
- GPUI renders with Vulkan. If the window does not appear, check `vulkaninfo`; try
  `WAYLAND_DISPLAY=''` for X11. The old WebKit DMABUF workaround is gone.
- `panic = "abort"` is set for release; a panic terminates the app. Test debug builds before release.
- The app locks the vault after 15 minutes. Services enforce expiry on every call; the UI also runs a
  15-second ticker and throttles `touch_session` to once per minute.

## Git

- Remote `origin` is configured; branch `main`.
- No PR/release conventions exist yet. Do not commit unless the user explicitly asks.
- Before any commit run `git status` and `git add --dry-run .` to confirm `reference/`, `target/`,
  and generated output are excluded.
