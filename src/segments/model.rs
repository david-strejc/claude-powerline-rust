use crate::segments::Segment;
use crate::utils::{debug_with_context, DataAggregator};
use anyhow::Result;
use chrono::{Duration, Utc};

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub current_model: Option<String>,
    pub display_name: Option<String>,
}

pub struct ModelSegment {
    pub enabled: bool,
}

impl ModelSegment {
    pub fn new() -> Self {
        Self {
            enabled: true,
        }
    }

    /// Get the most recently used model from transcript data
    pub async fn get_current_model_info(&self) -> Result<ModelInfo> {
        if !self.enabled {
            return Ok(ModelInfo::default());
        }

        debug_with_context("model", "Looking for current model in recent entries");

        // Load entries from the last hour to find the most recent model
        let aggregator = DataAggregator::new().with_time_filter(1);
        let entries = aggregator.load_all_entries().await?;

        if entries.is_empty() {
            debug_with_context("model", "No recent entries found");
            return Ok(ModelInfo::default());
        }

        // Find the most recent entry with a model
        let mut latest_model: Option<String> = None;
        let mut latest_timestamp = Utc::now() - Duration::days(365); // Very old date

        for entry in entries.iter().rev() {
            if let Some(message) = &entry.message {
                if let Some(model) = &message.model {
                    if entry.timestamp > latest_timestamp {
                        latest_timestamp = entry.timestamp;
                        latest_model = Some(model.clone());
                        debug_with_context("model", &format!("Found model: {}", model));
                        break; // We found the most recent one
                    }
                }
            }
        }

        // Map model ID to display name
        let display_name = latest_model.as_ref().map(|model| {
            get_display_name(model)
        });

        Ok(ModelInfo {
            current_model: latest_model,
            display_name,
        })
    }
}

/// Map model IDs to user-friendly display names
fn get_display_name(model_id: &str) -> String {
    let lower = model_id.to_lowercase();
    
    if lower.contains("opus-4-1") || lower.contains("claude-opus-4-1") {
        "Opus 4.1".to_string()
    } else if lower.contains("opus-4") || lower.contains("claude-opus-4") {
        "Opus 4".to_string()
    } else if lower.contains("opus") {
        "Opus 3".to_string()
    } else if lower.contains("sonnet-4") || lower.contains("claude-sonnet-4") || lower.contains("claude-4-sonnet") {
        "Sonnet 4".to_string()
    } else if lower.contains("3-7-sonnet") || lower.contains("3.7-sonnet") {
        "Sonnet 3.7".to_string()
    } else if lower.contains("3-5-sonnet") || lower.contains("3.5-sonnet") {
        "Sonnet 3.5".to_string()
    } else if lower.contains("sonnet") {
        "Sonnet".to_string()
    } else if lower.contains("3-5-haiku") || lower.contains("3.5-haiku") {
        "Haiku 3.5".to_string()
    } else if lower.contains("haiku") {
        "Haiku".to_string()
    } else {
        // Return a shortened version of the model ID
        model_id.split('-')
            .filter(|s| !s.is_empty() && !s.chars().all(|c| c.is_numeric()))
            .take(2)
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(15)
            .collect()
    }
}

impl Default for ModelInfo {
    fn default() -> Self {
        Self {
            current_model: None,
            display_name: None,
        }
    }
}

impl Segment for ModelSegment {
    fn render(&self) -> Result<String> {
        Ok("⚡ Model".to_string())
    }

    fn name(&self) -> &'static str {
        "model"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}