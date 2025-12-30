use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_endpoint: String,
    pub vault_path: PathBuf,
    pub auth_token: Option<String>,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub team_id: Option<String>,
    pub auto_sync: bool,
    pub confirm_before_run: bool,
    pub default_visibility: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_endpoint: "https://api.scriptvault.dev".to_string(),
            vault_path: Self::default_vault_path().unwrap_or_default(),
            auth_token: None,
            user_id: None,
            username: None,
            team_id: None,
            auto_sync: true,
            confirm_before_run: true,
            default_visibility: "private".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let contents = fs::read_to_string(&path).context("Failed to read config file")?;
            let config: Config =
                serde_json::from_str(&contents).context("Failed to parse config file")?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let dir = path.parent().unwrap();

        fs::create_dir_all(dir)?;

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents)?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home.join(".scriptvault").join("config.json"))
    }

    pub fn data_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        let dir = home.join(".scriptvault");
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn vault_dir() -> Result<PathBuf> {
        let dir = Self::data_dir()?.join("vault");
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn scripts_path() -> Result<PathBuf> {
        Ok(Self::vault_dir()?.join("scripts.json"))
    }

    pub fn history_path() -> Result<PathBuf> {
        Ok(Self::data_dir()?.join("history.jsonl"))
    }

    fn default_vault_path() -> Result<PathBuf> {
        Self::vault_dir()
    }

    pub fn is_authenticated(&self) -> bool {
        self.auth_token.is_some() && self.user_id.is_some()
    }

    pub fn set_auth(&mut self, token: String, user_id: String, username: String) {
        self.auth_token = Some(token);
        self.user_id = Some(user_id);
        self.username = Some(username);
    }

    pub fn clear_auth(&mut self) {
        self.auth_token = None;
        self.user_id = None;
        self.username = None;
        self.team_id = None;
    }
}
