use crate::rules::{Rule, RuleError};
use crate::types::{LintResult, RuleContext, Severity};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Project type detection for Husky initialization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectType {
    JavaScript,
    Rust,
}

/// Strategy trait for project-specific Husky initialization
trait HuskyStrategy: Send + Sync {
    fn project_type(&self) -> ProjectType;
    fn check(&self, repo_root: &Path, rule_id: &str) -> Vec<LintResult>;
    fn fix(&self, repo_root: &Path) -> Result<bool, RuleError>;
}

/// JavaScript/TypeScript Husky strategy
struct JsHuskyStrategy;

impl HuskyStrategy for JsHuskyStrategy {
    fn project_type(&self) -> ProjectType {
        ProjectType::JavaScript
    }

    fn check(&self, repo_root: &Path, rule_id: &str) -> Vec<LintResult> {
        let mut results = Vec::new();
        let husky_dir = repo_root.join(".husky");
        let package_json_path = repo_root.join("package.json");

        // Check if .husky directory exists
        if !husky_dir.exists() {
            results.push(LintResult::new(
                rule_id,
                Severity::Warning,
                "Missing .husky directory - Husky is not initialized".into(),
                repo_root.to_path_buf(),
                None,
                Some("Run 'npx husky init' or 'pnpm exec husky init' to initialize Husky".into()),
            ));
            return results;
        }

        // Check if package.json has prepare script with husky
        if package_json_path.exists() {
            match std::fs::read_to_string(&package_json_path) {
                Ok(content) => {
                    if let Ok(json) = serde_json::from_str::<Value>(&content) {
                        let has_prepare_script = json
                            .get("scripts")
                            .and_then(|s| s.get("prepare"))
                            .and_then(|p| p.as_str())
                            .is_some_and(|s| s.contains("husky"));

                        if !has_prepare_script {
                            results.push(LintResult::new(
                                rule_id,
                                Severity::Warning,
                                "Missing 'prepare' script with Husky in package.json".into(),
                                package_json_path.clone(),
                                None,
                                Some(
                                    "Add '\"prepare\": \"husky\"' to scripts in package.json".into(),
                                ),
                            ));
                        }
                    }
                }
                Err(e) => {
                    results.push(LintResult::new(
                        rule_id,
                        Severity::Error,
                        format!("Cannot read package.json: {}", e),
                        package_json_path,
                        None,
                        None,
                    ));
                }
            }
        }

        // Check for at least one hook file (pre-commit is most common)
        let has_hooks = husky_dir
            .read_dir()
            .map(|entries| {
                entries.filter_map(|e| e.ok()).any(|entry| {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    // Common git hooks
                    matches!(
                        name_str.as_ref(),
                        "pre-commit"
                            | "commit-msg"
                            | "pre-push"
                            | "post-merge"
                            | "post-checkout"
                    )
                })
            })
            .unwrap_or(false);

        if !has_hooks {
            results.push(LintResult::new(
                rule_id,
                Severity::Info,
                "No git hooks found in .husky directory".into(),
                husky_dir,
                None,
                Some("Add hooks like 'npx husky add .husky/pre-commit \"npm test\"'".into()),
            ));
        }

        results
    }

    fn fix(&self, repo_root: &Path) -> Result<bool, RuleError> {
        let husky_dir = repo_root.join(".husky");
        let package_json_path = repo_root.join("package.json");

        if husky_dir.exists() {
            return Ok(false); // Already initialized
        }

        // Try to initialize Husky using npx
        let init_result = Command::new("npx")
            .args(["husky", "init"])
            .current_dir(repo_root)
            .output();

        match init_result {
            Ok(output) if output.status.success() => {
                // Update package.json prepare script if needed
                if package_json_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&package_json_path) {
                        if let Ok(mut json) = serde_json::from_str::<Value>(&content) {
                            let scripts = json
                                .as_object_mut()
                                .and_then(|obj| obj.get_mut("scripts"))
                                .and_then(|s| s.as_object_mut());

                            if let Some(scripts) = scripts {
                                if !scripts.contains_key("prepare") {
                                    scripts
                                        .insert("prepare".into(), Value::String("husky".into()));
                                    if let Ok(updated) = serde_json::to_string_pretty(&json) {
                                        let _ = std::fs::write(&package_json_path, updated);
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(true)
            }
            Ok(output) => Err(RuleError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Husky init failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ))),
            Err(e) => Err(RuleError::Io(e)),
        }
    }
}

/// Rust husky-rs strategy
struct RustHuskyStrategy;

impl HuskyStrategy for RustHuskyStrategy {
    fn project_type(&self) -> ProjectType {
        ProjectType::Rust
    }

    fn check(&self, repo_root: &Path, rule_id: &str) -> Vec<LintResult> {
        let mut results = Vec::new();
        let husky_dir = repo_root.join(".husky");
        let cargo_toml_path = repo_root.join("Cargo.toml");

        // Check if .husky directory exists
        if !husky_dir.exists() {
            results.push(LintResult::new(
                rule_id,
                Severity::Warning,
                "Missing .husky directory - husky-rs is not initialized".into(),
                repo_root.to_path_buf(),
                None,
                Some("Run 'cargo husky-rs init' to initialize husky-rs".into()),
            ));
            return results;
        }

        // Check if Cargo.toml has husky-rs as dev-dependency
        if cargo_toml_path.exists() {
            match std::fs::read_to_string(&cargo_toml_path) {
                Ok(content) => {
                    let has_husky_rs = content.contains("husky-rs")
                        || content.contains("[dev-dependencies.husky-rs]");

                    if !has_husky_rs {
                        results.push(LintResult::new(
                            rule_id,
                            Severity::Warning,
                            "Missing husky-rs in dev-dependencies".into(),
                            cargo_toml_path.clone(),
                            None,
                            Some(
                                "Add 'husky-rs = \"<version>\"' to [dev-dependencies] in Cargo.toml"
                                    .into(),
                            ),
                        ));
                    }
                }
                Err(e) => {
                    results.push(LintResult::new(
                        rule_id,
                        Severity::Error,
                        format!("Cannot read Cargo.toml: {}", e),
                        cargo_toml_path,
                        None,
                        None,
                    ));
                }
            }
        }

        // Check for at least one hook file
        let has_hooks = husky_dir
            .read_dir()
            .map(|entries| {
                entries.filter_map(|e| e.ok()).any(|entry| {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    matches!(
                        name_str.as_ref(),
                        "pre-commit"
                            | "commit-msg"
                            | "pre-push"
                            | "post-merge"
                            | "post-checkout"
                    )
                })
            })
            .unwrap_or(false);

        if !has_hooks {
            results.push(LintResult::new(
                rule_id,
                Severity::Info,
                "No git hooks found in .husky directory".into(),
                husky_dir,
                None,
                Some("Add hooks using 'cargo husky-rs add pre-commit \"cargo test\"'".into()),
            ));
        }

        results
    }

    fn fix(&self, repo_root: &Path) -> Result<bool, RuleError> {
        let husky_dir = repo_root.join(".husky");

        if husky_dir.exists() {
            return Ok(false); // Already initialized
        }

        // Try to initialize husky-rs using cargo
        let init_result = Command::new("cargo")
            .args(["husky-rs", "init"])
            .current_dir(repo_root)
            .output();

        match init_result {
            Ok(output) if output.status.success() => Ok(true),
            Ok(output) => Err(RuleError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "husky-rs init failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ))),
            Err(e) => Err(RuleError::Io(e)),
        }
    }
}

/// Rule: Ensure git repositories have Husky initialized based on project type
pub struct HuskyInitRule;

impl HuskyInitRule {
    pub fn new() -> Self {
        Self
    }

    /// Find all git repositories in the given root
    fn find_git_repos(&self, root: &Path) -> Vec<PathBuf> {
        let mut repos = Vec::new();

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_dir() && path.file_name().is_some_and(|n| n == ".git") {
                if let Some(parent) = path.parent() {
                    repos.push(parent.to_path_buf());
                }
            }
        }

        repos
    }

    /// Detect project type based on manifest files
    fn detect_project_type(&self, repo_root: &Path) -> Option<ProjectType> {
        let has_package_json = repo_root.join("package.json").exists();
        let has_cargo_toml = repo_root.join("Cargo.toml").exists();

        match (has_package_json, has_cargo_toml) {
            // Pure Rust project
            (false, true) => Some(ProjectType::Rust),
            // JavaScript project (or hybrid with JS as primary)
            (true, _) => Some(ProjectType::JavaScript),
            // No recognized project type
            (false, false) => None,
        }
    }

    /// Get the appropriate strategy for the project type
    fn get_strategy(&self, project_type: ProjectType) -> Box<dyn HuskyStrategy> {
        match project_type {
            ProjectType::JavaScript => Box::new(JsHuskyStrategy),
            ProjectType::Rust => Box::new(RustHuskyStrategy),
        }
    }

    /// Check a single repository
    fn check_repo(&self, repo_root: &Path) -> Vec<LintResult> {
        match self.detect_project_type(repo_root) {
            Some(project_type) => {
                let strategy = self.get_strategy(project_type);
                strategy.check(repo_root, self.id())
            }
            None => {
                // Skip repositories without package.json or Cargo.toml
                Vec::new()
            }
        }
    }

    /// Fix a single repository
    fn fix_repo(&self, repo_root: &Path) -> Result<bool, RuleError> {
        match self.detect_project_type(repo_root) {
            Some(project_type) => {
                let strategy = self.get_strategy(project_type);
                strategy.fix(repo_root)
            }
            None => Ok(false),
        }
    }
}

impl Default for HuskyInitRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for HuskyInitRule {
    fn id(&self) -> &'static str {
        "husky-init"
    }

    fn name(&self) -> &'static str {
        "Husky Initialization"
    }

    fn description(&self) -> &'static str {
        "Ensures git repositories have Husky (JS) or husky-rs (Rust) initialized for git hooks"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, context: &RuleContext) -> Vec<LintResult> {
        let mut results = Vec::new();

        let repos = self.find_git_repos(&context.root);

        for repo in repos {
            results.extend(self.check_repo(&repo));
        }

        results
    }

    fn can_fix(&self) -> bool {
        true
    }

    fn fix(&self, context: &RuleContext) -> Result<u32, RuleError> {
        let mut fixed = 0;

        let repos = self.find_git_repos(&context.root);

        for repo in repos {
            if self.fix_repo(&repo)? {
                fixed += 1;
            }
        }

        Ok(fixed)
    }
}
