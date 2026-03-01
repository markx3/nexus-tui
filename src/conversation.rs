use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::types::{ConversationTurn, Role};

/// Parse a Claude Code JSONL session file for human and assistant turns.
///
/// Returns at most `max_turns` entries from the end of the conversation.
/// Only includes `type: "human"` and `type: "assistant"` entries.
pub fn parse_conversation(jsonl_path: &Path, max_turns: usize) -> Vec<ConversationTurn> {
    let file = match File::open(jsonl_path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut turns = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.trim().is_empty() {
            continue;
        }

        let value: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let entry_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");

        let role = match entry_type {
            "human" => Role::Human,
            "assistant" => Role::Assistant,
            _ => continue,
        };

        // Extract text content from the message
        let content = extract_content(&value);
        if content.is_empty() {
            continue;
        }

        turns.push(ConversationTurn { role, content });
    }

    // Return only the last max_turns
    if turns.len() > max_turns {
        turns.split_off(turns.len() - max_turns)
    } else {
        turns
    }
}

/// Extract text content from a JSONL entry.
///
/// Handles both simple `message.content` strings and array-of-blocks format
/// used by Claude Code sessions.
fn extract_content(value: &serde_json::Value) -> String {
    // Try message.content first (common format)
    if let Some(message) = value.get("message") {
        if let Some(content) = message.get("content") {
            return content_to_string(content);
        }
    }

    // Try top-level content
    if let Some(content) = value.get("content") {
        return content_to_string(content);
    }

    String::new()
}

/// Convert a content value (string or array of blocks) to a plain string.
fn content_to_string(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(blocks) => {
            let mut parts = Vec::new();
            for block in blocks {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    parts.push(text);
                }
            }
            parts.join("\n")
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_jsonl(lines: &[&str]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(file, "{}", line).unwrap();
        }
        file.flush().unwrap();
        file
    }

    #[test]
    fn parse_empty_file() {
        let file = write_jsonl(&[]);
        let turns = parse_conversation(file.path(), 100);
        assert!(turns.is_empty());
    }

    #[test]
    fn parse_human_and_assistant_turns() {
        let file = write_jsonl(&[
            r#"{"type":"human","message":{"content":"Hello"}}"#,
            r#"{"type":"assistant","message":{"content":"Hi there!"}}"#,
        ]);
        let turns = parse_conversation(file.path(), 100);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].role, Role::Human);
        assert_eq!(turns[0].content, "Hello");
        assert_eq!(turns[1].role, Role::Assistant);
        assert_eq!(turns[1].content, "Hi there!");
    }

    #[test]
    fn skips_non_conversation_types() {
        let file = write_jsonl(&[
            r#"{"type":"human","message":{"content":"Hello"}}"#,
            r#"{"type":"tool_use","name":"Read","input":{}}"#,
            r#"{"type":"tool_result","content":"file contents"}"#,
            r#"{"type":"assistant","message":{"content":"Done!"}}"#,
        ]);
        let turns = parse_conversation(file.path(), 100);
        assert_eq!(turns.len(), 2);
    }

    #[test]
    fn respects_max_turns() {
        let file = write_jsonl(&[
            r#"{"type":"human","message":{"content":"First"}}"#,
            r#"{"type":"assistant","message":{"content":"Response 1"}}"#,
            r#"{"type":"human","message":{"content":"Second"}}"#,
            r#"{"type":"assistant","message":{"content":"Response 2"}}"#,
        ]);
        let turns = parse_conversation(file.path(), 2);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].content, "Second");
        assert_eq!(turns[1].content, "Response 2");
    }

    #[test]
    fn handles_array_content_blocks() {
        let file = write_jsonl(&[
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Part 1"},{"type":"text","text":"Part 2"}]}}"#,
        ]);
        let turns = parse_conversation(file.path(), 100);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].content, "Part 1\nPart 2");
    }

    #[test]
    fn handles_nonexistent_file() {
        let turns = parse_conversation(Path::new("/nonexistent/path.jsonl"), 100);
        assert!(turns.is_empty());
    }

    #[test]
    fn skips_malformed_lines() {
        let file = write_jsonl(&[
            "not valid json",
            r#"{"type":"human","message":{"content":"Valid"}}"#,
            "{incomplete",
        ]);
        let turns = parse_conversation(file.path(), 100);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].content, "Valid");
    }
}
