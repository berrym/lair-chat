use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::prelude::*;

/// Encryption-specific error types
#[derive(Debug)]
pub enum EncryptionError {
    EncryptionError(String),
    DecryptionError(String),
    EncodingError(String),
}

impl std::fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionError::EncryptionError(msg) => write!(f, "Encryption error: {}", msg),
            EncryptionError::DecryptionError(msg) => write!(f, "Decryption error: {}", msg),
            EncryptionError::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
        }
    }
}

impl std::error::Error for EncryptionError {}

/// Encrypt strings with AES-256-GCM
pub fn encrypt(key_str: String, plaintext: String) -> Result<String, EncryptionError> {
    let key = Key::<Aes256Gcm>::from_slice(key_str.as_bytes());
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let cipher = Aes256Gcm::new(key);
    let ciphered_data = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| EncryptionError::EncryptionError(e.to_string()))?;
    // combining nonce and encrypted data together for storage purpose
    let mut encrypted_data: Vec<u8> = nonce.to_vec();
    encrypted_data.extend_from_slice(&ciphered_data);
    Ok(BASE64_STANDARD.encode(encrypted_data))
}

/// Decrypt strings that were encrypted with the encrypt function
pub fn decrypt(key_str: String, encrypted_data: String) -> Result<String, EncryptionError> {
    let encrypted_data = BASE64_STANDARD
        .decode(encrypted_data)
        .map_err(|e| EncryptionError::EncodingError(format!("Base64 decode error: {}", e)))?;
    let key = Key::<Aes256Gcm>::from_slice(key_str.as_bytes());
    let (nonce_arr, ciphered_data) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_arr);
    let cipher = Aes256Gcm::new(key);
    let plaintext = cipher
        .decrypt(nonce, ciphered_data)
        .map_err(|e| EncryptionError::DecryptionError(e.to_string()))?;
    String::from_utf8(plaintext)
        .map_err(|e| EncryptionError::EncodingError(format!("UTF-8 conversion error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = "test_key_32_bytes_exactly_here!!";
        let message = "Hello, World!";

        let encrypted =
            encrypt(key.to_string(), message.to_string()).expect("Encryption should succeed");
        let decrypted = decrypt(key.to_string(), encrypted).expect("Decryption should succeed");

        assert_eq!(message, decrypted);
    }

    #[test]
    fn test_decrypt_invalid_base64() {
        let key = "test_key_32_bytes_exactly_here!!";
        let invalid_base64 = "not_valid_base64!@#$";

        match decrypt(key.to_string(), invalid_base64.to_string()) {
            Err(EncryptionError::EncodingError(msg)) => {
                assert!(msg.contains("Base64 decode error"));
            }
            _ => panic!("Expected EncodingError for invalid base64"),
        }
    }

    #[test]
    fn test_decrypt_corrupted_data() {
        let key = "test_key_32_bytes_exactly_here!!";
        // Valid base64 but corrupted encrypted data (needs to be at least 12 bytes for nonce)
        let corrupted_data = "SGVsbG8gV29ybGQxMjM0NTY3ODkwMTI="; // Long enough base64, but not properly encrypted

        match decrypt(key.to_string(), corrupted_data.to_string()) {
            Err(EncryptionError::DecryptionError(_)) => {
                // This is expected - the data is valid base64 but not properly encrypted
            }
            _ => panic!("Expected DecryptionError for corrupted data"),
        }
    }

    #[test]
    fn test_error_display() {
        let enc_error = EncryptionError::EncryptionError("test encryption error".to_string());
        let dec_error = EncryptionError::DecryptionError("test decryption error".to_string());
        let encoding_error = EncryptionError::EncodingError("test encoding error".to_string());

        assert!(enc_error.to_string().contains("Encryption error"));
        assert!(dec_error.to_string().contains("Decryption error"));
        assert!(encoding_error.to_string().contains("Encoding error"));
    }
}
