use crate::rules::{Rule, RuleError};
use crate::types::{CheckEntry, FixEntry, LintResult, RuleContext, Severity};
use serde_json::Value;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// Check IDs
const CHECK_CSPELL_JSON_EXISTS: &str = "cspell-json-exists";
const CHECK_CSPELL_DEPENDENCY: &str = "cspell-dependency";
const CHECK_CSPELL_PRE_COMMIT: &str = "cspell-pre-commit-hook";

// Fix IDs
const FIX_CREATE_CSPELL_JSON: &str = "create-cspell-json";
const FIX_ADD_CSPELL_DEPENDENCY: &str = "add-cspell-dependency";
const FIX_ADD_CSPELL_PRE_COMMIT: &str = "add-cspell-pre-commit";

/// Rule: Ensure projects have cspell configured for spell checking
pub struct CspellConfigRule;

impl CspellConfigRule {
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

    /// Check a single project directory for cspell configuration
    fn check_project(&self, package_json_path: &Path) -> Vec<LintResult> {
        let mut results = Vec::new();
        let project_dir = package_json_path.parent().unwrap_or(Path::new("."));

        // Check 1: cspell.json exists
        let cspell_json = project_dir.join("cspell.json");
        let cspell_yaml = project_dir.join("cspell.yaml");
        let cspell_yml = project_dir.join("cspell.yml");
        let cspell_config_js = project_dir.join("cspell.config.js");
        let cspell_config_cjs = project_dir.join("cspell.config.cjs");

        let has_cspell_config = cspell_json.exists()
            || cspell_yaml.exists()
            || cspell_yml.exists()
            || cspell_config_js.exists()
            || cspell_config_cjs.exists();

        if !has_cspell_config {
            results.push(LintResult::new(
                self.id(),
                CHECK_CSPELL_JSON_EXISTS,
                self.default_severity(),
                "Missing cspell configuration file (cspell.json, cspell.yaml, or cspell.config.js)"
                    .into(),
                project_dir.to_path_buf(),
                None,
                Some("Create a cspell.json file to configure spell checking".into()),
                vec![FIX_CREATE_CSPELL_JSON],
            ));
        }

        // Check 2: cspell dependency in package.json
        match std::fs::read_to_string(package_json_path) {
            Ok(content) => match serde_json::from_str::<Value>(&content) {
                Ok(json) => {
                    let has_cspell_dep = self.has_cspell_dependency(&json);

                    if !has_cspell_dep {
                        results.push(LintResult::new(
                            self.id(),
                            CHECK_CSPELL_DEPENDENCY,
                            self.default_severity(),
                            "Missing cspell in devDependencies".into(),
                            package_json_path.to_path_buf(),
                            None,
                            Some("Add 'cspell' to devDependencies in package.json".into()),
                            vec![FIX_ADD_CSPELL_DEPENDENCY],
                        ));
                    }
                }
                Err(e) => {
                    results.push(LintResult::new(
                        self.id(),
                        CHECK_CSPELL_DEPENDENCY,
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
                    CHECK_CSPELL_DEPENDENCY,
                    Severity::Error,
                    format!("Cannot read package.json: {}", e),
                    package_json_path.to_path_buf(),
                    None,
                    None,
                    vec![],
                ));
            }
        }

        // Check 3: cspell in pre-commit hook
        let husky_pre_commit = project_dir.join(".husky").join("pre-commit");
        if husky_pre_commit.exists() {
            match std::fs::read_to_string(&husky_pre_commit) {
                Ok(content) => {
                    let has_cspell_hook = content.contains("cspell")
                        || content.contains("pnpm spell")
                        || content.contains("npm run spell")
                        || content.contains("yarn spell");

                    if !has_cspell_hook {
                        results.push(LintResult::new(
                            self.id(),
                            CHECK_CSPELL_PRE_COMMIT,
                            Severity::Warning,
                            "Pre-commit hook exists but does not include cspell check".into(),
                            husky_pre_commit.clone(),
                            None,
                            Some(
                                "Add 'pnpm exec cspell --no-progress' or similar to pre-commit hook"
                                    .into(),
                            ),
                            vec![FIX_ADD_CSPELL_PRE_COMMIT],
                        ));
                    }
                }
                Err(e) => {
                    results.push(LintResult::new(
                        self.id(),
                        CHECK_CSPELL_PRE_COMMIT,
                        Severity::Error,
                        format!("Cannot read pre-commit hook: {}", e),
                        husky_pre_commit,
                        None,
                        None,
                        vec![],
                    ));
                }
            }
        } else {
            // Pre-commit hook doesn't exist - check if .husky directory exists
            let husky_dir = project_dir.join(".husky");
            if husky_dir.exists() {
                results.push(LintResult::new(
                    self.id(),
                    CHECK_CSPELL_PRE_COMMIT,
                    Severity::Warning,
                    "Husky is configured but no pre-commit hook exists for cspell".into(),
                    husky_dir,
                    None,
                    Some("Create .husky/pre-commit with cspell check command".into()),
                    vec![FIX_ADD_CSPELL_PRE_COMMIT],
                ));
            }
        }

        results
    }

    /// Check if package.json has cspell as a dependency
    fn has_cspell_dependency(&self, json: &Value) -> bool {
        // Check devDependencies
        if let Some(dev_deps) = json.get("devDependencies").and_then(|d| d.as_object()) {
            if dev_deps.contains_key("cspell") {
                return true;
            }
        }

        // Check dependencies (less common but possible)
        if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
            if deps.contains_key("cspell") {
                return true;
            }
        }

        false
    }

    /// Create a basic cspell.json configuration file
    fn create_cspell_json(&self, project_dir: &Path) -> std::io::Result<bool> {
        let cspell_json_path = project_dir.join("cspell.json");

        if cspell_json_path.exists() {
            return Ok(false);
        }

        let default_config = serde_json::json!({
            "$schema": "https://raw.githubusercontent.com/streetsidesoftware/cspell/main/cspell.schema.json",
            "version": "0.2",
            "language": "en",
            "words": [],
            "ignorePaths": [
                "node_modules",
                "pnpm-lock.yaml",
                "package-lock.json",
                "yarn.lock",
                "dist",
                "build",
                "coverage",
                ".git"
            ]
        });

        let content = serde_json::to_string_pretty(&default_config)?;
        std::fs::write(&cspell_json_path, content)?;

        Ok(true)
    }

    /// Add cspell to devDependencies in package.json
    fn add_cspell_dependency(
        &self,
        package_json_path: &Path,
        context: &RuleContext,
    ) -> Result<bool, RuleError> {
        let content = context.read_file(package_json_path)?;
        let mut json: Value = serde_json::from_str(&content)?;

        // Check if already has cspell
        if self.has_cspell_dependency(&json) {
            return Ok(false);
        }

        // Ensure devDependencies exists
        if json.get("devDependencies").is_none() {
            json["devDependencies"] = serde_json::json!({});
        }

        // Add cspell
        if let Some(dev_deps) = json.get_mut("devDependencies").and_then(|d| d.as_object_mut()) {
            dev_deps.insert("cspell".to_string(), Value::String("^8.0.0".to_string()));
        }

        let updated_content = serde_json::to_string_pretty(&json)?;
        context.write_file(package_json_path, &updated_content)?;

        Ok(true)
    }

    /// Add cspell check to pre-commit hook
    fn add_cspell_pre_commit(&self, project_dir: &Path) -> std::io::Result<bool> {
        let husky_dir = project_dir.join(".husky");
        let pre_commit_path = husky_dir.join("pre-commit");

        if !husky_dir.exists() {
            // Can't add to non-existent husky directory
            return Ok(false);
        }

        let cspell_command = "pnpm exec cspell --no-progress \"**/*.{ts,tsx,js,jsx,md,json}\"";

        if pre_commit_path.exists() {
            // Append to existing pre-commit hook
            let content = std::fs::read_to_string(&pre_commit_path)?;

            if content.contains("cspell") {
                return Ok(false); // Already has cspell
            }

            let updated_content = format!("{}\n\n# Spell check\n{}\n", content.trim_end(), cspell_command);
            std::fs::write(&pre_commit_path, updated_content)?;
        } else {
            // Create new pre-commit hook
            let content = format!(
                "#!/usr/bin/env sh\n. \"$(dirname -- \"$0\")/_/husky.sh\"\n\n# Spell check\n{}\n",
                cspell_command
            );
            std::fs::write(&pre_commit_path, content)?;

            // Make executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&pre_commit_path)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&pre_commit_path, perms)?;
            }
        }

        Ok(true)
    }
}

impl Default for CspellConfigRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for CspellConfigRule {
    fn id(&self) -> &'static str {
        "cspell-config"
    }

    fn name(&self) -> &'static str {
        "CSpell Configuration"
    }

    fn description(&self) -> &'static str {
        "Ensures projects have cspell configured for spell checking with appropriate dependencies and pre-commit hooks"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn checks(&self) -> Vec<CheckEntry> {
        vec![
            CheckEntry::new(
                CHECK_CSPELL_JSON_EXISTS,
                "Verify cspell configuration file exists (cspell.json, cspell.yaml, etc.)",
            ),
            CheckEntry::new(
                CHECK_CSPELL_DEPENDENCY,
                "Verify cspell is in devDependencies",
            ),
            CheckEntry::new(
                CHECK_CSPELL_PRE_COMMIT,
                "Verify cspell check is in pre-commit hook",
            ),
        ]
    }

    fn fixes(&self) -> Vec<FixEntry> {
        vec![
            FixEntry::new(
                FIX_CREATE_CSPELL_JSON,
                "Create a default cspell.json configuration file",
                vec![CHECK_CSPELL_JSON_EXISTS],
            ),
            FixEntry::new(
                FIX_ADD_CSPELL_DEPENDENCY,
                "Add cspell to devDependencies in package.json",
                vec![CHECK_CSPELL_DEPENDENCY],
            ),
            FixEntry::new(
                FIX_ADD_CSPELL_PRE_COMMIT,
                "Add cspell check to pre-commit hook",
                vec![CHECK_CSPELL_PRE_COMMIT],
            ),
        ]
    }

    fn check(&self, context: &RuleContext) -> Vec<LintResult> {
        let mut results = Vec::new();

        // Find all package.json files
        let package_jsons = self.find_package_jsons(&context.root);

        for package_json in package_jsons {
            results.extend(self.check_project(&package_json));
        }

        results
    }

    fn fix(&self, context: &RuleContext) -> Result<u32, RuleError> {
        let mut fixed = 0;

        // Find all package.json files
        let package_jsons = self.find_package_jsons(&context.root);

        for package_json in package_jsons {
            let project_dir = package_json.parent().unwrap_or(Path::new("."));

            // Fix 1: Create cspell.json if missing
            if self.create_cspell_json(project_dir)? {
                fixed += 1;
            }

            // Fix 2: Add cspell dependency
            if self.add_cspell_dependency(&package_json, context)? {
                fixed += 1;
            }

            // Fix 3: Add cspell to pre-commit hook
            if self.add_cspell_pre_commit(project_dir)? {
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
    fn test_detects_missing_cspell_json() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json without cspell
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "version": "1.0.0"}"#,
        )
        .unwrap();

        let rule = CspellConfigRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        assert!(results
            .iter()
            .any(|r| r.check_id == CHECK_CSPELL_JSON_EXISTS));
    }

    #[test]
    fn test_detects_missing_cspell_dependency() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json without cspell in devDependencies
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"typescript": "^5.0.0"}}"#,
        )
        .unwrap();

        // Create cspell.json
        fs::write(root.join("cspell.json"), r#"{"version": "0.2"}"#).unwrap();

        let rule = CspellConfigRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        assert!(results.iter().any(|r| r.check_id == CHECK_CSPELL_DEPENDENCY));
    }

    #[test]
    fn test_detects_missing_cspell_in_precommit() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with cspell
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"cspell": "^8.0.0"}}"#,
        )
        .unwrap();

        // Create cspell.json
        fs::write(root.join("cspell.json"), r#"{"version": "0.2"}"#).unwrap();

        // Create .husky directory with pre-commit but no cspell
        fs::create_dir_all(root.join(".husky")).unwrap();
        fs::write(
            root.join(".husky").join("pre-commit"),
            "#!/usr/bin/env sh\npnpm lint\n",
        )
        .unwrap();

        let rule = CspellConfigRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        assert!(results
            .iter()
            .any(|r| r.check_id == CHECK_CSPELL_PRE_COMMIT));
    }

    #[test]
    fn test_passes_when_all_configured() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with cspell
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"cspell": "^8.0.0"}}"#,
        )
        .unwrap();

        // Create cspell.json
        fs::write(root.join("cspell.json"), r#"{"version": "0.2"}"#).unwrap();

        // Create .husky directory with pre-commit including cspell
        fs::create_dir_all(root.join(".husky")).unwrap();
        fs::write(
            root.join(".husky").join("pre-commit"),
            "#!/usr/bin/env sh\npnpm exec cspell\n",
        )
        .unwrap();

        let rule = CspellConfigRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        // Should have no results
        assert!(results.is_empty());
    }

    #[test]
    fn test_accepts_cspell_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json with cspell
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"cspell": "^8.0.0"}}"#,
        )
        .unwrap();

        // Create cspell.yaml instead of cspell.json
        fs::write(root.join("cspell.yaml"), "version: '0.2'\n").unwrap();

        let rule = CspellConfigRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        // Should not report missing cspell config
        assert!(!results
            .iter()
            .any(|r| r.check_id == CHECK_CSPELL_JSON_EXISTS));
    }

    #[test]
    fn test_fix_creates_cspell_json() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {}}"#,
        )
        .unwrap();

        let rule = CspellConfigRule::new();
        let context = create_context(root.clone());
        let fixed = rule.fix(&context).unwrap();

        assert!(fixed >= 1);
        assert!(root.join("cspell.json").exists());

        // Verify the content is valid JSON
        let content = fs::read_to_string(root.join("cspell.json")).unwrap();
        let json: Value = serde_json::from_str(&content).unwrap();
        assert!(json.get("version").is_some());
    }

    #[test]
    fn test_fix_adds_cspell_dependency() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json without cspell
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"typescript": "^5.0.0"}}"#,
        )
        .unwrap();

        let rule = CspellConfigRule::new();
        let context = create_context(root.clone());
        let fixed = rule.fix(&context).unwrap();

        assert!(fixed >= 1);

        let content: Value =
            serde_json::from_str(&fs::read_to_string(root.join("package.json")).unwrap()).unwrap();
        assert!(content["devDependencies"]["cspell"].is_string());
    }

    #[test]
    fn test_fix_adds_cspell_to_precommit() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create package.json
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"cspell": "^8.0.0"}}"#,
        )
        .unwrap();

        // Create cspell.json
        fs::write(root.join("cspell.json"), r#"{"version": "0.2"}"#).unwrap();

        // Create .husky directory with pre-commit but no cspell
        fs::create_dir_all(root.join(".husky")).unwrap();
        fs::write(
            root.join(".husky").join("pre-commit"),
            "#!/usr/bin/env sh\npnpm lint\n",
        )
        .unwrap();

        let rule = CspellConfigRule::new();
        let context = create_context(root.clone());
        let fixed = rule.fix(&context).unwrap();

        assert!(fixed >= 1);

        let content = fs::read_to_string(root.join(".husky").join("pre-commit")).unwrap();
        assert!(content.contains("cspell"));
    }

    #[test]
    fn test_skips_node_modules() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create main package.json with cspell
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "devDependencies": {"cspell": "^8.0.0"}}"#,
        )
        .unwrap();

        // Create cspell.json
        fs::write(root.join("cspell.json"), r#"{"version": "0.2"}"#).unwrap();

        // Create node_modules with a package.json without cspell
        let node_modules = root.join("node_modules").join("some-package");
        fs::create_dir_all(&node_modules).unwrap();
        fs::write(
            node_modules.join("package.json"),
            r#"{"name": "some-package", "devDependencies": {}}"#,
        )
        .unwrap();

        let rule = CspellConfigRule::new();
        let context = create_context(root);
        let results = rule.check(&context);

        // Should not report errors from node_modules
        assert!(!results.iter().any(|r| r.path.contains("node_modules")));
    }
}
