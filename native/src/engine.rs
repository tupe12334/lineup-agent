use crate::rules::RuleRegistry;
use crate::types::{Config, LintReport, LintResult, RuleContext, RuleInfo};
use std::path::PathBuf;

/// Error type for engine operations
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Path does not exist: {0}")]
    PathNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Rule execution engine
pub struct Runner {
    config: Config,
    registry: RuleRegistry,
}

impl Runner {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            registry: RuleRegistry::new(),
        }
    }

    /// Run all enabled rules on the specified path
    pub fn run(&self, path: &str) -> Result<LintReport, EngineError> {
        self.run_internal(path, false)
    }

    /// Run all enabled rules and apply fixes
    pub fn run_with_fix(&self, path: &str) -> Result<LintReport, EngineError> {
        self.run_internal(path, true)
    }

    fn run_internal(&self, path: &str, fix_mode: bool) -> Result<LintReport, EngineError> {
        let root = PathBuf::from(path);
        if !root.exists() {
            return Err(EngineError::PathNotFound(path.to_string()));
        }

        let mut all_results: Vec<LintResult> = Vec::new();
        let mut total_fixed: u32 = 0;

        for rule in self.registry.all() {
            // Check if rule is enabled
            let rule_config = self.config.rules.get(rule.id());
            let enabled = rule_config.map(|c| c.enabled).unwrap_or(true);

            if !enabled {
                continue;
            }

            let options = rule_config
                .map(|c| c.options.clone())
                .unwrap_or(serde_json::Value::Null);

            let context = RuleContext::new(root.clone(), fix_mode, options);

            // Run the rule check
            let results = rule.check(&context);
            all_results.extend(results);

            // Apply fixes if in fix mode and rule supports it
            if fix_mode && rule.can_fix() {
                if let Ok(fixed) = rule.fix(&context) {
                    total_fixed += fixed;
                }
            }
        }

        Ok(LintReport::new(all_results, total_fixed))
    }

    /// List all available rules
    pub fn list_rules(&self) -> Vec<RuleInfo> {
        self.registry.all().iter().map(|r| r.info()).collect()
    }
}
