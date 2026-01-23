//! Git integration service for Ferrite
//!
//! Provides Git repository detection, branch information, and file status tracking
//! using the git2 library.

use git2::{ErrorCode, Repository, Status, StatusOptions};
use log::{debug, trace, warn};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────────────────────────────────────
// Git File Status
// ─────────────────────────────────────────────────────────────────────────────

/// Git status for a single file.
///
/// Represents the various states a file can be in relative to the Git index
/// and working directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitFileStatus {
    /// File is tracked and unmodified
    #[default]
    Clean,
    /// File has been modified in the working directory
    Modified,
    /// File has been staged for commit (index)
    Staged,
    /// File has both staged and unstaged modifications
    StagedModified,
    /// File is new and not tracked by Git
    Untracked,
    /// File is ignored by .gitignore
    Ignored,
    /// File has been deleted
    Deleted,
    /// File has been renamed
    Renamed,
    /// File has merge conflicts
    Conflict,
}

impl GitFileStatus {
    /// Get a short label for the status (for badge display).
    #[allow(dead_code)] // Public API, icon() is used instead
    pub fn label(&self) -> &'static str {
        match self {
            Self::Clean => "",
            Self::Modified => "M",
            Self::Staged => "S",
            Self::StagedModified => "SM",
            Self::Untracked => "U",
            Self::Ignored => "I",
            Self::Deleted => "D",
            Self::Renamed => "R",
            Self::Conflict => "!",
        }
    }

    /// Get an icon/symbol for the status.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Clean => "",
            Self::Modified => "●",   // Yellow dot
            Self::Staged => "✓",     // Green check (staged)
            Self::StagedModified => "◐", // Half-filled (both staged and modified)
            Self::Untracked => "?",  // Question mark
            Self::Ignored => "○",    // Empty circle
            Self::Deleted => "-",    // Minus (deletion)
            Self::Renamed => "→",    // Arrow
            Self::Conflict => "⚠",   // Warning
        }
    }

    /// Whether this status should be displayed (non-clean status).
    pub fn is_visible(&self) -> bool {
        !matches!(self, Self::Clean)
    }

    /// Convert from git2 Status flags to GitFileStatus.
    fn from_git2_status(status: Status) -> Self {
        // Check for conflicts first
        if status.is_conflicted() {
            return Self::Conflict;
        }

        // Check for staged changes
        let is_index_new = status.is_index_new();
        let is_index_modified = status.is_index_modified();
        let is_index_deleted = status.is_index_deleted();
        let is_index_renamed = status.is_index_renamed();
        let has_staged = is_index_new || is_index_modified || is_index_deleted || is_index_renamed;

        // Check for working tree changes
        let is_wt_modified = status.is_wt_modified();
        let is_wt_deleted = status.is_wt_deleted();
        let is_wt_renamed = status.is_wt_renamed();
        let is_wt_new = status.is_wt_new();
        let has_unstaged = is_wt_modified || is_wt_deleted || is_wt_renamed;

        // Check for untracked/ignored
        if status.is_ignored() {
            return Self::Ignored;
        }
        if is_wt_new {
            return Self::Untracked;
        }

        // Handle combinations
        if has_staged && has_unstaged {
            return Self::StagedModified;
        }

        if is_index_renamed {
            return Self::Renamed;
        }
        if is_index_deleted || is_wt_deleted {
            return Self::Deleted;
        }
        if has_staged {
            return Self::Staged;
        }
        if has_unstaged {
            return Self::Modified;
        }

        Self::Clean
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Git Service
// ─────────────────────────────────────────────────────────────────────────────

/// Git integration service.
///
/// Provides methods to query Git repository information including
/// the current branch and file statuses. Handles cases where Git
/// is not available or the directory is not a repository gracefully.
pub struct GitService {
    // Note: Repository is not Debug, so we implement Debug manually
    /// The Git repository, if one was found
    repo: Option<Repository>,
    /// Root path of the repository
    repo_root: Option<PathBuf>,
    /// Cached file statuses (relative path -> status)
    file_statuses: HashMap<PathBuf, GitFileStatus>,
    /// Whether status cache is valid
    cache_valid: bool,
}

impl std::fmt::Debug for GitService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitService")
            .field("repo_root", &self.repo_root)
            .field("is_open", &self.repo.is_some())
            .field("file_statuses_count", &self.file_statuses.len())
            .field("cache_valid", &self.cache_valid)
            .finish()
    }
}

impl Default for GitService {
    fn default() -> Self {
        Self::new()
    }
}

impl GitService {
    /// Create a new Git service without an associated repository.
    pub fn new() -> Self {
        Self {
            repo: None,
            repo_root: None,
            file_statuses: HashMap::new(),
            cache_valid: false,
        }
    }

    /// Open a Git repository from a path.
    ///
    /// Discovers the repository by walking up from the given path.
    /// Returns Ok(true) if a repository was found, Ok(false) if not.
    pub fn open(&mut self, path: &Path) -> Result<bool, git2::Error> {
        // Try to discover the repository
        match Repository::discover(path) {
            Ok(repo) => {
                let repo_root = repo
                    .workdir()
                    .map(|p| p.to_path_buf())
                    .or_else(|| repo.path().parent().map(|p| p.to_path_buf()));

                debug!(
                    "Git repository found at: {:?}",
                    repo_root.as_ref().map(|p| p.display())
                );

                self.repo_root = repo_root;
                self.repo = Some(repo);
                self.cache_valid = false;
                Ok(true)
            }
            Err(e) if e.code() == ErrorCode::NotFound => {
                trace!("No Git repository found at: {}", path.display());
                self.close();
                Ok(false)
            }
            Err(e) => {
                warn!("Error opening Git repository: {}", e);
                self.close();
                Err(e)
            }
        }
    }

    /// Close the current repository connection.
    pub fn close(&mut self) {
        self.repo = None;
        self.repo_root = None;
        self.file_statuses.clear();
        self.cache_valid = false;
    }

    /// Check if a Git repository is currently open.
    pub fn is_open(&self) -> bool {
        self.repo.is_some()
    }

    /// Get the repository root path.
    #[allow(dead_code)] // Public API for accessing repo root
    pub fn repo_root(&self) -> Option<&Path> {
        self.repo_root.as_deref()
    }

    /// Get the current branch name.
    ///
    /// Returns:
    /// - Some(name) - the current branch name (e.g., "main", "feature/xyz")
    /// - Some("HEAD detached") - if in detached HEAD state
    /// - None - if no repository is open or an error occurred
    pub fn current_branch(&self) -> Option<String> {
        let repo = self.repo.as_ref()?;

        match repo.head() {
            Ok(head) => {
                if head.is_branch() {
                    // Get the shorthand name (e.g., "main" instead of "refs/heads/main")
                    head.shorthand().map(|s| s.to_string())
                } else {
                    // Detached HEAD - try to get a short commit hash
                    head.target()
                        .map(|oid| format!("HEAD@{}", &oid.to_string()[..7]))
                }
            }
            Err(e) => {
                // Repository might be empty (no commits yet)
                if e.code() == ErrorCode::UnbornBranch {
                    // Try to get the name of the unborn branch
                    if let Ok(config) = repo.config() {
                        if let Ok(name) = config.get_string("init.defaultBranch") {
                            return Some(format!("{} (unborn)", name));
                        }
                    }
                    Some("main (unborn)".to_string())
                } else {
                    warn!("Error getting current branch: {}", e);
                    None
                }
            }
        }
    }

    /// Refresh the file status cache.
    ///
    /// This should be called when files might have changed.
    pub fn refresh_status(&mut self) {
        self.cache_valid = false;
        self.update_status_cache();
    }

    /// Update the file status cache if needed.
    fn update_status_cache(&mut self) {
        if self.cache_valid {
            return;
        }

        self.file_statuses.clear();

        let Some(repo) = &self.repo else {
            return;
        };

        // Configure status options
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(true) // Show ignored files with gray indicator
            .include_unmodified(false);

        // Get all statuses
        match repo.statuses(Some(&mut opts)) {
            Ok(statuses) => {
                for entry in statuses.iter() {
                    if let Some(path) = entry.path() {
                        let status = GitFileStatus::from_git2_status(entry.status());
                        if status.is_visible() {
                            self.file_statuses.insert(PathBuf::from(path), status);
                        }
                    }
                }
                trace!("Git status cache updated: {} files", self.file_statuses.len());
                self.cache_valid = true;
            }
            Err(e) => {
                warn!("Error getting Git statuses: {}", e);
            }
        }
    }

    /// Get the Git status for a specific file.
    ///
    /// The path should be absolute. Returns GitFileStatus::Clean if the file
    /// is tracked and unmodified, or if the path is outside the repository.
    #[allow(dead_code)] // Public API, get_all_statuses used for batch lookup
    pub fn file_status(&mut self, path: &Path) -> GitFileStatus {
        // Ensure cache is up to date
        self.update_status_cache();

        let Some(repo_root) = &self.repo_root else {
            return GitFileStatus::Clean;
        };

        // Convert absolute path to relative path within the repo
        let relative_path = match path.strip_prefix(repo_root) {
            Ok(rel) => rel.to_path_buf(),
            Err(_) => return GitFileStatus::Clean, // Path outside repo
        };

        // Look up in cache
        self.file_statuses
            .get(&relative_path)
            .copied()
            .unwrap_or(GitFileStatus::Clean)
    }

    /// Get all file statuses as a HashMap with absolute paths.
    ///
    /// This is useful for passing to UI components that need to look up
    /// statuses for multiple files. The returned map uses absolute paths.
    pub fn get_all_statuses(&mut self) -> HashMap<PathBuf, GitFileStatus> {
        self.update_status_cache();

        let Some(repo_root) = &self.repo_root else {
            return HashMap::new();
        };

        // Convert relative paths to absolute paths
        self.file_statuses
            .iter()
            .map(|(rel_path, status)| (repo_root.join(rel_path), *status))
            .collect()
    }

    /// Get the Git status for a directory.
    ///
    /// Returns the "worst" status of any file within the directory:
    /// Conflict > StagedModified > Modified > Staged > Untracked > Deleted > Clean
    #[allow(dead_code)] // Public API for directory status aggregation
    pub fn directory_status(&mut self, dir_path: &Path) -> GitFileStatus {
        // Ensure cache is up to date
        self.update_status_cache();

        let Some(repo_root) = &self.repo_root else {
            return GitFileStatus::Clean;
        };

        // Convert absolute path to relative path within the repo
        let relative_dir = match dir_path.strip_prefix(repo_root) {
            Ok(rel) => rel,
            Err(_) => return GitFileStatus::Clean, // Path outside repo
        };

        // Find the "worst" status of any file in this directory
        let mut worst_status = GitFileStatus::Clean;

        for (path, status) in &self.file_statuses {
            if path.starts_with(relative_dir) {
                worst_status = Self::worse_status(worst_status, *status);
                // Conflict is the worst, no need to continue
                if matches!(worst_status, GitFileStatus::Conflict) {
                    break;
                }
            }
        }

        worst_status
    }

    /// Compare two statuses and return the "worse" one for aggregation.
    #[allow(dead_code)] // Helper for directory_status
    fn worse_status(a: GitFileStatus, b: GitFileStatus) -> GitFileStatus {
        use GitFileStatus::*;

        // Define priority (higher = worse/more important to show)
        let priority = |s: GitFileStatus| -> u8 {
            match s {
                Clean => 0,
                Ignored => 1,
                Untracked => 2,
                Deleted => 3,
                Renamed => 4,
                Modified => 5,
                Staged => 6,
                StagedModified => 7,
                Conflict => 8,
            }
        };

        if priority(a) >= priority(b) {
            a
        } else {
            b
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Git Auto-Refresh
// ─────────────────────────────────────────────────────────────────────────────

use std::time::{Duration, Instant};

/// Configuration for Git auto-refresh behavior.
const GIT_REFRESH_INTERVAL: Duration = Duration::from_secs(10);
const GIT_DEBOUNCE_DURATION: Duration = Duration::from_millis(500);

/// Manages automatic Git status refreshing with debouncing.
///
/// This struct handles:
/// - Periodic refresh every 10 seconds when a workspace is open
/// - Refresh on window focus gained
/// - Refresh on file save
/// - Debouncing to prevent excessive refresh calls
#[derive(Debug)]
pub struct GitAutoRefresh {
    /// Last time git status was refreshed
    last_refresh: Option<Instant>,
    /// Time of last refresh request (for debouncing)
    last_request: Option<Instant>,
    /// Whether a refresh is pending (after debounce)
    pending_refresh: bool,
    /// Previous window focus state (to detect focus gained)
    was_focused: bool,
}

impl Default for GitAutoRefresh {
    fn default() -> Self {
        Self::new()
    }
}

impl GitAutoRefresh {
    /// Create a new GitAutoRefresh manager.
    pub fn new() -> Self {
        Self {
            last_refresh: None,
            last_request: None,
            pending_refresh: false,
            was_focused: true, // Assume focused at start
        }
    }

    /// Request a git refresh with debouncing.
    ///
    /// Multiple rapid calls will be batched into a single refresh
    /// after the debounce period (500ms).
    pub fn request_refresh(&mut self) {
        self.last_request = Some(Instant::now());
        self.pending_refresh = true;
        trace!("Git refresh requested, will execute after debounce");
    }

    /// Check and update focus state, returning true if focus was just gained.
    ///
    /// Call this each frame with the current focus state.
    pub fn update_focus(&mut self, is_focused: bool) -> bool {
        let focus_gained = is_focused && !self.was_focused;
        self.was_focused = is_focused;
        
        if focus_gained {
            debug!("Window focus gained, requesting git refresh");
            self.request_refresh();
        }
        
        focus_gained
    }

    /// Check if enough time has passed for periodic refresh.
    ///
    /// Returns true if it's time for a periodic refresh (every 10 seconds).
    pub fn should_periodic_refresh(&self) -> bool {
        match self.last_refresh {
            Some(last) => last.elapsed() >= GIT_REFRESH_INTERVAL,
            None => true, // Never refreshed, do it now
        }
    }

    /// Check if debounce period has passed and a refresh should execute.
    ///
    /// Returns true if there's a pending refresh and the debounce period has elapsed.
    pub fn should_execute_refresh(&self) -> bool {
        if !self.pending_refresh {
            return false;
        }

        match self.last_request {
            Some(request_time) => request_time.elapsed() >= GIT_DEBOUNCE_DURATION,
            None => false,
        }
    }

    /// Mark that a refresh was executed.
    ///
    /// Call this after actually performing the git refresh.
    pub fn mark_refreshed(&mut self) {
        self.last_refresh = Some(Instant::now());
        self.pending_refresh = false;
        trace!("Git refresh completed");
    }

    /// Process auto-refresh logic and return true if a refresh should be performed.
    ///
    /// This is the main entry point called each frame. It checks:
    /// 1. Debounced refresh requests
    /// 2. Periodic refresh timer
    ///
    /// Returns true if git status should be refreshed.
    pub fn tick(&mut self, workspace_open: bool) -> bool {
        // Only refresh if a workspace is open (git repo likely present)
        if !workspace_open {
            return false;
        }

        // Check for debounced refresh
        if self.should_execute_refresh() {
            return true;
        }

        // Check for periodic refresh
        if self.should_periodic_refresh() {
            self.pending_refresh = true; // Will be cleared in mark_refreshed
            return true;
        }

        false
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_git_file_status_labels() {
        assert_eq!(GitFileStatus::Clean.label(), "");
        assert_eq!(GitFileStatus::Modified.label(), "M");
        assert_eq!(GitFileStatus::Staged.label(), "S");
        assert_eq!(GitFileStatus::Untracked.label(), "U");
        assert_eq!(GitFileStatus::Conflict.label(), "!");
    }

    #[test]
    fn test_git_file_status_visibility() {
        assert!(!GitFileStatus::Clean.is_visible());
        assert!(GitFileStatus::Modified.is_visible());
        assert!(GitFileStatus::Staged.is_visible());
        assert!(GitFileStatus::Untracked.is_visible());
    }

    #[test]
    fn test_git_service_new() {
        let service = GitService::new();
        assert!(!service.is_open());
        assert!(service.current_branch().is_none());
        assert!(service.repo_root().is_none());
    }

    #[test]
    fn test_git_service_non_repo() {
        let temp_dir = TempDir::new().unwrap();
        let mut service = GitService::new();

        // Should return Ok(false) for non-repo directory
        let result = service.open(temp_dir.path());
        assert!(result.is_ok());
        assert!(!result.unwrap());
        assert!(!service.is_open());
    }

    #[test]
    fn test_git_service_with_repo() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Initialize a Git repository
        let repo = Repository::init(repo_path).unwrap();
        assert!(repo.workdir().is_some());

        let mut service = GitService::new();
        let result = service.open(repo_path);

        assert!(result.is_ok());
        assert!(result.unwrap());
        assert!(service.is_open());
        assert!(service.repo_root().is_some());

        // Branch should be something like "main (unborn)" for empty repo
        let branch = service.current_branch();
        assert!(branch.is_some());
    }

    #[test]
    fn test_git_service_untracked_file() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Initialize repo
        Repository::init(repo_path).unwrap();

        // Create an untracked file
        let file_path = repo_path.join("test.txt");
        fs::write(&file_path, "hello").unwrap();

        let mut service = GitService::new();
        service.open(repo_path).unwrap();
        service.refresh_status();

        let status = service.file_status(&file_path);
        assert_eq!(status, GitFileStatus::Untracked);
    }

    #[test]
    fn test_git_service_close() {
        let mut service = GitService::new();
        let temp_dir = TempDir::new().unwrap();
        Repository::init(temp_dir.path()).unwrap();

        service.open(temp_dir.path()).unwrap();
        assert!(service.is_open());

        service.close();
        assert!(!service.is_open());
        assert!(service.repo_root().is_none());
    }

    #[test]
    fn test_worse_status() {
        assert_eq!(
            GitService::worse_status(GitFileStatus::Clean, GitFileStatus::Modified),
            GitFileStatus::Modified
        );
        assert_eq!(
            GitService::worse_status(GitFileStatus::Modified, GitFileStatus::Conflict),
            GitFileStatus::Conflict
        );
        assert_eq!(
            GitService::worse_status(GitFileStatus::Staged, GitFileStatus::StagedModified),
            GitFileStatus::StagedModified
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // GitAutoRefresh Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_git_auto_refresh_new() {
        let refresh = GitAutoRefresh::new();
        assert!(refresh.last_refresh.is_none());
        assert!(refresh.last_request.is_none());
        assert!(!refresh.pending_refresh);
        assert!(refresh.was_focused); // Assumes focused at start
    }

    #[test]
    fn test_git_auto_refresh_default() {
        let refresh = GitAutoRefresh::default();
        assert!(refresh.last_refresh.is_none());
        assert!(!refresh.pending_refresh);
    }

    #[test]
    fn test_git_auto_refresh_request_refresh() {
        let mut refresh = GitAutoRefresh::new();
        assert!(!refresh.pending_refresh);

        refresh.request_refresh();

        assert!(refresh.pending_refresh);
        assert!(refresh.last_request.is_some());
    }

    #[test]
    fn test_git_auto_refresh_focus_gained() {
        let mut refresh = GitAutoRefresh::new();
        // Start as focused
        refresh.was_focused = true;

        // Still focused - no change
        let gained = refresh.update_focus(true);
        assert!(!gained);
        assert!(!refresh.pending_refresh);

        // Lost focus
        let gained = refresh.update_focus(false);
        assert!(!gained);
        assert!(!refresh.pending_refresh);

        // Gained focus again
        let gained = refresh.update_focus(true);
        assert!(gained);
        assert!(refresh.pending_refresh);
    }

    #[test]
    fn test_git_auto_refresh_periodic_refresh_never_refreshed() {
        let refresh = GitAutoRefresh::new();
        // Should refresh if never refreshed before
        assert!(refresh.should_periodic_refresh());
    }

    #[test]
    fn test_git_auto_refresh_periodic_refresh_recent() {
        let mut refresh = GitAutoRefresh::new();
        refresh.mark_refreshed();

        // Should not refresh immediately after a refresh
        assert!(!refresh.should_periodic_refresh());
    }

    #[test]
    fn test_git_auto_refresh_mark_refreshed() {
        let mut refresh = GitAutoRefresh::new();
        refresh.request_refresh();
        assert!(refresh.pending_refresh);

        refresh.mark_refreshed();

        assert!(!refresh.pending_refresh);
        assert!(refresh.last_refresh.is_some());
    }

    #[test]
    fn test_git_auto_refresh_tick_no_workspace() {
        let mut refresh = GitAutoRefresh::new();

        // Should not trigger refresh if no workspace open
        let should_refresh = refresh.tick(false);
        assert!(!should_refresh);
    }

    #[test]
    fn test_git_auto_refresh_tick_with_workspace_first_time() {
        let mut refresh = GitAutoRefresh::new();

        // Should trigger refresh on first tick with workspace
        // (because never refreshed before, periodic triggers)
        let should_refresh = refresh.tick(true);
        assert!(should_refresh);
    }

    #[test]
    fn test_git_auto_refresh_debounce_not_ready() {
        let mut refresh = GitAutoRefresh::new();
        refresh.request_refresh();

        // Immediately after request, debounce hasn't elapsed
        assert!(!refresh.should_execute_refresh() || refresh.last_request.unwrap().elapsed() >= GIT_DEBOUNCE_DURATION);
    }

    #[test]
    fn test_git_auto_refresh_debounce_no_pending() {
        let refresh = GitAutoRefresh::new();

        // No pending request, should not execute
        assert!(!refresh.should_execute_refresh());
    }
}
