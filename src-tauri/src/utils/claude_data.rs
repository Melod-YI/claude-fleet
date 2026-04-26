use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use std::io::{BufRead, BufReader};

/// Claude Session 数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeSession {
    pub id: String,
    pub name: String,
    pub working_directory: String,
    pub status: String,
    pub created_at: String,
    pub last_activity_at: String,
    pub conversation_count: u32,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_window_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
}

/// 对话消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

/// 对话结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub session_id: String,
    pub messages: Vec<ConversationMessage>,
    pub total_messages: u32,
}

/// 运行中 Session 元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningSessionMetadata {
    pub pid: u32,
    pub session_id: String,
    pub cwd: String,
    pub started_at: u64,
    pub kind: String,
    pub entrypoint: String,
}

/// JSONL 行类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum JsonlLine {
    #[serde(rename = "user")]
    User {
        message: MessageContent,
        uuid: String,
        timestamp: String,
        cwd: Option<String>,
        session_id: Option<String>,
    },
    #[serde(rename = "assistant")]
    Assistant {
        message: AssistantMessage,
        uuid: String,
        timestamp: String,
        cwd: Option<String>,
        session_id: Option<String>,
    },
    #[serde(rename = "permission-mode")]
    PermissionMode {
        permission_mode: String,
        session_id: String,
    },
    #[serde(rename = "file-history-snapshot")]
    FileHistorySnapshot {
        message_id: String,
        snapshot: serde_json::Value,
        is_snapshot_update: bool,
    },
    #[serde(other)]
    Other,
}

/// 消息内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    pub role: String,
    #[serde(default)]
    pub content: String,
}

/// 助手消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub role: String,
    #[serde(default)]
    pub content: Vec<serde_json::Value>,
    #[serde(default)]
    pub usage: UsageStats,
}

/// 使用统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
}

/// 获取 Claude 数据根目录
pub fn get_claude_data_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户目录")
        .join(".claude")
}

/// 获取项目目录列表
pub fn get_projects_dir() -> PathBuf {
    get_claude_data_dir().join("projects")
}

/// 获取运行中 sessions 目录
pub fn get_sessions_dir() -> PathBuf {
    get_claude_data_dir().join("sessions")
}

/// 解析项目目录名，还原为工作目录路径
pub fn decode_project_path(project_name: &str) -> String {
    // 例如: C--workspace-claude-fleet-sp -> C:\workspace\claude-fleet-sp
    let parts: Vec<&str> = project_name.split('-').collect();
    if parts.is_empty() {
        return project_name.to_string();
    }

    // 第一部分可能是盘符（如 C）
    let first = parts[0];
    let mut path = if first.len() == 1 && first.chars().next().unwrap().is_ascii_uppercase() {
        format!("{}:", first)
    } else {
        first.to_string()
    };

    // 添加其余部分
    for part in parts.iter().skip(1) {
        path.push_str("\\");
        path.push_str(part);
    }

    path
}

/// 获取运行中 session 的元数据
pub fn get_running_sessions() -> Result<Vec<RunningSessionMetadata>, String> {
    let sessions_dir = get_sessions_dir();

    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut running_sessions: Vec<RunningSessionMetadata> = Vec::new();

    for entry in fs::read_dir(&sessions_dir)
        .map_err(|e| format!("读取 sessions 目录失败: {}", e))?
    {
        let file_path = entry
            .map_err(|e| format!("读取条目失败: {}", e))?
            .path();

        if file_path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("读取文件 {} 失败: {}", file_path.display(), e))?;

        if let Ok(metadata) = serde_json::from_str::<RunningSessionMetadata>(&content) {
            running_sessions.push(metadata);
        }
    }

    Ok(running_sessions)
}

/// 解析单个 session JSONL 文件
pub fn parse_session_file(
    file_path: &PathBuf,
    project_path: &str,
) -> Result<(ClaudeSession, Conversation), String> {
    let file = fs::File::open(file_path)
        .map_err(|e| format!("无法打开文件 {}: {}", file_path.display(), e))?;

    let reader = BufReader::new(file);
    let mut messages: Vec<ConversationMessage> = Vec::new();
    let mut created_at = String::new();
    let mut last_activity_at = String::new();
    let mut working_directory = String::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| format!("读取行失败: {}", e))?;
        if line.trim().is_empty() {
            continue;
        }

        // 尝试解析 JSON
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&line) {
            let line_type = json_value
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // 提取时间戳
            let timestamp = json_value
                .get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // 提取工作目录
            if working_directory.is_empty() {
                working_directory = json_value
                    .get("cwd")
                    .and_then(|v| v.as_str())
                    .unwrap_or(project_path)
                    .to_string();
            }

            // 第一条消息的时间作为创建时间
            if index == 0 {
                created_at = timestamp.clone();
            }
            last_activity_at = timestamp.clone();

            // 提取消息内容
            if line_type == "user" {
                let message = json_value.get("message");
                if let Some(msg) = message {
                    let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
                    let content = extract_message_content(msg);

                    messages.push(ConversationMessage {
                        id: json_value
                            .get("uuid")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&format!("msg-{}", index))
                            .to_string(),
                        role: role.to_string(),
                        content,
                        timestamp,
                    });
                }
            } else if line_type == "assistant" {
                let message = json_value.get("message");
                if let Some(msg) = message {
                    let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("assistant");
                    let content = extract_assistant_content(msg);

                    messages.push(ConversationMessage {
                        id: json_value
                            .get("uuid")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&format!("msg-{}", index))
                            .to_string(),
                        role: role.to_string(),
                        content,
                        timestamp,
                    });
                }
            }
        }
    }

    // 从文件名提取 session ID
    let session_id = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // 使用工作目录名作为默认名称
    let name = if working_directory.is_empty() {
        session_id.clone()
    } else {
        get_last_path_segment(&working_directory)
    };

    let session = ClaudeSession {
        id: session_id.clone(),
        name,
        working_directory: if working_directory.is_empty() {
            project_path.to_string()
        } else {
            working_directory
        },
        status: "idle".to_string(),
        created_at,
        last_activity_at,
        conversation_count: messages.len() as u32,
        is_favorite: false,
        terminal_window_id: None,
        process_id: None,
    };

    let message_count = messages.len() as u32;

    let conversation = Conversation {
        session_id,
        messages,
        total_messages: message_count,
    };

    Ok((session, conversation))
}

/// 从消息对象中提取内容
fn extract_message_content(msg: &serde_json::Value) -> String {
    // 尝试直接获取 content 字符串
    if let Some(content) = msg.get("content").and_then(|v| v.as_str()) {
        return content.to_string();
    }

    // 尝试获取 content 数组
    if let Some(content_array) = msg.get("content").and_then(|v| v.as_array()) {
        let mut result = String::new();
        for item in content_array {
            if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                result.push_str(text);
                result.push_str("\n");
            }
        }
        return result.trim().to_string();
    }

    String::new()
}

/// 从助手消息中提取内容
fn extract_assistant_content(msg: &serde_json::Value) -> String {
    if let Some(content_array) = msg.get("content").and_then(|v| v.as_array()) {
        let mut result = String::new();
        for item in content_array {
            if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                result.push_str(text);
                result.push_str("\n");
            } else if let Some(thinking) = item.get("thinking").and_then(|v| v.as_str()) {
                result.push_str("[Thinking]\n");
                result.push_str(thinking);
                result.push_str("\n");
            }
        }
        return result.trim().to_string();
    }

    String::new()
}

/// 获取路径的最后一部分
fn get_last_path_segment(path: &str) -> String {
    let parts: Vec<&str> = path.split(|c| c == '\\' || c == '/').filter(|s| !s.is_empty()).collect();
    parts.last().unwrap_or(&path).to_string()
}

/// 获取所有 session 列表
pub fn get_all_sessions() -> Result<Vec<ClaudeSession>, String> {
    let projects_dir = get_projects_dir();

    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    // 获取运行中 session 的 PID 映射
    let running_sessions = get_running_sessions()?;
    let running_map: std::collections::HashMap<String, u32> = running_sessions
        .iter()
        .map(|s| (s.session_id.clone(), s.pid))
        .collect();

    let mut sessions: Vec<ClaudeSession> = Vec::new();

    // 遍历所有项目目录
    for entry in fs::read_dir(&projects_dir)
        .map_err(|e| format!("读取项目目录失败: {}", e))?
    {
        let project_dir = entry
            .map_err(|e| format!("读取条目失败: {}", e))?
            .path();

        let project_name = project_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let decoded_path = decode_project_path(project_name);

        // 遍历项目目录下的所有 jsonl 文件
        for project_entry in fs::read_dir(&project_dir)
            .map_err(|e| format!("读取项目 {} 目录失败: {}", project_dir.display(), e))?
        {
            let entry_path = project_entry
                .map_err(|e| format!("读取条目失败: {}", e))?
                .path();

            // 只处理 jsonl 文件
            if entry_path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                continue;
            }

            if let Ok((session, _)) = parse_session_file(&entry_path, &decoded_path) {
                // 检查是否正在运行
                let process_id = running_map.get(&session.id).copied();
                let status = if process_id.is_some() {
                    // 检查进程是否确实在运行
                    if is_process_running(process_id.unwrap()) {
                        "running".to_string()
                    } else {
                        "idle".to_string()
                    }
                } else {
                    "idle".to_string()
                };

                sessions.push(ClaudeSession {
                    status,
                    process_id,
                    ..session
                });
            }
        }
    }

    // 按最后活动时间排序（最近的在前）
    sessions.sort_by(|a, b| b.last_activity_at.cmp(&a.last_activity_at));

    Ok(sessions)
}

/// 检查进程是否正在运行
fn is_process_running(pid: u32) -> bool {
    // Windows: 使用 tasklist 检查进程
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let output = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains(&pid.to_string())
        } else {
            false
        }
    }

    // macOS/Linux: 使用 ps 检查进程
    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        let output = Command::new("ps")
            .args(["-p", &pid.to_string()])
            .output();

        if let Ok(output) = output {
            output.status.success()
        } else {
            false
        }
    }
}

/// 获取指定 session 的对话内容
pub fn get_session_conversation(session_id: &str) -> Result<Conversation, String> {
    let projects_dir = get_projects_dir();

    // 需要遍历找到对应 session 文件
    for entry in fs::read_dir(&projects_dir)
        .map_err(|e| format!("读取项目目录失败: {}", e))?
    {
        let project_dir = entry
            .map_err(|e| format!("读取条目失败: {}", e))?
            .path();

        let project_name = project_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let decoded_path = decode_project_path(project_name);

        let session_file = project_dir.join(format!("{}.jsonl", session_id));

        if session_file.exists() {
            let (_, conversation) = parse_session_file(&session_file, &decoded_path)?;
            return Ok(conversation);
        }
    }

    Err(format!("Session {} 不存在", session_id))
}

/// 删除 session 文件
pub fn delete_session(session_id: &str) -> Result<(), String> {
    let projects_dir = get_projects_dir();

    // 遍历找到对应 session 文件
    for entry in fs::read_dir(&projects_dir)
        .map_err(|e| format!("读取项目目录失败: {}", e))?
    {
        let project_dir = entry
            .map_err(|e| format!("读取条目失败: {}", e))?
            .path();

        let sessions_dir = project_dir;

        if !sessions_dir.exists() {
            continue;
        }

        let session_file = sessions_dir.join(format!("{}.jsonl", session_id));

        if session_file.exists() {
            fs::remove_file(&session_file)
                .map_err(|e| format!("删除 session 文件失败: {}", e))?;
            return Ok(())
        }
    }

    Err(format!("Session {} 不存在", session_id))
}