# Encrypted engagement bundles

Bundles are the local-first way to move or back up one engagement. A `.ttbundle` contains a
passphrase-encrypted ZIP payload with a signed manifest.

## File format

```
magic         8 bytes  "TTBUNDLE"
version       1 byte   0x01
m_cost        u32 LE   Argon2id memory cost in KiB
t_cost        u32 LE   Argon2id iterations
p_cost        u32 LE   Argon2id parallelism
salt          32 bytes
nonce         24 bytes
ciphertext    remaining bytes
```

- Key derivation: Argon2id with the parameters in the header. Import enforces bounds of
  8 MiB–256 MiB memory, 1–10 iterations, and 1–8 lanes.
- Encryption: XChaCha20-Poly1305 with AAD `testtracker-bundle-v1`.
- Export passphrases must be at least 12 characters.
- Limits: 25 MB plaintext payload, 40 MB encrypted file.

## Payload ZIP

```
manifest.json       signed metadata and file hashes
engagement.json     engagement, scope, risks, assumptions, contacts, tests, findings,
                    evidence metadata, and document metadata
evidence/<id>       plaintext evidence bytes
documents/<id>      plaintext generated-document bytes (optional)
```

Unknown files, duplicate entries, directories, and oversized entries are rejected. All content is
read into memory; bundles are not extracted to disk by the importer.

## Manifest

```json
{
  "format": "testtracker.bundle.v1",
  "version": 1,
  "createdAt": "2026-11-07T...",
  "engagementId": "...",
  "engagementName": "Example",
  "senderProfileId": "...",
  "senderDisplayName": "Tester",
  "senderPublicKey": "64 hex chars",
  "includeDocuments": true,
  "files": {
    "engagement.json": "blake3 hex",
    "evidence/...": "blake3 hex"
  },
  "payloadHash": "blake3 hex",
  "signature": "ed25519 hex"
}
```

- `payloadHash` is BLAKE3 over the JSON serialization of the sorted `files` map.
- `signature` is Ed25519 over the manifest serialized with an empty signature field.
- Import verifies the manifest signature, every file hash, and the payload hash.
- The sender public key is included, so the signature is self-signed. The UI shows a fingerprint;
  it proves integrity, not the sender's real-world identity. Confirm fingerprints out of band.

## Import behavior

- A bundle always creates a **new engagement**; it never merges into an existing one.
- Scope, risk, contact, assumption, test-case, finding, evidence, and document identifiers are
  regenerated.
- Test-case links on findings and evidence are remapped to the new test-case IDs.
- Evidence and documents are re-encrypted with the new engagement's keys.
- The engagement name receives an ` (imported)` suffix.
- Snapshots, approvals, audit history, AI data, and plugins are not included.
- Import is transactional for the database. Files written before a failed commit are removed.

## UI

- **Export bundle** on an engagement asks for a passphrase, optionally includes generated
  documents, downloads the file, and shows the sender fingerprint.
- **Import bundle** on the Engagements page asks for the file and passphrase, shows a summary and
  sender fingerprint, then opens the imported engagement.

Bundles are intended for authorized sharing and per-engagement backup. They contain sensitive
client data and must not be committed to the repository. `.gitignore` covers `*.ttbundle`.
