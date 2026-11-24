use rusqlite::{Connection, params};
use tempfile::{TempDir, tempdir};

use crate::db::{self, schema};
use crate::error::AppError;
use crate::security::audit;

const NOW: &str = "2026-01-01T00:00:00Z";

fn open_temp_db() -> (TempDir, Connection) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("testtracker.db");
    let master = crate::security::crypto::random_bytes::<32>();
    let conn = db::open_encrypted(&path, &master).expect("open encrypted db");
    (dir, conn)
}

/// Apply only the first `target` migrations, leaving the database at that version.
fn apply_to(conn: &mut Connection, target: i64) {
    let mut version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read version");
    while version < target {
        let sql = schema::MIGRATIONS[version as usize];
        let tx = conn.transaction().expect("transaction");
        tx.execute_batch(sql).expect("apply migration");
        tx.pragma_update(None, "user_version", version + 1)
            .expect("set version");
        tx.commit().expect("commit migration");
        version += 1;
    }
}

fn insert_fixture(conn: &Connection) {
    conn.execute(
        "INSERT INTO profiles
            (id, display_name, role, public_key, encrypted_private_key, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "fixture-profile",
            "Synthetic Tester",
            "admin",
            "synthetic-public-key",
            "synthetic-encrypted-private-key",
            NOW
        ],
    )
    .expect("profile");

    conn.execute(
        "INSERT INTO engagements
            (id, name, client_code, status, start_date, end_date, application_summary,
             attack_surface, created_by, created_at, updated_at, version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            "fixture-engagement",
            "Synthetic engagement",
            "SYN-001",
            "active",
            "2026-01-01",
            "2026-01-31",
            "Synthetic application summary.",
            "Login, API, and uploads.",
            "fixture-profile",
            NOW,
            NOW,
            1
        ],
    )
    .expect("engagement");

    conn.execute(
        "INSERT INTO scope_assets (id, engagement_id, kind, value, in_scope, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "fixture-asset",
            "fixture-engagement",
            "url",
            "https://app.example.test",
            1,
            "Synthetic scope asset."
        ],
    )
    .expect("scope asset");

    conn.execute(
        "INSERT INTO risks
            (id, engagement_id, title, description, likelihood, impact, severity, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            "fixture-risk",
            "fixture-engagement",
            "Synthetic risk",
            "Synthetic risk description.",
            5,
            5,
            "medium",
            ""
        ],
    )
    .expect("risk");

    conn.execute(
        "INSERT INTO assumptions (id, engagement_id, text) VALUES (?1, ?2, ?3)",
        params![
            "fixture-assumption",
            "fixture-engagement",
            "Synthetic assumption."
        ],
    )
    .expect("assumption");

    conn.execute(
        "INSERT INTO contacts (id, engagement_id, name, role, email, phone)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "fixture-contact",
            "fixture-engagement",
            "Synthetic Contact",
            "Owner",
            "contact@example.test",
            ""
        ],
    )
    .expect("contact");

    conn.execute(
        "INSERT INTO test_cases
            (id, engagement_id, template_id, title, approach, category, status, result,
             notes, assignee, sort_order, created_at, updated_at)
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            "fixture-test",
            "fixture-engagement",
            "Synthetic test",
            "Do a synthetic thing.",
            "Fixture",
            "done",
            "Synthetic result.",
            "",
            "Synthetic Tester",
            0,
            NOW,
            NOW
        ],
    )
    .expect("test case");

    conn.execute(
        "INSERT INTO findings
            (id, engagement_id, code, title, description, impact, remediation, severity,
             probability, impact_score, owasp, cwe, status, retest_status, owner,
             created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            "fixture-finding",
            "fixture-engagement",
            "F-001",
            "Synthetic finding",
            "Synthetic finding description.",
            "Synthetic impact.",
            "Synthetic remediation.",
            "medium",
            5,
            5,
            "A1",
            "CWE-000",
            "open",
            "pending",
            "Synthetic Tester",
            NOW,
            NOW
        ],
    )
    .expect("finding");

    conn.execute(
        "INSERT INTO finding_test_cases (finding_id, test_case_id) VALUES (?1, ?2)",
        params!["fixture-finding", "fixture-test"],
    )
    .expect("finding test link");

    conn.execute(
        "INSERT INTO finding_assets (finding_id, asset_id) VALUES (?1, ?2)",
        params!["fixture-finding", "fixture-asset"],
    )
    .expect("finding asset link");

    conn.execute(
        "INSERT INTO evidence_blobs (blob_id, engagement_id, nonce, path, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            "fixture-blob",
            "fixture-engagement",
            "synthetic-nonce",
            "evidence/fixture-engagement/synthetic.bin",
            NOW
        ],
    )
    .expect("evidence blob");

    conn.execute(
        "INSERT INTO evidence
            (id, engagement_id, finding_id, test_case_id, filename, mime_type, size,
             blob_id, plaintext_hash, caption, created_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            "fixture-evidence",
            "fixture-engagement",
            "fixture-finding",
            "fixture-test",
            "synthetic.txt",
            "text/plain",
            17,
            "fixture-blob",
            "synthetic-plaintext-hash",
            "Synthetic evidence.",
            "fixture-profile",
            NOW
        ],
    )
    .expect("evidence");

    conn.execute(
        "INSERT INTO documents
            (id, engagement_id, kind, title, status, content_json, created_by, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            "fixture-document",
            "fixture-engagement",
            "report",
            "Synthetic document",
            "draft",
            "{}",
            "fixture-profile",
            NOW,
            NOW
        ],
    )
    .expect("document");

    conn.execute(
        "INSERT INTO document_versions
            (id, document_id, version, content_json, content_hash, created_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            "fixture-document-version",
            "fixture-document",
            1,
            "{}",
            "synthetic-content-hash",
            "fixture-profile",
            NOW
        ],
    )
    .expect("document version");

    conn.execute(
        "INSERT INTO approvals
            (id, engagement_id, document_id, reviewer_id, decision, comment, signature, signed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            "fixture-approval",
            "fixture-engagement",
            "fixture-document",
            "fixture-profile",
            "approved",
            "Synthetic approval.",
            "synthetic-signature",
            NOW
        ],
    )
    .expect("approval");

    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)",
        params!["fixture-setting", "synthetic"],
    )
    .expect("setting");

    for (action, entity_type, entity_id) in [
        ("fixture.created", "engagement", "fixture-engagement"),
        ("fixture.reviewed", "document", "fixture-document"),
    ] {
        audit::append(
            conn,
            audit::AuditEntry {
                engagement_id: Some("fixture-engagement"),
                actor_id: Some("fixture-profile"),
                action,
                entity_type,
                entity_id: Some(entity_id),
                metadata: serde_json::json!({ "synthetic": true }),
                created_at: NOW,
            },
        )
        .expect("audit event");
    }
}

fn insert_v9_file(conn: &Connection) {
    conn.execute(
        "INSERT INTO engagement_files
            (id, engagement_id, kind, filename, mime_type, size, plaintext_hash, nonce,
             path, summary_json, extracted_text, created_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            "fixture-file",
            "fixture-engagement",
            "text",
            "synthetic.txt",
            "text/plain",
            17,
            "synthetic-file-hash",
            "synthetic-file-nonce",
            "engagement-files/fixture-engagement/synthetic.bin",
            "{}",
            "synthetic text",
            "fixture-profile",
            NOW
        ],
    )
    .expect("engagement file");

    conn.execute(
        "INSERT INTO test_case_templates
            (id, domain, category, title, approach, expected_result, references_text, source, origin_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)",
        params![
            "fixture-template",
            "fixture",
            "Fixture",
            "Synthetic custom template",
            "Do a synthetic thing.",
            "",
            "",
            "custom"
        ],
    )
    .expect("custom template");
}

fn user_version(conn: &Connection) -> i64 {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version")
}

fn table_count(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .expect("count rows")
}

fn assert_healthy(conn: &Connection) {
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .expect("integrity check");
    assert_eq!(integrity, "ok", "database integrity check failed");

    let mut stmt = conn.prepare("PRAGMA foreign_key_check").expect("fk check");
    let violations = stmt.query_map([], |_| Ok(())).expect("fk rows").count();
    assert_eq!(violations, 0, "foreign key violations after migration");

    assert!(
        audit::verify_chain(conn).expect("verify audit chain"),
        "audit chain must remain valid after migration"
    );
}

#[test]
fn migration_registry_covers_latest_version() {
    assert_eq!(schema::LATEST_VERSION, schema::MIGRATIONS.len() as i64);
    assert!(!schema::MIGRATIONS.is_empty());
    assert!(
        schema::MIGRATIONS.iter().all(|sql| !sql.trim().is_empty()),
        "every registered migration must contain SQL"
    );
}

#[test]
fn migrates_v1_fixture_to_latest() {
    let (_dir, mut conn) = open_temp_db();
    apply_to(&mut conn, 1);
    assert_eq!(user_version(&conn), 1);
    insert_fixture(&conn);

    db::apply_migrations(&mut conn).expect("apply all migrations");

    assert_eq!(user_version(&conn), schema::LATEST_VERSION);
    assert_healthy(&conn);
    assert_eq!(table_count(&conn, "engagements"), 1);
    assert_eq!(table_count(&conn, "test_cases"), 1);
    assert_eq!(table_count(&conn, "findings"), 1);
    assert_eq!(table_count(&conn, "evidence"), 1);
    assert_eq!(table_count(&conn, "documents"), 1);
    assert_eq!(table_count(&conn, "approvals"), 1);
    assert!(table_count(&conn, "test_case_templates") > 0);
    assert!(table_count(&conn, "test_template_bundles") > 0);
}

#[test]
fn migrates_v9_fixture_to_latest_and_backfills_paths() {
    let (_dir, mut conn) = open_temp_db();
    apply_to(&mut conn, 9);
    assert_eq!(user_version(&conn), 9);
    insert_fixture(&conn);
    insert_v9_file(&conn);

    db::apply_migrations(&mut conn).expect("apply migrations 10+");

    assert_eq!(user_version(&conn), schema::LATEST_VERSION);
    assert_healthy(&conn);

    let relative_path: String = conn
        .query_row(
            "SELECT relative_path FROM engagement_files WHERE id = 'fixture-file'",
            [],
            |row| row.get(0),
        )
        .expect("relative path");
    assert_eq!(relative_path, "synthetic.txt");

    let (created_by, updated_at): (String, String) = conn
        .query_row(
            "SELECT created_by, updated_at FROM test_case_templates
             WHERE id = 'fixture-template'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("template ownership defaults");
    assert_eq!(created_by, "");
    assert_eq!(updated_at, "");
}

#[test]
fn rejects_database_from_newer_build() {
    let (_dir, mut conn) = open_temp_db();
    conn.pragma_update(None, "user_version", schema::LATEST_VERSION + 1)
        .expect("set newer version");

    let error = db::apply_migrations(&mut conn).expect_err("newer schema must be rejected");
    assert!(matches!(error, AppError::Conflict(_)));
}

#[test]
fn applying_migrations_twice_is_content_idempotent() {
    let (_dir, mut conn) = open_temp_db();
    db::apply_migrations(&mut conn).expect("first migration");
    let templates = table_count(&conn, "test_case_templates");
    let bundles = table_count(&conn, "test_template_bundles");
    let version = user_version(&conn);

    db::apply_migrations(&mut conn).expect("second migration");

    assert_eq!(user_version(&conn), version);
    assert_eq!(table_count(&conn, "test_case_templates"), templates);
    assert_eq!(table_count(&conn, "test_template_bundles"), bundles);
    let orphan_items: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM test_template_bundle_items i
             LEFT JOIN test_case_templates t ON t.id = i.template_id
             WHERE t.id IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("orphan check");
    assert_eq!(orphan_items, 0);
    assert_healthy(&conn);
}
