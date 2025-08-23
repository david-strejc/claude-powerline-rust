use anyhow::Result;
use std::collections::HashMap;

use crate::utils::claude::{ParsedEntry, UsageInfo};

/// Current Claude API pricing (2025) per million tokens
#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input: f64,
    pub output: f64,
    pub cache_write_5m: f64,  // 1.25x input price for 5-minute cache
    pub cache_write_1h: f64,  // 2x input price for 1-hour cache
    pub cache_read: f64,      // 0.1x input price for cache reads
}

impl ModelPricing {
    pub fn new(input: f64, output: f64) -> Self {
        Self {
            input,
            output,
            cache_write_5m: input * 1.25,
            cache_write_1h: input * 2.0,
            cache_read: input * 0.1,
        }
    }
}

/// Pricing service with current 2025 Claude model pricing
pub struct PricingService {
    pricing_table: HashMap<String, ModelPricing>,
}

impl PricingService {
    pub fn new() -> Self {
        let mut pricing_table = HashMap::new();
        
        // Claude 3.5 Sonnet / Claude 3.7 Sonnet pricing
        pricing_table.insert("claude-3-5-sonnet".to_string(), ModelPricing::new(3.0, 15.0));
        pricing_table.insert("claude-3.5-sonnet".to_string(), ModelPricing::new(3.0, 15.0));
        pricing_table.insert("claude-3-7-sonnet".to_string(), ModelPricing::new(3.0, 15.0));
        
        // Claude Sonnet 4 pricing
        pricing_table.insert("claude-sonnet-4".to_string(), ModelPricing::new(3.0, 15.0));
        pricing_table.insert("claude-4-sonnet".to_string(), ModelPricing::new(3.0, 15.0));
        pricing_table.insert("claude-sonnet-4-20250514".to_string(), ModelPricing::new(3.0, 15.0));
        
        // Claude Opus 4.1 pricing
        pricing_table.insert("claude-opus-4-1".to_string(), ModelPricing::new(15.0, 75.0));
        pricing_table.insert("claude-opus-4-1-20250805".to_string(), ModelPricing::new(15.0, 75.0));
        
        // Claude 3.5 Haiku pricing
        pricing_table.insert("claude-3-5-haiku".to_string(), ModelPricing::new(0.80, 4.0));
        pricing_table.insert("claude-3.5-haiku-20241022".to_string(), ModelPricing::new(0.80, 4.0));
        
        // Legacy Claude 3 Opus pricing (discontinued but may still appear in old transcripts)
        // Using approximate pricing from when it was available
        pricing_table.insert("claude-3-opus".to_string(), ModelPricing::new(15.0, 75.0));
        pricing_table.insert("claude-3-opus-20240229".to_string(), ModelPricing::new(15.0, 75.0));
        
        // Legacy models (approximate pricing)
        pricing_table.insert("claude-3-sonnet".to_string(), ModelPricing::new(3.0, 15.0));
        pricing_table.insert("claude-3-haiku".to_string(), ModelPricing::new(0.25, 1.25));
        
        Self { pricing_table }
    }

    /// Calculate cost for a single transcript entry
    pub fn calculate_cost_for_entry(&self, entry: &ParsedEntry) -> Result<f64> {
        // Use cached cost if available
        if let Some(cost) = entry.cost_usd {
            return Ok(cost);
        }

        // Extract model and usage information
        let message = entry.message.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No message information in entry"))?;
        
        let model_id = message.model.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No model information in entry"))?;
        
        let usage = message.usage.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No usage information in entry"))?;

        self.calculate_cost_for_usage(model_id, usage)
    }

    /// Calculate cost for specific usage and model
    pub fn calculate_cost_for_usage(&self, model_id: &str, usage: &UsageInfo) -> Result<f64> {
        let pricing = self.get_model_pricing(model_id)?;
        
        let input_tokens = usage.input_tokens.unwrap_or(0) as f64;
        let output_tokens = usage.output_tokens.unwrap_or(0) as f64;
        let cache_creation_tokens = usage.cache_creation_input_tokens.unwrap_or(0) as f64;
        let cache_read_tokens = usage.cache_read_input_tokens.unwrap_or(0) as f64;
        
        // Calculate costs per token type
        let input_cost = (input_tokens / 1_000_000.0) * pricing.input;
        let output_cost = (output_tokens / 1_000_000.0) * pricing.output;
        let cache_creation_cost = (cache_creation_tokens / 1_000_000.0) * pricing.cache_write_5m; // Default to 5-minute cache
        let cache_read_cost = (cache_read_tokens / 1_000_000.0) * pricing.cache_read;
        
        Ok(input_cost + output_cost + cache_creation_cost + cache_read_cost)
    }

    /// Get pricing for a specific model with fuzzy matching
    pub fn get_model_pricing(&self, model_id: &str) -> Result<&ModelPricing> {
        // Try exact match first
        if let Some(pricing) = self.pricing_table.get(model_id) {
            return Ok(pricing);
        }
        
        // Try fuzzy matching for various model name formats
        let normalized_model = self.normalize_model_name(model_id);
        
        for (key, pricing) in &self.pricing_table {
            if key.contains(&normalized_model) || normalized_model.contains(key) {
                return Ok(pricing);
            }
        }
        
        // Fallback to reasonable defaults based on model family
        if model_id.to_lowercase().contains("opus") {
            Ok(self.pricing_table.get("claude-3-opus").unwrap())
        } else if model_id.to_lowercase().contains("haiku") {
            Ok(self.pricing_table.get("claude-3-5-haiku").unwrap())
        } else {
            // Default to Sonnet pricing
            Ok(self.pricing_table.get("claude-3-5-sonnet").unwrap())
        }
    }

    /// Normalize model names for fuzzy matching
    fn normalize_model_name(&self, model_id: &str) -> String {
        model_id
            .to_lowercase()
            .replace("@", "")
            .replace("-", "")
            .replace("_", "")
            .replace(".", "")
    }

    /// Get the rate limit weight for a model (used for weighted token calculations)
    pub fn get_model_rate_limit_weight(&self, model_id: &str) -> u32 {
        if model_id.to_lowercase().contains("opus") {
            5 // Opus models had 5x weight for rate limiting
        } else {
            1 // Sonnet, Haiku, and other models have 1x weight
        }
    }

    /// Calculate total cost for a list of entries (handles cumulative token counts per session)
    pub fn calculate_total_cost(&self, entries: &[ParsedEntry]) -> Result<f64> {
        use std::collections::HashMap;
        
        let mut total_cost = 0.0;
        
        // Group entries by session to handle cumulative counts properly
        let mut sessions: HashMap<String, Vec<&ParsedEntry>> = HashMap::new();
        
        for entry in entries {
            let session_key = entry.source_file.clone()
                .or_else(|| entry.raw.get("sessionId").and_then(|v| v.as_str()).map(String::from))
                .unwrap_or_else(|| "unknown".to_string());
            
            sessions.entry(session_key).or_insert_with(Vec::new).push(entry);
        }
        
        // Process each session separately
        for (_session_key, session_entries) in sessions {
            // Sort by timestamp to ensure proper delta calculation
            let mut sorted_entries = session_entries;
            sorted_entries.sort_by_key(|e| e.timestamp);
            
            // Track previous cumulative values for this session
            let mut prev_input = 0u32;
            let mut prev_output = 0u32;
            let mut prev_cache_create = 0u32;
            let mut prev_cache_read = 0u32;
            
            for entry in sorted_entries {
                if let Some(message) = &entry.message {
                    if let Some(usage) = &message.usage {
                        let input_now = usage.input_tokens.unwrap_or(0);
                        let output_now = usage.output_tokens.unwrap_or(0);
                        let cache_create_now = usage.cache_creation_input_tokens.unwrap_or(0);
                        let cache_read_now = usage.cache_read_input_tokens.unwrap_or(0);
                        
                        // Calculate deltas (new tokens since last message in this session)
                        let delta_input = input_now.saturating_sub(prev_input);
                        let delta_output = output_now.saturating_sub(prev_output);
                        let delta_cache_create = cache_create_now.saturating_sub(prev_cache_create);
                        let delta_cache_read = cache_read_now.saturating_sub(prev_cache_read);
                        
                        // Calculate cost for this entry's delta tokens
                        if let Some(model) = &message.model {
                            if let Ok(pricing) = self.get_model_pricing(model) {
                                let input_cost = (delta_input as f64 / 1_000_000.0) * pricing.input;
                                let output_cost = (delta_output as f64 / 1_000_000.0) * pricing.output;
                                let cache_create_cost = (delta_cache_create as f64 / 1_000_000.0) * pricing.cache_write_5m;
                                let cache_read_cost = (delta_cache_read as f64 / 1_000_000.0) * pricing.cache_read;
                                
                                total_cost += input_cost + output_cost + cache_create_cost + cache_read_cost;
                            }
                        }
                        
                        // Update previous values for next iteration
                        prev_input = input_now;
                        prev_output = output_now;
                        prev_cache_create = cache_create_now;
                        prev_cache_read = cache_read_now;
                    } else {
                        // No usage data - keep previous counters unchanged
                        // DO NOT reset them as this would cause the next entry to be counted in full
                    }
                }
            }
        }
        
        Ok(total_cost)
    }

    /// Calculate token breakdown for a list of entries (handles cumulative token counts per session)
    pub fn calculate_token_breakdown(&self, entries: &[ParsedEntry]) -> TokenBreakdown {
        use std::collections::HashMap;
        
        let mut breakdown = TokenBreakdown::default();
        
        if entries.is_empty() {
            return breakdown;
        }
        
        // Group entries by session (source file)
        let mut sessions: HashMap<String, Vec<&ParsedEntry>> = HashMap::new();
        
        for entry in entries {
            let session_key = entry.source_file.clone()
                .or_else(|| entry.raw.get("sessionId").and_then(|v| v.as_str()).map(String::from))
                .unwrap_or_else(|| "unknown".to_string());
            
            sessions.entry(session_key).or_insert_with(Vec::new).push(entry);
        }
        
        // Process each session separately
        for (_session_key, session_entries) in sessions {
            // Sort by timestamp to ensure proper delta calculation
            let mut sorted_entries = session_entries;
            sorted_entries.sort_by_key(|e| e.timestamp);
            
            // Track previous cumulative values for this session
            let mut prev_input = 0u32;
            let mut prev_output = 0u32;
            let mut prev_cache_create = 0u32;
            let mut prev_cache_read = 0u32;
            
            for entry in sorted_entries {
                if let Some(message) = &entry.message {
                    if let Some(usage) = &message.usage {
                        let input_now = usage.input_tokens.unwrap_or(0);
                        let output_now = usage.output_tokens.unwrap_or(0);
                        let cache_create_now = usage.cache_creation_input_tokens.unwrap_or(0);
                        let cache_read_now = usage.cache_read_input_tokens.unwrap_or(0);
                        
                        // Calculate deltas (new tokens since last message in this session)
                        // Use saturating_sub to handle session boundaries where counts reset
                        let delta_input = input_now.saturating_sub(prev_input);
                        let delta_output = output_now.saturating_sub(prev_output);
                        let delta_cache_create = cache_create_now.saturating_sub(prev_cache_create);
                        let delta_cache_read = cache_read_now.saturating_sub(prev_cache_read);
                        
                        // Only add the delta (new tokens) not the cumulative total
                        breakdown.input_tokens += delta_input;
                        breakdown.output_tokens += delta_output;
                        breakdown.cache_creation_input_tokens += delta_cache_create;
                        breakdown.cache_read_input_tokens += delta_cache_read;
                        
                        // Update previous values for next iteration
                        prev_input = input_now;
                        prev_output = output_now;
                        prev_cache_create = cache_create_now;
                        prev_cache_read = cache_read_now;
                    } else {
                        // No usage data - keep previous counters unchanged
                        // DO NOT reset them as this would cause the next entry to be counted in full
                    }
                }
            }
        }
        
        breakdown
    }

    /// Calculate weighted tokens (applying model-specific multipliers and handling cumulative counts)
    pub fn calculate_weighted_tokens(&self, entries: &[ParsedEntry]) -> u32 {
        use std::collections::HashMap;
        
        // Group entries by session (source file)
        let mut sessions: HashMap<String, Vec<&ParsedEntry>> = HashMap::new();
        
        for entry in entries {
            let session_key = entry.source_file.clone()
                .or_else(|| entry.raw.get("sessionId").and_then(|v| v.as_str()).map(String::from))
                .unwrap_or_else(|| "unknown".to_string());
            
            sessions.entry(session_key).or_insert_with(Vec::new).push(entry);
        }
        
        let mut total_weighted = 0u32;
        
        // Process each session separately
        for (_session_key, session_entries) in sessions {
            // Sort by timestamp to ensure proper delta calculation
            let mut sorted_entries = session_entries;
            sorted_entries.sort_by_key(|e| e.timestamp);
            
            // Track previous cumulative values for this session
            let mut prev_input = 0u32;
            let mut prev_output = 0u32;
            let mut prev_cache_create = 0u32;
            let mut prev_cache_read = 0u32;
            
            for entry in sorted_entries {
                if let Some(message) = &entry.message {
                    if let Some(usage) = &message.usage {
                        let input_now = usage.input_tokens.unwrap_or(0);
                        let output_now = usage.output_tokens.unwrap_or(0);
                        let cache_create_now = usage.cache_creation_input_tokens.unwrap_or(0);
                        let cache_read_now = usage.cache_read_input_tokens.unwrap_or(0);
                        
                        // Calculate deltas for this session
                        let delta_input = input_now.saturating_sub(prev_input);
                        let delta_output = output_now.saturating_sub(prev_output);
                        let delta_cache_create = cache_create_now.saturating_sub(prev_cache_create);
                        let delta_cache_read = cache_read_now.saturating_sub(prev_cache_read);
                        
                        let delta_total = delta_input + delta_output + delta_cache_create + delta_cache_read;
                        
                        // Apply model weight
                        let weight = if let Some(model) = &message.model {
                            self.get_model_rate_limit_weight(model)
                        } else {
                            1
                        };
                        
                        total_weighted += delta_total * weight;
                        
                        // Update previous values for next iteration
                        prev_input = input_now;
                        prev_output = output_now;
                        prev_cache_create = cache_create_now;
                        prev_cache_read = cache_read_now;
                    } else {
                        // No usage data - keep previous counters unchanged
                        // DO NOT reset them as this would cause the next entry to be counted in full
                    }
                }
            }
        }
        
        total_weighted
    }
}

impl Default for PricingService {
    fn default() -> Self {
        Self::new()
    }
}

/// Token usage breakdown
#[derive(Debug, Clone, Default)]
pub struct TokenBreakdown {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
}

impl TokenBreakdown {
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens + 
        self.cache_creation_input_tokens + self.cache_read_input_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::claude::MessageInfo;

    #[test]
    fn test_sonnet_pricing() {
        let pricing_service = PricingService::new();
        let usage = UsageInfo {
            input_tokens: Some(1000000), // 1M tokens
            output_tokens: Some(500000),  // 0.5M tokens
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        };
        
        let cost = pricing_service.calculate_cost_for_usage("claude-3-5-sonnet", &usage).unwrap();
        let expected = 3.0 + (0.5 * 15.0); // $3 input + $7.5 output = $10.5
        assert!((cost - expected).abs() < 0.001);
    }

    #[test]
    fn test_model_weight_calculation() {
        let pricing_service = PricingService::new();
        
        assert_eq!(pricing_service.get_model_rate_limit_weight("claude-3-opus"), 5);
        assert_eq!(pricing_service.get_model_rate_limit_weight("claude-3-5-sonnet"), 1);
        assert_eq!(pricing_service.get_model_rate_limit_weight("claude-3-5-haiku"), 1);
    }
}