use crate::rules::{Rule, RuleError};
use crate::types::{CheckEntry, FixEntry, LintResult, RuleContext, Severity};
use serde_json::Value;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// Check IDs
const CHECK_YARN_LOCK_EXISTS: &str = "yarn-lock-exists";
const CHECK_PACKAGE_LOCK_EXISTS: &str = "package-lock-exists";
const CHECK_PACKAGE_MANAGER_FIELD: &str = "package-manager-field";
const CHECK_PNPM_SETUP: &str = "pnpm-setup";
const CHECK_SCRIPTS_NPM: &str = "scripts-use-npm";
const CHECK_SCRIPTS_YARN: &str = "scripts-use-yarn";
const CHECK_ENGINES_NPM: &str = "engines-npm";
const CHECK_ENGINES_YARN: &str = "engines-yarn";

// Fix IDs
const FIX_REMOVE_YARN_LOCK: &str = "remove-yarn-lock";
const FIX_REMOVE_PACKAGE_LOCK: &str = "remove-package-lock";
const FIX_UPDATE_PACKAGE_MANAGER: &str = "update-package-manager";

/// Rule: Ensure projects use pnpm instead of npm or yarn
pub struct PnpmUsageRule;

impl PnpmUsageRule {
    pub fn new() -> Self {
        Self
    }

    /// Find all package.json files in the given root
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

    /// Check a single package.json and its surrounding files for pnpm compliance
    fn check_package_json(&self, package_json_path: &Path) -> Vec<LintResult> {
        let mut results = Vec::new();
        let parent_dir = package_json_path.parent().unwrap_or(Path::new("."));

        // Check for yarn.lock (indicates yarn usage)
        let yarn_lock = parent_dir.join("yarn.lock");
        if yarn_lock.exists() {
            results.push(LintResult::new(
                self.id(),
                CHECK_YARN_LOCK_EXISTS,
                self.default_severity(),
                "Found yarn.lock - project appears to use yarn instead of pnpm".into(),
                yarn_lock,
                None,
                Some("Remove yarn.lock and use 'pnpm install' to generate pnpm-lock.yaml".into()),
                vec![FIX_REMOVE_YARN_LOCK],
            ));
        }

        // Check for package-lock.json (indicates npm usage)
        let package_lock = parent_dir.join("package-lock.json");
        if package_lock.exists() {
            results.push(LintResult::new(
                self.id(),
                CHECK_PACKAGE_LOCK_EXISTS,
                self.default_severity(),
                "Found package-lock.json - project appears to use npm instead of pnpm".into(),
                package_lock,
                None,
                Some(
                    "Remove package-lock.json and use 'pnpm install' to generate pnpm-lock.yaml"
                        .into(),
                ),
                vec![FIX_REMOVE_PACKAGE_LOCK],
            ));
        }

        // Check for pnpm-lock.yaml (good sign, but let's validate package.json too)
        let pnpm_lock = parent_dir.join("pnpm-lock.yaml");
        let has_pnpm_lock = pnpm_lock.exists();

        // Parse and check package.json content
        match std::fs::read_to_string(package_json_path) {
            Ok(content) => match serde_json::from_str::<Value>(&content) {
                Ok(json) => {
                    // Check packageManager field
                    if let Some(pkg_manager) = json.get("packageManager").and_then(|v| v.as_str()) {
                        if !pkg_manager.starts_with("pnpm@") {
                            results.push(LintResult::new(
                                self.id(),
                                CHECK_PACKAGE_MANAGER_FIELD,
                                self.default_severity(),
                                format!(
                                    "packageManager is set to '{}' instead of pnpm",
                                    pkg_manager
                                ),
                                package_json_path.to_path_buf(),
                                None,
                                Some(
                                    "Change packageManager to 'pnpm@<version>' (e.g., 'pnpm@9.0.0')"
                                        .into(),
                                ),
                                vec![FIX_UPDATE_PACKAGE_MANAGER],
                            ));
                        }
                    } else if !has_pnpm_lock {
                        // No packageManager field and no pnpm-lock.yaml - warn about missing pnpm setup
                        results.push(LintResult::new(
                            self.id(),
                            CHECK_PNPM_SETUP,
                            Severity::Warning,
                            "No packageManager field and no pnpm-lock.yaml found".into(),
                            package_json_path.to_path_buf(),
                            None,
                            Some(
                                "Add 'packageManager' field with pnpm version or run 'pnpm install'"
                                    .into(),
                            ),
                            vec![], // Manual setup required
                        ));
                    }

                    // Check for scripts using npm or yarn directly
                    if let Some(scripts) = json.get("scripts").and_then(|s| s.as_object()) {
                        for (script_name, script_value) in scripts {
                            if let Some(script_cmd) = script_value.as_str() {
                                if script_cmd.contains("npm ")
                                    || script_cmd.starts_with("npm ")
                                    || script_cmd.contains(" npm")
                                {
                                    results.push(LintResult::new(
                                        self.id(),
                                        CHECK_SCRIPTS_NPM,
                                        Severity::Warning,
                                        format!(
                                            "Script '{}' uses npm command - consider using pnpm",
                                            script_name
                                        ),
                                        package_json_path.to_path_buf(),
                                        None,
                                        Some("Replace 'npm' with 'pnpm' in script commands".into()),
                                        vec![], // Manual fix required
                                    ));
                                }
                                if script_cmd.contains("yarn ")
                                    || script_cmd.starts_with("yarn ")
                                    || script_cmd.contains(" yarn")
                                {
                                    results.push(LintResult::new(
                                        self.id(),
                                        CHECK_SCRIPTS_YARN,
                                        Severity::Warning,
                                        format!(
                                            "Script '{}' uses yarn command - consider using pnpm",
                                            script_name
                                        ),
                                        package_json_path.to_path_buf(),
                                        None,
                                        Some("Replace 'yarn' with 'pnpm' in script commands".into()),
                                        vec![], // Manual fix required
                                    ));
                                }
                            }
                        }
                    }

                    // Check engines field for npm/yarn requirements
                    if let Some(engines) = json.get("engines").and_then(|e| e.as_object()) {
                        if engines.contains_key("npm") {
                            results.push(LintResult::new(
                                self.id(),
                                CHECK_ENGINES_NPM,
                                Severity::Warning,
                                "engines.npm field found - suggests npm dependency".into(),
                                package_json_path.to_path_buf(),
                                None,
                                Some(
                                    "Consider removing engines.npm and adding engines.pnpm instead"
                                        .into(),
                                ),
                                vec![], // Manual fix required
                            ));
                        }
                        if engines.contains_key("yarn") {
                            results.push(LintResult::new(
                                self.id(),
                                CHECK_ENGINES_YARN,
                                Severity::Warning,
                                "engines.yarn field found - suggests yarn dependency".into(),
                                package_json_path.to_path_buf(),
                                None,
                                Some(
                                    "Consider removing engines.yarn and adding engines.pnpm instead"
                                        .into(),
                                ),
                                vec![], // Manual fix required
                            ));
                        }
                    }
                }
                Err(e) => {
                    results.push(LintResult::new(
                        self.id(),
                        CHECK_PACKAGE_MANAGER_FIELD,
                        Severity::Error,
                        format!("Invalid JSON in package.json: {}", e),
                        package_json_path.to_path_buf(),
                        None,
                        Some("Fix JSON syntax errors".into()),
                        vec![],
                    ));
                }
            },
            Err(e) => {
                results.push(LintResult::new(
                    self.id(),
                    CHECK_PACKAGE_MANAGER_FIELD,
                    Severity::Error,
                    format!("Cannot read package.json: {}", e),
                    package_json_path.to_path_buf(),
                    None,
                    None,
                    vec![],
                ));
            }
        }

        results
    }

    /// Remove non-pnpm lock files
    fn remove_lock_files(&self, parent_dir: &Path) -> std::io::Result<u32> {
        let mut removed = 0;

        let yarn_lock = parent_dir.join("yarn.lock");
        if yarn_lock.exists() {
            std::fs::remove_file(&yarn_lock)?;
            removed += 1;
        }

        let package_lock = parent_dir.join("package-lock.json");
        if package_lock.exists() {
            std::fs::remove_file(&package_lock)?;
            removed += 1;
        }

        Ok(removed)
    }

    /// Update packageManager field in package.json if it's set to npm or yarn
    fn fix_package_manager_field(
        &self,
        package_json_path: &Path,
        context: &RuleContext,
    ) -> Result<bool, RuleError> {
        let content = context.read_file(package_json_path)?;
        let mut json: Value = serde_json::from_str(&content)?;

        let mut changed = false;

        if let Some(pkg_manager) = json.get("packageManager").and_then(|v| v.as_str()) {
            if pkg_manager.starts_with("npm@") || pkg_manager.starts_with("yarn@") {
                // Extract version pattern and suggest equivalent pnpm version
                json["packageManager"] = Value::String("pnpm@9.0.0".to_string());
                changed = true;
            }
        }

        if changed {
            let updated_content = serde_json::to_string_pretty(&json)?;
            context.write_file(package_json_path, &updated_content)?;
        }

        Ok(changed)
    }
}

impl Default for PnpmUsageRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for PnpmUsageRule {
    fn id(&self) -> &'static str {
        "pnpm-usage"
    }

    fn name(&self) -> &'static str {
        "Pnpm Usage Validation"
    }

    fn description(&self) -> &'static str {
        "Ensures projects use pnpm instead of npm or yarn for package management"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn checks(&self) -> Vec<CheckEntry> {
        vec![
            CheckEntry::new(
                CHECK_YARN_LOCK_EXISTS,
                "Detect yarn.lock files indicating yarn usage",
            ),
            CheckEntry::new(
                CHECK_PACKAGE_LOCK_EXISTS,
                "Detect package-lock.json files indicating npm usage",
            ),
            CheckEntry::new(
                CHECK_PACKAGE_MANAGER_FIELD,
                "Verify packageManager field uses pnpm",
            ),
            CheckEntry::new(
                CHECK_PNPM_SETUP,
                "Verify pnpm is set up (packageManager or pnpm-lock.yaml)",
            ),
            CheckEntry::new(
                CHECK_SCRIPTS_NPM,
                "Detect scripts that use npm commands",
            ),
            CheckEntry::new(
                CHECK_SCRIPTS_YARN,
                "Detect scripts that use yarn commands",
            ),
            CheckEntry::new(
                CHECK_ENGINES_NPM,
                "Detect engines.npm field in package.json",
            ),
            CheckEntry::new(
                CHECK_ENGINES_YARN,
                "Detect engines.yarn field in package.json",
            ),
        ]
    }

    fn fixes(&self) -> Vec<FixEntry> {
        vec![
            FixEntry::new(
                FIX_REMOVE_YARN_LOCK,
                "Remove yarn.lock file",
                vec![CHECK_YARN_LOCK_EXISTS],
            ),
            FixEntry::new(
                FIX_REMOVE_PACKAGE_LOCK,
                "Remove package-lock.json file",
                vec![CHECK_PACKAGE_LOCK_EXISTS],
            ),
            FixEntry::new(
                FIX_UPDATE_PACKAGE_MANAGER,
                "Update packageManager field to use pnpm",
                vec![CHECK_PACKAGE_MANAGER_FIELD],
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
            let parent_dir = package_json.parent().unwrap_or(Path::new("."));

            // Remove non-pnpm lock files
            fixed += self.remove_lock_files(parent_dir)?;

            // Fix packageManager field if needed
            if self.fix_package_manager_field(&package_json, context)? {
                fixed += 1;
            }
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
    fn test_detects_yarn_lock() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "version": "1.0.0"}"#,
        )
        .unwrap();

        // Create yarn.lock
        fs::write(root.join("yarn.lock"), "# yarn lockfile v1").unwrap();

        let rule = PnpmUsageRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        assert!(results.iter().any(|r| r.message.contains("yarn.lock")));
    }

    #[test]
    fn test_detects_package_lock_json() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "version": "1.0.0"}"#,
        )
        .unwrap();

        // Create package-lock.json
        fs::write(
            root.join("package-lock.json"),
            r#"{"lockfileVersion": 2}"#,
        )
        .unwrap();

        let rule = PnpmUsageRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        assert!(results
            .iter()
            .any(|r| r.message.contains("package-lock.json")));
    }

    #[test]
    fn test_detects_wrong_package_manager_field() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with npm as packageManager
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "packageManager": "npm@10.0.0"}"#,
        )
        .unwrap();

        let rule = PnpmUsageRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        assert!(results
            .iter()
            .any(|r| r.message.contains("packageManager is set to")));
    }

    #[test]
    fn test_accepts_pnpm_package_manager() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with pnpm as packageManager
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "packageManager": "pnpm@9.0.0"}"#,
        )
        .unwrap();

        // Create pnpm-lock.yaml
        fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: 9").unwrap();

        let rule = PnpmUsageRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        // Should have no errors
        assert!(results.is_empty());
    }

    #[test]
    fn test_detects_npm_in_scripts() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with npm in scripts
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "packageManager": "pnpm@9.0.0", "scripts": {"publish": "npm publish"}}"#,
        )
        .unwrap();

        // Create pnpm-lock.yaml
        fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: 9").unwrap();

        let rule = PnpmUsageRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        assert!(results.iter().any(|r| r.message.contains("uses npm command")));
    }

    #[test]
    fn test_detects_engines_npm() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with engines.npm
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "packageManager": "pnpm@9.0.0", "engines": {"npm": ">=8.0.0"}}"#,
        )
        .unwrap();

        // Create pnpm-lock.yaml
        fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: 9").unwrap();

        let rule = PnpmUsageRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        assert!(results.iter().any(|r| r.message.contains("engines.npm")));
    }

    #[test]
    fn test_fix_removes_yarn_lock() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "packageManager": "pnpm@9.0.0"}"#,
        )
        .unwrap();

        // Create yarn.lock
        fs::write(root.join("yarn.lock"), "# yarn lockfile v1").unwrap();

        let rule = PnpmUsageRule::new();
        let context = create_context(root.clone());
        let fixed = rule.fix(&context).unwrap();

        assert!(fixed >= 1);
        assert!(!root.join("yarn.lock").exists());
    }

    #[test]
    fn test_fix_removes_package_lock_json() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "packageManager": "pnpm@9.0.0"}"#,
        )
        .unwrap();

        // Create package-lock.json
        fs::write(
            root.join("package-lock.json"),
            r#"{"lockfileVersion": 2}"#,
        )
        .unwrap();

        let rule = PnpmUsageRule::new();
        let context = create_context(root.clone());
        let fixed = rule.fix(&context).unwrap();

        assert!(fixed >= 1);
        assert!(!root.join("package-lock.json").exists());
    }

    #[test]
    fn test_fix_updates_package_manager_field() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with npm as packageManager
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "packageManager": "npm@10.0.0"}"#,
        )
        .unwrap();

        let rule = PnpmUsageRule::new();
        let context = create_context(root.clone());
        let fixed = rule.fix(&context).unwrap();

        assert!(fixed >= 1);

        let content: Value =
            serde_json::from_str(&fs::read_to_string(root.join("package.json")).unwrap()).unwrap();
        assert!(content["packageManager"]
            .as_str()
            .unwrap()
            .starts_with("pnpm@"));
    }

    #[test]
    fn test_skips_node_modules() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create main package.json
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "packageManager": "pnpm@9.0.0"}"#,
        )
        .unwrap();

        // Create pnpm-lock.yaml
        fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: 9").unwrap();

        // Create node_modules with a package.json using npm
        let node_modules = root.join("node_modules").join("some-package");
        fs::create_dir_all(&node_modules).unwrap();
        fs::write(
            node_modules.join("package.json"),
            r#"{"name": "some-package", "packageManager": "npm@10.0.0"}"#,
        )
        .unwrap();

        let rule = PnpmUsageRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        // Should not report errors from node_modules
        assert!(results.is_empty());
    }

    #[test]
    fn test_warns_when_no_pnpm_setup() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json without packageManager and no lock files
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "version": "1.0.0"}"#,
        )
        .unwrap();

        let rule = PnpmUsageRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        assert!(results
            .iter()
            .any(|r| r.message.contains("No packageManager field")));
    }
}
