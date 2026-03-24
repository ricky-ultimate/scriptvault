pub mod auth;
pub mod cli;
pub mod config;
pub mod constants;
pub mod context;
pub mod execution;
pub mod script;
pub mod storage;
pub mod sync;
pub mod utils;
pub mod vault;
pub mod versions;

pub use config::Config;
pub use script::{ExecutionRecord, Script, ScriptContext, ScriptLanguage, SyncState, SyncStatus, Visibility};
pub use sync::manager::{ConflictResolution, SyncManager, SyncReport};
pub use sync::remote::RemoteBackend;
pub use versions::VersionStore;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    mod script_tests {
        use super::*;

        #[test]
        fn test_language_from_extension() {
            assert_eq!(ScriptLanguage::from_extension("sh"), ScriptLanguage::Shell);
            assert_eq!(ScriptLanguage::from_extension("bash"), ScriptLanguage::Bash);
            assert_eq!(ScriptLanguage::from_extension("py"), ScriptLanguage::Python);
            assert_eq!(
                ScriptLanguage::from_extension("js"),
                ScriptLanguage::JavaScript
            );
            assert_eq!(ScriptLanguage::from_extension("rb"), ScriptLanguage::Ruby);
            assert_eq!(ScriptLanguage::from_extension("pl"), ScriptLanguage::Perl);
            assert_eq!(
                ScriptLanguage::from_extension("ps1"),
                ScriptLanguage::PowerShell
            );
            assert_eq!(ScriptLanguage::from_extension("bat"), ScriptLanguage::Batch);
            assert_eq!(ScriptLanguage::from_extension("cmd"), ScriptLanguage::Batch);
            assert_eq!(
                ScriptLanguage::from_extension("xyz"),
                ScriptLanguage::Unknown
            );
        }

        #[test]
        fn test_language_display() {
            assert_eq!(ScriptLanguage::Bash.to_string(), "bash");
            assert_eq!(ScriptLanguage::Python.to_string(), "python");
            assert_eq!(ScriptLanguage::Shell.to_string(), "shell");
            assert_eq!(ScriptLanguage::Unknown.to_string(), "unknown");
        }

        #[test]
        fn test_shebang() {
            assert_eq!(
                ScriptLanguage::Bash.get_shebang(),
                Some("#!/usr/bin/env bash")
            );
            assert_eq!(
                ScriptLanguage::Python.get_shebang(),
                Some("#!/usr/bin/env python3")
            );
            assert_eq!(ScriptLanguage::Shell.get_shebang(), Some("#!/bin/sh"));
            assert_eq!(ScriptLanguage::PowerShell.get_shebang(), None);
        }

        #[test]
        fn test_script_creation() {
            let script = Script::new(
                "test-script".to_string(),
                "echo 'hello'".to_string(),
                ScriptLanguage::Bash,
            );
            assert_eq!(script.name, "test-script");
            assert_eq!(script.content, "echo 'hello'");
            assert_eq!(script.language, ScriptLanguage::Bash);
            assert_eq!(script.version, "v1.0.0");
            assert_eq!(script.metadata.use_count, 0);
            assert_eq!(script.visibility, Visibility::Private);
        }

        #[test]
        fn test_safety_check_safe() {
            let script = Script::new(
                "safe".to_string(),
                "echo 'Hello World'\nls -la\n".to_string(),
                ScriptLanguage::Bash,
            );
            assert!(script.is_safe());
        }

        #[test]
        fn test_safety_check_dangerous() {
            let cases = vec![
                "rm -rf /",
                "rm -rf /*",
                "mkfs /dev/sda",
                "dd if=/dev/zero of=/dev/sda",
                "> /dev/sda",
                ":(){ :|:& };:",
            ];
            for content in cases {
                let script = Script::new(
                    "dangerous".to_string(),
                    content.to_string(),
                    ScriptLanguage::Bash,
                );
                assert!(!script.is_safe(), "Failed to detect: {}", content);
            }
        }

        #[test]
        fn test_success_rate_zero_runs() {
            let script = Script::new(
                "test".to_string(),
                "echo test".to_string(),
                ScriptLanguage::Bash,
            );
            assert_eq!(script.success_rate(), 0.0);
        }

        #[test]
        fn test_success_rate_calculation() {
            let mut script = Script::new(
                "test".to_string(),
                "echo test".to_string(),
                ScriptLanguage::Bash,
            );
            script.metadata.success_count = 8;
            script.metadata.failure_count = 2;
            assert_eq!(script.success_rate(), 80.0);
        }

        #[test]
        fn test_execution_record_success() {
            let record = ExecutionRecord {
                id: "test-id".to_string(),
                script_id: "script-id".to_string(),
                script_version: "v1.0.0".to_string(),
                executed_by: "user".to_string(),
                executed_at: Utc::now(),
                exit_code: 0,
                duration_ms: 1000,
                output: Some("Success".to_string()),
                error: None,
                context: ScriptContext {
                    directory: None,
                    git_repo: None,
                    git_branch: None,
                    environment: HashMap::new(),
                },
            };
            assert!(record.was_successful());
        }

        #[test]
        fn test_execution_record_failure() {
            let record = ExecutionRecord {
                id: "test-id".to_string(),
                script_id: "script-id".to_string(),
                script_version: "v1.0.0".to_string(),
                executed_by: "user".to_string(),
                executed_at: Utc::now(),
                exit_code: 1,
                duration_ms: 1000,
                output: None,
                error: Some("Error".to_string()),
                context: ScriptContext {
                    directory: None,
                    git_repo: None,
                    git_branch: None,
                    environment: HashMap::new(),
                },
            };
            assert!(!record.was_successful());
        }
    }

    mod context_tests {
        use super::*;
        use crate::context::{contexts_match, normalize_git_url};

        #[test]
        fn test_normalize_https() {
            assert_eq!(
                normalize_git_url("https://github.com/user/repo.git"),
                "github.com/user/repo"
            );
        }

        #[test]
        fn test_normalize_ssh() {
            assert_eq!(
                normalize_git_url("git@github.com:user/repo.git"),
                "github.com/user/repo"
            );
        }

        #[test]
        fn test_normalize_no_extension() {
            assert_eq!(
                normalize_git_url("https://github.com/user/repo"),
                "github.com/user/repo"
            );
        }

        #[test]
        fn test_contexts_match_git_repo() {
            let ctx1 = ScriptContext {
                directory: Some("/home/user/project".to_string()),
                git_repo: Some("github.com/user/repo".to_string()),
                git_branch: Some("main".to_string()),
                environment: HashMap::new(),
            };
            let ctx2 = ScriptContext {
                directory: Some("/home/user/project2".to_string()),
                git_repo: Some("github.com/user/repo".to_string()),
                git_branch: Some("develop".to_string()),
                environment: HashMap::new(),
            };
            assert!(contexts_match(&ctx1, &ctx2));
        }

        #[test]
        fn test_contexts_match_same_directory() {
            let ctx1 = ScriptContext {
                directory: Some("/home/user/project".to_string()),
                git_repo: None,
                git_branch: None,
                environment: HashMap::new(),
            };
            let ctx2 = ScriptContext {
                directory: Some("/home/user/project".to_string()),
                git_repo: None,
                git_branch: None,
                environment: HashMap::new(),
            };
            assert!(contexts_match(&ctx1, &ctx2));
        }

        #[test]
        fn test_contexts_no_match() {
            let ctx1 = ScriptContext {
                directory: Some("/home/user/project1".to_string()),
                git_repo: Some("github.com/user/repo1".to_string()),
                git_branch: None,
                environment: HashMap::new(),
            };
            let ctx2 = ScriptContext {
                directory: Some("/home/user/project2".to_string()),
                git_repo: Some("github.com/user/repo2".to_string()),
                git_branch: None,
                environment: HashMap::new(),
            };
            assert!(!contexts_match(&ctx1, &ctx2));
        }

        #[test]
        fn test_contexts_match_parent_directory() {
            let ctx1 = ScriptContext {
                directory: Some("/home/user/project".to_string()),
                git_repo: None,
                git_branch: None,
                environment: HashMap::new(),
            };
            let ctx2 = ScriptContext {
                directory: Some("/home/user/project/subdir".to_string()),
                git_repo: None,
                git_branch: None,
                environment: HashMap::new(),
            };
            assert!(contexts_match(&ctx1, &ctx2));
        }
    }

    mod config_tests {
        use super::*;

        #[test]
        fn test_default_config() {
            let config = Config::default();
            assert!(!config.auto_sync);
            assert!(config.confirm_before_run);
            assert_eq!(config.default_visibility, "private");
            assert!(config.auth_token.is_none());
            assert!(config.user_id.is_none());
        }

        #[test]
        fn test_is_authenticated_false() {
            assert!(!Config::default().is_authenticated());
        }

        #[test]
        fn test_local_user_not_authenticated() {
            let mut config = Config::default();
            config.set_local_user("testuser".to_string());
            assert!(!config.is_authenticated());
            assert!(config.has_identity());
        }

        #[test]
        fn test_api_key_is_authenticated() {
            let mut config = Config::default();
            config.set_api_key("t".to_string(), "u".to_string(), "n".to_string());
            assert!(config.is_authenticated());
        }

        #[test]
        fn test_clear_auth() {
            let mut config = Config::default();
            config.set_api_key("t".to_string(), "u".to_string(), "n".to_string());
            config.clear_auth();
            assert!(!config.is_authenticated());
            assert!(!config.has_identity());
        }
    }
}
