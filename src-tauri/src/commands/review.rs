use tauri::State;
use uuid::Uuid;

use crate::db;
use crate::domain::enums::ApprovalDecision;
use crate::domain::models::{Approval, Snapshot, SnapshotSummary};
use crate::domain::rbac;
use crate::domain::signing::approval_message;
use crate::error::{AppError, AppResult};
use crate::security::{audit, identity};
use crate::state::AppState;

#[tauri::command]
pub async fn create_snapshot(
    state: State<'_, AppState>,
    engagement_id: String,
    label: String,
) -> AppResult<Snapshot> {
    let label = label.trim().to_string();
    if label.chars().count() > 200 {
        return Err(AppError::Validation(
            "Snapshot label must be at most 200 characters.".into(),
        ));
    }
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let detail = db::engagements::detail(&session.conn, &engagement_id)?;
        let test_cases = db::test_cases::list(&session.conn, &engagement_id)?;
        let findings = db::findings::list(&session.conn, &engagement_id)?;
        let evidence = db::evidence::list(&session.conn, &engagement_id)?;
        let version = detail.engagement.version;
        let payload = serde_json::json!({
            "schema": "testtracker.snapshot.v1",
            "engagement": detail.engagement,
            "scopeAssets": detail.scope_assets,
            "risks": detail.risks,
            "assumptions": detail.assumptions,
            "contacts": detail.contacts,
            "testCases": test_cases,
            "findings": findings,
            "evidence": evidence,
        });
        let snapshot_json = serde_json::to_string_pretty(&payload)?;
        let content_hash = blake3::hash(snapshot_json.as_bytes()).to_hex().to_string();
        let actor = session.profile.id.clone();
        let now = db::now();
        let snapshot = Snapshot {
            id: Uuid::now_v7().to_string(),
            engagement_id: engagement_id.clone(),
            version,
            label,
            content_hash: content_hash.clone(),
            snapshot_json,
            created_by: actor.clone(),
            created_at: now.clone(),
            verified: true,
        };
        let tx = session.conn.transaction()?;
        db::snapshots::insert(&tx, &snapshot)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&engagement_id),
                actor_id: Some(&actor),
                action: "snapshot.created",
                entity_type: "snapshot",
                entity_id: Some(&snapshot.id),
                metadata: serde_json::json!({
                    "version": version,
                    "contentHash": content_hash,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(snapshot)
    })
}

#[tauri::command]
pub async fn list_snapshots(
    state: State<'_, AppState>,
    engagement_id: String,
) -> AppResult<Vec<SnapshotSummary>> {
    state.with_session(|session| db::snapshots::list(&session.conn, &engagement_id))
}

#[tauri::command]
pub async fn get_snapshot(state: State<'_, AppState>, id: String) -> AppResult<Snapshot> {
    state.with_session(|session| db::snapshots::get(&session.conn, &id))
}

#[tauri::command]
pub async fn sign_off(
    state: State<'_, AppState>,
    engagement_id: String,
    snapshot_id: String,
    decision: ApprovalDecision,
    comment: String,
) -> AppResult<Approval> {
    if comment.chars().count() > 5_000 {
        return Err(AppError::Validation(
            "Approval comment must be at most 5,000 characters.".into(),
        ));
    }
    state.with_session(|session| {
        if !rbac::can_approve(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        if !db::snapshots::belongs_to_engagement(&session.conn, &snapshot_id, &engagement_id)? {
            return Err(AppError::Validation(
                "The snapshot does not belong to this engagement.".into(),
            ));
        }
        let snapshot = db::snapshots::get(&session.conn, &snapshot_id)?;
        let computed_hash = blake3::hash(snapshot.snapshot_json.as_bytes())
            .to_hex()
            .to_string();
        if computed_hash != snapshot.content_hash || !snapshot.verified {
            return Err(AppError::Crypto);
        }
        let profile = db::profiles::get(&session.conn, &session.profile.id)?;
        let signing_key = identity::decrypt_signing_key(
            &session.master_key,
            &profile.id,
            &profile.public_key,
            &profile.encrypted_private_key,
        )?;
        let approval_id = Uuid::now_v7().to_string();
        let signed_at = db::now();
        let message = approval_message(
            &approval_id,
            &engagement_id,
            &snapshot_id,
            decision,
            &signed_at,
            &snapshot.content_hash,
        );
        let signature = identity::sign_hex(&signing_key, &message);
        let actor = session.profile.id.clone();
        let tx = session.conn.transaction()?;
        db::approvals::insert(
            &tx,
            db::approvals::NewApproval {
                id: &approval_id,
                engagement_id: &engagement_id,
                snapshot_id: &snapshot_id,
                reviewer_id: &actor,
                decision,
                comment: comment.trim(),
                signature: &signature,
                signed_at: &signed_at,
            },
        )?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&engagement_id),
                actor_id: Some(&actor),
                action: "approval.signed",
                entity_type: "approval",
                entity_id: Some(&approval_id),
                metadata: serde_json::json!({
                    "decision": decision.as_str(),
                    "snapshotId": snapshot_id,
                    "contentHash": snapshot.content_hash,
                }),
                created_at: &signed_at,
            },
        )?;
        tx.commit()?;
        Ok(Approval {
            id: approval_id,
            engagement_id,
            snapshot_id: Some(snapshot_id),
            reviewer_id: session.profile.id.clone(),
            reviewer_name: session.profile.display_name.clone(),
            decision,
            comment: comment.trim().to_string(),
            signature,
            signed_at,
            verified: true,
        })
    })
}

#[tauri::command]
pub async fn list_approvals(
    state: State<'_, AppState>,
    engagement_id: String,
) -> AppResult<Vec<Approval>> {
    state.with_session(|session| {
        let stored = db::approvals::list(&session.conn, &engagement_id)?;
        let mut out = Vec::with_capacity(stored.len());
        for approval in stored {
            let verified = match approval.snapshot_id.as_deref() {
                Some(snapshot_id) => match db::snapshots::get(&session.conn, snapshot_id) {
                    Ok(snapshot) => {
                        let signed_snapshot: Option<&str> = Some(snapshot_id);
                        if let Some(signed_snapshot) = signed_snapshot {
                            let message = approval_message(
                                &approval.id,
                                &approval.engagement_id,
                                signed_snapshot,
                                approval.decision,
                                &approval.signed_at,
                                &snapshot.content_hash,
                            );
                            identity::verify_hex(
                                &approval.public_key,
                                &message,
                                &approval.signature,
                            )
                        } else {
                            false
                        }
                    }
                    Err(_) => false,
                },
                None => false,
            };
            out.push(Approval {
                id: approval.id,
                engagement_id: approval.engagement_id,
                snapshot_id: approval.snapshot_id,
                reviewer_id: approval.reviewer_id,
                reviewer_name: approval.reviewer_name,
                decision: approval.decision,
                comment: approval.comment,
                signature: approval.signature,
                signed_at: approval.signed_at,
                verified,
            });
        }
        Ok(out)
    })
}
