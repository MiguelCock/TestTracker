use std::io::{Cursor, Read, Write};

use ed25519_dalek::{Signer, SigningKey};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{AppError, AppResult};
use crate::plugins::model::PluginManifest;
use crate::security::identity;

const MAX_PACKAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_WASM_BYTES: usize = 5 * 1024 * 1024;
const MAX_SIGNATURE_BYTES: usize = 1024;
const SIGNATURE_PREFIX: &[u8] = b"testtracker-plugin-v1";

pub struct ParsedPackage {
    pub manifest: PluginManifest,
    pub manifest_bytes: Vec<u8>,
    pub wasm: Vec<u8>,
    pub signature: String,
}

pub fn package_digest(manifest: &[u8], wasm: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(SIGNATURE_PREFIX);
    hasher.update(manifest);
    hasher.update(wasm);
    *hasher.finalize().as_bytes()
}

pub fn parse_package(bytes: &[u8]) -> AppResult<ParsedPackage> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(AppError::Validation(
            "Plugin package is larger than 8 MB.".into(),
        ));
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| AppError::Validation("Invalid plugin package.".into()))?;
    if archive.len() > 3 {
        return Err(AppError::Validation(
            "Plugin package contains unexpected files.".into(),
        ));
    }

    let mut manifest_bytes: Option<Vec<u8>> = None;
    let mut wasm: Option<Vec<u8>> = None;
    let mut signature: Option<String> = None;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|_| AppError::Validation("Invalid plugin package.".into()))?;
        if file.is_dir() {
            return Err(AppError::Validation(
                "Plugin package must not contain directories.".into(),
            ));
        }
        let name = file
            .name()
            .map_err(|_| AppError::Validation("Invalid plugin package.".into()))?
            .to_string();
        match name.as_str() {
            "manifest.json" => {
                if manifest_bytes.is_some() {
                    return Err(AppError::Validation(
                        "Plugin package contains duplicate files.".into(),
                    ));
                }
                manifest_bytes = Some(read_limited(&mut file, MAX_MANIFEST_BYTES)?);
            }
            "plugin.wasm" => {
                if wasm.is_some() {
                    return Err(AppError::Validation(
                        "Plugin package contains duplicate files.".into(),
                    ));
                }
                wasm = Some(read_limited(&mut file, MAX_WASM_BYTES)?);
            }
            "signature" => {
                if signature.is_some() {
                    return Err(AppError::Validation(
                        "Plugin package contains duplicate files.".into(),
                    ));
                }
                let data = read_limited(&mut file, MAX_SIGNATURE_BYTES)?;
                let text = String::from_utf8(data)
                    .map_err(|_| AppError::Validation("Invalid plugin signature.".into()))?;
                signature = Some(text.trim().to_string());
            }
            _ => {
                return Err(AppError::Validation(
                    "Plugin package contains unexpected files.".into(),
                ));
            }
        }
    }

    let manifest_bytes =
        manifest_bytes.ok_or_else(|| AppError::Validation("Plugin manifest is missing.".into()))?;
    let wasm = wasm.ok_or_else(|| AppError::Validation("Plugin WASM file is missing.".into()))?;
    let signature =
        signature.ok_or_else(|| AppError::Validation("Plugin signature is missing.".into()))?;
    if !wasm.starts_with(b"\0asm") {
        return Err(AppError::Validation(
            "Plugin WASM file is not a valid WebAssembly module.".into(),
        ));
    }
    if signature.len() != 128 || !signature.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::Validation("Invalid plugin signature.".into()));
    }
    let manifest: PluginManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|_| AppError::Validation("Invalid plugin manifest.".into()))?;
    manifest.validate()?;

    Ok(ParsedPackage {
        manifest,
        manifest_bytes,
        wasm,
        signature,
    })
}

pub fn verify_signature(package: &ParsedPackage, public_key_hex: &str) -> bool {
    let digest = package_digest(&package.manifest_bytes, &package.wasm);
    identity::verify_hex(public_key_hex, &digest, &package.signature)
}

pub fn sign_package(manifest: &[u8], wasm: &[u8], key: &SigningKey) -> String {
    let digest = package_digest(manifest, wasm);
    hex::encode(key.sign(&digest).to_bytes())
}

pub fn create_package(manifest: &[u8], wasm: &[u8], signature: &str) -> AppResult<Vec<u8>> {
    if manifest.len() > MAX_MANIFEST_BYTES {
        return Err(AppError::Validation("Plugin manifest is too large.".into()));
    }
    if wasm.len() > MAX_WASM_BYTES {
        return Err(AppError::Validation(
            "Plugin WASM file is too large.".into(),
        ));
    }
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("manifest.json", options)
        .map_err(|_| AppError::Internal("unable to write plugin package".into()))?;
    writer
        .write_all(manifest)
        .map_err(|_| AppError::Internal("unable to write plugin package".into()))?;
    writer
        .start_file("plugin.wasm", options)
        .map_err(|_| AppError::Internal("unable to write plugin package".into()))?;
    writer
        .write_all(wasm)
        .map_err(|_| AppError::Internal("unable to write plugin package".into()))?;
    writer
        .start_file("signature", options)
        .map_err(|_| AppError::Internal("unable to write plugin package".into()))?;
    writer
        .write_all(signature.as_bytes())
        .map_err(|_| AppError::Internal("unable to write plugin package".into()))?;
    let cursor = writer
        .finish()
        .map_err(|_| AppError::Internal("unable to write plugin package".into()))?;
    Ok(cursor.into_inner())
}

pub fn generate_keypair() -> (String, String) {
    let key = identity::generate();
    let secret_bytes = identity::private_key_bytes(&key);
    let secret = hex::encode(secret_bytes.as_slice());
    (secret, identity::public_key_hex(&key))
}

pub fn signing_key_from_hex(secret_hex: &str) -> AppResult<SigningKey> {
    let bytes =
        hex::decode(secret_hex).map_err(|_| AppError::Validation("Invalid secret key.".into()))?;
    let array = <[u8; 32]>::try_from(bytes.as_slice())
        .map_err(|_| AppError::Validation("Invalid secret key.".into()))?;
    Ok(SigningKey::from_bytes(&array))
}

fn read_limited(reader: &mut impl Read, limit: usize) -> AppResult<Vec<u8>> {
    let mut data = Vec::new();
    reader
        .take((limit + 1) as u64)
        .read_to_end(&mut data)
        .map_err(|_| AppError::Validation("Unable to read plugin package.".into()))?;
    if data.len() > limit {
        return Err(AppError::Validation(
            "Plugin package contains an oversized file.".into(),
        ));
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest(publisher_id: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "id": "com.example.checklist",
            "name": "Checklist Helper",
            "version": "1.0.0",
            "publisherId": publisher_id,
            "description": "Synthetic plugin",
            "runtime": "wasm",
            "entry": "plugin.wasm",
            "capabilities": ["engagement.read"]
        }))
        .expect("manifest")
    }

    fn sample_wasm() -> Vec<u8> {
        wat::parse_str(
            r#"(module
                (memory (export "memory") 1)
                (func (export "alloc") (param i32) (result i32) i32.const 1024)
                (func (export "run") (param i32 i32) (result i64) i64.const 0)
            )"#,
        )
        .expect("wasm")
    }

    #[test]
    fn signs_and_verifies_package() {
        let (secret, public) = generate_keypair();
        let key = signing_key_from_hex(&secret).expect("key");
        let manifest = sample_manifest("com.example.publisher");
        let wasm = sample_wasm();
        let signature = sign_package(&manifest, &wasm, &key);
        let package = create_package(&manifest, &wasm, &signature).expect("package");
        let parsed = parse_package(&package).expect("parse");
        assert!(verify_signature(&parsed, &public));
        assert_eq!(parsed.manifest.id, "com.example.checklist");
    }

    #[test]
    fn rejects_tampered_manifest() {
        let (secret, public) = generate_keypair();
        let key = signing_key_from_hex(&secret).expect("key");
        let manifest = sample_manifest("com.example.publisher");
        let wasm = sample_wasm();
        let signature = sign_package(&manifest, &wasm, &key);
        let mut tampered = manifest.clone();
        tampered.push(b' ');
        let package = create_package(&tampered, &wasm, &signature).expect("package");
        let parsed = parse_package(&package).expect("parse");
        assert!(!verify_signature(&parsed, &public));
    }

    #[test]
    fn rejects_unknown_package_files() {
        let options = SimpleFileOptions::default();
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer.start_file("evil.txt", options).expect("start");
        writer.write_all(b"nope").expect("write");
        let bytes = writer.finish().expect("finish").into_inner();
        assert!(parse_package(&bytes).is_err());
    }
}
