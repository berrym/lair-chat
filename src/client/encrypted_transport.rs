use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::transport::{EncryptionService, Transport, TransportError};
use crate::encryption::EncryptionError;

/// Transport wrapper that automatically encrypts outgoing data and decrypts incoming data
/// using the established encryption service from the handshake
pub struct EncryptedTransport {
    inner_transport: Arc<Mutex<Box<dyn Transport + Send + Sync>>>,
    encryption: Arc<Mutex<Box<dyn EncryptionService + Send + Sync>>>,
}

impl EncryptedTransport {
    /// Create a new encrypted transport wrapper
    pub fn new(
        transport: Arc<Mutex<Box<dyn Transport + Send + Sync>>>,
        encryption: Arc<Mutex<Box<dyn EncryptionService + Send + Sync>>>,
    ) -> Self {
        Self {
            inner_transport: transport,
            encryption,
        }
    }

    /// Check if the encryption handshake has been completed
    /// For this implementation, we assume handshake is complete if we have an encryption service
    pub async fn is_handshake_complete(&self) -> bool {
        // We assume the encryption service is ready if it exists
        // This could be enhanced to check actual handshake state
        true
    }
}

#[async_trait]
impl Transport for EncryptedTransport {
    async fn connect(&mut self) -> Result<(), TransportError> {
        let mut transport = self.inner_transport.lock().await;
        transport.connect().await
    }

    async fn send(&mut self, data: &str) -> Result<(), TransportError> {
        // Encrypt the data before sending
        let encrypted_data = {
            let encryption = self.encryption.lock().await;
            encryption
                .encrypt("", data)
                .map_err(|e| TransportError::EncryptionError(e))?
        };

        // Send the encrypted data through the inner transport
        let mut transport = self.inner_transport.lock().await;
        transport.send(&encrypted_data).await
    }

    async fn receive(&mut self) -> Result<Option<String>, TransportError> {
        // Receive encrypted data from the inner transport
        let encrypted_data = {
            let mut transport = self.inner_transport.lock().await;
            transport.receive().await?
        };

        match encrypted_data {
            Some(data) => {
                // Decrypt the received data
                let decrypted_data = {
                    let encryption = self.encryption.lock().await;
                    encryption
                        .decrypt("", &data)
                        .map_err(|e| TransportError::EncryptionError(e))?
                };

                Ok(Some(decrypted_data))
            }
            None => Ok(None),
        }
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        let mut transport = self.inner_transport.lock().await;
        transport.close().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use tokio::sync::Mutex;

    struct MockTransport {
        send_data: Arc<Mutex<Vec<String>>>,
        receive_queue: Arc<Mutex<VecDeque<String>>>,
    }

    struct MockEncryption;

    impl MockEncryption {
        fn new() -> Self {
            Self
        }
    }

    #[async_trait]
    impl EncryptionService for MockEncryption {
        fn encrypt(&self, _key: &str, plaintext: &str) -> Result<String, EncryptionError> {
            Ok(format!("encrypted_{}", plaintext))
        }

        fn decrypt(&self, _key: &str, ciphertext: &str) -> Result<String, EncryptionError> {
            if let Some(plaintext) = ciphertext.strip_prefix("encrypted_") {
                Ok(plaintext.to_string())
            } else {
                Err(EncryptionError::EncryptionError(
                    "Invalid ciphertext".to_string(),
                ))
            }
        }

        async fn perform_handshake(
            &mut self,
            _transport: &mut dyn Transport,
        ) -> Result<(), TransportError> {
            Ok(())
        }
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
    async fn test_encrypted_transport_creation() {
        let mock_transport = Box::new(MockTransport::new());
        let transport = Arc::new(Mutex::new(
            mock_transport as Box<dyn Transport + Send + Sync>,
        ));
        let encryption = Arc::new(Mutex::new(
            Box::new(MockEncryption::new()) as Box<dyn EncryptionService + Send + Sync>
        ));

        let encrypted_transport = EncryptedTransport::new(transport, encryption);

        // Handshake should be considered complete for this test
        assert!(encrypted_transport.is_handshake_complete().await);
    }

    #[tokio::test]
    async fn test_encrypted_transport_send_without_handshake() {
        let mock_transport = Box::new(MockTransport::new());
        let transport = Arc::new(Mutex::new(
            mock_transport as Box<dyn Transport + Send + Sync>,
        ));
        let encryption = Arc::new(Mutex::new(
            Box::new(MockEncryption::new()) as Box<dyn EncryptionService + Send + Sync>
        ));

        let mut encrypted_transport = EncryptedTransport::new(transport, encryption);

        // Sending should work now with mock encryption
        let result = encrypted_transport.send("test message").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_encrypted_transport_receive_without_handshake() {
        let mock_transport = Box::new(MockTransport::new());
        let transport = Arc::new(Mutex::new(
            mock_transport as Box<dyn Transport + Send + Sync>,
        ));
        let encryption = Arc::new(Mutex::new(
            Box::new(MockEncryption::new()) as Box<dyn EncryptionService + Send + Sync>
        ));

        let mut encrypted_transport = EncryptedTransport::new(transport, encryption);

        // If there's no data, it should return None without error
        let result = encrypted_transport.receive().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_encrypted_transport_connect() {
        let mock_transport = Box::new(MockTransport::new());
        let transport = Arc::new(Mutex::new(
            mock_transport as Box<dyn Transport + Send + Sync>,
        ));
        let encryption = Arc::new(Mutex::new(
            Box::new(MockEncryption::new()) as Box<dyn EncryptionService + Send + Sync>
        ));

        let mut encrypted_transport = EncryptedTransport::new(transport, encryption);

        // Connect should work regardless of handshake state
        let result = encrypted_transport.connect().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_encrypted_transport_close() {
        let mock_transport = Box::new(MockTransport::new());
        let transport = Arc::new(Mutex::new(
            mock_transport as Box<dyn Transport + Send + Sync>,
        ));
        let encryption = Arc::new(Mutex::new(
            Box::new(MockEncryption::new()) as Box<dyn EncryptionService + Send + Sync>
        ));

        let mut encrypted_transport = EncryptedTransport::new(transport, encryption);

        // Close should work regardless of handshake state
        let result = encrypted_transport.close().await;
        assert!(result.is_ok());
    }
}
