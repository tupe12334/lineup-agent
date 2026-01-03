use crate::rules::{Rule, RuleError};
use crate::types::{CheckEntry, FixEntry, LintResult, RuleContext, Severity};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

// Check IDs
const CHECK_DEPENDENCY_EXISTS: &str = "eslint-config-agent-dependency";
const CHECK_CONFIG_FILE_EXISTS: &str = "eslint-config-mjs-exists";
const CHECK_CONFIG_USES_AGENT: &str = "eslint-config-uses-agent";
const CHECK_NO_OVERRIDES: &str = "no-custom-overrides";
const CHECK_NO_LEGACY_CONFIG: &str = "no-legacy-eslint-config";

// Fix IDs
const FIX_INSTALL_DEPENDENCY: &str = "install-eslint-config-agent";
const FIX_CREATE_CONFIG: &str = "create-eslint-config-mjs";
const FIX_REMOVE_LEGACY: &str = "remove-legacy-eslint-configs";

/// Rule: Ensure projects use eslint-config-agent as the only ESLint configuration
pub struct EslintConfigAgentRule;

impl EslintConfigAgentRule {
    pub fn new() -> Self {
        Self
    }

    /// Find all package.json files in the given root (excluding node_modules)
    fn find_package_jsons(&self, root: &Path) -> Vec<PathBuf> {
        let mut package_jsons = Vec::new();

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip node_modules directories
            if path
                .components()
                .any(|c| c.as_os_str() == "node_modules")
            {
                continue;
            }

            if path.is_file() && path.file_name().is_some_and(|n| n == "package.json") {
                package_jsons.push(path.to_path_buf());
            }
        }

        package_jsons
    }

    /// Check if a package.json represents a JavaScript/TypeScript project that should have ESLint
    fn is_js_project(&self, package_json_path: &Path) -> bool {
        if let Ok(content) = std::fs::read_to_string(package_json_path) {
            if let Ok(json) = serde_json::from_str::<Value>(&content) {
                // Check for common JS indicators
                let has_deps = json.get("dependencies").is_some()
                    || json.get("devDependencies").is_some();
                let has_scripts = json.get("scripts").is_some();
                return has_deps || has_scripts;
            }
        }
        false
    }

    /// Check if eslint-config-agent is in dependencies
    fn has_eslint_config_agent(&self, json: &Value) -> bool {
        let check_deps = |deps_key: &str| -> bool {
            json.get(deps_key)
                .and_then(|d| d.as_object())
                .is_some_and(|deps| deps.contains_key("eslint-config-agent"))
        };

        check_deps("dependencies") || check_deps("devDependencies")
    }

    /// Check eslint.config.mjs content
    fn check_eslint_config(&self, parent_dir: &Path) -> Vec<LintResult> {
        let mut results = Vec::new();
        let eslint_config_path = parent_dir.join("eslint.config.mjs");

        if !eslint_config_path.exists() {
            results.push(LintResult::new(
                self.id(),
                CHECK_CONFIG_FILE_EXISTS,
                self.default_severity(),
                "Missing eslint.config.mjs file".into(),
                parent_dir.to_path_buf(),
                None,
                Some(
                    "Create eslint.config.mjs that exports eslint-config-agent as the only config"
                        .into(),
                ),
                vec![FIX_CREATE_CONFIG],
            ));
            return results;
        }

        // Read and check content
        match std::fs::read_to_string(&eslint_config_path) {
            Ok(content) => {
                // Check if it imports from eslint-config-agent
                if !content.contains("eslint-config-agent") {
                    results.push(LintResult::new(
                        self.id(),
                        CHECK_CONFIG_USES_AGENT,
                        self.default_severity(),
                        "eslint.config.mjs does not use eslint-config-agent".into(),
                        eslint_config_path.clone(),
                        None,
                        Some(
                            "Update eslint.config.mjs to use eslint-config-agent as the only config"
                                .into(),
                        ),
                        vec![FIX_CREATE_CONFIG],
                    ));
                }

                // Check if there are any overrides or additional configurations
                // Look for patterns that indicate custom rules or extensions
                let has_spread = content.contains("...");
                let has_rules_override = content.contains("rules:");

                // The ideal config should just re-export the config without modifications
                if has_spread || has_rules_override {
                    results.push(LintResult::new(
                        self.id(),
                        CHECK_NO_OVERRIDES,
                        Severity::Warning,
                        "eslint.config.mjs contains custom overrides or rules".into(),
                        eslint_config_path,
                        None,
                        Some(
                            "Remove all custom overrides - eslint-config-agent should be the only config"
                                .into(),
                        ),
                        vec![FIX_CREATE_CONFIG],
                    ));
                }
            }
            Err(e) => {
                results.push(LintResult::new(
                    self.id(),
                    CHECK_CONFIG_FILE_EXISTS,
                    Severity::Error,
                    format!("Cannot read eslint.config.mjs: {}", e),
                    eslint_config_path,
                    None,
                    None,
                    vec![], // Cannot auto-fix read errors
                ));
            }
        }

        results
    }

    /// Check a single package.json and its ESLint configuration
    fn check_package_json(&self, package_json_path: &Path) -> Vec<LintResult> {
        let mut results = Vec::new();
        let parent_dir = package_json_path.parent().unwrap_or(Path::new("."));

        // Only check JS/TS projects
        if !self.is_js_project(package_json_path) {
            return results;
        }

        // Parse package.json
        match std::fs::read_to_string(package_json_path) {
            Ok(content) => match serde_json::from_str::<Value>(&content) {
                Ok(json) => {
                    // Check for eslint-config-agent dependency
                    if !self.has_eslint_config_agent(&json) {
                        results.push(LintResult::new(
                            self.id(),
                            CHECK_DEPENDENCY_EXISTS,
                            self.default_severity(),
                            "Missing eslint-config-agent in devDependencies".into(),
                            package_json_path.to_path_buf(),
                            None,
                            Some("Install eslint-config-agent using 'pnpm add -D eslint-config-agent@latest'".into()),
                            vec![FIX_INSTALL_DEPENDENCY],
                        ));
                    }

                    // Check for old ESLint config files that should be removed
                    let old_configs = [".eslintrc", ".eslintrc.js", ".eslintrc.json", ".eslintrc.yml", ".eslintrc.yaml", "eslint.config.js"];
                    for old_config in old_configs {
                        let old_path = parent_dir.join(old_config);
                        if old_path.exists() {
                            results.push(LintResult::new(
                                self.id(),
                                CHECK_NO_LEGACY_CONFIG,
                                Severity::Warning,
                                format!("Found legacy ESLint config file: {}", old_config),
                                old_path,
                                None,
                                Some(format!("Remove {} and use eslint.config.mjs with eslint-config-agent", old_config)),
                                vec![FIX_REMOVE_LEGACY],
                            ));
                        }
                    }

                    // Check eslint.config.mjs
                    results.extend(self.check_eslint_config(parent_dir));
                }
                Err(e) => {
                    results.push(LintResult::new(
                        self.id(),
                        CHECK_DEPENDENCY_EXISTS,
                        Severity::Error,
                        format!("Invalid JSON in package.json: {}", e),
                        package_json_path.to_path_buf(),
                        None,
                        Some("Fix JSON syntax errors".into()),
                        vec![], // Cannot auto-fix invalid JSON
                    ));
                }
            },
            Err(e) => {
                results.push(LintResult::new(
                    self.id(),
                    CHECK_DEPENDENCY_EXISTS,
                    Severity::Error,
                    format!("Cannot read package.json: {}", e),
                    package_json_path.to_path_buf(),
                    None,
                    None,
                    vec![], // Cannot auto-fix read errors
                ));
            }
        }

        results
    }

    /// Generate the correct eslint.config.mjs content
    fn get_eslint_config_content(&self) -> String {
        r#"import config from "eslint-config-agent";

export default config;
"#
        .to_string()
    }

    /// Install eslint-config-agent and create eslint.config.mjs
    fn fix_package(&self, package_json_path: &Path, context: &RuleContext) -> Result<u32, RuleError> {
        let mut fixed = 0;
        let parent_dir = package_json_path.parent().unwrap_or(Path::new("."));

        // Only fix JS/TS projects
        if !self.is_js_project(package_json_path) {
            return Ok(0);
        }

        // Check if we need to install eslint-config-agent
        let content = context.read_file(package_json_path)?;
        let json: Value = serde_json::from_str(&content)?;

        if !self.has_eslint_config_agent(&json) {
            // Install eslint-config-agent using pnpm
            let install_result = Command::new("pnpm")
                .args(["add", "-D", "eslint-config-agent@latest"])
                .current_dir(parent_dir)
                .output();

            match install_result {
                Ok(output) if output.status.success() => {
                    fixed += 1;
                }
                Ok(output) => {
                    return Err(RuleError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "Failed to install eslint-config-agent: {}",
                            String::from_utf8_lossy(&output.stderr)
                        ),
                    )));
                }
                Err(e) => {
                    return Err(RuleError::Io(e));
                }
            }
        }

        // Remove legacy ESLint config files
        let old_configs = [".eslintrc", ".eslintrc.js", ".eslintrc.json", ".eslintrc.yml", ".eslintrc.yaml", "eslint.config.js"];
        for old_config in old_configs {
            let old_path = parent_dir.join(old_config);
            if old_path.exists() {
                std::fs::remove_file(&old_path)?;
                fixed += 1;
            }
        }

        // Create or update eslint.config.mjs
        let eslint_config_path = parent_dir.join("eslint.config.mjs");
        let expected_content = self.get_eslint_config_content();

        let needs_update = if eslint_config_path.exists() {
            let current_content = context.read_file(&eslint_config_path)?;
            // Check if current content differs from expected
            current_content.trim() != expected_content.trim()
        } else {
            true
        };

        if needs_update {
            context.write_file(&eslint_config_path, &expected_content)?;
            fixed += 1;
        }

        Ok(fixed)
    }
}

impl Default for EslintConfigAgentRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for EslintConfigAgentRule {
    fn id(&self) -> &'static str {
        "eslint-config-agent"
    }

    fn name(&self) -> &'static str {
        "ESLint Config Agent"
    }

    fn description(&self) -> &'static str {
        "Ensures projects use eslint-config-agent as the only ESLint configuration without any overrides"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn checks(&self) -> Vec<CheckEntry> {
        vec![
            CheckEntry::new(
                CHECK_DEPENDENCY_EXISTS,
                "Verify eslint-config-agent is in devDependencies",
            ),
            CheckEntry::new(
                CHECK_CONFIG_FILE_EXISTS,
                "Verify eslint.config.mjs file exists",
            ),
            CheckEntry::new(
                CHECK_CONFIG_USES_AGENT,
                "Verify eslint.config.mjs imports from eslint-config-agent",
            ),
            CheckEntry::new(
                CHECK_NO_OVERRIDES,
                "Verify eslint.config.mjs has no custom overrides or rules",
            ),
            CheckEntry::new(
                CHECK_NO_LEGACY_CONFIG,
                "Verify no legacy ESLint config files exist (.eslintrc, etc.)",
            ),
        ]
    }

    fn fixes(&self) -> Vec<FixEntry> {
        vec![
            FixEntry::new(
                FIX_INSTALL_DEPENDENCY,
                "Install eslint-config-agent@latest via pnpm",
                vec![CHECK_DEPENDENCY_EXISTS],
            ),
            FixEntry::new(
                FIX_CREATE_CONFIG,
                "Create or update eslint.config.mjs to use eslint-config-agent as the only config",
                vec![CHECK_CONFIG_FILE_EXISTS, CHECK_CONFIG_USES_AGENT, CHECK_NO_OVERRIDES],
            ),
            FixEntry::new(
                FIX_REMOVE_LEGACY,
                "Remove legacy ESLint config files (.eslintrc, .eslintrc.js, etc.)",
                vec![CHECK_NO_LEGACY_CONFIG],
            ),
        ]
    }

    fn check(&self, context: &RuleContext) -> Vec<LintResult> {
        let mut results = Vec::new();

        // Find all package.json files
        let package_jsons = self.find_package_jsons(&context.root);

        for package_json in package_jsons {
            results.extend(self.check_package_json(&package_json));
        }

        results
    }

    fn fix(&self, context: &RuleContext) -> Result<u32, RuleError> {
        let mut fixed = 0;

        // Find all package.json files
        let package_jsons = self.find_package_jsons(&context.root);

        for package_json in package_jsons {
            fixed += self.fix_package(&package_json, context)?;
        }

        Ok(fixed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_context(root: PathBuf) -> RuleContext {
        RuleContext::new(root, true, serde_json::json!({}))
    }

    #[test]
    fn test_detects_missing_eslint_config_agent() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json without eslint-config-agent
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"eslint": "^8.0.0"}}"#,
        )
        .unwrap();

        let rule = EslintConfigAgentRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        assert!(results
            .iter()
            .any(|r| r.message.contains("Missing eslint-config-agent")));
    }

    #[test]
    fn test_detects_missing_eslint_config_mjs() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with eslint-config-agent
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"eslint-config-agent": "^1.0.0"}}"#,
        )
        .unwrap();

        let rule = EslintConfigAgentRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        assert!(results
            .iter()
            .any(|r| r.message.contains("Missing eslint.config.mjs")));
    }

    #[test]
    fn test_detects_eslint_config_not_using_agent() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with eslint-config-agent
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"eslint-config-agent": "^1.0.0"}}"#,
        )
        .unwrap();

        // Create eslint.config.mjs without eslint-config-agent
        fs::write(
            root.join("eslint.config.mjs"),
            r#"export default [];"#,
        )
        .unwrap();

        let rule = EslintConfigAgentRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        assert!(results
            .iter()
            .any(|r| r.message.contains("does not use eslint-config-agent")));
    }

    #[test]
    fn test_detects_custom_overrides() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with eslint-config-agent
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"eslint-config-agent": "^1.0.0"}}"#,
        )
        .unwrap();

        // Create eslint.config.mjs with overrides
        fs::write(
            root.join("eslint.config.mjs"),
            r#"
import config from "eslint-config-agent";

export default [
    ...config,
    {
        rules: {
            "no-console": "off"
        }
    }
];
"#,
        )
        .unwrap();

        let rule = EslintConfigAgentRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        assert!(results
            .iter()
            .any(|r| r.message.contains("custom overrides")));
    }

    #[test]
    fn test_detects_legacy_eslint_configs() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with eslint-config-agent
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"eslint-config-agent": "^1.0.0"}}"#,
        )
        .unwrap();

        // Create correct eslint.config.mjs
        fs::write(
            root.join("eslint.config.mjs"),
            r#"import config from "eslint-config-agent";
export default config;
"#,
        )
        .unwrap();

        // Create legacy .eslintrc.json
        fs::write(root.join(".eslintrc.json"), r#"{"extends": ["eslint:recommended"]}"#).unwrap();

        let rule = EslintConfigAgentRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        assert!(results
            .iter()
            .any(|r| r.message.contains("legacy ESLint config")));
    }

    #[test]
    fn test_accepts_correct_configuration() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with eslint-config-agent
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"eslint-config-agent": "^1.0.0"}}"#,
        )
        .unwrap();

        // Create correct eslint.config.mjs
        fs::write(
            root.join("eslint.config.mjs"),
            r#"import config from "eslint-config-agent";

export default config;
"#,
        )
        .unwrap();

        let rule = EslintConfigAgentRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        // Should have no errors
        assert!(results.is_empty());
    }

    #[test]
    fn test_skips_non_js_projects() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create minimal package.json without deps or scripts
        fs::write(root.join("package.json"), r#"{"name": "test"}"#).unwrap();

        let rule = EslintConfigAgentRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        // Should have no errors for non-JS projects
        assert!(results.is_empty());
    }

    #[test]
    fn test_skips_node_modules() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create main package.json with eslint-config-agent
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"eslint-config-agent": "^1.0.0"}}"#,
        )
        .unwrap();

        // Create correct eslint.config.mjs
        fs::write(
            root.join("eslint.config.mjs"),
            r#"import config from "eslint-config-agent";

export default config;
"#,
        )
        .unwrap();

        // Create node_modules with a package.json missing eslint-config-agent
        let node_modules = root.join("node_modules").join("some-package");
        fs::create_dir_all(&node_modules).unwrap();
        fs::write(
            node_modules.join("package.json"),
            r#"{"name": "some-package", "devDependencies": {"eslint": "^8.0.0"}}"#,
        )
        .unwrap();

        let rule = EslintConfigAgentRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        // Should not report errors from node_modules
        assert!(results.is_empty());
    }

    #[test]
    fn test_fix_creates_eslint_config_mjs() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with eslint-config-agent already installed
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"eslint-config-agent": "^1.0.0"}}"#,
        )
        .unwrap();

        let rule = EslintConfigAgentRule::new();
        let context = create_context(root.clone());
        let fixed = rule.fix(&context).unwrap();

        assert!(fixed >= 1);
        assert!(root.join("eslint.config.mjs").exists());

        let content = fs::read_to_string(root.join("eslint.config.mjs")).unwrap();
        assert!(content.contains("eslint-config-agent"));
        assert!(content.contains("export default config"));
    }

    #[test]
    fn test_fix_removes_legacy_configs() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with eslint-config-agent already installed
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"eslint-config-agent": "^1.0.0"}}"#,
        )
        .unwrap();

        // Create legacy config files
        fs::write(root.join(".eslintrc.json"), r#"{}"#).unwrap();
        fs::write(root.join(".eslintrc.js"), "module.exports = {}").unwrap();

        let rule = EslintConfigAgentRule::new();
        let context = create_context(root.clone());
        let fixed = rule.fix(&context).unwrap();

        assert!(fixed >= 2);
        assert!(!root.join(".eslintrc.json").exists());
        assert!(!root.join(".eslintrc.js").exists());
        assert!(root.join("eslint.config.mjs").exists());
    }

    #[test]
    fn test_fix_updates_incorrect_eslint_config() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with eslint-config-agent already installed
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"eslint-config-agent": "^1.0.0"}}"#,
        )
        .unwrap();

        // Create incorrect eslint.config.mjs
        fs::write(
            root.join("eslint.config.mjs"),
            r#"export default [];"#,
        )
        .unwrap();

        let rule = EslintConfigAgentRule::new();
        let context = create_context(root.clone());
        let fixed = rule.fix(&context).unwrap();

        assert!(fixed >= 1);

        let content = fs::read_to_string(root.join("eslint.config.mjs")).unwrap();
        assert!(content.contains("eslint-config-agent"));
        assert!(content.contains("export default config"));
    }
}
