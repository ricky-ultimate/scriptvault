use std::env;

pub const DEFAULT_VERSION: &str = "v1.0.0";
pub const DEFAULT_VISIBILITY: &str = "private";
pub const SCRIPTVAULT_DIR: &str = ".scriptvault";
pub const CONFIG_FILE: &str = "config.json";
#[allow(dead_code)]
pub const SCRIPTS_FILE: &str = "scripts.json";
pub const HISTORY_FILE: &str = "history.jsonl";
pub const VAULT_DIR: &str = "vault";
pub const DEFAULT_HISTORY_LIMIT: usize = 20;
pub const MAX_HISTORY_ENTRIES: usize = 1000;

pub const DANGEROUS_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "mkfs",
    "dd if=",
    "> /dev/sda",
    ":(){ :|:& };:",
    "chmod -R 777 /",
    "chown -R",
    "> /dev/sd",
    "mkfs.ext",
    ":(){:|:&};:",
];

#[allow(dead_code)]
pub const SUPPORTED_EXTENSIONS: &[&str] =
    &["sh", "bash", "py", "js", "rb", "pl", "ps1", "bat", "cmd"];

pub const ENV_SCRIPTVAULT_HOME: &str = "SCRIPTVAULT_HOME";
pub const ENV_SCRIPTVAULT_CI: &str = "SCRIPTVAULT_CI";

pub const BASH_INTERPRETER: &str = "bash";
pub const SHELL_INTERPRETER: &str = "sh";
pub const PYTHON_INTERPRETER: &str = "python3";
pub const RUBY_INTERPRETER: &str = "ruby";
pub const PERL_INTERPRETER: &str = "perl";
pub const POWERSHELL_INTERPRETER: &str = "powershell";

#[allow(dead_code)]
pub const BASH_SHEBANG: &str = "#!/usr/bin/env bash";
#[allow(dead_code)]
pub const SHELL_SHEBANG: &str = "#!/bin/sh";
#[allow(dead_code)]
pub const PYTHON_SHEBANG: &str = "#!/usr/bin/env python3";
#[allow(dead_code)]
pub const RUBY_SHEBANG: &str = "#!/usr/bin/env ruby";
#[allow(dead_code)]
pub const PERL_SHEBANG: &str = "#!/usr/bin/env perl";

pub fn default_author() -> String {
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "local".to_string())
}

pub fn api_endpoint() -> String {
    env::var("SCRIPTVAULT_API_ENDPOINT")
        .unwrap_or_else(|_| "https://scriptvault.fly.dev".to_string())
}
