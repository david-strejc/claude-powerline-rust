use claude_powerline_rust::utils::claude::*;
use claude_powerline_rust::utils::*;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_parse_jsonl_content() {
    let jsonl_content = r#"{"timestamp":"2024-01-01T10:00:00.000Z","hook_event_name":"message_send","session_id":"test-session","message":{"id":"msg-001","usage":{"input_tokens":1000,"output_tokens":500},"model":"claude-3-5-sonnet"},"costUSD":0.05,"requestId":"req-001"}
{"timestamp":"2024-01-01T10:01:00.000Z","hook_event_name":"message_send","session_id":"test-session","message":{"id":"msg-002","usage":{"input_tokens":1500,"output_tokens":750},"model":"claude-3-5-sonnet"},"costUSD":0.08,"requestId":"req-002"}"#;

    let entries = parse_jsonl_content(jsonl_content).unwrap();
    
    assert_eq!(entries.len(), 2);
    
    // Test first entry
    assert_eq!(entries[0].message.as_ref().unwrap().id.as_ref().unwrap(), "msg-001");
    assert_eq!(entries[0].message.as_ref().unwrap().usage.as_ref().unwrap().input_tokens.unwrap(), 1000);
    assert_eq!(entries[0].message.as_ref().unwrap().usage.as_ref().unwrap().output_tokens.unwrap(), 500);
    assert_eq!(entries[0].cost_usd.unwrap(), 0.05);
    
    // Test second entry  
    assert_eq!(entries[1].message.as_ref().unwrap().id.as_ref().unwrap(), "msg-002");
    assert_eq!(entries[1].message.as_ref().unwrap().usage.as_ref().unwrap().input_tokens.unwrap(), 1500);
    assert_eq!(entries[1].cost_usd.unwrap(), 0.08);
}

#[tokio::test]
async fn test_transcript_parser_today_entries() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("projects").join("test-project");
    fs::create_dir_all(&project_dir).await.unwrap();
    
    // Create a transcript file with today's entries
    let today = Utc::now();
    let transcript_content = format!(
        r#"{{"timestamp":"{}","message":{{"id":"msg-today-1","usage":{{"input_tokens":500,"output_tokens":250}},"model":"claude-3-5-sonnet"}},"costUSD":0.025,"requestId":"req-1"}}
{{"timestamp":"{}","message":{{"id":"msg-today-2","usage":{{"input_tokens":750,"output_tokens":375}},"model":"claude-3-5-sonnet"}},"costUSD":0.0375,"requestId":"req-2"}}"#,
        today.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
        today.format("%Y-%m-%dT%H:%M:%S%.3fZ")
    );
    
    let transcript_path = project_dir.join("test-session.jsonl");
    fs::write(&transcript_path, transcript_content).await.unwrap();
    
    // Set up environment for testing
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    
    let parser = TranscriptParser::new().unwrap();
    let entries = parser.get_today_entries().await.unwrap();
    
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().all(|e| e.timestamp.date_naive() == today.date_naive()));
}

#[tokio::test]
async fn test_transcript_parser_recent_entries() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("projects").join("test-project");
    fs::create_dir_all(&project_dir).await.unwrap();
    
    let now = Utc::now();
    let recent_time = now - chrono::Duration::hours(2);
    let old_time = now - chrono::Duration::days(2);
    
    let transcript_content = format!(
        r#"{{"timestamp":"{}","message":{{"id":"msg-recent","usage":{{"input_tokens":500,"output_tokens":250}}}},"costUSD":0.025,"requestId":"req-recent"}}
{{"timestamp":"{}","message":{{"id":"msg-old","usage":{{"input_tokens":300,"output_tokens":150}}}},"costUSD":0.015,"requestId":"req-old"}}"#,
        recent_time.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
        old_time.format("%Y-%m-%dT%H:%M:%S%.3fZ")
    );
    
    let transcript_path = project_dir.join("test-session.jsonl");
    fs::write(&transcript_path, transcript_content).await.unwrap();
    
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    
    let parser = TranscriptParser::new().unwrap();
    let entries = parser.get_recent_entries(6).await.unwrap(); // Last 6 hours
    
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].message.as_ref().unwrap().id.as_ref().unwrap(), "msg-recent");
}

#[tokio::test] 
async fn test_unique_hash_generation() {
    let entry = ParsedEntry {
        timestamp: Utc::now(),
        message: Some(MessageInfo {
            id: Some("msg-123".to_string()),
            usage: None,
            model: None,
        }),
        cost_usd: None,
        is_sidechain: None,
        raw: [("requestId".to_string(), serde_json::Value::String("req-456".to_string()))]
            .into_iter()
            .collect(),
    };
    
    let hash = create_unique_hash(&entry).unwrap();
    assert_eq!(hash, "msg-123:req-456");
}

#[tokio::test]
async fn test_claude_paths_discovery() {
    let temp_dir = TempDir::new().unwrap();
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir(&claude_dir).await.unwrap();
    
    // Test environment variable override
    std::env::set_var("CLAUDE_CONFIG_DIR", claude_dir.to_str().unwrap());
    let paths = get_claude_paths().unwrap();
    assert!(paths.contains(&claude_dir));
    
    std::env::remove_var("CLAUDE_CONFIG_DIR");
}