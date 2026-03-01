use std::path::PathBuf;

pub type SessionId = String;
pub type GroupId = i64;

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionTarget {
    Session(SessionId),
    Group(GroupId),
}

#[derive(Debug)]
pub struct SelectionState {
    pub selected: Option<SelectionTarget>,
    pub focused_panel: FocusPanel,
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            selected: None,
            focused_panel: FocusPanel::Tree,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPanel {
    Tree,
    Radar,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GroupNode {
    pub id: GroupId,
    pub name: String,
    pub icon: GroupIcon,
    pub children: Vec<TreeNode>,
    #[serde(skip)]
    #[allow(dead_code)]
    pub collapsed: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum TreeNode {
    Group(GroupNode),
    Session(SessionSummary),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum SessionStatus {
    Active,
    Detached,
    Dead,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Active => "active",
            SessionStatus::Detached => "detached",
            SessionStatus::Dead => "dead",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => SessionStatus::Active,
            "detached" => SessionStatus::Detached,
            _ => SessionStatus::Dead,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum SessionOrigin {
    Nexus,
    Scanner,
}

impl SessionOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionOrigin::Nexus => "nexus",
            SessionOrigin::Scanner => "scanner",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "nexus" => SessionOrigin::Nexus,
            _ => SessionOrigin::Scanner,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    TextInput,
    Confirm,
    GroupPicker,
}

#[derive(Debug, Clone)]
pub enum InputContext {
    NewSessionName,
    NewSessionCwd { name: String },
    RenameSession { session_id: String },
    RenameGroup { group_id: GroupId },
    NewGroupName,
    ConfirmDeleteSession { session_id: String, tmux_name: Option<String> },
    ConfirmDeleteGroup { group_id: GroupId },
    MoveSession { session_id: String },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionSummary {
    pub session_id: SessionId,
    pub display_name: String,
    pub cwd: Option<PathBuf>,
    pub last_active: String,
    pub is_active: bool,
    pub status: SessionStatus,
    pub tmux_name: Option<String>,
    pub created_by: SessionOrigin,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum GroupIcon {
    Root,
    SubGroup,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TmuxSessionInfo {
    pub session_id: SessionId,
    pub window_name: String,
    pub is_active: bool,
    pub status: TmuxSessionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum TmuxSessionStatus {
    Running,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum ThemeElement {
    Background,
    Surface,
    Text,
    Dim,
    NeonCyan,
    AcidGreen,
    Hazard,
    NeonMagenta,
    Border,
    ActiveSession,
    IdleSession,
    SelectedItem,
    FocusedBorder,
    UnfocusedBorder,
    TreeIndent,
    RadarRing,
    RadarSweep,
    RadarBlip,
    TopBarLabel,
    TopBarValue,
    DetailLabel,
    DetailValue,
    ActivityGauge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelType {
    TopBar,
    SessionTree,
    Radar,
    Detail,
    ActivityStrip,
}
