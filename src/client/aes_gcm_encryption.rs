use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::prelude::*;
use sha2::{Sha256, Digest};
use super::transport::EncryptionService;
use super::encryption::EncryptionError;

/// AES-GCM encryption service with proper key management
pub struct AesGcmEncryption {
    key: [u8; 32], // AES-256 requires exactly 32 bytes
    cipher: Aes256Gcm,
}

impl AesGcmEncryption {
    /// Create a new AES-GCM encryption service with a password-derived key
    pub fn new(password: &str) -> Self {
        let key = Self::derive_key(password);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        
        Self {
            key,
            cipher,
        }
    }
    
    /// Create a new AES-GCM encryption service with a raw 32-byte key
    pub fn from_key(key: [u8; 32]) -> Self {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        
        Self {
            key,
            cipher,
        }
    }
    
    /// Derive a 32-byte AES key from a password using SHA-256
    fn derive_key(password: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.finalize().into()
    }
    
    /// Get the raw key bytes (for key exchange purposes)
    pub fn get_key(&self) -> &[u8; 32] {
        &self.key
    }
    
    /// Set a new key (for key exchange purposes)
    pub fn set_key(&mut self, key: [u8; 32]) {
        self.key = key;
        self.cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    }
    
    /// Generate a random 32-byte key
    pub fn generate_random_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        use aes_gcm::aead::rand_core::RngCore;
        OsRng.fill_bytes(&mut key);
        key
    }
    
    /// Encrypt data directly with internal cipher
    fn encrypt_internal(&self, plaintext: &str) -> Result<String, EncryptionError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphered_data = self.cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| EncryptionError::EncryptionError(e.to_string()))?;
        
        // Combine nonce and encrypted data together for storage
        let mut encrypted_data: Vec<u8> = nonce.to_vec();
        encrypted_data.extend_from_slice(&ciphered_data);
        
        Ok(BASE64_STANDARD.encode(encrypted_data))
    }
    
    /// Decrypt data directly with internal cipher
    fn decrypt_internal(&self, encrypted_data: &str) -> Result<String, EncryptionError> {
        let encrypted_data = BASE64_STANDARD.decode(encrypted_data)
            .map_err(|e| EncryptionError::EncodingError(format!("Base64 decode error: {}", e)))?;
        
        if encrypted_data.len() < 12 {
            return Err(EncryptionError::DecryptionError("Encrypted data too short".to_string()));
        }
        
        let (nonce_arr, ciphered_data) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_arr);
        
        let plaintext = self.cipher
            .decrypt(nonce, ciphered_data)
            .map_err(|e| EncryptionError::DecryptionError(e.to_string()))?;
        
        String::from_utf8(plaintext)
            .map_err(|e| EncryptionError::EncodingError(format!("UTF-8 conversion error: {}", e)))
    }
}

impl EncryptionService for AesGcmEncryption {
    /// Encrypt plaintext with the stored key (ignores the key parameter for consistency with trait)
    fn encrypt(&self, _key: &str, plaintext: &str) -> Result<String, EncryptionError> {
        self.encrypt_internal(plaintext)
    }
    
    /// Decrypt ciphertext with the stored key (ignores the key parameter for consistency with trait)
    fn decrypt(&self, _key: &str, ciphertext: &str) -> Result<String, EncryptionError> {
        self.decrypt_internal(ciphertext)
    }
}

/// Create a boxed AES-GCM encryption service for use with ConnectionManager
pub fn create_aes_gcm_encryption(password: &str) -> Box<dyn EncryptionService + Send + Sync> {
    Box::new(AesGcmEncryption::new(password))
}

/// Create a boxed AES-GCM encryption service with a random key
pub fn create_aes_gcm_encryption_with_random_key() -> Box<dyn EncryptionService + Send + Sync> {
    let key = AesGcmEncryption::generate_random_key();
    Box::new(AesGcmEncryption::from_key(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes_gcm_creation() {
        let encryption = AesGcmEncryption::new("test_password");
        
        // Key should be derived and 32 bytes
        assert_eq!(encryption.get_key().len(), 32);
    }

    #[test]
    fn test_key_derivation_consistency() {
        let encryption1 = AesGcmEncryption::new("same_password");
        let encryption2 = AesGcmEncryption::new("same_password");
        
        // Same password should derive same key
        assert_eq!(encryption1.get_key(), encryption2.get_key());
    }

    #[test]
    fn test_key_derivation_difference() {
        let encryption1 = AesGcmEncryption::new("password1");
        let encryption2 = AesGcmEncryption::new("password2");
        
        // Different passwords should derive different keys
        assert_ne!(encryption1.get_key(), encryption2.get_key());
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let encryption = AesGcmEncryption::new("test_password");
        let message = "Hello, World!";
        
        let encrypted = encryption.encrypt("ignored", message).expect("Encryption should succeed");
        let decrypted = encryption.decrypt("ignored", &encrypted).expect("Decryption should succeed");
        
        assert_eq!(message, decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_different_keys() {
        let encryption1 = AesGcmEncryption::new("password1");
        let encryption2 = AesGcmEncryption::new("password2");
        let message = "Secret message";
        
        let encrypted = encryption1.encrypt("ignored", message).expect("Encryption should succeed");
        
        // Different key should fail to decrypt
        let result = encryption2.decrypt("ignored", &encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_random_key_generation() {
        let key1 = AesGcmEncryption::generate_random_key();
        let key2 = AesGcmEncryption::generate_random_key();
        
        // Random keys should be different
        assert_ne!(key1, key2);
        assert_eq!(key1.len(), 32);
        assert_eq!(key2.len(), 32);
    }

    #[test]
    fn test_key_setting() {
        let mut encryption = AesGcmEncryption::new("initial_password");
        let new_key = AesGcmEncryption::generate_random_key();
        
        encryption.set_key(new_key);
        assert_eq!(encryption.get_key(), &new_key);
        
        // Should be able to encrypt/decrypt with new key
        let message = "Test with new key";
        let encrypted = encryption.encrypt("ignored", message).unwrap();
        let decrypted = encryption.decrypt("ignored", &encrypted).unwrap();
        assert_eq!(message, decrypted);
    }

    #[test]
    fn test_invalid_encrypted_data() {
        let encryption = AesGcmEncryption::new("test_password");
        
        // Invalid base64
        let result = encryption.decrypt("ignored", "invalid_base64!@#");
        assert!(result.is_err());
        
        // Valid base64 but too short (less than 12 bytes for nonce)
        let short_data = BASE64_STANDARD.encode(b"short");
        let result = encryption.decrypt("ignored", &short_data);
        assert!(result.is_err());
        
        // Valid base64 but corrupted data
        let corrupted_data = BASE64_STANDARD.encode(b"this_is_12_bytes_but_not_encrypted_properly");
        let result = encryption.decrypt("ignored", &corrupted_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_encryption_service_trait() {
        let encryption: Box<dyn EncryptionService + Send + Sync> = create_aes_gcm_encryption("test_password");
        let message = "Testing trait implementation";
        
        let encrypted = encryption.encrypt("ignored_key", message).unwrap();
        let decrypted = encryption.decrypt("ignored_key", &encrypted).unwrap();
        
        assert_eq!(message, decrypted);
    }

    #[test]
    fn test_random_key_encryption_service() {
        let encryption = create_aes_gcm_encryption_with_random_key();
        let message = "Testing with random key";
        
        let encrypted = encryption.encrypt("ignored_key", message).unwrap();
        let decrypted = encryption.decrypt("ignored_key", &encrypted).unwrap();
        
        assert_eq!(message, decrypted);
    }
}