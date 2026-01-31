//! X25519 key exchange
//!
//! Implements ephemeral Diffie-Hellman key exchange using X25519.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};

/// Errors that can occur during key exchange.
#[derive(Debug, thiserror::Error)]
pub enum KeyExchangeError {
    #[error("Invalid public key length: expected 32 bytes, got {0}")]
    InvalidKeyLength(usize),

    #[error("Base64 decode error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),
}

/// An X25519 keypair for ephemeral key exchange.
pub struct KeyPair {
    secret: EphemeralSecret,
    public: PublicKey,
}

impl KeyPair {
    /// Generate a new random keypair.
    pub fn generate() -> Self {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Get the public key as base64-encoded string.
    pub fn public_key_base64(&self) -> String {
        BASE64.encode(self.public.as_bytes())
    }

    /// Get the raw public key bytes.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        *self.public.as_bytes()
    }

    /// Perform Diffie-Hellman key exchange with the peer's public key.
    /// Returns a 32-byte shared secret suitable for use as an AES-256 key.
    pub fn diffie_hellman(self, peer_public: PublicKey) -> [u8; 32] {
        let shared: SharedSecret = self.secret.diffie_hellman(&peer_public);
        *shared.as_bytes()
    }
}

/// Parse a base64-encoded public key.
pub fn parse_public_key(base64_key: &str) -> Result<PublicKey, KeyExchangeError> {
    let bytes = BASE64.decode(base64_key)?;
    if bytes.len() != 32 {
        return Err(KeyExchangeError::InvalidKeyLength(bytes.len()));
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(PublicKey::from(array))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp = KeyPair::generate();
        let base64_key = kp.public_key_base64();
        assert!(!base64_key.is_empty());

        // Should be 44 characters (32 bytes base64 encoded with padding)
        assert_eq!(base64_key.len(), 44);
    }

    #[test]
    fn test_parse_public_key() {
        let kp = KeyPair::generate();
        let base64_key = kp.public_key_base64();

        let parsed = parse_public_key(&base64_key).unwrap();
        assert_eq!(parsed.as_bytes(), kp.public_key_bytes().as_slice());
    }

    #[test]
    fn test_key_exchange() {
        // Alice generates keypair
        let alice = KeyPair::generate();
        let alice_public = alice.public_key_base64();

        // Bob generates keypair
        let bob = KeyPair::generate();
        let bob_public = bob.public_key_base64();

        // Both derive the same shared secret
        let alice_peer = parse_public_key(&bob_public).unwrap();
        let bob_peer = parse_public_key(&alice_public).unwrap();

        let alice_shared = alice.diffie_hellman(alice_peer);
        let bob_shared = bob.diffie_hellman(bob_peer);

        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn test_invalid_key_length() {
        let result = parse_public_key("aGVsbG8="); // "hello" in base64
        assert!(matches!(result, Err(KeyExchangeError::InvalidKeyLength(5))));
    }

    #[test]
    fn test_invalid_base64() {
        let result = parse_public_key("not valid base64!!!");
        assert!(matches!(
            result,
            Err(KeyExchangeError::Base64DecodeError(_))
        ));
    }

    #[test]
    fn test_public_key_bytes() {
        let kp = KeyPair::generate();
        let bytes = kp.public_key_bytes();
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn test_different_keypairs_produce_different_keys() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();

        assert_ne!(kp1.public_key_bytes(), kp2.public_key_bytes());
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            KeyExchangeError::InvalidKeyLength(16).to_string(),
            "Invalid public key length: expected 32 bytes, got 16"
        );
    }

    #[test]
    fn test_empty_base64() {
        let result = parse_public_key("");
        // Empty string decodes to empty bytes, which is wrong length
        assert!(matches!(result, Err(KeyExchangeError::InvalidKeyLength(0))));
    }

    #[test]
    fn test_key_exchange_produces_32_byte_secret() {
        let alice = KeyPair::generate();
        let bob = KeyPair::generate();

        let bob_public = parse_public_key(&bob.public_key_base64()).unwrap();
        let shared = alice.diffie_hellman(bob_public);

        assert_eq!(shared.len(), 32);
    }
}
