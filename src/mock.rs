use std::path::PathBuf;

use crate::types::*;

pub fn mock_tree() -> Vec<TreeNode> {
    vec![
        TreeNode::Group(GroupNode {
            id: 1,
            name: "nexus".to_string(),
            icon: GroupIcon::Root,
            collapsed: false,
            children: vec![
                TreeNode::Session(SessionSummary {
                    session_id: "a1b2c3d4-e5f6-7890-abcd-ef1234567890".to_string(),
                    display_name: "feat/scanner".to_string(),
                    cwd: Some(PathBuf::from("/Users/dev/Code/nexus")),
                    project_dir: "-Users-dev-Code-nexus".to_string(),
                    git_branch: Some("feat/scanner".to_string()),
                    model: Some("claude-opus-4-6".to_string()),
                    first_message: Some("Implement the session scanner module".to_string()),
                    message_count: 47,
                    input_tokens: 125_000,
                    output_tokens: 89_000,
                    subagent_count: 3,
                    last_active: "2026-02-28T15:30:00Z".to_string(),
                    is_active: true,
                }),
                TreeNode::Session(SessionSummary {
                    session_id: "b2c3d4e5-f6a7-8901-bcde-f12345678901".to_string(),
                    display_name: "fix/render-tick".to_string(),
                    cwd: Some(PathBuf::from("/Users/dev/Code/nexus")),
                    project_dir: "-Users-dev-Code-nexus".to_string(),
                    git_branch: Some("fix/render-tick".to_string()),
                    model: Some("claude-sonnet-4-6".to_string()),
                    first_message: Some("Fix the render tick delta timing".to_string()),
                    message_count: 12,
                    input_tokens: 34_000,
                    output_tokens: 21_000,
                    subagent_count: 0,
                    last_active: "2026-02-27T09:15:00Z".to_string(),
                    is_active: false,
                }),
            ],
        }),
        TreeNode::Group(GroupNode {
            id: 2,
            name: "website".to_string(),
            icon: GroupIcon::Root,
            collapsed: false,
            children: vec![
                TreeNode::Session(SessionSummary {
                    session_id: "c3d4e5f6-a7b8-9012-cdef-123456789012".to_string(),
                    display_name: "redesign-landing".to_string(),
                    cwd: Some(PathBuf::from("/Users/dev/Code/website")),
                    project_dir: "-Users-dev-Code-website".to_string(),
                    git_branch: Some("redesign/landing".to_string()),
                    model: Some("claude-opus-4-6".to_string()),
                    first_message: Some("Redesign the landing page with new brand".to_string()),
                    message_count: 83,
                    input_tokens: 450_000,
                    output_tokens: 320_000,
                    subagent_count: 7,
                    last_active: "2026-02-28T14:00:00Z".to_string(),
                    is_active: true,
                }),
                TreeNode::Group(GroupNode {
                    id: 3,
                    name: "api-work".to_string(),
                    icon: GroupIcon::SubGroup,
                    collapsed: true,
                    children: vec![TreeNode::Session(SessionSummary {
                        session_id: "d4e5f6a7-b8c9-0123-defa-234567890123".to_string(),
                        display_name: "api-auth-endpoints".to_string(),
                        cwd: Some(PathBuf::from("/Users/dev/Code/website")),
                        project_dir: "-Users-dev-Code-website".to_string(),
                        git_branch: Some("feat/api-auth".to_string()),
                        model: Some("claude-sonnet-4-6".to_string()),
                        first_message: Some("Add OAuth endpoints to the API".to_string()),
                        message_count: 31,
                        input_tokens: 78_000,
                        output_tokens: 55_000,
                        subagent_count: 2,
                        last_active: "2026-02-25T11:30:00Z".to_string(),
                        is_active: false,
                    })],
                }),
            ],
        }),
        TreeNode::Group(GroupNode {
            id: 4,
            name: "Ungrouped".to_string(),
            icon: GroupIcon::Root,
            collapsed: false,
            children: vec![SessionSummary {
                session_id: "e5f6a7b8-c9d0-1234-efab-345678901234".to_string(),
                display_name: "quick-question".to_string(),
                cwd: Some(PathBuf::from("/Users/dev")),
                project_dir: "-Users-dev".to_string(),
                git_branch: None,
                model: Some("claude-haiku-4-5-20251001".to_string()),
                first_message: Some("How do I configure rustfmt?".to_string()),
                message_count: 4,
                input_tokens: 8_000,
                output_tokens: 3_000,
                subagent_count: 0,
                last_active: "2026-02-20T08:00:00Z".to_string(),
                is_active: false,
            }]
            .into_iter()
            .map(TreeNode::Session)
            .collect(),
        }),
    ]
}

pub fn mock_selection() -> SelectionState {
    SelectionState {
        selected: Some(SelectionTarget::Session(
            "a1b2c3d4-e5f6-7890-abcd-ef1234567890".to_string(),
        )),
        focused_panel: FocusPanel::Tree,
    }
}

pub fn mock_tmux_windows() -> Vec<TmuxWindowInfo> {
    vec![
        TmuxWindowInfo {
            session_id: "a1b2c3d4-e5f6-7890-abcd-ef1234567890".to_string(),
            window_name: "nexus:feat-scanner".to_string(),
            is_active: true,
            status: TmuxSessionStatus::Running,
        },
        TmuxWindowInfo {
            session_id: "c3d4e5f6-a7b8-9012-cdef-123456789012".to_string(),
            window_name: "website:redesign".to_string(),
            is_active: false,
            status: TmuxSessionStatus::Idle,
        },
    ]
}
