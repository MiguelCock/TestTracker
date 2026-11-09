use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDraft {
    #[serde(default)]
    pub source_name: String,
    #[serde(default = "default_source_format")]
    pub source_format: String,
    pub engagement: ImportEngagement,
    #[serde(default)]
    pub scope_assets: Vec<ImportScopeAsset>,
    #[serde(default)]
    pub risks: Vec<ImportRisk>,
    #[serde(default)]
    pub assumptions: Vec<String>,
    #[serde(default)]
    pub contacts: Vec<ImportContact>,
    #[serde(default)]
    pub test_cases: Vec<ImportTestCase>,
    #[serde(default)]
    pub findings: Vec<ImportFinding>,
}

fn default_source_format() -> String {
    "unknown".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportEngagement {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub client_code: String,
    #[serde(default)]
    pub start_date: String,
    #[serde(default)]
    pub end_date: String,
    #[serde(default)]
    pub application_summary: String,
    #[serde(default)]
    pub attack_surface: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportScopeAsset {
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub value: String,
    #[serde(default = "default_true")]
    pub in_scope: bool,
    #[serde(default)]
    pub notes: String,
}

fn default_kind() -> String {
    "other".into()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRisk {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_score")]
    pub likelihood: u8,
    #[serde(default = "default_score")]
    pub impact: u8,
    #[serde(default)]
    pub notes: String,
}

fn default_score() -> u8 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportContact {
    #[serde(default)]
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
pub struct ImportTestCase {
    pub title: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub approach: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub result: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFinding {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub impact: String,
    #[serde(default)]
    pub remediation: String,
    #[serde(default = "default_score")]
    pub probability: u8,
    #[serde(default = "default_score")]
    pub impact_score: u8,
    #[serde(default)]
    pub owasp: String,
    #[serde(default)]
    pub cwe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportExtraction {
    pub extraction_id: String,
    pub filename: String,
    pub format: String,
    pub text_preview: String,
    pub text_chars: usize,
    pub truncated: bool,
    pub draft_id: Option<String>,
    pub draft: Option<ImportDraft>,
}

#[derive(Clone)]
pub struct ImportExtractionRecord {
    pub id: String,
    pub filename: String,
    pub format: String,
    pub text: String,
    pub expires_at: std::time::Instant,
}

impl ImportExtractionRecord {
    pub fn is_expired(&self) -> bool {
        std::time::Instant::now() >= self.expires_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDraftDto {
    pub draft_id: String,
    pub draft: ImportDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreviewDto {
    pub consent_id: String,
    pub provider_name: String,
    pub model: String,
    pub redacted_prompt: String,
    pub redaction_count: usize,
    pub redactions: Vec<String>,
    pub expires_in_seconds: u64,
    pub is_local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportApplyResult {
    pub engagement_id: String,
    pub engagement_name: String,
    pub test_case_count: usize,
    pub finding_count: usize,
}

#[derive(Clone)]
pub struct ImportDraftRecord {
    pub id: String,
    pub draft: ImportDraft,
    pub expires_at: std::time::Instant,
}

impl ImportDraftRecord {
    pub fn is_expired(&self) -> bool {
        std::time::Instant::now() >= self.expires_at
    }
}

pub struct ImportPreviewRecord {
    pub id: String,
    pub provider_id: String,
    pub provider_name: String,
    pub source_name: String,
    pub kind: crate::ai::model::AiProviderKind,
    pub base_url: String,
    pub model: String,
    pub api_key: zeroize::Zeroizing<String>,
    pub prompt: String,
    pub prompt_hash: String,
    pub expires_at: std::time::Instant,
}

impl ImportPreviewRecord {
    pub fn is_expired(&self) -> bool {
        std::time::Instant::now() >= self.expires_at
    }
}

pub fn validate_draft(draft: &ImportDraft) -> AppResult<()> {
    if draft.engagement.name.trim().is_empty() || draft.engagement.name.chars().count() > 200 {
        return Err(AppError::Validation(
            "The imported engagement needs a name of at most 200 characters.".into(),
        ));
    }
    if draft.scope_assets.len() > 500
        || draft.risks.len() > 200
        || draft.assumptions.len() > 200
        || draft.contacts.len() > 100
        || draft.test_cases.len() > 5_000
        || draft.findings.len() > 1_000
    {
        return Err(AppError::Validation(
            "The import contains too many records.".into(),
        ));
    }
    for test_case in &draft.test_cases {
        if test_case.title.trim().is_empty() || test_case.title.chars().count() > 300 {
            return Err(AppError::Validation(
                "The import contains an invalid test-case title.".into(),
            ));
        }
    }
    for finding in &draft.findings {
        if finding.title.trim().is_empty() || finding.title.chars().count() > 300 {
            return Err(AppError::Validation(
                "The import contains an invalid finding title.".into(),
            ));
        }
        if finding.probability > 9 || finding.impact_score > 9 {
            return Err(AppError::Validation(
                "The import contains an invalid finding severity.".into(),
            ));
        }
    }
    Ok(())
}
