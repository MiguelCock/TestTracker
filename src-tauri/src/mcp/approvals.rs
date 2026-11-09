use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::model::{McpApprovalRecord, McpApprovalResult, McpApprovalState, McpApprovalStatus};

pub const COMPLETED_RETENTION: Duration = Duration::from_secs(5 * 60);
pub const MAX_PENDING: usize = 20;
pub const MAX_PENDING_PER_ENGAGEMENT: usize = 5;

#[derive(Default)]
pub struct McpApprovalBook {
    records: Mutex<HashMap<String, McpApprovalRecord>>,
}

impl McpApprovalBook {
    pub fn insert(&self, record: McpApprovalRecord) -> bool {
        let engagement_id = record.engagement_id.clone();
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        cleanup_locked(&mut records);
        let pending = records
            .values()
            .filter(|record| record.state == McpApprovalState::Pending && !record.is_expired())
            .count();
        if pending >= MAX_PENDING {
            return false;
        }
        let pending_for_engagement = records
            .values()
            .filter(|record| {
                record.state == McpApprovalState::Pending
                    && !record.is_expired()
                    && record.engagement_id == engagement_id
            })
            .count();
        if pending_for_engagement >= MAX_PENDING_PER_ENGAGEMENT {
            return false;
        }
        records.insert(record.id.clone(), record);
        true
    }

    pub fn has_capacity(&self, engagement_id: &str) -> bool {
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        cleanup_locked(&mut records);
        let pending = records
            .values()
            .filter(|record| record.state == McpApprovalState::Pending && !record.is_expired())
            .count();
        let pending_for_engagement = records
            .values()
            .filter(|record| {
                record.state == McpApprovalState::Pending
                    && !record.is_expired()
                    && record.engagement_id == engagement_id
            })
            .count();
        pending < MAX_PENDING && pending_for_engagement < MAX_PENDING_PER_ENGAGEMENT
    }

    pub fn list_pending(&self) -> Vec<McpApprovalRecord> {
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        cleanup_locked(&mut records);
        let mut pending: Vec<McpApprovalRecord> = records
            .values()
            .filter(|record| record.state == McpApprovalState::Pending && !record.is_expired())
            .cloned()
            .collect();
        pending.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        pending
    }

    pub fn pending_count(&self) -> usize {
        self.list_pending().len()
    }

    pub fn get(&self, id: &str) -> Option<McpApprovalStatus> {
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        cleanup_locked(&mut records);
        records.get(id).map(|record| McpApprovalStatus {
            approval_id: record.id.clone(),
            state: if record.state == McpApprovalState::Pending && record.is_expired() {
                McpApprovalState::Expired
            } else {
                record.state
            },
            result: record.result.clone(),
            expires_in_seconds: record
                .expires_at
                .saturating_duration_since(Instant::now())
                .as_secs(),
        })
    }

    pub fn take_pending(&self, id: &str) -> Option<McpApprovalRecord> {
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        cleanup_locked(&mut records);
        let record = records.get(id)?;
        if record.state != McpApprovalState::Pending || record.is_expired() {
            return None;
        }
        records.remove(id)
    }

    pub fn complete(&self, record: &McpApprovalRecord, result: McpApprovalResult) {
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut completed = record.clone();
        completed.state = McpApprovalState::Approved;
        completed.result = Some(result);
        records.insert(completed.id.clone(), completed);
    }

    pub fn reject(&self, record: &McpApprovalRecord) {
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut rejected = record.clone();
        rejected.state = McpApprovalState::Rejected;
        records.insert(rejected.id.clone(), rejected);
    }

    pub fn remove(&self, id: &str) {
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        records.remove(id);
    }

    pub fn restore(&self, record: McpApprovalRecord) {
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        records.insert(record.id.clone(), record);
    }

    pub fn clear(&self) {
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        records.clear();
    }
}

fn cleanup_locked(records: &mut HashMap<String, McpApprovalRecord>) {
    let now = Instant::now();
    records.retain(|_, record| now < record.expires_at + COMPLETED_RETENTION);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::agent::AgentSkill;
    use serde_json::json;

    fn record(id: &str, engagement_id: &str) -> McpApprovalRecord {
        McpApprovalRecord {
            id: id.into(),
            engagement_id: engagement_id.into(),
            engagement_name: "Synthetic".into(),
            skill: AgentSkill::TestCases,
            target_id: None,
            target_label: None,
            payload: json!({"testCases": [{"title": "Check auth"}]}),
            client: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            expires_at: Instant::now() + Duration::from_secs(15 * 60),
            state: McpApprovalState::Pending,
            result: None,
        }
    }

    #[test]
    fn enforces_pending_limits() {
        let book = McpApprovalBook::default();
        for index in 0..MAX_PENDING_PER_ENGAGEMENT {
            assert!(book.insert(record(&format!("id-{index}"), "engagement-1")));
        }
        assert!(!book.insert(record("overflow", "engagement-1")));
        assert!(book.insert(record("other", "engagement-2")));
    }

    #[test]
    fn takes_pending_once() {
        let book = McpApprovalBook::default();
        let item = record("id-1", "engagement-1");
        assert!(book.insert(item.clone()));
        assert!(book.take_pending("id-1").is_some());
        assert!(book.take_pending("id-1").is_none());
        book.complete(
            &item,
            McpApprovalResult {
                message: "Applied".into(),
                created_test_cases: 1,
                created_findings: 0,
                updated_engagement: false,
            },
        );
        assert_eq!(book.get("id-1").unwrap().state, McpApprovalState::Approved);
    }
}
