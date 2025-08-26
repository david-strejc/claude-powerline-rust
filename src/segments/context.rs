use crate::segments::Segment;
use crate::utils::debug_with_context;
use crate::utils::claude::{parse_jsonl_content, ParsedEntry};
use anyhow::Result;
use tokio::fs;

#[derive(Debug, Clone)]
pub struct ContextInfo {
    pub input_tokens: u32,
    pub context_left_percentage: u32,
    pub usable_percentage: u32,
    pub max_tokens: u32,
    pub usable_tokens: u32,
}

pub struct ContextSegment {
    pub enabled: bool,
    pub show_percentage_only: bool,
}

impl ContextSegment {
    pub fn new() -> Self {
        Self {
            enabled: true,
            show_percentage_only: false,
        }
    }

    /// Get context window information by analyzing current session transcript
    pub async fn get_context_info(&self) -> Result<ContextInfo> {
        if !self.enabled {
            return Ok(ContextInfo::default());
        }

        debug_with_context("context", "Analyzing current session for context usage");

        // Try to get current session transcript
        if let Some(transcript_path) = self.find_current_session_transcript().await? {
            debug_with_context("context", &format!("Found session transcript: {}", transcript_path.display()));
            
            let context_info = self.calculate_context_from_transcript(&transcript_path).await?;
            
            debug_with_context("context", &format!(
                "Context: input_tokens={}, left_percentage={}%",
                context_info.input_tokens,
                context_info.context_left_percentage
            ));
            
            return Ok(context_info);
        }

        debug_with_context("context", "No current session transcript found");
        Ok(ContextInfo::default())
    }

    /// Find the current session transcript file
    async fn find_current_session_transcript(&self) -> Result<Option<std::path::PathBuf>> {
        // Try to get specific session ID first (same logic as session segment)
        if let Ok(session_id) = std::env::var("CLAUDE_SESSION_ID") {
            debug_with_context("context", &format!("Using session ID from env: {}", session_id));
            match crate::utils::claude::find_transcript_file(&session_id).await {
                Ok(Some(transcript_path)) => {
                    debug_with_context("context", &format!("Found specific session transcript: {}", transcript_path.display()));
                    return Ok(Some(transcript_path));
                }
                Ok(None) => {
                    debug_with_context("context", &format!("Session transcript not found for ID: {}", session_id));
                }
                Err(e) => {
                    debug_with_context("context", &format!("Error finding session transcript: {}", e));
                }
            }
        }

        // Fallback: find most recent transcript file in Claude projects
        debug_with_context("context", "No session ID found, using most recent transcript");
        let claude_paths = crate::utils::claude::get_claude_paths()?;
        let project_paths = crate::utils::claude::find_project_paths(&claude_paths).await?;
        
        let mut most_recent_file = None;
        let mut most_recent_time = std::time::SystemTime::UNIX_EPOCH;
        
        for project_path in project_paths {
            if let Ok(entries) = fs::read_dir(&project_path).await {
                let mut entries = entries;
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                        if let Ok(metadata) = entry.metadata().await {
                            let mtime = metadata.modified()?;
                            if mtime > most_recent_time {
                                most_recent_time = mtime;
                                most_recent_file = Some(path);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(most_recent_file)
    }
    
    /// Calculate context info from transcript file (replicates TypeScript logic)
    async fn calculate_context_from_transcript(&self, transcript_path: &std::path::Path) -> Result<ContextInfo> {
        // Read and parse the transcript file
        let content = fs::read_to_string(transcript_path).await?;
        let entries = parse_jsonl_content(&content)?;
        
        // Find most recent non-sidechain entry with usage data (reverse iteration like TypeScript)
        for entry in entries.iter().rev() {
            // Skip sidechain entries
            if entry.is_sidechain == Some(true) {
                continue;
            }
            
            if let Some(message) = &entry.message {
                if let Some(usage) = &message.usage {
                    // Calculate total context length (input + cache tokens)
                    let context_length = usage.input_tokens.unwrap_or(0)
                        + usage.cache_read_input_tokens.unwrap_or(0)
                        + usage.cache_creation_input_tokens.unwrap_or(0);
                    
                    if context_length == 0 {
                        continue;
                    }
                    
                    // Constants matching TypeScript version
                    const CONTEXT_LIMIT: u32 = 200_000;  // 200K context limit
                    const USABLE_LIMIT: u32 = 154_000;   // 77% of total (200K * 0.77)
                    
                    // Calculate percentages
                    let percentage = ((context_length as f64 / CONTEXT_LIMIT as f64) * 100.0)
                        .round().min(100.0) as u32;
                    
                    let usable_percentage = ((context_length as f64 / USABLE_LIMIT as f64) * 100.0)
                        .round().min(100.0) as u32;
                    
                    // Context left percentage (the key metric!)
                    let context_left_percentage = 100u32.saturating_sub(usable_percentage);
                    
                    return Ok(ContextInfo {
                        input_tokens: context_length,
                        context_left_percentage,
                        usable_percentage,
                        max_tokens: CONTEXT_LIMIT,
                        usable_tokens: USABLE_LIMIT,
                    });
                }
            }
        }
        
        // No valid entries found, return default
        Ok(ContextInfo::default())
    }
}

impl Default for ContextInfo {
    fn default() -> Self {
        Self {
            input_tokens: 0,
            context_left_percentage: 100,
            usable_percentage: 0,
            max_tokens: 200000,
            usable_tokens: 154000,
        }
    }
}

impl Segment for ContextSegment {
    fn render(&self) -> Result<String> {
        // This will be implemented as part of the display logic
        Ok("◔ Context".to_string())
    }

    fn name(&self) -> &'static str {
        "context"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}