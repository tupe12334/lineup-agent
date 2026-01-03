use crate::rules::{Rule, RuleError};
use crate::types::{CheckEntry, FixEntry, LintResult, RuleContext, Severity};
use serde_json::{json, Value};
use std::path::Path;
use walkdir::WalkDir;

// Check IDs
const CHECK_CLAUDE_DIR_EXISTS: &str = "claude-dir-exists";
const CHECK_SETTINGS_FILE_EXISTS: &str = "settings-file-exists";
const CHECK_HOOKS_OBJECT_EXISTS: &str = "hooks-object-exists";
const CHECK_PRE_TOOL_USE_EXISTS: &str = "pre-tool-use-exists";
const CHECK_BASH_MATCHER_EXISTS: &str = "bash-matcher-exists";

// Fix IDs
const FIX_CREATE_SETTINGS: &str = "create-settings";
const FIX_MERGE_HOOKS: &str = "merge-hooks";

/// Rule: Ensure all git repositories have .claude/settings.json with required hooks
pub struct ClaudeSettingsRule;

impl ClaudeSettingsRule {
    pub fn new() -> Self {
        Self
    }

    /// Find all .git directories in the given root (each represents a git repository)
    fn find_git_repos(&self, root: &Path) -> Vec<std::path::PathBuf> {
        let mut repos = Vec::new();

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_dir() && path.file_name().is_some_and(|n| n == ".git") {
                // Return the parent directory (the repo root), not the .git folder itself
                if let Some(parent) = path.parent() {
                    repos.push(parent.to_path_buf());
                }
            }
        }

        repos
    }

    /// Check if a git repository has proper .claude/settings.json configuration
    fn check_repo(&self, repo_root: &Path) -> Vec<LintResult> {
        let mut results = Vec::new();
        let claude_dir = repo_root.join(".claude");
        let settings_path = claude_dir.join("settings.json");

        // Check if .claude directory exists
        if !claude_dir.exists() {
            results.push(LintResult::new(
                self.id(),
                CHECK_CLAUDE_DIR_EXISTS,
                self.default_severity(),
                "Missing .claude directory in git repository".into(),
                repo_root.to_path_buf(),
                None,
                Some("Create .claude/settings.json with required hooks configuration".into()),
                vec![FIX_CREATE_SETTINGS],
            ));
            return results;
        }

        // Check if settings.json exists
        if !settings_path.exists() {
            results.push(LintResult::new(
                self.id(),
                CHECK_SETTINGS_FILE_EXISTS,
                self.default_severity(),
                "Missing settings.json in .claude directory".into(),
                claude_dir.to_path_buf(),
                None,
                Some("Create settings.json with required hooks configuration".into()),
                vec![FIX_CREATE_SETTINGS],
            ));
            return results;
        }

        // Validate the settings file content
        self.check_settings_content(&settings_path)
    }

    /// Check if the settings.json has the required hooks configuration
    fn check_settings_content(&self, path: &Path) -> Vec<LintResult> {
        let mut results = Vec::new();

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
                                        CHECK_BASH_MATCHER_EXISTS,
                                        Severity::Warning,
                                        "PreToolUse hooks missing Bash matcher".into(),
                                        path.to_path_buf(),
                                        None,
                                        Some(
                                            "Add a Bash matcher hook to prevent dangerous commands"
                                                .into(),
                                        ),
                                        vec![FIX_MERGE_HOOKS],
                                    ));
                                }
                            }
                        } else {
                            results.push(LintResult::new(
                                self.id(),
                                CHECK_PRE_TOOL_USE_EXISTS,
                                Severity::Warning,
                                "Missing PreToolUse hook configuration".into(),
                                path.to_path_buf(),
                                None,
                                Some("Add PreToolUse hooks to validate tool usage".into()),
                                vec![FIX_MERGE_HOOKS],
                            ));
                        }
                    } else {
                        results.push(LintResult::new(
                            self.id(),
                            CHECK_HOOKS_OBJECT_EXISTS,
                            Severity::Error,
                            "Missing 'hooks' configuration object".into(),
                            path.to_path_buf(),
                            None,
                            Some("Add 'hooks' object with required hook configurations".into()),
                            vec![FIX_MERGE_HOOKS],
                        ));
                    }
                }
                Err(e) => {
                    results.push(LintResult::new(
                        self.id(),
                        CHECK_SETTINGS_FILE_EXISTS,
                        Severity::Error,
                        format!("Invalid JSON: {}", e),
                        path.to_path_buf(),
                        None,
                        Some("Fix JSON syntax errors".into()),
                        vec![], // Cannot auto-fix invalid JSON
                    ));
                }
            },
            Err(e) => {
                results.push(LintResult::new(
                    self.id(),
                    CHECK_SETTINGS_FILE_EXISTS,
                    Severity::Error,
                    format!("Cannot read file: {}", e),
                    path.to_path_buf(),
                    None,
                    None,
                    vec![], // Cannot auto-fix read errors
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

    /// Deep merge hooks into existing settings, returns true if changes were made
    fn deep_merge_hooks(&self, existing: &mut Value) -> bool {
        let required_hook = self.get_required_bash_hook();
        let mut changes_made = false;

        // Ensure "hooks" object exists
        if !existing.get("hooks").is_some() {
            existing["hooks"] = json!({});
            changes_made = true;
        }

        let hooks = existing.get_mut("hooks").unwrap();

        // Ensure "PreToolUse" array exists
        if !hooks.get("PreToolUse").is_some() {
            hooks["PreToolUse"] = json!([]);
            changes_made = true;
        }

        let pre_tool_use = hooks.get_mut("PreToolUse").unwrap();

        if let Some(arr) = pre_tool_use.as_array_mut() {
            // Check if a Bash matcher already exists
            let has_bash_hook = arr.iter().any(|item| {
                item.get("matcher")
                    .and_then(|m| m.as_str())
                    .is_some_and(|m| m == "Bash")
            });

            if !has_bash_hook {
                arr.push(required_hook);
                changes_made = true;
            }
        }

        changes_made
    }

    /// Get the required Bash hook configuration
    fn get_required_bash_hook(&self) -> Value {
        json!({
            "matcher": "Bash",
            "hooks": [
                {
                    "type": "command",
                    "command": "INPUT=$(cat); if echo \"$INPUT\" | grep -q 'git push' && echo \"$INPUT\" | grep -qE -- '--no-verify|-n[^a-z]'; then echo 'BLOCKED: --no-verify is not allowed on git push' >&2; exit 2; fi"
                }
            ]
        })
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
        "Ensures all git repositories have .claude/settings.json with required hooks configuration"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn checks(&self) -> Vec<CheckEntry> {
        vec![
            CheckEntry::new(
                CHECK_CLAUDE_DIR_EXISTS,
                "Verify .claude directory exists in git repositories",
            ),
            CheckEntry::new(
                CHECK_SETTINGS_FILE_EXISTS,
                "Verify settings.json file exists in .claude directory",
            ),
            CheckEntry::new(
                CHECK_HOOKS_OBJECT_EXISTS,
                "Verify 'hooks' configuration object exists in settings.json",
            ),
            CheckEntry::new(
                CHECK_PRE_TOOL_USE_EXISTS,
                "Verify PreToolUse hook array is configured",
            ),
            CheckEntry::new(
                CHECK_BASH_MATCHER_EXISTS,
                "Verify Bash matcher hook is present to prevent dangerous commands",
            ),
        ]
    }

    fn fixes(&self) -> Vec<FixEntry> {
        vec![
            FixEntry::new(
                FIX_CREATE_SETTINGS,
                "Create .claude/settings.json with default hooks configuration",
                vec![CHECK_CLAUDE_DIR_EXISTS, CHECK_SETTINGS_FILE_EXISTS],
            ),
            FixEntry::new(
                FIX_MERGE_HOOKS,
                "Deep merge required hooks into existing settings.json",
                vec![
                    CHECK_HOOKS_OBJECT_EXISTS,
                    CHECK_PRE_TOOL_USE_EXISTS,
                    CHECK_BASH_MATCHER_EXISTS,
                ],
            ),
        ]
    }

    fn check(&self, context: &RuleContext) -> Vec<LintResult> {
        let mut results = Vec::new();

        // Find all git repositories
        let repos = self.find_git_repos(&context.root);

        for repo in repos {
            results.extend(self.check_repo(&repo));
        }

        results
    }

    fn fix(&self, context: &RuleContext) -> Result<u32, RuleError> {
        let mut fixed = 0;

        // Find all git repositories
        let repos = self.find_git_repos(&context.root);

        for repo in repos {
            let claude_dir = repo.join(".claude");
            let settings_path = claude_dir.join("settings.json");

            if !settings_path.exists() {
                // Create the .claude directory and settings.json file
                // write_file handles creating parent directories
                context.write_file(&settings_path, &self.default_settings_content())?;
                fixed += 1;
            } else {
                // File exists - deep merge to add missing hooks without overriding existing content
                if let Ok(content) = context.read_file(&settings_path) {
                    if let Ok(mut existing) = serde_json::from_str::<Value>(&content) {
                        if self.deep_merge_hooks(&mut existing) {
                            let merged_content = serde_json::to_string_pretty(&existing)?;
                            context.write_file(&settings_path, &merged_content)?;
                            fixed += 1;
                        }
                    }
                }
            }
        }

        Ok(fixed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup_git_repo(temp_dir: &TempDir) -> PathBuf {
        let repo_root = temp_dir.path().to_path_buf();
        fs::create_dir_all(repo_root.join(".git")).unwrap();
        repo_root
    }

    fn create_context(root: PathBuf) -> RuleContext {
        RuleContext::new(root, true, serde_json::json!({}))
    }

    #[test]
    fn test_fix_creates_new_file_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = setup_git_repo(&temp_dir);
        let rule = ClaudeSettingsRule::new();
        let context = create_context(repo_root.clone());

        let fixed = rule.fix(&context).unwrap();

        assert_eq!(fixed, 1);
        let settings_path = repo_root.join(".claude/settings.json");
        assert!(settings_path.exists());

        let content: Value = serde_json::from_str(&fs::read_to_string(&settings_path).unwrap()).unwrap();
        assert!(content.get("hooks").is_some());
        assert!(content["hooks"].get("PreToolUse").is_some());
    }

    #[test]
    fn test_fix_deep_merges_existing_file_without_hooks() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = setup_git_repo(&temp_dir);
        let claude_dir = repo_root.join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();

        // Create existing settings with other config but no hooks
        let existing_settings = json!({
            "apiKey": "test-key",
            "model": "claude-3",
            "customSetting": {
                "nested": "value"
            }
        });
        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&existing_settings).unwrap(),
        )
        .unwrap();

        let rule = ClaudeSettingsRule::new();
        let context = create_context(repo_root.clone());

        let fixed = rule.fix(&context).unwrap();

        assert_eq!(fixed, 1);

        let content: Value = serde_json::from_str(
            &fs::read_to_string(claude_dir.join("settings.json")).unwrap(),
        )
        .unwrap();

        // Verify existing settings are preserved
        assert_eq!(content["apiKey"], "test-key");
        assert_eq!(content["model"], "claude-3");
        assert_eq!(content["customSetting"]["nested"], "value");

        // Verify hooks were added
        assert!(content.get("hooks").is_some());
        assert!(content["hooks"].get("PreToolUse").is_some());
        let pre_tool_use = content["hooks"]["PreToolUse"].as_array().unwrap();
        assert!(pre_tool_use.iter().any(|h| h["matcher"] == "Bash"));
    }

    #[test]
    fn test_fix_deep_merges_existing_file_with_other_hooks() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = setup_git_repo(&temp_dir);
        let claude_dir = repo_root.join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();

        // Create existing settings with hooks but different matcher
        let existing_settings = json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Write",
                        "hooks": [{"type": "command", "command": "echo 'write hook'"}]
                    }
                ],
                "PostToolUse": [
                    {
                        "matcher": "Read",
                        "hooks": [{"type": "command", "command": "echo 'read hook'"}]
                    }
                ]
            }
        });
        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&existing_settings).unwrap(),
        )
        .unwrap();

        let rule = ClaudeSettingsRule::new();
        let context = create_context(repo_root.clone());

        let fixed = rule.fix(&context).unwrap();

        assert_eq!(fixed, 1);

        let content: Value = serde_json::from_str(
            &fs::read_to_string(claude_dir.join("settings.json")).unwrap(),
        )
        .unwrap();

        // Verify existing hooks are preserved
        let pre_tool_use = content["hooks"]["PreToolUse"].as_array().unwrap();
        assert!(pre_tool_use.iter().any(|h| h["matcher"] == "Write"));

        // Verify PostToolUse hook is preserved
        assert!(content["hooks"].get("PostToolUse").is_some());
        let post_tool_use = content["hooks"]["PostToolUse"].as_array().unwrap();
        assert!(post_tool_use.iter().any(|h| h["matcher"] == "Read"));

        // Verify Bash hook was added
        assert!(pre_tool_use.iter().any(|h| h["matcher"] == "Bash"));
    }

    #[test]
    fn test_fix_does_not_duplicate_existing_bash_hook() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = setup_git_repo(&temp_dir);
        let claude_dir = repo_root.join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();

        // Create existing settings with Bash hook already present
        let existing_settings = json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [{"type": "command", "command": "existing bash command"}]
                    }
                ]
            }
        });
        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&existing_settings).unwrap(),
        )
        .unwrap();

        let rule = ClaudeSettingsRule::new();
        let context = create_context(repo_root.clone());

        let fixed = rule.fix(&context).unwrap();

        // No changes should be made since Bash hook already exists
        assert_eq!(fixed, 0);

        let content: Value = serde_json::from_str(
            &fs::read_to_string(claude_dir.join("settings.json")).unwrap(),
        )
        .unwrap();

        // Verify only one Bash hook exists and it's the original one
        let pre_tool_use = content["hooks"]["PreToolUse"].as_array().unwrap();
        let bash_hooks: Vec<_> = pre_tool_use
            .iter()
            .filter(|h| h["matcher"] == "Bash")
            .collect();
        assert_eq!(bash_hooks.len(), 1);
        assert_eq!(
            bash_hooks[0]["hooks"][0]["command"],
            "existing bash command"
        );
    }

    #[test]
    fn test_fix_preserves_complex_nested_structure() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = setup_git_repo(&temp_dir);
        let claude_dir = repo_root.join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();

        // Create existing settings with complex nested structure
        let existing_settings = json!({
            "apiKey": "secret-key",
            "permissions": {
                "allow": ["read", "write"],
                "deny": ["execute"],
                "advanced": {
                    "level1": {
                        "level2": {
                            "deepValue": true
                        }
                    }
                }
            },
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Edit",
                        "hooks": [{"type": "command", "command": "lint check"}]
                    }
                ]
            },
            "arrayConfig": [1, 2, {"nested": "object"}]
        });
        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&existing_settings).unwrap(),
        )
        .unwrap();

        let rule = ClaudeSettingsRule::new();
        let context = create_context(repo_root.clone());

        let fixed = rule.fix(&context).unwrap();

        assert_eq!(fixed, 1);

        let content: Value = serde_json::from_str(
            &fs::read_to_string(claude_dir.join("settings.json")).unwrap(),
        )
        .unwrap();

        // Verify all original settings are preserved
        assert_eq!(content["apiKey"], "secret-key");
        assert_eq!(content["permissions"]["allow"][0], "read");
        assert_eq!(content["permissions"]["deny"][0], "execute");
        assert_eq!(
            content["permissions"]["advanced"]["level1"]["level2"]["deepValue"],
            true
        );
        assert_eq!(content["arrayConfig"][2]["nested"], "object");

        // Verify Edit hook is preserved
        let pre_tool_use = content["hooks"]["PreToolUse"].as_array().unwrap();
        assert!(pre_tool_use.iter().any(|h| h["matcher"] == "Edit"));

        // Verify Bash hook was added
        assert!(pre_tool_use.iter().any(|h| h["matcher"] == "Bash"));
    }

    #[test]
    fn test_deep_merge_adds_hooks_to_empty_object() {
        let rule = ClaudeSettingsRule::new();
        let mut existing = json!({});

        let changed = rule.deep_merge_hooks(&mut existing);

        assert!(changed);
        assert!(existing.get("hooks").is_some());
        assert!(existing["hooks"].get("PreToolUse").is_some());
        let pre_tool_use = existing["hooks"]["PreToolUse"].as_array().unwrap();
        assert!(pre_tool_use.iter().any(|h| h["matcher"] == "Bash"));
    }

    #[test]
    fn test_deep_merge_adds_pre_tool_use_to_existing_hooks() {
        let rule = ClaudeSettingsRule::new();
        let mut existing = json!({
            "hooks": {
                "PostToolUse": []
            }
        });

        let changed = rule.deep_merge_hooks(&mut existing);

        assert!(changed);
        assert!(existing["hooks"].get("PreToolUse").is_some());
        assert!(existing["hooks"].get("PostToolUse").is_some());
    }

    #[test]
    fn test_deep_merge_returns_false_when_bash_hook_exists() {
        let rule = ClaudeSettingsRule::new();
        let mut existing = json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "Bash", "hooks": []}
                ]
            }
        });

        let changed = rule.deep_merge_hooks(&mut existing);

        assert!(!changed);
    }
}
