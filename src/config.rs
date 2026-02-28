use std::path::PathBuf;

use color_eyre::eyre::{bail, WrapErr};
use color_eyre::Result;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Config structs
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct NexusConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub groups: Vec<GroupDef>,
    #[serde(default)]
    pub auto_group: Vec<AutoGroupRule>,
    #[serde(default)]
    pub tmux: TmuxConfig,
}

#[derive(Debug, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_projects_dir")]
    pub projects_dir: PathBuf,
    #[serde(default = "default_db_path")]
    pub db_path: PathBuf,
    #[serde(default = "default_tick_rate")]
    pub tick_rate_ms: u64,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            projects_dir: default_projects_dir(),
            db_path: default_db_path(),
            tick_rate_ms: default_tick_rate(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GroupDef {
    pub name: String,
    #[serde(default)]
    pub icon: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AutoGroupRule {
    pub pattern: String, // glob pattern matched against cwd
    pub group: String,   // target group name
}

#[derive(Debug, Deserialize)]
pub struct TmuxConfig {
    #[serde(default = "default_socket_name")]
    pub socket_name: String,
    #[serde(default = "default_true")]
    pub auto_launch: bool,
}

impl Default for TmuxConfig {
    fn default() -> Self {
        Self {
            socket_name: default_socket_name(),
            auto_launch: default_true(),
        }
    }
}

// ---------------------------------------------------------------------------
// Default value helpers
// ---------------------------------------------------------------------------

fn default_projects_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".claude/projects")
}

fn default_db_path() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("nexus/nexus.db")
}

fn default_tick_rate() -> u64 {
    250
}

fn default_socket_name() -> String {
    "nexus".to_string()
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Config file path
// ---------------------------------------------------------------------------

/// Returns the canonical config file path: `~/.config/nexus/config.toml`.
fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("nexus/config.toml"))
}

// ---------------------------------------------------------------------------
// Loading & validation
// ---------------------------------------------------------------------------

/// Load configuration from `~/.config/nexus/config.toml`.
///
/// Returns a fully-defaulted `NexusConfig` if the file does not exist.
/// Fails with a descriptive error if the file exists but cannot be parsed,
/// or if validation checks fail.
pub fn load_config() -> Result<NexusConfig> {
    let path = match config_path() {
        Some(p) => p,
        None => return Ok(NexusConfig::default()),
    };

    if !path.exists() {
        return Ok(NexusConfig::default());
    }

    let raw = std::fs::read_to_string(&path)
        .wrap_err_with(|| format!("failed to read config at {}", path.display()))?;

    parse_and_validate(&raw)
        .wrap_err_with(|| format!("invalid config at {}", path.display()))
}

/// Parse a TOML string into a validated `NexusConfig`.
///
/// This is split out so tests can call it without touching the filesystem.
pub fn parse_and_validate(toml_str: &str) -> Result<NexusConfig> {
    let config: NexusConfig =
        toml::from_str(toml_str).wrap_err("TOML parse error")?;

    validate(&config)?;
    Ok(config)
}

/// Run validation rules across the config.
fn validate(config: &NexusConfig) -> Result<()> {
    // Tick rate must be at least 16 ms (~60 fps) to avoid busy-looping.
    if config.general.tick_rate_ms < 16 {
        bail!(
            "tick_rate_ms must be >= 16 (got {})",
            config.general.tick_rate_ms
        );
    }

    // Auto-group patterns must be non-empty.
    for (i, rule) in config.auto_group.iter().enumerate() {
        if rule.pattern.trim().is_empty() {
            bail!("auto_group[{}].pattern must not be empty", i);
        }
        if rule.group.trim().is_empty() {
            bail!("auto_group[{}].group must not be empty", i);
        }
    }

    // Group names must be non-empty.
    for (i, g) in config.groups.iter().enumerate() {
        if g.name.trim().is_empty() {
            bail!("groups[{}].name must not be empty", i);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        let cfg = NexusConfig::default();
        assert!(validate(&cfg).is_ok());
        assert_eq!(cfg.general.tick_rate_ms, 250);
        assert!(cfg.general.projects_dir.ends_with(".claude/projects"));
        assert!(cfg.general.db_path.ends_with("nexus/nexus.db"));
        assert_eq!(cfg.tmux.socket_name, "nexus");
        assert!(cfg.tmux.auto_launch);
        assert!(cfg.groups.is_empty());
        assert!(cfg.auto_group.is_empty());
    }

    #[test]
    fn test_parse_full_toml() {
        let toml_str = r#"
[general]
projects_dir = "/custom/projects"
db_path = "/custom/db/nexus.db"
tick_rate_ms = 500

[[groups]]
name = "Work"
icon = "briefcase"

[[groups]]
name = "Personal"
icon = "home"

[[auto_group]]
pattern = "*/work/*"
group = "Work"

[[auto_group]]
pattern = "*/personal/*"
group = "Personal"

[tmux]
socket_name = "custom-nexus"
auto_launch = false
"#;

        let cfg = parse_and_validate(toml_str).unwrap();
        assert_eq!(
            cfg.general.projects_dir,
            PathBuf::from("/custom/projects")
        );
        assert_eq!(
            cfg.general.db_path,
            PathBuf::from("/custom/db/nexus.db")
        );
        assert_eq!(cfg.general.tick_rate_ms, 500);
        assert_eq!(cfg.groups.len(), 2);
        assert_eq!(cfg.groups[0].name, "Work");
        assert_eq!(cfg.groups[0].icon, "briefcase");
        assert_eq!(cfg.groups[1].name, "Personal");
        assert_eq!(cfg.auto_group.len(), 2);
        assert_eq!(cfg.auto_group[0].pattern, "*/work/*");
        assert_eq!(cfg.auto_group[0].group, "Work");
        assert_eq!(cfg.tmux.socket_name, "custom-nexus");
        assert!(!cfg.tmux.auto_launch);
    }

    #[test]
    fn test_empty_toml_returns_defaults() {
        let cfg = parse_and_validate("").unwrap();
        assert_eq!(cfg.general.tick_rate_ms, 250);
        assert!(cfg.groups.is_empty());
    }

    #[test]
    fn test_missing_file_returns_defaults() {
        // load_config returns defaults when file doesn't exist
        let cfg = load_config().unwrap();
        assert_eq!(cfg.general.tick_rate_ms, 250);
    }

    #[test]
    fn test_tick_rate_too_low() {
        let toml_str = r#"
[general]
tick_rate_ms = 5
"#;
        let err = parse_and_validate(toml_str);
        assert!(err.is_err());
        let msg = format!("{}", err.unwrap_err());
        assert!(msg.contains("tick_rate_ms"), "error: {msg}");
    }

    #[test]
    fn test_empty_auto_group_pattern() {
        let toml_str = r#"
[[auto_group]]
pattern = ""
group = "Work"
"#;
        let err = parse_and_validate(toml_str);
        assert!(err.is_err());
        let msg = format!("{}", err.unwrap_err());
        assert!(msg.contains("pattern"), "error: {msg}");
    }

    #[test]
    fn test_empty_auto_group_group_name() {
        let toml_str = r#"
[[auto_group]]
pattern = "*/foo/*"
group = ""
"#;
        let err = parse_and_validate(toml_str);
        assert!(err.is_err());
        let msg = format!("{}", err.unwrap_err());
        assert!(msg.contains("group"), "error: {msg}");
    }

    #[test]
    fn test_empty_group_name() {
        let toml_str = r#"
[[groups]]
name = "  "
icon = "x"
"#;
        let err = parse_and_validate(toml_str);
        assert!(err.is_err());
        let msg = format!("{}", err.unwrap_err());
        assert!(msg.contains("name"), "error: {msg}");
    }

    #[test]
    fn test_partial_config_fills_defaults() {
        let toml_str = r#"
[general]
tick_rate_ms = 100
"#;
        let cfg = parse_and_validate(toml_str).unwrap();
        assert_eq!(cfg.general.tick_rate_ms, 100);
        // Remaining fields should have defaults
        assert!(cfg.general.projects_dir.ends_with(".claude/projects"));
        assert_eq!(cfg.tmux.socket_name, "nexus");
        assert!(cfg.tmux.auto_launch);
    }

    #[test]
    fn test_invalid_toml_syntax() {
        let bad = r#"
[general
tick_rate_ms = 100
"#;
        let err = parse_and_validate(bad);
        assert!(err.is_err());
    }
}
