use claude_powerline_rust::{config, themes::*};
use std::process::Command;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_full_statusline_generation() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("projects").join("test-project");
    fs::create_dir_all(&project_dir).await.unwrap();
    
    // Create a realistic transcript file
    let today = chrono::Utc::now();
    let transcript_content = format!(
        r#"{{"timestamp":"{}","hook_event_name":"message_send","session_id":"integration-test","message":{{"id":"msg-int-1","usage":{{"input_tokens":1000,"output_tokens":500,"cache_creation_input_tokens":100}},"model":"claude-3-5-sonnet"}},"costUSD":0.05,"requestId":"req-int-1"}}
{{"timestamp":"{}","hook_event_name":"message_send","session_id":"integration-test","message":{{"id":"msg-int-2","usage":{{"input_tokens":1500,"output_tokens":750,"cache_read_input_tokens":200}},"model":"claude-3-5-sonnet"}},"costUSD":0.075,"requestId":"req-int-2"}}
{{"timestamp":"{}","hook_event_name":"message_send","session_id":"integration-test","message":{{"id":"msg-int-3","usage":{{"input_tokens":800,"output_tokens":400}},"model":"claude-3-5-sonnet"}},"costUSD":0.04,"requestId":"req-int-3"}}"#,
        (today - chrono::Duration::hours(2)).format("%Y-%m-%dT%H:%M:%S%.3fZ"),
        (today - chrono::Duration::hours(1)).format("%Y-%m-%dT%H:%M:%S%.3fZ"),
        (today - chrono::Duration::minutes(30)).format("%Y-%m-%dT%H:%M:%S%.3fZ")
    );
    
    let transcript_path = project_dir.join("integration-test.jsonl");
    fs::write(&transcript_path, transcript_content).await.unwrap();
    
    // Create a test config
    let config_content = r#"{
        "theme": "dark",
        "style": "minimal",
        "segments": {
            "directory": {"enabled": true},
            "git": {"enabled": true, "showSha": true},
            "session": {"enabled": true, "type": "tokens"},
            "today": {"enabled": true, "type": "cost"},
            "block": {"enabled": true, "type": "weighted"},
            "context": {"enabled": false}
        }
    }"#;
    
    let config_path = temp_dir.path().join("test-config.json");
    fs::write(&config_path, config_content).await.unwrap();
    
    // Set environment variables
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("CLAUDE_SESSION_ID", "integration-test");
    
    // Test the binary execution
    let output = Command::new("./target/release/claude-powerline")
        .args(&[
            "--config", config_path.to_str().unwrap(),
            "--theme", "dark"
        ])
        .output()
        .expect("Failed to execute claude-powerline");
    
    let statusline = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    
    // Print for debugging
    println!("Statusline output: {}", statusline);
    println!("Stderr: {}", stderr);
    
    // Verify the statusline contains expected segments
    assert!(!statusline.is_empty(), "Statusline should not be empty");
    
    // Should contain directory segment (showing current directory name)
    assert!(statusline.contains("claude-powerline-rust") || statusline.contains("/"));
    
    // Should contain usage information 
    assert!(statusline.contains("$") || statusline.contains("T")); // Cost or tokens
    
    // Should contain today segment
    assert!(statusline.contains("☉")); // Today symbol
    
    // Should contain block segment  
    assert!(statusline.contains("◱")); // Block symbol
    
    // Clean up environment
    std::env::remove_var("CLAUDE_CONFIG_DIR");
    std::env::remove_var("CLAUDE_SESSION_ID");
}

#[tokio::test]
async fn test_config_loading() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a config file
    let config_content = r#"{
        "theme": "tokyo-night",
        "style": "powerline", 
        "segments": {
            "directory": {"enabled": true, "showBasename": true},
            "git": {"enabled": true, "showSha": false, "showWorkingTree": true},
            "session": {"enabled": true, "type": "both", "costSource": "official"},
            "today": {"enabled": false}
        }
    }"#;
    
    let config_path = temp_dir.path().join("custom-config.json");
    fs::write(&config_path, config_content).await.unwrap();
    
    // Load config
    let config = config::load_config(Some(config_path)).await.unwrap();
    
    // Verify config values
    assert_eq!(config.theme, "tokyo-night");
    assert_eq!(config.style, "powerline");
    
    // Test segment configs
    assert!(config.segments.directory.as_ref().unwrap().enabled);
    assert_eq!(config.segments.directory.as_ref().unwrap().show_basename, Some(true));
    
    assert!(config.segments.git.as_ref().unwrap().enabled);
    assert_eq!(config.segments.git.as_ref().unwrap().show_sha, Some(false));
    assert_eq!(config.segments.git.as_ref().unwrap().show_working_tree, Some(true));
    
    assert!(config.segments.session.as_ref().unwrap().enabled);
    assert_eq!(config.segments.session.as_ref().unwrap().display_type, Some("both".to_string()));
    assert_eq!(config.segments.session.as_ref().unwrap().cost_source, Some("official".to_string()));
    
    assert!(!config.segments.today.as_ref().unwrap().enabled);
}

#[tokio::test] 
async fn test_theme_colors() {
    let dark_theme = get_theme("dark");
    let light_theme = get_theme("light");
    let nord_theme = get_theme("nord");
    let tokyo_theme = get_theme("tokyo-night");
    let rose_theme = get_theme("rose-pine");
    
    // Verify themes have required segments
    let required_segments = ["directory", "git", "block", "today", "session", "context", "metrics"];
    
    for segment in &required_segments {
        assert!(dark_theme.colors.contains_key(*segment), "Dark theme missing {}", segment);
        assert!(light_theme.colors.contains_key(*segment), "Light theme missing {}", segment);
        assert!(nord_theme.colors.contains_key(*segment), "Nord theme missing {}", segment);
        assert!(tokyo_theme.colors.contains_key(*segment), "Tokyo Night theme missing {}", segment);
        assert!(rose_theme.colors.contains_key(*segment), "Rose Pine theme missing {}", segment);
    }
    
    // Verify color format (should be valid hex colors)
    let (bg, fg) = dark_theme.colors.get("directory").unwrap();
    assert!(bg.starts_with("#"), "Background color should start with #");
    assert!(fg.starts_with("#"), "Foreground color should start with #");
    assert_eq!(bg.len(), 7, "Background color should be 7 characters (#rrggbb)");
    assert_eq!(fg.len(), 7, "Foreground color should be 7 characters (#rrggbb)");
}

#[tokio::test]
async fn test_environment_variable_overrides() {
    // Set environment variables
    std::env::set_var("CLAUDE_POWERLINE_THEME", "nord");
    std::env::set_var("CLAUDE_POWERLINE_STYLE", "powerline");
    
    let config = config::load_config(None).await.unwrap();
    
    assert_eq!(config.theme, "nord");
    assert_eq!(config.style, "powerline");
    
    // Clean up
    std::env::remove_var("CLAUDE_POWERLINE_THEME");
    std::env::remove_var("CLAUDE_POWERLINE_STYLE");
}

#[tokio::test]
async fn test_performance_with_large_transcript() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("projects").join("perf-test");
    fs::create_dir_all(&project_dir).await.unwrap();
    
    // Generate a large transcript file (1000 entries)
    let mut transcript_lines = Vec::new();
    let base_time = chrono::Utc::now() - chrono::Duration::days(1);
    
    for i in 0..1000 {
        let timestamp = base_time + chrono::Duration::minutes(i * 5);
        let entry = format!(
            r#"{{"timestamp":"{}","message":{{"id":"msg-{}","usage":{{"input_tokens":{},"output_tokens":{}}},"model":"claude-3-5-sonnet"}},"costUSD":{},"requestId":"req-{}"}},"#,
            timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
            i,
            500 + i * 10,
            250 + i * 5,
            0.025 + (i as f64 * 0.001),
            i
        );
        transcript_lines.push(entry);
    }
    
    let transcript_content = transcript_lines.join("\n");
    let transcript_path = project_dir.join("large-session.jsonl");
    fs::write(&transcript_path, &transcript_content).await.unwrap();
    
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    
    // Time the execution
    let start = std::time::Instant::now();
    
    let output = Command::new("./target/release/claude-powerline")
        .args(&["--theme", "dark"])
        .output()
        .expect("Failed to execute claude-powerline");
    
    let duration = start.elapsed();
    
    // Should complete in under 1 second even with large file
    assert!(duration.as_millis() < 1000, "Large transcript processing took too long: {}ms", duration.as_millis());
    
    let statusline = String::from_utf8(output.stdout).unwrap();
    assert!(!statusline.is_empty(), "Statusline should not be empty for large transcript");
    
    // Clean up
    std::env::remove_var("CLAUDE_CONFIG_DIR");
}

#[tokio::test]
async fn test_cli_argument_parsing() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test help flag
    let output = Command::new("./target/release/claude-powerline")
        .arg("--help")
        .output()
        .expect("Failed to execute claude-powerline --help");
    
    let help_text = String::from_utf8(output.stdout).unwrap();
    assert!(help_text.contains("Claude Powerline"));
    assert!(help_text.contains("--theme"));
    assert!(help_text.contains("--style"));
    assert!(help_text.contains("CLAUDE_POWERLINE_THEME"));
    
    // Test theme argument
    let output = Command::new("./target/release/claude-powerline")
        .args(&["--theme", "nord", "--style", "minimal"])
        .output()
        .expect("Failed to execute with theme args");
    
    // Should execute without errors (stderr might have warnings about missing transcript files)
    assert!(output.status.success(), "Command should succeed with theme args");
}