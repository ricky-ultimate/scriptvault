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
}

impl RemoteBackend for HttpRemoteBackend {
    fn test_connection(&self) -> Result<()> {
        Err(anyhow!(
            "Cloud sync is not yet available. Endpoint: {}",
            self.endpoint
        ))
    }

    fn list_scripts(&self) -> Result<Vec<RemoteScriptMeta>> {
        Err(anyhow!("Cloud sync is not yet available"))
    }

    fn fetch_script(&self, _id: &str) -> Result<Script> {
        Err(anyhow!("Cloud sync is not yet available"))
    }

    fn push_script(&self, _script: &Script) -> Result<RemoteScriptMeta> {
        Err(anyhow!("Cloud sync is not yet available"))
    }

    fn delete_script(&self, _id: &str) -> Result<()> {
        Err(anyhow!("Cloud sync is not yet available"))
    }
}
