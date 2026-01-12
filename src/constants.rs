use std::env;

/// Default script version for newly created scripts
pub const DEFAULT_VERSION: &str = "v1.0.0";

/// Default author when not authenticated
pub fn default_author() -> String {
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "local".to_string())
}

/// Default API endpoint (currently unused in local mode)
pub fn api_endpoint() -> String {
    env::var("SCRIPTVAULT_API_ENDPOINT")
        .unwrap_or_else(|_| "https://api.scriptvault.dev".to_string())
}

/// Default visibility for new scripts
pub const DEFAULT_VISIBILITY: &str = "private";

/// Base directory name for ScriptVault data
pub const SCRIPTVAULT_DIR: &str = ".scriptvault";

/// Configuration file name
pub const CONFIG_FILE: &str = "config.json";

/// Scripts storage file name
pub const SCRIPTS_FILE: &str = "scripts.json";

/// Execution history file name
pub const HISTORY_FILE: &str = "history.jsonl";

/// Vault subdirectory name
pub const VAULT_DIR: &str = "vault";

/// Maximum number of history entries to display by default
pub const DEFAULT_HISTORY_LIMIT: usize = 20;

/// Maximum number of search results to display by default
pub const DEFAULT_SEARCH_LIMIT: usize = 20;

/// Dangerous command patterns that trigger safety warnings
pub const DANGEROUS_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "mkfs",
    "dd if=",
    "> /dev/sda",
    ":(){ :|:& };:", // fork bomb
    "chmod -R 777 /",
    "chown -R",
    "> /dev/sd",
    "mkfs.ext",
    ":(){:|:&};:",
];

/// Supported script file extensions
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "sh", "bash", "py", "js", "rb", "pl", "ps1", "bat", "cmd",
];

/// Environment variable for custom ScriptVault directory
pub const ENV_SCRIPTVAULT_HOME: &str = "SCRIPTVAULT_HOME";

/// Environment variable for API endpoint override
pub const ENV_API_ENDPOINT: &str = "SCRIPTVAULT_API_ENDPOINT";

/// Environment variable for disabling interactive prompts
pub const ENV_SCRIPTVAULT_CI: &str = "SCRIPTVAULT_CI";

/// Default shell interpreters by language
pub const BASH_INTERPRETER: &str = "bash";
pub const SHELL_INTERPRETER: &str = "sh";
pub const PYTHON_INTERPRETER: &str = "python3";
pub const RUBY_INTERPRETER: &str = "ruby";
pub const PERL_INTERPRETER: &str = "perl";
pub const POWERSHELL_INTERPRETER: &str = "powershell";

/// Shebang lines for different languages
pub const BASH_SHEBANG: &str = "#!/usr/bin/env bash";
pub const SHELL_SHEBANG: &str = "#!/bin/sh";
pub const PYTHON_SHEBANG: &str = "#!/usr/bin/env python3";
pub const RUBY_SHEBANG: &str = "#!/usr/bin/env ruby";
pub const PERL_SHEBANG: &str = "#!/usr/bin/env perl";
