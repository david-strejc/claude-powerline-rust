use crate::segments::Segment;
use crate::utils::{get_transcript_parser, debug_with_context};
use anyhow::Result;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct MetricsInfo {
    pub avg_response_time: Option<f64>,
    pub last_response_time: Option<f64>,
    pub session_duration: Option<i64>,
    pub message_count: Option<u32>,
    pub lines_added: Option<u32>,
    pub lines_removed: Option<u32>,
}

pub struct MetricsSegment {
    pub enabled: bool,
    pub show_response_time: bool,
    pub show_last_response_time: bool,
    pub show_duration: bool,
    pub show_message_count: bool,
    pub show_lines_added: bool,
    pub show_lines_removed: bool,
}

impl MetricsSegment {
    pub fn new() -> Self {
        Self {
            enabled: true,
            show_response_time: true,
            show_last_response_time: false,
            show_duration: true,
            show_message_count: true,
            show_lines_added: true,
            show_lines_removed: true,
        }
    }

    /// Get performance metrics information
    pub async fn get_metrics_info(&self) -> Result<MetricsInfo> {
        if !self.enabled {
            return Ok(MetricsInfo::default());
        }

        debug_with_context("metrics", "Loading performance metrics");

        // Get recent entries for analysis
        let parser = get_transcript_parser();
        let entries = parser.get_recent_entries(24).await?;

        if entries.is_empty() {
            debug_with_context("metrics", "No entries found for metrics");
            return Ok(MetricsInfo::default());
        }

        let metrics = self.calculate_metrics(&entries)?;
        
        debug_with_context("metrics", &format!(
            "Metrics: avg_response={:?}ms, messages={:?}, duration={:?}min",
            metrics.avg_response_time,
            metrics.message_count,
            metrics.session_duration
        ));

        Ok(metrics)
    }

    /// Calculate various performance metrics from entries
    fn calculate_metrics(&self, entries: &[crate::utils::ParsedEntry]) -> Result<MetricsInfo> {
        let mut info = MetricsInfo::default();

        if entries.is_empty() {
            return Ok(info);
        }

        // Extract response times and calculate averages
        let response_times: Vec<f64> = entries
            .iter()
            .filter_map(|entry| {
                entry.raw.get("response_time_ms")
                    .and_then(|v| v.as_f64())
                    .or_else(|| {
                        entry.raw.get("duration_ms")
                            .and_then(|v| v.as_f64())
                    })
            })
            .collect();

        if !response_times.is_empty() {
            if self.show_response_time {
                let avg = response_times.iter().sum::<f64>() / response_times.len() as f64;
                info.avg_response_time = Some(avg);
            }

            if self.show_last_response_time {
                info.last_response_time = response_times.last().copied();
            }
        }

        // Calculate session duration
        if self.show_duration && entries.len() >= 2 {
            let mut timestamps: Vec<DateTime<Utc>> = entries.iter().map(|e| e.timestamp).collect();
            timestamps.sort();
            
            if let (Some(first), Some(last)) = (timestamps.first(), timestamps.last()) {
                let duration = (*last - *first).num_minutes();
                info.session_duration = Some(duration);
            }
        }

        // Count messages
        if self.show_message_count {
            let message_count = entries.len() as u32;
            info.message_count = Some(message_count);
        }

        // Extract lines added/removed from cost data
        if self.show_lines_added || self.show_lines_removed {
            let total_lines_added: u32 = entries
                .iter()
                .filter_map(|entry| {
                    entry.raw.get("cost")
                        .and_then(|cost| cost.get("total_lines_added"))
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32)
                })
                .sum();

            let total_lines_removed: u32 = entries
                .iter()
                .filter_map(|entry| {
                    entry.raw.get("cost")
                        .and_then(|cost| cost.get("total_lines_removed"))
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32)
                })
                .sum();

            if self.show_lines_added && total_lines_added > 0 {
                info.lines_added = Some(total_lines_added);
            }

            if self.show_lines_removed && total_lines_removed > 0 {
                info.lines_removed = Some(total_lines_removed);
            }
        }

        Ok(info)
    }
}

impl Default for MetricsInfo {
    fn default() -> Self {
        Self {
            avg_response_time: None,
            last_response_time: None,
            session_duration: None,
            message_count: None,
            lines_added: None,
            lines_removed: None,
        }
    }
}

impl Segment for MetricsSegment {
    fn render(&self) -> Result<String> {
        // This will be implemented as part of the display logic
        Ok("⧖ Metrics".to_string())
    }

    fn name(&self) -> &'static str {
        "metrics"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}