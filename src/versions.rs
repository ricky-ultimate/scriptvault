use crate::script::Script;
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_VERSIONS: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    pub version: String,
    pub saved_at: DateTime<Utc>,
    pub author: String,
    pub hash: String,
    pub size_bytes: usize,
    pub line_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct VersionManifest {
    entries: Vec<VersionEntry>,
}

pub struct VersionStore {
    base: PathBuf,
}

impl VersionStore {
    pub fn new(vault_path: &Path) -> Self {
        Self {
            base: vault_path.join("history"),
        }
    }

    fn script_dir(&self, script_id: &str) -> PathBuf {
        self.base.join(script_id)
    }

    fn manifest_path(&self, script_id: &str) -> PathBuf {
        self.script_dir(script_id).join("manifest.json")
    }

    fn snapshot_path(&self, script_id: &str, version: &str) -> PathBuf {
        self.script_dir(script_id)
            .join(format!("{}.json", sanitize_version(version)))
    }

    fn load_manifest(&self, script_id: &str) -> Result<VersionManifest> {
        let path = self.manifest_path(script_id);
        if !path.exists() {
            return Ok(VersionManifest::default());
        }
        let raw = fs::read_to_string(&path).context("failed to read version manifest")?;
        serde_json::from_str(&raw).context("failed to parse version manifest")
    }

    fn save_manifest(&self, script_id: &str, manifest: &VersionManifest) -> Result<()> {
        let path = self.manifest_path(script_id);
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, serde_json::to_string_pretty(manifest)?)?;
        fs::rename(&tmp, &path).context("failed to write manifest")
    }

    pub fn save_version(&self, script: &Script) -> Result<()> {
        let dir = self.script_dir(&script.id);
        fs::create_dir_all(&dir).context("failed to create history directory")?;

        let snapshot_path = self.snapshot_path(&script.id, &script.version);
        if snapshot_path.exists() {
            return Ok(());
        }

        let tmp = snapshot_path.with_extension("tmp");
        fs::write(&tmp, serde_json::to_string_pretty(script)?)?;
        fs::rename(&tmp, &snapshot_path).context("failed to write snapshot")?;

        let mut manifest = self.load_manifest(&script.id)?;
        manifest.entries.push(VersionEntry {
            version: script.version.clone(),
            saved_at: script.updated_at,
            author: script.author.clone(),
            hash: script.metadata.hash.clone(),
            size_bytes: script.metadata.size_bytes,
            line_count: script.metadata.line_count,
        });

        if manifest.entries.len() > MAX_VERSIONS {
            let to_remove = manifest.entries.remove(0);
            let _ = fs::remove_file(self.snapshot_path(&script.id, &to_remove.version));
        }

        self.save_manifest(&script.id, &manifest)
    }

    pub fn list_versions(&self, script_id: &str) -> Result<Vec<VersionEntry>> {
        Ok(self.load_manifest(script_id)?.entries)
    }

    pub fn load_version(&self, script_id: &str, version: &str) -> Result<Script> {
        let path = self.snapshot_path(script_id, version);
        if !path.exists() {
            return Err(anyhow!(
                "version {} not found for script {}",
                version,
                script_id
            ));
        }
        let raw = fs::read_to_string(&path).context("failed to read snapshot")?;
        serde_json::from_str(&raw).context("failed to parse snapshot")
    }

    pub fn diff_versions(&self, script_id: &str, v1: &str, v2: &str) -> Result<(Script, Script)> {
        let a = self.load_version(script_id, v1)?;
        let b = self.load_version(script_id, v2)?;
        Ok((a, b))
    }

    pub fn purge_script(&self, script_id: &str) -> Result<()> {
        let dir = self.script_dir(script_id);
        if dir.exists() {
            fs::remove_dir_all(&dir).context("failed to purge script history")?;
        }
        Ok(())
    }
}

fn sanitize_version(v: &str) -> String {
    v.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::{
        Script, ScriptContext, ScriptLanguage, ScriptMetadata, SyncState, Visibility,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn make_script(name: &str, version: &str) -> Script {
        Script {
            id: "test-id".to_string(),
            name: name.to_string(),
            content: format!("echo {}", version),
            version: version.to_string(),
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
                hash: format!("hash-{}", version),
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
    fn test_save_and_list_versions() {
        let tmp = TempDir::new().unwrap();
        let store = VersionStore::new(tmp.path());
        store
            .save_version(&make_script("deploy", "v1.0.0"))
            .unwrap();
        store
            .save_version(&make_script("deploy", "v1.0.1"))
            .unwrap();
        let versions = store.list_versions("test-id").unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version, "v1.0.0");
        assert_eq!(versions[1].version, "v1.0.1");
    }

    #[test]
    fn test_load_specific_version() {
        let tmp = TempDir::new().unwrap();
        let store = VersionStore::new(tmp.path());
        store
            .save_version(&make_script("deploy", "v1.0.0"))
            .unwrap();
        store
            .save_version(&make_script("deploy", "v1.0.1"))
            .unwrap();
        let v = store.load_version("test-id", "v1.0.0").unwrap();
        assert_eq!(v.content, "echo v1.0.0");
    }

    #[test]
    fn test_diff_returns_both_versions() {
        let tmp = TempDir::new().unwrap();
        let store = VersionStore::new(tmp.path());
        store
            .save_version(&make_script("deploy", "v1.0.0"))
            .unwrap();
        store
            .save_version(&make_script("deploy", "v1.0.1"))
            .unwrap();
        let (a, b) = store.diff_versions("test-id", "v1.0.0", "v1.0.1").unwrap();
        assert_eq!(a.version, "v1.0.0");
        assert_eq!(b.version, "v1.0.1");
    }

    #[test]
    fn test_duplicate_version_not_saved_twice() {
        let tmp = TempDir::new().unwrap();
        let store = VersionStore::new(tmp.path());
        store
            .save_version(&make_script("deploy", "v1.0.0"))
            .unwrap();
        store
            .save_version(&make_script("deploy", "v1.0.0"))
            .unwrap();
        assert_eq!(store.list_versions("test-id").unwrap().len(), 1);
    }

    #[test]
    fn test_purge_removes_history() {
        let tmp = TempDir::new().unwrap();
        let store = VersionStore::new(tmp.path());
        store
            .save_version(&make_script("deploy", "v1.0.0"))
            .unwrap();
        store.purge_script("test-id").unwrap();
        assert_eq!(store.list_versions("test-id").unwrap().len(), 0);
    }
}
