use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use chrono::{DateTime, FixedOffset};
use serde_json::Value;

/// Maximum number of characters for session titles
pub const TITLE_MAX_CHARS: usize = 80;

/// Read the first `head_n` lines and last `tail_n` lines from a file efficiently.
/// For small files (< 32 KB), reads all lines once to avoid unnecessary seeking.
/// For large files, seeks to last ~32 KB and finds line boundaries properly.
pub fn read_head_tail_lines(
    path: &Path,
    head_n: usize,
    tail_n: usize,
) -> io::Result<(Vec<String>, Vec<String>)> {
    use tracing::{debug, warn};
    debug!("[read_head_tail_lines] 开始: path={}, head_n={}, tail_n={}", path.display(), head_n, tail_n);

    let file = File::open(path)?;
    let file_len = file.metadata()?.len();
    debug!("[read_head_tail_lines] 文件大小: {} 字节", file_len);

    // Threshold for "small file" - increased to 32KB for better safety
    const SMALL_FILE_THRESHOLD: u64 = 32_768;

    // For small files, read all lines once and split
    if file_len < SMALL_FILE_THRESHOLD {
        debug!("[read_head_tail_lines] 小文件 (<32KB)，读取全部行");
        let reader = BufReader::new(file);
        let all: Vec<String> = reader.lines().map_while(Result::ok).collect();
        debug!("[read_head_tail_lines] 总行数: {}", all.len());
        let head: Vec<String> = all.iter().take(head_n).cloned().collect();
        let skip = all.len().saturating_sub(tail_n);
        let tail: Vec<String> = all.into_iter().skip(skip).collect();
        debug!("[read_head_tail_lines] 返回: head={}, tail={}", head.len(), tail.len());
        return Ok((head, tail));
    }

    // Read head lines from the beginning
    debug!("[read_head_tail_lines] 大文件，分步读取");
    let reader = BufReader::new(file);
    let head: Vec<String> = reader.lines().take(head_n).map_while(Result::ok).collect();
    debug!("[read_head_tail_lines] head 行数: {}", head.len());

    // Seek to last ~32 KB for tail lines
    let tail_chunk_size = 32_768u64;
    let seek_pos = file_len.saturating_sub(tail_chunk_size);
    debug!("[read_head_tail_lines] seek 位置: {} (倒数 32KB)", seek_pos);

    // Open file again and seek
    let mut file2 = File::open(path)?;
    file2.seek(SeekFrom::Start(seek_pos))?;

    // Read raw bytes and find first newline to avoid UTF-8 boundary issues
    let mut buf = vec![0u8; tail_chunk_size as usize];
    let bytes_read = file2.read(&mut buf)?;
    debug!("[read_head_tail_lines] 读取字节: {}", bytes_read);

    // Find the first newline (0x0A) to skip partial first line
    let newline_pos = buf.iter().position(|&b| b == 0x0A);
    let start_pos = if seek_pos > 0 {
        newline_pos.map(|p| p + 1).unwrap_or(0)
    } else {
        0
    };
    debug!("[read_head_tail_lines] 换行符位置: {:?}, start_pos: {}", newline_pos, start_pos);

    // Convert bytes after first newline to string and split into lines
    let tail_bytes = &buf[start_pos..bytes_read];
    let tail_str = String::from_utf8_lossy(tail_bytes);
    let all_tail: Vec<String> = tail_str.lines().map(|s| s.to_string()).collect();
    debug!("[read_head_tail_lines] 解析行数: {}", all_tail.len());

    // Take only the last tail_n lines
    let skip = all_tail.len().saturating_sub(tail_n);
    let tail: Vec<String> = all_tail.into_iter().skip(skip).collect();
    debug!("[read_head_tail_lines] 最终 tail 行数: {}", tail.len());

    Ok((head, tail))
}

/// Parse timestamp to milliseconds - handles integer (ms or s) and RFC3339 string
pub fn parse_timestamp_to_ms(value: &Value) -> Option<i64> {
    // Integer: milliseconds (>1e12) or seconds
    if let Some(n) = value.as_i64() {
        return Some(if n > 1_000_000_000_000 { n } else { n * 1000 });
    }
    if let Some(n) = value.as_f64() {
        let n = n as i64;
        return Some(if n > 1_000_000_000_000 { n } else { n * 1000 });
    }
    // RFC3339 string
    let raw = value.as_str()?;
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt: DateTime<FixedOffset>| dt.timestamp_millis())
}

/// Extract text from message content (handles string and array formats)
pub fn extract_text(content: &Value) -> String {
    match content {
        Value::String(text) => text.to_string(),
        Value::Array(items) => items
            .iter()
            .filter_map(extract_text_from_item)
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(map) => map
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        _ => String::new(),
    }
}

fn extract_text_from_item(item: &Value) -> Option<String> {
    let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");

    // tool_use: show tool name and input parameters
    if item_type == "tool_use" {
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown");

        // Extract input parameters
        if let Some(input) = item.get("input") {
            let input_str = format_input(input);
            if !input_str.is_empty() {
                return Some(format!("[Tool: {name}]\n入参: {input_str}"));
            }
        }
        return Some(format!("[Tool: {name}]"));
    }

    // tool_result: extract nested content
    if item_type == "tool_result" {
        if let Some(content) = item.get("content") {
            let text = extract_text(content);
            if !text.is_empty() {
                return Some(text);
            }
        }
        return None;
    }

    // text content
    if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
        return Some(text.to_string());
    }

    if let Some(content) = item.get("content") {
        let text = extract_text(content);
        if !text.is_empty() {
            return Some(text);
        }
    }

    None
}

/// Format tool input parameters for display
fn format_input(input: &Value) -> String {
    match input {
        Value::Object(map) => {
            // Format key-value pairs, truncate long values
            map.iter()
                .filter_map(|(k, v)| {
                    let value_str = format_value(v, 100);
                    if !value_str.is_empty() {
                        Some(format!("{k}: {value_str}"))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        }
        Value::String(s) => truncate_summary(s, 200),
        Value::Array(arr) => {
            if arr.len() > 3 {
                format!("{} items", arr.len())
            } else {
                arr.iter()
                    .filter_map(|v| Some(format_value(v, 50)))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        }
        _ => String::new(),
    }
}

/// Format a single value with max length
fn format_value(v: &Value, max_len: usize) -> String {
    match v {
        Value::String(s) => truncate_summary(s, max_len),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => {
            if arr.len() == 0 {
                "[]".to_string()
            } else if arr.len() <= 2 {
                arr.iter()
                    .map(|i| format_value(i, max_len / 2))
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                format!("[{}]", arr.len())
            }
        }
        Value::Object(_) => "{...}".to_string(),
        Value::Null => "null".to_string(),
    }
}

/// Truncate text to max characters with "..." suffix
pub fn truncate_summary(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    let mut result = trimmed.chars().take(max_chars).collect::<String>();
    result.push_str("...");
    result
}

/// Get the last segment of a path
pub fn path_basename(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = trimmed.trim_end_matches(['/', '\\']);
    let last = normalized
        .split(['/', '\\'])
        .next_back()
        .filter(|segment| !segment.is_empty())?;
    Some(last.to_string())
}