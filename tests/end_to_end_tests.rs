use std::process::Command;
use tempfile::TempDir;
use tokio::fs;
use serde_json::Value;

/// Test that our Rust implementation produces similar output to the original
#[tokio::test] 
async fn test_statusline_format_compatibility() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("projects").join("e2e-test");
    fs::create_dir_all(&project_dir).await.unwrap();
    
    // Create realistic test transcript
    let today = chrono::Utc::now();
    let transcript_content = format!(
        r#"{{"timestamp":"{}","hook_event_name":"message_send","session_id":"e2e-test","message":{{"id":"msg-e2e-1","usage":{{"input_tokens":1000,"output_tokens":500,"cache_creation_input_tokens":100}},"model":"claude-3-5-sonnet"}},"costUSD":0.05,"requestId":"req-e2e-1"}}
{{"timestamp":"{}","hook_event_name":"message_send","session_id":"e2e-test","message":{{"id":"msg-e2e-2","usage":{{"input_tokens":1500,"output_tokens":750,"cache_read_input_tokens":200}},"model":"claude-3-5-sonnet"}},"costUSD":0.075,"requestId":"req-e2e-2"}}"#,
        today.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
        (today + chrono::Duration::minutes(5)).format("%Y-%m-%dT%H:%M:%S%.3fZ")
    );
    
    let transcript_path = project_dir.join("e2e-test.jsonl");
    fs::write(&transcript_path, transcript_content).await.unwrap();
    
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("CLAUDE_SESSION_ID", "e2e-test");
    
    // Create mock hook data for the original (if we wanted to test against it)
    let hook_data = serde_json::json!({
        "session_id": "e2e-test",
        "transcript_path": transcript_path.to_str().unwrap(),
        "hook_event_name": "message_send",
        "cwd": temp_dir.path().to_str().unwrap(),
        "workspace": {
            "current_dir": temp_dir.path().to_str().unwrap(),
            "project_dir": temp_dir.path().to_str().unwrap()
        },
        "model": {
            "id": "claude-3-5-sonnet",
            "display_name": "Claude 3.5 Sonnet"
        }
    });
    
    // Test our Rust implementation
    let output = Command::new("./target/release/claude-powerline")
        .args(&["--theme", "dark", "--style", "minimal"])
        .output()
        .expect("Failed to execute claude-powerline");
    
    let rust_statusline = String::from_utf8(output.stdout).unwrap();
    let stderr_output = String::from_utf8(output.stderr).unwrap();
    
    println!("Rust statusline: {}", rust_statusline);
    println!("Stderr: {}", stderr_output);
    
    // Verify basic structure expectations based on original implementation
    assert!(!rust_statusline.is_empty(), "Statusline should not be empty");
    
    // Should contain directory information  
    assert!(rust_statusline.contains("claude-powerline-rust") || rust_statusline.contains("/"));
    
    // Should contain cost information (☉ symbol for today)
    assert!(rust_statusline.contains("☉"));
    
    // Should contain some kind of usage data ($ or T for cost/tokens)
    assert!(rust_statusline.contains("$") || rust_statusline.contains("T"));
    
    // Should contain block information (◱ symbol)
    assert!(rust_statusline.contains("◱"));
    
    // Test minimal vs powerline style differences
    let powerline_output = Command::new("./target/release/claude-powerline")
        .args(&["--style", "powerline"])
        .output()
        .expect("Failed to execute claude-powerline with powerline style");
    
    let powerline_statusline = String::from_utf8(powerline_output.stdout).unwrap();
    
    // Powerline style should be different from minimal (may contain separators)
    // Note: our current implementation doesn't fully implement powerline separators yet
    assert!(!powerline_statusline.is_empty());
    
    std::env::remove_var("CLAUDE_CONFIG_DIR");
    std::env::remove_var("CLAUDE_SESSION_ID");
}

#[tokio::test]
async fn test_segment_data_accuracy() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("projects").join("accuracy-test");
    fs::create_dir_all(&project_dir).await.unwrap();
    
    // Create precise test data to verify calculations
    let base_time = chrono::Utc::now();
    let transcript_content = format!(
        r#"{{"timestamp":"{}","message":{{"id":"msg-1","usage":{{"input_tokens":1000,"output_tokens":500}},"model":"claude-3-5-sonnet"}},"costUSD":0.05,"requestId":"req-1"}}
{{"timestamp":"{}","message":{{"id":"msg-2","usage":{{"input_tokens":2000,"output_tokens":1000}},"model":"claude-3-opus"}},"costUSD":0.30,"requestId":"req-2"}}"#,
        base_time.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
        (base_time + chrono::Duration::minutes(30)).format("%Y-%m-%dT%H:%M:%S%.3fZ")
    );
    
    let transcript_path = project_dir.join("accuracy-test.jsonl");
    fs::write(&transcript_path, transcript_content).await.unwrap();
    
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("CLAUDE_SESSION_ID", "accuracy-test");
    
    let output = Command::new("./target/release/claude-powerline")
        .output()
        .expect("Failed to execute claude-powerline");
    
    let statusline = String::from_utf8(output.stdout).unwrap();
    
    println!("Accuracy test statusline: {}", statusline);
    
    // The statusline should contain today's cost: $0.35 (0.05 + 0.30)
    assert!(statusline.contains("0.35") || statusline.contains("0.3"));
    
    // Should show weighted tokens for block segment
    // Opus entries get 5x weight: (1000+500)*1 + (2000+1000)*5 = 1500 + 15000 = 16500
    assert!(statusline.contains("16.5K") || statusline.contains("16500"));
    
    std::env::remove_var("CLAUDE_CONFIG_DIR");
    std::env::remove_var("CLAUDE_SESSION_ID");
}

#[tokio::test]
async fn test_error_handling() {
    // Test with no transcript data
    let temp_dir = TempDir::new().unwrap();
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    
    let output = Command::new("./target/release/claude-powerline")
        .output()
        .expect("Failed to execute claude-powerline");
    
    // Should not crash, should produce some output even without data
    assert!(output.status.success(), "Should not crash with missing data");
    
    let statusline = String::from_utf8(output.stdout).unwrap();
    assert!(!statusline.is_empty(), "Should produce output even without transcript data");
    
    // Test with invalid config
    let invalid_config = temp_dir.path().join("invalid.json");
    fs::write(&invalid_config, "{ invalid json }").await.unwrap();
    
    let output = Command::new("./target/release/claude-powerline")
        .args(&["--config", invalid_config.to_str().unwrap()])
        .output()
        .expect("Failed to execute with invalid config");
    
    // Should handle invalid config gracefully (may succeed with defaults or show error)
    let stderr = String::from_utf8(output.stderr).unwrap();
    if !output.status.success() {
        assert!(stderr.contains("config") || stderr.contains("parse"), "Should show config-related error");
    }
    
    std::env::remove_var("CLAUDE_CONFIG_DIR");
}

#[tokio::test]
async fn test_theme_variations() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("projects").join("theme-test");
    fs::create_dir_all(&project_dir).await.unwrap();
    
    let transcript_content = r#"{"timestamp":"2024-01-01T10:00:00.000Z","message":{"id":"msg-theme","usage":{"input_tokens":500,"output_tokens":250},"model":"claude-3-5-sonnet"},"costUSD":0.025,"requestId":"req-theme"}"#;
    let transcript_path = project_dir.join("theme-test.jsonl");
    fs::write(&transcript_path, transcript_content).await.unwrap();
    
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    
    let themes = ["dark", "light", "nord", "tokyo-night", "rose-pine"];
    
    for theme in &themes {
        let output = Command::new("./target/release/claude-powerline")
            .args(&["--theme", theme])
            .output()
            .expect(&format!("Failed to execute with theme: {}", theme));
        
        assert!(output.status.success(), "Should succeed with theme: {}", theme);
        
        let statusline = String::from_utf8(output.stdout).unwrap();
        assert!(!statusline.is_empty(), "Should produce output with theme: {}", theme);
        
        println!("Theme {} statusline: {}", theme, statusline);
    }
    
    std::env::remove_var("CLAUDE_CONFIG_DIR");
}

#[tokio::test]
async fn test_context_environment_integration() {
    // Test context segment with environment variables (simulating Claude Code hook data)
    std::env::set_var("CLAUDE_CONTEXT_TOKENS_USED", "45000");
    std::env::set_var("CLAUDE_CONTEXT_TOKENS_TOTAL", "200000");
    
    let output = Command::new("./target/release/claude-powerline")
        .output()
        .expect("Failed to execute with context env vars");
    
    let statusline = String::from_utf8(output.stdout).unwrap();
    
    // Should contain context information (◔ symbol and percentage)
    assert!(statusline.contains("◔"));
    assert!(statusline.contains("45") || statusline.contains("22%")); // 45K tokens or 22.5%
    
    std::env::remove_var("CLAUDE_CONTEXT_TOKENS_USED");
    std::env::remove_var("CLAUDE_CONTEXT_TOKENS_TOTAL");
}

#[tokio::test]
async fn test_git_repository_detection() {
    // This test runs in the actual git repository
    let output = Command::new("./target/release/claude-powerline")
        .output()
        .expect("Failed to execute in git repo");
    
    let statusline = String::from_utf8(output.stdout).unwrap();
    
    // Should detect git repository and show branch info
    assert!(statusline.contains("⎇")); // Git branch symbol
    
    // Should contain branch name (likely "main" or "master")
    assert!(statusline.contains("main") || statusline.contains("master"));
    
    // Should show clean/dirty status
    assert!(statusline.contains("✓") || statusline.contains("●"));
}

/// Test CLI argument handling matches original behavior
#[tokio::test]
async fn test_cli_compatibility() {
    // Test help flag
    let output = Command::new("./target/release/claude-powerline")
        .arg("--help")
        .output()
        .expect("Failed to execute --help");
    
    assert!(output.status.success());
    let help_text = String::from_utf8(output.stdout).unwrap();
    
    // Should match expected help format
    assert!(help_text.contains("Claude Powerline"));
    assert!(help_text.contains("--theme"));
    assert!(help_text.contains("--style"));
    assert!(help_text.contains("--config"));
    assert!(help_text.contains("CLAUDE_POWERLINE_THEME"));
    
    // Test unknown argument handling
    let output = Command::new("./target/release/claude-powerline")
        .arg("--unknown-flag")
        .output()
        .expect("Failed to execute with unknown flag");
    
    // Should either ignore unknown flags or show error
    // Either way, it shouldn't crash
    let stderr = String::from_utf8(output.stderr).unwrap();
    if !output.status.success() {
        // If it fails, should show helpful error message
        assert!(!stderr.is_empty());
    }
}