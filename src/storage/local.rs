use super::{StorageBackend, StorageMetadata};
use crate::script::{Script, SyncState, SyncStatus};
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};

pub struct LocalStorage {
    vault_path: PathBuf,
    index_path: PathBuf,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Index {
    entries: std::collections::HashMap<String, String>,
}

impl Index {
    fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path).context("failed to read index")?;
        serde_json::from_str(&raw).context("failed to parse index")
    }

    fn save(&self, path: &Path) -> Result<()> {
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        fs::rename(&tmp, path).context("failed to replace index")
    }
}

impl LocalStorage {
    pub fn new(vault_path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&vault_path).context("failed to create vault directory")?;
        let index_path = vault_path.join("index.json");
        Ok(Self {
            vault_path,
            index_path,
        })
    }

    fn script_path(&self, id: &str) -> PathBuf {
        self.vault_path.join(format!("{}.json", id))
    }

    fn read_script(&self, id: &str) -> Result<Script> {
        let path = self.script_path(id);
        let raw =
            fs::read_to_string(&path).with_context(|| format!("script file not found: {}", id))?;
        serde_json::from_str(&raw).context("failed to parse script file")
    }

    fn write_script(&self, script: &Script) -> Result<()> {
        let path = self.script_path(&script.id);
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, serde_json::to_string_pretty(script)?)?;
        fs::rename(&tmp, &path).context("failed to atomically write script")
    }

    fn index_add(&self, name: &str, id: &str) -> Result<()> {
        let mut idx = Index::load(&self.index_path)?;
        idx.entries.insert(name.to_string(), id.to_string());
        idx.save(&self.index_path)
    }

    fn index_remove_by_id(&self, id: &str) -> Result<()> {
        let mut idx = Index::load(&self.index_path)?;
        idx.entries.retain(|_, v| v != id);
        idx.save(&self.index_path)
    }

    fn id_for_name(&self, name: &str) -> Result<String> {
        let idx = Index::load(&self.index_path)?;
        idx.entries
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("script not found: {}", name))
    }

    fn mutate(&self, id: &str, f: impl FnOnce(&mut Script)) -> Result<()> {
        let mut script = self.read_script(id)?;
        f(&mut script);
        self.write_script(&script)
    }
}

impl StorageBackend for LocalStorage {
    fn save_script(&self, script: &Script) -> Result<()> {
        if let Ok(existing_id) = self.id_for_name(&script.name) {
            if existing_id != script.id {
                let old_path = self.script_path(&existing_id);
                let _ = fs::remove_file(old_path);
            }
        }
        self.write_script(script)?;
        self.index_add(&script.name, &script.id)
    }

    fn update_script(&self, script: &Script) -> Result<()> {
        if !self.script_path(&script.id).exists() {
            return Err(anyhow!("script not found: {}", script.id));
        }
        let old = self.read_script(&script.id)?;
        if old.name != script.name {
            let mut idx = Index::load(&self.index_path)?;
            idx.entries.remove(&old.name);
            idx.entries.insert(script.name.clone(), script.id.clone());
            idx.save(&self.index_path)?;
        }
        self.write_script(script)
    }

    fn load_script(&self, id: &str) -> Result<Script> {
        self.read_script(id)
    }

    fn load_script_by_name(&self, name: &str) -> Result<Script> {
        let id = self.id_for_name(name)?;
        self.read_script(&id)
    }

    fn list_scripts(&self) -> Result<Vec<Script>> {
        let idx = Index::load(&self.index_path)?;
        let mut scripts = Vec::with_capacity(idx.entries.len());
        for id in idx.entries.values() {
            match self.read_script(id) {
                Ok(s) => scripts.push(s),
                Err(_) => {}
            }
        }
        Ok(scripts)
    }

    fn delete_script(&self, id: &str) -> Result<()> {
        let path = self.script_path(id);
        if !path.exists() {
            return Err(anyhow!("script not found: {}", id));
        }
        fs::remove_file(&path).context("failed to delete script file")?;
        self.index_remove_by_id(id)
    }

    fn script_exists(&self, id: &str) -> Result<bool> {
        Ok(self.script_path(id).exists())
    }

    fn get_metadata(&self) -> Result<StorageMetadata> {
        let scripts = self.list_scripts()?;
        let total_size = scripts.iter().map(|s| s.metadata.size_bytes as u64).sum();
        Ok(StorageMetadata {
            total_scripts: scripts.len(),
            total_size_bytes: total_size,
            last_sync: None,
            backend_type: self.backend_type().to_string(),
        })
    }

    fn health_check(&self) -> Result<bool> {
        Ok(self.vault_path.exists())
    }

    fn get_sync_status(&self, script_id: &str) -> Result<SyncStatus> {
        Ok(self.read_script(script_id)?.sync_state.status)
    }

    fn mark_synced(
        &self,
        script_id: &str,
        remote_version: &str,
        synced_at: DateTime<Utc>,
    ) -> Result<()> {
        self.mutate(script_id, |s| {
            let hash = s.metadata.hash.clone();
            s.sync_state = SyncState {
                status: SyncStatus::Synced,
                last_synced_at: Some(synced_at),
                remote_version: Some(remote_version.to_string()),
                conflict_base_hash: Some(hash),
            };
        })
    }

    fn mark_conflict(&self, script_id: &str) -> Result<()> {
        self.mutate(script_id, |s| {
            s.sync_state.status = SyncStatus::Conflict;
        })
    }

    fn list_pending_push(&self) -> Result<Vec<Script>> {
        Ok(self
            .list_scripts()?
            .into_iter()
            .filter(|s| s.sync_state.status == SyncStatus::PendingPush)
            .collect())
    }

    fn list_conflicts(&self) -> Result<Vec<Script>> {
        Ok(self
            .list_scripts()?
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
                hash: "testhash".to_string(),
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

    fn storage(tmp: &TempDir) -> LocalStorage {
        LocalStorage::new(tmp.path().to_path_buf()).unwrap()
    }

    #[test]
    fn test_save_and_load_by_name() {
        let tmp = TempDir::new().unwrap();
        let s = storage(&tmp);
        s.save_script(&make_script("deploy")).unwrap();
        assert_eq!(s.load_script_by_name("deploy").unwrap().name, "deploy");
    }

    #[test]
    fn test_save_and_load_by_id() {
        let tmp = TempDir::new().unwrap();
        let s = storage(&tmp);
        let script = make_script("backup");
        let id = script.id.clone();
        s.save_script(&script).unwrap();
        assert_eq!(s.load_script(&id).unwrap().id, id);
    }

    #[test]
    fn test_save_deduplicates_by_name() {
        let tmp = TempDir::new().unwrap();
        let s = storage(&tmp);
        let a = make_script("same");
        s.save_script(&a).unwrap();
        let b = make_script("same");
        s.save_script(&b).unwrap();
        assert_eq!(s.list_scripts().unwrap().len(), 1);
        assert_eq!(s.list_scripts().unwrap()[0].id, b.id);
        assert!(!s.script_path(&a.id).exists());
    }

    #[test]
    fn test_update_modifies_in_place() {
        let tmp = TempDir::new().unwrap();
        let s = storage(&tmp);
        let mut script = make_script("update-test");
        s.save_script(&script).unwrap();
        script.content = "echo updated".to_string();
        s.update_script(&script).unwrap();
        assert_eq!(
            s.load_script_by_name("update-test").unwrap().content,
            "echo updated"
        );
    }

    #[test]
    fn test_update_unknown_id_errors() {
        let tmp = TempDir::new().unwrap();
        let s = storage(&tmp);
        assert!(s.update_script(&make_script("ghost")).is_err());
    }

    #[test]
    fn test_rename_via_update_fixes_index() {
        let tmp = TempDir::new().unwrap();
        let s = storage(&tmp);
        let mut script = make_script("old");
        s.save_script(&script).unwrap();
        script.name = "new".to_string();
        s.update_script(&script).unwrap();
        assert!(s.load_script_by_name("old").is_err());
        assert!(s.load_script_by_name("new").is_ok());
    }

    #[test]
    fn test_delete_removes_file_and_index() {
        let tmp = TempDir::new().unwrap();
        let s = storage(&tmp);
        let script = make_script("del");
        let id = script.id.clone();
        s.save_script(&script).unwrap();
        s.delete_script(&id).unwrap();
        assert!(!s.script_path(&id).exists());
        assert!(s.load_script_by_name("del").is_err());
    }

    #[test]
    fn test_list_scripts() {
        let tmp = TempDir::new().unwrap();
        let s = storage(&tmp);
        s.save_script(&make_script("a")).unwrap();
        s.save_script(&make_script("b")).unwrap();
        s.save_script(&make_script("c")).unwrap();
        assert_eq!(s.list_scripts().unwrap().len(), 3);
    }

    #[test]
    fn test_mark_synced_and_conflict() {
        let tmp = TempDir::new().unwrap();
        let s = storage(&tmp);
        let script = make_script("sync");
        let id = script.id.clone();
        s.save_script(&script).unwrap();
        s.mark_synced(&id, "v1.0.0", Utc::now()).unwrap();
        assert_eq!(s.get_sync_status(&id).unwrap(), SyncStatus::Synced);
        s.mark_conflict(&id).unwrap();
        assert_eq!(s.get_sync_status(&id).unwrap(), SyncStatus::Conflict);
    }

    #[test]
    fn test_health_check() {
        let tmp = TempDir::new().unwrap();
        assert!(storage(&tmp).health_check().unwrap());
    }
}
