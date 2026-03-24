use crate::script::Script;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteScriptMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub updated_at: DateTime<Utc>,
    pub hash: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
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

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }
}

impl RemoteBackend for HttpRemoteBackend {
    fn test_connection(&self) -> Result<()> {
        ureq::get(&format!("{}/health", self.endpoint))
            .call()
            .map_err(|e| anyhow!("connection failed: {}", e))?;
        Ok(())
    }

    fn list_scripts(&self) -> Result<Vec<RemoteScriptMeta>> {
        let resp = ureq::get(&format!("{}/scripts", self.endpoint))
            .set("Authorization", &self.auth_header())
            .call()
            .map_err(|e| anyhow!("list_scripts failed: {}", e))?;
        resp.into_json::<Vec<RemoteScriptMeta>>()
            .map_err(|e| anyhow!("failed to parse script list: {}", e))
    }

    fn fetch_script(&self, id: &str) -> Result<Script> {
        let resp = ureq::get(&format!("{}/scripts/{}", self.endpoint, id))
            .set("Authorization", &self.auth_header())
            .call()
            .map_err(|e| anyhow!("fetch_script failed: {}", e))?;
        resp.into_json::<Script>()
            .map_err(|e| anyhow!("failed to parse script: {}", e))
    }

    fn push_script(&self, script: &Script) -> Result<RemoteScriptMeta> {
        let etag = script.sync_state.conflict_base_hash.clone();
        let body = serde_json::to_value(script)?;

        let mut req = ureq::put(&format!("{}/scripts/{}", self.endpoint, script.id))
            .set("Authorization", &self.auth_header())
            .set("Content-Type", "application/json");

        if let Some(ref e) = etag {
            req = req.set("If-Match", &format!("\"{}\"", e));
        }

        req.send_json(body).map_err(|e| match e {
            ureq::Error::Status(412, _) => {
                anyhow!("push rejected: remote was modified since last sync")
            }
            other => anyhow!("push_script failed: {}", other),
        })?;

        Ok(RemoteScriptMeta {
            id: script.id.clone(),
            name: script.name.clone(),
            version: script.version.clone(),
            updated_at: script.updated_at,
            hash: script.metadata.hash.clone(),
            tags: script.tags.clone(),
            description: script.description.clone(),
        })
    }

    fn delete_script(&self, id: &str) -> Result<()> {
        ureq::delete(&format!("{}/scripts/{}", self.endpoint, id))
            .set("Authorization", &self.auth_header())
            .call()
            .map_err(|e| anyhow!("delete_script failed: {}", e))?;
        Ok(())
    }
}
