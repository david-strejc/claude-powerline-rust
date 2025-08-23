use std::env;

pub fn debug(message: &str) {
    if env::var("CLAUDE_POWERLINE_DEBUG").is_ok() {
        eprintln!("[DEBUG] {}", message);
    }
}

pub fn debug_with_context(context: &str, message: &str) {
    if env::var("CLAUDE_POWERLINE_DEBUG").is_ok() {
        eprintln!("[DEBUG] {}: {}", context, message);
    }
}