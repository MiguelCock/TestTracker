# Plugin SDK

TestTracker supports signed, read-only WebAssembly plugins. Plugins receive engagement data only
when they declare the `engagement.read` capability, run inside a resource-limited WASM sandbox, and
return suggestions that the tester must explicitly apply.

This is the v1 SDK. Process runners, network access, MCP, and schema-driven UI contributions are
planned.

## Package format

A plugin is a `.ttplugin` file (ZIP) containing exactly:

```
manifest.json     plugin metadata and capabilities
plugin.wasm       WebAssembly module
signature         Ed25519 signature in hex
```

Extra files and duplicate entries are rejected. Limits: 8 MB package, 64 KB manifest, 5 MB WASM,
1 KB signature.

## Manifest

```json
{
  "id": "com.example.scope-helper",
  "name": "Scope Helper (example)",
  "version": "1.0.0",
  "publisherId": "com.example.publisher",
  "description": "Example read-only plugin that suggests a test case.",
  "runtime": "wasm",
  "entry": "plugin.wasm",
  "capabilities": ["engagement.read"]
}
```

Validation rules:

- `id` and `publisherId`: 3–100 lowercase ASCII letters, digits, dots, dashes, or underscores; no
  leading dot/dash and no `..`.
- `version`: alphanumeric plus `.`, `-`, `+`; no `..`.
- `runtime` must be `wasm`; `entry` must be `plugin.wasm`.
- `capabilities` must be a subset of the supported capabilities; duplicates are rejected.

## Capabilities

| Capability        | Grants                                                                                                 |
| ----------------- | ------------------------------------------------------------------------------------------------------ |
| `engagement.read` | Engagement metadata, scope, risks, assumptions, contacts, test cases, and findings in the plugin input |

Plugins never receive evidence content, provider credentials, signing keys, or audit data. They
cannot write to the database.

## Signing and trust

The signature is Ed25519 over:

```
BLAKE3("testtracker-plugin-v1" || manifest bytes || WASM bytes)
```

Installation succeeds only when the manifest's `publisherId` matches a public key imported in
**Settings → Plugins → Add publisher key** and the signature verifies. The app also stores a BLAKE3
hash of the WASM file and verifies it before every execution.

The CLI creates and verifies packages:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin ttplugin -- keygen publisher.key
cargo run --manifest-path src-tauri/Cargo.toml --bin ttplugin -- sign \
  examples/plugins/scope-helper publisher.key scope-helper.ttplugin
cargo run --manifest-path src-tauri/Cargo.toml --bin ttplugin -- verify \
  scope-helper.ttplugin <public-key-hex>
```

`keygen` writes the secret key with `0600` permissions. Never commit signing keys. `.gitignore`
covers `*.key` and `*.wasm`.

If the plugin directory contains `plugin.wat` instead of `plugin.wasm`, the signing CLI compiles it
with `wat` before packaging.

## Guest ABI

Exports:

| Export                           | Type     | Purpose                                 |
| -------------------------------- | -------- | --------------------------------------- |
| `memory`                         | memory   | Linear memory                           |
| `alloc(len: i32) -> i32`         | function | Allocate input space; returns a pointer |
| `run(ptr: i32, len: i32) -> i64` | function | Execute with input at `ptr..ptr+len`    |

Imports:

| Import         | Signature                    | Purpose                                        |
| -------------- | ---------------------------- | ---------------------------------------------- |
| `env.host_log` | `(ptr: i32, len: i32) -> ()` | Append a UTF-8 log line; capped at 64 KB total |

The return value of `run` packs the output pointer and length:

```
packed = (output_ptr as i64) << 32 | output_len as i64
```

The host validates pointers and lengths against guest memory. Missing exports, invalid UTF-8,
traps, invalid JSON, and oversized output are reported as plugin errors.

## Input JSON

```json
{
  "schema": "testtracker.plugin.input.v1",
  "plugin": { "id": "com.example.scope-helper", "version": "1.0.0" },
  "capabilities": ["engagement.read"],
  "engagement": {
    "engagement": {},
    "scopeAssets": [],
    "risks": [],
    "assumptions": [],
    "contacts": []
  },
  "testCases": [],
  "findings": []
}
```

Without `engagement.read`, `engagement` is `null` and the test/finding arrays are empty. Evidence
content is never included.

## Output JSON

```json
{
  "summary": "Short factual summary.",
  "testCases": [
    {
      "title": "Review authentication and session handling",
      "category": "Authentication",
      "approach": "Inspect login, logout, reset, and session rotation.",
      "expectedResult": "Sessions rotate and unauthorized access is blocked."
    }
  ],
  "findings": [
    {
      "title": "Example suggestion",
      "description": "What to verify.",
      "impact": "Why it matters.",
      "remediation": "What to change.",
      "severity": "medium"
    }
  ],
  "notes": ["Optional note"]
}
```

Limits: summary 5,000 chars; 50 test suggestions; 20 finding suggestions; 50 notes; 2,000 chars per
note; 10,000 chars per finding field. Suggested test cases are added only when the tester selects
**Add suggested tests**. Suggested findings are display-only and must be created manually.

## Sandbox limits

- Fuel: 10,000,000 instructions.
- Memory: 64 MB maximum guest memory.
- Tables/instances: 1 table, 1 instance, 1 memory.
- Input: 512 KB.
- Output: 256 KB.
- Logs: 64 KB total, 8 KB per line.
- No WASI, filesystem, network, shell, environment, or clock access.
- Only `env.host_log` is linked; other imports fail instantiation.

## Example plugin

`examples/plugins/scope-helper/` contains a WAT plugin that returns one suggested authentication
test case. See its README for signing and installation steps.

## Security notes

- The package signature establishes publisher trust; the WASM sandbox limits what the code can do.
- Plugin output is untrusted text. It is rendered as plain text and never auto-applied.
- Plugin files are installed under the app data directory with `0600` file / `0700` directory
  permissions.
- Execution is audited with plugin ID/version, engagement, output hash, fuel used, and log count.
  Plugin output is not persisted.
- Install/upgrade replaces the WASM file atomically from the app's perspective and removes the old
  file after a successful database commit.
- A user with filesystem access can modify plugin files, but the stored WASM hash check prevents
  execution after modification.

## Non-goals for v1

- Process runners for external tools.
- Network access from plugins.
- Arbitrary JavaScript or HTML in the main webview.
- Plugin marketplace or remote registry.
- Automatic creation of findings or test cases without user action.
