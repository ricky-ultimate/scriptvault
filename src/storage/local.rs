use super::{StorageBackend, StorageMetadata, SyncStatus};
use crate::script::Script;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub struct LocalStorage {
    vault_path: PathBuf,
    scripts_file: PathBuf,
}

impl LocalStorage {
    pub fn new(vault_path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&vault_path).context("Failed to create vault directory")?;

        let scripts_file = vault_path.join("scripts.json");

        if !scripts_file.exists() {
            let empty: Vec<Script> = vec![];
            let json = serde_json::to_string_pretty(&empty)?;
            fs::write(&scripts_file, json).context("Failed to initialize scripts file")?;
        }

        Ok(Self {
            vault_path,
            scripts_file,
        })
    }

    fn load_all(&self) -> Result<Vec<Script>> {
        if !self.scripts_file.exists() {
            return Ok(Vec::new());
        }
        let contents =
            fs::read_to_string(&self.scripts_file).context("Failed to read scripts file")?;
        let scripts: Vec<Script> =
            serde_json::from_str(&contents).context("Failed to parse scripts file")?;
        Ok(scripts)
    }

    fn persist(&self, scripts: &[Script]) -> Result<()> {
        let json =
            serde_json::to_string_pretty(scripts).context("Failed to serialize scripts")?;
        fs::write(&self.scripts_file, json).context("Failed to write scripts file")?;
        Ok(())
    }

    fn total_size(scripts: &[Script]) -> u64 {
        scripts.iter().map(|s| s.metadata.size_bytes as u64).sum()
    }
}

impl StorageBackend for LocalStorage {
    fn save_script(&self, script: &Script) -> Result<()> {
        let mut scripts = self.load_all()?;
        scripts.retain(|s| s.id != script.id && s.name != script.name);
        scripts.push(script.clone());
        self.persist(&scripts)
    }

    fn update_script(&self, script: &Script) -> Result<()> {
        let mut scripts = self.load_all()?;
        let pos = scripts
            .iter()
            .position(|s| s.id == script.id)
            .ok_or_else(|| anyhow::anyhow!("Script not found: {}", script.id))?;
        scripts[pos] = script.clone();
        self.persist(&scripts)
    }

    fn load_script(&self, id: &str) -> Result<Script> {
        self.load_all()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| anyhow::anyhow!("Script not found with ID: {}", id))
    }

    fn load_script_by_name(&self, name: &str) -> Result<Script> {
        self.load_all()?
            .into_iter()
            .find(|s| s.name == name)
            .ok_or_else(|| anyhow::anyhow!("Script not found: {}", name))
    }

    fn list_scripts(&self) -> Result<Vec<Script>> {
        self.load_all()
    }

    fn delete_script(&self, id: &str) -> Result<()> {
        let mut scripts = self.load_all()?;
        let original_len = scripts.len();
        scripts.retain(|s| s.id != id);
        if scripts.len() == original_len {
            anyhow::bail!("Script not found with ID: {}", id);
        }
        self.persist(&scripts)
    }

    fn script_exists(&self, id: &str) -> Result<bool> {
        Ok(self.load_all()?.iter().any(|s| s.id == id))
    }

    fn get_metadata(&self) -> Result<StorageMetadata> {
        let scripts = self.load_all()?;
        let total_size = Self::total_size(&scripts);
        Ok(StorageMetadata {
            total_scripts: scripts.len(),
            total_size_bytes: total_size,
            last_sync: None,
            backend_type: self.backend_type().to_string(),
        })
    }

    fn health_check(&self) -> Result<bool> {
        if !self.vault_path.exists() || !self.scripts_file.exists() {
            return Ok(false);
        }
        self.load_all()?;
        Ok(true)
    }

    fn get_sync_status(&self, _script_id: &str) -> Result<SyncStatus> {
        Ok(SyncStatus::Synced)
    }

    fn backend_type(&self) -> &str {
        "local"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::{ScriptContext, ScriptLanguage, ScriptMetadata, Visibility};
    use chrono::Utc;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn make_script(name: &str) -> Script {
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
            context: ScriptContext {
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
            visibility: Visibility::Private,
        }
    }

    #[test]
    fn test_creation() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        assert!(storage.vault_path.exists());
        assert!(storage.scripts_file.exists());
    }

    #[test]
    fn test_save_and_load_by_id() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let script = make_script("test-script");
        let id = script.id.clone();
        storage.save_script(&script).unwrap();
        let loaded = storage.load_script(&id).unwrap();
        assert_eq!(loaded.name, "test-script");
    }

    #[test]
    fn test_save_and_load_by_name() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        storage.save_script(&make_script("find-me")).unwrap();
        let loaded = storage.load_script_by_name("find-me").unwrap();
        assert_eq!(loaded.name, "find-me");
    }

    #[test]
    fn test_update_modifies_in_place() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let mut script = make_script("update-test");
        storage.save_script(&script).unwrap();
        script.content = "echo 'updated'".to_string();
        storage.update_script(&script).unwrap();
        let scripts = storage.list_scripts().unwrap();
        assert_eq!(scripts.len(), 1);
        let loaded = storage.load_script_by_name("update-test").unwrap();
        assert_eq!(loaded.content, "echo 'updated'");
    }

    #[test]
    fn test_update_unknown_id_errors() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let script = make_script("ghost");
        assert!(storage.update_script(&script).is_err());
    }

    #[test]
    fn test_list() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        storage.save_script(&make_script("s1")).unwrap();
        storage.save_script(&make_script("s2")).unwrap();
        storage.save_script(&make_script("s3")).unwrap();
        assert_eq!(storage.list_scripts().unwrap().len(), 3);
    }

    #[test]
    fn test_delete() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let script = make_script("to-delete");
        let id = script.id.clone();
        storage.save_script(&script).unwrap();
        assert!(storage.script_exists(&id).unwrap());
        storage.delete_script(&id).unwrap();
        assert!(!storage.script_exists(&id).unwrap());
    }

    #[test]
    fn test_save_deduplicates_by_name() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let mut s1 = make_script("same-name");
        storage.save_script(&s1).unwrap();
        s1.id = uuid::Uuid::new_v4().to_string();
        storage.save_script(&s1).unwrap();
        assert_eq!(storage.list_scripts().unwrap().len(), 1);
    }

    #[test]
    fn test_health_check() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        assert!(storage.health_check().unwrap());
    }

    #[test]
    fn test_metadata() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        storage.save_script(&make_script("s1")).unwrap();
        storage.save_script(&make_script("s2")).unwrap();
        let meta = storage.get_metadata().unwrap();
        assert_eq!(meta.total_scripts, 2);
        assert_eq!(meta.backend_type, "local");
    }
}
