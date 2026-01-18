pub mod local;

use crate::script::Script;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for different storage backends
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StorageConfig {
    /// Local filesystem storage (default)
    Local {
        path: PathBuf,
    },
    /// Backblaze B2 cloud storage
    Backblaze {
        key_id: String,
        application_key: String,
        bucket_name: String,
        endpoint: Option<String>,
    },
    /// AWS S3 storage (future)
    S3 {
        access_key: String,
        secret_key: String,
        bucket: String,
        region: String,
    },
    /// Google Cloud Storage (future)
    Gcs {
        project_id: String,
        bucket: String,
        credentials_path: PathBuf,
    },
    /// Azure Blob Storage (future)
    Azure {
        account_name: String,
        account_key: String,
        container: String,
    },
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self::Local {
            path: PathBuf::from(".scriptvault/vault"),
        }
    }
}

/// Metadata about stored scripts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetadata {
    pub total_scripts: usize,
    pub total_size_bytes: u64,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    pub backend_type: String,
}

/// Sync status for a script
#[derive(Debug, Clone, PartialEq)]
pub enum SyncStatus {
    /// Script is up to date
    Synced,
    /// Local version is newer
    LocalNewer,
    /// Remote version is newer
    RemoteNewer,
    /// Script only exists locally
    LocalOnly,
    /// Script only exists remotely
    RemoteOnly,
    /// Conflict - both modified
    Conflict,
}

/// Core storage backend trait
/// All storage implementations must implement this trait
pub trait StorageBackend: Send + Sync {
    /// Save a script to storage
    fn save_script(&self, script: &Script) -> Result<()>;

    /// Load a script by ID
    fn load_script(&self, id: &str) -> Result<Script>;

    /// Load a script by name
    fn load_script_by_name(&self, name: &str) -> Result<Script>;

    /// List all scripts
    fn list_scripts(&self) -> Result<Vec<Script>>;

    /// Delete a script by ID
    fn delete_script(&self, id: &str) -> Result<()>;

    /// Check if a script exists
    fn script_exists(&self, id: &str) -> Result<bool>;

    /// Get storage metadata (stats)
    fn get_metadata(&self) -> Result<StorageMetadata>;

    /// Health check - verify storage is accessible
    fn health_check(&self) -> Result<bool>;

    /// Sync local changes to remote (for cloud backends)
    fn sync_push(&self) -> Result<Vec<String>> {
        // Default implementation for local-only storage
        Ok(vec![])
    }

    /// Sync remote changes to local (for cloud backends)
    fn sync_pull(&self) -> Result<Vec<String>> {
        // Default implementation for local-only storage
        Ok(vec![])
    }

    /// Get sync status for a script
    fn get_sync_status(&self, _script_id: &str) -> Result<SyncStatus> {
        // Default: always synced for local storage
        Ok(SyncStatus::Synced)
    }

    /// Get backend type name
    fn backend_type(&self) -> &str;
}

/// Factory function to create storage backend from config
pub fn create_storage_backend(config: &StorageConfig) -> Result<Box<dyn StorageBackend>> {
    match config {
        StorageConfig::Local { path } => {
            let backend = local::LocalStorage::new(path.clone())?;
            Ok(Box::new(backend))
        }
        StorageConfig::Backblaze { .. } => {
            anyhow::bail!("Backblaze B2 storage not yet implemented. Coming in Phase 3!");
        }
        StorageConfig::S3 { .. } => {
            anyhow::bail!("S3 storage not yet implemented. Coming in Phase 7!");
        }
        StorageConfig::Gcs { .. } => {
            anyhow::bail!("Google Cloud Storage not yet implemented. Coming in Phase 7!");
        }
        StorageConfig::Azure { .. } => {
            anyhow::bail!("Azure Blob Storage not yet implemented. Coming in Phase 7!");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_storage_config() {
        let config = StorageConfig::default();
        match config {
            StorageConfig::Local { path } => {
                assert_eq!(path, PathBuf::from(".scriptvault/vault"));
            }
            _ => panic!("Default should be Local storage"),
        }
    }

    #[test]
    fn test_storage_config_serialization() {
        let config = StorageConfig::Local {
            path: PathBuf::from("/test/path"),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: StorageConfig = serde_json::from_str(&json).unwrap();

        match deserialized {
            StorageConfig::Local { path } => {
                assert_eq!(path, PathBuf::from("/test/path"));
            }
            _ => panic!("Should deserialize to Local"),
        }
    }
}
