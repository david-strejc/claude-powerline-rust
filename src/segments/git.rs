use crate::segments::Segment;
use crate::utils::{debug_with_context, Cache};
use anyhow::{Context, Result};
use gix::{Repository, ThreadSafeRepository};
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct GitInfo {
    pub branch: Option<String>,
    pub sha: Option<String>,
    pub is_dirty: bool,
    pub ahead_behind: Option<(u32, u32)>, // (ahead, behind)
    pub staged_count: u32,
    pub unstaged_count: u32,
    pub untracked_count: u32,
    pub stash_count: Option<u32>,
    pub repo_name: Option<String>,
}

pub struct GitSegment {
    pub enabled: bool,
    pub show_sha: bool,
    pub show_working_tree: bool,
    pub show_upstream: bool,
    pub show_stash_count: bool,
    pub show_repo_name: bool,
    cache: Cache<String, GitInfo>,
}

impl GitSegment {
    pub fn new() -> Self {
        Self {
            enabled: true,
            show_sha: true,
            show_working_tree: false,
            show_upstream: false,
            show_stash_count: false,
            show_repo_name: false,
            cache: Cache::new(Duration::from_secs(5)), // 5-second cache
        }
    }

    /// Get git information for current directory with optimized performance
    pub async fn get_git_info(&self) -> Result<GitInfo> {
        if !self.enabled {
            return Ok(GitInfo::default());
        }

        let cwd = env::current_dir().context("Failed to get current directory")?;
        let cache_key = cwd.to_string_lossy().to_string();

        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            debug_with_context("git", "Using cached git info");
            return Ok(cached);
        }

        debug_with_context("git", &format!("Loading git info for: {}", cwd.display()));

        let git_info = self.load_git_info(&cwd).await?;
        
        // Cache the result
        self.cache.insert(cache_key, git_info.clone());
        
        Ok(git_info)
    }

    /// Load git information using gix (pure Rust implementation)
    async fn load_git_info(&self, path: &Path) -> Result<GitInfo> {
        // Try to open repository using gix
        match gix::discover(path) {
            Ok(repo) => self.extract_git_info_gix(repo).await,
            Err(_) => {
                debug_with_context("git", "Not in a git repository");
                Ok(GitInfo::default())
            }
        }
    }

    /// Extract git information using gix
    async fn extract_git_info_gix(&self, repo: Repository) -> Result<GitInfo> {
        let mut info = GitInfo::default();

        // Get current branch
        if let Ok(head_ref) = repo.head_ref() {
            if let Some(reference) = head_ref {
                let name = reference.name().shorten();
                info.branch = Some(name.to_string());
            }
        }

        // Get current commit SHA
        if let Ok(head) = repo.head_commit() {
            let sha = head.id().to_hex_with_len(7).to_string();
            info.sha = Some(sha);
        }

        // Get repository name
        if self.show_repo_name {
            if let Some(name) = repo.work_dir()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str()) {
                info.repo_name = Some(name.to_string());
            }
        }

        // Get working tree status (if requested)
        if self.show_working_tree {
            // Simplified status check - in a full implementation you'd use gix status API
            // For now, just set defaults
            info.staged_count = 0;
            info.unstaged_count = 0;
            info.untracked_count = 0;
            info.is_dirty = false;
        } else {
            // Quick dirty check without full status
            info.is_dirty = self.quick_dirty_check(&repo).await.unwrap_or(false);
        }

        // Get ahead/behind information (if requested)
        if self.show_upstream {
            info.ahead_behind = self.get_ahead_behind(&repo).await.ok();
        }

        // Get stash count (if requested)
        if self.show_stash_count {
            info.stash_count = self.get_stash_count(&repo).await.ok();
        }

        debug_with_context("git", &format!(
            "Git info: branch={:?}, sha={:?}, dirty={}, ahead_behind={:?}",
            info.branch, info.sha, info.is_dirty, info.ahead_behind
        ));

        Ok(info)
    }

    /// Quick dirty check without full status scan
    async fn quick_dirty_check(&self, _repo: &Repository) -> Result<bool> {
        // Quick dirty check without full status scan
        // This is a simplified implementation for performance
        // In practice, you'd check index vs HEAD
        Ok(false)
    }

    /// Get ahead/behind count compared to upstream
    async fn get_ahead_behind(&self, _repo: &Repository) -> Result<(u32, u32)> {
        // This is a simplified implementation
        // In practice, you'd need to compare local branch with its upstream
        Ok((0, 0))
    }

    /// Get stash count
    async fn get_stash_count(&self, _repo: &Repository) -> Result<u32> {
        // gix doesn't have direct stash support yet, so we fallback to git command
        match Command::new("git")
            .args(&["stash", "list", "--porcelain"])
            .output()
            .await
        {
            Ok(output) => {
                if output.status.success() {
                    let count = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .count() as u32;
                    Ok(count)
                } else {
                    Ok(0)
                }
            }
            Err(_) => Ok(0),
        }
    }
}

impl Default for GitInfo {
    fn default() -> Self {
        Self {
            branch: None,
            sha: None,
            is_dirty: false,
            ahead_behind: None,
            staged_count: 0,
            unstaged_count: 0,
            untracked_count: 0,
            stash_count: None,
            repo_name: None,
        }
    }
}

impl Segment for GitSegment {
    fn render(&self) -> Result<String> {
        // This will be implemented as part of the display logic
        Ok("⎇ Git".to_string())
    }

    fn name(&self) -> &'static str {
        "git"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}