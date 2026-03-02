use crate::types::{GroupNode, SessionStatus, TreeNode};

/// A flattened entry for the finder, derived from the session tree.
#[derive(Debug, Clone)]
pub struct FinderEntry {
    pub session_id: String,
    pub display_name: String,
    pub group_name: String,
    pub cwd: String,
    pub status: SessionStatus,
    pub last_active: String,
}

/// State for the modal session finder.
pub struct FinderState {
    pub query: String,
    entries: Vec<FinderEntry>,
    filtered: Vec<usize>,
    pub cursor: usize,
}

impl FinderState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            entries: Vec::new(),
            filtered: Vec::new(),
            cursor: 0,
        }
    }

    /// Open the finder by flattening the tree into searchable entries.
    pub fn open(&mut self, tree: &[TreeNode], show_dead: bool) {
        self.query.clear();
        self.cursor = 0;
        self.entries.clear();
        Self::flatten_tree(tree, show_dead, "", &mut self.entries);
        // Initial filtered list: all entries, sorted by last_active descending
        self.filtered = (0..self.entries.len()).collect();
        self.filtered.sort_by(|&a, &b| {
            self.entries[b]
                .last_active
                .cmp(&self.entries[a].last_active)
        });
    }

    /// Flatten tree nodes into finder entries, recursing into groups.
    fn flatten_tree(
        nodes: &[TreeNode],
        show_dead: bool,
        parent_group: &str,
        out: &mut Vec<FinderEntry>,
    ) {
        for node in nodes {
            match node {
                TreeNode::Group(GroupNode { name, children, .. }) => {
                    Self::flatten_tree(children, show_dead, name, out);
                }
                TreeNode::Session(s) => {
                    if !show_dead && s.status == SessionStatus::Dead {
                        continue;
                    }
                    out.push(FinderEntry {
                        session_id: s.session_id.clone(),
                        display_name: s.display_name.clone(),
                        group_name: parent_group.to_string(),
                        cwd: s
                            .cwd
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default(),
                        status: s.status,
                        last_active: s.last_active.clone(),
                    });
                }
            }
        }
    }

    /// Update the query and re-filter/re-score entries.
    pub fn update_query(&mut self, query: String) {
        self.query = query;
        self.refilter();
    }

    /// Re-filter entries based on current query.
    fn refilter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.entries.len()).collect();
            self.filtered.sort_by(|&a, &b| {
                self.entries[b]
                    .last_active
                    .cmp(&self.entries[a].last_active)
            });
        } else {
            let query_lower = self.query.to_lowercase();
            let mut scored: Vec<(usize, u32)> = self
                .entries
                .iter()
                .enumerate()
                .filter_map(|(i, entry)| {
                    let score = Self::score(entry, &query_lower);
                    if score > 0 {
                        Some((i, score))
                    } else {
                        None
                    }
                })
                .collect();
            // Higher score first; ties broken by last_active descending
            scored.sort_by(|a, b| {
                b.1.cmp(&a.1).then_with(|| {
                    self.entries[b.0]
                        .last_active
                        .cmp(&self.entries[a.0].last_active)
                })
            });
            self.filtered = scored.into_iter().map(|(i, _)| i).collect();
        }
        // Reset cursor to top
        self.cursor = 0;
    }

    /// Score a finder entry against a lowercased query.
    /// Returns 0 if no match. Higher scores = better matches.
    fn score(entry: &FinderEntry, query_lower: &str) -> u32 {
        let name_lower = entry.display_name.to_lowercase();
        let group_lower = entry.group_name.to_lowercase();
        let cwd_lower = entry.cwd.to_lowercase();

        // Priority: name prefix (100) > name substring (80) >
        //           group prefix (60) > group substring (40) > cwd match (20)
        if name_lower.starts_with(query_lower) {
            100
        } else if name_lower.contains(query_lower) {
            80
        } else if group_lower.starts_with(query_lower) {
            60
        } else if group_lower.contains(query_lower) {
            40
        } else if cwd_lower.contains(query_lower) {
            20
        } else {
            0
        }
    }

    /// Get the currently selected entry, if any.
    pub fn selected(&self) -> Option<&FinderEntry> {
        self.filtered.get(self.cursor).map(|&i| &self.entries[i])
    }

    /// Get the filtered results for rendering.
    pub fn results(&self) -> Vec<&FinderEntry> {
        self.filtered.iter().map(|&i| &self.entries[i]).collect()
    }

    /// Number of filtered results.
    #[cfg(test)]
    fn result_count(&self) -> usize {
        self.filtered.len()
    }

    /// Move cursor up, wrapping to bottom.
    pub fn cursor_up(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        if self.cursor == 0 {
            self.cursor = self.filtered.len() - 1;
        } else {
            self.cursor -= 1;
        }
    }

    /// Move cursor down, wrapping to top.
    pub fn cursor_down(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.cursor = (self.cursor + 1) % self.filtered.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;

    #[test]
    fn test_open_flattens_all_sessions() {
        let tree = mock::mock_tree();
        let mut state = FinderState::new();
        state.open(&tree, true);
        // mock_tree has 5 sessions total
        assert_eq!(state.entries.len(), 5);
    }

    #[test]
    fn test_open_excludes_dead_when_show_dead_false() {
        let tree = mock::mock_tree();
        let mut state = FinderState::new();
        state.open(&tree, false);
        // 5 total, 2 dead (api-auth-endpoints + quick-question)
        assert_eq!(state.entries.len(), 3);
    }

    #[test]
    fn test_empty_query_shows_all_sorted_by_last_active() {
        let tree = mock::mock_tree();
        let mut state = FinderState::new();
        state.open(&tree, true);
        let results = state.results();
        // Verify sorted by last_active descending
        for window in results.windows(2) {
            assert!(window[0].last_active >= window[1].last_active);
        }
    }

    #[test]
    fn test_filter_by_name_prefix() {
        let tree = mock::mock_tree();
        let mut state = FinderState::new();
        state.open(&tree, true);
        state.update_query("feat".to_string());
        let results = state.results();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].display_name, "feat/scanner");
    }

    #[test]
    fn test_filter_by_name_substring() {
        let tree = mock::mock_tree();
        let mut state = FinderState::new();
        state.open(&tree, true);
        state.update_query("render".to_string());
        let results = state.results();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].display_name, "fix/render-tick");
    }

    #[test]
    fn test_filter_by_group_name() {
        let tree = mock::mock_tree();
        let mut state = FinderState::new();
        state.open(&tree, true);
        state.update_query("api-work".to_string());
        let results = state.results();
        // "api-work" group has 1 session
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].group_name, "api-work");
    }

    #[test]
    fn test_filter_by_cwd() {
        let tree = mock::mock_tree();
        let mut state = FinderState::new();
        state.open(&tree, true);
        state.update_query("/Users/dev/Code/website".to_string());
        let results = state.results();
        // 2 sessions with cwd containing "website"
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_filter_case_insensitive() {
        let tree = mock::mock_tree();
        let mut state = FinderState::new();
        state.open(&tree, true);
        state.update_query("FEAT".to_string());
        let results = state.results();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].display_name, "feat/scanner");
    }

    #[test]
    fn test_no_match_returns_empty() {
        let tree = mock::mock_tree();
        let mut state = FinderState::new();
        state.open(&tree, true);
        state.update_query("zzzznotfound".to_string());
        assert_eq!(state.result_count(), 0);
        assert!(state.selected().is_none());
    }

    #[test]
    fn test_cursor_navigation() {
        let tree = mock::mock_tree();
        let mut state = FinderState::new();
        state.open(&tree, true);
        assert_eq!(state.cursor, 0);

        state.cursor_down();
        assert_eq!(state.cursor, 1);

        state.cursor_up();
        assert_eq!(state.cursor, 0);

        // Wrap up from 0
        state.cursor_up();
        assert_eq!(state.cursor, state.result_count() - 1);

        // Wrap down from last
        state.cursor_down();
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_cursor_on_empty_results() {
        let tree = mock::mock_tree();
        let mut state = FinderState::new();
        state.open(&tree, true);
        state.update_query("zzzznotfound".to_string());
        // Should not panic
        state.cursor_up();
        state.cursor_down();
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_scoring_priority() {
        let entry = FinderEntry {
            session_id: "1".to_string(),
            display_name: "auth-refactor".to_string(),
            group_name: "work".to_string(),
            cwd: "/home/user/code".to_string(),
            status: SessionStatus::Active,
            last_active: "2026-01-01".to_string(),
        };

        assert_eq!(FinderState::score(&entry, "auth"), 100); // name prefix
        assert_eq!(FinderState::score(&entry, "refactor"), 80); // name substring
        assert_eq!(FinderState::score(&entry, "work"), 60); // group prefix
        assert_eq!(FinderState::score(&entry, "code"), 20); // CWD match
        assert_eq!(FinderState::score(&entry, "zzz"), 0); // no match
    }

    #[test]
    fn test_group_name_populated() {
        let tree = mock::mock_tree();
        let mut state = FinderState::new();
        state.open(&tree, true);
        // feat/scanner is in "nexus" group
        let scanner = state
            .entries
            .iter()
            .find(|e| e.display_name == "feat/scanner")
            .unwrap();
        assert_eq!(scanner.group_name, "nexus");

        // api-auth-endpoints is in "api-work" subgroup
        let api = state
            .entries
            .iter()
            .find(|e| e.display_name == "api-auth-endpoints")
            .unwrap();
        assert_eq!(api.group_name, "api-work");
    }

    #[test]
    fn test_query_reset_on_open() {
        let tree = mock::mock_tree();
        let mut state = FinderState::new();
        state.open(&tree, true);
        state.update_query("test".to_string());
        state.cursor = 2;

        // Re-open should reset
        state.open(&tree, true);
        assert!(state.query.is_empty());
        assert_eq!(state.cursor, 0);
    }
}
