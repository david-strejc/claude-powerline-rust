use crate::segments::Segment;
use crate::utils::{debug_with_context, DataAggregator, PricingService, ParsedEntry};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct TodayInfo {
    pub cost: Option<f64>,
    pub tokens: Option<u32>,
    pub message_count: Option<u32>,
}

pub struct TodaySegment {
    pub enabled: bool,
    pub display_type: String,
}

impl TodaySegment {
    pub fn new() -> Self {
        Self {
            enabled: true,
            display_type: "cost".to_string(),
        }
    }

    /// Get today's usage information using global data aggregation
    pub async fn get_today_info(&self) -> Result<TodayInfo> {
        if !self.enabled {
            return Ok(TodayInfo::default());
        }

        debug_with_context("today", "Loading today's entries");

        // Use the new global data aggregation pipeline
        let aggregator = DataAggregator::new();
        let entries = aggregator.load_today_entries().await?;

        if entries.is_empty() {
            debug_with_context("today", "No entries found for today");
            return Ok(TodayInfo::default());
        }

        debug_with_context("today", &format!("Found {} entries for today", entries.len()));

        // Calculate totals using pricing service
        Ok(self.calculate_today_info(&entries))
    }

    /// Calculate today's usage information using pricing service
    fn calculate_today_info(&self, entries: &[ParsedEntry]) -> TodayInfo {
        if entries.is_empty() {
            return TodayInfo::default();
        }

        let pricing_service = PricingService::new();

        // Calculate total cost using pricing service
        let total_cost = pricing_service.calculate_total_cost(entries).unwrap_or(0.0);
        
        // Calculate token breakdown
        let token_breakdown = pricing_service.calculate_token_breakdown(entries);
        let total_tokens = token_breakdown.total_tokens();

        // Count messages (approximate)
        let message_count = entries.len() as u32;

        TodayInfo {
            cost: if total_cost > 0.0 { Some(total_cost) } else { None },
            tokens: if total_tokens > 0 { Some(total_tokens) } else { None },
            message_count: if message_count > 0 { Some(message_count) } else { None },
        }
    }
}

impl Default for TodayInfo {
    fn default() -> Self {
        Self {
            cost: None,
            tokens: None,
            message_count: None,
        }
    }
}

impl Segment for TodaySegment {
    fn render(&self) -> Result<String> {
        Ok("☉ Today".to_string())
    }

    fn name(&self) -> &'static str {
        "today"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}