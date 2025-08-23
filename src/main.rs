use anyhow::Result;
use claude_powerline_rust::*;
use pico_args::Arguments;
use std::env;
use std::path::PathBuf;

#[derive(Debug)]
struct Args {
    theme: String,
    style: String,
    config: Option<PathBuf>,
    help: bool,
    install_fonts: bool,
    basename: bool,
}

impl Args {
    fn from_env() -> Result<Self> {
        let mut args = Arguments::from_env();
        
        Ok(Self {
            theme: args.opt_value_from_str("--theme")
                .unwrap_or(None)
                .or_else(|| env::var("CLAUDE_POWERLINE_THEME").ok())
                .unwrap_or_else(|| "dark".to_string()),
            style: args.opt_value_from_str("--style")
                .unwrap_or(None)
                .or_else(|| env::var("CLAUDE_POWERLINE_STYLE").ok())
                .unwrap_or_else(|| "minimal".to_string()),
            config: args.opt_value_from_str::<_, PathBuf>("--config")
                .unwrap_or(None)
                .or_else(|| env::var("CLAUDE_POWERLINE_CONFIG").ok().map(PathBuf::from)),
            help: args.contains("--help"),
            install_fonts: args.contains("--install-fonts"),
            basename: args.contains("--basename"),
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::from_env()?;

    if args.help {
        print_help();
        return Ok(());
    }

    if args.install_fonts {
        install_fonts().await?;
        return Ok(());
    }

    // Load configuration
    let mut config = config::load_config(args.config).await?;
    config.theme = args.theme.clone();
    config.style = args.style.clone();
    
    // Override directory config with CLI flag
    if args.basename {
        if config.segments.directory.is_none() {
            config.segments.directory = Some(config::DirectoryConfig {
                enabled: true,
                show_basename: Some(true),
            });
        } else if let Some(ref mut dir_config) = config.segments.directory {
            dir_config.show_basename = Some(true);
        }
    }

    // Generate and display statusline
    let statusline = generate_statusline(&config).await?;
    println!("{}", statusline);

    Ok(())
}

async fn generate_statusline(config: &Config) -> Result<String> {
    let mut segments = Vec::new();
    let theme = themes::get_theme(&config.theme);

    // Directory segment
    if config.segments.directory.as_ref().map_or(true, |c| c.enabled) {
        let dir_segment = render_directory_segment(&config, &theme)?;
        segments.push(dir_segment);
    }

    // Git segment
    if config.segments.git.as_ref().map_or(true, |c| c.enabled) {
        let git_segment = render_git_segment(&config, &theme).await?;
        if !git_segment.is_empty() {
            segments.push(git_segment);
        }
    }

    // Session segment
    if config.segments.session.as_ref().map_or(true, |c| c.enabled) {
        let session_segment = render_session_segment(&config, &theme).await?;
        if !session_segment.is_empty() {
            segments.push(session_segment);
        }
    }

    // Today segment
    if config.segments.today.as_ref().map_or(true, |c| c.enabled) {
        let today_segment = render_today_segment(&config, &theme).await?;
        if !today_segment.is_empty() {
            segments.push(today_segment);
        }
    }

    // Block segment
    if config.segments.block.as_ref().map_or(true, |c| c.enabled) {
        let block_segment = render_block_segment(&config, &theme).await?;
        if !block_segment.is_empty() {
            segments.push(block_segment);
        }
    }

    // Context segment
    if config.segments.context.as_ref().map_or(true, |c| c.enabled) {
        let context_segment = render_context_segment(&config, &theme).await?;
        if !context_segment.is_empty() {
            segments.push(context_segment);
        }
    }

    // Model segment
    if config.segments.model.as_ref().map_or(true, |c| c.enabled) {
        let model_segment = render_model_segment(&config, &theme).await?;
        if !model_segment.is_empty() {
            segments.push(model_segment);
        }
    }

    // Join segments with appropriate separators
    let separator = if config.style == "powerline" { " ⮀ " } else { "  " };
    Ok(segments.join(separator))
}

fn render_directory_segment(config: &Config, theme: &themes::Theme) -> Result<String> {
    let current_dir = env::current_dir()?;
    let show_basename = config.segments.directory
        .as_ref()
        .and_then(|c| c.show_basename)
        .unwrap_or(false);

    let dir_name = if show_basename {
        current_dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
    } else {
        &current_dir.to_string_lossy()
    };

    let formatted = format!(" {} ", dir_name);
    Ok(apply_theme_colors(&formatted, "directory", theme))
}

async fn render_git_segment(config: &Config, theme: &themes::Theme) -> Result<String> {
    let default_git_config = config::GitConfig::default();
    let git_config = config.segments.git.as_ref().unwrap_or(&default_git_config);
    let mut git_segment = segments::GitSegment::new();
    
    git_segment.show_sha = git_config.show_sha.unwrap_or(true);
    git_segment.show_working_tree = git_config.show_working_tree.unwrap_or(false);
    git_segment.show_upstream = git_config.show_upstream.unwrap_or(false);
    git_segment.show_stash_count = git_config.show_stash_count.unwrap_or(false);
    git_segment.show_repo_name = git_config.show_repo_name.unwrap_or(false);

    let git_info = git_segment.get_git_info().await?;
    
    if git_info.branch.is_none() {
        return Ok(String::new());
    }

    let mut parts = Vec::new();
    parts.push("⎇".to_string());
    
    if let Some(branch) = &git_info.branch {
        parts.push(branch.clone());
    }
    
    if git_segment.show_sha {
        if let Some(sha) = &git_info.sha {
            parts.push(format!("♯{}", sha));
        }
    }

    if git_info.is_dirty {
        parts.push("●".to_string());
    } else {
        parts.push("✓".to_string());
    }

    let formatted = format!(" {} ", parts.join(" "));
    Ok(apply_theme_colors(&formatted, "git", theme))
}

async fn render_session_segment(config: &Config, theme: &themes::Theme) -> Result<String> {
    let default_session_config = config::SessionConfig::default();
    let session_config = config.segments.session.as_ref().unwrap_or(&default_session_config);
    let mut session_segment = segments::SessionSegment::new();
    
    session_segment.display_type = session_config.display_type.clone().unwrap_or_else(|| "tokens".to_string());
    session_segment.cost_source = session_config.cost_source.clone().unwrap_or_else(|| "calculated".to_string());

    let session_info = session_segment.get_session_info().await?;
    
    if session_info.tokens.is_none() && session_info.cost.is_none() {
        return Ok(String::new());
    }

    let mut parts = vec!["§".to_string()];
    
    match session_segment.display_type.as_str() {
        "cost" => {
            if let Some(cost) = session_info.cost {
                parts.push(format!("${:.2}", cost));
            }
        }
        "tokens" => {
            if let Some(tokens) = session_info.tokens {
                parts.push(format!("{}T", format_number(tokens)));
            }
        }
        "both" => {
            if let Some(cost) = session_info.cost {
                parts.push(format!("${:.2}", cost));
            }
            if let Some(tokens) = session_info.tokens {
                parts.push(format!("{}T", format_number(tokens)));
            }
        }
        _ => {}
    }

    let formatted = format!(" {} ", parts.join(" "));
    Ok(apply_theme_colors(&formatted, "session", theme))
}

async fn render_today_segment(config: &Config, theme: &themes::Theme) -> Result<String> {
    let default_today_config = config::TodayConfig::default();
    let today_config = config.segments.today.as_ref().unwrap_or(&default_today_config);
    let mut today_segment = segments::TodaySegment::new();
    
    today_segment.display_type = today_config.display_type.clone().unwrap_or_else(|| "cost".to_string());

    let today_info = today_segment.get_today_info().await?;
    
    if today_info.tokens.is_none() && today_info.cost.is_none() {
        return Ok(String::new());
    }

    let mut parts = vec!["💰".to_string()];
    
    match today_segment.display_type.as_str() {
        "cost" => {
            if let Some(cost) = today_info.cost {
                parts.push(format!("${:.2}", cost));
            }
        }
        "tokens" => {
            if let Some(tokens) = today_info.tokens {
                parts.push(format!("{}T", format_number(tokens)));
            }
        }
        "both" => {
            if let Some(cost) = today_info.cost {
                parts.push(format!("${:.2}", cost));
            }
            if let Some(tokens) = today_info.tokens {
                parts.push(format!("{}T", format_number(tokens)));
            }
        }
        _ => {}
    }

    let formatted = format!(" {} ", parts.join(" "));
    Ok(apply_theme_colors(&formatted, "today", theme))
}

async fn render_block_segment(config: &Config, theme: &themes::Theme) -> Result<String> {
    let default_block_config = config::BlockConfig::default();
    let block_config = config.segments.block.as_ref().unwrap_or(&default_block_config);
    let mut block_segment = segments::BlockSegment::new();
    
    block_segment.display_type = block_config.display_type.clone().unwrap_or_else(|| "tokens".to_string());
    block_segment.burn_type = block_config.burn_type.clone().unwrap_or_else(|| "cost".to_string());

    let block_info = block_segment.get_active_block_info().await?;
    
    if block_info.tokens.is_none() && block_info.cost.is_none() {
        return Ok(String::new());
    }

    let mut parts = vec!["🎪".to_string()];
    
    match block_segment.display_type.as_str() {
        "cost" => {
            if let Some(cost) = block_info.cost {
                parts.push(format!("${:.2}", cost));
            }
        }
        "tokens" => {
            if let Some(tokens) = block_info.tokens {
                parts.push(format!("{}T", format_number(tokens)));
            }
        }
        "weighted" => {
            if let Some(weighted) = block_info.weighted_tokens {
                parts.push(format!("{}T", format_tokens(weighted)));
            }
        }
        _ => {}
    }

    // Show reset time instead of minutes remaining
    if let Some(reset_time) = block_info.reset_time {
        let now = chrono::Local::now();
        let local_reset_time = reset_time.with_timezone(&chrono::Local);
        parts.push(format!("Reset@:{}->{}", 
                          now.format("%H:%M"), 
                          local_reset_time.format("%H:%M")));
    }

    let formatted = format!(" {} ", parts.join(" "));
    Ok(apply_theme_colors(&formatted, "block", theme))
}

async fn render_model_segment(config: &Config, theme: &themes::Theme) -> Result<String> {
    let default_model_config = config::ModelConfig::default();
    let model_config = config.segments.model.as_ref().unwrap_or(&default_model_config);
    
    if !model_config.enabled {
        return Ok(String::new());
    }

    let mut model_segment = segments::ModelSegment::new();
    let model_info = model_segment.get_current_model_info().await?;
    
    if model_info.display_name.is_none() {
        return Ok(String::new());
    }

    let mut parts = vec!["🤖".to_string()];
    if let Some(name) = model_info.display_name {
        parts.push(name);
    }

    let text = parts.join(" ");
    Ok(apply_theme_colors(&text, "model", theme))
}

async fn render_context_segment(config: &Config, theme: &themes::Theme) -> Result<String> {
    let default_context_config = config::ContextConfig::default();
    let context_config = config.segments.context.as_ref().unwrap_or(&default_context_config);
    let mut context_segment = segments::ContextSegment::new();
    
    context_segment.show_percentage_only = context_config.show_percentage_only.unwrap_or(false);

    let context_info = context_segment.get_context_info().await?;
    
    // Always show context info (even default values are useful)
    // Default shows "◔ 0 (100%)" indicating 100% context remaining

    let mut parts = vec!["🧠".to_string()];
    
    if context_segment.show_percentage_only {
        parts.push(format!("{}%", context_info.context_left_percentage));
    } else {
        parts.push(format_number(context_info.input_tokens).to_string());
        parts.push(format!("({}%)", context_info.context_left_percentage));
    }

    let formatted = format!(" {} ", parts.join(" "));
    Ok(apply_theme_colors(&formatted, "context", theme))
}

fn apply_theme_colors(text: &str, segment: &str, theme: &themes::Theme) -> String {
    // Check if we should use colors
    if !should_use_colors() {
        return text.to_string();
    }
    
    if let Some((bg_color, fg_color)) = theme.get_colors(segment) {
        let bg_rgb = parse_color(bg_color);
        let fg_rgb = parse_color(fg_color);
        
        // Try 24-bit RGB first, fallback to 8-bit if not supported
        if supports_rgb_colors() {
            format!("\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m{}\x1b[0m", 
                    bg_rgb.0, bg_rgb.1, bg_rgb.2,
                    fg_rgb.0, fg_rgb.1, fg_rgb.2,
                    text)
        } else {
            // Fallback to basic 8-bit colors
            let bg_code = rgb_to_8bit(bg_rgb);
            let fg_code = rgb_to_8bit(fg_rgb);
            format!("\x1b[48;5;{}m\x1b[38;5;{}m{}\x1b[0m", bg_code, fg_code, text)
        }
    } else {
        text.to_string()
    }
}

fn should_use_colors() -> bool {
    // Always use colors unless explicitly disabled
    // Claude Code can handle ANSI escape codes even when not in direct TTY
    env::var("NO_COLOR").is_err() &&
        env::var("TERM").map_or(true, |term| term != "dumb") &&
        env::var("TERM").map_or(false, |term| !term.is_empty())
}

fn supports_rgb_colors() -> bool {
    env::var("COLORTERM").map_or(false, |ct| ct.contains("truecolor") || ct.contains("24bit")) ||
    env::var("TERM").map_or(false, |term| 
        term.contains("256") || 
        term.contains("color") || 
        term == "xterm-kitty" ||
        term == "alacritty"
    )
}

fn rgb_to_8bit((r, g, b): (u8, u8, u8)) -> u8 {
    // Convert RGB to closest 8-bit color (216 color cube + grayscale)
    if r == g && g == b {
        // Grayscale
        if r < 8 { 16 }
        else if r > 248 { 231 }
        else { ((r - 8) / 10) + 232 }
    } else {
        // Color cube: 16 + 36*r + 6*g + b
        let r6 = (r * 5 / 255);
        let g6 = (g * 5 / 255); 
        let b6 = (b * 5 / 255);
        16 + 36 * r6 + 6 * g6 + b6
    }
}

fn parse_color(color: &str) -> (u8, u8, u8) {
    if color.starts_with('#') && color.len() == 7 {
        let r = u8::from_str_radix(&color[1..3], 16).unwrap_or(255);
        let g = u8::from_str_radix(&color[3..5], 16).unwrap_or(255);
        let b = u8::from_str_radix(&color[5..7], 16).unwrap_or(255);
        (r, g, b)
    } else {
        (255, 255, 255) // Default to white
    }
}

fn format_number(num: u32) -> String {
    if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}K", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}

fn format_tokens(num: u32) -> String {
    if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}K", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}

async fn install_fonts() -> Result<()> {
    println!("Font installation not implemented in this version.");
    println!("Please install powerline fonts manually from: https://github.com/powerline/fonts");
    Ok(())
}

fn print_help() {
    println!("Claude Powerline - High-performance statusline for Claude Code");
    println!();
    println!("USAGE:");
    println!("    claude-powerline [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --theme <THEME>        Theme: dark, light, nord, tokyo-night, rose-pine [default: dark]");
    println!("    --style <STYLE>        Style: minimal, powerline [default: minimal]");
    println!("    --config <FILE>        Custom config file path");
    println!("    --basename             Show only directory name instead of full path");
    println!("    --install-fonts        Install powerline fonts");
    println!("    --help                 Show this help message");
    println!();
    println!("ENVIRONMENT VARIABLES:");
    println!("    CLAUDE_POWERLINE_THEME     Override theme");
    println!("    CLAUDE_POWERLINE_STYLE     Override style");
    println!("    CLAUDE_POWERLINE_CONFIG    Override config path");
    println!("    CLAUDE_POWERLINE_DEBUG     Enable debug logging");
}