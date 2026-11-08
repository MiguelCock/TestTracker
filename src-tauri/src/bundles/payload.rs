use std::collections::BTreeMap;

use ed25519_dalek::SigningKey;

use crate::error::{AppError, AppResult};
use crate::security::identity;

use super::model::{BUNDLE_FORMAT, BUNDLE_SCHEMA, BundleData, BundleManifest};

pub fn file_hashes(files: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, String> {
    files
        .iter()
        .map(|(name, data)| (name.clone(), blake3::hash(data).to_hex().to_string()))
        .collect()
}

pub fn payload_hash(files: &BTreeMap<String, String>) -> AppResult<String> {
    let bytes = serde_json::to_vec(files)?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

pub fn sign_manifest(manifest: &mut BundleManifest, key: &SigningKey) -> AppResult<()> {
    manifest.signature.clear();
    let bytes = manifest.signing_bytes()?;
    manifest.signature = identity::sign_hex(key, &bytes);
    Ok(())
}

pub fn verify_manifest(manifest: &BundleManifest) -> bool {
    if !identity::validate_public_key(&manifest.sender_public_key) {
        return false;
    }
    let Ok(bytes) = manifest.signing_bytes() else {
        return false;
    };
    identity::verify_hex(&manifest.sender_public_key, &bytes, &manifest.signature)
}

pub fn fingerprint(public_key_hex: &str) -> String {
    let hash = blake3::hash(public_key_hex.as_bytes()).to_hex().to_string();
    hash[..16].to_string()
}

pub fn validate_manifest(manifest: &BundleManifest) -> AppResult<()> {
    if manifest.format != BUNDLE_FORMAT || manifest.version != 1 {
        return Err(AppError::Validation("Unsupported bundle format.".into()));
    }
    if !verify_manifest(manifest) {
        return Err(AppError::Validation(
            "Bundle signature verification failed.".into(),
        ));
    }
    Ok(())
}

pub fn validate_data(data: &BundleData) -> AppResult<()> {
    if data.schema != BUNDLE_SCHEMA {
        return Err(AppError::Validation(
            "Unsupported bundle engagement schema.".into(),
        ));
    }
    if data.scope_assets.len() > 500
        || data.risks.len() > 200
        || data.assumptions.len() > 200
        || data.contacts.len() > 100
        || data.test_cases.len() > 5_000
        || data.findings.len() > 1_000
        || data.evidence.len() > 500
        || data.documents.len() > 200
    {
        return Err(AppError::Validation(
            "Bundle contains too many records.".into(),
        ));
    }
    if data.engagement.name.trim().is_empty() || data.engagement.name.chars().count() > 200 {
        return Err(AppError::Validation(
            "Bundle engagement name is invalid.".into(),
        ));
    }
    for test_case in &data.test_cases {
        if test_case.title.trim().is_empty() || test_case.title.chars().count() > 300 {
            return Err(AppError::Validation(
                "Bundle contains an invalid test-case title.".into(),
            ));
        }
    }
    for finding in &data.findings {
        if finding.title.trim().is_empty() || finding.title.chars().count() > 300 {
            return Err(AppError::Validation(
                "Bundle contains an invalid finding title.".into(),
            ));
        }
        if finding.probability > 9 || finding.impact_score > 9 {
            return Err(AppError::Validation(
                "Bundle contains an invalid finding severity.".into(),
            ));
        }
    }
    for evidence in &data.evidence {
        if evidence.filename.trim().is_empty() || evidence.filename.chars().count() > 255 {
            return Err(AppError::Validation(
                "Bundle contains an invalid evidence filename.".into(),
            ));
        }
        if evidence.size < 0 || evidence.size > 8 * 1024 * 1024 {
            return Err(AppError::Validation(
                "Bundle contains oversized evidence.".into(),
            ));
        }
    }
    for document in &data.documents {
        if document.filename.trim().is_empty() || document.filename.chars().count() > 255 {
            return Err(AppError::Validation(
                "Bundle contains an invalid document filename.".into(),
            ));
        }
        if document.size < 0 || document.size > 12 * 1024 * 1024 {
            return Err(AppError::Validation(
                "Bundle contains an oversized document.".into(),
            ));
        }
    }
    Ok(())
}

pub fn validate_files(
    data: &BundleData,
    manifest: &BundleManifest,
    files: &BTreeMap<String, Vec<u8>>,
) -> AppResult<()> {
    if !files.contains_key("engagement.json") {
        return Err(AppError::Validation(
            "Bundle engagement data is missing.".into(),
        ));
    }
    for name in files.keys() {
        if name == "manifest.json" {
            continue;
        }
        if !manifest.files.contains_key(name) {
            return Err(AppError::Validation(
                "Bundle contains an undeclared file.".into(),
            ));
        }
    }
    for (name, expected_hash) in &manifest.files {
        let Some(data) = files.get(name) else {
            return Err(AppError::Validation(
                "Bundle is missing a declared file.".into(),
            ));
        };
        if &blake3::hash(data).to_hex().to_string() != expected_hash {
            return Err(AppError::Validation(
                "Bundle file hash verification failed.".into(),
            ));
        }
    }
    let expected_payload = payload_hash(&manifest.files)?;
    if expected_payload != manifest.payload_hash {
        return Err(AppError::Validation(
            "Bundle payload hash verification failed.".into(),
        ));
    }
    for evidence in &data.evidence {
        if !manifest.files.contains_key(&evidence.file) {
            return Err(AppError::Validation(
                "Bundle evidence is not declared in the manifest.".into(),
            ));
        }
    }
    for document in &data.documents {
        if !manifest.files.contains_key(&document.file) {
            return Err(AppError::Validation(
                "Bundle document is not declared in the manifest.".into(),
            ));
        }
    }
    Ok(())
}
