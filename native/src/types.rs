use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Severity level for lint results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Severity {
    #[default]
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

/// A single lint result
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    pub rule_id: String,
    pub severity: String,
    pub message: String,
    pub path: String,
    pub line: Option<u32>,
    pub suggestion: Option<String>,
}

impl LintResult {
    pub fn new(
        rule_id: &str,
        severity: Severity,
        message: String,
        path: PathBuf,
        line: Option<u32>,
        suggestion: Option<String>,
    ) -> Self {
        Self {
            rule_id: rule_id.to_string(),
            severity: severity.to_string(),
            message,
            path: path.display().to_string(),
            line,
            suggestion,
        }
    }
}

/// Complete lint report
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintReport {
    pub results: Vec<LintResult>,
    pub error_count: u32,
    pub warning_count: u32,
    pub info_count: u32,
    pub fixed_count: u32,
}

impl LintReport {
    pub fn new(results: Vec<LintResult>, fixed_count: u32) -> Self {
        let error_count = results.iter().filter(|r| r.severity == "error").count() as u32;
        let warning_count = results.iter().filter(|r| r.severity == "warning").count() as u32;
        let info_count = results.iter().filter(|r| r.severity == "info").count() as u32;

        Self {
            results,
            error_count,
            warning_count,
            info_count,
            fixed_count,
        }
    }
}

/// Rule information for listing
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub default_severity: String,
    pub can_fix: bool,
}

/// Configuration for a single rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub severity: Option<Severity>,
    #[serde(default)]
    pub options: serde_json::Value,
}

fn default_true() -> bool {
    true
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            severity: None,
            options: serde_json::Value::Null,
        }
    }
}

/// Main configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub rules: HashMap<String, RuleConfig>,
}

/// Context passed to rules during execution
pub struct RuleContext {
    pub root: PathBuf,
    pub fix_mode: bool,
    pub config: serde_json::Value,
}

impl RuleContext {
    pub fn new(root: PathBuf, fix_mode: bool, config: serde_json::Value) -> Self {
        Self {
            root,
            fix_mode,
            config,
        }
    }

    pub fn read_file(&self, path: &std::path::Path) -> Result<String, std::io::Error> {
        std::fs::read_to_string(path)
    }

    pub fn write_file(&self, path: &std::path::Path, content: &str) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
    }

    pub fn file_exists(&self, path: &std::path::Path) -> bool {
        path.exists()
    }
}
