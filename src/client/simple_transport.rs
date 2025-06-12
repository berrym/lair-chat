//! Simple transport implementation that matches the server's expected protocol
//! This bypasses the complex ConnectionManager encryption layers that were causing conflicts

use async_trait::async_trait;
use base64::prelude::*;
use md5;
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use x25519_dalek::{EphemeralSecret, PublicKey};

use crate::auth::{AuthError, AuthResult, Credentials};
use crate::encryption::EncryptionError;
use crate::transport::{Transport, TransportError};

/// Simple transport that implements the exact protocol the server expects
pub struct SimpleTransport {
    stream: Option<TcpStream>,
    shared_key: Option<String>,
    address: SocketAddr,
}

impl SimpleTransport {
    pub fn new(address: SocketAddr) -> Self {
        Self {
            stream: None,
            shared_key: None,
            address,
        }
    }

    /// Connect and perform the exact handshake sequence the server expects
    pub async fn connect_and_handshake(&mut self) -> Result<(), TransportError> {
        // Step 1: Establish TCP connection
        let stream = TcpStream::connect(self.address)
            .await
            .map_err(TransportError::ConnectionError)?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Step 2: Receive server's public key (server sends first)
        let mut server_key_line = String::new();
        reader
            .read_line(&mut server_key_line)
            .await
            .map_err(TransportError::ConnectionError)?;
        let server_key_b64 = server_key_line.trim();

        // Step 3: Generate our keypair
        let client_secret = EphemeralSecret::random();
        let client_public = PublicKey::from(&client_secret);

        // Step 4: Send our public key to server
        let client_key_b64 = BASE64_STANDARD.encode(client_public);
        writer
            .write_all(format!("{}\n", client_key_b64).as_bytes())
            .await
            .map_err(TransportError::ConnectionError)?;
        writer
            .flush()
            .await
            .map_err(TransportError::ConnectionError)?;

        // Step 5: Derive shared secret
        let server_key_bytes = BASE64_STANDARD
            .decode(server_key_b64)
            .map_err(|e| TransportError::KeyExchangeError(format!("Invalid server key: {}", e)))?;

        if server_key_bytes.len() != 32 {
            return Err(TransportError::KeyExchangeError(
                "Server key must be 32 bytes".to_string(),
            ));
        }

        let mut server_key_array = [0u8; 32];
        server_key_array.copy_from_slice(&server_key_bytes);
        let server_public_key = PublicKey::from(server_key_array);

        let shared_secret = client_secret.diffie_hellman(&server_public_key);
        let shared_key = format!("{:x}", md5::compute(BASE64_STANDARD.encode(shared_secret)));

        // Step 6: Receive welcome message from server
        let mut welcome_line = String::new();
        reader
            .read_line(&mut welcome_line)
            .await
            .map_err(TransportError::ConnectionError)?;
        let _welcome_msg = self.decrypt(&shared_key, welcome_line.trim())?;

        // Step 7: Recreate full stream and store everything
        let reunited_stream = reader.into_inner().reunite(writer).map_err(|e| {
            TransportError::ConnectionError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to reunite stream: {}", e),
            ))
        })?;

        self.stream = Some(reunited_stream);
        self.shared_key = Some(shared_key);

        Ok(())
    }

    /// Register a new user
    pub async fn register(&mut self, credentials: Credentials) -> AuthResult<()> {
        if self.stream.is_none() || self.shared_key.is_none() {
            return Err(AuthError::ConnectionError("Not connected".to_string()));
        }

        let auth_request = serde_json::json!({
            "username": credentials.username,
            "password": credentials.password,
            "fingerprint": "simple_transport_fingerprint",
            "is_registration": true
        });

        self.send_encrypted(&auth_request.to_string())
            .await
            .map_err(|e| AuthError::ConnectionError(e.to_string()))?;

        // Wait for response
        let response = self
            .receive_encrypted()
            .await
            .map_err(|e| AuthError::ConnectionError(e.to_string()))?;

        if response.contains("Welcome back") || response.contains("has joined") {
            Ok(())
        } else if response.contains("Authentication failed") {
            Err(AuthError::AuthenticationFailed(response))
        } else {
            Err(AuthError::ProtocolError(format!(
                "Unexpected response: {}",
                response
            )))
        }
    }

    /// Login with existing credentials
    pub async fn login(&mut self, credentials: Credentials) -> AuthResult<()> {
        if self.stream.is_none() || self.shared_key.is_none() {
            return Err(AuthError::ConnectionError("Not connected".to_string()));
        }

        let auth_request = serde_json::json!({
            "username": credentials.username,
            "password": credentials.password,
            "fingerprint": "simple_transport_fingerprint",
            "is_registration": false
        });

        self.send_encrypted(&auth_request.to_string())
            .await
            .map_err(|e| AuthError::ConnectionError(e.to_string()))?;

        // Wait for response
        let response = self
            .receive_encrypted()
            .await
            .map_err(|e| AuthError::ConnectionError(e.to_string()))?;

        if response.contains("Welcome back") {
            Ok(())
        } else if response.contains("Authentication failed") {
            Err(AuthError::AuthenticationFailed(response))
        } else {
            Err(AuthError::ProtocolError(format!(
                "Unexpected response: {}",
                response
            )))
        }
    }

    /// Send an encrypted message
    async fn send_encrypted(&mut self, message: &str) -> Result<(), TransportError> {
        // Extract the key first to avoid borrowing conflicts
        let key = match &self.shared_key {
            Some(k) => k.clone(),
            None => {
                return Err(TransportError::ConnectionError(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    "Not connected - no key",
                )))
            }
        };

        // Encrypt the message before borrowing stream mutably
        let encrypted = self.encrypt(&key, message)?;

        // Now borrow stream mutably
        if let Some(stream) = &mut self.stream {
            stream
                .write_all(format!("{}\n", encrypted).as_bytes())
                .await
                .map_err(TransportError::ConnectionError)?;
            stream
                .flush()
                .await
                .map_err(TransportError::ConnectionError)?;
            Ok(())
        } else {
            Err(TransportError::ConnectionError(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected - no stream",
            )))
        }
    }

    /// Receive an encrypted message
    async fn receive_encrypted(&mut self) -> Result<String, TransportError> {
        if let Some(stream) = self.stream.take() {
            if let Some(key) = &self.shared_key {
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                reader
                    .read_line(&mut line)
                    .await
                    .map_err(TransportError::ConnectionError)?;

                if line.trim().is_empty() {
                    return Err(TransportError::ConnectionError(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Connection closed",
                    )));
                }

                let decrypted = self.decrypt(key, line.trim())?;

                // Restore stream
                self.stream = Some(reader.into_inner());

                Ok(decrypted)
            } else {
                // Restore stream
                self.stream = Some(stream);
                Err(TransportError::ConnectionError(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    "Not connected - no key",
                )))
            }
        } else {
            Err(TransportError::ConnectionError(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected - no stream",
            )))
        }
    }

    /// Encrypt a message using AES-256-GCM (matching server implementation)
    fn encrypt(&self, key: &str, plaintext: &str) -> Result<String, TransportError> {
        use aes_gcm::{
            aead::{Aead, AeadCore, KeyInit, OsRng},
            Aes256Gcm, Key, Nonce,
        };

        let key = Key::<Aes256Gcm>::from_slice(key.as_bytes());
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let cipher = Aes256Gcm::new(key);

        let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes()).map_err(|e| {
            TransportError::EncryptionError(EncryptionError::EncryptionError(e.to_string()))
        })?;

        // Combine nonce and ciphertext
        let mut encrypted_data: Vec<u8> = nonce.to_vec();
        encrypted_data.extend_from_slice(&ciphertext);

        Ok(BASE64_STANDARD.encode(encrypted_data))
    }

    /// Decrypt a message using AES-256-GCM (matching server implementation)
    fn decrypt(&self, key: &str, encrypted_data: &str) -> Result<String, TransportError> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Key, Nonce,
        };

        let encrypted_bytes = BASE64_STANDARD.decode(encrypted_data).map_err(|e| {
            TransportError::EncryptionError(EncryptionError::EncodingError(format!(
                "Base64 decode error: {}",
                e
            )))
        })?;

        if encrypted_bytes.len() < 12 {
            return Err(TransportError::EncryptionError(
                EncryptionError::DecryptionError("Encrypted data too short".to_string()),
            ));
        }

        let key = Key::<Aes256Gcm>::from_slice(key.as_bytes());
        let (nonce_bytes, ciphertext) = encrypted_bytes.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        let cipher = Aes256Gcm::new(key);

        let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| {
            TransportError::EncryptionError(EncryptionError::DecryptionError(e.to_string()))
        })?;

        String::from_utf8(plaintext).map_err(|e| {
            TransportError::EncryptionError(EncryptionError::EncodingError(format!(
                "UTF-8 decode error: {}",
                e
            )))
        })
    }
}

#[async_trait]
impl Transport for SimpleTransport {
    async fn connect(&mut self) -> Result<(), TransportError> {
        self.connect_and_handshake().await
    }

    async fn send(&mut self, data: &str) -> Result<(), TransportError> {
        self.send_encrypted(data).await
    }

    async fn receive(&mut self) -> Result<Option<String>, TransportError> {
        match self.receive_encrypted().await {
            Ok(data) => Ok(Some(data)),
            Err(TransportError::ConnectionError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        if let Some(mut stream) = self.stream.take() {
            stream
                .shutdown()
                .await
                .map_err(TransportError::ConnectionError)?;
        }
        self.shared_key = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encrypt_decrypt() {
        let transport = SimpleTransport::new("127.0.0.1:8080".parse().unwrap());
        let key = "test_key_32_bytes_long_exactly!!";
        let message = "Hello, World!";

        let encrypted = transport.encrypt(key, message).unwrap();
        let decrypted = transport.decrypt(key, &encrypted).unwrap();

        assert_eq!(message, decrypted);
    }
}
