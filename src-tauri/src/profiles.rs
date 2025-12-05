//! Profile storage manager with encrypted credentials.
//!
//! Handles loading, saving, and migrating S3 connection profiles.
//! Credentials are encrypted using AES-256-GCM before being stored on disk.

use crate::crypto::Crypto;
use crate::errors::AppError;
use crate::models::{EncryptedProfile, Profile};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Current storage format version
const STORAGE_VERSION: u32 = 2;

/// Profile storage with encryption support
#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileStore {
    /// Storage format version for migration detection
    #[serde(default = "default_version")]
    version: u32,
    /// Encrypted profiles stored on disk
    profiles: HashMap<String, EncryptedProfile>,
    /// Runtime-only: decrypted profiles cache (not serialized)
    #[serde(skip)]
    decrypted_cache: HashMap<String, Profile>,
    /// Runtime-only: crypto handler (not serialized)
    #[serde(skip)]
    crypto: Option<Crypto>,
}

fn default_version() -> u32 {
    1 // Old format without encryption
}

impl ProfileStore {
    /// Create a new empty profile store
    pub fn new() -> Self {
        Self {
            version: STORAGE_VERSION,
            profiles: HashMap::new(),
            decrypted_cache: HashMap::new(),
            crypto: Some(Crypto::new().expect("Failed to initialize crypto")),
        }
    }

    /// Get the crypto handler with proper error handling
    fn get_crypto(&self) -> Result<&Crypto, AppError> {
        self.crypto
            .as_ref()
            .ok_or_else(|| AppError::CryptoError("Crypto not initialized".into()))
    }

    /// Load profiles from disk with automatic migration
    pub fn load() -> Result<Self, AppError> {
        let path = Self::get_storage_path()?;

        if !path.exists() {
            return Ok(Self::new());
        }

        let content =
            fs::read_to_string(&path).map_err(|e| AppError::ProfileStorageError(e.to_string()))?;

        // Parse the JSON
        let mut store: ProfileStore = serde_json::from_str(&content)?;

        // Initialize crypto
        store.crypto = Some(Crypto::new()?);

        // Check if migration is needed
        let needs_migration =
            store.version < STORAGE_VERSION || store.profiles.values().any(|p| p.needs_migration());

        if needs_migration {
            store.migrate()?;
        }

        // Decrypt and cache all profiles
        store.decrypt_all()?;

        Ok(store)
    }

    /// Migrate old plaintext profiles to encrypted format
    fn migrate(&mut self) -> Result<(), AppError> {
        let crypto = self.get_crypto()?;

        // Re-encrypt all profiles that need migration
        let mut migrated_profiles = HashMap::new();
        for (id, encrypted_profile) in &self.profiles {
            if encrypted_profile.needs_migration() {
                // Decrypt (plaintext in this case) and re-encrypt properly
                let profile = encrypted_profile.to_decrypted(crypto)?;
                let new_encrypted = profile.to_encrypted(crypto)?;
                migrated_profiles.insert(id.clone(), new_encrypted);
            } else {
                migrated_profiles.insert(id.clone(), encrypted_profile.clone());
            }
        }

        self.profiles = migrated_profiles;
        self.version = STORAGE_VERSION;

        // Save the migrated data
        self.save_internal()?;

        Ok(())
    }

    /// Decrypt all profiles into the cache
    fn decrypt_all(&mut self) -> Result<(), AppError> {
        let crypto = self
            .crypto
            .as_ref()
            .ok_or_else(|| AppError::CryptoError("Crypto not initialized".into()))?;

        self.decrypted_cache.clear();
        for (id, encrypted_profile) in &self.profiles {
            let profile = encrypted_profile.to_decrypted(crypto)?;
            self.decrypted_cache.insert(id.clone(), profile);
        }
        Ok(())
    }

    /// Save profiles to disk (internal, doesn't update cache)
    fn save_internal(&self) -> Result<(), AppError> {
        let path = Self::get_storage_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::ProfileStorageError(e.to_string()))?;
        }

        let content = serde_json::to_string_pretty(&self)?;
        fs::write(&path, content).map_err(|e| AppError::ProfileStorageError(e.to_string()))?;

        Ok(())
    }

    /// Save profiles to disk
    pub fn save(&self) -> Result<(), AppError> {
        self.save_internal()
    }

    /// Get the storage file path
    fn get_storage_path() -> Result<PathBuf, AppError> {
        let app_dir = dirs::config_dir()
            .ok_or_else(|| AppError::ConfigError("Cannot find config directory".to_string()))?;

        Ok(app_dir.join("s3explorer").join("profiles.json"))
    }

    /// List all profiles (decrypted)
    pub fn list(&self) -> Vec<Profile> {
        self.decrypted_cache.values().cloned().collect()
    }

    /// Get a profile by ID (decrypted)
    pub fn get(&self, id: &str) -> Result<Profile, AppError> {
        self.decrypted_cache
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::ProfileNotFound(id.to_string()))
    }

    /// Add or update a profile
    pub fn upsert(&mut self, profile: Profile) -> Result<(), AppError> {
        let crypto = self
            .crypto
            .as_ref()
            .ok_or_else(|| AppError::CryptoError("Crypto not initialized".into()))?;

        // Encrypt and store
        let encrypted = profile.to_encrypted(crypto)?;
        self.profiles.insert(profile.id.clone(), encrypted);

        // Update cache
        self.decrypted_cache.insert(profile.id.clone(), profile);

        // Save to disk
        self.save()?;
        Ok(())
    }

    /// Delete a profile
    pub fn delete(&mut self, id: &str) -> Result<(), AppError> {
        self.profiles
            .remove(id)
            .ok_or_else(|| AppError::ProfileNotFound(id.to_string()))?;

        self.decrypted_cache.remove(id);

        self.save()?;
        Ok(())
    }
}

impl Default for ProfileStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_profile(id: &str) -> Profile {
        Profile {
            id: id.to_string(),
            name: "Test Profile".to_string(),
            endpoint: Some("http://localhost:9000".to_string()),
            region: Some("us-east-1".to_string()),
            access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            session_token: None,
            path_style: true,
        }
    }

    #[test]
    fn test_profile_store_new() {
        let store = ProfileStore::new();
        assert_eq!(store.list().len(), 0);
        assert_eq!(store.version, STORAGE_VERSION);
    }

    #[test]
    fn test_profile_encrypt_decrypt_roundtrip() {
        let mut store = ProfileStore::new();
        let profile = create_test_profile("test-1");

        // Upsert encrypts and caches
        store.upsert(profile.clone()).unwrap();

        // Get should return decrypted profile
        let retrieved = store.get("test-1").unwrap();
        assert_eq!(retrieved.name, "Test Profile");
        assert_eq!(retrieved.access_key, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(
            retrieved.secret_key,
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
        );

        // Verify the stored version is encrypted
        let encrypted = store.profiles.get("test-1").unwrap();
        assert!(encrypted.encrypted);
        assert_ne!(encrypted.access_key_encrypted, profile.access_key);
        assert_ne!(encrypted.secret_key_encrypted, profile.secret_key);
    }

    #[test]
    fn test_profile_delete() {
        let mut store = ProfileStore::new();
        let profile = create_test_profile("test-1");

        store.upsert(profile).unwrap();
        assert_eq!(store.list().len(), 1);

        store.delete("test-1").unwrap();
        assert_eq!(store.list().len(), 0);
    }

    #[test]
    fn test_profile_not_found() {
        let store = ProfileStore::new();
        let result = store.get("non-existent");
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_with_session_token() {
        let mut store = ProfileStore::new();
        let mut profile = create_test_profile("test-1");
        profile.session_token = Some("FwoGZXIvYXdzEBYaD...".to_string());

        store.upsert(profile).unwrap();

        let retrieved = store.get("test-1").unwrap();
        assert_eq!(
            retrieved.session_token,
            Some("FwoGZXIvYXdzEBYaD...".to_string())
        );

        // Verify session token is encrypted
        let encrypted = store.profiles.get("test-1").unwrap();
        assert!(encrypted.session_token_encrypted.is_some());
        assert_ne!(
            encrypted.session_token_encrypted.as_ref().unwrap(),
            "FwoGZXIvYXdzEBYaD..."
        );
    }
}
