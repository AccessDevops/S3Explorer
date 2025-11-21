use crate::errors::AppError;
use crate::models::Profile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Profile storage manager
#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileStore {
    profiles: HashMap<String, Profile>,
}

impl ProfileStore {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }

    /// Load profiles from disk
    pub fn load() -> Result<Self, AppError> {
        let path = Self::get_storage_path()?;

        if !path.exists() {
            return Ok(Self::new());
        }

        let content =
            fs::read_to_string(&path).map_err(|e| AppError::ProfileStorageError(e.to_string()))?;

        let store: ProfileStore = serde_json::from_str(&content)?;
        Ok(store)
    }

    /// Save profiles to disk
    pub fn save(&self) -> Result<(), AppError> {
        let path = Self::get_storage_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::ProfileStorageError(e.to_string()))?;
        }

        let content = serde_json::to_string_pretty(&self)?;
        fs::write(&path, content).map_err(|e| AppError::ProfileStorageError(e.to_string()))?;

        Ok(())
    }

    /// Get the storage file path
    fn get_storage_path() -> Result<PathBuf, AppError> {
        let app_dir = dirs::config_dir()
            .ok_or_else(|| AppError::ConfigError("Cannot find config directory".to_string()))?;

        Ok(app_dir.join("s3browser").join("profiles.json"))
    }

    /// List all profiles
    pub fn list(&self) -> Vec<Profile> {
        self.profiles.values().cloned().collect()
    }

    /// Get a profile by ID
    pub fn get(&self, id: &str) -> Result<Profile, AppError> {
        self.profiles
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::ProfileNotFound(id.to_string()))
    }

    /// Add or update a profile
    pub fn upsert(&mut self, profile: Profile) -> Result<(), AppError> {
        self.profiles.insert(profile.id.clone(), profile);
        self.save()?;
        Ok(())
    }

    /// Delete a profile
    pub fn delete(&mut self, id: &str) -> Result<(), AppError> {
        self.profiles
            .remove(id)
            .ok_or_else(|| AppError::ProfileNotFound(id.to_string()))?;
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

    #[test]
    fn test_profile_store_new() {
        let store = ProfileStore::new();
        assert_eq!(store.list().len(), 0);
    }

    #[test]
    fn test_profile_upsert_and_get() {
        let mut store = ProfileStore::new();
        let profile = Profile {
            id: "test-1".to_string(),
            name: "Test Profile".to_string(),
            endpoint: Some("http://localhost:9000".to_string()),
            region: "us-east-1".to_string(),
            access_key: "test-key".to_string(),
            secret_key: "test-secret".to_string(),
            session_token: None,
            path_style: true,
            use_tls: false,
        };

        store.profiles.insert(profile.id.clone(), profile.clone());

        let retrieved = store.get("test-1").unwrap();
        assert_eq!(retrieved.name, "Test Profile");
        assert_eq!(retrieved.access_key, "test-key");
    }

    #[test]
    fn test_profile_delete() {
        let mut store = ProfileStore::new();
        let profile = Profile {
            id: "test-1".to_string(),
            name: "Test Profile".to_string(),
            endpoint: None,
            region: "us-east-1".to_string(),
            access_key: "test-key".to_string(),
            secret_key: "test-secret".to_string(),
            session_token: None,
            path_style: false,
            use_tls: true,
        };

        store.profiles.insert(profile.id.clone(), profile);
        assert_eq!(store.list().len(), 1);

        store.profiles.remove("test-1");
        assert_eq!(store.list().len(), 0);
    }

    #[test]
    fn test_profile_not_found() {
        let store = ProfileStore::new();
        let result = store.get("non-existent");
        assert!(result.is_err());
    }
}
