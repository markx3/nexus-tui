use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent};

use crate::types::{GroupIcon, GroupId, SelectionTarget, SessionSummary, TreeNode};

// ---------------------------------------------------------------------------
// Actions emitted by key handling
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum TreeAction {
    Select(SelectionTarget),
    ToggleExpand(GroupId),
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
// Free function: flatten tree into visible nodes
// ---------------------------------------------------------------------------

fn flatten_tree(
    nodes: &[TreeNode],
    expanded: &HashSet<GroupId>,
    depth: u16,
    out: &mut Vec<FlatNode>,
) {
    for node in nodes {
        match node {
            TreeNode::Group(g) => {
                let is_expanded = expanded.contains(&g.id);
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
                    flatten_tree(&g.children, expanded, depth + 1, out);
                }
            }
            TreeNode::Session(s) => {
                out.push(FlatNode {
                    depth,
                    node: FlatNodeKind::Session { summary: s.clone() },
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TreeState
// ---------------------------------------------------------------------------

pub struct TreeState {
    pub cursor_index: usize,
    pub expanded: HashSet<GroupId>,
    pub scroll_offset: usize,
    // Internal cache for visible nodes (Todo 014)
    cached_flat: Vec<FlatNode>,
    cache_valid: bool,
}

impl TreeState {
    /// Create a new TreeState with all groups expanded by default.
    pub fn new(tree: &[TreeNode]) -> Self {
        let mut expanded = HashSet::new();
        Self::collect_group_ids(tree, &mut expanded);
        let mut state = Self {
            cursor_index: 0,
            expanded,
            scroll_offset: 0,
            cached_flat: Vec::new(),
            cache_valid: false,
        };
        state.ensure_cache(tree);
        state
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

    /// Recompute the flat node cache if it has been invalidated.
    fn ensure_cache(&mut self, tree: &[TreeNode]) {
        if !self.cache_valid {
            self.cached_flat.clear();
            flatten_tree(tree, &self.expanded, 0, &mut self.cached_flat);
            self.cache_valid = true;
        }
    }

    /// Mark the cache as stale so it will be recomputed on next access.
    pub fn invalidate_cache(&mut self) {
        self.cache_valid = false;
    }

    /// Flatten the tree into a list of visible nodes, respecting collapsed state.
    /// This returns a freshly computed Vec (backward-compatible for external callers).
    pub fn visible_nodes(&self, tree: &[TreeNode]) -> Vec<FlatNode> {
        // If cache is valid, clone from cache to avoid recomputing
        if self.cache_valid {
            return self.cached_flat.clone();
        }
        let mut out = Vec::new();
        flatten_tree(tree, &self.expanded, 0, &mut out);
        out
    }

    /// Move cursor up, wrapping around to the bottom.
    pub fn move_cursor_up(&mut self, tree: &[TreeNode]) {
        self.ensure_cache(tree);
        let count = self.cached_flat.len();
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
        self.ensure_cache(tree);
        let count = self.cached_flat.len();
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
        self.invalidate_cache();
    }

    /// Get the selection target at the current cursor position.
    pub fn selected_target(&mut self, tree: &[TreeNode]) -> Option<SelectionTarget> {
        self.ensure_cache(tree);
        self.cached_flat
            .get(self.cursor_index)
            .map(|n| match &n.node {
                FlatNodeKind::Group { id, .. } => SelectionTarget::Group(*id),
                FlatNodeKind::Session { summary } => {
                    SelectionTarget::Session(summary.session_id.clone())
                }
            })
    }

    /// Handle a key event, returning an optional action.
    pub fn handle_key(&mut self, key: KeyEvent, tree: &[TreeNode]) -> Option<TreeAction> {
        self.handle_normal_key(key, tree)
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
                self.ensure_cache(tree);
                if let Some(node) = self.cached_flat.get(self.cursor_index) {
                    match &node.node {
                        FlatNodeKind::Group { id, .. } => {
                            let gid = *id;
                            self.toggle_expand(gid);
                            Some(TreeAction::ToggleExpand(gid))
                        }
                        FlatNodeKind::Session { summary } => {
                            let target = SelectionTarget::Session(summary.session_id.clone());
                            Some(TreeAction::Select(target))
                        }
                    }
                } else {
                    None
                }
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

    #[test]
    fn test_cache_invalidated_on_toggle() {
        let tree = mock::mock_tree();
        let mut state = TreeState::new(&tree);

        // Cache starts valid after new()
        assert!(state.cache_valid);

        // Toggle invalidates
        state.toggle_expand(1);
        assert!(!state.cache_valid);

        // ensure_cache rebuilds
        state.ensure_cache(&tree);
        assert!(state.cache_valid);
        assert_eq!(state.cached_flat.len(), 7); // nexus collapsed: 9-2=7
    }

    #[test]
    fn test_visible_nodes_uses_cache_when_valid() {
        let tree = mock::mock_tree();
        let state = TreeState::new(&tree);

        // cache_valid should be true after new()
        assert!(state.cache_valid);

        let flat = state.visible_nodes(&tree);
        assert_eq!(flat.len(), 9);
    }
}
