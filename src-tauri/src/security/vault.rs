use std::fs;
use std::io::Write;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::error::{AppError, AppResult};
use crate::security::crypto::{
    self, KEY_LEN, KdfParams, NONCE_LEN, SALT_LEN, derive_key, random_bytes,
};

pub const MIN_PASSPHRASE_LEN: usize = 12;
const VAULT_VERSION: u32 = 1;
const VAULT_AAD: &[u8] = b"testtracker-vault-v1";

#[derive(Debug, Serialize, Deserialize)]
struct VaultFile {
    version: u32,
    salt: String,
    nonce: String,
    ciphertext: String,
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
}

pub fn is_initialized(path: &Path) -> bool {
    path.is_file()
}

pub fn create(path: &Path, passphrase: &str) -> AppResult<Zeroizing<[u8; KEY_LEN]>> {
    validate_passphrase(passphrase)?;
    if path.exists() {
        return Err(AppError::Conflict("vault already exists".into()));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let params = KdfParams::default();
    let salt = random_bytes::<SALT_LEN>();
    let kek = derive_key(passphrase, &salt, &params)?;
    let master_key = Zeroizing::new(random_bytes::<KEY_LEN>());
    let encrypted = crypto::encrypt(&kek, master_key.as_ref(), VAULT_AAD)?;

    let file = VaultFile {
        version: VAULT_VERSION,
        salt: BASE64.encode(salt),
        nonce: BASE64.encode(encrypted.nonce),
        ciphertext: BASE64.encode(encrypted.ciphertext),
        m_cost: params.m_cost,
        t_cost: params.t_cost,
        p_cost: params.p_cost,
    };
    let json = serde_json::to_vec_pretty(&file)?;
    write_private(path, &json)?;
    Ok(master_key)
}

pub fn unlock(path: &Path, passphrase: &str) -> AppResult<Zeroizing<[u8; KEY_LEN]>> {
    if !path.is_file() {
        return Err(AppError::NotInitialized);
    }
    let json = fs::read(path)?;
    let file: VaultFile = serde_json::from_slice(&json).map_err(|_| AppError::Crypto)?;
    if file.version != VAULT_VERSION {
        return Err(AppError::Crypto);
    }
    let salt = BASE64.decode(file.salt).map_err(|_| AppError::Crypto)?;
    let nonce = BASE64.decode(file.nonce).map_err(|_| AppError::Crypto)?;
    let ciphertext = BASE64
        .decode(file.ciphertext)
        .map_err(|_| AppError::Crypto)?;
    if salt.len() != SALT_LEN || nonce.len() != NONCE_LEN {
        return Err(AppError::Crypto);
    }
    let params = KdfParams {
        m_cost: file.m_cost,
        t_cost: file.t_cost,
        p_cost: file.p_cost,
    };
    let kek = derive_key(passphrase, &salt, &params)?;
    let master_key = crypto::decrypt(&kek, &nonce, &ciphertext, VAULT_AAD)?;
    if master_key.len() != KEY_LEN {
        return Err(AppError::Crypto);
    }
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    key.copy_from_slice(&master_key);
    Ok(key)
}

pub fn validate_passphrase(passphrase: &str) -> AppResult<()> {
    if passphrase.chars().count() < MIN_PASSPHRASE_LEN {
        return Err(AppError::Validation(format!(
            "Passphrase must be at least {MIN_PASSPHRASE_LEN} characters."
        )));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_unlock_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("vault.json");
        let a = create(&path, "correct horse battery staple").expect("create");
        let b = unlock(&path, "correct horse battery staple").expect("unlock");
        assert_eq!(*a, *b);
        assert!(is_initialized(&path));
    }

    #[test]
    fn wrong_passphrase_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("vault.json");
        create(&path, "correct horse battery staple").expect("create");
        assert!(unlock(&path, "incorrect horse battery staple").is_err());
    }

    #[test]
    fn short_passphrase_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("vault.json");
        assert!(create(&path, "short").is_err());
        assert!(!is_initialized(&path));
    }

    #[test]
    fn duplicate_init_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("vault.json");
        create(&path, "correct horse battery staple").expect("create");
        assert!(matches!(
            create(&path, "another correct horse battery"),
            Err(AppError::Conflict(_))
        ));
    }
}
