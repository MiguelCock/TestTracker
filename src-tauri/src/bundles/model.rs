use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::documents::model::{DocumentFormat, DocumentKind};
use crate::domain::models::{Assumption, Contact, Engagement, Finding, Risk, ScopeAsset, TestCase};

pub const BUNDLE_SCHEMA: &str = "testtracker.engagement.v1";
pub const BUNDLE_FORMAT: &str = "testtracker.bundle.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleManifest {
    pub format: String,
    pub version: u32,
    pub created_at: String,
    pub engagement_id: String,
    pub engagement_name: String,
    pub sender_profile_id: String,
    pub sender_display_name: String,
    pub sender_public_key: String,
    pub include_documents: bool,
    pub files: BTreeMap<String, String>,
    pub payload_hash: String,
    pub signature: String,
}

impl BundleManifest {
    pub fn signing_bytes(&self) -> crate::error::AppResult<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature.clear();
        Ok(serde_json::to_vec(&unsigned)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleData {
    pub schema: String,
    pub engagement: Engagement,
    pub scope_assets: Vec<ScopeAsset>,
    pub risks: Vec<Risk>,
    pub assumptions: Vec<Assumption>,
    pub contacts: Vec<Contact>,
    pub test_cases: Vec<TestCase>,
    pub findings: Vec<Finding>,
    pub evidence: Vec<BundleEvidence>,
    pub documents: Vec<BundleDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleEvidence {
    pub id: String,
    pub filename: String,
    pub mime_type: String,
    pub size: i64,
    pub plaintext_hash: String,
    pub caption: String,
    pub finding_id: Option<String>,
    pub test_case_id: Option<String>,
    pub created_at: String,
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleDocument {
    pub id: String,
    pub kind: DocumentKind,
    pub format: DocumentFormat,
    pub title: String,
    pub filename: String,
    pub size: i64,
    pub content_hash: String,
    pub created_at: String,
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleExportResult {
    pub filename: String,
    pub size: i64,
    pub data_base64: String,
    pub engagement_id: String,
    pub engagement_name: String,
    pub evidence_count: usize,
    pub document_count: usize,
    pub signature_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleImportResult {
    pub engagement_id: String,
    pub engagement_name: String,
    pub test_case_count: usize,
    pub finding_count: usize,
    pub evidence_count: usize,
    pub document_count: usize,
    pub sender_display_name: String,
    pub sender_fingerprint: String,
}
