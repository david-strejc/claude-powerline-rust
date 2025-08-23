use crate::segments::Segment;
use crate::utils::{ParsedEntry, debug_with_context, DataAggregator, PricingService};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc, Timelike};


#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub cost: Option<f64>,
    pub tokens: Option<u32>,
    pub weighted_tokens: Option<u32>,
    pub time_remaining: Option<i64>,
    pub reset_time: Option<DateTime<Utc>>,
    pub burn_rate: Option<f64>,
    pub token_burn_rate: Option<f64>,
}

pub struct BlockSegment {
    pub enabled: bool,
    pub display_type: String,
    pub burn_type: String,
}

impl BlockSegment {
    pub fn new() -> Self {
        Self {
            enabled: true,
            display_type: "weighted".to_string(),
            burn_type: "cost".to_string(),
        }
    }

    /// Get active block information using global data aggregation
    pub async fn get_active_block_info(&self) -> Result<BlockInfo> {
        if !self.enabled {
            return Ok(BlockInfo::default());
        }

        debug_with_context("block", "Loading entries for 5-hour session blocks");

        // Use new data aggregation pipeline to get all recent entries
        let aggregator = DataAggregator::new().with_time_filter(24);
        let entries = aggregator.load_all_entries().await?;

        if entries.is_empty() {
            debug_with_context("block", "No entries found in recent window");
            return Ok(BlockInfo::default());
        }

        debug_with_context("block", &format!("Loaded {} entries from global aggregation", entries.len()));
        
        // Identify session blocks using the original algorithm
        let blocks = self.identify_session_blocks(&entries);
        debug_with_context("block", &format!("Found {} session blocks", blocks.len()));

        // Find active block
        if let Some(active_block) = self.find_active_block(&blocks) {
            debug_with_context("block", &format!("Found active block with {} entries", active_block.len()));
            Ok(self.calculate_block_info(&active_block))
        } else {
            debug_with_context("block", "No active block found");
            Ok(BlockInfo::default())
        }
    }


    /// Identify 5-hour session blocks using the original TypeScript algorithm
    fn identify_session_blocks(&self, entries: &[ParsedEntry]) -> Vec<Vec<ParsedEntry>> {
        if entries.is_empty() {
            return Vec::new();
        }

        // Entries should already be sorted by timestamp from data aggregation
        let session_duration_ms = 5 * 60 * 60 * 1000; // 5 hours in milliseconds
        let mut blocks = Vec::new();
        let mut current_block_entries = Vec::new();
        let mut current_block_start: Option<DateTime<Utc>> = None;

        for entry in entries {
            let entry_time = entry.timestamp;

            match current_block_start {
                None => {
                    // Start first block - floor to the hour
                    current_block_start = Some(self.floor_to_hour(entry_time));
                    current_block_entries.push(entry.clone());
                }
                Some(block_start) => {
                    let time_since_block_start = entry_time.signed_duration_since(block_start).num_milliseconds();
                    
                    let time_since_last_entry = if let Some(last) = current_block_entries.last() {
                        entry_time.signed_duration_since(last.timestamp).num_milliseconds()
                    } else {
                        0
                    };

                    // Check if we need to start a new block
                    // New block starts if: time since block start > 5 hours OR time since last entry > 5 hours
                    if time_since_block_start > session_duration_ms || time_since_last_entry > session_duration_ms {
                        // Finalize current block
                        if !current_block_entries.is_empty() {
                            blocks.push(current_block_entries.clone());
                        }

                        // Start new block
                        current_block_start = Some(self.floor_to_hour(entry_time));
                        current_block_entries = vec![entry.clone()];
                    } else {
                        // Add to current block
                        current_block_entries.push(entry.clone());
                    }
                }
            }
        }

        // Don't forget the last block
        if !current_block_entries.is_empty() {
            blocks.push(current_block_entries);
        }

        blocks
    }

    /// Find the currently active block using original algorithm
    fn find_active_block<'a>(&self, blocks: &'a [Vec<ParsedEntry>]) -> Option<&'a Vec<ParsedEntry>> {
        let now = Utc::now();
        let session_duration_ms = 5 * 60 * 60 * 1000; // 5 hours in milliseconds

        // Check blocks in reverse order (most recent first)
        for block in blocks.iter().rev() {
            if let Some(first_entry) = block.first() {
                let block_start = self.floor_to_hour(first_entry.timestamp);
                let block_end_time = block_start + Duration::hours(5);
                
                // Get the actual end time (last entry in the block)
                let actual_end_time = block.last()
                    .map(|e| e.timestamp)
                    .unwrap_or(block_start);
                
                // Block is active if:
                // 1. Current time is within 5 hours of the last entry
                // 2. Current time is before the theoretical block end time
                let time_since_last_entry_ms = now.signed_duration_since(actual_end_time).num_milliseconds();
                let is_active = time_since_last_entry_ms < session_duration_ms && now < block_end_time;
                
                if is_active {
                    return Some(block);
                }
            }
        }

        None
    }

    /// Calculate comprehensive block information using pricing service
    fn calculate_block_info(&self, entries: &[ParsedEntry]) -> BlockInfo {
        if entries.is_empty() {
            return BlockInfo::default();
        }

        let pricing_service = PricingService::new();

        // Calculate total cost using pricing service
        let total_cost = pricing_service.calculate_total_cost(entries).unwrap_or(0.0);
        
        // Calculate token breakdown
        let token_breakdown = pricing_service.calculate_token_breakdown(entries);
        let total_tokens = token_breakdown.total_tokens();
        
        // Calculate weighted tokens (applies 5x multiplier for Opus models)
        let weighted_tokens = pricing_service.calculate_weighted_tokens(entries);

        // Calculate time remaining and reset time based on block start time
        let (time_remaining, reset_time) = if let Some(first_entry) = entries.first() {
            let block_start = self.floor_to_hour(first_entry.timestamp);
            let session_end = block_start + Duration::hours(5);
            let now = Utc::now();
            
            if now < session_end {
                (Some((session_end - now).num_minutes()), Some(session_end))
            } else {
                (Some(0), Some(session_end))
            }
        } else {
            (None, None)
        };

        // Calculate burn rates based on actual activity duration
        let (burn_rate, token_burn_rate) = if entries.len() >= 2 {
            let first_timestamp = entries.first().unwrap().timestamp;
            let last_timestamp = entries.last().unwrap().timestamp;
            
            let duration_minutes = last_timestamp.signed_duration_since(first_timestamp).num_minutes() as f64;
            
            if duration_minutes > 0.0 {
                let duration_hours = duration_minutes / 60.0;
                let cost_burn_rate = if total_cost > 0.0 { 
                    Some(total_cost / duration_hours) 
                } else { 
                    None 
                };
                let token_burn_rate = if total_tokens > 0 { 
                    Some(total_tokens as f64 / duration_hours) 
                } else { 
                    None 
                };
                (cost_burn_rate, token_burn_rate)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        BlockInfo {
            cost: if total_cost > 0.0 { Some(total_cost) } else { None },
            tokens: if total_tokens > 0 { Some(total_tokens) } else { None },
            weighted_tokens: if weighted_tokens > 0 { Some(weighted_tokens) } else { None },
            time_remaining,
            reset_time,
            burn_rate,
            token_burn_rate,
        }
    }

    /// Floor timestamp to the nearest hour
    fn floor_to_hour(&self, timestamp: DateTime<Utc>) -> DateTime<Utc> {
        timestamp.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap()
    }

}

impl Default for BlockInfo {
    fn default() -> Self {
        Self {
            cost: None,
            tokens: None,
            weighted_tokens: None,
            time_remaining: None,
            reset_time: None,
            burn_rate: None,
            token_burn_rate: None,
        }
    }
}

impl Segment for BlockSegment {
    fn render(&self) -> Result<String> {
        // This will be implemented as part of the display logic
        Ok("◱ Block".to_string())
    }

    fn name(&self) -> &'static str {
        "block"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}