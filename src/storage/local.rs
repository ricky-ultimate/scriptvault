use super::{StorageBackend, StorageMetadata, SyncStatus};
use crate::script::Script;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Local filesystem storage implementation
pub struct LocalStorage {
    vault_path: PathBuf,
    scripts_file: PathBuf,
}

impl LocalStorage {
    /// Create a new local storage backend
    pub fn new(vault_path: PathBuf) -> Result<Self> {
        // Ensure vault directory exists
        fs::create_dir_all(&vault_path).context("Failed to create vault directory")?;

        let scripts_file = vault_path.join("scripts.json");

        // Initialize empty scripts file if it doesn't exist
        if !scripts_file.exists() {
            let empty_scripts: Vec<Script> = vec![];
            let json = serde_json::to_string_pretty(&empty_scripts)?;
            fs::write(&scripts_file, json).context("Failed to initialize scripts file")?;
        }

        Ok(Self {
            vault_path,
            scripts_file,
        })
    }

    /// Load all scripts from disk
    fn load_all_scripts(&self) -> Result<Vec<Script>> {
        if !self.scripts_file.exists() {
            return Ok(Vec::new());
        }

        let contents =
            fs::read_to_string(&self.scripts_file).context("Failed to read scripts file")?;

        let scripts: Vec<Script> =
            serde_json::from_str(&contents).context("Failed to parse scripts file")?;

        Ok(scripts)
    }

    /// Save all scripts to disk
    fn save_all_scripts(&self, scripts: &[Script]) -> Result<()> {
        let json = serde_json::to_string_pretty(scripts).context("Failed to serialize scripts")?;

        fs::write(&self.scripts_file, json).context("Failed to write scripts file")?;

        Ok(())
    }

    /// Calculate total storage size
    fn calculate_total_size(&self, scripts: &[Script]) -> u64 {
        scripts.iter().map(|s| s.metadata.size_bytes as u64).sum()
    }
}

impl StorageBackend for LocalStorage {
    fn save_script(&self, script: &Script) -> Result<()> {
        let mut scripts = self.load_all_scripts()?;

        // Remove existing script with same ID or name
        scripts.retain(|s| s.id != script.id && s.name != script.name);

        // Add the new/updated script
        scripts.push(script.clone());

        self.save_all_scripts(&scripts)?;

        Ok(())
    }

    fn load_script(&self, id: &str) -> Result<Script> {
        let scripts = self.load_all_scripts()?;

        scripts
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| anyhow::anyhow!("Script not found with ID: {}", id))
    }

    fn load_script_by_name(&self, name: &str) -> Result<Script> {
        let scripts = self.load_all_scripts()?;

        scripts
            .into_iter()
            .find(|s| s.name == name)
            .ok_or_else(|| anyhow::anyhow!("Script not found with name: {}", name))
    }

    fn list_scripts(&self) -> Result<Vec<Script>> {
        self.load_all_scripts()
    }

    fn delete_script(&self, id: &str) -> Result<()> {
        let mut scripts = self.load_all_scripts()?;

        let original_len = scripts.len();
        scripts.retain(|s| s.id != id);

        if scripts.len() == original_len {
            anyhow::bail!("Script not found with ID: {}", id);
        }

        self.save_all_scripts(&scripts)?;

        Ok(())
    }

    fn script_exists(&self, id: &str) -> Result<bool> {
        let scripts = self.load_all_scripts()?;
        Ok(scripts.iter().any(|s| s.id == id))
    }

    fn get_metadata(&self) -> Result<StorageMetadata> {
        let scripts = self.load_all_scripts()?;
        let total_size = self.calculate_total_size(&scripts);

        Ok(StorageMetadata {
            total_scripts: scripts.len(),
            total_size_bytes: total_size,
            last_sync: None, // Local storage doesn't sync
            backend_type: self.backend_type().to_string(),
        })
    }

    fn health_check(&self) -> Result<bool> {
        // Check if vault directory exists and is writable
        if !self.vault_path.exists() {
            return Ok(false);
        }

        // Try to read scripts file
        if !self.scripts_file.exists() {
            return Ok(false);
        }

        // Try to parse scripts
        self.load_all_scripts()?;

        Ok(true)
    }

    fn get_sync_status(&self, _script_id: &str) -> Result<SyncStatus> {
        // Local storage is always "synced" (no remote)
        Ok(SyncStatus::Synced)
    }

    fn backend_type(&self) -> &str {
        "local"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::{ScriptLanguage, ScriptMetadata};
    use chrono::Utc;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_script(name: &str) -> Script {
        Script {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            content: format!("echo '{}'", name),
            version: "v1.0.0".to_string(),
            language: ScriptLanguage::Bash,
            tags: vec![],
            description: None,
            author: "test".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            context: crate::script::ScriptContext {
                directory: None,
                git_repo: None,
                git_branch: None,
                environment: HashMap::new(),
            },
            metadata: ScriptMetadata {
                hash: "test".to_string(),
                size_bytes: 10,
                line_count: 1,
                use_count: 0,
                success_count: 0,
                failure_count: 0,
                last_run: None,
                last_run_by: None,
                avg_runtime_ms: None,
            },
            visibility: crate::script::Visibility::Private,
        }
    }

    #[test]
    fn test_local_storage_creation() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_path_buf()).unwrap();

        assert!(storage.vault_path.exists());
        assert!(storage.scripts_file.exists());
    }

    #[test]
    fn test_save_and_load_script() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let script = create_test_script("test-script");
        let script_id = script.id.clone();

        // Save script
        storage.save_script(&script).unwrap();

        // Load script
        let loaded = storage.load_script(&script_id).unwrap();
        assert_eq!(loaded.name, "test-script");
    }

    #[test]
    fn test_list_scripts() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_path_buf()).unwrap();

        // Save multiple scripts
        storage.save_script(&create_test_script("script1")).unwrap();
        storage.save_script(&create_test_script("script2")).unwrap();
        storage.save_script(&create_test_script("script3")).unwrap();

        // List scripts
        let scripts = storage.list_scripts().unwrap();
        assert_eq!(scripts.len(), 3);
    }

    #[test]
    fn test_delete_script() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let script = create_test_script("to-delete");
        let script_id = script.id.clone();

        storage.save_script(&script).unwrap();
        assert!(storage.script_exists(&script_id).unwrap());

        storage.delete_script(&script_id).unwrap();
        assert!(!storage.script_exists(&script_id).unwrap());
    }

    #[test]
    fn test_load_by_name() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let script = create_test_script("find-me");
        storage.save_script(&script).unwrap();

        let loaded = storage.load_script_by_name("find-me").unwrap();
        assert_eq!(loaded.name, "find-me");
    }

    #[test]
    fn test_update_existing_script() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let mut script = create_test_script("update-test");
        storage.save_script(&script).unwrap();

        // Update script
        script.content = "echo 'updated'".to_string();
        storage.save_script(&script).unwrap();

        // Should still have only one script
        let scripts = storage.list_scripts().unwrap();
        assert_eq!(scripts.len(), 1);

        // Content should be updated
        let loaded = storage.load_script_by_name("update-test").unwrap();
        assert_eq!(loaded.content, "echo 'updated'");
    }

    #[test]
    fn test_health_check() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_path_buf()).unwrap();

        assert!(storage.health_check().unwrap());
    }

    #[test]
    fn test_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_path_buf()).unwrap();

        storage.save_script(&create_test_script("script1")).unwrap();
        storage.save_script(&create_test_script("script2")).unwrap();

        let metadata = storage.get_metadata().unwrap();
        assert_eq!(metadata.total_scripts, 2);
        assert_eq!(metadata.backend_type, "local");
    }
}
