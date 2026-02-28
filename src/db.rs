use std::path::Path;

use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use rusqlite::{params, Connection};

use crate::scanner::SessionInfo;
use crate::types::{GroupIcon, GroupNode, SessionSummary, TreeNode};

// ---------------------------------------------------------------------------
// Schema SQL
// ---------------------------------------------------------------------------

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
    session_id    TEXT PRIMARY KEY,
    display_name  TEXT NOT NULL,
    cwd           TEXT,
    project_dir   TEXT NOT NULL,
    git_branch    TEXT,
    model         TEXT,
    first_message TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    input_tokens  INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    subagent_count INTEGER NOT NULL DEFAULT 0,
    last_active   TEXT NOT NULL,
    is_active     INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS groups (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL UNIQUE,
    icon       TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS session_groups (
    session_id TEXT NOT NULL,
    group_id   INTEGER NOT NULL,
    PRIMARY KEY (session_id, group_id),
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE,
    FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
);
"#;

// ---------------------------------------------------------------------------
// Database wrapper
// ---------------------------------------------------------------------------

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) a database at the given file path.
    ///
    /// Creates parent directories if they don't exist.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .wrap_err_with(|| format!("cannot create db directory {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .wrap_err_with(|| format!("cannot open database at {}", path.display()))?;

        // Enable WAL mode for better concurrent read performance.
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Open an in-memory database (for tests).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .wrap_err("cannot open in-memory database")?;

        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Create all tables if they don't already exist.
    pub fn init_schema(&self) -> Result<()> {
        self.conn
            .execute_batch(SCHEMA_SQL)
            .wrap_err("failed to initialise database schema")?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Session CRUD
    // -----------------------------------------------------------------------

    /// Insert or replace sessions from a scan result.
    ///
    /// Derives `display_name` from the first non-`None` of:
    /// slug, git_branch, first_message (truncated to 60 chars), or session_id.
    pub fn upsert_sessions(&self, sessions: &[SessionInfo]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;

        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO sessions
                    (session_id, display_name, cwd, project_dir, git_branch,
                     model, first_message, message_count, input_tokens,
                     output_tokens, subagent_count, last_active, is_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            )?;

            for s in sessions {
                let display_name = derive_display_name(s);
                let cwd_str = s.cwd.as_ref().map(|p| p.to_string_lossy().into_owned());

                stmt.execute(params![
                    s.session_id,
                    display_name,
                    cwd_str,
                    s.project_dir,
                    s.git_branch,
                    s.model,
                    s.first_message,
                    s.message_count,
                    s.token_usage.input_tokens,
                    s.token_usage.output_tokens,
                    s.subagent_count,
                    s.last_active,
                    !s.is_complete as i32, // active = not yet completed
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Group CRUD
    // -----------------------------------------------------------------------

    /// Create a group and return its id.
    pub fn create_group(&self, name: &str, icon: &str) -> Result<i64> {
        let max_order: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) FROM groups",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        self.conn.execute(
            "INSERT INTO groups (name, icon, sort_order) VALUES (?1, ?2, ?3)",
            params![name, icon, max_order + 1],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Delete a group by id. Assignments referencing it are cascade-deleted.
    pub fn delete_group(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM groups WHERE id = ?1", params![id])?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Session <-> Group assignment
    // -----------------------------------------------------------------------

    /// Assign a session to a group.
    pub fn assign_session_to_group(&self, session_id: &str, group_id: i64) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO session_groups (session_id, group_id)
             VALUES (?1, ?2)",
            params![session_id, group_id],
        )?;
        Ok(())
    }

    /// Remove a session from all groups.
    pub fn unassign_session(&self, session_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM session_groups WHERE session_id = ?1",
            params![session_id],
        )?;
        Ok(())
    }

    /// Return session_ids that are not assigned to any group.
    pub fn get_ungrouped_sessions(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id FROM sessions
             WHERE session_id NOT IN (SELECT session_id FROM session_groups)
             ORDER BY last_active DESC",
        )?;

        let ids = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ids)
    }

    // -----------------------------------------------------------------------
    // Tree building
    // -----------------------------------------------------------------------

    /// Build the full tree of groups and sessions for the UI.
    ///
    /// 1. Fetch all groups ordered by `sort_order`.
    /// 2. For each group, attach its assigned sessions.
    /// 3. Collect remaining (unassigned) sessions into an "Ungrouped" node.
    pub fn get_tree(&self) -> Result<Vec<TreeNode>> {
        let mut tree: Vec<TreeNode> = Vec::new();

        // -- named groups -------------------------------------------------
        let mut group_stmt = self.conn.prepare(
            "SELECT id, name, icon, sort_order FROM groups ORDER BY sort_order",
        )?;

        let groups: Vec<(i64, String, String)> = group_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        for (gid, gname, _icon) in &groups {
            let children = self.sessions_for_group(*gid)?;
            tree.push(TreeNode::Group(GroupNode {
                id: *gid,
                name: gname.clone(),
                icon: GroupIcon::SubGroup,
                children,
                collapsed: false,
            }));
        }

        // -- ungrouped sessions ------------------------------------------
        let ungrouped = self.ungrouped_session_summaries()?;
        if !ungrouped.is_empty() {
            tree.push(TreeNode::Group(GroupNode {
                id: 0, // sentinel for "Ungrouped"
                name: "Ungrouped".to_string(),
                icon: GroupIcon::Root,
                children: ungrouped.into_iter().map(TreeNode::Session).collect(),
                collapsed: false,
            }));
        }

        Ok(tree)
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Fetch sessions assigned to a specific group.
    fn sessions_for_group(&self, group_id: i64) -> Result<Vec<TreeNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.session_id, s.display_name, s.cwd, s.project_dir,
                    s.git_branch, s.model, s.first_message, s.message_count,
                    s.input_tokens, s.output_tokens, s.subagent_count,
                    s.last_active, s.is_active
             FROM sessions s
             JOIN session_groups sg ON s.session_id = sg.session_id
             WHERE sg.group_id = ?1
             ORDER BY s.last_active DESC",
        )?;

        let rows: Vec<TreeNode> = stmt
            .query_map(params![group_id], |row| Ok(row_to_summary(row)))?
            .filter_map(|r| r.ok())
            .map(TreeNode::Session)
            .collect();

        Ok(rows)
    }

    /// Fetch sessions not assigned to any group.
    fn ungrouped_session_summaries(&self) -> Result<Vec<SessionSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.session_id, s.display_name, s.cwd, s.project_dir,
                    s.git_branch, s.model, s.first_message, s.message_count,
                    s.input_tokens, s.output_tokens, s.subagent_count,
                    s.last_active, s.is_active
             FROM sessions s
             WHERE s.session_id NOT IN (SELECT session_id FROM session_groups)
             ORDER BY s.last_active DESC",
        )?;

        let rows: Vec<SessionSummary> = stmt
            .query_map([], |row| Ok(row_to_summary(row)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Look up a session's cwd by its id.
    pub fn get_session_cwd(&self, session_id: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT cwd FROM sessions WHERE session_id = ?1")?;

        let result = stmt
            .query_row(params![session_id], |row| row.get::<_, Option<String>>(0))
            .ok()
            .flatten();

        Ok(result)
    }

    /// Look up a group id by name, returning `None` if not found.
    pub fn get_group_id_by_name(&self, name: &str) -> Result<Option<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM groups WHERE name = ?1")?;

        let result = stmt
            .query_row(params![name], |row| row.get::<_, i64>(0))
            .ok();

        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

/// Map a rusqlite Row into a `SessionSummary`.
fn row_to_summary(row: &rusqlite::Row<'_>) -> SessionSummary {
    let cwd_str: Option<String> = row.get(2).unwrap_or(None);
    SessionSummary {
        session_id: row.get(0).unwrap_or_default(),
        display_name: row.get(1).unwrap_or_default(),
        cwd: cwd_str.map(std::path::PathBuf::from),
        project_dir: row.get(3).unwrap_or_default(),
        git_branch: row.get(4).unwrap_or(None),
        model: row.get(5).unwrap_or(None),
        first_message: row.get(6).unwrap_or(None),
        message_count: row.get::<_, u32>(7).unwrap_or(0),
        input_tokens: row.get::<_, u64>(8).unwrap_or(0),
        output_tokens: row.get::<_, u64>(9).unwrap_or(0),
        subagent_count: row.get::<_, u16>(10).unwrap_or(0),
        last_active: row.get(11).unwrap_or_default(),
        is_active: row.get::<_, i32>(12).unwrap_or(0) != 0,
    }
}

/// Derive a human-friendly display name from scanner data.
///
/// Priority: slug -> git_branch -> first_message (truncated to 60 chars) -> session_id.
fn derive_display_name(s: &SessionInfo) -> String {
    if let Some(slug) = &s.slug {
        return slug.clone();
    }
    if let Some(branch) = &s.git_branch {
        return branch.clone();
    }
    if let Some(msg) = &s.first_message {
        let truncated = if msg.len() > 60 {
            let mut end = 60;
            while !msg.is_char_boundary(end) && end > 0 {
                end -= 1;
            }
            format!("{}...", &msg[..end])
        } else {
            msg.clone()
        };
        return truncated;
    }
    s.session_id.clone()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{SessionInfo, TokenUsage};
    use std::path::PathBuf;

    fn make_session(id: &str) -> SessionInfo {
        SessionInfo {
            session_id: id.to_string(),
            slug: Some(format!("slug-{id}")),
            cwd: Some(PathBuf::from(format!("/home/user/{id}"))),
            project_dir: "test-project".to_string(),
            git_branch: Some("main".to_string()),
            model: Some("claude-opus-4-6".to_string()),
            version: Some("2.1.50".to_string()),
            first_message: Some("Hello world".to_string()),
            message_count: 5,
            token_usage: TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
            },
            subagent_count: 2,
            last_active: "2026-02-28T10:00:00Z".to_string(),
            source_file: PathBuf::from("/tmp/test.jsonl"),
            is_complete: true,
        }
    }

    #[test]
    fn test_init_schema_idempotent() {
        let db = Database::open_in_memory().unwrap();
        // Calling init_schema again should not fail
        db.init_schema().unwrap();
        db.init_schema().unwrap();
    }

    #[test]
    fn test_upsert_sessions() {
        let db = Database::open_in_memory().unwrap();

        let sessions = vec![make_session("aaa"), make_session("bbb")];
        db.upsert_sessions(&sessions).unwrap();

        // Verify both are stored
        let ungrouped = db.get_ungrouped_sessions().unwrap();
        assert_eq!(ungrouped.len(), 2);
        assert!(ungrouped.contains(&"aaa".to_string()));
        assert!(ungrouped.contains(&"bbb".to_string()));
    }

    #[test]
    fn test_upsert_replaces_existing() {
        let db = Database::open_in_memory().unwrap();

        let mut s = make_session("aaa");
        s.message_count = 5;
        db.upsert_sessions(&[s]).unwrap();

        let mut s2 = make_session("aaa");
        s2.message_count = 10;
        db.upsert_sessions(&[s2]).unwrap();

        // Should still be one session, not two
        let ungrouped = db.get_ungrouped_sessions().unwrap();
        assert_eq!(ungrouped.len(), 1);
    }

    #[test]
    fn test_create_and_delete_group() {
        let db = Database::open_in_memory().unwrap();

        let id = db.create_group("Work", "briefcase").unwrap();
        assert!(id > 0);

        let id2 = db.create_group("Personal", "home").unwrap();
        assert_ne!(id, id2);

        db.delete_group(id).unwrap();

        // After deletion, only "Personal" remains
        let gid = db.get_group_id_by_name("Work").unwrap();
        assert!(gid.is_none());

        let gid2 = db.get_group_id_by_name("Personal").unwrap();
        assert_eq!(gid2, Some(id2));
    }

    #[test]
    fn test_assign_and_unassign_session() {
        let db = Database::open_in_memory().unwrap();

        db.upsert_sessions(&[make_session("aaa"), make_session("bbb")]).unwrap();
        let gid = db.create_group("Work", "").unwrap();

        // Assign aaa to Work
        db.assign_session_to_group("aaa", gid).unwrap();

        let ungrouped = db.get_ungrouped_sessions().unwrap();
        assert_eq!(ungrouped, vec!["bbb".to_string()]);

        // Unassign
        db.unassign_session("aaa").unwrap();
        let ungrouped = db.get_ungrouped_sessions().unwrap();
        assert_eq!(ungrouped.len(), 2);
    }

    #[test]
    fn test_duplicate_assign_is_ignored() {
        let db = Database::open_in_memory().unwrap();

        db.upsert_sessions(&[make_session("aaa")]).unwrap();
        let gid = db.create_group("Work", "").unwrap();

        db.assign_session_to_group("aaa", gid).unwrap();
        // Assigning again should not fail (INSERT OR IGNORE)
        db.assign_session_to_group("aaa", gid).unwrap();

        let ungrouped = db.get_ungrouped_sessions().unwrap();
        assert!(ungrouped.is_empty());
    }

    #[test]
    fn test_get_tree_empty() {
        let db = Database::open_in_memory().unwrap();
        let tree = db.get_tree().unwrap();
        assert!(tree.is_empty());
    }

    #[test]
    fn test_get_tree_with_groups_and_ungrouped() {
        let db = Database::open_in_memory().unwrap();

        db.upsert_sessions(&[
            make_session("aaa"),
            make_session("bbb"),
            make_session("ccc"),
        ])
        .unwrap();

        let gid = db.create_group("Work", "briefcase").unwrap();
        db.assign_session_to_group("aaa", gid).unwrap();

        let tree = db.get_tree().unwrap();

        // Should have 2 nodes: "Work" group + "Ungrouped"
        assert_eq!(tree.len(), 2);

        // First node: Work group with aaa
        match &tree[0] {
            TreeNode::Group(g) => {
                assert_eq!(g.name, "Work");
                assert_eq!(g.children.len(), 1);
                match &g.children[0] {
                    TreeNode::Session(s) => assert_eq!(s.session_id, "aaa"),
                    _ => panic!("Expected session node"),
                }
            }
            _ => panic!("Expected group node"),
        }

        // Second node: Ungrouped with bbb and ccc
        match &tree[1] {
            TreeNode::Group(g) => {
                assert_eq!(g.name, "Ungrouped");
                assert_eq!(g.children.len(), 2);
            }
            _ => panic!("Expected group node"),
        }
    }

    #[test]
    fn test_get_tree_no_ungrouped_when_all_assigned() {
        let db = Database::open_in_memory().unwrap();

        db.upsert_sessions(&[make_session("aaa")]).unwrap();
        let gid = db.create_group("Work", "").unwrap();
        db.assign_session_to_group("aaa", gid).unwrap();

        let tree = db.get_tree().unwrap();
        assert_eq!(tree.len(), 1); // Only Work group, no Ungrouped

        match &tree[0] {
            TreeNode::Group(g) => {
                assert_eq!(g.name, "Work");
                assert_eq!(g.children.len(), 1);
            }
            _ => panic!("Expected group node"),
        }
    }

    #[test]
    fn test_derive_display_name_priority() {
        // slug wins
        let s = make_session("aaa");
        assert_eq!(derive_display_name(&s), "slug-aaa");

        // branch wins when no slug
        let mut s = make_session("aaa");
        s.slug = None;
        assert_eq!(derive_display_name(&s), "main");

        // first_message wins when no slug or branch
        let mut s = make_session("aaa");
        s.slug = None;
        s.git_branch = None;
        assert_eq!(derive_display_name(&s), "Hello world");

        // session_id as fallback
        let mut s = make_session("aaa");
        s.slug = None;
        s.git_branch = None;
        s.first_message = None;
        assert_eq!(derive_display_name(&s), "aaa");
    }

    #[test]
    fn test_derive_display_name_truncation() {
        let mut s = make_session("aaa");
        s.slug = None;
        s.git_branch = None;
        s.first_message = Some("a".repeat(100));

        let name = derive_display_name(&s);
        assert!(name.len() <= 63); // 60 + "..."
        assert!(name.ends_with("..."));
    }

    #[test]
    fn test_get_session_cwd() {
        let db = Database::open_in_memory().unwrap();
        db.upsert_sessions(&[make_session("aaa")]).unwrap();

        let cwd = db.get_session_cwd("aaa").unwrap();
        assert_eq!(cwd, Some("/home/user/aaa".to_string()));

        let cwd_missing = db.get_session_cwd("nonexistent").unwrap();
        assert!(cwd_missing.is_none());
    }

    #[test]
    fn test_cascade_delete_group_removes_assignments() {
        let db = Database::open_in_memory().unwrap();

        db.upsert_sessions(&[make_session("aaa")]).unwrap();
        let gid = db.create_group("Work", "").unwrap();
        db.assign_session_to_group("aaa", gid).unwrap();

        // aaa is assigned
        let ungrouped = db.get_ungrouped_sessions().unwrap();
        assert!(ungrouped.is_empty());

        // Delete the group
        db.delete_group(gid).unwrap();

        // aaa should be ungrouped again
        let ungrouped = db.get_ungrouped_sessions().unwrap();
        assert_eq!(ungrouped, vec!["aaa".to_string()]);
    }

    #[test]
    fn test_group_sort_order() {
        let db = Database::open_in_memory().unwrap();

        db.create_group("Charlie", "").unwrap();
        db.create_group("Alpha", "").unwrap();
        db.create_group("Bravo", "").unwrap();

        // Groups should be ordered by sort_order (creation order), not name
        db.upsert_sessions(&[make_session("aaa")]).unwrap();
        let tree = db.get_tree().unwrap();

        // 3 empty named groups + 1 Ungrouped
        assert_eq!(tree.len(), 4);

        let names: Vec<&str> = tree
            .iter()
            .filter_map(|n| match n {
                TreeNode::Group(g) => Some(g.name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["Charlie", "Alpha", "Bravo", "Ungrouped"]);
    }
}
