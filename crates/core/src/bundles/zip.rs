use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{AppError, AppResult};

const MAX_ENTRIES: usize = 2_000;
const MAX_ENTRY_BYTES: usize = 12 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 25 * 1024 * 1024;

pub fn create_zip(entries: &BTreeMap<String, Vec<u8>>) -> AppResult<Vec<u8>> {
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        writer
            .start_file(name, options)
            .map_err(|_| AppError::Internal("unable to create bundle archive".into()))?;
        writer
            .write_all(data)
            .map_err(|_| AppError::Internal("unable to create bundle archive".into()))?;
    }
    let cursor = writer
        .finish()
        .map_err(|_| AppError::Internal("unable to create bundle archive".into()))?;
    Ok(cursor.into_inner())
}

pub fn read_zip(bytes: &[u8]) -> AppResult<BTreeMap<String, Vec<u8>>> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| AppError::Validation("Invalid bundle archive.".into()))?;
    if archive.len() > MAX_ENTRIES {
        return Err(AppError::Validation(
            "Bundle archive contains too many files.".into(),
        ));
    }
    let mut entries = BTreeMap::new();
    let mut total = 0usize;
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|_| AppError::Validation("Invalid bundle archive.".into()))?;
        if file.is_dir() {
            return Err(AppError::Validation(
                "Bundle archive must not contain directories.".into(),
            ));
        }
        let name = file
            .name()
            .map_err(|_| AppError::Validation("Invalid bundle archive.".into()))?
            .to_string();
        validate_entry_name(&name)?;
        if entries.contains_key(&name) {
            return Err(AppError::Validation(
                "Bundle archive contains duplicate files.".into(),
            ));
        }
        let mut data = Vec::new();
        file.take((MAX_ENTRY_BYTES + 1) as u64)
            .read_to_end(&mut data)
            .map_err(|_| AppError::Validation("Unable to read bundle archive.".into()))?;
        if data.len() > MAX_ENTRY_BYTES {
            return Err(AppError::Validation(
                "Bundle archive contains an oversized file.".into(),
            ));
        }
        total += data.len();
        if total > MAX_TOTAL_BYTES {
            return Err(AppError::Validation(
                "Bundle archive is larger than 25 MB.".into(),
            ));
        }
        entries.insert(name, data);
    }
    Ok(entries)
}

fn validate_entry_name(name: &str) -> AppResult<()> {
    if name == "manifest.json" || name == "engagement.json" {
        return Ok(());
    }
    if let Some(id) = name.strip_prefix("evidence/") {
        return validate_component(id);
    }
    if let Some(id) = name.strip_prefix("documents/") {
        return validate_component(id);
    }
    Err(AppError::Validation(
        "Bundle archive contains unexpected files.".into(),
    ))
}

fn validate_component(value: &str) -> AppResult<()> {
    if value.is_empty()
        || value.len() > 100
        || value.contains('/')
        || value.contains('\\')
        || value == "."
        || value == ".."
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(AppError::Validation(
            "Bundle archive contains an invalid file name.".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_entries() {
        let mut entries = BTreeMap::new();
        entries.insert("manifest.json".into(), b"{}".to_vec());
        entries.insert("engagement.json".into(), b"{}".to_vec());
        entries.insert("evidence/one".into(), b"data".to_vec());
        let zip = create_zip(&entries).expect("zip");
        let read = read_zip(&zip).expect("read");
        assert_eq!(read, entries);
    }

    #[test]
    fn rejects_unexpected_entries() {
        let mut entries = BTreeMap::new();
        entries.insert("evil.txt".into(), b"data".to_vec());
        let zip = create_zip(&entries).expect("zip");
        assert!(read_zip(&zip).is_err());
    }
}
