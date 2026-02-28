use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent};

use crate::types::{GroupIcon, GroupId, SelectionTarget, SessionSummary, TreeNode};

// ---------------------------------------------------------------------------
// Input mode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeInputMode {
    Normal,
    Search,
    CreateGroup,
    Rename,
    MoveMode,
    ConfirmDelete,
}

// ---------------------------------------------------------------------------
// Actions emitted by key handling
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum TreeAction {
    Select(SelectionTarget),
    ToggleExpand(GroupId),
    EnterSearch,
    ExitSearch,
    StartCreate,
    StartRename,
    StartMove,
    ConfirmDelete,
    CancelAction,
    ScrollUp,
    ScrollDown,
}

// ---------------------------------------------------------------------------
// Flat node produced by visible_nodes()
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FlatNode {
    pub depth: u16,
    pub node: FlatNodeKind,
}

#[derive(Debug, Clone)]
pub enum FlatNodeKind {
    Group {
        id: GroupId,
        name: String,
        icon: GroupIcon,
        child_count: usize,
        collapsed: bool,
    },
    Session {
        summary: SessionSummary,
    },
}

// ---------------------------------------------------------------------------
// TreeState
// ---------------------------------------------------------------------------

pub struct TreeState {
    pub cursor_index: usize,
    pub expanded: HashSet<GroupId>,
    pub search_query: String,
    pub input_mode: TreeInputMode,
    pub scroll_offset: usize,
}

impl TreeState {
    /// Create a new TreeState with all groups expanded by default.
    pub fn new(tree: &[TreeNode]) -> Self {
        let mut expanded = HashSet::new();
        Self::collect_group_ids(tree, &mut expanded);
        Self {
            cursor_index: 0,
            expanded,
            search_query: String::new(),
            input_mode: TreeInputMode::Normal,
            scroll_offset: 0,
        }
    }

    /// Recursively collect all group IDs so they start expanded.
    fn collect_group_ids(nodes: &[TreeNode], ids: &mut HashSet<GroupId>) {
        for node in nodes {
            if let TreeNode::Group(g) = node {
                ids.insert(g.id);
                Self::collect_group_ids(&g.children, ids);
            }
        }
    }

    /// Flatten the tree into a list of visible nodes, respecting collapsed state.
    pub fn visible_nodes(&self, tree: &[TreeNode]) -> Vec<FlatNode> {
        let mut out = Vec::new();
        self.flatten(tree, 0, &mut out);
        out
    }

    fn flatten(&self, nodes: &[TreeNode], depth: u16, out: &mut Vec<FlatNode>) {
        for node in nodes {
            match node {
                TreeNode::Group(g) => {
                    let is_expanded = self.expanded.contains(&g.id);
                    out.push(FlatNode {
                        depth,
                        node: FlatNodeKind::Group {
                            id: g.id,
                            name: g.name.clone(),
                            icon: g.icon,
                            child_count: g.children.len(),
                            collapsed: !is_expanded,
                        },
                    });
                    if is_expanded {
                        self.flatten(&g.children, depth + 1, out);
                    }
                }
                TreeNode::Session(s) => {
                    out.push(FlatNode {
                        depth,
                        node: FlatNodeKind::Session {
                            summary: s.clone(),
                        },
                    });
                }
            }
        }
    }

    /// Move cursor up, wrapping around to the bottom.
    pub fn move_cursor_up(&mut self, tree: &[TreeNode]) {
        let count = self.visible_nodes(tree).len();
        if count == 0 {
            return;
        }
        if self.cursor_index == 0 {
            self.cursor_index = count - 1;
        } else {
            self.cursor_index -= 1;
        }
    }

    /// Move cursor down, wrapping around to the top.
    pub fn move_cursor_down(&mut self, tree: &[TreeNode]) {
        let count = self.visible_nodes(tree).len();
        if count == 0 {
            return;
        }
        self.cursor_index = (self.cursor_index + 1) % count;
    }

    /// Toggle expand/collapse for a group.
    pub fn toggle_expand(&mut self, group_id: GroupId) {
        if self.expanded.contains(&group_id) {
            self.expanded.remove(&group_id);
        } else {
            self.expanded.insert(group_id);
        }
    }

    /// Get the selection target at the current cursor position.
    pub fn selected_target(&self, tree: &[TreeNode]) -> Option<SelectionTarget> {
        let flat = self.visible_nodes(tree);
        flat.get(self.cursor_index).map(|n| match &n.node {
            FlatNodeKind::Group { id, .. } => SelectionTarget::Group(*id),
            FlatNodeKind::Session { summary } => {
                SelectionTarget::Session(summary.session_id.clone())
            }
        })
    }

    /// Handle a key event, returning an optional action.
    pub fn handle_key(&mut self, key: KeyEvent, tree: &[TreeNode]) -> Option<TreeAction> {
        match self.input_mode {
            TreeInputMode::Normal => self.handle_normal_key(key, tree),
            TreeInputMode::Search => self.handle_search_key(key),
            TreeInputMode::ConfirmDelete => self.handle_confirm_delete_key(key),
            _ => self.handle_modal_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent, tree: &[TreeNode]) -> Option<TreeAction> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_cursor_down(tree);
                Some(TreeAction::ScrollDown)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_cursor_up(tree);
                Some(TreeAction::ScrollUp)
            }
            KeyCode::Enter => {
                let flat = self.visible_nodes(tree);
                if let Some(node) = flat.get(self.cursor_index) {
                    match &node.node {
                        FlatNodeKind::Group { id, .. } => {
                            self.toggle_expand(*id);
                            Some(TreeAction::ToggleExpand(*id))
                        }
                        FlatNodeKind::Session { summary } => {
                            let target =
                                SelectionTarget::Session(summary.session_id.clone());
                            Some(TreeAction::Select(target))
                        }
                    }
                } else {
                    None
                }
            }
            KeyCode::Char('/') => {
                self.input_mode = TreeInputMode::Search;
                self.search_query.clear();
                Some(TreeAction::EnterSearch)
            }
            KeyCode::Char('n') => {
                self.input_mode = TreeInputMode::CreateGroup;
                Some(TreeAction::StartCreate)
            }
            KeyCode::Char('r') => {
                self.input_mode = TreeInputMode::Rename;
                Some(TreeAction::StartRename)
            }
            KeyCode::Char('m') => {
                self.input_mode = TreeInputMode::MoveMode;
                Some(TreeAction::StartMove)
            }
            KeyCode::Char('d') => {
                self.input_mode = TreeInputMode::ConfirmDelete;
                Some(TreeAction::ConfirmDelete)
            }
            KeyCode::Esc => Some(TreeAction::CancelAction),
            _ => None,
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Option<TreeAction> {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = TreeInputMode::Normal;
                self.search_query.clear();
                Some(TreeAction::ExitSearch)
            }
            KeyCode::Enter => {
                self.input_mode = TreeInputMode::Normal;
                Some(TreeAction::ExitSearch)
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                None
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                None
            }
            _ => None,
        }
    }

    fn handle_confirm_delete_key(&mut self, key: KeyEvent) -> Option<TreeAction> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                self.input_mode = TreeInputMode::Normal;
                Some(TreeAction::ConfirmDelete)
            }
            _ => {
                self.input_mode = TreeInputMode::Normal;
                Some(TreeAction::CancelAction)
            }
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent) -> Option<TreeAction> {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = TreeInputMode::Normal;
                Some(TreeAction::CancelAction)
            }
            _ => None,
        }
    }

    /// Ensure scroll_offset keeps cursor visible in the given viewport height.
    pub fn ensure_cursor_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }
        if self.cursor_index < self.scroll_offset {
            self.scroll_offset = self.cursor_index;
        } else if self.cursor_index >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.cursor_index - viewport_height + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;

    #[test]
    fn test_visible_nodes_all_expanded() {
        let tree = mock::mock_tree();
        let state = TreeState::new(&tree);
        let flat = state.visible_nodes(&tree);

        // mock_tree has: nexus(group) + 2 sessions, website(group) + 1 session +
        // api-work(group) + 1 session, ungrouped(group) + 1 session
        // All expanded: 3 groups + 5 sessions + 1 subgroup = 9 nodes
        // nexus, feat/scanner, fix/render-tick,
        // website, redesign-landing, api-work, api-auth-endpoints,
        // ungrouped, quick-question
        assert_eq!(flat.len(), 9);
    }

    #[test]
    fn test_visible_nodes_collapsed_group() {
        let tree = mock::mock_tree();
        let mut state = TreeState::new(&tree);

        // Collapse nexus group (id=1) -- hides 2 sessions
        state.toggle_expand(1);
        let flat = state.visible_nodes(&tree);
        // Was 9, minus 2 children of nexus = 7
        assert_eq!(flat.len(), 7);
    }

    #[test]
    fn test_cursor_wraps_down() {
        let tree = mock::mock_tree();
        let mut state = TreeState::new(&tree);
        let count = state.visible_nodes(&tree).len();

        state.cursor_index = count - 1;
        state.move_cursor_down(&tree);
        assert_eq!(state.cursor_index, 0);
    }

    #[test]
    fn test_cursor_wraps_up() {
        let tree = mock::mock_tree();
        let mut state = TreeState::new(&tree);

        state.cursor_index = 0;
        state.move_cursor_up(&tree);
        let count = state.visible_nodes(&tree).len();
        assert_eq!(state.cursor_index, count - 1);
    }

    #[test]
    fn test_toggle_expand() {
        let tree = mock::mock_tree();
        let mut state = TreeState::new(&tree);

        assert!(state.expanded.contains(&1));
        state.toggle_expand(1);
        assert!(!state.expanded.contains(&1));
        state.toggle_expand(1);
        assert!(state.expanded.contains(&1));
    }

    #[test]
    fn test_selected_target_group() {
        let tree = mock::mock_tree();
        let mut state = TreeState::new(&tree);
        state.cursor_index = 0; // first node is nexus group
        let target = state.selected_target(&tree);
        assert_eq!(target, Some(SelectionTarget::Group(1)));
    }

    #[test]
    fn test_selected_target_session() {
        let tree = mock::mock_tree();
        let mut state = TreeState::new(&tree);
        state.cursor_index = 1; // second node is feat/scanner session
        let target = state.selected_target(&tree);
        assert_eq!(
            target,
            Some(SelectionTarget::Session(
                "a1b2c3d4-e5f6-7890-abcd-ef1234567890".to_string()
            ))
        );
    }

    #[test]
    fn test_flat_node_depths() {
        let tree = mock::mock_tree();
        let state = TreeState::new(&tree);
        let flat = state.visible_nodes(&tree);

        // nexus group at depth 0
        assert_eq!(flat[0].depth, 0);
        // its children at depth 1
        assert_eq!(flat[1].depth, 1);
        assert_eq!(flat[2].depth, 1);
        // website group at depth 0
        assert_eq!(flat[3].depth, 0);
        // website child sessions at depth 1
        assert_eq!(flat[4].depth, 1);
        // api-work subgroup at depth 1
        assert_eq!(flat[5].depth, 1);
        // api-work child at depth 2
        assert_eq!(flat[6].depth, 2);
    }

    #[test]
    fn test_ensure_cursor_visible() {
        let tree = mock::mock_tree();
        let mut state = TreeState::new(&tree);
        state.cursor_index = 8;
        state.scroll_offset = 0;
        state.ensure_cursor_visible(5);
        // cursor 8 needs offset at least 8-5+1=4
        assert_eq!(state.scroll_offset, 4);
    }

    #[test]
    fn test_handle_key_j_moves_down() {
        let tree = mock::mock_tree();
        let mut state = TreeState::new(&tree);
        state.cursor_index = 0;

        let key = KeyEvent::from(KeyCode::Char('j'));
        let action = state.handle_key(key, &tree);
        assert_eq!(action, Some(TreeAction::ScrollDown));
        assert_eq!(state.cursor_index, 1);
    }

    #[test]
    fn test_handle_key_enter_toggles_group() {
        let tree = mock::mock_tree();
        let mut state = TreeState::new(&tree);
        state.cursor_index = 0; // nexus group

        let key = KeyEvent::from(KeyCode::Enter);
        let action = state.handle_key(key, &tree);
        assert_eq!(action, Some(TreeAction::ToggleExpand(1)));
        assert!(!state.expanded.contains(&1));
    }
}
