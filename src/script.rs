use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    pub id: String,
    pub name: String,
    pub content: String,
    pub version: String,
    pub language: ScriptLanguage,
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub context: ScriptContext,
    pub metadata: ScriptMetadata,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptContext {
    pub directory: Option<String>,
    pub git_repo: Option<String>,
    pub git_branch: Option<String>,
    pub environment: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptMetadata {
    pub hash: String,
    pub size_bytes: usize,
    pub line_count: usize,
    pub use_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub last_run: Option<DateTime<Utc>>,
    pub last_run_by: Option<String>,
    pub avg_runtime_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Visibility {
    Private,
    Team,
    Public,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScriptLanguage {
    Bash,
    Shell,
    Python,
    JavaScript,
    Ruby,
    Perl,
    PowerShell,
    Batch,
    Unknown,
}

impl ScriptLanguage {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "sh" => Self::Shell,
            "bash" => Self::Bash,
            "py" => Self::Python,
            "js" => Self::JavaScript,
            "rb" => Self::Ruby,
            "pl" => Self::Perl,
            "ps1" => Self::PowerShell,
            "bat" | "cmd" => Self::Batch,
            _ => Self::Unknown,
        }
    }

    pub fn to_string(&self) -> &str {
        match self {
            Self::Bash => "bash",
            Self::Shell => "shell",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::Ruby => "ruby",
            Self::Perl => "perl",
            Self::PowerShell => "powershell",
            Self::Batch => "batch",
            Self::Unknown => "unknown",
        }
    }

    pub fn get_shebang(&self) -> Option<&str> {
        match self {
            Self::Bash => Some("#!/usr/bin/env bash"),
            Self::Shell => Some("#!/bin/sh"),
            Self::Python => Some("#!/usr/bin/env python3"),
            Self::Ruby => Some("#!/usr/bin/env ruby"),
            Self::Perl => Some("#!/usr/bin/env perl"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub id: String,
    pub script_id: String,
    pub script_version: String,
    pub executed_by: String,
    pub executed_at: DateTime<Utc>,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub output: Option<String>,
    pub error: Option<String>,
    pub context: ScriptContext,
}

impl Script {
    pub fn new(name: String, content: String, language: ScriptLanguage) -> Self {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        let line_count = content.lines().count();
        let size_bytes = content.len();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            content: content.clone(),
            version: "v1.0.0".to_string(),
            language,
            tags: Vec::new(),
            description: None,
            author: "local".to_string(),
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
                size_bytes,
                line_count,
                use_count: 0,
                success_count: 0,
                failure_count: 0,
                last_run: None,
                last_run_by: None,
                avg_runtime_ms: None,
            },
            visibility: Visibility::Private,
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.metadata.success_count + self.metadata.failure_count;
        if total == 0 {
            0.0
        } else {
            (self.metadata.success_count as f64 / total as f64) * 100.0
        }
    }

    pub fn is_safe(&self) -> bool {
        let dangerous_patterns = [
            "rm -rf /",
            "rm -rf /*",
            "mkfs",
            "dd if=",
            "> /dev/sda",
            ":(){ :|:& };:",
        ];

        !dangerous_patterns
            .iter()
            .any(|pattern| self.content.contains(pattern))
    }
}

impl ExecutionRecord {
    pub fn was_successful(&self) -> bool {
        self.exit_code == 0
    }
}
