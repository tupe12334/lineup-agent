use crate::rules::{Rule, RuleError};
use crate::types::{LintResult, RuleContext, Severity};
use serde_json::{json, Value};
use std::path::Path;
use walkdir::WalkDir;

/// Rule: Ensure all .claude/ directories have settings.json with required hooks
pub struct ClaudeSettingsRule;

impl ClaudeSettingsRule {
    pub fn new() -> Self {
        Self
    }

    /// Find all .claude directories in the given root
    fn find_claude_dirs(&self, root: &Path) -> Vec<std::path::PathBuf> {
        let mut dirs = Vec::new();

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_dir() && path.file_name().is_some_and(|n| n == ".claude") {
                dirs.push(path.to_path_buf());
            }
        }

        dirs
    }

    /// Check if the settings.json has the required hooks configuration
    fn check_settings_file(&self, path: &Path) -> Vec<LintResult> {
        let mut results = Vec::new();

        // Check if settings.json exists
        if !path.exists() {
            results.push(LintResult::new(
                self.id(),
                self.default_severity(),
                format!(
                    "Missing settings.json in {}",
                    path.parent().unwrap().display()
                ),
                path.to_path_buf(),
                None,
                Some("Create settings.json with required hooks configuration".into()),
            ));
            return results;
        }

        // Parse and validate the settings file
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<Value>(&content) {
                Ok(json) => {
                    // Check for hooks configuration
                    if let Some(hooks) = json.get("hooks") {
                        // Check for PreToolUse hook
                        if let Some(pre_tool_use) = hooks.get("PreToolUse") {
                            // Check if it's an array with the Bash matcher
                            if let Some(arr) = pre_tool_use.as_array() {
                                let has_bash_hook = arr.iter().any(|item| {
                                    item.get("matcher")
                                        .and_then(|m| m.as_str())
                                        .is_some_and(|m| m == "Bash")
                                });

                                if !has_bash_hook {
                                    results.push(LintResult::new(
                                        self.id(),
                                        Severity::Warning,
                                        "PreToolUse hooks missing Bash matcher".into(),
                                        path.to_path_buf(),
                                        None,
                                        Some(
                                            "Add a Bash matcher hook to prevent dangerous commands"
                                                .into(),
                                        ),
                                    ));
                                }
                            }
                        } else {
                            results.push(LintResult::new(
                                self.id(),
                                Severity::Warning,
                                "Missing PreToolUse hook configuration".into(),
                                path.to_path_buf(),
                                None,
                                Some("Add PreToolUse hooks to validate tool usage".into()),
                            ));
                        }
                    } else {
                        results.push(LintResult::new(
                            self.id(),
                            Severity::Error,
                            "Missing 'hooks' configuration object".into(),
                            path.to_path_buf(),
                            None,
                            Some("Add 'hooks' object with required hook configurations".into()),
                        ));
                    }
                }
                Err(e) => {
                    results.push(LintResult::new(
                        self.id(),
                        Severity::Error,
                        format!("Invalid JSON: {}", e),
                        path.to_path_buf(),
                        None,
                        Some("Fix JSON syntax errors".into()),
                    ));
                }
            },
            Err(e) => {
                results.push(LintResult::new(
                    self.id(),
                    Severity::Error,
                    format!("Cannot read file: {}", e),
                    path.to_path_buf(),
                    None,
                    None,
                ));
            }
        }

        results
    }

    /// Generate the default settings content
    fn default_settings_content(&self) -> String {
        let settings = json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "INPUT=$(cat); if echo \"$INPUT\" | grep -q 'git push' && echo \"$INPUT\" | grep -qE -- '--no-verify|-n[^a-z]'; then echo 'BLOCKED: --no-verify is not allowed on git push' >&2; exit 2; fi"
                            }
                        ]
                    }
                ]
            }
        });
        serde_json::to_string_pretty(&settings).unwrap()
    }
}

impl Default for ClaudeSettingsRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for ClaudeSettingsRule {
    fn id(&self) -> &'static str {
        "claude-settings-hooks"
    }

    fn name(&self) -> &'static str {
        "Claude Settings Hooks"
    }

    fn description(&self) -> &'static str {
        "Ensures all .claude/ directories have settings.json with required hooks configuration"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, context: &RuleContext) -> Vec<LintResult> {
        let mut results = Vec::new();

        // Find all .claude directories
        let claude_dirs = self.find_claude_dirs(&context.root);

        for dir in claude_dirs {
            let settings_path = dir.join("settings.json");
            results.extend(self.check_settings_file(&settings_path));
        }

        results
    }

    fn can_fix(&self) -> bool {
        true
    }

    fn fix(&self, context: &RuleContext) -> Result<u32, RuleError> {
        let mut fixed = 0;

        let claude_dirs = self.find_claude_dirs(&context.root);

        for dir in claude_dirs {
            let settings_path = dir.join("settings.json");

            if !settings_path.exists() {
                // Create the settings file with default content
                context.write_file(&settings_path, &self.default_settings_content())?;
                fixed += 1;
            }
        }

        Ok(fixed)
    }
}
