use crate::script::Script;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteScriptMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub updated_at: DateTime<Utc>,
    pub hash: String,
}

#[allow(dead_code)]
pub trait RemoteBackend: Send + Sync {
    fn test_connection(&self) -> Result<()>;
    fn list_scripts(&self) -> Result<Vec<RemoteScriptMeta>>;
    fn fetch_script(&self, id: &str) -> Result<Script>;
    fn push_script(&self, script: &Script) -> Result<RemoteScriptMeta>;
    fn delete_script(&self, id: &str) -> Result<()>;
}

pub struct HttpRemoteBackend {
    endpoint: String,
    token: String,
}

impl HttpRemoteBackend {
    pub fn new(endpoint: String, token: String) -> Self {
        Self { endpoint, token }
    }

    fn auth(&self) -> String {
        format!("Bearer {}", self.token)
    }

    fn download_vault(&self) -> Result<Vec<Script>> {
        let url = format!("{}/vault", self.endpoint);
        let response = ureq::get(&url)
            .set("Authorization", &self.auth())
            .call()
            .map_err(|e| anyhow!("Failed to download vault: {}", e))?;

        response
            .into_json::<Vec<Script>>()
            .map_err(|e| anyhow!("Failed to parse vault: {}", e))
    }

    fn upload_vault(&self, scripts: &[Script]) -> Result<()> {
        let url = format!("{}/vault", self.endpoint);
        ureq::put(&url)
            .set("Authorization", &self.auth())
            .set("Content-Type", "application/json")
            .send_json(scripts)
            .map_err(|e| anyhow!("Failed to upload vault: {}", e))?;
        Ok(())
    }
}

impl RemoteBackend for HttpRemoteBackend {
    fn test_connection(&self) -> Result<()> {
        let url = format!("{}/health", self.endpoint);
        ureq::get(&url)
            .call()
            .map_err(|e| anyhow!("Connection failed: {}", e))?;
        Ok(())
    }

    fn list_scripts(&self) -> Result<Vec<RemoteScriptMeta>> {
        let scripts = self.download_vault()?;
        Ok(scripts
            .into_iter()
            .map(|s| RemoteScriptMeta {
                id: s.id,
                name: s.name,
                version: s.version,
                updated_at: s.updated_at,
                hash: s.metadata.hash,
            })
            .collect())
    }

    fn fetch_script(&self, id: &str) -> Result<Script> {
        self.download_vault()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| anyhow!("Script not found on remote: {}", id))
    }

    fn push_script(&self, script: &Script) -> Result<RemoteScriptMeta> {
        let mut scripts = self.download_vault()?;
        scripts.retain(|s| s.id != script.id && s.name != script.name);
        scripts.push(script.clone());
        self.upload_vault(&scripts)?;
        Ok(RemoteScriptMeta {
            id: script.id.clone(),
            name: script.name.clone(),
            version: script.version.clone(),
            updated_at: script.updated_at,
            hash: script.metadata.hash.clone(),
        })
    }

    fn delete_script(&self, id: &str) -> Result<()> {
        let mut scripts = self.download_vault()?;
        scripts.retain(|s| s.id != id);
        self.upload_vault(&scripts)
    }
}
