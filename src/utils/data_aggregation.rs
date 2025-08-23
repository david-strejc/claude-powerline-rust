use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rayon::prelude::*;
use serde_json::{Deserializer, Value};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::utils::claude::{ParsedEntry, MessageInfo, UsageInfo, get_claude_paths};

/// High-performance data aggregation pipeline that discovers all Claude projects,
/// loads transcript files in parallel, and performs global deduplication
pub struct DataAggregator {
    time_filter_hours: Option<u32>,
}

impl DataAggregator {
    pub fn new() -> Self {
        Self {
            time_filter_hours: None,
        }
    }

    pub fn with_time_filter(mut self, hours: u32) -> Self {
        self.time_filter_hours = Some(hours);
        self
    }

    /// Load all entries from all projects with optional time filtering
    pub async fn load_all_entries(&self) -> Result<Vec<ParsedEntry>> {
        // Phase 1: Discover all project directories
        let claude_paths = get_claude_paths()?;
        let project_paths = self.discover_all_projects(&claude_paths)?;
        
        // Phase 2: Discover all transcript files with time filtering
        let transcript_files = self.discover_transcript_files(&project_paths)?;
        
        // Phase 3: Parse files in parallel using streaming
        let all_entries = self.parse_files_parallel(&transcript_files)?;
        
        // Phase 4: Global deduplication and sorting
        let deduplicated_entries = self.deduplicate_and_sort(all_entries)?;
        
        Ok(deduplicated_entries)
    }

    /// Discover all project directories across all Claude paths
    fn discover_all_projects(&self, claude_paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let mut project_paths = Vec::new();
        
        for claude_path in claude_paths {
            let projects_dir = claude_path.join("projects");
            if !projects_dir.exists() {
                continue;
            }
            
            for entry in WalkDir::new(&projects_dir)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_entry(|e| e.file_type().is_dir()) 
            {
                let entry = entry.context("Failed to read project directory")?;
                project_paths.push(entry.into_path());
            }
        }
        
        Ok(project_paths)
    }

    /// Discover all transcript files with optional time-based filtering
    fn discover_transcript_files(&self, project_paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let mut transcript_files = Vec::new();
        let cutoff_time = self.time_filter_hours
            .map(|hours| Utc::now() - chrono::Duration::hours(hours as i64));
        
        for project_path in project_paths {
            for entry in WalkDir::new(project_path)
                .max_depth(1) // Only look in project directory, not subdirectories
                .into_iter()
            {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue, // Skip files we can't read
                };
                
                let path = entry.path();
                
                // Only process .jsonl files
                if !path.is_file() || !path.file_name()
                    .map(|name| name.to_string_lossy().ends_with(".jsonl"))
                    .unwrap_or(false) {
                    continue;
                }
                
                // Apply time-based filtering if specified
                if let Some(cutoff) = cutoff_time {
                    if let Ok(metadata) = std::fs::metadata(path) {
                        if let Ok(modified) = metadata.modified() {
                            let modified_utc: DateTime<Utc> = modified.into();
                            if modified_utc < cutoff {
                                continue; // Skip old files
                            }
                        }
                    }
                }
                
                transcript_files.push(path.to_path_buf());
            }
        }
        
        Ok(transcript_files)
    }

    /// Parse multiple files in parallel using streaming JSON parsing
    fn parse_files_parallel(&self, file_paths: &[PathBuf]) -> Result<Vec<ParsedEntry>> {
        let all_entries: Vec<ParsedEntry> = file_paths
            .par_iter()
            .flat_map(|path| {
                match self.parse_transcript_file_streaming(path) {
                    Ok(entries) => entries,
                    Err(e) => {
                        // Log error but continue processing other files
                        eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                        Vec::new()
                    }
                }
            })
            .collect();
            
        Ok(all_entries)
    }

    /// Parse a single transcript file using streaming JSON parsing
    fn parse_transcript_file_streaming(&self, file_path: &Path) -> Result<Vec<ParsedEntry>> {
        let file = File::open(file_path)
            .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
        
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        
        // Get the file path as string for source tracking
        let source_file = file_path.to_string_lossy().to_string();
        
        // Stream through JSONL file line by line
        for line in std::io::BufRead::lines(reader) {
            let line = line.context("Failed to read line from transcript file")?;
            let line = line.trim();
            
            if line.is_empty() {
                continue;
            }
            
            match self.parse_jsonl_line(line) {
                Ok(Some(mut entry)) => {
                    // Set the source file for this entry
                    entry.source_file = Some(source_file.clone());
                    entries.push(entry);
                },
                Ok(None) => continue, // Skip entries without timestamp
                Err(_) => continue, // Skip invalid lines silently
            }
        }
        
        Ok(entries)
    }

    /// Parse a single JSONL line into a ParsedEntry
    fn parse_jsonl_line(&self, line: &str) -> Result<Option<ParsedEntry>> {
        let raw_value: Value = serde_json::from_str(line)
            .context("Failed to parse JSON line")?;
        
        // Extract timestamp - skip entries without valid timestamp
        let timestamp_str = raw_value
            .get("timestamp")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid timestamp"))?;
        
        let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
            .or_else(|_| DateTime::parse_from_str(timestamp_str, "%Y-%m-%dT%H:%M:%S%.fZ"))
            .with_context(|| format!("Invalid timestamp format: {}", timestamp_str))?
            .with_timezone(&Utc);

        // Apply time filter at entry level if specified
        if let Some(hours) = self.time_filter_hours {
            let cutoff_time = Utc::now() - chrono::Duration::hours(hours as i64);
            if timestamp < cutoff_time {
                return Ok(None); // Skip entries outside time window
            } else {
            }
        }

        // Parse message info if present
        let message = raw_value.get("message")
            .and_then(|v| self.parse_message_info(v));

        // Extract cost if present
        let cost_usd = raw_value.get("costUSD")
            .and_then(|v| v.as_f64());

        // Extract sidechain flag
        let is_sidechain = raw_value.get("isSidechain")
            .and_then(|v| v.as_bool());

        // Convert to HashMap for raw storage
        let raw: HashMap<String, Value> = serde_json::from_value(raw_value)
            .context("Failed to convert to HashMap")?;

        Ok(Some(ParsedEntry {
            timestamp,
            message,
            cost_usd,
            is_sidechain,
            raw,
            source_file: None,  // Will be set by the caller
        }))
    }

    /// Parse message information from raw JSON value
    fn parse_message_info(&self, message_value: &Value) -> Option<MessageInfo> {
        let id = message_value.get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let model = message_value.get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let usage = message_value.get("usage")
            .and_then(|v| self.parse_usage_info(v));
        
        Some(MessageInfo { id, usage, model })
    }

    /// Parse usage information from raw JSON value
    fn parse_usage_info(&self, usage_value: &Value) -> Option<UsageInfo> {
        Some(UsageInfo {
            input_tokens: usage_value.get("input_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            output_tokens: usage_value.get("output_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            cache_creation_input_tokens: usage_value.get("cache_creation_input_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            cache_read_input_tokens: usage_value.get("cache_read_input_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
        })
    }

    /// Perform global deduplication and sorting
    fn deduplicate_and_sort(&self, mut entries: Vec<ParsedEntry>) -> Result<Vec<ParsedEntry>> {
        // First, sort all entries by timestamp for deterministic deduplication
        entries.sort_by_key(|e| e.timestamp);
        
        // Create set to track seen message/request ID combinations
        let mut seen_hashes = HashSet::new();
        let mut deduplicated = Vec::new();
        
        for entry in entries {
            if let Some(hash) = self.create_unique_hash(&entry) {
                if seen_hashes.insert(hash) {
                    deduplicated.push(entry);
                }
                // Skip duplicates silently
            } else {
                // Include entries without proper IDs (shouldn't happen normally)
                deduplicated.push(entry);
            }
        }
        
        Ok(deduplicated)
    }

    /// Create unique hash for deduplication (messageId:requestId)
    fn create_unique_hash(&self, entry: &ParsedEntry) -> Option<String> {
        // Try to get message ID from the message structure
        let message_id = entry.message.as_ref()
            .and_then(|m| m.id.as_ref())
            .map(|s| s.as_str())
            .or_else(|| {
                // Fallback: try to get it from raw JSON
                entry.raw.get("message")
                    .and_then(|v| v.get("id"))
                    .and_then(|v| v.as_str())
            })?;

        // Get request ID from raw JSON
        let request_id = entry.raw.get("requestId")
            .and_then(|v| v.as_str())?;

        Some(format!("{}:{}", message_id, request_id))
    }
}

impl Default for DataAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for common use cases
impl DataAggregator {
    /// Load entries for today only
    pub async fn load_today_entries(&self) -> Result<Vec<ParsedEntry>> {
        let aggregator = DataAggregator::new().with_time_filter(24);
        let all_entries = aggregator.load_all_entries().await?;
        
        let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0)
            .unwrap().and_utc();
        
        let today_entries = all_entries
            .into_iter()
            .filter(|entry| entry.timestamp >= today_start)
            .collect();
            
        Ok(today_entries)
    }

    /// Load entries for recent hours (for block calculations)
    pub async fn load_recent_entries(&self, hours: u32) -> Result<Vec<ParsedEntry>> {
        let aggregator = DataAggregator::new().with_time_filter(hours);
        aggregator.load_all_entries().await
    }

    /// Load entries from a specific session transcript file
    pub async fn load_session_entries(&self, transcript_path: &std::path::Path) -> Result<Vec<ParsedEntry>> {
        self.parse_transcript_file_streaming(transcript_path)
    }
}