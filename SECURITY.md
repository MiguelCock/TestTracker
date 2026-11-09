# Security policy

TestTracker handles confidential penetration-testing data. Security issues are treated as high
priority.

## Reporting

Do not open a public issue for a vulnerability. Use GitHub's private vulnerability reporting:

https://github.com/MiguelCock/TestTracker/security/advisories/new

Include the affected version, reproduction steps, impact, and any relevant logs. Do not include
client data, evidence, credentials, or production bundles.

## Supported versions

Only the current `0.1.x` line is supported while the application is pre-release.

## Security model

- Vault: Argon2id-derived key, SQLCipher database, per-engagement evidence/document keys.
- Audit: append-only BLAKE3 hash chain, verified in-app.
- Permissions: Rust-enforced local roles; no secrets are sent to the webview.
- AI: encrypted provider keys, local-only default policy, redaction preview, one-time server-side
  consent, no message-content persistence.
- Plugins: trusted-publisher Ed25519 signatures, WASM hash verification, no WASI/filesystem/network
  imports, fuel/memory limits, suggestions only.
- Bundles: Argon2id + XChaCha20-Poly1305, signed manifest, per-file hashes, new-engagement import.
- MCP bridge: off by default, loopback-only, bearer token, Host/Origin validation, read/draft tools
  only, explicit in-app approval for every proposal, stops on vault lock. The token grants access to
  all engagements while running; treat it as a high-value local credential.
- Assistant: fixed server-validated actions, explicit apply, bounded context, redacted text
  previews, per-provider vision opt-in, image metadata stripping, and audit metadata only.
- Template bundles: validated JSON, size/hash checks, domain/text limits, and local ID remapping.

## Release status

- Release artifacts are currently **unsigned**; code signing and an updater endpoint are not
  configured yet. Do not distribute builds publicly until signing is configured.
- Signing keys and update keys must live in CI secrets, never in the repository.
- CI runs lint, typecheck, tests, `cargo clippy -D warnings`, `cargo audit`, `bun audit`, a Tauri
  build, and a verified-secret scan.

## Dependency exceptions

`.cargo/audit.toml` documents ignored advisories with narrow applicability: Typst's bibliography
XML parser (not used with untrusted input) and docx-rs's ZIP writer (no archive extraction).
`cargo audit` still reports unmaintained/unsound warnings without failing the build. Review these
exceptions whenever Typst or docx-rs updates.
