use std::path::{Path, PathBuf};
use std::fs;
use std::io::{BufRead, BufReader};
use serde_json::Value;
use tracing::info;

use super::session_types::{SessionMeta, SessionMessage};
use super::session_utils::{
    read_head_tail_lines, parse_timestamp_to_ms, extract_text,
    truncate_summary, path_basename, TITLE_MAX_CHARS
};

const PROVIDER_ID: &str = "claude";

/// Get Claude data directory
pub fn get_claude_data_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Cannot get home directory")
        .join(".claude")
}

/// Get projects directory
pub fn get_projects_dir() -> PathBuf {
    get_claude_data_dir().join("projects")
}

/// Scan all sessions - optimized version reading only head/tail lines
pub fn scan_sessions() -> Vec<SessionMeta> {
    info!("[scan_sessions] Starting scan");
    let root = get_projects_dir();
    let mut files = Vec::new();
    collect_jsonl_files(&root, &mut files);

    info!("[scan_sessions] Found {} jsonl files", files.len());

    let mut sessions = Vec::new();
    for path in files {
        if let Some(meta) = parse_session(&path) {
            sessions.push(meta);
        }
    }

    // Sort by last_active_at descending
    sessions.sort_by(|a, b| {
        let a_ts = a.last_active_at.or(a.created_at).unwrap_or(0);
        let b_ts = b.last_active_at.or(b.created_at).unwrap_or(0);
        b_ts.cmp(&a_ts)
    });

    info!("[scan_sessions] Completed with {} sessions", sessions.len());
    sessions
}

/// Load messages from a session file
pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    info!("[load_messages] Loading from {}", path.display());
    let file = fs::File::open(path)
        .map_err(|e| format!("Failed to open file: {e}"))?;
    let reader = BufReader::new(file);
    let mut messages = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|e| format!("Read error: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }

        let value: Value = serde_json::from_str(&line)
            .map_err(|e| format!("Parse error: {e}"))?;

        // Skip meta entries
        if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
            continue;
        }

        let message = value.get("message");
        if let Some(msg) = message {
            let mut role = msg.get("role")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();

            // Reclassify pure tool_result messages as "tool"
            if role == "user" {
                if let Some(Value::Array(items)) = msg.get("content") {
                    let all_tool_results = !items.is_empty()
                        && items.iter().all(|item| {
                            item.get("type").and_then(Value::as_str) == Some("tool_result")
                        });
                    if all_tool_results {
                        role = "tool".to_string();
                    }
                }
            }

            let content = msg.get("content").map(extract_text).unwrap_or_default();
            if content.trim().is_empty() {
                continue;
            }

            let ts = value.get("timestamp").and_then(parse_timestamp_to_ms);

            messages.push(SessionMessage { role, content, ts });
        }
    }

    info!("[load_messages] Loaded {} messages", messages.len());
    Ok(messages)
}

/// Parse session metadata from file - reads only head 10 + tail 30 lines
fn parse_session(path: &Path) -> Option<SessionMeta> {
    // Skip agent sessions
    if is_agent_session(path) {
        return None;
    }

    let (head, tail) = read_head_tail_lines(path, 10, 30).ok()?;

    let mut session_id: Option<String> = None;
    let mut project_dir: Option<String> = None;
    let mut created_at: Option<i64> = None;
    let mut first_user_message: Option<String> = None;

    // Extract metadata from head lines
    for line in &head {
        let value: Value = serde_json::from_str(line).ok()?;

        if session_id.is_none() {
            session_id = value.get("sessionId")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
        }
        if project_dir.is_none() {
            project_dir = value.get("cwd")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
        }
        if created_at.is_none() {
            created_at = value.get("timestamp").and_then(parse_timestamp_to_ms);
        }

        // Extract first user message as title
        if first_user_message.is_none() {
            let is_user = value.get("type").and_then(Value::as_str) == Some("user")
                || value.get("message")
                    .and_then(|m| m.get("role"))
                    .and_then(Value::as_str) == Some("user");

            if is_user {
                if let Some(message) = value.get("message") {
                    let text = message.get("content").map(extract_text).unwrap_or_default();
                    let trimmed = text.trim();
                    if !trimmed.is_empty()
                        && !trimmed.contains("<local-command-caveat>")
                        && !trimmed.starts_with("<command-name>")
                    {
                        first_user_message = Some(trimmed.to_string());
                    }
                }
            }
        }

        if session_id.is_some() && project_dir.is_some()
            && created_at.is_some() && first_user_message.is_some() {
            break;
        }
    }

    // Extract last_active_at, summary, custom_title from tail lines (reverse)
    let mut last_active_at: Option<i64> = None;
    let mut summary: Option<String> = None;
    let mut custom_title: Option<String> = None;

    for line in tail.iter().rev() {
        let value: Value = serde_json::from_str(line).ok()?;

        if last_active_at.is_none() {
            last_active_at = value.get("timestamp").and_then(parse_timestamp_to_ms);
        }

        // Custom title from special entry
        if custom_title.is_none()
            && value.get("type").and_then(Value::as_str) == Some("custom-title") {
            custom_title = value.get("customTitle")
                .and_then(Value::as_str)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
        }

        if summary.is_none() {
            if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            if let Some(message) = value.get("message") {
                let text = message.get("content").map(extract_text).unwrap_or_default();
                if !text.trim().is_empty() {
                    summary = Some(text);
                }
            }
        }

        if last_active_at.is_some() && summary.is_some() && custom_title.is_some() {
            break;
        }
    }

    let session_id = session_id.or_else(|| infer_session_id_from_filename(path))?;

    // Title priority: custom-title > first user message > directory basename
    let title = custom_title
        .map(|t| truncate_summary(&t, TITLE_MAX_CHARS))
        .or_else(|| first_user_message.map(|t| truncate_summary(&t, TITLE_MAX_CHARS)))
        .or_else(|| {
            project_dir.as_deref()
                .and_then(path_basename)
                .map(|v| v.to_string())
        });

    let summary = summary.map(|text| truncate_summary(&text, 160));

    Some(SessionMeta {
        provider_id: PROVIDER_ID.to_string(),
        session_id: session_id.clone(),
        title,
        summary,
        project_dir,
        created_at,
        last_active_at,
        source_path: Some(path.to_string_lossy().to_string()),
        resume_command: Some(format!("claude --resume {session_id} --permission-mode bypassPermissions")),
    })
}

fn is_agent_session(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with("agent-"))
        .unwrap_or(false)
}

fn infer_session_id_from_filename(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string())
}

fn collect_jsonl_files(root: &Path, files: &mut Vec<PathBuf>) {
    if !root.exists() {
        return;
    }
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }
}

/// Delete session file and its sidecar directory
pub fn delete_session(session_id: &str) -> Result<bool, String> {
    info!("[delete_session] Deleting {}", session_id);

    let sessions = scan_sessions();
    let session = sessions.iter()
        .find(|s| s.session_id == session_id)
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    let path = session.source_path.as_ref()
        .ok_or_else(|| format!("Session {} has no source path", session_id))?;

    let path = Path::new(path);

    // Delete main file
    std::fs::remove_file(path)
        .map_err(|e| format!("Failed to delete file: {e}"))?;

    // Delete sidecar directory if exists
    if let Some(stem) = path.file_stem() {
        let sibling = path.parent()
            .unwrap_or_else(|| Path::new(""))
            .join(stem);
        if sibling.exists() {
            std::fs::remove_dir_all(&sibling)
                .map_err(|e| format!("Failed to delete sidecar: {e}"))?;
        }
    }

    info!("[delete_session] Deleted {}", session_id);
    Ok(true)
}

/// Find session file by session_id and load messages
pub fn get_session_messages(session_id: &str) -> Result<Vec<SessionMessage>, String> {
    info!("[get_session_messages] Loading messages for {}", session_id);

    let sessions = scan_sessions();
    let session = sessions.iter()
        .find(|s| s.session_id == session_id)
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    let path = session.source_path.as_ref()
        .ok_or_else(|| format!("Session {} has no source path", session_id))?;

    let messages = load_messages(Path::new(path))?;
    info!("[get_session_messages] Loaded {} messages", messages.len());
    Ok(messages)
}