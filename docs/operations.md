# Operations: backup, restore, and rollback

This document covers the local data directory only. It does not replace a device-level backup
policy, and it never permits storing vault copies inside the repository.

## Data directory

The application stores everything in one directory:

- Linux: `~/.local/share/com.testtracker.desktop`
- macOS: `~/Library/Application Support/com.testtracker.desktop`
- Windows: `%LOCALAPPDATA%\com.testtracker.desktop`

`TESTTRACKER_DATA_DIR` overrides the location for drills and tests.

| Entry | Contents | Backup |
| --- | --- | --- |
| `vault.json` | Salt, KDF parameters, and the encrypted master key | Required |
| `testtracker.db` | SQLCipher database containing engagements, tests, findings, templates, audit events, and metadata | Required |
| `testtracker.db-wal`, `testtracker.db-shm` | SQLite sidecars; may contain committed pages | Required if present |
| `documents/` | Encrypted generated documents | Required |
| `engagement-files/` | Encrypted Files workspace | Required if present |
| `evidence/` | Legacy encrypted evidence pending migration | Required if present |
| `plugins/` | Installed signed plugins | Required if present |

Legacy browser caches from the pre-GPUI build (`WebKitCache/`, `CacheStorage/`, `storage/`,
`mediakeys/`, `hsts-storage.sqlite`) hold no application data. They can be skipped and should not
be copied while a browser process is active.

**`vault.json` and `testtracker.db` are a pair.** The database key is derived from the master key
stored in the vault. Restoring one without the other produces an unreadable database. Restore the
whole directory, always.

## Before taking a backup

1. Close TestTracker completely. Confirm no process is running.
2. Confirm no `testtracker-app` process holds the database.
3. Record the current commit and baseline tag.
4. Ensure the destination is outside the repository and outside any folder synced to a third-party
   cloud service. Prefer an encrypted volume.
5. Keep at least twice the data-directory size free.

## Backup procedure

```bash
set -euo pipefail

src="$HOME/.local/share/com.testtracker.desktop"
dst="$HOME/testtracker-backups/$(date +%F)-pre-simplify"

umask 077
mkdir -p "$dst"
chmod 700 "$HOME/testtracker-backups" "$dst"

for entry in vault.json testtracker.db testtracker.db-wal testtracker.db-shm \
             documents engagement-files evidence plugins; do
  if [ -e "$src/$entry" ]; then
    cp -a "$src/$entry" "$dst/"
  fi
done

chmod -R go-rwx "$dst"

(
  cd "$dst"
  find . -type f ! -name SHA256SUMS -print0 \
    | sort -z \
    | xargs -0 sha256sum > SHA256SUMS
  sha256sum -c SHA256SUMS --quiet
)
```

Record the commit and tag with the backup, for example in a local `BACKUP-NOTES.txt` next to the
manifest:

```text
baseline tag: pre-simplify
commit: <sha>
created: <UTC timestamp>
source: <data directory>
```

Do not put the vault passphrase in the backup directory or in the notes.

## Backup verification drill

Run the app against the copy, never against the original:

```bash
TESTTRACKER_DATA_DIR="$dst" cargo run -p testtracker-app
```

Unlock with the normal passphrase and verify:

- The engagement list matches the original.
- One engagement opens and its Overview and scope render.
- Files and Documents open.
- Settings → Activity loads, and the audit chain verifies.

The copy can be used for this drill and then kept or deleted according to the retention policy.
Do not run write-heavy operations against the only backup.

A synthetic, automated version of this drill runs in CI:

```bash
cargo test -p testtracker-core backup_restore_tests -- --nocapture
```

It creates a vault and encrypted database, copies the whole directory, unlocks the copy, and
verifies the profile, a nested document, and the audit chain.

## Restore procedure

1. Close TestTracker.
2. Move the current data directory aside; do not delete it:

   ```bash
   mv "$HOME/.local/share/com.testtracker.desktop" \
      "$HOME/.local/share/com.testtracker.desktop.failed-$(date +%s)"
   ```

3. Copy the backup into place:

   ```bash
   cp -a "$dst" "$HOME/.local/share/com.testtracker.desktop"
   chmod 700 "$HOME/.local/share/com.testtracker.desktop"
   ```

4. Launch the app, unlock, and repeat the verification checklist above.
5. Only after verification, remove the failed directory if it is no longer needed.

Never merge two data directories, copy only `testtracker.db`, or restore an old database under a
new `vault.json`. The pair and the audit chain must come from the same backup.

## Rollback after a migration

Schema migrations are forward-only. There are no down migrations.

- To roll back the **application**, check out the baseline tag and rebuild.
- To roll back the **data**, restore the entire pre-migration directory from the verified backup.
- A database migrated to a newer schema is not expected to open in an older binary.
- Audit events written after the backup are lost if the backup is restored. That is the accepted
  cost of rollback; record it in the incident notes.

The destructive migration in E2 must not run until the migration policy gates and the pre-drop
backup are satisfied. See `docs/migration-policy.md`.

## Retention

Keep the pre-migration backup until the post-cutover verification in TT-124 passes and the owner
signs off. Do not delete it automatically. Backups contain client data even though the vault and
database are encrypted; handle them with the same care as the live data directory.
