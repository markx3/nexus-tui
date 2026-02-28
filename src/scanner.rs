use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use color_eyre::Result;
use serde_json::Value;

const QUICK_SCAN_MAX_LINES: usize = 50;
const FIRST_MESSAGE_MAX_LEN: usize = 200;

#[derive(Debug, Default)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug)]
pub struct ScanResult {
    pub sessions: Vec<SessionInfo>,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct SessionInfo {
    pub session_id: String,
    pub slug: Option<String>,
    pub cwd: Option<PathBuf>,
    pub project_dir: String,
    pub git_branch: Option<String>,
    pub model: Option<String>,
    pub version: Option<String>,
    pub first_message: Option<String>,
    pub message_count: u32,
    pub token_usage: TokenUsage,
    pub subagent_count: u16,
    pub last_active: String,
    pub source_file: PathBuf,
    pub is_complete: bool,
}

pub fn scan_quick(projects_dir: &Path) -> Result<ScanResult> {
    scan(projects_dir, ScanMode::Quick)
}

pub fn scan_full(projects_dir: &Path) -> Result<ScanResult> {
    scan(projects_dir, ScanMode::Full)
}

#[derive(Clone, Copy, PartialEq)]
enum ScanMode {
    Quick,
    Full,
}

struct SessionBuilder {
    session_id: String,
    slug: Option<String>,
    cwd: Option<PathBuf>,
    git_branch: Option<String>,
    model: Option<String>,
    version: Option<String>,
    first_message: Option<String>,
    message_count: u32,
    token_usage: TokenUsage,
    last_timestamp: Option<String>,
    found_first_message: bool,
}

impl SessionBuilder {
    fn new(session_id: String) -> Self {
        Self {
            session_id,
            slug: None,
            cwd: None,
            git_branch: None,
            model: None,
            version: None,
            first_message: None,
            message_count: 0,
            token_usage: TokenUsage::default(),
            last_timestamp: None,
            found_first_message: false,
        }
    }

    /// Process a single JSONL entry, accumulating fields into the builder.
    ///
    /// Field update semantics:
    /// - "first wins": cwd, slug, model, version (keep earliest value)
    /// - "last wins": git_branch, last_timestamp (always update to latest)
    fn process_entry(&mut self, entry: &Value) {
        let entry_type = entry["type"].as_str().unwrap_or("");

        // Extract timestamp from any entry that has one
        if let Some(ts) = entry["timestamp"].as_str() {
            self.last_timestamp = Some(ts.to_string());
        }

        // Extract slug from any entry that has it (appears on assistant + progress entries)
        if self.slug.is_none() {
            if let Some(slug) = entry["slug"].as_str() {
                self.slug = Some(slug.to_string());
            }
        }

        match entry_type {
            "user" | "progress" => {
                // Common metadata from user/progress entries
                if self.cwd.is_none() {
                    if let Some(cwd) = entry["cwd"].as_str() {
                        self.cwd = Some(PathBuf::from(cwd));
                    }
                }
                // Always update — branch may change mid-session ("last wins")
                if let Some(branch) = entry["gitBranch"].as_str() {
                    self.git_branch = Some(branch.to_string());
                }
                if self.version.is_none() {
                    if let Some(v) = entry["version"].as_str() {
                        self.version = Some(v.to_string());
                    }
                }

                // User-specific: first message and message count
                if entry_type == "user" {
                    let is_meta = entry["isMeta"].as_bool().unwrap_or(false);
                    if !is_meta {
                        self.message_count += 1;

                        if !self.found_first_message {
                            self.try_extract_first_message(entry);
                        }
                    }
                }
            }
            "assistant" => {
                if let Some(model) = entry["message"]["model"].as_str() {
                    if model != "<synthetic>" {
                        if self.model.is_none() {
                            self.model = Some(model.to_string());
                        }
                        self.message_count += 1;
                    }
                }

                // Token usage
                let usage = &entry["message"]["usage"];
                if let Some(input) = usage["input_tokens"].as_u64() {
                    self.token_usage.input_tokens += input;
                }
                if let Some(output) = usage["output_tokens"].as_u64() {
                    self.token_usage.output_tokens += output;
                }
            }
            // queue-operation, system: only timestamp (already handled above)
            // file-history-snapshot, pr-link, unknown: skip
            _ => {}
        }
    }

    fn try_extract_first_message(&mut self, entry: &Value) {
        let content = &entry["message"]["content"];

        match content {
            Value::String(s) => {
                // Skip command-message invocations
                if s.contains("<command-message>") {
                    return;
                }
                let preview = crate::text_utils::truncate(s, FIRST_MESSAGE_MAX_LEN);
                self.first_message = Some(preview);
                self.found_first_message = true;
            }
            Value::Array(arr) => {
                // Look for the first text-type item
                for item in arr {
                    if item["type"].as_str() == Some("text") {
                        if let Some(text) = item["text"].as_str() {
                            if text.contains("<command-message>") {
                                return;
                            }
                            let preview = crate::text_utils::truncate(text, FIRST_MESSAGE_MAX_LEN);
                            self.first_message = Some(preview);
                            self.found_first_message = true;
                            return;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn has_meaningful_data(&self) -> bool {
        self.cwd.is_some() || self.slug.is_some() || self.first_message.is_some()
    }

    fn has_all_quick_fields(&self) -> bool {
        self.cwd.is_some()
            && self.slug.is_some()
            && self.model.is_some()
            && self.version.is_some()
            && self.found_first_message
    }

    fn build(
        self,
        project_dir: String,
        subagent_count: u16,
        fallback_timestamp: String,
        source_file: PathBuf,
        is_complete: bool,
    ) -> SessionInfo {
        SessionInfo {
            session_id: self.session_id,
            slug: self.slug,
            cwd: self.cwd,
            project_dir,
            git_branch: self.git_branch,
            model: self.model,
            version: self.version,
            first_message: self.first_message,
            message_count: self.message_count,
            token_usage: self.token_usage,
            subagent_count,
            last_active: self.last_timestamp.unwrap_or(fallback_timestamp),
            source_file,
            is_complete,
        }
    }
}

fn scan(projects_dir: &Path, mode: ScanMode) -> Result<ScanResult> {
    if !projects_dir.exists() {
        return Ok(ScanResult { sessions: vec![], warnings: vec![] });
    }

    let mut sessions = Vec::new();
    let mut warnings = Vec::new();
    let is_complete = mode == ScanMode::Full;

    let project_entries = match fs::read_dir(projects_dir) {
        Ok(entries) => entries,
        Err(e) => {
            warnings.push(format!("cannot read {}: {e}", projects_dir.display()));
            return Ok(ScanResult { sessions, warnings });
        }
    };

    for project_entry in project_entries.filter_map(|e| e.ok()) {
        let project_path = project_entry.path();
        if !project_path.is_dir() {
            continue;
        }
        // Reject symlinked project directories
        if fs::symlink_metadata(&project_path)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
        {
            continue;
        }

        let project_dir = project_entry
            .file_name()
            .to_string_lossy()
            .into_owned();

        let jsonl_files = match fs::read_dir(&project_path) {
            Ok(entries) => entries,
            Err(e) => {
                warnings.push(format!("cannot read {}: {e}", project_path.display()));
                continue;
            }
        };

        for file_entry in jsonl_files.filter_map(|e| e.ok()) {
            let file_path = file_entry.path();

            // Only top-level .jsonl files (not inside subdirectories)
            if !file_path.is_file() {
                continue;
            }
            // Reject symlinked session files
            if fs::symlink_metadata(&file_path)
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false)
            {
                continue;
            }
            let ext = file_path.extension().and_then(|e| e.to_str());
            if ext != Some("jsonl") {
                continue;
            }

            let session_id = file_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();

            match parse_session_file(&file_path, &session_id, mode, &mut warnings) {
                Ok(Some(builder)) => {
                    let subagent_count = count_subagents(&project_path, &session_id);
                    let fallback_ts = file_mtime_iso(&file_path);
                    sessions.push(builder.build(
                        project_dir.clone(),
                        subagent_count,
                        fallback_ts,
                        file_path,
                        is_complete,
                    ));
                }
                Ok(None) => {} // Skipped (snapshot-only or empty)
                Err(e) => {
                    warnings.push(format!(
                        "failed to parse {}: {e}",
                        file_path.display()
                    ));
                }
            }
        }
    }

    sessions.sort_unstable_by(|a, b| b.last_active.cmp(&a.last_active));
    Ok(ScanResult { sessions, warnings })
}

fn parse_session_file(
    path: &Path,
    session_id: &str,
    mode: ScanMode,
    warnings: &mut Vec<String>,
) -> Result<Option<SessionBuilder>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::with_capacity(256 * 1024, file);
    let mut builder = SessionBuilder::new(session_id.to_string());
    let mut had_parse_error = false;

    for (i, line_result) in reader.lines().enumerate() {
        if mode == ScanMode::Quick
            && (i >= QUICK_SCAN_MAX_LINES || builder.has_all_quick_fields())
        {
            break;
        }

        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.is_empty() {
            continue;
        }

        // Cheap pre-filter: skip file-history-snapshot lines without parsing
        if line.contains("\"file-history-snapshot\"") {
            continue;
        }

        match serde_json::from_str::<Value>(&line) {
            Ok(entry) => builder.process_entry(&entry),
            Err(_) => {
                if !had_parse_error {
                    warnings.push(format!(
                        "malformed JSON in {} line {}",
                        path.display(),
                        i + 1
                    ));
                    had_parse_error = true;
                }
            }
        }
    }

    if !builder.has_meaningful_data() {
        return Ok(None);
    }

    Ok(Some(builder))
}

fn count_subagents(project_path: &Path, session_id: &str) -> u16 {
    let subagents_dir = project_path.join(session_id).join("subagents");
    if !subagents_dir.is_dir() {
        return 0;
    }

    fs::read_dir(&subagents_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        == Some("jsonl")
                })
                .count().min(u16::MAX as usize) as u16
        })
        .unwrap_or(0)
}

fn file_mtime_iso(path: &Path) -> String {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| {
            let secs = t.duration_since(SystemTime::UNIX_EPOCH).ok()?.as_secs();
            Some(crate::time_utils::epoch_to_iso(secs))
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    fn create_fixture_dir(name: &str) -> PathBuf {
        let dir = fixtures_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_jsonl(dir: &Path, filename: &str, lines: &[&str]) {
        let content = lines.join("\n");
        fs::write(dir.join(filename), content).unwrap();
    }

    #[test]
    fn test_normal_session() {
        let projects = create_fixture_dir("test_normal");
        let project = projects.join("-Users-test-Code-myproject");
        fs::create_dir_all(&project).unwrap();

        write_jsonl(
            &project,
            "aaa-bbb-ccc.jsonl",
            &[
                r#"{"type":"progress","sessionId":"aaa-bbb-ccc","cwd":"/Users/test/Code/myproject","gitBranch":"main","version":"2.1.50","timestamp":"2026-02-28T10:00:00Z"}"#,
                r#"{"type":"user","sessionId":"aaa-bbb-ccc","cwd":"/Users/test/Code/myproject","gitBranch":"main","version":"2.1.50","message":{"role":"user","content":"Hello, help me with this code"},"timestamp":"2026-02-28T10:00:01Z"}"#,
                r#"{"type":"assistant","sessionId":"aaa-bbb-ccc","slug":"happy-coding-fox","message":{"model":"claude-opus-4-6","role":"assistant","content":[{"type":"text","text":"Sure!"}],"usage":{"input_tokens":100,"output_tokens":50}},"timestamp":"2026-02-28T10:00:02Z"}"#,
            ],
        );

        let result = scan_full(&projects).unwrap();
        assert!(result.warnings.is_empty());
        let results = result.sessions;
        assert_eq!(results.len(), 1);

        let s = &results[0];
        assert_eq!(s.session_id, "aaa-bbb-ccc");
        assert_eq!(s.slug.as_deref(), Some("happy-coding-fox"));
        assert_eq!(s.cwd.as_deref(), Some(Path::new("/Users/test/Code/myproject")));
        assert_eq!(s.git_branch.as_deref(), Some("main"));
        assert_eq!(s.model.as_deref(), Some("claude-opus-4-6"));
        assert_eq!(s.version.as_deref(), Some("2.1.50"));
        assert_eq!(s.first_message.as_deref(), Some("Hello, help me with this code"));
        assert_eq!(s.message_count, 2); // 1 user + 1 assistant
        assert_eq!(s.token_usage.input_tokens, 100);
        assert_eq!(s.token_usage.output_tokens, 50);
        assert_eq!(s.last_active, "2026-02-28T10:00:02Z");
        assert_eq!(s.subagent_count, 0);
    }

    #[test]
    fn test_no_slug_session() {
        let projects = create_fixture_dir("test_no_slug");
        let project = projects.join("-Users-test-project");
        fs::create_dir_all(&project).unwrap();

        write_jsonl(
            &project,
            "ddd-eee-fff.jsonl",
            &[
                r#"{"type":"progress","sessionId":"ddd-eee-fff","cwd":"/Users/test/project","gitBranch":"dev","version":"2.1.50","timestamp":"2026-02-28T11:00:00Z"}"#,
                r#"{"type":"user","sessionId":"ddd-eee-fff","cwd":"/Users/test/project","message":{"role":"user","content":"Quick question"},"timestamp":"2026-02-28T11:00:01Z"}"#,
            ],
        );

        let results = scan_quick(&projects).unwrap().sessions;
        assert_eq!(results.len(), 1);
        assert!(results[0].slug.is_none());
        assert_eq!(results[0].first_message.as_deref(), Some("Quick question"));
    }

    #[test]
    fn test_is_meta_messages_skipped() {
        let projects = create_fixture_dir("test_is_meta");
        let project = projects.join("-Users-test-meta");
        fs::create_dir_all(&project).unwrap();

        write_jsonl(
            &project,
            "ggg-hhh-iii.jsonl",
            &[
                r#"{"type":"progress","sessionId":"ggg-hhh-iii","cwd":"/Users/test/meta","gitBranch":"main","version":"2.1.50","timestamp":"2026-02-28T12:00:00Z"}"#,
                r#"{"type":"user","sessionId":"ggg-hhh-iii","isMeta":true,"message":{"role":"user","content":"System injected prompt"},"timestamp":"2026-02-28T12:00:01Z"}"#,
                r#"{"type":"user","sessionId":"ggg-hhh-iii","message":{"role":"user","content":"Real user question"},"timestamp":"2026-02-28T12:00:02Z"}"#,
            ],
        );

        let results = scan_full(&projects).unwrap().sessions;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].first_message.as_deref(), Some("Real user question"));
        assert_eq!(results[0].message_count, 1); // Only the non-meta user message
    }

    #[test]
    fn test_command_message_skipped() {
        let projects = create_fixture_dir("test_command_msg");
        let project = projects.join("-Users-test-cmd");
        fs::create_dir_all(&project).unwrap();

        write_jsonl(
            &project,
            "jjj-kkk-lll.jsonl",
            &[
                r#"{"type":"progress","sessionId":"jjj-kkk-lll","cwd":"/Users/test/cmd","gitBranch":"main","version":"2.1.50","timestamp":"2026-02-28T13:00:00Z"}"#,
                r#"{"type":"user","sessionId":"jjj-kkk-lll","message":{"role":"user","content":"<command-message>some-skill</command-message>"},"timestamp":"2026-02-28T13:00:01Z"}"#,
                r#"{"type":"user","sessionId":"jjj-kkk-lll","message":{"role":"user","content":"Actual user question after command"},"timestamp":"2026-02-28T13:00:02Z"}"#,
            ],
        );

        let results = scan_full(&projects).unwrap().sessions;
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].first_message.as_deref(),
            Some("Actual user question after command")
        );
    }

    #[test]
    fn test_snapshot_only_session_skipped() {
        let projects = create_fixture_dir("test_snapshot");
        let project = projects.join("-Users-test-snap");
        fs::create_dir_all(&project).unwrap();

        write_jsonl(
            &project,
            "mmm-nnn-ooo.jsonl",
            &[
                r#"{"type":"file-history-snapshot","messageId":"abc","snapshot":{"trackedFileBackups":{},"timestamp":"2026-02-28T14:00:00Z"}}"#,
                r#"{"type":"file-history-snapshot","messageId":"def","snapshot":{"trackedFileBackups":{},"timestamp":"2026-02-28T14:00:01Z"}}"#,
            ],
        );

        let results = scan_full(&projects).unwrap().sessions;
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_empty_file() {
        let projects = create_fixture_dir("test_empty");
        let project = projects.join("-Users-test-empty");
        fs::create_dir_all(&project).unwrap();

        write_jsonl(&project, "ppp-qqq-rrr.jsonl", &[]);

        let results = scan_full(&projects).unwrap().sessions;
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_malformed_json_skipped() {
        let projects = create_fixture_dir("test_malformed");
        let project = projects.join("-Users-test-bad");
        fs::create_dir_all(&project).unwrap();

        write_jsonl(
            &project,
            "sss-ttt-uuu.jsonl",
            &[
                r#"{"type":"progress","sessionId":"sss-ttt-uuu","cwd":"/Users/test/bad","gitBranch":"main","version":"2.1.50","timestamp":"2026-02-28T15:00:00Z"}"#,
                r#"NOT VALID JSON AT ALL"#,
                r#"{"type":"user","sessionId":"sss-ttt-uuu","message":{"role":"user","content":"Still works"},"timestamp":"2026-02-28T15:00:02Z"}"#,
            ],
        );

        let results = scan_full(&projects).unwrap().sessions;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].first_message.as_deref(), Some("Still works"));
    }

    #[test]
    fn test_subagent_count() {
        let projects = create_fixture_dir("test_subagents");
        let project = projects.join("-Users-test-agents");
        fs::create_dir_all(&project).unwrap();

        let subagents_dir = project.join("vvv-www-xxx").join("subagents");
        fs::create_dir_all(&subagents_dir).unwrap();
        fs::write(subagents_dir.join("agent-a001.jsonl"), "{}").unwrap();
        fs::write(subagents_dir.join("agent-a002.jsonl"), "{}").unwrap();
        fs::write(subagents_dir.join("agent-a003.jsonl"), "{}").unwrap();

        write_jsonl(
            &project,
            "vvv-www-xxx.jsonl",
            &[
                r#"{"type":"progress","sessionId":"vvv-www-xxx","cwd":"/Users/test/agents","gitBranch":"main","version":"2.1.50","timestamp":"2026-02-28T16:00:00Z"}"#,
                r#"{"type":"user","sessionId":"vvv-www-xxx","message":{"role":"user","content":"Build something"},"timestamp":"2026-02-28T16:00:01Z"}"#,
            ],
        );

        let results = scan_full(&projects).unwrap().sessions;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].subagent_count, 3);
    }

    #[test]
    fn test_missing_projects_dir() {
        let results = scan_quick(Path::new("/nonexistent/path/that/doesnt/exist")).unwrap().sessions;
        assert!(results.is_empty());
    }

    #[test]
    fn test_quick_vs_full_token_usage() {
        let projects = create_fixture_dir("test_quick_full");
        let project = projects.join("-Users-test-quick");
        fs::create_dir_all(&project).unwrap();

        // Create a session with an assistant entry beyond line 50
        let mut lines: Vec<String> = Vec::new();
        lines.push(
            r#"{"type":"progress","sessionId":"yyy-zzz","cwd":"/Users/test/quick","gitBranch":"main","version":"2.1.50","timestamp":"2026-02-28T17:00:00Z"}"#.to_string(),
        );
        lines.push(
            r#"{"type":"user","sessionId":"yyy-zzz","message":{"role":"user","content":"Hello"},"timestamp":"2026-02-28T17:00:01Z"}"#.to_string(),
        );
        // Pad with system entries to push past QUICK_SCAN_MAX_LINES
        for i in 0..55 {
            lines.push(format!(
                r#"{{"type":"system","timestamp":"2026-02-28T17:00:{:02}Z"}}"#,
                (i % 60)
            ));
        }
        lines.push(
            r#"{"type":"assistant","sessionId":"yyy-zzz","slug":"test-slug","message":{"model":"claude-opus-4-6","role":"assistant","content":[{"type":"text","text":"response"}],"usage":{"input_tokens":500,"output_tokens":200}},"timestamp":"2026-02-28T17:01:00Z"}"#.to_string(),
        );

        let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        write_jsonl(&project, "yyy-zzz.jsonl", &line_refs);

        let quick = scan_quick(&projects).unwrap().sessions;
        let full = scan_full(&projects).unwrap().sessions;

        assert_eq!(quick.len(), 1);
        assert_eq!(full.len(), 1);

        // Quick scan stops at 50 lines — misses the assistant entry with tokens
        assert_eq!(quick[0].token_usage.output_tokens, 0);
        assert_eq!(full[0].token_usage.output_tokens, 200);
        assert_eq!(full[0].token_usage.input_tokens, 500);

        // Quick scan also misses slug (it was on the assistant entry past line 50)
        assert!(quick[0].slug.is_none());
        assert_eq!(full[0].slug.as_deref(), Some("test-slug"));

        // is_complete reflects scan mode
        assert!(!quick[0].is_complete);
        assert!(full[0].is_complete);
    }

    #[test]
    fn test_synthetic_model_not_counted() {
        let projects = create_fixture_dir("test_synthetic");
        let project = projects.join("-Users-test-syn");
        fs::create_dir_all(&project).unwrap();

        write_jsonl(
            &project,
            "syn-test-001.jsonl",
            &[
                r#"{"type":"progress","sessionId":"syn-test-001","cwd":"/Users/test/syn","gitBranch":"main","version":"2.1.50","timestamp":"2026-02-28T18:00:00Z"}"#,
                r#"{"type":"user","sessionId":"syn-test-001","message":{"role":"user","content":"Hello"},"timestamp":"2026-02-28T18:00:01Z"}"#,
                r#"{"type":"assistant","sessionId":"syn-test-001","message":{"model":"claude-opus-4-6","role":"assistant","content":[{"type":"text","text":"Hi"}],"usage":{"input_tokens":10,"output_tokens":5}},"timestamp":"2026-02-28T18:00:02Z"}"#,
                r#"{"type":"assistant","sessionId":"syn-test-001","message":{"model":"<synthetic>","role":"assistant","content":[{"type":"text","text":"..."}],"usage":{"input_tokens":0,"output_tokens":0}},"timestamp":"2026-02-28T18:00:03Z"}"#,
            ],
        );

        let results = scan_full(&projects).unwrap().sessions;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].model.as_deref(), Some("claude-opus-4-6"));
        assert_eq!(results[0].message_count, 2); // 1 user + 1 real assistant (synthetic excluded)
    }

    #[test]
    fn test_array_content_first_message() {
        let projects = create_fixture_dir("test_array_content");
        let project = projects.join("-Users-test-arr");
        fs::create_dir_all(&project).unwrap();

        write_jsonl(
            &project,
            "arr-test-001.jsonl",
            &[
                r#"{"type":"progress","sessionId":"arr-test-001","cwd":"/Users/test/arr","gitBranch":"main","version":"2.1.50","timestamp":"2026-02-28T19:00:00Z"}"#,
                r#"{"type":"user","sessionId":"arr-test-001","message":{"role":"user","content":[{"type":"text","text":"Fix the login bug"}]},"timestamp":"2026-02-28T19:00:01Z"}"#,
            ],
        );

        let results = scan_full(&projects).unwrap().sessions;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].first_message.as_deref(), Some("Fix the login bug"));
    }

    #[test]
    #[ignore] // Run with: cargo test -- --ignored
    fn test_real_session_files() {
        use std::time::Instant;

        let home = std::env::var("HOME").unwrap();
        let projects_dir = PathBuf::from(home).join(".claude/projects");

        let t0 = Instant::now();
        let quick = scan_quick(&projects_dir).unwrap().sessions;
        let quick_ms = t0.elapsed().as_millis();
        assert!(!quick.is_empty(), "Expected at least one session");

        for s in &quick {
            assert!(!s.session_id.is_empty());
            assert!(!s.project_dir.is_empty());
            assert!(s.source_file.exists());
        }

        let with_cwd: Vec<_> = quick.iter().filter(|s| s.cwd.is_some()).collect();
        assert!(!with_cwd.is_empty(), "Expected at least one session with cwd");

        let t1 = Instant::now();
        let full = scan_full(&projects_dir).unwrap().sessions;
        let full_ms = t1.elapsed().as_millis();
        assert!(full.len() >= quick.len());

        let with_tokens: Vec<_> = full
            .iter()
            .filter(|s| s.token_usage.output_tokens > 0)
            .collect();
        assert!(!with_tokens.is_empty(), "Expected at least one session with token data");

        eprintln!(
            "Real data: {} sessions | quick: {quick_ms}ms | full: {full_ms}ms | with tokens: {}",
            full.len(),
            with_tokens.len()
        );
        assert!(quick_ms < 500, "Quick scan took {quick_ms}ms (target: <500ms)");
    }

    #[test]
    fn test_git_branch_uses_last_value() {
        let projects = create_fixture_dir("test_branch_change");
        let project = projects.join("-Users-test-branch");
        fs::create_dir_all(&project).unwrap();

        write_jsonl(
            &project,
            "branch-001.jsonl",
            &[
                r#"{"type":"progress","sessionId":"branch-001","cwd":"/Users/test/branch","gitBranch":"main","version":"2.1.50","timestamp":"2026-02-28T20:00:00Z"}"#,
                r#"{"type":"user","sessionId":"branch-001","cwd":"/Users/test/branch","gitBranch":"feat/new-feature","message":{"role":"user","content":"Working on feature"},"timestamp":"2026-02-28T20:00:01Z"}"#,
            ],
        );

        let results = scan_full(&projects).unwrap().sessions;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].git_branch.as_deref(), Some("feat/new-feature"));
    }
}
