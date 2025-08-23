use crate::config::*;

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            style: "minimal".to_string(),
            segments: SegmentConfig::default(),
            colors: None,
            budget: None,
            display: None,
        }
    }
}

impl Default for SegmentConfig {
    fn default() -> Self {
        Self {
            directory: Some(DirectoryConfig::default()),
            git: Some(GitConfig::default()),
            block: Some(BlockConfig::default()),
            today: Some(TodayConfig::default()),
            session: Some(SessionConfig::default()),
            context: Some(ContextConfig::default()),
            metrics: Some(MetricsConfig::default()),
            model: Some(ModelConfig::default()),
        }
    }
}

impl Default for DirectoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_basename: Some(false),
        }
    }
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_sha: Some(true),
            show_working_tree: Some(false),
            show_upstream: Some(false),
            show_stash_count: Some(false),
            show_repo_name: Some(false),
        }
    }
}

impl Default for BlockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            display_type: Some("tokens".to_string()),
            burn_type: Some("cost".to_string()),
        }
    }
}

impl Default for TodayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            display_type: Some("cost".to_string()),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            display_type: Some("tokens".to_string()),
            cost_source: Some("calculated".to_string()),
        }
    }
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_percentage_only: Some(false),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_response_time: Some(true),
            show_last_response_time: Some(false),
            show_duration: Some(true),
            show_message_count: Some(true),
            show_lines_added: Some(true),
            show_lines_removed: Some(true),
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
        }
    }
}