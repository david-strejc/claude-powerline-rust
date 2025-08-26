use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures::future::try_join_all;
use std::sync::OnceLock;
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeHookData {
    pub hook_event_name: String,
    pub session_id: String,
    pub transcript_path: String,
    pub cwd: String,
    pub model: ModelInfo,
    pub workspace: WorkspaceInfo,
    pub version: Option<String>,
    pub output_style: Option<OutputStyle>,
    pub cost: Option<CostInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub current_dir: String,
    pub project_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputStyle {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostInfo {
    pub total_cost_usd: f64,
    pub total_duration_ms: u64,
    pub total_api_duration_ms: u64,
    pub total_lines_added: u64,
    pub total_lines_removed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedEntry {
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<MessageInfo>,
    #[serde(rename = "costUSD", skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(skip)]
    pub source_file: Option<String>,  // Track which transcript file this entry came from
    #[serde(rename = "isSidechain", skip_serializing_if = "Option::is_none")]
    pub is_sidechain: Option<bool>,
    #[serde(flatten)]
    pub raw: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageInfo {
    pub id: Option<String>,
    pub usage: Option<UsageInfo>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub cache_creation_input_tokens: Option<u32>,
    pub cache_read_input_tokens: Option<u32>,
}

/// High-performance shared transcript parser with memory mapping and caching
pub struct TranscriptParser {
    cache: Arc<DashMap<PathBuf, Arc<Vec<ParsedEntry>>>>,
    claude_paths: Vec<PathBuf>,
}

impl TranscriptParser {
    pub fn new() -> Result<Self> {
        let claude_paths = get_claude_paths()?;
        Ok(Self {
            cache: Arc::new(DashMap::new()),
            claude_paths,
        })
    }

    /// Load entries with optional time filter, using shared parsing and caching
    pub async fn load_entries(
        &self,
        time_filter: Option<impl Fn(&ParsedEntry) -> bool + Send + Sync>,
        file_filter: Option<impl Fn(&Path, DateTime<Utc>) -> bool + Send + Sync>,
    ) -> Result<Vec<ParsedEntry>> {
        let project_paths = find_project_paths(&self.claude_paths).await?;
        let mut all_entries = Vec::new();

        // Collect all transcript files with metadata
        let mut file_tasks = Vec::new();
        for project_path in project_paths {
            let entries = fs::read_dir(&project_path).await?;
            let mut entries = entries;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                    if let Ok(metadata) = entry.metadata().await {
                        let mtime = metadata.modified()?.into();
                        if file_filter.as_ref().map_or(true, |f| f(&path, mtime)) {
                            file_tasks.push(self.parse_file_cached(path));
                        }
                    }
                }
            }
        }

        // Parse all files in parallel
        let results = try_join_all(file_tasks).await?;
        
        // Flatten results and apply time filter
        for entries in results {
            if let Some(ref filter) = time_filter {
                all_entries.extend(entries.iter().filter(|e| filter(e)).cloned());
            } else {
                all_entries.extend(entries.iter().cloned());
            }
        }

        // Sort by timestamp for consistent deduplication
        all_entries.sort_by_key(|e| e.timestamp);

        // Deduplicate entries
        let mut seen_hashes = std::collections::HashSet::new();
        let mut dedup_entries = Vec::new();

        for entry in all_entries {
            if let Some(hash) = create_unique_hash(&entry) {
                if seen_hashes.insert(hash) {
                    dedup_entries.push(entry);
                }
            } else {
                dedup_entries.push(entry);
            }
        }

        Ok(dedup_entries)
    }

    /// Parse a single file with caching and memory mapping
    async fn parse_file_cached(&self, path: PathBuf) -> Result<Arc<Vec<ParsedEntry>>> {
        // Check cache first
        if let Some(cached) = self.cache.get(&path) {
            return Ok(cached.clone());
        }

        // Parse file with memory mapping for large files
        let entries = parse_jsonl_file_mmap(&path).await?;
        let entries_arc = Arc::new(entries);
        
        // Cache the result
        self.cache.insert(path, entries_arc.clone());
        
        Ok(entries_arc)
    }

    /// Get entries for a specific time range (optimized for recent data)
    pub async fn get_recent_entries(
        &self,
        hours_back: u32,
    ) -> Result<Vec<ParsedEntry>> {
        let cutoff = Utc::now() - chrono::Duration::hours(hours_back as i64);
        
        self.load_entries(
            Some(move |entry: &ParsedEntry| entry.timestamp >= cutoff),
            Some(move |_path: &Path, mtime: DateTime<Utc>| mtime >= cutoff),
        ).await
    }

    /// Get entries for today only (optimized for daily segments)
    pub async fn get_today_entries(&self) -> Result<Vec<ParsedEntry>> {
        let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0)
            .unwrap().and_utc();
        
        self.load_entries(
            Some(move |entry: &ParsedEntry| entry.timestamp >= today_start),
            Some(move |_path: &Path, mtime: DateTime<Utc>| mtime >= today_start),
        ).await
    }
}

/// Memory-mapped JSONL parsing for maximum performance
async fn parse_jsonl_file_mmap(path: &Path) -> Result<Vec<ParsedEntry>> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))?;
    
    let metadata = file.metadata()?;
    let file_size = metadata.len();
    
    // For small files, use regular parsing
    if file_size < 1024 * 1024 {
        return parse_jsonl_file_regular(path).await;
    }

    // Use memory mapping for large files
    let mmap = unsafe {
        Mmap::map(&file)
            .with_context(|| format!("Failed to mmap file: {}", path.display()))?
    };

    let content = std::str::from_utf8(&mmap)
        .with_context(|| format!("Invalid UTF-8 in file: {}", path.display()))?;

    parse_jsonl_content(content)
}

/// Regular file parsing for smaller files
async fn parse_jsonl_file_regular(path: &Path) -> Result<Vec<ParsedEntry>> {
    let content = fs::read_to_string(path).await
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    
    parse_jsonl_content(&content)
}

/// Parse JSONL content with SIMD JSON for maximum performance
pub fn parse_jsonl_content(content: &str) -> Result<Vec<ParsedEntry>> {
    let mut entries = Vec::new();
    
    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match parse_jsonl_line(trimmed) {
            Ok(Some(entry)) => entries.push(entry),
            Ok(None) => continue, // Skip entries without timestamp
            Err(_e) => {
                // Silently skip invalid lines instead of showing warnings
                continue;
            }
        }
    }

    Ok(entries)
}

/// Parse a single JSONL line with error handling
fn parse_jsonl_line(line: &str) -> Result<Option<ParsedEntry>> {
    // Try SIMD JSON first for performance
    let mut line_bytes = line.as_bytes().to_vec();
    let raw_value = match simd_json::to_borrowed_value(&mut line_bytes) {
        Ok(val) => serde_json::to_value(&val).unwrap(),
        Err(_) => {
            // Fallback to regular serde_json
            serde_json::from_str(line)?
        }
    };

    // Extract timestamp - skip entries without valid timestamp
    let timestamp_str = raw_value
        .get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid timestamp"))?;
    
    let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
        .or_else(|_| DateTime::parse_from_str(timestamp_str, "%Y-%m-%dT%H:%M:%S%.fZ"))
        .with_context(|| format!("Invalid timestamp format: {}", timestamp_str))?
        .with_timezone(&Utc);

    // Parse message info if present
    let message = raw_value.get("message")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    // Extract cost if present
    let cost_usd = raw_value.get("costUSD")
        .and_then(|v| v.as_f64());

    // Extract sidechain flag
    let is_sidechain = raw_value.get("isSidechain")
        .and_then(|v| v.as_bool());

    // Convert to HashMap for raw storage
    let raw: HashMap<String, serde_json::Value> = serde_json::from_value(raw_value)?;

    Ok(Some(ParsedEntry {
        timestamp,
        message,
        cost_usd,
        is_sidechain,
        raw,
        source_file: None,  // Not used in legacy parser
    }))
}

/// Create unique hash for deduplication
pub fn create_unique_hash(entry: &ParsedEntry) -> Option<String> {
    let message_id = entry.message.as_ref()
        .and_then(|m| m.id.as_ref())
        .map(|s| s.as_str())
        .or_else(|| {
            entry.raw.get("message")
                .and_then(|v| v.get("id"))
                .and_then(|v| v.as_str())
        })?;

    let request_id = entry.raw.get("requestId")
        .and_then(|v| v.as_str())?;

    Some(format!("{}:{}", message_id, request_id))
}

/// Get Claude configuration paths with cross-platform support
pub fn get_claude_paths() -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    // Check environment variable first
    if let Ok(env_paths) = std::env::var("CLAUDE_CONFIG_DIR") {
        let separator = if cfg!(windows) { ';' } else { ',' };
        for path_str in env_paths.split(separator) {
            let path = PathBuf::from(path_str.trim());
            if path.exists() {
                paths.push(path);
            }
        }
    }

    // Fallback to platform-specific default locations
    if paths.is_empty() {
        if let Some(home) = dirs::home_dir() {
            if cfg!(windows) {
                // Windows: %APPDATA%\Claude and %USERPROFILE%\.claude
                if let Some(appdata) = std::env::var_os("APPDATA") {
                    let appdata_claude = PathBuf::from(appdata).join("Claude");
                    if appdata_claude.exists() {
                        paths.push(appdata_claude);
                    }
                }
                let user_claude = home.join(".claude");
                if user_claude.exists() {
                    paths.push(user_claude);
                }
            } else if cfg!(target_os = "macos") {
                // macOS: ~/Library/Application Support/Claude, ~/.config/claude, ~/.claude
                let app_support = home.join("Library").join("Application Support").join("Claude");
                let config_path = home.join(".config").join("claude");
                let claude_path = home.join(".claude");
                
                // Check in order of preference
                if app_support.exists() {
                    paths.push(app_support);
                } else if config_path.exists() {
                    paths.push(config_path);
                } else if claude_path.exists() {
                    paths.push(claude_path);
                }
            } else {
                // Linux/Unix: ~/.config/claude and ~/.claude
                let config_path = home.join(".config").join("claude");
                let claude_path = home.join(".claude");

                // Check both paths and add them if they exist (config first)
                if config_path.exists() {
                    paths.push(config_path);
                }
                if claude_path.exists() {
                    paths.push(claude_path);
                }
            }
        }
    }

    if paths.is_empty() {
        anyhow::bail!("No Claude configuration directory found");
    }

    Ok(paths)
}

/// Find all project paths within Claude directories
pub async fn find_project_paths(claude_paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut project_paths = Vec::new();

    for claude_path in claude_paths {
        let projects_dir = claude_path.join("projects");
        if projects_dir.exists() {
            let mut entries = fs::read_dir(&projects_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if entry.file_type().await?.is_dir() {
                    project_paths.push(entry.path());
                }
            }
        }
    }

    Ok(project_paths)
}

/// Find transcript file for a specific session
pub async fn find_transcript_file(session_id: &str) -> Result<Option<PathBuf>> {
    let claude_paths = get_claude_paths()?;
    let project_paths = find_project_paths(&claude_paths).await?;

    for project_path in project_paths {
        let transcript_path = project_path.join(format!("{}.jsonl", session_id));
        if transcript_path.exists() {
            return Ok(Some(transcript_path));
        }
    }

    Ok(None)
}

/// Global transcript parser instance
static PARSER: OnceLock<TranscriptParser> = OnceLock::new();

/// Get global transcript parser instance
pub fn get_transcript_parser() -> &'static TranscriptParser {
    PARSER.get_or_init(|| TranscriptParser::new().unwrap())
}