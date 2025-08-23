use colored::{ColoredString, Colorize};
use std::collections::HashMap;

pub struct Theme {
    pub colors: HashMap<String, (String, String)>, // (bg, fg)
}

impl Theme {
    pub fn get_colors(&self, segment: &str) -> Option<&(String, String)> {
        self.colors.get(segment)
    }
}

pub fn get_theme(name: &str) -> Theme {
    match name {
        "dark" => dark_theme(),
        "light" => light_theme(),
        "nord" => nord_theme(),
        "tokyo-night" => tokyo_night_theme(),
        "rose-pine" => rose_pine_theme(),
        _ => dark_theme(), // fallback
    }
}

fn dark_theme() -> Theme {
    let mut colors = HashMap::new();
    colors.insert("directory".to_string(), ("#2d3748".to_string(), "#e2e8f0".to_string()));
    colors.insert("git".to_string(), ("#38a169".to_string(), "#f7fafc".to_string()));
    colors.insert("block".to_string(), ("#3182ce".to_string(), "#f7fafc".to_string()));
    colors.insert("today".to_string(), ("#d69e2e".to_string(), "#1a202c".to_string()));
    colors.insert("session".to_string(), ("#805ad5".to_string(), "#f7fafc".to_string()));
    colors.insert("context".to_string(), ("#e53e3e".to_string(), "#f7fafc".to_string()));
    colors.insert("metrics".to_string(), ("#38b2ac".to_string(), "#f7fafc".to_string()));
    colors.insert("model".to_string(), ("#ed8936".to_string(), "#f7fafc".to_string()));
    
    Theme { colors }
}

fn light_theme() -> Theme {
    let mut colors = HashMap::new();
    colors.insert("directory".to_string(), ("#f7fafc".to_string(), "#2d3748".to_string()));
    colors.insert("git".to_string(), ("#c6f6d5".to_string(), "#1a202c".to_string()));
    colors.insert("block".to_string(), ("#bee3f8".to_string(), "#1a202c".to_string()));
    colors.insert("today".to_string(), ("#faf089".to_string(), "#1a202c".to_string()));
    colors.insert("session".to_string(), ("#d6bcfa".to_string(), "#1a202c".to_string()));
    colors.insert("context".to_string(), ("#feb2b2".to_string(), "#1a202c".to_string()));
    colors.insert("metrics".to_string(), ("#b2f5ea".to_string(), "#1a202c".to_string()));
    colors.insert("model".to_string(), ("#fed7aa".to_string(), "#1a202c".to_string()));
    
    Theme { colors }
}

fn nord_theme() -> Theme {
    let mut colors = HashMap::new();
    colors.insert("directory".to_string(), ("#2e3440".to_string(), "#d8dee9".to_string()));
    colors.insert("git".to_string(), ("#5e81ac".to_string(), "#eceff4".to_string()));
    colors.insert("block".to_string(), ("#81a1c1".to_string(), "#eceff4".to_string()));
    colors.insert("today".to_string(), ("#ebcb8b".to_string(), "#2e3440".to_string()));
    colors.insert("session".to_string(), ("#b48ead".to_string(), "#eceff4".to_string()));
    colors.insert("context".to_string(), ("#bf616a".to_string(), "#eceff4".to_string()));
    colors.insert("metrics".to_string(), ("#88c0d0".to_string(), "#eceff4".to_string()));
    colors.insert("model".to_string(), ("#d08770".to_string(), "#eceff4".to_string()));
    
    Theme { colors }
}

fn tokyo_night_theme() -> Theme {
    let mut colors = HashMap::new();
    colors.insert("directory".to_string(), ("#1a1b26".to_string(), "#c0caf5".to_string()));
    colors.insert("git".to_string(), ("#9ece6a".to_string(), "#1a1b26".to_string()));
    colors.insert("block".to_string(), ("#7aa2f7".to_string(), "#1a1b26".to_string()));
    colors.insert("today".to_string(), ("#e0af68".to_string(), "#1a1b26".to_string()));
    colors.insert("session".to_string(), ("#bb9af7".to_string(), "#1a1b26".to_string()));
    colors.insert("context".to_string(), ("#f7768e".to_string(), "#1a1b26".to_string()));
    colors.insert("metrics".to_string(), ("#2ac3de".to_string(), "#1a1b26".to_string()));
    colors.insert("model".to_string(), ("#ff9e64".to_string(), "#1a1b26".to_string()));
    
    Theme { colors }
}

fn rose_pine_theme() -> Theme {
    let mut colors = HashMap::new();
    colors.insert("directory".to_string(), ("#191724".to_string(), "#e0def4".to_string()));
    colors.insert("git".to_string(), ("#31748f".to_string(), "#e0def4".to_string()));
    colors.insert("block".to_string(), ("#c4a7e7".to_string(), "#191724".to_string()));
    colors.insert("today".to_string(), ("#f6c177".to_string(), "#191724".to_string()));
    colors.insert("session".to_string(), ("#eb6f92".to_string(), "#e0def4".to_string()));
    colors.insert("context".to_string(), ("#ebbcba".to_string(), "#191724".to_string()));
    colors.insert("metrics".to_string(), ("#9ccfd8".to_string(), "#191724".to_string()));
    colors.insert("model".to_string(), ("#ebbcba".to_string(), "#191724".to_string()));
    
    Theme { colors }
}