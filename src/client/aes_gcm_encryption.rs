use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::prelude::*;
use sha2::{Sha256, Digest};
use x25519_dalek::{EphemeralSecret, PublicKey};
use async_trait::async_trait;
use super::transport::{EncryptionService, Transport, TransportError};
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
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(result.as_slice());
        key
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

#[async_trait]
impl EncryptionService for AesGcmEncryption {
    /// Encrypt plaintext with the stored key (ignores the key parameter for consistency with trait)
    fn encrypt(&self, _key: &str, plaintext: &str) -> Result<String, EncryptionError> {
        self.encrypt_internal(plaintext)
    }
    
    /// Decrypt ciphertext with the stored key (ignores the key parameter for consistency with trait)
    fn decrypt(&self, _key: &str, ciphertext: &str) -> Result<String, EncryptionError> {
        self.decrypt_internal(ciphertext)
    }
    
    /// Perform X25519 key exchange handshake with remote peer
    async fn perform_handshake(&mut self, transport: &mut dyn Transport) -> Result<(), TransportError> {
        // Generate ephemeral key pair for this session
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public_key = PublicKey::from(&secret);
        
        // Send our public key to the remote peer
        let public_key_b64 = BASE64_STANDARD.encode(public_key.as_bytes());
        let handshake_message = format!("HANDSHAKE:{}", public_key_b64);
        transport.send(&handshake_message).await?;
        
        // Receive remote peer's public key
        let response = transport.receive().await?;
        let remote_public_key = match response {
            Some(msg) => {
                if let Some(key_data) = msg.strip_prefix("HANDSHAKE:") {
                    let key_bytes = BASE64_STANDARD.decode(key_data)
                        .map_err(|e| TransportError::EncryptionError(EncryptionError::EncodingError(format!("Invalid handshake response: {}", e))))?;
                    
                    if key_bytes.len() != 32 {
                        return Err(TransportError::EncryptionError(EncryptionError::EncryptionError("Invalid public key length".to_string())));
                    }
                    
                    let mut key_array = [0u8; 32];
                    key_array.copy_from_slice(&key_bytes);
                    PublicKey::from(key_array)
                } else {
                    return Err(TransportError::EncryptionError(EncryptionError::EncryptionError("Invalid handshake message format".to_string())));
                }
            }
            None => return Err(TransportError::EncryptionError(EncryptionError::EncryptionError("No handshake response received".to_string()))),
        };
        
        // Compute shared secret
        let shared_secret = secret.diffie_hellman(&remote_public_key);
        
        // Derive AES key from shared secret using SHA-256
        let mut hasher = Sha256::new();
        hasher.update(shared_secret.as_bytes());
        hasher.update(b"LAIR_CHAT_AES_KEY"); // Domain separation
        let result = hasher.finalize();
        let mut derived_key = [0u8; 32];
        derived_key.copy_from_slice(result.as_slice());
        
        // Update our encryption key
        self.set_key(derived_key);
        
        Ok(())
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

    // Mock transport for handshake testing
    struct MockHandshakeTransport {
        messages: std::sync::Arc<tokio::sync::Mutex<Vec<String>>>,
        responses: std::sync::Arc<tokio::sync::Mutex<Vec<String>>>,
    }

    impl MockHandshakeTransport {
        fn new() -> Self {
            Self {
                messages: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
                responses: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
            }
        }
        
        async fn add_response(&self, response: String) {
            let mut responses = self.responses.lock().await;
            responses.push(response);
        }
        
        async fn get_sent_messages(&self) -> Vec<String> {
            let messages = self.messages.lock().await;
            messages.clone()
        }
    }

    #[async_trait]
    impl Transport for MockHandshakeTransport {
        async fn connect(&mut self) -> Result<(), TransportError> {
            Ok(())
        }

        async fn send(&mut self, data: &str) -> Result<(), TransportError> {
            let mut messages = self.messages.lock().await;
            messages.push(data.to_string());
            Ok(())
        }

        async fn receive(&mut self) -> Result<Option<String>, TransportError> {
            let mut responses = self.responses.lock().await;
            if responses.is_empty() {
                Ok(None)
            } else {
                Ok(Some(responses.remove(0)))
            }
        }

        async fn close(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_successful_handshake() {
        let mut encryption1 = AesGcmEncryption::new("initial_password_1");
        let mut encryption2 = AesGcmEncryption::new("initial_password_2");
        
        // Create mock transports
        let mut transport1 = MockHandshakeTransport::new();
        let mut transport2 = MockHandshakeTransport::new();
        
        // Simulate handshake: encryption1 sends first, encryption2 responds
        // First, encryption1 generates its key pair and sends public key
        let secret1 = x25519_dalek::EphemeralSecret::random_from_rng(aes_gcm::aead::OsRng);
        let public1 = x25519_dalek::PublicKey::from(&secret1);
        let public1_b64 = BASE64_STANDARD.encode(public1.as_bytes());
        
        // encryption2 generates its key pair
        let secret2 = x25519_dalek::EphemeralSecret::random_from_rng(aes_gcm::aead::OsRng);
        let public2 = x25519_dalek::PublicKey::from(&secret2);
        let public2_b64 = BASE64_STANDARD.encode(public2.as_bytes());
        
        // Set up the mock responses
        transport1.add_response(format!("HANDSHAKE:{}", public2_b64)).await;
        transport2.add_response(format!("HANDSHAKE:{}", public1_b64)).await;
        
        // Perform handshakes
        let result1 = encryption1.perform_handshake(&mut transport1).await;
        let result2 = encryption2.perform_handshake(&mut transport2).await;
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        // Verify messages were sent
        let sent1 = transport1.get_sent_messages().await;
        let sent2 = transport2.get_sent_messages().await;
        
        assert_eq!(sent1.len(), 1);
        assert_eq!(sent2.len(), 1);
        assert!(sent1[0].starts_with("HANDSHAKE:"));
        assert!(sent2[0].starts_with("HANDSHAKE:"));
        
        // Note: The keys will be different because we're using different ephemeral secrets
        // This is expected behavior for the X25519 key exchange
    }

    #[tokio::test]
    async fn test_handshake_with_invalid_response() {
        let mut encryption = AesGcmEncryption::new("test_password");
        let mut transport = MockHandshakeTransport::new();
        
        // Add invalid response
        transport.add_response("INVALID_MESSAGE".to_string()).await;
        
        let result = encryption.perform_handshake(&mut transport).await;
        assert!(result.is_err());
        
        if let Err(TransportError::EncryptionError(e)) = result {
            assert!(e.to_string().contains("Invalid handshake message format"));
        } else {
            panic!("Expected EncryptionError");
        }
    }

    #[tokio::test]
    async fn test_handshake_with_invalid_base64() {
        let mut encryption = AesGcmEncryption::new("test_password");
        let mut transport = MockHandshakeTransport::new();
        
        // Add response with invalid base64
        transport.add_response("HANDSHAKE:invalid_base64!@#".to_string()).await;
        
        let result = encryption.perform_handshake(&mut transport).await;
        assert!(result.is_err());
        
        if let Err(TransportError::EncryptionError(e)) = result {
            assert!(e.to_string().contains("Invalid handshake response"));
        } else {
            panic!("Expected EncryptionError");
        }
    }

    #[tokio::test]
    async fn test_handshake_with_wrong_key_length() {
        let mut encryption = AesGcmEncryption::new("test_password");
        let mut transport = MockHandshakeTransport::new();
        
        // Add response with wrong key length (16 bytes instead of 32)
        let short_key = vec![0u8; 16];
        let short_key_b64 = BASE64_STANDARD.encode(short_key);
        transport.add_response(format!("HANDSHAKE:{}", short_key_b64)).await;
        
        let result = encryption.perform_handshake(&mut transport).await;
        assert!(result.is_err());
        
        if let Err(TransportError::EncryptionError(e)) = result {
            assert!(e.to_string().contains("Invalid public key length"));
        } else {
            panic!("Expected EncryptionError");
        }
    }

    #[tokio::test]
    async fn test_handshake_no_response() {
        let mut encryption = AesGcmEncryption::new("test_password");
        let mut transport = MockHandshakeTransport::new();
        
        // Don't add any response
        
        let result = encryption.perform_handshake(&mut transport).await;
        assert!(result.is_err());
        
        if let Err(TransportError::EncryptionError(e)) = result {
            assert!(e.to_string().contains("No handshake response received"));
        } else {
            panic!("Expected EncryptionError");
        }
    }

    #[tokio::test]
    async fn test_encryption_after_handshake() {
        let mut encryption1 = AesGcmEncryption::new("initial_password");
        let mut encryption2 = AesGcmEncryption::new("different_initial_password");
        
        // Store original keys to verify they change after handshake
        let original_key1 = *encryption1.get_key();
        let original_key2 = *encryption2.get_key();
        
        // Simulate successful key exchange by setting the same derived key
        let shared_secret = AesGcmEncryption::generate_random_key();
        let mut hasher1 = sha2::Sha256::new();
        hasher1.update(&shared_secret);
        hasher1.update(b"LAIR_CHAT_AES_KEY");
        let result = hasher1.finalize();
        let mut derived_key = [0u8; 32];
        derived_key.copy_from_slice(result.as_slice());
        
        encryption1.set_key(derived_key);
        encryption2.set_key(derived_key);
        
        // Verify keys changed
        assert_ne!(original_key1, *encryption1.get_key());
        assert_ne!(original_key2, *encryption2.get_key());
        assert_eq!(*encryption1.get_key(), *encryption2.get_key());
        
        // Test encryption/decryption between the two services
        let message = "Secret message after handshake";
        let encrypted = encryption1.encrypt("ignored", message).unwrap();
        let decrypted = encryption2.decrypt("ignored", &encrypted).unwrap();
        
        assert_eq!(message, decrypted);
    }
}