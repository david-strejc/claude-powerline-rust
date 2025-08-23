use crate::segments::Segment;
use crate::utils::{find_transcript_file, debug_with_context, DataAggregator, PricingService, ParsedEntry};
use anyhow::Result;
use std::env;

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub cost: Option<f64>,
    pub tokens: Option<u32>,
    pub message_count: Option<u32>,
    pub duration_minutes: Option<i64>,
    pub session_id: Option<String>,
}

pub struct SessionSegment {
    pub enabled: bool,
    pub display_type: String,
    pub cost_source: String,
}

impl SessionSegment {
    pub fn new() -> Self {
        Self {
            enabled: true,
            display_type: "tokens".to_string(),
            cost_source: "calculated".to_string(),
        }
    }

    /// Get current session information with optimized performance
    pub async fn get_session_info(&self) -> Result<SessionInfo> {
        if !self.enabled {
            return Ok(SessionInfo::default());
        }

        // Try to get session ID from environment or hook data
        let session_id = self.get_current_session_id().await?;
        
        if let Some(ref sid) = session_id {
            debug_with_context("session", &format!("Loading session entries for: {}", sid));
            
            // Load entries for this specific session using new architecture
            if let Some(transcript_path) = find_transcript_file(sid).await? {
                // Use DataAggregator to load entries from specific session file
                let aggregator = DataAggregator::new();
                let entries = aggregator.load_session_entries(&transcript_path).await?;

                debug_with_context("session", &format!("Found {} entries in current session", entries.len()));

                let mut info = self.calculate_session_info(&entries);
                info.session_id = session_id;
                return Ok(info);
            }
        }

        debug_with_context("session", "No current session found");
        Ok(SessionInfo::default())
    }

    /// Try to determine the current session ID
    async fn get_current_session_id(&self) -> Result<Option<String>> {
        // Try environment variables first
        if let Ok(session_id) = env::var("CLAUDE_SESSION_ID") {
            return Ok(Some(session_id));
        }

        // Try to get from hook data or other sources
        // This could be extended to read from Claude's state files
        
        Ok(None)
    }

    /// Calculate comprehensive session information using pricing service
    fn calculate_session_info(&self, entries: &[ParsedEntry]) -> SessionInfo {
        if entries.is_empty() {
            return SessionInfo::default();
        }

        let pricing_service = PricingService::new();

        // Calculate total cost using pricing service
        let total_cost = pricing_service.calculate_total_cost(entries).unwrap_or(0.0);
        
        // Calculate token breakdown
        let token_breakdown = pricing_service.calculate_token_breakdown(entries);
        let total_tokens = token_breakdown.total_tokens();

        let message_count = entries.len() as u32;

        // Calculate session duration from first to last entry
        let duration_minutes = if entries.len() >= 2 {
            let mut timestamps: Vec<_> = entries.iter().map(|e| e.timestamp).collect();
            timestamps.sort();
            if let (Some(first), Some(last)) = (timestamps.first(), timestamps.last()) {
                Some((*last - *first).num_minutes().max(0))
            } else {
                None
            }
        } else {
            None
        };

        debug_with_context("session", &format!(
            "Session totals: ${:.2}, {} tokens, {} messages, {} minutes",
            total_cost,
            total_tokens,
            message_count,
            duration_minutes.unwrap_or(0)
        ));

        SessionInfo {
            cost: if total_cost > 0.0 { Some(total_cost) } else { None },
            tokens: if total_tokens > 0 { Some(total_tokens) } else { None },
            message_count: if message_count > 0 { Some(message_count) } else { None },
            duration_minutes,
            session_id: None, // Will be set by caller
        }
    }
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self {
            cost: None,
            tokens: None,
            message_count: None,
            duration_minutes: None,
            session_id: None,
        }
    }
}

impl Segment for SessionSegment {
    fn render(&self) -> Result<String> {
        // This will be implemented as part of the display logic
        Ok("§ Session".to_string())
    }

    fn name(&self) -> &'static str {
        "session"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}