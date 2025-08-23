pub mod block;
pub mod today;
pub mod session;
pub mod git;
pub mod context;
pub mod metrics;

pub use block::*;
pub use today::*;
pub use session::*;
pub use git::*;
pub use context::*;
pub use metrics::*;

use anyhow::Result;
use std::collections::HashMap;

/// Trait for all statusline segments
pub trait Segment {
    /// Render the segment as a string
    fn render(&self) -> Result<String>;
    
    /// Get segment name for debugging
    fn name(&self) -> &'static str;
    
    /// Check if segment should be displayed
    fn is_enabled(&self) -> bool {
        true
    }
}