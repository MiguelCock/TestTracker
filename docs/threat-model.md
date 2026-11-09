# Threat model

## Assets

- Engagement scope, attack surface, risks, and assumptions.
- Test cases, results, and evidence.
- Findings, severity, remediation, and retest status.
- Reviewer approvals and sign-offs.
- Audit history and version history.
- Provider credentials, the MCP bearer token, and signing private keys.
- The integrity of the local database and generated reports.

## Adversaries

- A thief or unauthorized user with access to the device.
- A malicious or compromised third-party plugin.
- A compromised or untrusted AI provider.
- Prompt-injection content inside evidence or imported engagement data.
- A supply-chain attacker in npm, Cargo, Tauri, or the updater.
- A local user making accidental or unauthorized changes.
- A malicious local process or website attempting to reach a loopback MCP server.
- A cloud-backed MCP client receiving data the user chose to expose.
- A recipient of an exported bundle attempting to alter it.

## Trust boundaries

1. **Webview → Rust IPC.** The webview is untrusted. Rust validates every command, enforces
   permissions, and owns all secrets.
2. **Rust → database.** The database is encrypted at rest. Queries are parameterized.
3. **Rust → plugin sandbox.** Plugins are untrusted. Capabilities are host-mediated and audited.
4. **Rust → AI providers.** Provider responses are untrusted. Mutations require user approval.
5. **Device → bundle recipient.** Bundles are encrypted and signed; imports verify both.
6. **External MCP client → local bridge.** The client is outside the trust boundary. It must present
   the bearer token, and every proposal requires in-app approval before a write.

## Threats and mitigations

| Threat                    | Mitigation                                                                    | Status      |
| ------------------------- | ----------------------------------------------------------------------------- | ----------- |
| Stolen device             | SQLCipher at rest, Argon2id passphrase, auto-lock clears keys                 | Implemented |
| Weak passphrase           | Minimum 12 characters, memory-hard KDF; strength guidance in UI               | Implemented |
| Database tampering        | SQLCipher authentication, hash-chained audit, chain verification              | Implemented |
| Unauthorized mutation     | Rust-side RBAC checks on every write command                                  | Implemented |
| Audit tampering           | Hash chain with previous-hash linkage; verification command and tests         | Implemented |
| Session left open         | Automatic lock after inactivity; frontend idle timer calls lock               | Implemented |
| Secrets in logs           | No secret logging; errors are mapped to safe messages                         | Implemented |
| Remote content in webview | Strict CSP, no remote assets, no shell or HTTP webview plugins                | Implemented |
| Command injection         | No shell invocation in the current build                                      | Implemented |
| SQL injection             | Parameterized queries only                                                    | Implemented |
| Malicious plugin          | WASM/process sandbox, declared capabilities, Ed25519 signatures, user consent | Planned     |
| Plugin escape             | Resource limits, no raw filesystem/network, host-mediated calls               | Planned     |
| Prompt injection          | Untrusted-content delimiters, tool allowlist, human approval for writes       | Planned     |
| Cloud data leakage        | Local models default, per-engagement policy, redaction preview, consent log   | Planned     |
| Provider key theft        | Keys wrapped by the master key; never exposed to the webview                  | Planned     |
| Bundle tampering          | Encryption plus Ed25519 signature; version and hash checks on import          | Planned     |
| Evidence leakage          | Encrypted blob store, per-engagement keys, crypto-shredding                   | Planned     |
| Supply-chain compromise   | Lockfiles, `cargo audit`, `bun audit`, pinned CI actions, SBOM                | Partial     |
| Malicious update          | Signed updater artifacts with signature verification                          | Planned     |
| Backup theft              | Encrypted backups with passphrase or recipient keys                           | Planned     |

## Security invariants

- No secret is ever serialized to the webview.
- No mutation commits without an audit event.
- Severity is computed on the server from probability and impact.
- Findings are records, never a boolean.
- Evidence and generated reports stay out of Git.
- A locked vault rejects all data commands.
- The audit chain verifies from the genesis hash.

## Residual risks

- The device owner can read the local database; local RBAC is not hostile-user isolation.
- A compromised OS, keylogger, or memory scraper can capture the passphrase or decrypted data.
- SQLCipher protects data at rest, not a running unlocked process.
- Redaction can miss sensitive context; users must review AI payload previews.
- The MCP token grants access to all engagements while the bridge runs. Any local process that
  obtains it can read that data; a connected MCP client may send retrieved data off-device.
- MCP prompt injection can influence connected models. Drafts remain untrusted and require review.
- SSD wear-leveling can leave traces after deletion; crypto-shredding is the primary deletion
  control.
- Third-party dependencies and the webview engine remain part of the attack surface.

## Dependency exceptions

`.cargo/audit.toml` records six ignored advisories with narrow applicability:

- `quick-xml` 0.38.4 is pulled by Typst's bibliography stack. TestTracker generates the Typst
  source itself, never parses untrusted XML, and does not use citations or bibliographies.
- `zip` 8.6.0 is pulled by docx-rs and is used only to write DOCX packages. TestTracker never
  extracts untrusted ZIP or DOCX archives.

These are re-evaluated when Typst or docx-rs updates its dependencies. Unmaintained and unsound
warnings remain visible in `cargo audit` output.

## Review triggers

Revisit this model when adding: evidence storage, AI providers, plugins, bundle sync, external tool
execution, updater signing, or any server component.
