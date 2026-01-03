pub mod claude_settings;
pub mod eslint_config_agent;
pub mod husky_init;
pub mod pnpm_usage;

use crate::types::{CheckEntry, FixEntry, LintResult, RuleContext, RuleInfo, Severity};
use std::collections::HashMap;
use std::sync::Arc;

/// Error type for rule operations
#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    #[error("Fix not supported for this rule")]
    FixNotSupported,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Core trait that all rules must implement
pub trait Rule: Send + Sync {
    // ─────────────────────────────────────────────────────────────────────────
    // Identity
    // ─────────────────────────────────────────────────────────────────────────

    /// Unique identifier for the rule
    fn id(&self) -> &'static str;

    /// Human-readable name
    fn name(&self) -> &'static str;

    /// Description of what this rule checks
    fn description(&self) -> &'static str;

    /// Default severity level
    fn default_severity(&self) -> Severity;

    // ─────────────────────────────────────────────────────────────────────────
    // Declarative Entries - What checks/fixes does this rule provide?
    // ─────────────────────────────────────────────────────────────────────────

    /// Returns all checks this rule performs.
    /// Each check represents a specific validation this rule can do.
    fn checks(&self) -> Vec<CheckEntry>;

    /// Returns all fixes this rule can apply.
    /// Each fix can address one or more check failures.
    fn fixes(&self) -> Vec<FixEntry>;

    // ─────────────────────────────────────────────────────────────────────────
    // Execution - Run checks or fixes
    // ─────────────────────────────────────────────────────────────────────────

    /// Execute all checks and return results
    fn check(&self, context: &RuleContext) -> Vec<LintResult>;

    /// Apply all applicable fixes, returns count of fixes applied
    fn fix(&self, _context: &RuleContext) -> Result<u32, RuleError> {
        Err(RuleError::FixNotSupported)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Derived - Computed from other methods
    // ─────────────────────────────────────────────────────────────────────────

    /// Can this rule auto-fix violations?
    fn can_fix(&self) -> bool {
        !self.fixes().is_empty()
    }

    /// Get complete rule info for listing/introspection
    fn info(&self) -> RuleInfo {
        RuleInfo {
            id: self.id().to_string(),
            name: self.name().to_string(),
            description: self.description().to_string(),
            default_severity: self.default_severity().to_string(),
            can_fix: self.can_fix(),
            checks: self.checks(),
            fixes: self.fixes(),
        }
    }
}

/// Registry holding all available rules
pub struct RuleRegistry {
    rules: HashMap<String, Arc<dyn Rule>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            rules: HashMap::new(),
        };
        registry.register_builtin_rules();
        registry
    }

    fn register_builtin_rules(&mut self) {
        self.register(Arc::new(claude_settings::ClaudeSettingsRule::new()));
        self.register(Arc::new(eslint_config_agent::EslintConfigAgentRule::new()));
        self.register(Arc::new(husky_init::HuskyInitRule::new()));
        self.register(Arc::new(pnpm_usage::PnpmUsageRule::new()));
    }

    pub fn register(&mut self, rule: Arc<dyn Rule>) {
        self.rules.insert(rule.id().to_string(), rule);
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Rule>> {
        self.rules.get(id).cloned()
    }

    pub fn all(&self) -> Vec<Arc<dyn Rule>> {
        self.rules.values().cloned().collect()
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
