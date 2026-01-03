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

/// Describes a single check operation a rule performs
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckEntry {
    /// Unique identifier within the rule (e.g., "settings-file-exists")
    pub id: String,
    /// Human-readable description of what this check validates
    pub description: String,
}

impl CheckEntry {
    pub fn new(id: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
        }
    }
}

/// Describes a single fix operation a rule can perform
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixEntry {
    /// Unique identifier within the rule (e.g., "create-settings-file")
    pub id: String,
    /// Human-readable description of what this fix does
    pub description: String,
    /// Which check IDs this fix addresses
    pub addresses: Vec<String>,
}

impl FixEntry {
    pub fn new(id: &str, description: &str, addresses: Vec<&str>) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
            addresses: addresses.into_iter().map(String::from).collect(),
        }
    }
}

/// A single lint result
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    /// The rule that produced this result
    pub rule_id: String,
    /// The specific check within the rule that produced this result
    pub check_id: String,
    /// Severity level (error, warning, info)
    pub severity: String,
    /// Human-readable message describing the issue
    pub message: String,
    /// File or directory path where the issue was found
    pub path: String,
    /// Line number (if applicable)
    pub line: Option<u32>,
    /// Suggestion for how to fix the issue
    pub suggestion: Option<String>,
    /// Which fix IDs can address this issue
    pub fixable_by: Vec<String>,
}

impl LintResult {
    /// Create a new LintResult with check_id and fixable_by
    pub fn new(
        rule_id: &str,
        check_id: &str,
        severity: Severity,
        message: String,
        path: PathBuf,
        line: Option<u32>,
        suggestion: Option<String>,
        fixable_by: Vec<&str>,
    ) -> Self {
        Self {
            rule_id: rule_id.to_string(),
            check_id: check_id.to_string(),
            severity: severity.to_string(),
            message,
            path: path.display().to_string(),
            line,
            suggestion,
            fixable_by: fixable_by.into_iter().map(String::from).collect(),
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
    /// Unique identifier for the rule
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this rule does
    pub description: String,
    /// Default severity level
    pub default_severity: String,
    /// Whether any fixes are available
    pub can_fix: bool,
    /// All checks this rule performs
    pub checks: Vec<CheckEntry>,
    /// All fixes this rule can apply
    pub fixes: Vec<FixEntry>,
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
