# AGENTS.md

## Repo purpose

Build **TestTracker**, an internal application that standardizes how penetration-testing engagements are planned, executed, tracked, reviewed, and reported. It replaces manual spreadsheets with engagements, scope/attack surface, reusable test cases, structured findings, evidence, approvals, audit history, versioning, and PDF/DOCX/XLSX exports.

## Current state

Tauri 2 + React 19 + TypeScript desktop app, local-first and encrypted. Implemented: vault/key management, profiles, engagements, scope/risks/assumptions/contacts, test-case tracker with domain templates, structured findings, severity matrix, encrypted evidence store, immutable snapshots, Ed25519 sign-offs with hash verification, audit hash chain, auto-lock, document generation, AI providers/chat/agent skills, signed plugins, encrypted bundles, assessment import (XLSX/XLSM, DOCX, PDF, text), local MCP server bridge, and strict CSP. Planned: MCP client/tool calling, encrypted multi-profile separation.

Use **Bun** (lockfile is `bun.lock`). Do not introduce npm/yarn/pnpm.

## Commands

```bash
bun install
bun run tauri dev          # desktop app
bun run dev                # Vite only
bun run check              # lint + typecheck + Vitest
bun run build              # tsc --noEmit + vite build
bun run lint
bun run typecheck
bun run test
bunx vitest run src/test/severity.test.ts   # single frontend test

cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml severity   # filter
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo audit --file src-tauri/Cargo.lock
bun audit --audit-level=high
bun run tauri build --debug --no-bundle

# Plugin signing CLI
cargo run --manifest-path src-tauri/Cargo.toml --bin ttplugin -- keygen publisher.key
cargo run --manifest-path src-tauri/Cargo.toml --bin ttplugin -- sign examples/plugins/scope-helper publisher.key scope-helper.ttplugin
```

First Rust build is slow: `rusqlite` compiles bundled SQLCipher + vendored OpenSSL. Linux needs `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `librsvg2-dev`, `libayatana-appindicator3-dev`, `libxdo-dev`, `libssl-dev`, `patchelf`, `build-essential`, `pkg-config`.

## Architecture rules

- All secrets and trust decisions live in Rust. The React webview never receives the master key, DB key, provider credentials, or signing private key.
- Every Tauri command validates input, checks the unlocked session (and role for writes), and returns safe errors. Do not move validation or authorization into the UI.
- Every mutation appends an audit event in the same SQLite transaction. Do not write a mutation path that skips `security/audit.rs`.
- Severity is computed server-side from probability/impact (`domain/severity.rs`); the frontend mirror is preview-only.
- Findings are records, never booleans. Legacy `Finding = No` means "no finding linked". Keep `status`, `result`, evidence, and linked findings separate.
- Use parameterized SQL only. Migrations are versioned in `src-tauri/src/db/schema.rs` via `PRAGMA user_version`.
- No remote content, `eval`, or shell/HTTP Tauri plugins in the webview. Keep CSP strict and capabilities minimal. AI network calls happen in Rust only.
- AI provider API keys are encrypted with the master key and never returned to the UI. Prompts require a server-held redacted preview and one-time consent; never add a command that sends an arbitrary frontend payload directly to a provider. Default policy is local-only; enforce policy in Rust on preview and again on send.
- Plugins are untrusted. Verify the publisher signature at install, store and verify the WASM hash before execution, and keep the runtime free of WASI/filesystem/network imports. Plugin output is untrusted; never auto-apply it. Do not add arbitrary JavaScript/HTML plugin UI to the main webview.
- The MCP server is opt-in, binds only to loopback, requires a bearer token, and exposes no direct write tools. All MCP proposals require in-app approval. Never log the token or full tool payloads. Stop the server and clear proposals on lock; do not extend the session from MCP calls.
- Bundles are passphrase-encrypted with Argon2id + XChaCha20-Poly1305. Never weaken the KDF bounds, skip manifest signature verification, or import without checking every file hash and remapping identifiers. Bundles must create new records, never merge into existing engagements.
- Use `#![deny(unsafe_code)]` and avoid `unsafe` in app code. The only exception is the Linux WebKit DMABUF workaround in `main.rs`, which is explicitly allowed.
- Do not enable the updater or distribution signing without real keys and an endpoint. Never commit signing keys; use CI secrets. Release artifacts are currently unsigned.

Key files: `src-tauri/src/security/` (crypto, vault, identity, audit), `src-tauri/src/state.rs` (session/auto-lock), `src-tauri/src/mcp/` (local MCP bridge), `src-tauri/src/db/` (schema/repos), `src-tauri/src/commands/` (IPC, including `agent.rs` and `mcp.rs`), `src/features/ai/` and `src/features/mcp/` (UI), `docs/` (architecture, threat model, data model, plugin SDK, AI policy, MCP bridge).

## Confidential material

- `reference/` is git-ignored and contains client-identifying and offensive-security source documents. Never commit it, never use `git add -f`, and never copy its contents into tracked files, tests, fixtures, commit messages, docs, or logs.
- Never commit evidence, generated reports, exports, bundles, databases, or `.env` files. `.gitignore` covers `reference/`, `evidence/`, `reports/`, `exports/`, `*.ttbundle`, `*.ttplugin`, `vault.json`, and `*.db*`.
- Use synthetic data in tests and examples. Do not name clients in code or docs.
- A fresh clone will not contain `reference/`; that is expected, not missing requirements.
- New sensitive files at the repo root are only partially covered by `.gitignore`; move them into `reference/` immediately.

## Reference inputs (read-only)

`reference/` holds the legacy tracker, findings register (macro-enabled; vulnerability catalog, inventory, Probability×Impact matrix, OWASP/CWE mapping), per-domain checklists, methodology, engagement lifecycle, report-review criteria, and tools inventory. Never edit these. List the folder locally for exact filenames; do not enumerate client filenames in tracked files.

Reading formats: `unzip -p file.xlsx xl/sharedStrings.xml`, `pdftotext -layout file.pdf -`, `soffice --headless --convert-to ...`. Not installed: `openpyxl`, `python-docx`, `pandas`, `pdfplumber`.

## Testing and runtime gotchas

- Rust unit tests cover crypto, vault, identity, audit chain, severity, RBAC, AI redaction/provider parsing/URL policy/agent proposal validation, plugin package signing/ZIP safety/WASM fuel/memory/export checks, bundle encryption/ZIP validation, MCP token/auth/approval/proposal behavior, and a full encrypted-DB flow including snapshots/approvals (`src-tauri/src/test_flow.rs`). Add tests for every new security or domain rule.
- Frontend tests: Vitest + Testing Library (`src/test/`). Use synthetic fixtures only.
- Before finishing a task run `bun run check`, `cargo clippy ... -- -D warnings`, `cargo test`, `cargo audit`, and `bun audit`.
- Do not use `reference/` data in automated tests or CI.
- `tauri build --debug` embeds the current `dist/`. Run `bun run build` first when testing the packaged frontend, or use `tauri dev` for live Vite. Otherwise you may test a stale UI.
- Linux: `src-tauri/src/main.rs` sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` before WebKit starts to avoid a WebKitGTK + Wayland/NVIDIA freeze. Respect an existing override; do not remove this without testing on Wayland.
- E2E smoke tests use `tauri-driver` + `WebKitWebDriver` with a throwaway `XDG_DATA_HOME`; never point them at a real vault.

## Git

- Remote `origin` is configured; branch `main`; the repository has zero commits.
- No PR/release conventions exist yet. Do not commit unless the user explicitly asks.
- Before any commit run `git status` and `git add --dry-run .` to confirm `reference/`, `dist/`, `src-tauri/target/`, and generated output are excluded.
