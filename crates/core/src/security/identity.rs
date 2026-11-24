use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::RngCore;
use zeroize::Zeroizing;

use crate::error::{AppError, AppResult};
use crate::security::crypto::{self, KEY_LEN};

pub fn generate() -> SigningKey {
    let mut secret = [0u8; 32];
    rand::rng().fill_bytes(&mut secret);
    SigningKey::from_bytes(&secret)
}

pub fn public_key_hex(key: &SigningKey) -> String {
    hex::encode(key.verifying_key().to_bytes())
}

#[allow(dead_code)]
pub fn private_key_bytes(key: &SigningKey) -> Zeroizing<[u8; 32]> {
    Zeroizing::new(key.to_bytes())
}

pub fn encrypt_signing_key(
    master_key: &[u8; KEY_LEN],
    profile_id: &str,
    signing_key: &SigningKey,
) -> AppResult<(String, String)> {
    let aad = profile_key_aad(profile_id);
    let encrypted = crypto::encrypt(master_key, &signing_key.to_bytes(), aad.as_bytes())?;
    let mut blob = Vec::with_capacity(24 + encrypted.ciphertext.len());
    blob.extend_from_slice(&encrypted.nonce);
    blob.extend_from_slice(&encrypted.ciphertext);
    Ok((public_key_hex(signing_key), BASE64.encode(blob)))
}

pub fn decrypt_signing_key(
    master_key: &[u8; KEY_LEN],
    profile_id: &str,
    expected_public_key_hex: &str,
    encrypted_base64: &str,
) -> AppResult<SigningKey> {
    let blob = BASE64
        .decode(encrypted_base64)
        .map_err(|_| AppError::Crypto)?;
    if blob.len() < 25 {
        return Err(AppError::Crypto);
    }
    let (nonce, ciphertext) = blob.split_at(24);
    let aad = profile_key_aad(profile_id);
    let plaintext = crypto::decrypt(master_key, nonce, ciphertext, aad.as_bytes())?;
    if plaintext.len() != 32 {
        return Err(AppError::Crypto);
    }
    let mut secret = [0u8; 32];
    secret.copy_from_slice(&plaintext);
    let key = SigningKey::from_bytes(&secret);
    if public_key_hex(&key) != expected_public_key_hex {
        return Err(AppError::Crypto);
    }
    Ok(key)
}

pub fn validate_public_key(public_key_hex: &str) -> bool {
    let Ok(bytes) = hex::decode(public_key_hex) else {
        return false;
    };
    let Ok(array) = <[u8; 32]>::try_from(bytes.as_slice()) else {
        return false;
    };
    VerifyingKey::from_bytes(&array).is_ok()
}

pub fn sign_hex(key: &SigningKey, message: &[u8]) -> String {
    hex::encode(key.sign(message).to_bytes())
}

pub fn verify_hex(public_key_hex: &str, message: &[u8], signature_hex: &str) -> bool {
    let Ok(public_bytes) = hex::decode(public_key_hex) else {
        return false;
    };
    let Ok(public_array) = <[u8; 32]>::try_from(public_bytes.as_slice()) else {
        return false;
    };
    let Ok(verifying_key) = VerifyingKey::from_bytes(&public_array) else {
        return false;
    };
    let Ok(signature_bytes) = hex::decode(signature_hex) else {
        return false;
    };
    let Ok(signature_array) = <[u8; 64]>::try_from(signature_bytes.as_slice()) else {
        return false;
    };
    let signature = ed25519_dalek::Signature::from_bytes(&signature_array);
    verifying_key.verify_strict(message, &signature).is_ok()
}

fn profile_key_aad(profile_id: &str) -> String {
    format!("testtracker-profile-key-v1:{profile_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_roundtrip() {
        let key = generate();
        let public = public_key_hex(&key);
        let signature = sign_hex(&key, b"approval");
        assert!(verify_hex(&public, b"approval", &signature));
        assert!(!verify_hex(&public, b"other", &signature));
    }

    #[test]
    fn encrypted_signing_key_roundtrip() {
        let master = crypto::random_bytes::<KEY_LEN>();
        let key = generate();
        let profile_id = "profile-1";
        let (public, encrypted) = encrypt_signing_key(&master, profile_id, &key).expect("encrypt");
        let decrypted =
            decrypt_signing_key(&master, profile_id, &public, &encrypted).expect("decrypt");
        assert_eq!(public_key_hex(&key), public_key_hex(&decrypted));
    }
}
