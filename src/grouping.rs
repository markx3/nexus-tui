use crate::config::AutoGroupRule;
use crate::db::Database;
use color_eyre::Result;

/// Apply auto-group rules to unassigned sessions.
///
/// For each unassigned session, check if its cwd matches any rule's glob
/// pattern. If matched, assign the session to the target group (creating the
/// group if it doesn't already exist). Sessions that don't match any rule
/// remain unassigned and will appear under "Ungrouped" in the tree.
///
/// Returns the number of sessions that were newly assigned.
pub fn apply_rules(rules: &[AutoGroupRule], db: &Database) -> Result<usize> {
    if rules.is_empty() {
        return Ok(0);
    }

    // Pre-fetch all ungrouped session cwds in one query
    let sessions_with_cwds = db.get_ungrouped_session_cwds()?;

    // Cache group IDs by name to avoid repeated lookups
    let mut group_cache: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();

    let mut assigned_count: usize = 0;

    for (session_id, cwd_opt) in &sessions_with_cwds {
        let cwd = match cwd_opt {
            Some(c) => c,
            None => continue,
        };

        for rule in rules {
            if glob_match(&rule.pattern, cwd) {
                let group_id = match group_cache.get(&rule.group) {
                    Some(&id) => id,
                    None => {
                        let id = match db.get_group_id_by_name(&rule.group)? {
                            Some(id) => id,
                            None => db.create_group(&rule.group, "")?,
                        };
                        group_cache.insert(rule.group.clone(), id);
                        id
                    }
                };

                db.assign_session_to_group(session_id, group_id)?;
                assigned_count += 1;
                break; // first matching rule wins
            }
        }
    }

    Ok(assigned_count)
}

/// Simple fnmatch-style glob matching.
///
/// Supports:
/// - `*` matches zero or more of any character
/// - `?` matches exactly one character
/// - all other characters match literally
///
/// This is intentionally minimal to avoid adding a dependency for simple
/// path matching.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    glob_match_inner(pattern.as_bytes(), text.as_bytes())
}

fn glob_match_inner(pattern: &[u8], text: &[u8]) -> bool {
    let mut pi = 0; // pattern index
    let mut ti = 0; // text index

    // Saved positions for backtracking when `*` doesn't match greedily.
    let mut star_pi: Option<usize> = None;
    let mut star_ti: usize = 0;

    while ti < text.len() {
        if pi < pattern.len() && (pattern[pi] == b'?' || pattern[pi] == text[ti]) {
            // Exact or single-char wildcard match: advance both.
            pi += 1;
            ti += 1;
        } else if pi < pattern.len() && pattern[pi] == b'*' {
            // Star: record position and try matching zero characters.
            star_pi = Some(pi);
            star_ti = ti;
            pi += 1;
        } else if let Some(sp) = star_pi {
            // Backtrack: the previous star should match one more character.
            pi = sp + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }

    // Consume trailing stars in the pattern.
    while pi < pattern.len() && pattern[pi] == b'*' {
        pi += 1;
    }

    pi == pattern.len()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{SessionInfo, TokenUsage};
    use std::path::PathBuf;

    // -- glob_match unit tests -------------------------------------------

    #[test]
    fn test_glob_exact() {
        assert!(glob_match("hello", "hello"));
        assert!(!glob_match("hello", "world"));
    }

    #[test]
    fn test_glob_star() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*", ""));
        assert!(glob_match("hello*", "hello"));
        assert!(glob_match("hello*", "hello world"));
        assert!(glob_match("*world", "hello world"));
        assert!(glob_match("*llo*", "hello world"));
    }

    #[test]
    fn test_glob_question_mark() {
        assert!(glob_match("h?llo", "hello"));
        assert!(!glob_match("h?llo", "hllo"));
        assert!(glob_match("???", "abc"));
        assert!(!glob_match("???", "ab"));
    }

    #[test]
    fn test_glob_combined() {
        assert!(glob_match("*/work/*", "/Users/foo/work/project"));
        assert!(!glob_match("*/work/*", "/Users/foo/personal/project"));
        assert!(glob_match("*/Code/*", "/Users/marcos/Code/nexus"));
        assert!(glob_match("/home/*/projects/*", "/home/user/projects/myapp"));
    }

    #[test]
    fn test_glob_empty() {
        assert!(glob_match("", ""));
        assert!(!glob_match("", "notempty"));
        assert!(glob_match("*", ""));
    }

    #[test]
    fn test_glob_multiple_stars() {
        assert!(glob_match("*/*", "a/b"));
        assert!(glob_match("**", "anything"));
        assert!(glob_match("a*b*c", "aXbYc"));
        assert!(glob_match("a*b*c", "abc"));
        assert!(!glob_match("a*b*c", "aXbY"));
    }

    // -- apply_rules integration tests -----------------------------------

    fn make_session(id: &str, cwd: &str) -> SessionInfo {
        SessionInfo {
            session_id: id.to_string(),
            slug: Some(format!("slug-{id}")),
            cwd: Some(PathBuf::from(cwd)),
            project_dir: "test".to_string(),
            git_branch: Some("main".to_string()),
            model: None,
            version: None,
            first_message: None,
            message_count: 0,
            token_usage: TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
            },
            subagent_count: 0,
            last_active: "2026-02-28T10:00:00Z".to_string(),
            source_file: PathBuf::from("/tmp/test.jsonl"),
            is_complete: true,
        }
    }

    #[test]
    fn test_apply_rules_basic() {
        let db = Database::open_in_memory().unwrap();

        db.upsert_sessions(&[
            make_session("s1", "/Users/user/work/project-a"),
            make_session("s2", "/Users/user/personal/notes"),
            make_session("s3", "/Users/user/work/project-b"),
        ])
        .unwrap();

        let rules = vec![
            AutoGroupRule {
                pattern: "*/work/*".to_string(),
                group: "Work".to_string(),
            },
            AutoGroupRule {
                pattern: "*/personal/*".to_string(),
                group: "Personal".to_string(),
            },
        ];

        let count = apply_rules(&rules, &db).unwrap();
        assert_eq!(count, 3);

        let ungrouped = db.get_ungrouped_sessions().unwrap();
        assert!(ungrouped.is_empty());

        // Verify groups were created
        assert!(db.get_group_id_by_name("Work").unwrap().is_some());
        assert!(db.get_group_id_by_name("Personal").unwrap().is_some());
    }

    #[test]
    fn test_apply_rules_no_match() {
        let db = Database::open_in_memory().unwrap();

        db.upsert_sessions(&[make_session("s1", "/opt/random/path")])
            .unwrap();

        let rules = vec![AutoGroupRule {
            pattern: "*/work/*".to_string(),
            group: "Work".to_string(),
        }];

        let count = apply_rules(&rules, &db).unwrap();
        assert_eq!(count, 0);

        let ungrouped = db.get_ungrouped_sessions().unwrap();
        assert_eq!(ungrouped.len(), 1);
    }

    #[test]
    fn test_apply_rules_empty_rules() {
        let db = Database::open_in_memory().unwrap();
        db.upsert_sessions(&[make_session("s1", "/Users/test")])
            .unwrap();

        let count = apply_rules(&[], &db).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_apply_rules_first_match_wins() {
        let db = Database::open_in_memory().unwrap();

        db.upsert_sessions(&[make_session("s1", "/Users/user/work/project")])
            .unwrap();

        let rules = vec![
            AutoGroupRule {
                pattern: "*".to_string(),
                group: "CatchAll".to_string(),
            },
            AutoGroupRule {
                pattern: "*/work/*".to_string(),
                group: "Work".to_string(),
            },
        ];

        apply_rules(&rules, &db).unwrap();

        // Should be assigned to CatchAll (first match), not Work
        let ungrouped = db.get_ungrouped_sessions().unwrap();
        assert!(ungrouped.is_empty());

        let catch_all_id = db.get_group_id_by_name("CatchAll").unwrap().unwrap();
        let tree = db.get_tree().unwrap();

        // CatchAll group should contain s1
        let catch_all = tree.iter().find(|n| match n {
            crate::types::TreeNode::Group(g) => g.id == catch_all_id,
            _ => false,
        });
        assert!(catch_all.is_some());
    }

    #[test]
    fn test_apply_rules_reuses_existing_group() {
        let db = Database::open_in_memory().unwrap();

        let gid = db.create_group("Work", "briefcase").unwrap();
        db.upsert_sessions(&[make_session("s1", "/Users/user/work/project")])
            .unwrap();

        let rules = vec![AutoGroupRule {
            pattern: "*/work/*".to_string(),
            group: "Work".to_string(),
        }];

        apply_rules(&rules, &db).unwrap();

        // Should be assigned to the existing group, not a new one
        let found_id = db.get_group_id_by_name("Work").unwrap().unwrap();
        assert_eq!(found_id, gid);
    }
}
