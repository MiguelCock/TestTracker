use tempfile::tempdir;

use crate::db;
use crate::domain::enums::{
    ApprovalDecision, EngagementStatus, FindingStatus, ProfileRole, RetestStatus, Severity,
    TestCaseStatus,
};
use crate::domain::models::{
    ContactInput, EngagementInput, Evidence, FindingInput, RiskInput, ScopeAssetInput, Snapshot,
    TestCaseInput,
};
use crate::domain::severity::severity_for;
use crate::domain::signing::approval_message;
use crate::security::{audit, crypto, identity};

#[test]
fn local_flow_persists_and_verifies() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("testtracker.db");
    let master = crypto::random_bytes::<32>();
    let mut conn = db::open_encrypted(&db_path, &master).expect("open encrypted db");
    db::apply_migrations(&mut conn).expect("migrate");

    let profile_id = "profile-1";
    let signing = identity::generate();
    let (public_key, encrypted_private_key) =
        identity::encrypt_signing_key(&master, profile_id, &signing).expect("encrypt identity");
    let now = "2026-01-01T00:00:00Z";
    db::profiles::insert(
        &conn,
        profile_id,
        "Tester",
        ProfileRole::Admin,
        &public_key,
        &encrypted_private_key,
        now,
    )
    .expect("profile");

    let engagement_input = EngagementInput {
        name: "Synthetic portal assessment".into(),
        client_code: "SYN-001".into(),
        status: EngagementStatus::Active,
        start_date: "2026-01-05".into(),
        end_date: "2026-01-09".into(),
        application_summary: "Synthetic application.".into(),
        attack_surface: "Login, APIs, uploads.".into(),
        scope_assets: vec![ScopeAssetInput {
            kind: "url".into(),
            value: "https://app.example.test".into(),
            in_scope: true,
            notes: String::new(),
        }],
        risks: vec![RiskInput {
            title: "Unauthorized access".into(),
            description: String::new(),
            likelihood: 7,
            impact: 7,
            notes: String::new(),
        }],
        assumptions: vec!["Testing is limited to listed assets.".into()],
        contacts: vec![ContactInput {
            name: "Client Contact".into(),
            role: "Owner".into(),
            email: "contact@example.test".into(),
            phone: String::new(),
        }],
    };

    let tx = conn.transaction().expect("transaction");
    let engagement =
        db::engagements::insert(&tx, "engagement-1", &engagement_input, profile_id, now)
            .expect("engagement");
    audit::append(
        &tx,
        audit::AuditEntry {
            engagement_id: Some("engagement-1"),
            actor_id: Some(profile_id),
            action: "engagement.created",
            entity_type: "engagement",
            entity_id: Some("engagement-1"),
            metadata: serde_json::json!({ "name": engagement.name }),
            created_at: now,
        },
    )
    .expect("audit");
    tx.commit().expect("commit");
    assert_eq!(engagement.name, "Synthetic portal assessment");

    let test_case = db::test_cases::create(
        &conn,
        &TestCaseInput {
            engagement_id: "engagement-1".into(),
            template_id: Some("tpl-web-reflected-xss".into()),
            title: "Reflected cross-site scripting".into(),
            approach: "Inject benign script payloads.".into(),
            category: "Injection".into(),
            status: TestCaseStatus::Done,
            result: "Encoding blocks execution.".into(),
            notes: String::new(),
            assignee: "Tester".into(),
            sort_order: 0,
        },
        now,
    )
    .expect("test case");

    let severity = severity_for(8, 8).expect("severity");
    let finding = db::findings::create(
        &conn,
        &FindingInput {
            engagement_id: "engagement-1".into(),
            code: String::new(),
            title: "Missing authorization on export".into(),
            description: "A regular user can export another tenant's data.".into(),
            impact: "Cross-tenant data exposure.".into(),
            remediation: "Enforce object-level authorization.".into(),
            owasp: "A1".into(),
            cwe: "CWE-639".into(),
            status: FindingStatus::Open,
            retest_status: RetestStatus::Pending,
            owner: "Tester".into(),
            probability: 8,
            impact_score: 8,
            test_case_ids: vec![test_case.id.clone()],
        },
        severity,
        "F-001",
        now,
    )
    .expect("finding");
    assert_eq!(finding.severity, Severity::Critical);
    assert_eq!(finding.test_case_ids, vec![test_case.id.clone()]);

    let detail = db::engagements::detail(&conn, "engagement-1").expect("detail");
    assert_eq!(detail.test_case_count, 1);
    assert_eq!(detail.finding_count, 1);
    assert_eq!(detail.open_finding_count, 1);
    assert_eq!(detail.risks[0].severity, Severity::Critical);

    let evidence = Evidence {
        id: "evidence-1".into(),
        engagement_id: "engagement-1".into(),
        finding_id: Some(finding.id.clone()),
        test_case_id: Some(test_case.id.clone()),
        filename: "request.txt".into(),
        mime_type: "text/plain".into(),
        size: 128,
        blob_id: "blob-1".into(),
        plaintext_hash: "hash".into(),
        caption: "Synthetic request".into(),
        created_by: profile_id.into(),
        created_at: now.into(),
    };
    let tx = conn.transaction().expect("transaction");
    db::evidence::insert_blob(
        &tx,
        "blob-1",
        "engagement-1",
        &hex::encode([0u8; 24]),
        "/tmp/synthetic-evidence.bin",
        now,
    )
    .expect("blob");
    db::evidence::insert(&tx, &evidence).expect("evidence");
    tx.commit().expect("commit");
    let listed = db::evidence::list(&conn, "engagement-1").expect("list evidence");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].filename, "request.txt");
    let (nonce, path) = db::evidence::blob(&conn, "blob-1").expect("blob row");
    assert_eq!(nonce.len(), 48);
    assert!(path.ends_with("synthetic-evidence.bin"));

    let key = crypto::derive_evidence_key(&master, "engagement-1");
    let aad = "testtracker-evidence-v1:engagement-1:evidence-1";
    let encrypted = crypto::encrypt(&key, b"synthetic evidence", aad.as_bytes()).expect("encrypt");
    let decrypted = crypto::decrypt(
        &key,
        &encrypted.nonce,
        &encrypted.ciphertext,
        aad.as_bytes(),
    )
    .expect("decrypt");
    assert_eq!(decrypted.as_slice(), b"synthetic evidence");

    let snapshot_json = serde_json::json!({
        "engagement": "engagement-1",
        "findings": 1,
        "tests": 1,
    })
    .to_string();
    let content_hash = blake3::hash(snapshot_json.as_bytes()).to_hex().to_string();
    let snapshot = Snapshot {
        id: "snapshot-1".into(),
        engagement_id: "engagement-1".into(),
        version: 1,
        label: "Initial review".into(),
        content_hash: content_hash.clone(),
        snapshot_json,
        created_by: profile_id.into(),
        created_at: now.into(),
        verified: true,
    };
    let tx = conn.transaction().expect("transaction");
    db::snapshots::insert(&tx, &snapshot).expect("snapshot");
    tx.commit().expect("commit");
    let snapshots = db::snapshots::list(&conn, "engagement-1").expect("list snapshots");
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].label, "Initial review");

    let approval_id = "approval-1";
    let signed_at = "2026-01-02T00:00:00Z";
    let message = approval_message(
        approval_id,
        "engagement-1",
        &snapshot.id,
        ApprovalDecision::Approved,
        signed_at,
        &content_hash,
    );
    let signature = identity::sign_hex(&signing, &message);
    let tx = conn.transaction().expect("transaction");
    db::approvals::insert(
        &tx,
        db::approvals::NewApproval {
            id: approval_id,
            engagement_id: "engagement-1",
            snapshot_id: &snapshot.id,
            reviewer_id: profile_id,
            decision: ApprovalDecision::Approved,
            comment: "Reviewed",
            signature: &signature,
            signed_at,
        },
    )
    .expect("approval");
    tx.commit().expect("commit");

    let approvals = db::approvals::list(&conn, "engagement-1").expect("list approvals");
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0].decision, ApprovalDecision::Approved);
    assert!(identity::verify_hex(
        &approvals[0].public_key,
        &message,
        &approvals[0].signature
    ));

    conn.execute(
        "UPDATE approvals SET decision = 'rejected' WHERE id = ?1",
        [approval_id],
    )
    .expect("tamper");
    let tampered = db::approvals::list(&conn, "engagement-1").expect("list tampered");
    let tampered_message = approval_message(
        &tampered[0].id,
        &tampered[0].engagement_id,
        &snapshot.id,
        tampered[0].decision,
        &tampered[0].signed_at,
        &content_hash,
    );
    assert!(!identity::verify_hex(
        &tampered[0].public_key,
        &tampered_message,
        &tampered[0].signature
    ));

    let metrics = db::metrics::dashboard(&conn).expect("metrics");
    assert_eq!(metrics.total_engagements, 1);
    assert_eq!(metrics.active_engagements, 1);
    assert_eq!(metrics.open_findings, 1);
    assert_eq!(metrics.findings_by_severity.critical, 1);
    assert_eq!(metrics.test_progress.done, 1);
    assert!(audit::verify_chain(&conn).expect("verify"));

    drop(conn);

    let reopened = db::open_encrypted(&db_path, &master).expect("reopen");
    assert!(audit::verify_chain(&reopened).expect("verify reopened"));
    let detail = db::engagements::detail(&reopened, "engagement-1").expect("detail reopened");
    assert_eq!(detail.test_case_count, 1);
    assert_eq!(detail.finding_count, 1);

    let second = db::test_cases::create(
        &reopened,
        &TestCaseInput {
            engagement_id: "engagement-1".into(),
            template_id: None,
            title: "Second test".into(),
            approach: String::new(),
            category: String::new(),
            status: TestCaseStatus::NotStarted,
            result: String::new(),
            notes: String::new(),
            assignee: String::new(),
            sort_order: 1,
        },
        now,
    )
    .expect("second test");
    let third = db::test_cases::create(
        &reopened,
        &TestCaseInput {
            engagement_id: "engagement-1".into(),
            template_id: None,
            title: "Third test".into(),
            approach: String::new(),
            category: String::new(),
            status: TestCaseStatus::NotStarted,
            result: String::new(),
            notes: String::new(),
            assignee: String::new(),
            sort_order: 2,
        },
        now,
    )
    .expect("third test");
    db::test_cases::reorder(
        &reopened,
        "engagement-1",
        &[third.id.clone(), test_case.id.clone(), second.id.clone()],
        "2026-01-03T00:00:00Z",
    )
    .expect("reorder");
    let ordered = db::test_cases::list(&reopened, "engagement-1").expect("ordered");
    assert_eq!(ordered.len(), 3);
    assert_eq!(ordered[0].id, third.id);
    assert_eq!(ordered[1].id, test_case.id);
    assert_eq!(ordered[2].id, second.id);
    assert!(
        db::test_cases::reorder(
            &reopened,
            "engagement-1",
            &[third.id.clone(), test_case.id.clone()],
            now
        )
        .is_err()
    );
    db::test_cases::delete(&reopened, &second.id).expect("delete second");
    db::test_cases::delete(&reopened, &third.id).expect("delete third");

    let wrong_key = crypto::random_bytes::<32>();
    assert!(db::open_encrypted(&db_path, &wrong_key).is_err());
}
