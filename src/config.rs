use crate::constants::*;
use crate::storage::StorageConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthMode {
    Local,
    ApiKey,
    OAuth,
}

impl Default for AuthMode {
    fn default() -> Self {
        Self::Local
    }
}

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
    pub storage: StorageConfig,
    #[serde(default)]
    pub auth_mode: AuthMode,
}

impl Default for Config {
    fn default() -> Self {
        let vault_path = Self::default_vault_path().unwrap_or_default();
        Self {
            api_endpoint: api_endpoint(),
            storage: StorageConfig {
                path: vault_path.clone(),
            },
            vault_path,
            auth_token: None,
            user_id: None,
            username: None,
            team_id: None,
            auto_sync: false,
            confirm_before_run: true,
            default_visibility: DEFAULT_VISIBILITY.to_string(),
            auth_mode: AuthMode::Local,
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
        Ok(Self::base_dir()?.join(CONFIG_FILE))
    }

    pub fn base_dir() -> Result<PathBuf> {
        if let Ok(custom_dir) = std::env::var(ENV_SCRIPTVAULT_HOME) {
            let path = PathBuf::from(custom_dir);
            fs::create_dir_all(&path)?;
            return Ok(path);
        }
        let home = dirs::home_dir().context("Could not determine home directory")?;
        let dir = home.join(SCRIPTVAULT_DIR);
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn data_dir() -> Result<PathBuf> {
        Self::base_dir()
    }

    pub fn vault_dir() -> Result<PathBuf> {
        let dir = Self::data_dir()?.join(VAULT_DIR);
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    #[allow(dead_code)]
    pub fn scripts_path() -> Result<PathBuf> {
        Ok(Self::vault_dir()?.join(SCRIPTS_FILE))
    }
    pub fn history_path() -> Result<PathBuf> {
        Ok(Self::data_dir()?.join(HISTORY_FILE))
    }

    fn default_vault_path() -> Result<PathBuf> {
        Self::vault_dir()
    }

    pub fn is_authenticated(&self) -> bool {
        self.auth_mode != AuthMode::Local && self.auth_token.is_some() && self.user_id.is_some()
    }

    #[allow(dead_code)]
    pub fn has_identity(&self) -> bool {
        self.username.is_some()
    }

    pub fn set_local_user(&mut self, username: String) {
        self.auth_mode = AuthMode::Local;
        self.auth_token = None;
        self.user_id = None;
        self.username = Some(username);
    }

    pub fn set_api_key(&mut self, token: String, user_id: String, username: String) {
        self.auth_mode = AuthMode::ApiKey;
        self.auth_token = Some(token);
        self.user_id = Some(user_id);
        self.username = Some(username);
    }

    #[allow(dead_code)]
    pub fn set_oauth(&mut self, token: String, user_id: String, username: String) {
        self.auth_mode = AuthMode::OAuth;
        self.auth_token = Some(token);
        self.user_id = Some(user_id);
        self.username = Some(username);
    }

    pub fn clear_auth(&mut self) {
        self.auth_mode = AuthMode::Local;
        self.auth_token = None;
        self.user_id = None;
        self.username = None;
        self.team_id = None;
    }

    pub fn get_storage_backend(&self) -> Result<Box<dyn crate::storage::StorageBackend>> {
        crate::storage::create_storage_backend(&self.storage)
    }

    pub fn set_storage(&mut self, storage: StorageConfig) -> Result<()> {
        self.vault_path = storage.path.clone();
        self.storage = storage;
        self.save()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(!config.auto_sync);
        assert!(config.confirm_before_run);
        assert_eq!(config.default_visibility, "private");
        assert!(config.auth_token.is_none());
        assert!(config.user_id.is_none());
        assert_eq!(config.auth_mode, AuthMode::Local);
    }

    #[test]
    fn test_is_authenticated_false_by_default() {
        assert!(!Config::default().is_authenticated());
    }

    #[test]
    fn test_local_user_is_not_authenticated() {
        let mut config = Config::default();
        config.set_local_user("testuser".to_string());
        assert!(!config.is_authenticated());
        assert!(config.has_identity());
        assert!(config.auth_token.is_none());
        assert!(config.user_id.is_none());
        assert_eq!(config.auth_mode, AuthMode::Local);
    }

    #[test]
    fn test_api_key_is_authenticated() {
        let mut config = Config::default();
        config.set_api_key(
            "token123".to_string(),
            "user123".to_string(),
            "TestUser".to_string(),
        );
        assert!(config.is_authenticated());
        assert!(config.has_identity());
        assert_eq!(config.auth_mode, AuthMode::ApiKey);
        assert_eq!(config.auth_token, Some("token123".to_string()));
        assert_eq!(config.user_id, Some("user123".to_string()));
        assert_eq!(config.username, Some("TestUser".to_string()));
    }

    #[test]
    fn test_oauth_is_authenticated() {
        let mut config = Config::default();
        config.set_oauth(
            "oauth_token".to_string(),
            "user456".to_string(),
            "OAuthUser".to_string(),
        );
        assert!(config.is_authenticated());
        assert_eq!(config.auth_mode, AuthMode::OAuth);
    }

    #[test]
    fn test_clear_auth_resets_to_local() {
        let mut config = Config::default();
        config.set_api_key(
            "token123".to_string(),
            "user123".to_string(),
            "TestUser".to_string(),
        );
        config.clear_auth();
        assert!(!config.is_authenticated());
        assert!(!config.has_identity());
        assert_eq!(config.auth_mode, AuthMode::Local);
        assert!(config.auth_token.is_none());
        assert!(config.user_id.is_none());
        assert!(config.username.is_none());
    }

    #[test]
    fn test_set_local_user_clears_existing_token() {
        let mut config = Config::default();
        config.set_api_key(
            "token123".to_string(),
            "user123".to_string(),
            "TestUser".to_string(),
        );
        config.set_local_user("localuser".to_string());
        assert!(!config.is_authenticated());
        assert!(config.auth_token.is_none());
        assert!(config.user_id.is_none());
        assert_eq!(config.username, Some("localuser".to_string()));
    }
}
