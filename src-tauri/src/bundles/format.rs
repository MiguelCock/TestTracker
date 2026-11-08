use crate::error::{AppError, AppResult};
use crate::security::crypto::{self, KdfParams, NONCE_LEN, SALT_LEN};

const MAGIC: &[u8; 8] = b"TTBUNDLE";
const VERSION: u8 = 1;
const AAD: &[u8] = b"testtracker-bundle-v1";
const HEADER_LEN: usize = 8 + 1 + 12 + SALT_LEN + NONCE_LEN;
pub const MAX_BUNDLE_BYTES: usize = 40 * 1024 * 1024;
pub const MAX_PLAINTEXT_BYTES: usize = 25 * 1024 * 1024;

pub fn encrypt_bundle(passphrase: &str, plaintext: &[u8]) -> AppResult<Vec<u8>> {
    encrypt_bundle_with_params(passphrase, plaintext, KdfParams::default())
}

pub fn encrypt_bundle_with_params(
    passphrase: &str,
    plaintext: &[u8],
    params: KdfParams,
) -> AppResult<Vec<u8>> {
    if plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err(AppError::Validation(
            "Bundle payload is larger than 25 MB.".into(),
        ));
    }
    let salt = crypto::random_bytes::<SALT_LEN>();
    let key = crypto::derive_key(passphrase, &salt, &params)?;
    let encrypted = crypto::encrypt(&key, plaintext, AAD)?;
    let mut out = Vec::with_capacity(HEADER_LEN + encrypted.ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&params.m_cost.to_le_bytes());
    out.extend_from_slice(&params.t_cost.to_le_bytes());
    out.extend_from_slice(&params.p_cost.to_le_bytes());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&encrypted.nonce);
    out.extend_from_slice(&encrypted.ciphertext);
    if out.len() > MAX_BUNDLE_BYTES {
        return Err(AppError::Validation(
            "Encrypted bundle is larger than 40 MB.".into(),
        ));
    }
    Ok(out)
}

pub fn decrypt_bundle(bytes: &[u8], passphrase: &str) -> AppResult<Vec<u8>> {
    if bytes.len() > MAX_BUNDLE_BYTES || bytes.len() <= HEADER_LEN + 16 {
        return Err(AppError::Validation("Invalid bundle file.".into()));
    }
    if &bytes[..8] != MAGIC || bytes[8] != VERSION {
        return Err(AppError::Validation("Unsupported bundle format.".into()));
    }
    let m_cost = u32::from_le_bytes([bytes[9], bytes[10], bytes[11], bytes[12]]);
    let t_cost = u32::from_le_bytes([bytes[13], bytes[14], bytes[15], bytes[16]]);
    let p_cost = u32::from_le_bytes([bytes[17], bytes[18], bytes[19], bytes[20]]);
    if !(8_192..=262_144).contains(&m_cost)
        || !(1..=10).contains(&t_cost)
        || !(1..=8).contains(&p_cost)
    {
        return Err(AppError::Validation(
            "Invalid bundle KDF parameters.".into(),
        ));
    }
    let salt_start = 21;
    let salt_end = salt_start + SALT_LEN;
    let nonce_end = salt_end + NONCE_LEN;
    let salt = &bytes[salt_start..salt_end];
    let nonce = &bytes[salt_end..nonce_end];
    let ciphertext = &bytes[nonce_end..];
    let params = KdfParams {
        m_cost,
        t_cost,
        p_cost,
    };
    let key = crypto::derive_key(passphrase, salt, &params)?;
    let plaintext = crypto::decrypt(&key, nonce, ciphertext, AAD)
        .map_err(|_| AppError::Validation("Invalid bundle or passphrase.".into()))?;
    if plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err(AppError::Validation(
            "Bundle payload is larger than 25 MB.".into(),
        ));
    }
    Ok(plaintext.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_params() -> KdfParams {
        KdfParams {
            m_cost: 8_192,
            t_cost: 1,
            p_cost: 1,
        }
    }

    #[test]
    fn encrypts_and_decrypts() {
        let bytes = encrypt_bundle_with_params(
            "correct horse battery staple",
            b"bundle data",
            test_params(),
        )
        .expect("encrypt");
        let plain = decrypt_bundle(&bytes, "correct horse battery staple").expect("decrypt");
        assert_eq!(plain, b"bundle data");
    }

    #[test]
    fn wrong_passphrase_fails() {
        let bytes = encrypt_bundle_with_params(
            "correct horse battery staple",
            b"bundle data",
            test_params(),
        )
        .expect("encrypt");
        assert!(decrypt_bundle(&bytes, "wrong passphrase").is_err());
    }

    #[test]
    fn tampering_fails() {
        let mut bytes = encrypt_bundle_with_params(
            "correct horse battery staple",
            b"bundle data",
            test_params(),
        )
        .expect("encrypt");
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        assert!(decrypt_bundle(&bytes, "correct horse battery staple").is_err());
    }
}
