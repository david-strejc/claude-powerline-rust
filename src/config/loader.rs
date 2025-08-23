use crate::config::Config;
use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;
use tokio::fs;

/// Load configuration with priority: CLI args > Env vars > Config files > Defaults
pub async fn load_config(config_path: Option<PathBuf>) -> Result<Config> {
    let mut config = if let Some(path) = config_path {
        load_config_file(&path).await?
    } else {
        load_config_from_default_locations().await?
    };

    // Apply environment variable overrides
    apply_env_overrides(&mut config);

    Ok(config)
}

/// Load configuration from default locations
async fn load_config_from_default_locations() -> Result<Config> {
    let search_paths = get_config_search_paths();
    
    for path in search_paths {
        if path.exists() {
            match load_config_file(&path).await {
                Ok(config) => return Ok(config),
                Err(e) => {
                    eprintln!("Warning: Failed to load config from {}: {}", path.display(), e);
                }
            }
        }
    }

    // Return default config if no config file found
    Ok(Config::default())
}

/// Get list of paths to search for configuration files
fn get_config_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Current directory
    paths.push(PathBuf::from(".claude-powerline.json"));

    // User home directory
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".claude").join("claude-powerline.json"));
        paths.push(home.join(".config").join("claude-powerline").join("config.json"));
    }

    paths
}

/// Load configuration from a specific file
async fn load_config_file(path: &PathBuf) -> Result<Config> {
    let content = fs::read_to_string(path).await
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    
    let config: Config = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
    
    Ok(config)
}

/// Apply environment variable overrides to configuration
fn apply_env_overrides(config: &mut Config) {
    if let Ok(theme) = env::var("CLAUDE_POWERLINE_THEME") {
        config.theme = theme;
    }

    if let Ok(style) = env::var("CLAUDE_POWERLINE_STYLE") {
        config.style = style;
    }

    // Add more environment variable overrides as needed
}