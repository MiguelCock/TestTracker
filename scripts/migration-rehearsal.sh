#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

echo "==> Migration fixture rehearsal"
cargo test -p testtracker-core db::migration_tests -- --nocapture

echo "==> Copied data-directory rehearsal"
cargo test -p testtracker-core backup_restore_tests -- --nocapture
