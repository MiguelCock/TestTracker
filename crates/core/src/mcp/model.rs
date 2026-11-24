use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ai::agent::AgentSkill;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpApprovalState {
    Pending,
    Approved,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpApprovalResult {
    pub message: String,
    pub created_test_cases: usize,
    pub created_findings: usize,
    pub updated_engagement: bool,
}

#[derive(Clone)]
pub struct McpApprovalRecord {
    pub id: String,
    pub engagement_id: String,
    pub engagement_name: String,
    pub skill: AgentSkill,
    pub target_id: Option<String>,
    pub target_label: Option<String>,
    pub payload: Value,
    pub client: Option<String>,
    pub created_at: String,
    pub expires_at: Instant,
    pub state: McpApprovalState,
    pub result: Option<McpApprovalResult>,
}

impl McpApprovalRecord {
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpApprovalDto {
    pub approval_id: String,
    pub state: McpApprovalState,
    pub engagement_id: String,
    pub engagement_name: String,
    pub skill: AgentSkill,
    pub target_id: Option<String>,
    pub target_label: Option<String>,
    pub payload: Value,
    pub client: Option<String>,
    pub created_at: String,
    pub expires_in_seconds: u64,
    pub result: Option<McpApprovalResult>,
}

impl McpApprovalDto {
    pub fn from_record(record: &McpApprovalRecord) -> Self {
        let expires_in_seconds = record
            .expires_at
            .saturating_duration_since(Instant::now())
            .as_secs();
        Self {
            approval_id: record.id.clone(),
            state: if record.state == McpApprovalState::Pending && record.is_expired() {
                McpApprovalState::Expired
            } else {
                record.state
            },
            engagement_id: record.engagement_id.clone(),
            engagement_name: record.engagement_name.clone(),
            skill: record.skill,
            target_id: record.target_id.clone(),
            target_label: record.target_label.clone(),
            payload: record.payload.clone(),
            client: record.client.clone(),
            created_at: record.created_at.clone(),
            expires_in_seconds,
            result: record.result.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpApprovalStatus {
    pub approval_id: String,
    pub state: McpApprovalState,
    pub result: Option<McpApprovalResult>,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub running: bool,
    pub endpoint: Option<String>,
    pub port: u16,
    pub has_token: bool,
    pub pending_count: usize,
    pub started_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpProposalAccepted {
    pub approval_id: String,
    pub state: McpApprovalState,
    pub expires_in_seconds: u64,
    pub message: String,
}
