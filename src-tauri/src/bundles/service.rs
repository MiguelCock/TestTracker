use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rusqlite::Connection;
use uuid::Uuid;

use crate::db;
use crate::documents::model::GeneratedDocument;
use crate::domain::models::{
    ContactInput, EngagementInput, Evidence, FindingInput, ProfileSummary, RiskInput,
    ScopeAssetInput, TestCaseInput,
};
use crate::domain::severity::severity_for;
use crate::error::{AppError, AppResult};
use crate::security::{audit, crypto, identity, vault};

use super::format;
use super::model::{
    BUNDLE_FORMAT, BUNDLE_SCHEMA, BundleData, BundleDocument, BundleEvidence, BundleExportResult,
    BundleImportResult, BundleManifest,
};
use super::payload;
use super::zip;

#[allow(clippy::too_many_arguments)]
pub fn export_bundle(
    conn: &Connection,
    master_key: &[u8; 32],
    data_dir: &Path,
    engagement_id: &str,
    include_documents: bool,
    passphrase: &str,
    actor: &ProfileSummary,
    now: &str,
) -> AppResult<BundleExportResult> {
    vault::validate_passphrase(passphrase)?;
    let detail = db::engagements::detail(conn, engagement_id)?;
    let test_cases = db::test_cases::list(conn, engagement_id)?;
    let findings = db::findings::list(conn, engagement_id)?;
    let evidence = db::evidence::list(conn, engagement_id)?;
    let profile = db::profiles::get(conn, &actor.id)?;
    let signing_key = identity::decrypt_signing_key(
        master_key,
        &profile.id,
        &profile.public_key,
        &profile.encrypted_private_key,
    )?;

    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut bundle_evidence = Vec::new();
    for item in &evidence {
        let (nonce_hex, stored_path) = db::evidence::blob(conn, &item.blob_id)?;
        let path = validate_under(&data_dir.join("evidence"), &stored_path)?;
        let ciphertext = fs::read(path)?;
        let nonce = hex::decode(nonce_hex)
            .map_err(|_| AppError::Internal("invalid evidence nonce".into()))?;
        let key = crypto::derive_evidence_key(master_key, engagement_id);
        let aad = format!("testtracker-evidence-v1:{engagement_id}:{}", item.id);
        let plaintext = crypto::decrypt(&key, &nonce, &ciphertext, aad.as_bytes())?;
        if blake3::hash(&plaintext).to_hex().to_string() != item.plaintext_hash {
            return Err(AppError::Crypto);
        }
        let file = format!("evidence/{}", item.id);
        files.insert(file.clone(), plaintext.to_vec());
        bundle_evidence.push(BundleEvidence {
            id: item.id.clone(),
            filename: item.filename.clone(),
            mime_type: item.mime_type.clone(),
            size: item.size,
            plaintext_hash: item.plaintext_hash.clone(),
            caption: item.caption.clone(),
            finding_id: item.finding_id.clone(),
            test_case_id: item.test_case_id.clone(),
            created_at: item.created_at.clone(),
            file,
        });
    }

    let mut bundle_documents = Vec::new();
    if include_documents {
        for document in db::generated_documents::list(conn, engagement_id)? {
            let stored = db::generated_documents::get(conn, &document.id)?;
            let path = validate_under(&data_dir.join("documents"), &stored.path)?;
            let ciphertext = fs::read(path)?;
            let nonce = hex::decode(stored.nonce)
                .map_err(|_| AppError::Internal("invalid document nonce".into()))?;
            let key = crypto::derive_document_key(master_key, engagement_id);
            let aad = format!("testtracker-document-v1:{engagement_id}:{}", document.id);
            let plaintext = crypto::decrypt(&key, &nonce, &ciphertext, aad.as_bytes())?;
            if blake3::hash(&plaintext).to_hex().to_string() != document.content_hash {
                return Err(AppError::Crypto);
            }
            let file = format!("documents/{}", document.id);
            files.insert(file.clone(), plaintext.to_vec());
            bundle_documents.push(BundleDocument {
                id: document.id.clone(),
                kind: document.kind,
                format: document.format,
                title: document.title.clone(),
                filename: document.filename.clone(),
                size: document.size,
                content_hash: document.content_hash.clone(),
                created_at: document.created_at.clone(),
                file,
            });
        }
    }

    let data = BundleData {
        schema: BUNDLE_SCHEMA.to_string(),
        engagement: detail.engagement.clone(),
        scope_assets: detail.scope_assets,
        risks: detail.risks,
        assumptions: detail.assumptions,
        contacts: detail.contacts,
        test_cases,
        findings,
        evidence: bundle_evidence,
        documents: bundle_documents,
    };
    files.insert("engagement.json".into(), serde_json::to_vec_pretty(&data)?);
    let total: usize = files.values().map(Vec::len).sum();
    if total > format::MAX_PLAINTEXT_BYTES {
        return Err(AppError::Validation(
            "Bundle payload is larger than 25 MB.".into(),
        ));
    }
    let file_hashes = payload::file_hashes(&files);
    let payload_hash = payload::payload_hash(&file_hashes)?;
    let mut manifest = BundleManifest {
        format: BUNDLE_FORMAT.to_string(),
        version: 1,
        created_at: now.to_string(),
        engagement_id: engagement_id.to_string(),
        engagement_name: detail.engagement.name.clone(),
        sender_profile_id: actor.id.clone(),
        sender_display_name: actor.display_name.clone(),
        sender_public_key: profile.public_key.clone(),
        include_documents,
        files: file_hashes,
        payload_hash,
        signature: String::new(),
    };
    payload::sign_manifest(&mut manifest, &signing_key)?;
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec_pretty(&manifest)?,
    );
    let archive = zip::create_zip(&files)?;
    let encrypted = format::encrypt_bundle(passphrase, &archive)?;
    let filename = format!(
        "{}_{}.ttbundle",
        sanitize(&detail.engagement.name),
        chrono::Utc::now().format("%Y%m%d")
    );
    Ok(BundleExportResult {
        filename,
        size: encrypted.len() as i64,
        data_base64: BASE64.encode(&encrypted),
        engagement_id: engagement_id.to_string(),
        engagement_name: detail.engagement.name,
        evidence_count: data.evidence.len(),
        document_count: data.documents.len(),
        signature_fingerprint: payload::fingerprint(&profile.public_key),
    })
}

#[allow(clippy::too_many_arguments)]
pub fn import_bundle(
    conn: &mut Connection,
    master_key: &[u8; 32],
    data_dir: &Path,
    bytes: &[u8],
    passphrase: &str,
    actor: &ProfileSummary,
    now: &str,
) -> AppResult<BundleImportResult> {
    let plaintext = format::decrypt_bundle(bytes, passphrase)?;
    let files = zip::read_zip(&plaintext)?;
    let manifest_bytes = files
        .get("manifest.json")
        .ok_or_else(|| AppError::Validation("Bundle manifest is missing.".into()))?;
    let manifest: BundleManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|_| AppError::Validation("Invalid bundle manifest.".into()))?;
    payload::validate_manifest(&manifest)?;
    let engagement_bytes = files
        .get("engagement.json")
        .ok_or_else(|| AppError::Validation("Bundle engagement data is missing.".into()))?;
    let data: BundleData = serde_json::from_slice(engagement_bytes)
        .map_err(|_| AppError::Validation("Invalid bundle engagement data.".into()))?;
    payload::validate_data(&data)?;
    payload::validate_files(&data, &manifest, &files)?;
    let sender_fingerprint = payload::fingerprint(&manifest.sender_public_key);

    let mut created_files: Vec<PathBuf> = Vec::new();
    let result = (|| -> AppResult<BundleImportResult> {
        let tx = conn.transaction()?;
        let engagement_id = Uuid::now_v7().to_string();
        let name = imported_name(&data.engagement.name);
        let input = EngagementInput {
            name,
            client_code: data.engagement.client_code.clone(),
            status: data.engagement.status,
            start_date: data.engagement.start_date.clone(),
            end_date: data.engagement.end_date.clone(),
            application_summary: data.engagement.application_summary.clone(),
            attack_surface: data.engagement.attack_surface.clone(),
            scope_assets: data
                .scope_assets
                .iter()
                .map(|asset| ScopeAssetInput {
                    kind: asset.kind.clone(),
                    value: asset.value.clone(),
                    in_scope: asset.in_scope,
                    notes: asset.notes.clone(),
                })
                .collect(),
            risks: data
                .risks
                .iter()
                .map(|risk| RiskInput {
                    title: risk.title.clone(),
                    description: risk.description.clone(),
                    likelihood: risk.likelihood,
                    impact: risk.impact,
                    notes: risk.notes.clone(),
                })
                .collect(),
            assumptions: data
                .assumptions
                .iter()
                .map(|assumption| assumption.text.clone())
                .collect(),
            contacts: data
                .contacts
                .iter()
                .map(|contact| ContactInput {
                    name: contact.name.clone(),
                    role: contact.role.clone(),
                    email: contact.email.clone(),
                    phone: contact.phone.clone(),
                })
                .collect(),
        };
        db::engagements::insert(&tx, &engagement_id, &input, &actor.id, now)?;

        let mut test_map: HashMap<String, String> = HashMap::new();
        for test_case in &data.test_cases {
            let template_id = match test_case.template_id.as_deref() {
                Some(template) if db::test_cases::template_exists(&tx, template)? => {
                    Some(template.to_string())
                }
                _ => None,
            };
            let input = TestCaseInput {
                engagement_id: engagement_id.clone(),
                template_id,
                title: test_case.title.clone(),
                approach: test_case.approach.clone(),
                category: test_case.category.clone(),
                status: test_case.status,
                result: test_case.result.clone(),
                notes: test_case.notes.clone(),
                assignee: actor.display_name.clone(),
                sort_order: test_case.sort_order,
            };
            let created = db::test_cases::create(&tx, &input, now)?;
            test_map.insert(test_case.id.clone(), created.id);
        }

        let mut finding_map: HashMap<String, String> = HashMap::new();
        for finding in &data.findings {
            let test_case_ids: Vec<String> = finding
                .test_case_ids
                .iter()
                .filter_map(|id| test_map.get(id).cloned())
                .collect();
            let severity = severity_for(finding.probability, finding.impact_score)?;
            let code = if finding.code.trim().is_empty() {
                db::findings::next_code(&tx, &engagement_id)?
            } else {
                finding.code.clone()
            };
            let input = FindingInput {
                engagement_id: engagement_id.clone(),
                code: code.clone(),
                title: finding.title.clone(),
                description: finding.description.clone(),
                impact: finding.impact.clone(),
                remediation: finding.remediation.clone(),
                owasp: finding.owasp.clone(),
                cwe: finding.cwe.clone(),
                status: finding.status,
                retest_status: finding.retest_status,
                owner: finding.owner.clone(),
                probability: finding.probability,
                impact_score: finding.impact_score,
                test_case_ids,
            };
            let created = db::findings::create(&tx, &input, severity, &code, now)?;
            finding_map.insert(finding.id.clone(), created.id);
        }

        let evidence_key = crypto::derive_evidence_key(master_key, &engagement_id);
        for evidence in &data.evidence {
            let content = files
                .get(&evidence.file)
                .ok_or_else(|| AppError::Validation("Bundle evidence is missing.".into()))?;
            if content.len() as i64 != evidence.size
                || blake3::hash(content).to_hex().to_string() != evidence.plaintext_hash
            {
                return Err(AppError::Validation(
                    "Bundle evidence verification failed.".into(),
                ));
            }
            let evidence_id = Uuid::now_v7().to_string();
            let blob_id = Uuid::now_v7().to_string();
            let aad = format!("testtracker-evidence-v1:{engagement_id}:{evidence_id}");
            let encrypted = crypto::encrypt(&evidence_key, content, aad.as_bytes())?;
            let directory = data_dir.join("evidence").join(&engagement_id);
            fs::create_dir_all(&directory)?;
            set_dir_private(&directory)?;
            let path = directory.join(format!("{blob_id}.bin"));
            write_private(&path, &encrypted.ciphertext)?;
            created_files.push(path.clone());
            db::evidence::insert_blob(
                &tx,
                &blob_id,
                &engagement_id,
                &hex::encode(encrypted.nonce),
                &path.to_string_lossy(),
                now,
            )?;
            db::evidence::insert(
                &tx,
                &Evidence {
                    id: evidence_id.clone(),
                    engagement_id: engagement_id.clone(),
                    finding_id: evidence
                        .finding_id
                        .as_ref()
                        .and_then(|id| finding_map.get(id).cloned()),
                    test_case_id: evidence
                        .test_case_id
                        .as_ref()
                        .and_then(|id| test_map.get(id).cloned()),
                    filename: evidence.filename.clone(),
                    mime_type: evidence.mime_type.clone(),
                    size: evidence.size,
                    blob_id,
                    plaintext_hash: evidence.plaintext_hash.clone(),
                    caption: evidence.caption.clone(),
                    created_by: actor.id.clone(),
                    created_at: now.to_string(),
                },
            )?;
        }

        let document_key = crypto::derive_document_key(master_key, &engagement_id);
        for document in &data.documents {
            let content = files
                .get(&document.file)
                .ok_or_else(|| AppError::Validation("Bundle document is missing.".into()))?;
            if content.len() as i64 != document.size
                || blake3::hash(content).to_hex().to_string() != document.content_hash
            {
                return Err(AppError::Validation(
                    "Bundle document verification failed.".into(),
                ));
            }
            let document_id = Uuid::now_v7().to_string();
            let aad = format!("testtracker-document-v1:{engagement_id}:{document_id}");
            let encrypted = crypto::encrypt(&document_key, content, aad.as_bytes())?;
            let directory = data_dir.join("documents").join(&engagement_id);
            fs::create_dir_all(&directory)?;
            set_dir_private(&directory)?;
            let path = directory.join(format!("{document_id}.{}", document.format.extension()));
            write_private(&path, &encrypted.ciphertext)?;
            created_files.push(path.clone());
            db::generated_documents::insert(
                &tx,
                &db::generated_documents::StoredDocument {
                    document: GeneratedDocument {
                        id: document_id.clone(),
                        engagement_id: engagement_id.clone(),
                        kind: document.kind,
                        format: document.format,
                        title: document.title.clone(),
                        filename: document.filename.clone(),
                        size: document.size,
                        content_hash: document.content_hash.clone(),
                        created_by: actor.id.clone(),
                        created_at: now.to_string(),
                    },
                    nonce: hex::encode(encrypted.nonce),
                    path: path.to_string_lossy().to_string(),
                },
            )?;
        }

        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&engagement_id),
                actor_id: Some(&actor.id),
                action: "bundle.imported",
                entity_type: "engagement",
                entity_id: Some(&engagement_id),
                metadata: serde_json::json!({
                    "sourceEngagementId": manifest.engagement_id,
                    "sourceEngagementName": manifest.engagement_name,
                    "sender": manifest.sender_display_name,
                    "senderFingerprint": sender_fingerprint,
                    "testCaseCount": data.test_cases.len(),
                    "findingCount": data.findings.len(),
                    "evidenceCount": data.evidence.len(),
                    "documentCount": data.documents.len(),
                }),
                created_at: now,
            },
        )?;
        tx.commit()?;
        Ok(BundleImportResult {
            engagement_id,
            engagement_name: imported_name(&data.engagement.name),
            test_case_count: data.test_cases.len(),
            finding_count: data.findings.len(),
            evidence_count: data.evidence.len(),
            document_count: data.documents.len(),
            sender_display_name: manifest.sender_display_name.clone(),
            sender_fingerprint,
        })
    })();
    if result.is_err() {
        for path in created_files {
            let _ = fs::remove_file(path);
        }
    }
    result
}

fn imported_name(name: &str) -> String {
    let suffix = " (imported)";
    let max = 200usize.saturating_sub(suffix.chars().count());
    let base: String = name.chars().take(max).collect();
    format!("{}{suffix}", base.trim_end())
}

fn sanitize(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "engagement".into()
    } else {
        trimmed.to_string()
    }
}

fn validate_under(root: &Path, stored: &str) -> AppResult<PathBuf> {
    let path = PathBuf::from(stored);
    if !path.starts_with(root) {
        return Err(AppError::Internal("invalid bundle source path".into()));
    }
    Ok(path)
}

fn set_dir_private(path: &Path) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn write_private(path: &Path, contents: &[u8]) -> AppResult<()> {
    let tmp = path.with_extension("tmp");
    let mut options = fs::OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&tmp)?;
    file.write_all(contents)?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    Ok(())
}
