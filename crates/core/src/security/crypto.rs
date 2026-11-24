use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use zeroize::Zeroizing;

use crate::error::{AppError, AppResult};

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 24;
pub const SALT_LEN: usize = 32;

pub struct KdfParams {
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            m_cost: 64 * 1024,
            t_cost: 3,
            p_cost: 4,
        }
    }
}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut out = [0u8; N];
    rand::rng().fill_bytes(&mut out);
    out
}

pub fn derive_key(
    passphrase: &str,
    salt: &[u8],
    params: &KdfParams,
) -> AppResult<Zeroizing<[u8; KEY_LEN]>> {
    let params = Params::new(params.m_cost, params.t_cost, params.p_cost, Some(KEY_LEN))
        .map_err(|_| AppError::Crypto)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = Zeroizing::new([0u8; KEY_LEN]);
    argon
        .hash_password_into(passphrase.as_bytes(), salt, out.as_mut())
        .map_err(|_| AppError::Crypto)?;
    Ok(out)
}

pub struct Encrypted {
    pub nonce: [u8; NONCE_LEN],
    pub ciphertext: Vec<u8>,
}

pub fn encrypt(key: &[u8; KEY_LEN], plaintext: &[u8], aad: &[u8]) -> AppResult<Encrypted> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let nonce = random_bytes::<NONCE_LEN>();
    let nonce_ref = XNonce::try_from(&nonce[..]).map_err(|_| AppError::Crypto)?;
    let ciphertext = cipher
        .encrypt(
            &nonce_ref,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| AppError::Crypto)?;
    Ok(Encrypted { nonce, ciphertext })
}

pub fn decrypt(
    key: &[u8; KEY_LEN],
    nonce: &[u8],
    ciphertext: &[u8],
    aad: &[u8],
) -> AppResult<Zeroizing<Vec<u8>>> {
    if nonce.len() != NONCE_LEN {
        return Err(AppError::Crypto);
    }
    let cipher = XChaCha20Poly1305::new(key.into());
    let nonce_ref = XNonce::try_from(nonce).map_err(|_| AppError::Crypto)?;
    let plaintext = cipher
        .decrypt(
            &nonce_ref,
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| AppError::Crypto)?;
    Ok(Zeroizing::new(plaintext))
}

pub fn derive_db_key(master_key: &[u8; KEY_LEN]) -> [u8; KEY_LEN] {
    blake3::derive_key("testtracker database key v1", master_key)
}

pub fn derive_evidence_key(master_key: &[u8; KEY_LEN], engagement_id: &str) -> [u8; KEY_LEN] {
    let mut hasher = blake3::Hasher::new_derive_key("testtracker evidence key v1");
    hasher.update(master_key);
    hasher.update(engagement_id.as_bytes());
    *hasher.finalize().as_bytes()
}

pub fn encrypt_secret(key: &[u8; KEY_LEN], aad: &[u8], plaintext: &[u8]) -> AppResult<String> {
    let encrypted = encrypt(key, plaintext, aad)?;
    let mut blob = Vec::with_capacity(NONCE_LEN + encrypted.ciphertext.len());
    blob.extend_from_slice(&encrypted.nonce);
    blob.extend_from_slice(&encrypted.ciphertext);
    Ok(BASE64.encode(blob))
}

pub fn decrypt_secret(
    key: &[u8; KEY_LEN],
    aad: &[u8],
    encoded: &str,
) -> AppResult<Zeroizing<Vec<u8>>> {
    let blob = BASE64.decode(encoded).map_err(|_| AppError::Crypto)?;
    if blob.len() < NONCE_LEN + 1 {
        return Err(AppError::Crypto);
    }
    let (nonce, ciphertext) = blob.split_at(NONCE_LEN);
    decrypt(key, nonce, ciphertext, aad)
}

pub fn derive_document_key(master_key: &[u8; KEY_LEN], engagement_id: &str) -> [u8; KEY_LEN] {
    let mut hasher = blake3::Hasher::new_derive_key("testtracker document key v1");
    hasher.update(master_key);
    hasher.update(engagement_id.as_bytes());
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encryption_roundtrip() {
        let key = random_bytes::<KEY_LEN>();
        let plaintext = b"engagement evidence";
        let aad = b"evidence-v1";
        let encrypted = encrypt(&key, plaintext, aad).expect("encrypt");
        let decrypted =
            decrypt(&key, &encrypted.nonce, &encrypted.ciphertext, aad).expect("decrypt");
        assert_eq!(decrypted.as_slice(), plaintext);
    }

    #[test]
    fn wrong_aad_fails() {
        let key = random_bytes::<KEY_LEN>();
        let encrypted = encrypt(&key, b"secret", b"aad-a").expect("encrypt");
        assert!(decrypt(&key, &encrypted.nonce, &encrypted.ciphertext, b"aad-b").is_err());
    }

    #[test]
    fn wrong_key_fails() {
        let key = random_bytes::<KEY_LEN>();
        let other = random_bytes::<KEY_LEN>();
        let encrypted = encrypt(&key, b"secret", b"aad").expect("encrypt");
        assert!(decrypt(&other, &encrypted.nonce, &encrypted.ciphertext, b"aad").is_err());
    }

    #[test]
    fn secret_roundtrip() {
        let key = random_bytes::<KEY_LEN>();
        let encoded =
            encrypt_secret(&key, b"ai-provider-v1:p1", b"sk-test-secret").expect("encrypt");
        let plaintext = decrypt_secret(&key, b"ai-provider-v1:p1", &encoded).expect("decrypt");
        assert_eq!(plaintext.as_slice(), b"sk-test-secret");
        assert!(decrypt_secret(&key, b"ai-provider-v1:p2", &encoded).is_err());
    }

    #[test]
    fn evidence_key_is_engagement_scoped() {
        let master = random_bytes::<KEY_LEN>();
        let a = derive_evidence_key(&master, "engagement-a");
        let b = derive_evidence_key(&master, "engagement-a");
        let c = derive_evidence_key(&master, "engagement-b");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn kdf_is_deterministic() {
        let salt = random_bytes::<SALT_LEN>();
        let params = KdfParams {
            m_cost: 4096,
            t_cost: 1,
            p_cost: 1,
        };
        let a = derive_key("correct horse battery staple", &salt, &params).expect("derive");
        let b = derive_key("correct horse battery staple", &salt, &params).expect("derive");
        assert_eq!(*a, *b);
    }
}
