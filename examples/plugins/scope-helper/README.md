# Scope Helper example plugin

This read-only example returns a suggested authentication test case. It declares the
`engagement.read` capability but does not access evidence or write data.

## Build and sign

From the repository root:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin ttplugin -- keygen publisher.key
```

Import the printed public key in **Settings → Plugins → Add publisher key** with publisher ID
`com.example.publisher`.

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin ttplugin -- sign \
  examples/plugins/scope-helper publisher.key examples/plugins/scope-helper.ttplugin
```

Install `examples/plugins/scope-helper.ttplugin` from **Settings → Plugins**, then run it from an
engagement's **Plugins** tab.

Keep `publisher.key` secret; it is ignored by Git only if you place it under a path covered by
`.gitignore`. Never commit signing keys.
