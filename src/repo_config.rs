use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct RepoConfig {
    #[serde(default)]
    pub worktree: RepoWorktreeConfig,
}

#[derive(Debug, Default, Deserialize)]
pub struct RepoWorktreeConfig {
    #[serde(default)]
    pub branch_prefix: Option<String>,
}

/// Load per-repo config from `.nexus.toml` at the repo root.
/// Returns defaults silently if the file is missing or malformed.
pub fn load_repo_config(repo_root: &Path) -> RepoConfig {
    let path = repo_root.join(".nexus.toml");
    if !path.exists() {
        return RepoConfig::default();
    }
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return RepoConfig::default();
    };
    toml::from_str(&raw).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = load_repo_config(tmp.path());
        assert!(cfg.worktree.branch_prefix.is_none());
    }

    #[test]
    fn test_load_with_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(".nexus.toml"),
            "[worktree]\nbranch_prefix = \"custom\"\n",
        )
        .unwrap();
        let cfg = load_repo_config(tmp.path());
        assert_eq!(cfg.worktree.branch_prefix, Some("custom".to_string()));
    }

    #[test]
    fn test_load_with_empty_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(".nexus.toml"),
            "[worktree]\nbranch_prefix = \"\"\n",
        )
        .unwrap();
        let cfg = load_repo_config(tmp.path());
        assert_eq!(cfg.worktree.branch_prefix, Some(String::new()));
    }

    #[test]
    fn test_load_malformed_toml() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".nexus.toml"), "[invalid\n").unwrap();
        let cfg = load_repo_config(tmp.path());
        assert!(cfg.worktree.branch_prefix.is_none());
    }

    #[test]
    fn test_load_empty_file() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".nexus.toml"), "").unwrap();
        let cfg = load_repo_config(tmp.path());
        assert!(cfg.worktree.branch_prefix.is_none());
    }
}
