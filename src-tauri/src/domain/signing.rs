use crate::domain::enums::ApprovalDecision;

pub fn approval_message(
    approval_id: &str,
    engagement_id: &str,
    snapshot_id: &str,
    decision: ApprovalDecision,
    signed_at: &str,
    content_hash: &str,
) -> Vec<u8> {
    format!(
        "testtracker-approval-v1\n{approval_id}\n{engagement_id}\n{snapshot_id}\n{}\n{signed_at}\n{content_hash}",
        decision.as_str()
    )
    .into_bytes()
}
