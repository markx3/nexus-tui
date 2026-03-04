use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use color_eyre::eyre::WrapErr;
use color_eyre::eyre::{bail, Result};
use wait_timeout::ChildExt;

const HOOK_TIMEOUT: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

pub struct RepoInfo {
    pub root: PathBuf,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Detect if path is inside a git work tree. Returns repo root.
/// Returns None for non-git dirs, bare repos, errors.
pub fn detect_repo(path: &str) -> Option<RepoInfo> {
    let output = Command::new("git")
        .args(["-C", path, "rev-parse", "--is-inside-work-tree"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let is_inside = String::from_utf8_lossy(&output.stdout).trim() == "true";
    if !is_inside {
        return None;
    }

    let root_output = Command::new("git")
        .args(["-C", path, "rev-parse", "--show-toplevel"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !root_output.status.success() {
        return None;
    }

    let root = String::from_utf8_lossy(&root_output.stdout)
        .trim()
        .to_string();
    if root.is_empty() {
        return None;
    }

    Some(RepoInfo {
        root: PathBuf::from(root),
    })
}

/// Check if a git branch exists in the repo.
pub fn branch_exists(repo_root: &Path, branch: &str) -> bool {
    Command::new("git")
        .args([
            "-C",
            &repo_root.to_string_lossy(),
            "rev-parse",
            "--verify",
            &format!("refs/heads/{branch}"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Sanitize a session name into a valid git branch name.
/// ALLOWLIST approach: only permit [a-zA-Z0-9._/-].
/// Always prefix with `nexus/`.
pub fn sanitize_branch_name(session_name: &str) -> String {
    let filtered: String = session_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '/' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();

    // Collapse runs of `-`, trim leading/trailing `-`
    let mut result = String::new();
    let mut last_was_dash = false;
    for c in filtered.chars() {
        if c == '-' {
            if !last_was_dash {
                result.push('-');
            }
            last_was_dash = true;
        } else {
            result.push(c);
            last_was_dash = false;
        }
    }

    let mut result = result.trim_matches('-').to_string();

    // Strip `..` sequences (git ref traversal) and leading dots
    while result.contains("..") {
        result = result.replace("..", ".");
    }
    result = result.trim_start_matches('.').to_string();

    // Collapse consecutive slashes and strip leading/trailing slashes
    while result.contains("//") {
        result = result.replace("//", "/");
    }
    result = result.trim_matches('/').to_string();

    // Strip trailing `.lock` (reserved by git)
    if result.ends_with(".lock") {
        result.truncate(result.len() - 5);
    }

    if result.is_empty() {
        "nexus/session".to_string()
    } else {
        format!("nexus/{result}")
    }
}

/// Create a worktree. Checks for `.nexus/on-worktree-create` convention hook.
/// If hook exists: delegates fully, hook must create worktree at $NEXUS_WORKTREE_PATH.
/// If no hook: runs `git worktree add <path> -b <branch>`.
pub fn create_worktree(
    repo_root: &Path,
    session_name: &str,
    worktree_path: &Path,
    branch: &str,
) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = worktree_path.parent() {
        std::fs::create_dir_all(parent)
            .wrap_err_with(|| format!("cannot create worktree parent dir {}", parent.display()))?;
    }

    let env_vars = hook_env_vars(repo_root, worktree_path, branch, session_name);

    if let Some(hook) = resolve_hook(repo_root, "on-worktree-create") {
        execute_hook(&hook, &env_vars)?;

        // Verify the hook actually created the worktree directory
        if !worktree_path.exists() {
            bail!(
                "Hook at {} did not create worktree at {}",
                hook.display(),
                worktree_path.display()
            );
        }
    } else {
        let output = Command::new("git")
            .args([
                "-C",
                &repo_root.to_string_lossy(),
                "worktree",
                "add",
                &worktree_path.to_string_lossy(),
                "-b",
                branch,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .wrap_err("failed to run git worktree add")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git worktree add failed: {}", stderr.trim());
        }
    }

    Ok(())
}

/// Remove a worktree. Checks for `.nexus/on-worktree-teardown` convention hook.
/// If worktree path doesn't exist on disk: skips cleanup, returns Ok.
pub fn remove_worktree(repo_root: &Path, worktree_path: &Path, branch: &str) -> Result<()> {
    if !worktree_path.exists() {
        return Ok(());
    }

    let env_vars = hook_env_vars(repo_root, worktree_path, branch, "");

    if let Some(hook) = resolve_hook(repo_root, "on-worktree-teardown") {
        execute_hook(&hook, &env_vars)?;
    } else {
        // Force remove in case there are uncommitted changes
        let output = Command::new("git")
            .args([
                "-C",
                &repo_root.to_string_lossy(),
                "worktree",
                "remove",
                "--force",
                &worktree_path.to_string_lossy(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .wrap_err("failed to run git worktree remove")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git worktree remove failed: {}", stderr.trim());
        }

        // Force-delete the branch to match --force worktree removal semantics
        let _ = Command::new("git")
            .args(["-C", &repo_root.to_string_lossy(), "branch", "-D", branch])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Resolve convention hook: check `{repo_root}/.nexus/on-worktree-{create,teardown}`.
/// Validates: file exists, is a regular file (not symlink), has executable bit.
///
/// NOTE: There is an inherent TOCTOU race between this check and the subsequent
/// `execute_hook` call (the file could be replaced between resolution and execution).
/// This is acceptable because hooks live in the user's own repo and this is the same
/// trust model as git's own hook system.
fn resolve_hook(repo_root: &Path, hook_name: &str) -> Option<PathBuf> {
    let hook_path = repo_root.join(".nexus").join(hook_name);

    if !hook_path.exists() {
        return None;
    }

    // Reject symlinks
    if hook_path
        .symlink_metadata()
        .map(|m| m.is_symlink())
        .unwrap_or(true)
    {
        return None;
    }

    // Check executable bit (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = hook_path.metadata() {
            if meta.permissions().mode() & 0o111 == 0 {
                return None;
            }
        } else {
            return None;
        }
    }

    Some(hook_path)
}

/// Execute a hook script DIRECTLY as an executable (not via $SHELL -c).
/// Sets env vars and enforces HOOK_TIMEOUT.
/// On timeout: kills process group to clean up child processes.
fn execute_hook(script: &Path, env: &[(String, String)]) -> Result<()> {
    let mut cmd = Command::new(script);
    cmd.stdout(Stdio::null()).stderr(Stdio::piped());

    // Scrub inherited environment, only forwarding safe vars
    cmd.env_clear();
    for key in &["PATH", "HOME", "SHELL", "USER", "LANG", "TERM"] {
        if let Ok(val) = std::env::var(key) {
            cmd.env(key, val);
        }
    }

    for (k, v) in env {
        cmd.env(k, v);
    }

    // Spawn in a new process group so we can kill the whole group on timeout
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let mut child = cmd
        .spawn()
        .wrap_err_with(|| format!("failed to execute hook at {}", script.display()))?;

    match child
        .wait_timeout(HOOK_TIMEOUT)
        .wrap_err("hook wait error")?
    {
        Some(status) => {
            if !status.success() {
                let stderr = child
                    .stderr
                    .take()
                    .map(|mut s| {
                        let mut buf = String::new();
                        std::io::Read::read_to_string(&mut s, &mut buf).ok();
                        buf
                    })
                    .unwrap_or_default();
                bail!(
                    "Hook {} exited with {}: {}",
                    script.display(),
                    status,
                    stderr.trim()
                );
            }
            Ok(())
        }
        None => {
            // Timeout — kill process group
            #[cfg(unix)]
            {
                let pid = child.id() as libc::pid_t;
                unsafe {
                    libc::kill(-pid, libc::SIGTERM);
                }
            }
            #[cfg(not(unix))]
            {
                let _ = child.kill();
            }
            let _ = child.wait();
            bail!(
                "Hook {} timed out after {}s",
                script.display(),
                HOOK_TIMEOUT.as_secs()
            );
        }
    }
}

fn hook_env_vars(
    repo_root: &Path,
    worktree_path: &Path,
    branch: &str,
    session_name: &str,
) -> Vec<(String, String)> {
    vec![
        (
            "NEXUS_WORKTREE_PATH".to_string(),
            worktree_path.to_string_lossy().to_string(),
        ),
        ("NEXUS_BRANCH".to_string(), branch.to_string()),
        ("NEXUS_SESSION_NAME".to_string(), session_name.to_string()),
        (
            "NEXUS_REPO_ROOT".to_string(),
            repo_root.to_string_lossy().to_string(),
        ),
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_branch_name_basic() {
        assert_eq!(sanitize_branch_name("my-feature"), "nexus/my-feature");
    }

    #[test]
    fn test_sanitize_branch_name_spaces() {
        assert_eq!(sanitize_branch_name("my feature"), "nexus/my-feature");
    }

    #[test]
    fn test_sanitize_branch_name_special_chars() {
        assert_eq!(sanitize_branch_name("hello!@#$world"), "nexus/hello-world");
    }

    #[test]
    fn test_sanitize_branch_name_unicode() {
        assert_eq!(sanitize_branch_name("café☕"), "nexus/caf");
    }

    #[test]
    fn test_sanitize_branch_name_empty() {
        assert_eq!(sanitize_branch_name(""), "nexus/session");
    }

    #[test]
    fn test_sanitize_branch_name_only_special() {
        assert_eq!(sanitize_branch_name("!!!"), "nexus/session");
    }

    #[test]
    fn test_sanitize_branch_name_force_flag() {
        // --force should be sanitized to nexus/force (dashes stripped from edges)
        assert_eq!(sanitize_branch_name("--force"), "nexus/force");
    }

    #[test]
    fn test_sanitize_branch_name_dash_b() {
        assert_eq!(sanitize_branch_name("-b"), "nexus/b");
    }

    #[test]
    fn test_sanitize_branch_name_path_escape() {
        // `..` sequences are stripped to prevent git ref traversal
        assert_eq!(sanitize_branch_name("../escape"), "nexus/escape");
    }

    #[test]
    fn test_sanitize_branch_name_dot_lock() {
        assert_eq!(sanitize_branch_name("my-branch.lock"), "nexus/my-branch");
    }

    #[test]
    fn test_sanitize_branch_name_consecutive_dashes() {
        assert_eq!(sanitize_branch_name("a---b"), "nexus/a-b");
    }

    #[test]
    fn test_sanitize_branch_name_dots_and_slashes() {
        assert_eq!(
            sanitize_branch_name("feat/v2.0/thing"),
            "nexus/feat/v2.0/thing"
        );
    }

    #[test]
    fn test_detect_repo_non_git_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let result = detect_repo(tmp.path().to_str().unwrap());
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_repo_real_git() {
        let tmp = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["init", tmp.path().to_str().unwrap()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        let result = detect_repo(tmp.path().to_str().unwrap());
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().root.canonicalize().unwrap(),
            tmp.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_branch_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        Command::new("git")
            .args(["init", path.to_str().unwrap()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();

        // Create initial commit so branches work
        Command::new("git")
            .args([
                "-C",
                path.to_str().unwrap(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();

        // Default branch should exist
        let default_branch = Command::new("git")
            .args(["-C", path.to_str().unwrap(), "branch", "--show-current"])
            .stdout(Stdio::piped())
            .output()
            .unwrap();
        let default = String::from_utf8_lossy(&default_branch.stdout)
            .trim()
            .to_string();
        assert!(branch_exists(path, &default));

        // Non-existent branch
        assert!(!branch_exists(path, "nexus/does-not-exist"));
    }

    #[test]
    fn test_create_and_remove_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir(&repo).unwrap();

        Command::new("git")
            .args(["init", repo.to_str().unwrap()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        Command::new("git")
            .args([
                "-C",
                repo.to_str().unwrap(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();

        let wt_path = repo.join(".worktrees").join("test-session");
        let branch = "nexus/test-session";

        // Create
        create_worktree(&repo, "test-session", &wt_path, branch).unwrap();
        assert!(wt_path.exists());
        assert!(branch_exists(&repo, branch));

        // Remove
        remove_worktree(&repo, &wt_path, branch).unwrap();
        assert!(!wt_path.exists());
    }

    #[test]
    fn test_remove_worktree_missing_path_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let non_existent = tmp.path().join("does-not-exist");
        // Should return Ok when path doesn't exist
        remove_worktree(tmp.path(), &non_existent, "nexus/whatever").unwrap();
    }

    #[test]
    fn test_resolve_hook_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(resolve_hook(tmp.path(), "on-worktree-create").is_none());
    }

    #[test]
    #[cfg(unix)]
    fn test_resolve_hook_not_executable() {
        let tmp = tempfile::tempdir().unwrap();
        let nexus_dir = tmp.path().join(".nexus");
        std::fs::create_dir(&nexus_dir).unwrap();
        let hook = nexus_dir.join("on-worktree-create");
        std::fs::write(&hook, "#!/bin/bash\necho hello").unwrap();
        // NOT setting executable bit
        assert!(resolve_hook(tmp.path(), "on-worktree-create").is_none());
    }

    #[test]
    #[cfg(unix)]
    fn test_resolve_hook_executable() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let nexus_dir = tmp.path().join(".nexus");
        std::fs::create_dir(&nexus_dir).unwrap();
        let hook = nexus_dir.join("on-worktree-create");
        std::fs::write(&hook, "#!/bin/bash\necho hello").unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
        let result = resolve_hook(tmp.path(), "on-worktree-create");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), hook);
    }

    #[test]
    #[cfg(unix)]
    fn test_resolve_hook_symlink_rejected() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let nexus_dir = tmp.path().join(".nexus");
        std::fs::create_dir(&nexus_dir).unwrap();

        // Create a real file
        let real_hook = tmp.path().join("real-hook");
        std::fs::write(&real_hook, "#!/bin/bash\necho hello").unwrap();
        std::fs::set_permissions(&real_hook, std::fs::Permissions::from_mode(0o755)).unwrap();

        // Create symlink
        let hook_link = nexus_dir.join("on-worktree-create");
        std::os::unix::fs::symlink(&real_hook, &hook_link).unwrap();

        assert!(resolve_hook(tmp.path(), "on-worktree-create").is_none());
    }
}
