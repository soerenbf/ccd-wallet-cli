#![allow(dead_code)]

use anyhow::{Context, Result, anyhow, bail};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    ChaCha20Poly1305, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use rand::{RngCore, rngs::OsRng};
use zeroize::Zeroizing;

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 12;
pub const SALT_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Argon2Params {
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl Default for Argon2Params {
    fn default() -> Self {
        Self {
            m_cost: 65_536,
            t_cost: 3,
            p_cost: 1,
        }
    }
}

impl Argon2Params {
    fn to_params(self) -> Result<Params> {
        Params::new(self.m_cost, self.t_cost, self.p_cost, Some(KEY_LEN))
            .map_err(|err| anyhow!("invalid Argon2 parameters: {err}"))
    }
}

pub fn random_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

pub fn generate_dek() -> Zeroizing<[u8; KEY_LEN]> {
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    OsRng.fill_bytes(&mut *key);
    key
}

pub fn derive_kek(
    password: &str,
    salt: &[u8],
    params: &Argon2Params,
) -> Result<Zeroizing<[u8; KEY_LEN]>> {
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params.to_params()?);
    let mut key = Zeroizing::new([0u8; KEY_LEN]);

    argon2
        .hash_password_into(password.as_bytes(), salt, &mut *key)
        .map_err(|err| anyhow!("failed to derive encryption key: {err}"))?;

    Ok(key)
}

pub fn aead_encrypt(
    key: &[u8; KEY_LEN],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<(Vec<u8>, [u8; NONCE_LEN])> {
    let cipher = ChaCha20Poly1305::new(key.into());
    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);

    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| anyhow!("failed to encrypt payload"))?;

    Ok((ciphertext, nonce))
}

pub fn aead_decrypt(
    key: &[u8; KEY_LEN],
    nonce: &[u8],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Zeroizing<Vec<u8>>> {
    if nonce.len() != NONCE_LEN {
        bail!(
            "invalid nonce length: expected {NONCE_LEN}, got {}",
            nonce.len()
        );
    }

    let cipher = ChaCha20Poly1305::new(key.into());
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| anyhow!("failed to decrypt payload"))?;

    Ok(Zeroizing::new(plaintext))
}

pub fn object_aad(id: &str, kind: &str, cipher_version: u32) -> Vec<u8> {
    format!("{id}:{kind}:v{cipher_version}").into_bytes()
}

pub fn zeroizing_array_from_slice<const N: usize>(
    slice: &[u8],
    label: &str,
) -> Result<Zeroizing<[u8; N]>> {
    let array: [u8; N] = slice
        .try_into()
        .with_context(|| format!("invalid {label} length: expected {N}, got {}", slice.len()))?;
    Ok(Zeroizing::new(array))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let salt = random_salt();
        let params = Argon2Params::default();
        let key = derive_kek("correct horse battery staple", &salt, &params).unwrap();
        let aad = object_aad("seed-id", "seed", 1);

        let (ciphertext, nonce) = aead_encrypt(&key, b"secret", &aad).unwrap();
        let plaintext = aead_decrypt(&key, &nonce, &ciphertext, &aad).unwrap();

        assert_eq!(&**plaintext, b"secret");

        let wrong_key = derive_kek("wrong password", &salt, &params).unwrap();
        assert!(aead_decrypt(&wrong_key, &nonce, &ciphertext, &aad).is_err());
    }

    #[test]
    fn aad_mismatch_fails() {
        let key = generate_dek();
        let (ciphertext, nonce) = aead_encrypt(&key, b"secret", b"seed:a:v1").unwrap();

        assert!(aead_decrypt(&key, &nonce, &ciphertext, b"seed:b:v1").is_err());
    }
}
