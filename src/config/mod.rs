pub mod loader;
pub mod defaults;

pub use loader::*;
pub use defaults::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub theme: String,
    pub style: String,
    pub segments: SegmentConfig,
    pub colors: Option<HashMap<String, ThemeColors>>,
    pub budget: Option<BudgetConfig>,
    pub display: Option<DisplayConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentConfig {
    pub directory: Option<DirectoryConfig>,
    pub git: Option<GitConfig>,
    pub block: Option<BlockConfig>,
    pub today: Option<TodayConfig>,
    pub session: Option<SessionConfig>,
    pub context: Option<ContextConfig>,
    pub metrics: Option<MetricsConfig>,
    pub model: Option<ModelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryConfig {
    pub enabled: bool,
    #[serde(rename = "showBasename")]
    pub show_basename: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    pub enabled: bool,
    #[serde(rename = "showSha")]
    pub show_sha: Option<bool>,
    #[serde(rename = "showWorkingTree")]
    pub show_working_tree: Option<bool>,
    #[serde(rename = "showUpstream")]
    pub show_upstream: Option<bool>,
    #[serde(rename = "showStashCount")]
    pub show_stash_count: Option<bool>,
    #[serde(rename = "showRepoName")]
    pub show_repo_name: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockConfig {
    pub enabled: bool,
    #[serde(rename = "type")]
    pub display_type: Option<String>,
    #[serde(rename = "burnType")]
    pub burn_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodayConfig {
    pub enabled: bool,
    #[serde(rename = "type")]
    pub display_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub enabled: bool,
    #[serde(rename = "type")]
    pub display_type: Option<String>,
    #[serde(rename = "costSource")]
    pub cost_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub enabled: bool,
    #[serde(rename = "showPercentageOnly")]
    pub show_percentage_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    #[serde(rename = "showResponseTime")]
    pub show_response_time: Option<bool>,
    #[serde(rename = "showLastResponseTime")]
    pub show_last_response_time: Option<bool>,
    #[serde(rename = "showDuration")]
    pub show_duration: Option<bool>,
    #[serde(rename = "showMessageCount")]
    pub show_message_count: Option<bool>,
    #[serde(rename = "showLinesAdded")]
    pub show_lines_added: Option<bool>,
    #[serde(rename = "showLinesRemoved")]
    pub show_lines_removed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    pub bg: String,
    pub fg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    pub session: Option<BudgetAmount>,
    pub today: Option<BudgetAmount>,
    pub block: Option<BudgetAmount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAmount {
    pub amount: f64,
    #[serde(rename = "type")]
    pub budget_type: Option<String>,
    #[serde(rename = "warningThreshold")]
    pub warning_threshold: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub lines: Option<Vec<LineConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineConfig {
    pub segments: SegmentConfig,
}