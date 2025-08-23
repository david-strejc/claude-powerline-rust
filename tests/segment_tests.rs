use claude_powerline_rust::segments::*;
use claude_powerline_rust::utils::*;
use chrono::{DateTime, Utc};
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_block_segment_calculation() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("projects").join("test-project");
    fs::create_dir_all(&project_dir).await.unwrap();
    
    // Create transcript with 5-hour window entries
    let now = Utc::now();
    let block_start = now - chrono::Duration::hours(2);
    let transcript_content = format!(
        r#"{{"timestamp":"{}","message":{{"id":"msg-1","usage":{{"input_tokens":1000,"output_tokens":500}},"model":"claude-3-opus"}},"costUSD":0.15,"requestId":"req-1"}}
{{"timestamp":"{}","message":{{"id":"msg-2","usage":{{"input_tokens":1500,"output_tokens":750}},"model":"claude-3-opus"}},"costUSD":0.225,"requestId":"req-2"}}"#,
        block_start.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
        (block_start + chrono::Duration::minutes(30)).format("%Y-%m-%dT%H:%M:%S%.3fZ")
    );
    
    let transcript_path = project_dir.join("block-session.jsonl");
    fs::write(&transcript_path, transcript_content).await.unwrap();
    
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    
    let block_segment = BlockSegment::new();
    let block_info = block_segment.get_active_block_info().await.unwrap();
    
    // Verify block calculations
    assert!(block_info.cost.is_some());
    assert!(block_info.tokens.is_some());
    assert!(block_info.weighted_tokens.is_some());
    
    let cost = block_info.cost.unwrap();
    let tokens = block_info.tokens.unwrap();
    let weighted_tokens = block_info.weighted_tokens.unwrap();
    
    assert!((cost - 0.375).abs() < 0.001); // 0.15 + 0.225
    assert_eq!(tokens, 3750); // 1000+500+1500+750
    assert_eq!(weighted_tokens, 18750); // 5x weight for Opus model
    
    // Time remaining should be positive (within 5-hour window)
    assert!(block_info.time_remaining.is_some());
    assert!(block_info.time_remaining.unwrap() > 0);
}

#[tokio::test]
async fn test_today_segment_calculation() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("projects").join("test-project");
    fs::create_dir_all(&project_dir).await.unwrap();
    
    // Create transcript with today's entries
    let today = Utc::now();
    let transcript_content = format!(
        r#"{{"timestamp":"{}","message":{{"id":"msg-today-1","usage":{{"input_tokens":800,"output_tokens":400}},"model":"claude-3-5-sonnet"}},"costUSD":0.04,"requestId":"req-1"}}
{{"timestamp":"{}","message":{{"id":"msg-today-2","usage":{{"input_tokens":1200,"output_tokens":600}},"model":"claude-3-5-sonnet"}},"costUSD":0.06,"requestId":"req-2"}}"#,
        today.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
        (today + chrono::Duration::hours(1)).format("%Y-%m-%dT%H:%M:%S%.3fZ")
    );
    
    let transcript_path = project_dir.join("today-session.jsonl");  
    fs::write(&transcript_path, transcript_content).await.unwrap();
    
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    
    let today_segment = TodaySegment::new();
    let today_info = today_segment.get_today_info().await.unwrap();
    
    assert!(today_info.cost.is_some());
    assert!(today_info.tokens.is_some());
    assert!(today_info.message_count.is_some());
    
    let cost = today_info.cost.unwrap();
    let tokens = today_info.tokens.unwrap();
    let messages = today_info.message_count.unwrap();
    
    assert!((cost - 0.10).abs() < 0.001); // 0.04 + 0.06
    assert_eq!(tokens, 3000); // 800+400+1200+600
    assert_eq!(messages, 2);
}

#[tokio::test]
async fn test_session_segment_calculation() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("projects").join("test-project");
    fs::create_dir_all(&project_dir).await.unwrap();
    
    let transcript_content = r#"{"timestamp":"2024-01-01T10:00:00.000Z","message":{"id":"msg-1","usage":{"input_tokens":500,"output_tokens":250},"model":"claude-3-5-sonnet"},"costUSD":0.025,"requestId":"req-1"}
{"timestamp":"2024-01-01T10:05:00.000Z","message":{"id":"msg-2","usage":{"input_tokens":750,"output_tokens":375},"model":"claude-3-5-sonnet"},"costUSD":0.0375,"requestId":"req-2"}"#;
    
    let session_id = "test-session-123";
    let transcript_path = project_dir.join(format!("{}.jsonl", session_id));
    fs::write(&transcript_path, transcript_content).await.unwrap();
    
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("CLAUDE_SESSION_ID", session_id);
    
    let session_segment = SessionSegment::new();
    let session_info = session_segment.get_session_info().await.unwrap();
    
    assert!(session_info.cost.is_some());
    assert!(session_info.tokens.is_some());
    assert!(session_info.message_count.is_some());
    assert!(session_info.duration_minutes.is_some());
    
    let cost = session_info.cost.unwrap();
    let tokens = session_info.tokens.unwrap();
    let messages = session_info.message_count.unwrap();
    let duration = session_info.duration_minutes.unwrap();
    
    assert!((cost - 0.0625).abs() < 0.001); // 0.025 + 0.0375
    assert_eq!(tokens, 1875); // 500+250+750+375
    assert_eq!(messages, 2);
    assert_eq!(duration, 5); // 5 minutes between entries
    
    std::env::remove_var("CLAUDE_SESSION_ID");
}

#[tokio::test]
async fn test_git_segment() {
    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    
    // Initialize git repo
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git repo");
        
    // Set up git config to avoid warnings
    std::process::Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to set git email");
        
    std::process::Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to set git name");
    
    // Create a test file and commit
    fs::write(repo_path.join("test.txt"), "test content").await.unwrap();
    
    std::process::Command::new("git")
        .args(&["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("Failed to add files");
        
    std::process::Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to commit");
    
    // Change to the test directory and test git segment
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(repo_path).unwrap();
    
    let git_segment = GitSegment::new();
    let git_info = git_segment.get_git_info().await.unwrap();
    
    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
    
    assert!(git_info.branch.is_some());
    assert!(git_info.sha.is_some());
    
    let branch = git_info.branch.unwrap();
    assert!(branch == "main" || branch == "master"); // Git may use either default
    
    let sha = git_info.sha.unwrap();
    assert_eq!(sha.len(), 7); // Short SHA should be 7 characters
}

#[tokio::test]
async fn test_metrics_segment() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("projects").join("test-project");
    fs::create_dir_all(&project_dir).await.unwrap();
    
    // Create transcript with varied response times
    let now = Utc::now();
    let transcript_content = format!(
        r#"{{"timestamp":"{}","message":{{"id":"msg-1","usage":{{"input_tokens":500,"output_tokens":250}}}},"response_time_ms":150,"costUSD":0.025,"requestId":"req-1"}}
{{"timestamp":"{}","message":{{"id":"msg-2","usage":{{"input_tokens":750,"output_tokens":375}}}},"response_time_ms":200,"costUSD":0.0375,"requestId":"req-2"}}
{{"timestamp":"{}","message":{{"id":"msg-3","usage":{{"input_tokens":600,"output_tokens":300}}}},"response_time_ms":180,"costUSD":0.03,"requestId":"req-3"}}"#,
        (now - chrono::Duration::hours(1)).format("%Y-%m-%dT%H:%M:%S%.3fZ"),
        (now - chrono::Duration::minutes(30)).format("%Y-%m-%dT%H:%M:%S%.3fZ"),
        (now - chrono::Duration::minutes(10)).format("%Y-%m-%dT%H:%M:%S%.3fZ")
    );
    
    let transcript_path = project_dir.join("metrics-session.jsonl");
    fs::write(&transcript_path, transcript_content).await.unwrap();
    
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    
    let metrics_segment = MetricsSegment::new();
    let metrics_info = metrics_segment.get_metrics_info().await.unwrap();
    
    assert!(metrics_info.avg_response_time.is_some());
    assert!(metrics_info.last_response_time.is_some());
    assert!(metrics_info.session_duration.is_some());
    assert!(metrics_info.message_count.is_some());
    
    let avg_response = metrics_info.avg_response_time.unwrap();
    let last_response = metrics_info.last_response_time.unwrap();
    let message_count = metrics_info.message_count.unwrap();
    
    assert!((avg_response - 176.67).abs() < 1.0); // (150+200+180)/3 ≈ 176.67
    assert_eq!(last_response, 180.0);
    assert_eq!(message_count, 3);
}

#[tokio::test]
async fn test_context_segment() {
    let context_segment = ContextSegment::new();
    
    // Test with environment variables
    std::env::set_var("CLAUDE_CONTEXT_TOKENS_USED", "34040");
    std::env::set_var("CLAUDE_CONTEXT_TOKENS_TOTAL", "200000");
    std::env::set_var("CLAUDE_AUTO_COMPACT_THRESHOLD", "85");
    
    let context_info = context_segment.get_context_info().await.unwrap();
    
    assert!(context_info.tokens_used.is_some());
    assert!(context_info.tokens_remaining.is_some());
    assert!(context_info.percentage_used.is_some());
    assert!(context_info.auto_compact_threshold.is_some());
    
    let used = context_info.tokens_used.unwrap();
    let remaining = context_info.tokens_remaining.unwrap();
    let percentage = context_info.percentage_used.unwrap();
    let threshold = context_info.auto_compact_threshold.unwrap();
    
    assert_eq!(used, 34040);
    assert_eq!(remaining, 165960); // 200000 - 34040
    assert!((percentage - 17.02).abs() < 0.1); // 34040/200000 * 100
    assert_eq!(threshold, 85.0);
    
    // Cleanup
    std::env::remove_var("CLAUDE_CONTEXT_TOKENS_USED");
    std::env::remove_var("CLAUDE_CONTEXT_TOKENS_TOTAL");  
    std::env::remove_var("CLAUDE_AUTO_COMPACT_THRESHOLD");
}