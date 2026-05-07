use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use std::io::{BufRead, BufReader};
use tracing::{info, debug, warn, error};
use std::time::Instant;

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
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub cwd: String,
    #[serde(rename = "startedAt")]
    pub started_at: u64,
    pub kind: String,
    pub entrypoint: String,
    #[serde(default)]
    pub status: String,
    #[serde(rename = "updatedAt", default)]
    pub updated_at: Option<u64>,
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
    debug!("[decode_project_path] 输入: {}", project_name);

    // 例如: C--workspace-claude-fleet-sp -> C:\workspace\claude-fleet-sp
    let parts: Vec<&str> = project_name.split('-').collect();
    if parts.is_empty() {
        warn!("[decode_project_path] 空输入，返回原值");
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

    debug!("[decode_project_path] 输出: {}", path);
    path
}

/// 获取运行中 session 的元数据
pub fn get_running_sessions() -> Result<Vec<RunningSessionMetadata>, String> {
    info!("[get_running_sessions] 开始读取运行中 session 元数据");
    let sessions_dir = get_sessions_dir();
    debug!("[get_running_sessions] sessions 目录: {}", sessions_dir.display());

    if !sessions_dir.exists() {
        warn!("[get_running_sessions] sessions 目录不存在，返回空列表");
        return Ok(Vec::new());
    }

    let mut running_sessions: Vec<RunningSessionMetadata> = Vec::new();
    let mut file_count = 0;
    let mut parse_success = 0;
    let mut parse_fail = 0;

    let entries = fs::read_dir(&sessions_dir)
        .map_err(|e| {
            error!("[get_running_sessions] 读取 sessions 目录失败: {}", e);
            format!("读取 sessions 目录失败: {}", e)
        })?;

    for entry in entries {
        let file_path = entry
            .map_err(|e| {
                warn!("[get_running_sessions] 读取条目失败: {}", e);
                format!("读取条目失败: {}", e)
            })?
            .path();

        if file_path.extension().and_then(|s| s.to_str()) != Some("json") {
            debug!("[get_running_sessions] 跳过非 json 文件: {}", file_path.display());
            continue;
        }

        file_count += 1;
        debug!("[get_running_sessions] 读取文件 #{}: {}", file_count, file_path.display());

        let content = fs::read_to_string(&file_path)
            .map_err(|e| {
                warn!("[get_running_sessions] 读取文件 {} 失败: {}", file_path.display(), e);
                format!("读取文件 {} 失败: {}", file_path.display(), e)
            })?;

        debug!("[get_running_sessions] 文件内容长度: {} 字节", content.len());

        if let Ok(metadata) = serde_json::from_str::<RunningSessionMetadata>(&content) {
            parse_success += 1;
            info!("[get_running_sessions] 解析成功: pid={}, session_id={}, cwd={}, status={}",
                  metadata.pid, metadata.session_id, metadata.cwd, metadata.status);
            running_sessions.push(metadata);
        } else {
            parse_fail += 1;
            warn!("[get_running_sessions] 解析失败: {}", file_path.display());
        }
    }

    info!("[get_running_sessions] 统计: 文件数={}, 解析成功={}, 解析失败={}",
          file_count, parse_success, parse_fail);
    info!("[get_running_sessions] 完成，返回 {} 个运行中 session", running_sessions.len());
    Ok(running_sessions)
}

/// 解析单个 session JSONL 文件
pub fn parse_session_file(
    file_path: &PathBuf,
    project_path: &str,
) -> Result<(ClaudeSession, Conversation), String> {
    info!("[parse_session_file] 开始解析: {}", file_path.display());
    let start = Instant::now();

    let file = fs::File::open(file_path)
        .map_err(|e| {
            error!("[parse_session_file] 无法打开文件 {}: {}", file_path.display(), e);
            format!("无法打开文件 {}: {}", file_path.display(), e)
        })?;

    let reader = BufReader::new(file);
    let mut messages: Vec<ConversationMessage> = Vec::new();
    let mut created_at = String::new();
    let mut last_activity_at = String::new();
    let mut working_directory = String::new();
    let mut line_count = 0;
    let mut user_msg_count = 0;
    let mut assistant_msg_count = 0;
    let mut other_line_count = 0;

    for (index, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| {
            warn!("[parse_session_file] 读取行 {} 失败: {}", index, e);
            format!("读取行失败: {}", e)
        })?;

        if line.trim().is_empty() {
            debug!("[parse_session_file] 跳过空行 #{}", index);
            continue;
        }

        line_count += 1;

        // 尝试解析 JSON
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&line) {
            let line_type = json_value
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            debug!("[parse_session_file] 行 #{}: type={}", index, line_type);

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
                debug!("[parse_session_file] 提取工作目录: {}", working_directory);
            }

            // 第一条消息的时间作为创建时间
            if index == 0 {
                created_at = timestamp.clone();
                debug!("[parse_session_file] 创建时间: {}", created_at);
            }
            last_activity_at = timestamp.clone();

            // 提取消息内容
            if line_type == "user" {
                user_msg_count += 1;
                let message = json_value.get("message");
                if let Some(msg) = message {
                    let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
                    let content = extract_message_content(msg);
                    debug!("[parse_session_file] user 消息: role={}, content 长度={}", role, content.len());

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
                assistant_msg_count += 1;
                let message = json_value.get("message");
                if let Some(msg) = message {
                    let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("assistant");
                    let content = extract_assistant_content(msg);
                    debug!("[parse_session_file] assistant 消息: role={}, content 长度={}", role, content.len());

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
            } else {
                other_line_count += 1;
                debug!("[parse_session_file] 其他类型行: {}", line_type);
            }
        } else {
            warn!("[parse_session_file] 行 #{} JSON 解析失败", index);
        }
    }

    // 从文件名提取 session ID
    let session_id = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    debug!("[parse_session_file] session_id: {}", session_id);

    // 使用工作目录名作为默认名称
    let name = if working_directory.is_empty() {
        session_id.clone()
    } else {
        get_last_path_segment(&working_directory)
    };
    debug!("[parse_session_file] session 名称: {}", name);

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

    let elapsed = start.elapsed();
    info!("[parse_session_file] 完成: session_id={}, 行数={}, user={}, assistant={}, other={}, 耗时: {}ms",
          session.id, line_count, user_msg_count, assistant_msg_count, other_line_count, elapsed.as_millis());

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
    let start = Instant::now();
    info!("[get_all_sessions] 开始获取 session 列表");

    let projects_dir = get_projects_dir();
    debug!("[get_all_sessions] projects 目录: {}", projects_dir.display());

    if !projects_dir.exists() {
        warn!("[get_all_sessions] projects 目录不存在，返回空列表");
        return Ok(Vec::new());
    }

    // 获取运行中 session 的 PID 映射
    debug!("[get_all_sessions] 读取运行中 session 元数据");
    let running_sessions = get_running_sessions()?;
    info!("[get_all_sessions] 运行中 session 数量: {}", running_sessions.len());
    let running_map: std::collections::HashMap<String, u32> = running_sessions
        .iter()
        .map(|s| (s.session_id.clone(), s.pid))
        .collect();
    debug!("[get_all_sessions] running_map 构建 完成，包含 {} 个映射", running_map.len());

    let mut sessions: Vec<ClaudeSession> = Vec::new();
    let mut project_count = 0;
    let mut total_jsonl_count = 0;
    let mut parse_success_count = 0;
    let mut parse_fail_count = 0;

    // 遍历所有项目目录
    let entries = fs::read_dir(&projects_dir)
        .map_err(|e| {
            error!("[get_all_sessions] 读取项目目录失败: {}", e);
            format!("读取项目目录失败: {}", e)
        })?;

    for entry in entries {
        let project_dir = entry
            .map_err(|e| {
                warn!("[get_all_sessions] 读取条目失败: {}", e);
                format!("读取条目失败: {}", e)
            })?
            .path();

        let project_name = project_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let decoded_path = decode_project_path(project_name);
        project_count += 1;
        debug!("[get_all_sessions] 处理项目 #{}: {} -> {}", project_count, project_name, decoded_path);

        // 遍历项目目录下的所有 jsonl 文件
        let mut jsonl_count = 0;
        for project_entry in fs::read_dir(&project_dir)
            .map_err(|e| {
                warn!("[get_all_sessions] 读取项目 {} 目录失败: {}", project_dir.display(), e);
                format!("读取项目 {} 目录失败: {}", project_dir.display(), e)
            })?
        {
            let entry_path = project_entry
                .map_err(|e| {
                    warn!("[get_all_sessions] 读取条目失败: {}", e);
                    format!("读取条目失败: {}", e)
                })?
                .path();

            // 只处理 jsonl 文件
            if entry_path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                debug!("[get_all_sessions] 跳过非 jsonl 文件: {}", entry_path.display());
                continue;
            }

            jsonl_count += 1;
            total_jsonl_count += 1;

            match parse_session_file(&entry_path, &decoded_path) {
                Ok((session, _)) => {
                    parse_success_count += 1;
                    debug!("[get_all_sessions] 解析成功: {} - 消息数: {}", session.id, session.conversation_count);

                    // 检查是否正在运行
                    let process_id = running_map.get(&session.id).copied();
                    let status = if process_id.is_some() {
                        // 检查进程是否确实在运行
                        let pid = process_id.unwrap();
                        if is_process_running(pid) {
                            debug!("[get_all_sessions] session {} 进程 {} 正在运行", session.id, pid);
                            "running".to_string()
                        } else {
                            warn!("[get_all_sessions] session {} 进程 {} 已退出，标记为 idle", session.id, pid);
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
                Err(e) => {
                    parse_fail_count += 1;
                    warn!("[get_all_sessions] 解析失败 {}: {}", entry_path.display(), e);
                }
            }
        }

        if jsonl_count > 0 {
            debug!("[get_all_sessions] 项目 {} 有 {} 个 jsonl 文件", project_name, jsonl_count);
        }
    }

    info!("[get_all_sessions] 统计: 项目数={}, jsonl文件数={}, 解析成功={}, 解析失败={}",
          project_count, total_jsonl_count, parse_success_count, parse_fail_count);

    // 按最后活动时间排序（最近的在前）
    sessions.sort_by(|a, b| b.last_activity_at.cmp(&a.last_activity_at));
    debug!("[get_all_sessions] 排序完成");

    let elapsed = start.elapsed();
    info!("[get_all_sessions] 完成，返回 {} 个 session，耗时: {}ms", sessions.len(), elapsed.as_millis());
    Ok(sessions)
}

/// 检查进程是否正在运行
fn is_process_running(pid: u32) -> bool {
    debug!("[is_process_running] 检查进程 PID: {}", pid);

    // Windows: 使用 tasklist 检查进程
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: 隐藏命令行窗口
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let output = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let is_running = stdout.contains(&pid.to_string());
            debug!("[is_process_running] Windows tasklist 结果: PID {} {}",
                   pid, if is_running { "存在" } else { "不存在" });
            is_running
        } else {
            warn!("[is_process_running] Windows tasklist 执行失败");
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
            let is_running = output.status.success();
            debug!("[is_process_running] ps 结果: PID {} {}",
                   pid, if is_running { "存在" } else { "不存在" });
            is_running
        } else {
            warn!("[is_process_running] ps 执行失败");
            false
        }
    }
}

/// 检查 PID 是否为 claude 进程
#[cfg(target_os = "windows")]
pub fn is_claude_process_running(pid: u32) -> bool {
    debug!("[is_claude_process_running] 检查 PID {} 是否为 claude 进程", pid);
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    // CREATE_NO_WINDOW: 隐藏命令行窗口
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let pid_exists = stdout.contains(&pid.to_string());
        let is_claude = stdout.to_lowercase().contains("claude");
        debug!("[is_claude_process_running] tasklist 输出: {}", stdout.trim());
        debug!("[is_claude_process_running] PID {} 存在: {}, 进程名包含 claude: {}", pid, pid_exists, is_claude);

        let result = pid_exists && is_claude;
        if result {
            info!("[is_claude_process_running] PID {} 确认是 claude 进程", pid);
        } else {
            warn!("[is_claude_process_running] PID {} 不是 claude 进程或进程已退出", pid);
        }
        result
    } else {
        error!("[is_claude_process_running] tasklist 执行失败");
        false
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_claude_process_running(pid: u32) -> bool {
    debug!("[is_claude_process_running] 检查 PID {} 是否为 claude 进程", pid);
    use std::process::Command;

    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let is_claude = stdout.to_lowercase().contains("claude");
        debug!("[is_claude_process_running] ps 输出: {}", stdout.trim());
        debug!("[is_claude_process_running] 进程名包含 claude: {}", is_claude);

        if is_claude {
            info!("[is_claude_process_running] PID {} 确认是 claude 进程", pid);
        } else {
            warn!("[is_claude_process_running] PID {} 不是 claude 进程", pid);
        }
        is_claude
    } else {
        error!("[is_claude_process_running] ps 执行失败");
        false
    }
}

/// 获取指定 session 的对话内容
pub fn get_session_conversation(session_id: &str) -> Result<Conversation, String> {
    info!("[get_session_conversation] 开始查找 session: {}", session_id);
    let start = Instant::now();

    let projects_dir = get_projects_dir();
    debug!("[get_session_conversation] projects 目录: {}", projects_dir.display());

    // 需要遍历找到对应 session 文件
    let mut checked_dirs = 0;
    for entry in fs::read_dir(&projects_dir)
        .map_err(|e| {
            error!("[get_session_conversation] 读取项目目录失败: {}", e);
            format!("读取项目目录失败: {}", e)
        })?
    {
        let project_dir = entry
            .map_err(|e| {
                warn!("[get_session_conversation] 读取条目失败: {}", e);
                format!("读取条目失败: {}", e)
            })?
            .path();

        checked_dirs += 1;
        let project_name = project_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let decoded_path = decode_project_path(project_name);
        debug!("[get_session_conversation] 检查项目 #{}: {} -> {}", checked_dirs, project_name, decoded_path);

        let session_file = project_dir.join(format!("{}.jsonl", session_id));

        if session_file.exists() {
            info!("[get_session_conversation] 找到文件: {}", session_file.display());
            let (_, conversation) = parse_session_file(&session_file, &decoded_path)?;
            let elapsed = start.elapsed();
            info!("[get_session_conversation] 完成，消息数: {}, 耗时: {}ms",
                  conversation.total_messages, elapsed.as_millis());
            return Ok(conversation);
        } else {
            debug!("[get_session_conversation] 文件不存在: {}", session_file.display());
        }
    }

    let elapsed = start.elapsed();
    warn!("[get_session_conversation] 未找到 session {}，检查了 {} 个目录，耗时: {}ms",
          session_id, checked_dirs, elapsed.as_millis());
    Err(format!("Session {} 不存在", session_id))
}

/// 删除 session 文件
pub fn delete_session(session_id: &str) -> Result<(), String> {
    info!("[delete_session] 开始删除 session: {}", session_id);
    let start = Instant::now();

    let projects_dir = get_projects_dir();
    debug!("[delete_session] projects 目录: {}", projects_dir.display());

    // 遍历找到对应 session 文件
    let mut checked_dirs = 0;
    for entry in fs::read_dir(&projects_dir)
        .map_err(|e| {
            error!("[delete_session] 读取项目目录失败: {}", e);
            format!("读取项目目录失败: {}", e)
        })?
    {
        let project_dir = entry
            .map_err(|e| {
                warn!("[delete_session] 读取条目失败: {}", e);
                format!("读取条目失败: {}", e)
            })?
            .path();

        checked_dirs += 1;
        debug!("[delete_session] 检查项目 #{}: {}", checked_dirs, project_dir.display());

        let sessions_dir = project_dir;

        if !sessions_dir.exists() {
            debug!("[delete_session] 目录不存在，跳过: {}", sessions_dir.display());
            continue;
        }

        let session_file = sessions_dir.join(format!("{}.jsonl", session_id));

        if session_file.exists() {
            info!("[delete_session] 找到文件，删除: {}", session_file.display());
            fs::remove_file(&session_file)
                .map_err(|e| {
                    error!("[delete_session] 删除文件 {} 失败: {}", session_file.display(), e);
                    format!("删除 session 文件失败: {}", e)
                })?;

            let elapsed = start.elapsed();
            info!("[delete_session] 完成，检查了 {} 个目录，耗时: {}ms", checked_dirs, elapsed.as_millis());
            return Ok(())
        } else {
            debug!("[delete_session] 文件不存在: {}", session_file.display());
        }
    }

    let elapsed = start.elapsed();
    warn!("[delete_session] 未找到 session {}，检查了 {} 个目录，耗时: {}ms",
          session_id, checked_dirs, elapsed.as_millis());
    Err(format!("Session {} 不存在", session_id))
}