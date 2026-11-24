# Threat model

## Assets

- Engagement scope, attack surface, risks, and assumptions.
- Test cases, results, and engagement files.
- Findings, severity, remediation, and retest status.
- Audit history and version history.
- Provider credentials, the MCP bearer token, and signing private keys.
- The integrity of the local database and generated reports.

## Adversaries

- A thief or unauthorized user with access to the device.
- A malicious or compromised third-party plugin.
- A compromised or untrusted AI provider.
- Prompt-injection content inside engagement files or imported engagement data.
- A supply-chain attacker in Cargo or a future updater.
- A local user making accidental or unauthorized changes.
- A malicious local process or website attempting to reach a loopback MCP server.
- A cloud-backed MCP client receiving data the user chose to expose.
- A recipient of an exported bundle attempting to alter it.
- An untrusted document, bundle, template, or engagement file.

## Trust boundaries

1. **GPUI views → core services.** Views are trusted in-process Rust, but they call a narrow service
   layer. Database, vault, and key access stay `pub(crate)` in `testtracker-core`; the UI never
   reads secrets directly. This is defense-in-depth, not an OS isolation boundary.
2. **Core → database.** The database is encrypted at rest. Queries are parameterized.
3. **Core → plugin sandbox.** Plugins are untrusted. Capabilities are host-mediated and audited.
4. **Core → AI providers.** Provider responses are untrusted. Mutations require user approval.
5. **Device → bundle recipient.** Bundles are encrypted and signed; imports verify both.
6. **External MCP client → local bridge.** The client is outside the trust boundary. It must present
   the bearer token, and every proposal requires in-app approval before a write.
7. **Core → native filesystem.** File dialogs choose paths, but core validates size, extension, and
   content before storing, executing, or importing.

## Threats and mitigations

| Threat                    | Mitigation                                                                    | Status      |
| ------------------------- | ----------------------------------------------------------------------------- | ----------- |
| Stolen device             | SQLCipher at rest, Argon2id passphrase, auto-lock clears keys                 | Implemented |
| Weak passphrase           | Minimum 12 characters, memory-hard KDF                                        | Implemented |
| Database tampering        | SQLCipher authentication, hash-chained audit, chain verification              | Implemented |
| Unauthorized mutation     | Rust-side RBAC checks on every service write                                  | Implemented |
| Audit tampering           | Hash chain with previous-hash linkage; verification command and tests         | Implemented |
| Session left open         | Automatic lock after inactivity; native activity tracker and ticker            | Implemented |
| Secrets in logs           | No secret logging; errors are mapped to safe messages                         | Implemented |
| Remote content            | No webview, no remote assets, no shell/HTTP UI plugins                        | Implemented |
| Command injection         | No shell invocation in the current build                                      | Implemented |
| SQL injection             | Parameterized queries only                                                    | Implemented |
| Malicious plugin          | WASM sandbox, declared capabilities, Ed25519 signatures, hash checks          | Implemented |
| Plugin escape             | Resource limits, no raw filesystem/network/WASI, host-mediated calls          | Implemented |
| Prompt injection          | Untrusted-content delimiters, tool allowlist, human approval for writes       | Implemented |
| Cloud data leakage        | Local models default, per-engagement policy, redaction preview, consent log   | Implemented |
| Provider key theft        | Keys wrapped by the master key; never exposed to views                        | Implemented |
| Bundle tampering          | Encryption plus Ed25519 signature; version and hash checks on import          | Implemented |
| File leakage          | Encrypted file store, per-engagement keys, crypto-shredding                   | Implemented |
| Engagement-file leakage    | Encryption at rest, engagement scope, virtual paths, explicit tool permission     | Implemented |
| Tool overreach            | Read-only allowlist, per-call approval, revocable always-allow, audit hashes  | Implemented |
| Imported document abuse   | Bounded parsers, size/type limits, deterministic or reviewed drafts           | Implemented |
| Supply-chain compromise   | Cargo lockfile, `cargo audit`, pinned CI actions                              | Partial     |
| Malicious update          | Signed updater artifacts with signature verification                          | Planned     |
| Backup theft              | Encrypted backups with passphrase or recipient keys                           | Planned     |
| UI memory scraping        | Same-process native UI; no secret values in view structs                      | Residual    |

## Security invariants

- No vault, database, provider, or signing key is stored in a view struct.
- No mutation commits without an audit event.
- Severity is computed in core from probability and impact.
- Findings are records, never a boolean.
- Engagement files and generated reports stay out of Git.
- A locked vault rejects all data services.
- The audit chain verifies from the genesis hash.
- Assistant, agent, plugin, and MCP output never applies itself.

## Residual risks

- The device owner can read the local database; local RBAC is not hostile-user isolation.
- A compromised OS, keylogger, or memory scraper can capture the passphrase or decrypted data.
- SQLCipher protects data at rest, not a running unlocked process.
- The GPUI UI runs in the same process as core. A memory-safety bug or malicious dependency in the
  UI process can reach core state; the crate boundary is not an OS sandbox.
- Redaction can miss sensitive context; users must review AI payload previews.
- The MCP token grants access to all engagements while the bridge runs. Any local process that
  obtains it can read that data; a connected MCP client may send retrieved data off-device.
- MCP prompt injection can influence connected models. Drafts remain untrusted and require review.
- File text is untrusted and bounded; binary files are stored metadata-only unless a parser is added.
- Imported template bundles are untrusted content. The content hash proves integrity, not
  authorship.
- SSD wear-leveling can leave traces after deletion; crypto-shredding is the primary deletion
  control.
- Third-party crates, the GPU driver, and the windowing stack remain part of the attack surface.

## Dependency exceptions

`.cargo/audit.toml` records ignored advisories with narrow applicability:

- `quick-xml` is pulled by Typst's bibliography stack. TestTracker generates the Typst source
  itself, never parses untrusted XML, and does not use citations or bibliographies.
- `zip` 8.6.0 is pulled by docx-rs and is used only to write DOCX packages. The reported archive
  extraction advisories are not reachable; TestTracker never extracts untrusted ZIP or DOCX
  archives. Revisit when docx-rs updates.

These are re-evaluated when Typst or docx-rs updates its dependencies. Unmaintained and unsound
warnings remain visible in `cargo audit` output.

## Review triggers

Revisit this model when adding: MCP client tool calling, plugin UI contributions, bundle sync,
external tool execution, updater signing, multi-profile isolation, or any server component.
