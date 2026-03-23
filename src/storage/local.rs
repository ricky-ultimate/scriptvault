use super::{StorageBackend, StorageMetadata};
use crate::script::{Script, SyncStatus, SyncState};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
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
        let temp_path = self.scripts_file.with_extension("tmp");
        fs::write(&temp_path, &json).context("Failed to write temporary scripts file")?;
        fs::rename(&temp_path, &self.scripts_file)
            .context("Failed to atomically replace scripts file")?;
        Ok(())
    }

    fn total_size(scripts: &[Script]) -> u64 {
        scripts.iter().map(|s| s.metadata.size_bytes as u64).sum()
    }

    fn mutate_script<F>(&self, script_id: &str, f: F) -> Result<()>
    where
        F: FnOnce(&mut Script),
    {
        let mut scripts = self.load_all()?;
        let pos = scripts
            .iter()
            .position(|s| s.id == script_id)
            .ok_or_else(|| anyhow::anyhow!("Script not found: {}", script_id))?;
        f(&mut scripts[pos]);
        self.persist(&scripts)
    }
}

impl StorageBackend for LocalStorage {
    fn save_script(&self, script: &Script) -> Result<()> {
        let mut scripts = self.load_all()?;
        scripts.retain(|s| s.name != script.name);
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

    fn get_sync_status(&self, script_id: &str) -> Result<SyncStatus> {
        let script = self.load_script(script_id)?;
        Ok(script.sync_state.status.clone())
    }

    fn mark_synced(
        &self,
        script_id: &str,
        remote_version: &str,
        synced_at: DateTime<Utc>,
    ) -> Result<()> {
        self.mutate_script(script_id, |script| {
            let hash = script.metadata.hash.clone();
            script.sync_state = SyncState {
                status: SyncStatus::Synced,
                last_synced_at: Some(synced_at),
                remote_version: Some(remote_version.to_string()),
                conflict_base_hash: Some(hash),
            };
        })
    }

    fn mark_conflict(&self, script_id: &str) -> Result<()> {
        self.mutate_script(script_id, |script| {
            script.sync_state.status = SyncStatus::Conflict;
        })
    }

    fn list_pending_push(&self) -> Result<Vec<Script>> {
        Ok(self
            .load_all()?
            .into_iter()
            .filter(|s| s.sync_state.status == SyncStatus::PendingPush)
            .collect())
    }

    fn list_conflicts(&self) -> Result<Vec<Script>> {
        Ok(self
            .load_all()?
            .into_iter()
            .filter(|s| s.sync_state.status == SyncStatus::Conflict)
            .collect())
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
            sync_state: SyncState::default(),
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
    fn test_save_deduplicates_by_name_only() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let original = make_script("same-name");
        storage.save_script(&original).unwrap();
        let mut diverged = make_script("same-name");
        diverged.id = uuid::Uuid::new_v4().to_string();
        storage.save_script(&diverged).unwrap();
        let all = storage.list_scripts().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, diverged.id);
    }

    #[test]
    fn test_save_different_names_kept_separate() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        storage.save_script(&make_script("a")).unwrap();
        storage.save_script(&make_script("b")).unwrap();
        assert_eq!(storage.list_scripts().unwrap().len(), 2);
    }

    #[test]
    fn test_update_modifies_in_place() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let mut script = make_script("update-test");
        storage.save_script(&script).unwrap();
        script.content = "echo 'updated'".to_string();
        storage.update_script(&script).unwrap();
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

    #[test]
    fn test_default_sync_status_is_local_only() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let script = make_script("sync-test");
        let id = script.id.clone();
        storage.save_script(&script).unwrap();
        assert_eq!(
            storage.get_sync_status(&id).unwrap(),
            SyncStatus::LocalOnly
        );
    }

    #[test]
    fn test_mark_synced() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let script = make_script("sync-test");
        let id = script.id.clone();
        storage.save_script(&script).unwrap();
        let now = Utc::now();
        storage.mark_synced(&id, "v1.0.0", now).unwrap();
        let loaded = storage.load_script(&id).unwrap();
        assert_eq!(loaded.sync_state.status, SyncStatus::Synced);
        assert_eq!(loaded.sync_state.remote_version, Some("v1.0.0".to_string()));
        assert!(loaded.sync_state.last_synced_at.is_some());
        assert!(loaded.sync_state.conflict_base_hash.is_some());
    }

    #[test]
    fn test_mark_conflict() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let script = make_script("conflict-test");
        let id = script.id.clone();
        storage.save_script(&script).unwrap();
        storage.mark_conflict(&id).unwrap();
        assert_eq!(
            storage.get_sync_status(&id).unwrap(),
            SyncStatus::Conflict
        );
    }

    #[test]
    fn test_list_pending_push_empty_when_none() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        storage.save_script(&make_script("a")).unwrap();
        storage.save_script(&make_script("b")).unwrap();
        assert!(storage.list_pending_push().unwrap().is_empty());
    }

    #[test]
    fn test_list_pending_push_returns_correct_scripts() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let mut pending = make_script("pending");
        pending.sync_state.status = SyncStatus::PendingPush;
        storage.save_script(&pending).unwrap();
        storage.save_script(&make_script("local-only")).unwrap();
        let results = storage.list_pending_push().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "pending");
    }

    #[test]
    fn test_list_conflicts() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let script = make_script("conflicted");
        let id = script.id.clone();
        storage.save_script(&script).unwrap();
        storage.mark_conflict(&id).unwrap();
        storage.save_script(&make_script("clean")).unwrap();
        let conflicts = storage.list_conflicts().unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].name, "conflicted");
    }

    #[test]
    fn test_atomic_persist_produces_valid_json() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        storage.save_script(&make_script("a")).unwrap();
        storage.save_script(&make_script("b")).unwrap();
        let contents = std::fs::read_to_string(&storage.scripts_file).unwrap();
        let parsed: Vec<Script> = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed.len(), 2);
        assert!(!storage.scripts_file.with_extension("tmp").exists());
    }

    #[test]
    fn test_sync_state_persists_across_load() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let script = make_script("persist-test");
        let id = script.id.clone();
        storage.save_script(&script).unwrap();
        let now = Utc::now();
        storage.mark_synced(&id, "v2.0.0", now).unwrap();

        let storage2 = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let loaded = storage2.load_script(&id).unwrap();
        assert_eq!(loaded.sync_state.status, SyncStatus::Synced);
        assert_eq!(loaded.sync_state.remote_version, Some("v2.0.0".to_string()));
    }
}
