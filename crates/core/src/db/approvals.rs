use rusqlite::Connection;

use crate::domain::enums::ApprovalDecision;
use crate::error::AppResult;

pub struct StoredApproval {
    pub id: String,
    pub engagement_id: String,
    pub snapshot_id: Option<String>,
    pub reviewer_id: String,
    pub reviewer_name: String,
    pub public_key: String,
    pub decision: ApprovalDecision,
    pub comment: String,
    pub signature: String,
    pub signed_at: String,
}

pub struct NewApproval<'a> {
    pub id: &'a str,
    pub engagement_id: &'a str,
    pub snapshot_id: &'a str,
    pub reviewer_id: &'a str,
    pub decision: ApprovalDecision,
    pub comment: &'a str,
    pub signature: &'a str,
    pub signed_at: &'a str,
}

pub fn insert(conn: &Connection, approval: NewApproval<'_>) -> AppResult<()> {
    conn.execute(
        "INSERT INTO approvals (
            id, engagement_id, document_id, snapshot_id, reviewer_id, decision,
            comment, signature, signed_at
         ) VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            approval.id,
            approval.engagement_id,
            approval.snapshot_id,
            approval.reviewer_id,
            approval.decision.as_str(),
            approval.comment,
            approval.signature,
            approval.signed_at,
        ],
    )?;
    Ok(())
}

pub fn list(conn: &Connection, engagement_id: &str) -> AppResult<Vec<StoredApproval>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.engagement_id, a.snapshot_id, a.reviewer_id, p.display_name,
                p.public_key, a.decision, a.comment, a.signature, a.signed_at
         FROM approvals a
         JOIN profiles p ON p.id = a.reviewer_id
         WHERE a.engagement_id = ?1
         ORDER BY a.signed_at DESC",
    )?;
    let rows = stmt.query_map([engagement_id], map_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(to_stored(row?)?);
    }
    Ok(out)
}

type ApprovalRow = (
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApprovalRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
    ))
}

fn to_stored(row: ApprovalRow) -> AppResult<StoredApproval> {
    let (
        id,
        engagement_id,
        snapshot_id,
        reviewer_id,
        reviewer_name,
        public_key,
        decision,
        comment,
        signature,
        signed_at,
    ) = row;
    Ok(StoredApproval {
        id,
        engagement_id,
        snapshot_id,
        reviewer_id,
        reviewer_name,
        public_key,
        decision: ApprovalDecision::from_db(&decision)?,
        comment,
        signature,
        signed_at,
    })
}
