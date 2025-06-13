use aes_gcm::aead::OsRng;
use async_trait::async_trait;
use base64::prelude::*;
use x25519_dalek::{EphemeralSecret, PublicKey};

use super::encryption::{decrypt, encrypt, EncryptionError};
use super::transport::{EncryptionService, Transport, TransportError};

/// **DEPRECATED**: This encryption service has a critical security vulnerability
///
/// # Security Issue
/// This implementation uses MD5 for key derivation, which is cryptographically broken
/// and vulnerable to collision attacks. Use `AesGcmEncryption` instead.
///
/// # Migration
/// Replace usage with:
/// ```rust
/// use crate::aes_gcm_encryption::create_aes_gcm_encryption_with_random_key;
/// let encryption = create_aes_gcm_encryption_with_random_key();
/// ```
///
/// # Legacy Protocol Support
/// This service implements the legacy server handshake sequence:
/// 1. Server sends its public key as base64
/// 2. Client responds with its public key as base64 (NOT prefixed with "HANDSHAKE:")
/// 3. Both derive shared secret using **insecure MD5** and use it for encryption
///
/// **WARNING**: Do not use in production environments!
#[deprecated(
    since = "0.6.1",
    note = "Uses insecure MD5 key derivation. Use AesGcmEncryption instead."
)]
pub struct ServerCompatibleEncryption {
    shared_key: Option<String>,
}

impl ServerCompatibleEncryption {
    /// Create a new server-compatible encryption service
    pub fn new() -> Self {
        Self { shared_key: None }
    }

    /// Get the shared key if handshake was completed
    pub fn get_shared_key(&self) -> Option<&str> {
        self.shared_key.as_deref()
    }

    /// Check if handshake has been completed
    pub fn is_handshake_complete(&self) -> bool {
        self.shared_key.is_some()
    }
}

#[async_trait]
impl EncryptionService for ServerCompatibleEncryption {
    fn encrypt(&self, _key: &str, plaintext: &str) -> Result<String, EncryptionError> {
        if let Some(shared_key) = &self.shared_key {
            encrypt(shared_key.clone(), plaintext.to_string())
        } else {
            Err(EncryptionError::EncryptionError(
                "Handshake not completed".to_string(),
            ))
        }
    }

    fn decrypt(&self, _key: &str, ciphertext: &str) -> Result<String, EncryptionError> {
        if let Some(shared_key) = &self.shared_key {
            decrypt(shared_key.clone(), ciphertext.to_string())
        } else {
            Err(EncryptionError::EncryptionError(
                "Handshake not completed".to_string(),
            ))
        }
    }

    /// Perform the exact handshake sequence that the server expects
    async fn perform_handshake(
        &mut self,
        transport: &mut dyn Transport,
    ) -> Result<(), TransportError> {
        // Step 1: Receive server's public key (server sends first)
        let server_key_response = transport.receive().await?;
        let server_key_b64 = match server_key_response {
            Some(key) => key,
            None => {
                return Err(TransportError::EncryptionError(
                    EncryptionError::EncryptionError("No server public key received".to_string()),
                ))
            }
        };

        // Step 2: Parse server's public key
        let server_key_bytes = BASE64_STANDARD.decode(&server_key_b64).map_err(|e| {
            TransportError::EncryptionError(EncryptionError::EncodingError(format!(
                "Invalid server public key: {}",
                e
            )))
        })?;

        if server_key_bytes.len() != 32 {
            return Err(TransportError::EncryptionError(
                EncryptionError::EncryptionError("Server public key must be 32 bytes".to_string()),
            ));
        }

        let mut server_key_array = [0u8; 32];
        server_key_array.copy_from_slice(&server_key_bytes);
        let server_public_key = PublicKey::from(server_key_array);

        // Step 3: Generate our ephemeral key pair
        let client_secret = EphemeralSecret::random_from_rng(OsRng);
        let client_public = PublicKey::from(&client_secret);

        // Step 4: Send our public key to server (as plain base64, no prefix)
        let client_key_b64 = BASE64_STANDARD.encode(client_public.as_bytes());
        transport.send(&client_key_b64).await?;

        // Step 5: Derive shared secret using the same method as the server
        let shared_secret = client_secret.diffie_hellman(&server_public_key);
        let shared_key = format!("{:x}", md5::compute(BASE64_STANDARD.encode(shared_secret)));

        // Step 6: Receive and decrypt welcome message to verify handshake
        let welcome_response = transport.receive().await?;
        if let Some(encrypted_welcome) = welcome_response {
            // Try to decrypt the welcome message to verify our shared key is correct
            match decrypt(shared_key.clone(), encrypted_welcome) {
                Ok(welcome_msg) => {
                    tracing::info!("Handshake successful, received welcome: {}", welcome_msg);
                }
                Err(e) => {
                    return Err(TransportError::EncryptionError(
                        EncryptionError::EncryptionError(format!(
                            "Failed to decrypt welcome message: {}",
                            e
                        )),
                    ));
                }
            }
        } else {
            return Err(TransportError::EncryptionError(
                EncryptionError::EncryptionError("No welcome message received".to_string()),
            ));
        }

        // Store the shared key
        self.shared_key = Some(shared_key);

        tracing::info!("Server-compatible handshake completed successfully");
        Ok(())
    }
}

/// **DEPRECATED**: Create a new server-compatible encryption service
///
/// # Security Warning
/// This function creates an encryption service that uses MD5 for key derivation,
/// which is cryptographically broken. Use `create_aes_gcm_encryption_with_random_key()` instead.
///
/// # Migration
/// ```rust
/// // OLD (insecure):
/// let encryption = create_server_compatible_encryption();
///
/// // NEW (secure):
/// use crate::aes_gcm_encryption::create_aes_gcm_encryption_with_random_key;
/// let encryption = create_aes_gcm_encryption_with_random_key();
/// ```
#[deprecated(
    since = "0.6.1",
    note = "Uses insecure MD5 key derivation. Use create_aes_gcm_encryption_with_random_key() instead."
)]
pub fn create_server_compatible_encryption() -> Box<dyn EncryptionService + Send + Sync> {
    Box::new(ServerCompatibleEncryption::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct MockTransport {
        send_data: Arc<Mutex<Vec<String>>>,
        receive_queue: Arc<Mutex<VecDeque<String>>>,
    }

    impl MockTransport {
        fn new() -> Self {
            Self {
                send_data: Arc::new(Mutex::new(Vec::new())),
                receive_queue: Arc::new(Mutex::new(VecDeque::new())),
            }
        }

        async fn add_receive_data(&mut self, data: String) {
            self.receive_queue.lock().await.push_back(data);
        }

        async fn get_sent_data(&self) -> Vec<String> {
            self.send_data.lock().await.clone()
        }
    }

    #[async_trait]
    impl Transport for MockTransport {
        async fn connect(&mut self) -> Result<(), TransportError> {
            Ok(())
        }

        async fn send(&mut self, data: &str) -> Result<(), TransportError> {
            self.send_data.lock().await.push(data.to_string());
            Ok(())
        }

        async fn receive(&mut self) -> Result<Option<String>, TransportError> {
            let mut queue = self.receive_queue.lock().await;
            Ok(queue.pop_front())
        }

        async fn close(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_server_compatible_encryption_creation() {
        let encryption = ServerCompatibleEncryption::new();
        assert!(!encryption.is_handshake_complete());
        assert!(encryption.get_shared_key().is_none());
    }

    #[tokio::test]
    async fn test_encryption_before_handshake() {
        let encryption = ServerCompatibleEncryption::new();

        let result = encryption.encrypt("ignored", "test message");
        assert!(result.is_err());

        let result = encryption.decrypt("ignored", "encrypted_data");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_successful_handshake() {
        let mut encryption = ServerCompatibleEncryption::new();
        let mut transport = MockTransport::new();

        // Generate a server key pair for the test
        let server_secret = EphemeralSecret::random_from_rng(OsRng);
        let server_public = PublicKey::from(&server_secret);
        let server_key_b64 = BASE64_STANDARD.encode(server_public.as_bytes());

        // Add server public key to transport
        transport.add_receive_data(server_key_b64.clone()).await;

        // Generate what the welcome message should be
        // We need to simulate what the server would send
        let client_secret = EphemeralSecret::random_from_rng(OsRng);
        let _client_public = PublicKey::from(&client_secret);
        let shared_secret = client_secret.diffie_hellman(&server_public);
        let shared_key = format!("{:x}", md5::compute(BASE64_STANDARD.encode(shared_secret)));

        let welcome_msg = "Welcome to The Lair! Please login or register.";
        let encrypted_welcome = encrypt(shared_key.clone(), welcome_msg.to_string()).unwrap();
        transport.add_receive_data(encrypted_welcome).await;

        // Perform handshake
        let result = encryption.perform_handshake(&mut transport).await;
        assert!(result.is_ok());
        assert!(encryption.is_handshake_complete());

        // Verify client sent its public key
        let sent_data = transport.get_sent_data().await;
        assert_eq!(sent_data.len(), 1);

        // The sent data should be a valid base64 public key
        let sent_key = &sent_data[0];
        let decoded = BASE64_STANDARD.decode(sent_key);
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap().len(), 32);
    }

    #[tokio::test]
    async fn test_handshake_invalid_server_key() {
        let mut encryption = ServerCompatibleEncryption::new();
        let mut transport = MockTransport::new();

        // Add invalid server key
        transport
            .add_receive_data("invalid_base64_key!@#".to_string())
            .await;

        let result = encryption.perform_handshake(&mut transport).await;
        assert!(result.is_err());
        assert!(!encryption.is_handshake_complete());
    }

    #[tokio::test]
    async fn test_handshake_no_server_key() {
        let mut encryption = ServerCompatibleEncryption::new();
        let mut transport = MockTransport::new();

        // Don't add any data to transport

        let result = encryption.perform_handshake(&mut transport).await;
        assert!(result.is_err());
        assert!(!encryption.is_handshake_complete());
    }

    #[tokio::test]
    async fn test_handshake_wrong_key_length() {
        let mut encryption = ServerCompatibleEncryption::new();
        let mut transport = MockTransport::new();

        // Add server key with wrong length (16 bytes instead of 32)
        let short_key = vec![0u8; 16];
        let short_key_b64 = BASE64_STANDARD.encode(short_key);
        transport.add_receive_data(short_key_b64).await;

        let result = encryption.perform_handshake(&mut transport).await;
        assert!(result.is_err());
        assert!(!encryption.is_handshake_complete());
    }

    #[tokio::test]
    async fn test_encryption_after_handshake() {
        let mut encryption = ServerCompatibleEncryption::new();

        // Manually set a shared key to simulate completed handshake
        encryption.shared_key = Some("test_key_32_bytes_exactly_here!!".to_string());

        let message = "Hello, World!";
        let encrypted = encryption.encrypt("ignored", message).unwrap();
        let decrypted = encryption.decrypt("ignored", &encrypted).unwrap();

        assert_eq!(message, decrypted);
    }

    #[test]
    fn test_factory_function() {
        let encryption = create_server_compatible_encryption();

        // Should be able to create without panicking
        // The actual encryption service is boxed, so we can't directly test its type,
        // but we can verify it implements the trait
        let result = encryption.encrypt("key", "test");
        assert!(result.is_err()); // Should fail because handshake not completed
    }
}
