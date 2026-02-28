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
    pub collapsed: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum TreeNode {
    Group(GroupNode),
    Session(SessionSummary),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionSummary {
    pub session_id: SessionId,
    pub display_name: String,
    pub cwd: Option<PathBuf>,
    pub project_dir: String,
    pub git_branch: Option<String>,
    pub model: Option<String>,
    pub first_message: Option<String>,
    pub message_count: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub subagent_count: u16,
    pub last_active: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum GroupIcon {
    Root,
    SubGroup,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TmuxWindowInfo {
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
