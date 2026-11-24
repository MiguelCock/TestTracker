use std::fs;
use std::path::Path;

use tempfile::tempdir;

use crate::db;
use crate::domain::enums::ProfileRole;
use crate::security::{audit, vault};

const PASSPHRASE: &str = "correct horse battery staple";
const NOW: &str = "2026-01-01T00:00:00Z";

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create destination");
    for entry in fs::read_dir(source).expect("read source directory") {
        let entry = entry.expect("directory entry");
        let file_type = entry.file_type().expect("file type");
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target);
        } else if file_type.is_file() {
            fs::copy(entry.path(), target).expect("copy file");
        }
    }
}

#[test]
fn copied_data_directory_unlocks_and_verifies() {
    let source = tempdir().expect("source dir");
    let source_path = source.path();
    let vault_path = source_path.join("vault.json");
    let db_path = source_path.join("testtracker.db");

    let master = vault::create(&vault_path, PASSPHRASE).expect("create vault");
    let mut conn = db::open_encrypted(&db_path, &master).expect("open database");
    db::apply_migrations(&mut conn).expect("migrate");

    db::profiles::insert(
        &conn,
        "fixture-profile",
        "Synthetic Tester",
        ProfileRole::Admin,
        "synthetic-public-key",
        "synthetic-encrypted-private-key",
        NOW,
    )
    .expect("profile");
    audit::append(
        &conn,
        audit::AuditEntry {
            engagement_id: None,
            actor_id: Some("fixture-profile"),
            action: "vault.initialized",
            entity_type: "vault",
            entity_id: None,
            metadata: serde_json::json!({ "synthetic": true }),
            created_at: NOW,
        },
    )
    .expect("audit");
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .expect("checkpoint");
    drop(conn);

    let documents = source_path.join("documents").join("fixture-engagement");
    fs::create_dir_all(&documents).expect("documents directory");
    fs::write(documents.join("synthetic.txt"), b"synthetic document").expect("write document");

    let backup = tempdir().expect("backup dir");
    copy_tree(source_path, backup.path());

    assert!(backup.path().join("vault.json").is_file());
    assert!(backup.path().join("testtracker.db").is_file());
    assert!(
        backup
            .path()
            .join("documents/fixture-engagement/synthetic.txt")
            .is_file()
    );

    let copied_master =
        vault::unlock(&backup.path().join("vault.json"), PASSPHRASE).expect("unlock copied vault");
    let copied_conn = db::open_encrypted(&backup.path().join("testtracker.db"), &copied_master)
        .expect("open copied database");

    let display_name: String = copied_conn
        .query_row(
            "SELECT display_name FROM profiles WHERE id = 'fixture-profile'",
            [],
            |row| row.get(0),
        )
        .expect("profile survives copy");
    assert_eq!(display_name, "Synthetic Tester");
    assert!(
        audit::verify_chain(&copied_conn).expect("verify copied chain"),
        "audit chain must survive a directory copy"
    );

    let document = fs::read_to_string(
        backup
            .path()
            .join("documents/fixture-engagement/synthetic.txt"),
    )
    .expect("read copied document");
    assert_eq!(document, "synthetic document");
}
