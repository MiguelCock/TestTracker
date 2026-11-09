use serde::{Deserialize, Serialize};

use crate::domain::enums::{
    ApprovalDecision, EngagementStatus, FindingStatus, ProfileRole, RetestStatus, Severity,
    TestCaseStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub id: String,
    pub display_name: String,
    pub role: ProfileRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub initialized: bool,
    pub unlocked: bool,
    pub profile: Option<ProfileSummary>,
    pub auto_lock_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeAsset {
    pub id: String,
    pub kind: String,
    pub value: String,
    pub in_scope: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeAssetInput {
    pub kind: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub in_scope: bool,
    #[serde(default)]
    pub notes: String,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Risk {
    pub id: String,
    pub title: String,
    pub description: String,
    pub likelihood: u8,
    pub impact: u8,
    pub severity: Severity,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskInput {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub likelihood: u8,
    pub impact: u8,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assumption {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id: String,
    pub name: String,
    pub role: String,
    pub email: String,
    pub phone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactInput {
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub phone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngagementInput {
    pub name: String,
    #[serde(default)]
    pub client_code: String,
    pub status: EngagementStatus,
    #[serde(default)]
    pub start_date: String,
    #[serde(default)]
    pub end_date: String,
    #[serde(default)]
    pub application_summary: String,
    #[serde(default)]
    pub attack_surface: String,
    #[serde(default)]
    pub scope_assets: Vec<ScopeAssetInput>,
    #[serde(default)]
    pub risks: Vec<RiskInput>,
    #[serde(default)]
    pub assumptions: Vec<String>,
    #[serde(default)]
    pub contacts: Vec<ContactInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Engagement {
    pub id: String,
    pub name: String,
    pub client_code: String,
    pub status: EngagementStatus,
    pub start_date: String,
    pub end_date: String,
    pub application_summary: String,
    pub attack_surface: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngagementSummary {
    pub id: String,
    pub name: String,
    pub client_code: String,
    pub status: EngagementStatus,
    pub start_date: String,
    pub end_date: String,
    pub updated_at: String,
    pub test_case_count: i64,
    pub finding_count: i64,
    pub open_finding_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngagementDetail {
    pub engagement: Engagement,
    pub scope_assets: Vec<ScopeAsset>,
    pub risks: Vec<Risk>,
    pub assumptions: Vec<Assumption>,
    pub contacts: Vec<Contact>,
    pub test_case_count: i64,
    pub finding_count: i64,
    pub open_finding_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCaseTemplate {
    pub id: String,
    pub domain: String,
    pub category: String,
    pub title: String,
    pub approach: String,
    pub expected_result: String,
    pub references: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCaseInput {
    pub engagement_id: String,
    #[serde(default)]
    pub template_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub approach: String,
    #[serde(default)]
    pub category: String,
    pub status: TestCaseStatus,
    #[serde(default)]
    pub result: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub assignee: String,
    #[serde(default)]
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCase {
    pub id: String,
    pub engagement_id: String,
    pub template_id: Option<String>,
    pub title: String,
    pub approach: String,
    pub category: String,
    pub status: TestCaseStatus,
    pub result: String,
    pub notes: String,
    pub assignee: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingInput {
    pub engagement_id: String,
    #[serde(default = "default_finding_code")]
    pub code: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub impact: String,
    #[serde(default)]
    pub remediation: String,
    #[serde(default)]
    pub owasp: String,
    #[serde(default)]
    pub cwe: String,
    pub status: FindingStatus,
    pub retest_status: RetestStatus,
    #[serde(default)]
    pub owner: String,
    pub probability: u8,
    pub impact_score: u8,
    #[serde(default)]
    pub test_case_ids: Vec<String>,
}

fn default_finding_code() -> String {
    "F-".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub id: String,
    pub engagement_id: String,
    pub code: String,
    pub title: String,
    pub description: String,
    pub impact: String,
    pub remediation: String,
    pub severity: Severity,
    pub probability: u8,
    pub impact_score: u8,
    pub owasp: String,
    pub cwe: String,
    pub status: FindingStatus,
    pub retest_status: RetestStatus,
    pub owner: String,
    pub created_at: String,
    pub updated_at: String,
    pub test_case_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub seq: i64,
    pub id: String,
    pub engagement_id: Option<String>,
    pub actor_id: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub metadata: serde_json::Value,
    pub prev_hash: String,
    pub hash: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeverityCounts {
    pub informational: i64,
    pub low: i64,
    pub medium: i64,
    pub high: i64,
    pub critical: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestProgress {
    pub total: i64,
    pub not_started: i64,
    pub in_progress: i64,
    pub done: i64,
    pub blocked: i64,
    pub not_applicable: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardMetrics {
    pub total_engagements: i64,
    pub active_engagements: i64,
    pub open_findings: i64,
    pub findings_by_severity: SeverityCounts,
    pub test_progress: TestProgress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResult {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceInput {
    pub engagement_id: String,
    pub finding_id: Option<String>,
    pub test_case_id: Option<String>,
    pub filename: String,
    pub mime_type: String,
    #[serde(default)]
    pub caption: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub id: String,
    pub engagement_id: String,
    pub finding_id: Option<String>,
    pub test_case_id: Option<String>,
    pub filename: String,
    pub mime_type: String,
    pub size: i64,
    pub blob_id: String,
    pub plaintext_hash: String,
    pub caption: String,
    pub created_by: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceData {
    pub evidence: Evidence,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: String,
    pub engagement_id: String,
    pub version: i64,
    pub label: String,
    pub content_hash: String,
    pub snapshot_json: String,
    pub created_by: String,
    pub created_at: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSummary {
    pub id: String,
    pub engagement_id: String,
    pub version: i64,
    pub label: String,
    pub content_hash: String,
    pub created_by: String,
    pub created_at: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    pub id: String,
    pub engagement_id: String,
    pub snapshot_id: Option<String>,
    pub reviewer_id: String,
    pub reviewer_name: String,
    pub decision: ApprovalDecision,
    pub comment: String,
    pub signature: String,
    pub signed_at: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestTemplateBundle {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub source: String,
    pub origin_id: Option<String>,
    pub template_count: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateBundleExport {
    pub filename: String,
    pub json: String,
    pub content_hash: String,
    pub template_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateBundleImportResult {
    pub bundle_id: String,
    pub name: String,
    pub version: String,
    pub template_count: usize,
    pub updated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateBundleApplyResult {
    pub created: usize,
    pub skipped: usize,
    pub test_cases: Vec<TestCase>,
}
