#![deny(clippy::all)]

mod engine;
mod rules;
mod types;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use engine::Runner;
use types::{Config, LintReport, RuleInfo};

/// Engine wrapper exposed to JavaScript
#[napi]
pub struct Engine {
    inner: Runner,
}

#[napi]
impl Engine {
    #[napi(constructor)]
    pub fn new(config_json: String) -> Result<Self> {
        let config: Config = if config_json.is_empty() || config_json == "{}" {
            Config::default()
        } else {
            serde_json::from_str(&config_json)
                .map_err(|e| Error::from_reason(format!("Invalid config: {}", e)))?
        };

        Ok(Self {
            inner: Runner::new(config),
        })
    }

    /// Run all enabled rules on the specified path
    #[napi]
    pub fn lint(&self, path: String) -> Result<LintReport> {
        self.inner
            .run(&path)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Run rules and apply fixes where possible
    #[napi]
    pub fn fix(&self, path: String) -> Result<LintReport> {
        self.inner
            .run_with_fix(&path)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// List all available rules
    #[napi]
    pub fn list_rules(&self) -> Vec<RuleInfo> {
        self.inner.list_rules()
    }
}

/// Create an engine with the given configuration
#[napi]
pub fn create_engine(config_json: String) -> Result<Engine> {
    Engine::new(config_json)
}
