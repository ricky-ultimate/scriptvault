pub mod commands;
pub mod local;

use crate::script::{Script, SyncStatus};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub path: PathBuf,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from(".scriptvault/vault"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetadata {
    pub total_scripts: usize,
    pub total_size_bytes: u64,
    pub last_sync: Option<DateTime<Utc>>,
    pub backend_type: String,
}

pub trait StorageBackend: Send + Sync {
    fn save_script(&self, script: &Script) -> Result<()>;
    fn update_script(&self, script: &Script) -> Result<()>;
    fn load_script(&self, id: &str) -> Result<Script>;
    fn load_script_by_name(&self, name: &str) -> Result<Script>;
    fn list_scripts(&self) -> Result<Vec<Script>>;
    fn delete_script(&self, id: &str) -> Result<()>;
    fn script_exists(&self, id: &str) -> Result<bool>;
    fn get_metadata(&self) -> Result<StorageMetadata>;
    fn health_check(&self) -> Result<bool>;
    fn get_sync_status(&self, script_id: &str) -> Result<SyncStatus>;
    fn mark_synced(
        &self,
        script_id: &str,
        remote_version: &str,
        synced_at: DateTime<Utc>,
    ) -> Result<()>;
    fn mark_conflict(&self, script_id: &str) -> Result<()>;
    fn list_pending_push(&self) -> Result<Vec<Script>>;
    fn list_conflicts(&self) -> Result<Vec<Script>>;
    fn backend_type(&self) -> &str;
}

pub fn create_storage_backend(config: &StorageConfig) -> Result<Box<dyn StorageBackend>> {
    let backend = local::LocalStorage::new(config.path.clone())?;
    Ok(Box::new(backend))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_storage_config() {
        let config = StorageConfig::default();
        assert_eq!(config.path, PathBuf::from(".scriptvault/vault"));
    }

    #[test]
    fn test_storage_config_serialization() {
        let config = StorageConfig {
            path: PathBuf::from("/test/path"),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: StorageConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, PathBuf::from("/test/path"));
    }
}
