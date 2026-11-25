use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::ai::model::{AiProviderInput, AiProviderKind};
use crate::documents::model::{DocumentFormat, DocumentKind};
use crate::domain::enums::{EngagementStatus, FindingStatus, RetestStatus, TestCaseStatus};
use crate::domain::models::{EngagementInput, FindingInput, ScopeAssetInput, TestCaseInput};
use crate::services;
use crate::state::AppState;

const PASSPHRASE: &str = "visual-fixture-passphrase";
const EXPECTED_DIR_NAME: &str = "tt-visual-data";

/// Build a throwaway synthetic vault for visual QA.
///
/// Run with:
/// `TESTTRACKER_VISUAL_DIR=/tmp/opencode/tt-visual-data \
///  cargo test -p testtracker-core seed_visual_fixture -- --ignored --nocapture`
#[test]
#[ignore = "creates a local synthetic vault for visual QA"]
fn seed_visual_fixture() {
    let dir = PathBuf::from(
        std::env::var("TESTTRACKER_VISUAL_DIR")
            .expect("set TESTTRACKER_VISUAL_DIR to a throwaway directory"),
    );
    assert_eq!(
        dir.file_name().and_then(|name| name.to_str()),
        Some(EXPECTED_DIR_NAME),
        "refusing to delete a directory not named {EXPECTED_DIR_NAME}"
    );
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear fixture directory");
    }
    fs::create_dir_all(&dir).expect("create fixture directory");

    let state = Arc::new(AppState::new(dir.clone()));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async move {
        services::vault::initialize_vault(
            state.clone(),
            PASSPHRASE.to_string(),
            "Visual QA".to_string(),
        )
        .await
        .expect("initialize fixture vault");

        services::ai::create_ai_provider(
            state.clone(),
            AiProviderInput {
                name: "Synthetic Local".into(),
                kind: AiProviderKind::Ollama,
                base_url: "http://127.0.0.1:11434".into(),
                model: "synthetic-model".into(),
                supports_vision: false,
                context_window: 8192,
                api_key: None,
            },
        )
        .await
        .expect("fixture provider");

        let rich = services::engagements::create_engagement(
            state.clone(),
            EngagementInput {
                name: "Synthetic Portal Assessment With A Deliberately Long Name To Test Truncation"
                    .into(),
                client_code: "SYN-001".into(),
                status: EngagementStatus::Active,
                start_date: "2026-11-02".into(),
                end_date: "2026-11-20".into(),
                application_summary: "Synthetic customer portal with login, API, uploads, and reporting. Used only for local visual QA.".into(),
                attack_surface: "Public web, authenticated API, file uploads, report exports.".into(),
                scope_assets: vec![
                    ScopeAssetInput {
                        kind: "url".into(),
                        value: "https://portal.example.test".into(),
                        in_scope: true,
                        notes: "Primary web application".into(),
                    },
                    ScopeAssetInput {
                        kind: "api".into(),
                        value: "https://api.example.test/v1".into(),
                        in_scope: true,
                        notes: "Authenticated JSON API".into(),
                    },
                    ScopeAssetInput {
                        kind: "url".into(),
                        value: "https://legacy.example.test".into(),
                        in_scope: false,
                        notes: "Out of scope for this engagement".into(),
                    },
                ],
                risks: Vec::new(),
                assumptions: vec!["Synthetic fixture only.".into()],
                contacts: Vec::new(),
            },
        )
        .await
        .expect("rich engagement");

        let first_test = services::test_cases::create_test_case(
            state.clone(),
            TestCaseInput {
                engagement_id: rich.id.clone(),
                template_id: Some("tpl-web-reflected-xss".into()),
                title: "Reflected cross-site scripting in search".into(),
                approach: "Inject benign script payloads into the search parameter and inspect the response.".into(),
                category: "Injection".into(),
                status: TestCaseStatus::Done,
                result: "Input is encoded and the payload does not execute.".into(),
                notes: "Synthetic result narrative.".into(),
                assignee: String::new(),
                sort_order: 0,
            },
        )
        .await
        .expect("first test");

        services::test_cases::create_test_case(
            state.clone(),
            TestCaseInput {
                engagement_id: rich.id.clone(),
                template_id: Some("tpl-web-auth".into()),
                title: "Authentication and session management with a deliberately long title for wrapping checks".into(),
                approach: "Review login, logout, session rotation, cookie flags, and lockout behavior.".into(),
                category: "Authentication".into(),
                status: TestCaseStatus::InProgress,
                result: String::new(),
                notes: "Synthetic in-progress test.".into(),
                assignee: String::new(),
                sort_order: 1,
            },
        )
        .await
        .expect("second test");

        services::findings::create_finding(
            state.clone(),
            FindingInput {
                engagement_id: rich.id.clone(),
                code: "F-001".into(),
                title: "Synthetic missing authorization on export".into(),
                description: "A regular user can request another tenant's export.".into(),
                impact: "Cross-tenant data exposure.".into(),
                remediation: "Enforce object-level authorization on export endpoints.".into(),
                owasp: "A1".into(),
                cwe: "CWE-639".into(),
                status: FindingStatus::Open,
                retest_status: RetestStatus::Pending,
                owner: "Visual QA".into(),
                probability: 7,
                impact_score: 8,
                test_case_ids: vec![first_test.id.clone()],
            },
        )
        .await
        .expect("linked finding");

        services::findings::create_finding(
            state.clone(),
            FindingInput {
                engagement_id: rich.id.clone(),
                code: "F-002".into(),
                title: "Synthetic standalone finding without a linked test".into(),
                description: "Used to exercise the standalone findings section.".into(),
                impact: "Synthetic impact.".into(),
                remediation: "Synthetic remediation.".into(),
                owasp: String::new(),
                cwe: String::new(),
                status: FindingStatus::Open,
                retest_status: RetestStatus::Pending,
                owner: "Visual QA".into(),
                probability: 3,
                impact_score: 3,
                test_case_ids: Vec::new(),
            },
        )
        .await
        .expect("standalone finding");

        services::files::add_engagement_file(
            state.clone(),
            rich.id.clone(),
            "reports/notes.txt".into(),
            "notes.txt".into(),
            "text/plain".into(),
            BASE64.encode(b"Synthetic visual QA notes.\nSecond line.\n"),
        )
        .await
        .expect("fixture file");

        if let Err(error) = services::documents::generate_document(
            state.clone(),
            rich.id.clone(),
            DocumentKind::ExecutiveSummary,
            DocumentFormat::Pdf,
        )
        .await
        {
            eprintln!("fixture document generation skipped: {error}");
        }

        services::engagements::create_engagement(
            state,
            EngagementInput {
                name: "Synthetic Empty Engagement".into(),
                client_code: "SYN-002".into(),
                status: EngagementStatus::Draft,
                start_date: String::new(),
                end_date: String::new(),
                application_summary: String::new(),
                attack_surface: String::new(),
                scope_assets: Vec::new(),
                risks: Vec::new(),
                assumptions: Vec::new(),
                contacts: Vec::new(),
            },
        )
        .await
        .expect("empty engagement");

        println!("fixture dir: {}", dir.display());
        println!("passphrase: {PASSPHRASE}");
        println!("engagement id: {}", rich.id);
    });
}
