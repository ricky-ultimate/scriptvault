use crate::script::{Script, SyncState, SyncStatus};
use crate::storage::StorageBackend;
use crate::sync::remote::{RemoteBackend, RemoteScriptMeta};
use anyhow::{Result, anyhow};
use chrono::Utc;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub enum ConflictResolution {
    TakeLocal,
    TakeRemote,
}

#[derive(Debug, Default)]
pub struct SyncReport {
    pub pushed: Vec<String>,
    pub pulled: Vec<String>,
    pub conflicts: Vec<String>,
    pub errors: Vec<(String, String)>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ScriptSyncStatus {
    pub name: String,
    pub version: String,
    pub status: SyncStatus,
    pub last_synced_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct SyncManager {
    local: Box<dyn StorageBackend>,
    remote: Box<dyn RemoteBackend>,
}

impl SyncManager {
    pub fn new(local: Box<dyn StorageBackend>, remote: Box<dyn RemoteBackend>) -> Self {
        Self { local, remote }
    }

    pub fn compute_status(
        &self,
        local: &Script,
        remote_meta: Option<&RemoteScriptMeta>,
    ) -> SyncStatus {
        let Some(meta) = remote_meta else {
            return SyncStatus::LocalOnly;
        };

        let Some(last_sync) = local.sync_state.last_synced_at else {
            if local.metadata.hash == meta.hash {
                return SyncStatus::Synced;
            } else {
                return SyncStatus::Conflict;
            }
        };

        let local_changed = local.updated_at > last_sync;
        let remote_changed = meta.updated_at > last_sync;

        match (local_changed, remote_changed) {
            (true, true) => SyncStatus::Conflict,
            (true, false) => SyncStatus::PendingPush,
            (false, true) => SyncStatus::PendingPull,
            (false, false) => SyncStatus::Synced,
        }
    }

    pub fn full_sync(&self) -> Result<SyncReport> {
        let mut report = SyncReport::default();

        let local_scripts = self.local.list_scripts()?;
        let remote_metas = self.remote.list_scripts()?;

        let remote_by_id: HashMap<String, &RemoteScriptMeta> =
            remote_metas.iter().map(|m| (m.id.clone(), m)).collect();

        let remote_by_name: HashMap<String, &RemoteScriptMeta> =
            remote_metas.iter().map(|m| (m.name.clone(), m)).collect();

        let local_ids: HashSet<String> = local_scripts.iter().map(|s| s.id.clone()).collect();
        let local_names: HashSet<String> = local_scripts.iter().map(|s| s.name.clone()).collect();

        for script in &local_scripts {
            let remote_meta = remote_by_id
                .get(&script.id)
                .copied()
                .or_else(|| remote_by_name.get(&script.name).copied());

            let status = self.compute_status(script, remote_meta);

            match status {
                SyncStatus::PendingPush | SyncStatus::LocalOnly => match self.do_push(script) {
                    Ok(_) => report.pushed.push(script.name.clone()),
                    Err(e) => report.errors.push((script.name.clone(), e.to_string())),
                },
                SyncStatus::PendingPull => {
                    if let Some(meta) = remote_meta {
                        match self.do_pull(&meta.id) {
                            Ok(_) => report.pulled.push(script.name.clone()),
                            Err(e) => report.errors.push((script.name.clone(), e.to_string())),
                        }
                    }
                }
                SyncStatus::Conflict => {
                    if let Err(e) = self.local.mark_conflict(&script.id) {
                        report.errors.push((script.name.clone(), e.to_string()));
                    } else {
                        report.conflicts.push(script.name.clone());
                    }
                }
                SyncStatus::Synced => {}
                SyncStatus::RemoteOnly => {}
            }
        }

        for meta in &remote_metas {
            if !local_ids.contains(&meta.id) && !local_names.contains(&meta.name) {
                match self.do_pull(&meta.id) {
                    Ok(_) => report.pulled.push(meta.name.clone()),
                    Err(e) => report.errors.push((meta.name.clone(), e.to_string())),
                }
            }
        }

        Ok(report)
    }

    pub fn push_pending(&self) -> Result<SyncReport> {
        let mut report = SyncReport::default();
        let pending = self.local.list_pending_push()?;

        for script in &pending {
            match self.do_push(script) {
                Ok(_) => report.pushed.push(script.name.clone()),
                Err(e) => report.errors.push((script.name.clone(), e.to_string())),
            }
        }

        Ok(report)
    }

    pub fn resolve_conflict(
        &self,
        script_name: &str,
        resolution: ConflictResolution,
    ) -> Result<()> {
        let script = self.local.load_script_by_name(script_name)?;

        if script.sync_state.status != SyncStatus::Conflict {
            return Err(anyhow!(
                "Script '{}' is not in a conflict state (current status: {})",
                script_name,
                script.sync_state.status
            ));
        }

        match resolution {
            ConflictResolution::TakeLocal => {
                self.do_push(&script)?;
            }
            ConflictResolution::TakeRemote => {
                let remote_id = script
                    .sync_state
                    .remote_version
                    .as_deref()
                    .unwrap_or(&script.id);
                self.do_pull(remote_id)?;
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn show_status(&self) -> Result<Vec<ScriptSyncStatus>> {
        let scripts = self.local.list_scripts()?;
        Ok(scripts
            .into_iter()
            .map(|s| ScriptSyncStatus {
                name: s.name.clone(),
                version: s.version.clone(),
                status: s.sync_state.status.clone(),
                last_synced_at: s.sync_state.last_synced_at,
            })
            .collect())
    }

    fn do_push(&self, script: &Script) -> Result<()> {
        let remote_meta = self.remote.push_script(script)?;
        self.local
            .mark_synced(&script.id, &remote_meta.version, Utc::now())?;
        Ok(())
    }

    fn do_pull(&self, remote_id: &str) -> Result<()> {
        let mut remote_script = self.remote.fetch_script(remote_id)?;
        let now = Utc::now();
        let hash = remote_script.metadata.hash.clone();
        let version = remote_script.version.clone();
        remote_script.sync_state = SyncState {
            status: SyncStatus::Synced,
            last_synced_at: Some(now),
            remote_version: Some(version),
            conflict_base_hash: Some(hash),
        };

        if self.local.script_exists(&remote_script.id)? {
            self.local.update_script(&remote_script)?;
        } else {
            self.local.save_script(&remote_script)?;
        }

        Ok(())
    }

    pub fn remote_list(&self) -> Result<Vec<crate::sync::remote::RemoteScriptMeta>> {
        self.remote.list_scripts()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::{ScriptContext, ScriptLanguage, ScriptMetadata, Visibility};
    use crate::storage::local::LocalStorage;
    use crate::sync::remote::RemoteScriptMeta;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tempfile::TempDir;

    struct MockRemoteBackend {
        scripts: Mutex<Vec<Script>>,
    }

    impl MockRemoteBackend {
        fn new() -> Self {
            Self {
                scripts: Mutex::new(vec![]),
            }
        }

        fn seed(&self, script: Script) {
            self.scripts.lock().unwrap().push(script);
        }
    }

    impl RemoteBackend for MockRemoteBackend {
        fn test_connection(&self) -> Result<()> {
            Ok(())
        }

        fn list_scripts(&self) -> Result<Vec<RemoteScriptMeta>> {
            Ok(self
                .scripts
                .lock()
                .unwrap()
                .iter()
                .map(|s| RemoteScriptMeta {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    version: s.version.clone(),
                    updated_at: s.updated_at,
                    hash: s.metadata.hash.clone(),
                })
                .collect())
        }

        fn fetch_script(&self, id: &str) -> Result<Script> {
            self.scripts
                .lock()
                .unwrap()
                .iter()
                .find(|s| s.id == id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Remote script not found: {}", id))
        }

        fn push_script(&self, script: &Script) -> Result<RemoteScriptMeta> {
            let mut scripts = self.scripts.lock().unwrap();
            scripts.retain(|s| s.id != script.id && s.name != script.name);
            scripts.push(script.clone());
            Ok(RemoteScriptMeta {
                id: script.id.clone(),
                name: script.name.clone(),
                version: script.version.clone(),
                updated_at: script.updated_at,
                hash: script.metadata.hash.clone(),
            })
        }

        fn delete_script(&self, id: &str) -> Result<()> {
            self.scripts.lock().unwrap().retain(|s| s.id != id);
            Ok(())
        }
    }

    fn make_script(name: &str, content: &str) -> Script {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hex::encode(hasher.finalize());

        Script {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            content: content.to_string(),
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
                hash,
                size_bytes: content.len(),
                line_count: content.lines().count(),
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

    fn make_manager(tmp: &TempDir) -> (SyncManager, std::sync::Arc<MockRemoteBackend>) {
        use std::sync::Arc;
        let local = LocalStorage::new(tmp.path().to_path_buf()).unwrap();
        let remote = Arc::new(MockRemoteBackend::new());
        let remote_clone = Arc::clone(&remote);

        struct ArcRemote(Arc<MockRemoteBackend>);
        impl RemoteBackend for ArcRemote {
            fn test_connection(&self) -> Result<()> {
                self.0.test_connection()
            }
            fn list_scripts(&self) -> Result<Vec<RemoteScriptMeta>> {
                self.0.list_scripts()
            }
            fn fetch_script(&self, id: &str) -> Result<Script> {
                self.0.fetch_script(id)
            }
            fn push_script(&self, script: &Script) -> Result<RemoteScriptMeta> {
                self.0.push_script(script)
            }
            fn delete_script(&self, id: &str) -> Result<()> {
                self.0.delete_script(id)
            }
        }

        let manager = SyncManager::new(Box::new(local), Box::new(ArcRemote(remote_clone)));
        (manager, remote)
    }

    #[test]
    fn test_compute_status_local_only_no_remote() {
        let tmp = TempDir::new().unwrap();
        let (manager, _remote) = make_manager(&tmp);
        let script = make_script("deploy", "echo deploy");
        assert_eq!(manager.compute_status(&script, None), SyncStatus::LocalOnly);
    }

    #[test]
    fn test_compute_status_synced_matching_hash() {
        let tmp = TempDir::new().unwrap();
        let (manager, _remote) = make_manager(&tmp);
        let script = make_script("deploy", "echo deploy");
        let meta = RemoteScriptMeta {
            id: script.id.clone(),
            name: script.name.clone(),
            version: script.version.clone(),
            updated_at: script.updated_at,
            hash: script.metadata.hash.clone(),
        };
        assert_eq!(
            manager.compute_status(&script, Some(&meta)),
            SyncStatus::Synced
        );
    }

    #[test]
    fn test_compute_status_conflict_never_synced_different_hash() {
        let tmp = TempDir::new().unwrap();
        let (manager, _remote) = make_manager(&tmp);
        let script = make_script("deploy", "echo local version");
        let meta = RemoteScriptMeta {
            id: script.id.clone(),
            name: script.name.clone(),
            version: "v1.0.1".to_string(),
            updated_at: script.updated_at,
            hash: "completely_different_hash".to_string(),
        };
        assert_eq!(
            manager.compute_status(&script, Some(&meta)),
            SyncStatus::Conflict
        );
    }

    #[test]
    fn test_compute_status_pending_push_only_local_changed() {
        let tmp = TempDir::new().unwrap();
        let (manager, _remote) = make_manager(&tmp);
        let last_sync = Utc::now() - chrono::Duration::hours(1);
        let mut script = make_script("deploy", "echo deploy");
        script.sync_state.last_synced_at = Some(last_sync);
        script.updated_at = Utc::now();
        let meta = RemoteScriptMeta {
            id: script.id.clone(),
            name: script.name.clone(),
            version: script.version.clone(),
            updated_at: last_sync - chrono::Duration::minutes(1),
            hash: script.metadata.hash.clone(),
        };
        assert_eq!(
            manager.compute_status(&script, Some(&meta)),
            SyncStatus::PendingPush
        );
    }

    #[test]
    fn test_compute_status_pending_pull_only_remote_changed() {
        let tmp = TempDir::new().unwrap();
        let (manager, _remote) = make_manager(&tmp);
        let last_sync = Utc::now() - chrono::Duration::hours(1);
        let mut script = make_script("deploy", "echo deploy");
        script.sync_state.last_synced_at = Some(last_sync);
        script.updated_at = last_sync - chrono::Duration::minutes(1);
        let meta = RemoteScriptMeta {
            id: script.id.clone(),
            name: script.name.clone(),
            version: "v1.0.1".to_string(),
            updated_at: Utc::now(),
            hash: "newer_hash".to_string(),
        };
        assert_eq!(
            manager.compute_status(&script, Some(&meta)),
            SyncStatus::PendingPull
        );
    }

    #[test]
    fn test_compute_status_conflict_both_changed() {
        let tmp = TempDir::new().unwrap();
        let (manager, _remote) = make_manager(&tmp);
        let last_sync = Utc::now() - chrono::Duration::hours(1);
        let mut script = make_script("deploy", "echo deploy");
        script.sync_state.last_synced_at = Some(last_sync);
        script.updated_at = Utc::now();
        let meta = RemoteScriptMeta {
            id: script.id.clone(),
            name: script.name.clone(),
            version: "v1.0.1".to_string(),
            updated_at: Utc::now(),
            hash: "different_hash".to_string(),
        };
        assert_eq!(
            manager.compute_status(&script, Some(&meta)),
            SyncStatus::Conflict
        );
    }

    #[test]
    fn test_compute_status_synced_neither_changed() {
        let tmp = TempDir::new().unwrap();
        let (manager, _remote) = make_manager(&tmp);
        let last_sync = Utc::now();
        let mut script = make_script("deploy", "echo deploy");
        script.sync_state.last_synced_at = Some(last_sync);
        script.updated_at = last_sync - chrono::Duration::seconds(10);
        let meta = RemoteScriptMeta {
            id: script.id.clone(),
            name: script.name.clone(),
            version: script.version.clone(),
            updated_at: last_sync - chrono::Duration::seconds(10),
            hash: script.metadata.hash.clone(),
        };
        assert_eq!(
            manager.compute_status(&script, Some(&meta)),
            SyncStatus::Synced
        );
    }

    #[test]
    fn test_full_sync_pushes_local_only_scripts() {
        let tmp = TempDir::new().unwrap();
        let (manager, remote) = make_manager(&tmp);
        let script = make_script("deploy", "echo deploy");
        manager.local.save_script(&script).unwrap();

        let report = manager.full_sync().unwrap();
        assert_eq!(report.pushed.len(), 1);
        assert_eq!(report.pushed[0], "deploy");
        assert!(report.conflicts.is_empty());

        assert_eq!(
            manager.local.get_sync_status(&script.id).unwrap(),
            SyncStatus::Synced
        );
        assert_eq!(remote.list_scripts().unwrap().len(), 1);
    }

    #[test]
    fn test_full_sync_pulls_remote_only_scripts() {
        let tmp = TempDir::new().unwrap();
        let (manager, remote) = make_manager(&tmp);
        let remote_script = make_script("remote-only", "echo remote");
        remote.seed(remote_script.clone());

        let report = manager.full_sync().unwrap();
        assert_eq!(report.pulled.len(), 1);
        assert_eq!(report.pulled[0], "remote-only");

        let loaded = manager.local.load_script_by_name("remote-only").unwrap();
        assert_eq!(loaded.sync_state.status, SyncStatus::Synced);
    }

    #[test]
    fn test_full_sync_flags_conflicts() {
        let tmp = TempDir::new().unwrap();
        let (manager, remote) = make_manager(&tmp);

        let local_script = make_script("conflict-script", "echo local");
        manager.local.save_script(&local_script).unwrap();

        let mut remote_script = local_script.clone();
        remote_script.metadata.hash = "different_hash".to_string();
        remote.seed(remote_script);

        let report = manager.full_sync().unwrap();
        assert_eq!(report.conflicts.len(), 1);

        assert_eq!(
            manager.local.get_sync_status(&local_script.id).unwrap(),
            SyncStatus::Conflict
        );
    }

    #[test]
    fn test_resolve_conflict_take_local() {
        let tmp = TempDir::new().unwrap();
        let (manager, remote) = make_manager(&tmp);

        let mut script = make_script("conflict-script", "echo local");
        script.sync_state.status = SyncStatus::Conflict;
        manager.local.save_script(&script).unwrap();

        manager
            .resolve_conflict("conflict-script", ConflictResolution::TakeLocal)
            .unwrap();

        assert_eq!(
            manager.local.get_sync_status(&script.id).unwrap(),
            SyncStatus::Synced
        );
        assert_eq!(remote.list_scripts().unwrap().len(), 1);
    }

    #[test]
    fn test_resolve_conflict_errors_on_non_conflict_script() {
        let tmp = TempDir::new().unwrap();
        let (manager, _remote) = make_manager(&tmp);
        let script = make_script("clean", "echo clean");
        manager.local.save_script(&script).unwrap();

        let result = manager.resolve_conflict("clean", ConflictResolution::TakeLocal);
        assert!(result.is_err());
    }

    #[test]
    fn test_show_status_reflects_sync_state() {
        let tmp = TempDir::new().unwrap();
        let (manager, _remote) = make_manager(&tmp);

        let script = make_script("check", "echo check");
        manager.local.save_script(&script).unwrap();

        let statuses = manager.show_status().unwrap();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].name, "check");
        assert_eq!(statuses[0].status, SyncStatus::LocalOnly);
    }
}
