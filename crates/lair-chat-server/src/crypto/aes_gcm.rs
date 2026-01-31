//! AES-256-GCM encryption
//!
//! Provides authenticated encryption for protocol messages.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

/// Size of the nonce in bytes (96 bits as per GCM spec).
pub const NONCE_SIZE: usize = 12;

/// Size of the authentication tag in bytes (128 bits).
pub const TAG_SIZE: usize = 16;

/// Errors that can occur during encryption/decryption.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Encryption failed")]
    EncryptionFailed,

    #[error("Decryption failed: authentication tag mismatch")]
    DecryptionFailed,

    #[error("Invalid key length: expected 32 bytes, got {0}")]
    InvalidKeyLength(usize),

    #[error("Invalid nonce length: expected {NONCE_SIZE} bytes, got {0}")]
    InvalidNonceLength(usize),

    #[error("Ciphertext too short: minimum {0} bytes required")]
    CiphertextTooShort(usize),
}

/// AES-256-GCM cipher for message encryption.
pub struct Cipher {
    inner: Aes256Gcm,
}

impl Cipher {
    /// Create a new cipher from a 32-byte key.
    pub fn new(key: &[u8; 32]) -> Self {
        Self {
            inner: Aes256Gcm::new_from_slice(key).expect("key length is valid"),
        }
    }

    /// Encrypt a plaintext message.
    ///
    /// Returns `(nonce, ciphertext)` where ciphertext includes the authentication tag.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<([u8; NONCE_SIZE], Vec<u8>), CryptoError> {
        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt (includes authentication tag)
        let ciphertext = self
            .inner
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::EncryptionFailed)?;

        Ok((nonce_bytes, ciphertext))
    }

    /// Decrypt a ciphertext message.
    ///
    /// The ciphertext must include the authentication tag.
    pub fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if nonce.len() != NONCE_SIZE {
            return Err(CryptoError::InvalidNonceLength(nonce.len()));
        }

        // Minimum ciphertext size is just the tag
        if ciphertext.len() < TAG_SIZE {
            return Err(CryptoError::CiphertextTooShort(TAG_SIZE));
        }

        let nonce = Nonce::from_slice(nonce);

        self.inner
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}

impl std::fmt::Debug for Cipher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cipher").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = test_key();
        let cipher = Cipher::new(&key);

        let plaintext = b"Hello, World!";
        let (nonce, ciphertext) = cipher.encrypt(plaintext).unwrap();

        let decrypted = cipher.decrypt(&nonce, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertext() {
        let key = test_key();
        let cipher = Cipher::new(&key);

        let plaintext = b"Hello, World!";
        let (nonce1, ciphertext1) = cipher.encrypt(plaintext).unwrap();
        let (nonce2, ciphertext2) = cipher.encrypt(plaintext).unwrap();

        // Different nonces
        assert_ne!(nonce1, nonce2);
        // Different ciphertexts
        assert_ne!(ciphertext1, ciphertext2);
    }

    #[test]
    fn test_ciphertext_includes_tag() {
        let key = test_key();
        let cipher = Cipher::new(&key);

        let plaintext = b"Hello";
        let (_, ciphertext) = cipher.encrypt(plaintext).unwrap();

        // Ciphertext should be plaintext length + tag size
        assert_eq!(ciphertext.len(), plaintext.len() + TAG_SIZE);
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = test_key();
        let cipher = Cipher::new(&key);

        let plaintext = b"Hello, World!";
        let (nonce, mut ciphertext) = cipher.encrypt(plaintext).unwrap();

        // Tamper with ciphertext
        ciphertext[0] ^= 0xFF;

        let result = cipher.decrypt(&nonce, &ciphertext);
        assert!(matches!(result, Err(CryptoError::DecryptionFailed)));
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = test_key();
        let key2 = test_key();
        let cipher1 = Cipher::new(&key1);
        let cipher2 = Cipher::new(&key2);

        let plaintext = b"Hello, World!";
        let (nonce, ciphertext) = cipher1.encrypt(plaintext).unwrap();

        let result = cipher2.decrypt(&nonce, &ciphertext);
        assert!(matches!(result, Err(CryptoError::DecryptionFailed)));
    }

    #[test]
    fn test_invalid_nonce_length() {
        let key = test_key();
        let cipher = Cipher::new(&key);

        let result = cipher.decrypt(&[0u8; 8], &[0u8; 32]);
        assert!(matches!(result, Err(CryptoError::InvalidNonceLength(8))));
    }

    #[test]
    fn test_ciphertext_too_short() {
        let key = test_key();
        let cipher = Cipher::new(&key);

        let result = cipher.decrypt(&[0u8; NONCE_SIZE], &[0u8; 8]);
        assert!(matches!(result, Err(CryptoError::CiphertextTooShort(_))));
    }

    #[test]
    fn test_empty_plaintext() {
        let key = test_key();
        let cipher = Cipher::new(&key);

        let plaintext = b"";
        let (nonce, ciphertext) = cipher.encrypt(plaintext).unwrap();

        let decrypted = cipher.decrypt(&nonce, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_large_plaintext() {
        let key = test_key();
        let cipher = Cipher::new(&key);

        let plaintext = vec![0xAB; 1_000_000]; // 1 MB
        let (nonce, ciphertext) = cipher.encrypt(&plaintext).unwrap();

        let decrypted = cipher.decrypt(&nonce, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_cipher_debug() {
        let key = test_key();
        let cipher = Cipher::new(&key);

        // Debug should not expose key material
        let debug = format!("{:?}", cipher);
        assert!(debug.contains("Cipher"));
        assert!(!debug.contains(&format!("{:02x}", key[0])));
    }

    #[test]
    fn test_crypto_error_display() {
        assert_eq!(
            CryptoError::EncryptionFailed.to_string(),
            "Encryption failed"
        );
        assert_eq!(
            CryptoError::DecryptionFailed.to_string(),
            "Decryption failed: authentication tag mismatch"
        );
        assert_eq!(
            CryptoError::InvalidKeyLength(16).to_string(),
            "Invalid key length: expected 32 bytes, got 16"
        );
        assert_eq!(
            CryptoError::InvalidNonceLength(8).to_string(),
            format!("Invalid nonce length: expected {} bytes, got 8", NONCE_SIZE)
        );
        assert_eq!(
            CryptoError::CiphertextTooShort(16).to_string(),
            "Ciphertext too short: minimum 16 bytes required"
        );
    }

    #[test]
    fn test_wrong_nonce_fails() {
        let key = test_key();
        let cipher = Cipher::new(&key);

        let plaintext = b"Hello, World!";
        let (mut nonce, ciphertext) = cipher.encrypt(plaintext).unwrap();

        // Change nonce
        nonce[0] ^= 0xFF;

        let result = cipher.decrypt(&nonce, &ciphertext);
        assert!(matches!(result, Err(CryptoError::DecryptionFailed)));
    }

    #[test]
    fn test_nonce_is_correct_size() {
        assert_eq!(NONCE_SIZE, 12);
    }

    #[test]
    fn test_tag_is_correct_size() {
        assert_eq!(TAG_SIZE, 16);
    }
}
