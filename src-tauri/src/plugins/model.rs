use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

pub const CAPABILITY_ENGAGEMENT_READ: &str = "engagement.read";
const ALLOWED_CAPABILITIES: &[&str] = &[CAPABILITY_ENGAGEMENT_READ];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher_id: String,
    #[serde(default)]
    pub description: String,
    pub runtime: String,
    pub entry: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub min_app_version: String,
}

impl PluginManifest {
    pub fn validate(&self) -> AppResult<()> {
        validate_plugin_id(&self.id, "plugin id")?;
        validate_plugin_id(&self.publisher_id, "publisher id")?;
        if self.name.trim().is_empty() || self.name.chars().count() > 120 {
            return Err(AppError::Validation(
                "Plugin name must be between 1 and 120 characters.".into(),
            ));
        }
        if self.version.is_empty()
            || self.version.contains("..")
            || self.version.chars().count() > 50
            || !self
                .version
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
        {
            return Err(AppError::Validation("Invalid plugin version.".into()));
        }
        if self.runtime != "wasm" {
            return Err(AppError::Validation(
                "Only the wasm plugin runtime is supported.".into(),
            ));
        }
        if self.entry != "plugin.wasm" {
            return Err(AppError::Validation(
                "The plugin entry must be plugin.wasm.".into(),
            ));
        }
        if self.description.chars().count() > 2_000 {
            return Err(AppError::Validation(
                "Plugin description is too long.".into(),
            ));
        }
        if self.min_app_version.chars().count() > 50 {
            return Err(AppError::Validation(
                "Minimum app version is too long.".into(),
            ));
        }
        let mut seen = Vec::new();
        for capability in &self.capabilities {
            if !ALLOWED_CAPABILITIES.contains(&capability.as_str()) {
                return Err(AppError::Validation(format!(
                    "Unsupported plugin capability: {capability}"
                )));
            }
            if seen.contains(capability) {
                return Err(AppError::Validation(format!(
                    "Duplicate plugin capability: {capability}"
                )));
            }
            seen.push(capability.clone());
        }
        Ok(())
    }

    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|item| item == capability)
    }
}

pub fn validate_plugin_id(value: &str, label: &str) -> AppResult<()> {
    let valid = (3..=100).contains(&value.len())
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
        && !value.starts_with('.')
        && !value.starts_with('-')
        && !value.contains("..");
    if !valid {
        return Err(AppError::Validation(format!(
            "Invalid {label}; use lowercase letters, digits, dots, dashes, or underscores."
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher_id: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub enabled: bool,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPublisher {
    pub id: String,
    pub name: String,
    pub public_key: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPluginPublisherInput {
    pub id: String,
    pub name: String,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginTestSuggestion {
    pub title: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub approach: String,
    #[serde(default)]
    pub expected_result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginFindingSuggestion {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub impact: String,
    #[serde(default)]
    pub remediation: String,
    #[serde(default)]
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginResult {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub test_cases: Vec<PluginTestSuggestion>,
    #[serde(default)]
    pub findings: Vec<PluginFindingSuggestion>,
    #[serde(default)]
    pub notes: Vec<String>,
}

impl PluginResult {
    pub fn validate(&self) -> AppResult<()> {
        if self.summary.chars().count() > 5_000 {
            return Err(AppError::Validation("Plugin summary is too long.".into()));
        }
        if self.test_cases.len() > 50 || self.findings.len() > 20 || self.notes.len() > 50 {
            return Err(AppError::Validation(
                "Plugin returned too many suggestions.".into(),
            ));
        }
        for test_case in &self.test_cases {
            if test_case.title.trim().is_empty() || test_case.title.chars().count() > 200 {
                return Err(AppError::Validation(
                    "Plugin returned an invalid test-case title.".into(),
                ));
            }
            if test_case.category.chars().count() > 120
                || test_case.approach.chars().count() > 5_000
                || test_case.expected_result.chars().count() > 5_000
            {
                return Err(AppError::Validation(
                    "Plugin returned an oversized test case.".into(),
                ));
            }
        }
        for finding in &self.findings {
            if finding.title.trim().is_empty() || finding.title.chars().count() > 200 {
                return Err(AppError::Validation(
                    "Plugin returned an invalid finding title.".into(),
                ));
            }
            if finding.description.chars().count() > 10_000
                || finding.impact.chars().count() > 10_000
                || finding.remediation.chars().count() > 10_000
                || finding.severity.chars().count() > 20
            {
                return Err(AppError::Validation(
                    "Plugin returned an oversized finding.".into(),
                ));
            }
        }
        for note in &self.notes {
            if note.chars().count() > 2_000 {
                return Err(AppError::Validation(
                    "Plugin returned an oversized note.".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRunResult {
    pub plugin_id: String,
    pub plugin_name: String,
    pub plugin_version: String,
    pub summary: String,
    pub test_cases: Vec<PluginTestSuggestion>,
    pub findings: Vec<PluginFindingSuggestion>,
    pub notes: Vec<String>,
    pub logs: Vec<String>,
    pub fuel_used: u64,
}
