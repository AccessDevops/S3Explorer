//! Cryptographic utilities for encrypting sensitive profile data.
//!
//! Uses AES-256-GCM with an embedded key for obfuscation of credentials.
//! This prevents credentials from being stored in plaintext while keeping
//! the solution simple and portable across machines.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;

use crate::errors::AppError;

/// AES-256 embedded key (32 bytes).
/// This key is compiled into the binary and used for all encryption/decryption.
/// WARNING: Do not change this key, or existing encrypted profiles will become unreadable.
const EMBEDDED_KEY: [u8; 32] = [
    0x7a, 0x4b, 0x9c, 0x2d, 0x8f, 0x1e, 0x6a, 0x3b,
    0xc5, 0x0d, 0x9f, 0x2e, 0x7b, 0x4c, 0x8d, 0x1f,
    0x6e, 0x3a, 0xb9, 0x0c, 0x5d, 0x2f, 0x8e, 0x4a,
    0x1b, 0x7c, 0x9d, 0x3e, 0x6f, 0x0a, 0xb8, 0x5c,
];

/// Nonce size for AES-GCM (96 bits = 12 bytes)
const NONCE_SIZE: usize = 12;

/// Cryptographic handler for profile credentials
pub struct Crypto {
    cipher: Aes256Gcm,
}

// Manual Debug implementation that doesn't expose the cipher
impl std::fmt::Debug for Crypto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Crypto")
            .field("cipher", &"[REDACTED]")
            .finish()
    }
}

impl Crypto {
    /// Create a new Crypto instance with the embedded key
    pub fn new() -> Result<Self, AppError> {
        let cipher = Aes256Gcm::new_from_slice(&EMBEDDED_KEY)
            .map_err(|e| AppError::CryptoError(format!("Failed to initialize cipher: {}", e)))?;
        Ok(Self { cipher })
    }

    /// Encrypt a plaintext string and return base64-encoded ciphertext.
    /// The output format is: base64(nonce || ciphertext || auth_tag)
    pub fn encrypt(&self, plaintext: &str) -> Result<String, AppError> {
        if plaintext.is_empty() {
            return Ok(String::new());
        }

        // Generate a random nonce for each encryption
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the plaintext
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| AppError::CryptoError(format!("Encryption failed: {}", e)))?;

        // Combine nonce + ciphertext and encode as base64
        let mut combined = nonce_bytes.to_vec();
        combined.extend(ciphertext);
        Ok(BASE64.encode(&combined))
    }

    /// Decrypt a base64-encoded ciphertext and return the plaintext string.
    pub fn decrypt(&self, ciphertext_b64: &str) -> Result<String, AppError> {
        if ciphertext_b64.is_empty() {
            return Ok(String::new());
        }

        // Decode from base64
        let combined = BASE64
            .decode(ciphertext_b64)
            .map_err(|e| AppError::CryptoError(format!("Invalid base64 encoding: {}", e)))?;

        // Ensure we have at least nonce + some ciphertext
        if combined.len() <= NONCE_SIZE {
            return Err(AppError::CryptoError(
                "Ciphertext too short (missing nonce or data)".into(),
            ));
        }

        // Split into nonce and ciphertext
        let (nonce_bytes, ciphertext) = combined.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let plaintext_bytes = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::CryptoError(format!("Decryption failed: {}", e)))?;

        // Convert to UTF-8 string
        String::from_utf8(plaintext_bytes)
            .map_err(|e| AppError::CryptoError(format!("Invalid UTF-8 in decrypted data: {}", e)))
    }

    /// Encrypt an optional string (returns None if input is None)
    pub fn encrypt_option(&self, plaintext: Option<&str>) -> Result<Option<String>, AppError> {
        match plaintext {
            Some(s) if !s.is_empty() => Ok(Some(self.encrypt(s)?)),
            _ => Ok(None),
        }
    }

    /// Decrypt an optional string (returns None if input is None)
    pub fn decrypt_option(&self, ciphertext: Option<&str>) -> Result<Option<String>, AppError> {
        match ciphertext {
            Some(s) if !s.is_empty() => Ok(Some(self.decrypt(s)?)),
            _ => Ok(None),
        }
    }
}

impl Default for Crypto {
    fn default() -> Self {
        Self::new().expect("Failed to initialize crypto with embedded key")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let crypto = Crypto::new().unwrap();
        let original = "AKIAIOSFODNN7EXAMPLE";

        let encrypted = crypto.encrypt(original).unwrap();
        assert_ne!(encrypted, original);
        assert!(!encrypted.is_empty());

        let decrypted = crypto.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_encrypt_empty_string() {
        let crypto = Crypto::new().unwrap();
        let encrypted = crypto.encrypt("").unwrap();
        assert!(encrypted.is_empty());

        let decrypted = crypto.decrypt("").unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn test_encrypt_produces_different_ciphertext() {
        let crypto = Crypto::new().unwrap();
        let original = "secret-key-123";

        let encrypted1 = crypto.encrypt(original).unwrap();
        let encrypted2 = crypto.encrypt(original).unwrap();

        // Same plaintext should produce different ciphertext (due to random nonce)
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same value
        assert_eq!(crypto.decrypt(&encrypted1).unwrap(), original);
        assert_eq!(crypto.decrypt(&encrypted2).unwrap(), original);
    }

    #[test]
    fn test_encrypt_option() {
        let crypto = Crypto::new().unwrap();

        let result = crypto.encrypt_option(Some("test")).unwrap();
        assert!(result.is_some());

        let result = crypto.encrypt_option(None).unwrap();
        assert!(result.is_none());

        let result = crypto.encrypt_option(Some("")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_ciphertext() {
        let crypto = Crypto::new().unwrap();

        // Too short
        let result = crypto.decrypt("YWJj"); // "abc" in base64
        assert!(result.is_err());

        // Invalid base64
        let result = crypto.decrypt("not-valid-base64!!!");
        assert!(result.is_err());
    }
}
