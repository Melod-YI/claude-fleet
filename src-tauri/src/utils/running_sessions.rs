use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, Arc, mpsc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use once_cell::sync::Lazy;
use std::fs;
use std::path::PathBuf;
use tauri::Emitter;
use tracing::{info, debug, warn, error};
use std::time::Instant;
use crate::utils::claude_data::is_claude_process_running;
use crate::utils::window_manager::{
    get_window_title_by_pid_chain,
    populate_window_cache_parallel,
    get_cached_window_title,
    get_cached_window,
    is_cached_console_window,
    clear_window_cache,
    invalidate_window_cache,
};
use crate::utils::claude_session::{extract_away_summary, extract_last_user_input};
use crate::utils::git::info::{gather_git_info, GitInfo};
use std::path::Path;

/// Session 运行状态（对应 Claude JSON 文件中的三种状态）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Busy,        // agent 与 LLM 在 loop 中
    Idle,        // 等待用户输入
    Waiting,     // 等待用户输入
}

/// 运行中 Session 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningSession {
    pub session_id: String,
    pub pid: u32,
    pub status: SessionStatus,
    pub cwd: String,
    pub name: String,
    pub updated_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub away_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub away_summary_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_user_input: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_name: Option<String>,  // Claude Fleet 自定义名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_info: Option<GitInfo>,  // 工作目录 git 概要信息
}

/// 全局运行中 Session 状态（PID 作为 key，因为 sessionId 会因 resume 变化）
pub static RUNNING_SESSIONS: Lazy<Mutex<HashMap<u32, RunningSession>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// 缓存 away_summary 结果（session_id -> (summary, timestamp, checked_at)）
static AWAY_SUMMARY_CACHE: Lazy<Mutex<HashMap<String, (Option<String>, Option<u64>, u64)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// git 信息后台采集的去重缓存：cwd -> 上次触发 Instant。
/// 同一 cwd 在 GIT_REFRESH_DEDUPE_SECS 内仅触发一次（自动触发场景）。
static GIT_REFRESH_CACHE: Lazy<Mutex<HashMap<String, Instant>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// 自动触发去重窗口（秒）。
const GIT_REFRESH_DEDUPE_SECS: u64 = 5;

/// 轮询运行状态
static POLLING_RUNNING: AtomicBool = AtomicBool::new(false);

/// Hook 事件结构（用于前端通知事件兼容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub session_id: String,
    pub hook_event_name: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub transcript_path: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Session 文件内容结构（从 ~/.claude/sessions/<pid>.json）
#[derive(Debug, Clone, Deserialize)]
pub struct SessionFileContent {
    pub pid: u32,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub cwd: String,
    #[serde(rename = "startedAt")]
    pub started_at: u64,
    #[serde(default)]
    pub status: String,
    #[serde(rename = "waitingFor", default)]
    pub waiting_for: Option<String>,
    #[serde(rename = "updatedAt", default)]
    pub updated_at: Option<u64>,
    #[serde(default)]
    pub name: Option<String>,
}

/// 获取 sessions 目录路径
pub fn get_sessions_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户目录")
        .join(".claude")
        .join("sessions")
}

/// 从路径提取最后一段作为名称
pub fn get_path_name(path: &str) -> String {
    path.split(|c| c == '\\' || c == '/')
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or(path)
        .to_string()
}

/// 解析 session 名称（优先级：自定义名称 > 窗口名 > 文件夹名）
pub fn resolve_session_name(content: &SessionFileContent) -> String {
    info!("[resolve_session_name] 开始解析名称: pid={}", content.pid);

    // 1. 优先使用用户自定义名称
    if let Some(custom_name) = &content.name {
        if !custom_name.is_empty() {
            info!("[resolve_session_name] 使用自定义名称: {}", custom_name);
            return custom_name.clone();
        }
    }

    // 2. 尝试获取窗口标题
    if let Some(window_title) = get_window_title_by_pid_chain(content.pid) {
        // 判断是否为默认标题（忽略大小写，以 "claude code" 或 "claude-code" 结尾）
        let title_lower = window_title.trim().to_lowercase();
        let is_default_title = title_lower.ends_with("claude code") || title_lower.ends_with("claude-code");

        if !is_default_title && !window_title.is_empty() {
            info!("[resolve_session_name] 使用窗口标题: {}", window_title);
            return window_title;
        }
        debug!("[resolve_session_name] 窗口标题为默认值 \"{}\"，使用文件夹名", window_title);
    }

    // 3. 使用文件夹名称
    let folder_name = get_path_name(&content.cwd);
    info!("[resolve_session_name] 使用文件夹名: {}", folder_name);
    folder_name
}

/// 从文件内容添加 session 到运行中列表
pub fn add_running_session_from_file(content: &SessionFileContent) -> Result<(), String> {
    info!("[add_running_session_from_file] 开始添加: pid={}, sessionId={}", content.pid, content.session_id);
    let start = Instant::now();

    // 验证进程是否为 claude
    debug!("[add_running_session_from_file] 检查进程 PID={} 是否为 claude", content.pid);
    if !is_claude_process_running(content.pid) {
        warn!("[add_running_session_from_file] PID {} 不是 claude 进程或进程已退出", content.pid);
        return Err(format!("PID {} 不是 claude 进程", content.pid));
    }
    info!("[add_running_session_from_file] PID {} 确认是 claude 进程", content.pid);

    // 提取名称（优先级：自定义名称 > 窗口名 > 文件夹名）
    let name = resolve_session_name(content);
    debug!("[add_running_session_from_file] 最终名称: {}", name);

    // 状态映射：busy -> Busy, idle -> Idle, waiting -> Waiting
    let status = match content.status.as_str() {
        "busy" => {
            debug!("[add_running_session_from_file] status=busy -> Busy");
            SessionStatus::Busy
        }
        "idle" => {
            debug!("[add_running_session_from_file] status=idle -> Idle");
            SessionStatus::Idle
        }
        "waiting" => {
            debug!("[add_running_session_from_file] status=waiting -> Waiting");
            SessionStatus::Waiting
        }
        _ => {
            warn!("[add_running_session_from_file] status={} 未知，默认为 Busy", content.status);
            SessionStatus::Busy
        }
    };

    let session = RunningSession {
        session_id: content.session_id.clone(),
        pid: content.pid,
        status,
        cwd: content.cwd.clone(),
        name,
        updated_at: content.updated_at.unwrap_or(content.started_at),
        away_summary: None,
        away_summary_at: None,
        last_user_input: None,
        custom_name: None,
        git_info: None,
    };

    info!("[add_running_session_from_file] 创建 RunningSession: id={}, pid={}, status={}, cwd={}",
          session.session_id, session.pid,
          match status {
              SessionStatus::Busy => "busy",
              SessionStatus::Idle => "idle",
              SessionStatus::Waiting => "waiting",
          },
          session.cwd);

    // 添加到全局状态（PID 作为 key）
    RUNNING_SESSIONS.lock().unwrap().insert(content.pid, session);

    let elapsed = start.elapsed();
    info!("[add_running_session_from_file] 完成，耗时: {}ms", elapsed.as_millis());
    Ok(())
}

/// 从文件内容更新 session 状态（PID 作为查找 key）
pub fn update_session_status_from_file(content: &SessionFileContent) {
    info!("[update_session_status_from_file] 开始更新: pid={}, sessionId={}, status={}, name={:?}",
          content.pid, content.session_id, content.status, content.name);

    let mut sessions = RUNNING_SESSIONS.lock().unwrap();

    // 用 PID 查找（sessionId 会因 resume 变化）
    if let Some(session) = sessions.get_mut(&content.pid) {
        // 检查 sessionId 是否变化（resume 情况）
        if session.session_id != content.session_id {
            info!("[update_session_status_from_file] sessionId 变化: {} -> {}",
                  session.session_id, content.session_id);
            session.session_id = content.session_id.clone();
            // sessionId 变化意味着新的 session，清空旧的 away_summary 和缓存
            session.away_summary = None;
            session.away_summary_at = None;
            AWAY_SUMMARY_CACHE.lock().unwrap().remove(&session.session_id);
        }

        // 更新名称（用户可能通过 /rename 命令修改）
        let new_name = resolve_session_name(content);
        if session.name != new_name {
            info!("[update_session_status_from_file] 名称变化: {} -> {}", session.name, new_name);
            session.name = new_name;
        }

        let new_status = match content.status.as_str() {
            "busy" => SessionStatus::Busy,
            "idle" => SessionStatus::Idle,
            "waiting" => SessionStatus::Waiting,
            _ => SessionStatus::Busy,
        };

        let old_status = session.status;
        session.status = new_status;
        session.updated_at = content.updated_at.unwrap_or(session.updated_at);

        info!("[update_session_status_from_file] 完成: {} -> {}",
              match old_status {
                  SessionStatus::Busy => "busy",
                  SessionStatus::Idle => "idle",
                  SessionStatus::Waiting => "waiting",
              },
              match new_status {
                  SessionStatus::Busy => "busy",
                  SessionStatus::Idle => "idle",
                  SessionStatus::Waiting => "waiting",
              });
    } else {
        warn!("[update_session_status_from_file] 未找到 PID={} 的 session", content.pid);
    }
}

/// 根据 PID 移除 session（PID 作为 key，直接移除）
pub fn remove_running_session_by_pid(pid: u32) {
    info!("[remove_running_session_by_pid] 开始移除: pid={}", pid);

    let mut sessions = RUNNING_SESSIONS.lock().unwrap();

    if let Some(session) = sessions.remove(&pid) {
        info!("[remove_running_session_by_pid] 完成: pid={}, sessionId={} 已移除，剩余 {} 个 session",
              pid, session.session_id, sessions.len());
    } else {
        warn!("[remove_running_session_by_pid] 未找到 pid={} 的 session", pid);
    }
}

/// 后台采集并更新指定 session 的 git 信息，非阻塞。
/// - `force = true`：绕过去重（手动刷新）。
/// - `force = false`：受 `GIT_REFRESH_DEDUPE_SECS` 去重约束（自动触发）。
/// 采集完成后写回 `RUNNING_SESSIONS` 并 emit `running_sessions_changed`。
pub fn refresh_git_info_background(pid: u32, app_handle: tauri::AppHandle, force: bool) {
    info!("[refresh_git_info_background] 触发: pid={}, force={}", pid, force);

    thread::spawn(move || {
        // 1. 读取 cwd（退出锁，避免长持有）
        let cwd = {
            let sessions = RUNNING_SESSIONS.lock().unwrap();
            match sessions.get(&pid) {
                Some(s) => s.cwd.clone(),
                None => {
                    info!("[refresh_git_info_background] pid={} 不存在，跳过", pid);
                    return;
                }
            }
        };

        // 2. 去重（仅自动触发）
        if !force {
            let now = Instant::now();
            let should_skip = {
                let mut cache = GIT_REFRESH_CACHE.lock().unwrap();
                if let Some(last) = cache.get(&cwd) {
                    if now.duration_since(*last).as_secs() < GIT_REFRESH_DEDUPE_SECS {
                        true
                    } else {
                        cache.insert(cwd.clone(), now);
                        false
                    }
                } else {
                    cache.insert(cwd.clone(), now);
                    false
                }
            };
            if should_skip {
                debug!("[refresh_git_info_background] cwd={} 在 {}s 内已触发，跳过",
                       cwd, GIT_REFRESH_DEDUPE_SECS);
                return;
            }
        }

        // 3. 采集
        let git_info = gather_git_info(Path::new(&cwd));

        // 4. 写回
        {
            let mut sessions = RUNNING_SESSIONS.lock().unwrap();
            if let Some(s) = sessions.get_mut(&pid) {
                s.git_info = git_info;
                info!("[refresh_git_info_background] 已更新 pid={} 的 git_info", pid);
            } else {
                info!("[refresh_git_info_background] pid={} 已移除，丢弃采集结果", pid);
                return;
            }
        }

        // 5. 通知前端
        let sessions = get_running_sessions();
        if let Err(e) = app_handle.emit("running_sessions_changed", sessions) {
            error!("[refresh_git_info_background] 发送事件失败: {}", e);
        } else {
            debug!("[refresh_git_info_background] 事件发送成功: pid={}", pid);
        }
    });
}

/// 获取所有运行中 session
pub fn get_running_sessions() -> Vec<RunningSession> {
    debug!("[get_running_sessions] 开始获取");

    let sessions: Vec<RunningSession> = RUNNING_SESSIONS.lock().unwrap().values().cloned().collect();

    let busy_count = sessions.iter().filter(|s| s.status == SessionStatus::Busy).count();
    let idle_count = sessions.iter().filter(|s| s.status == SessionStatus::Idle).count();
    let waiting_count = sessions.iter().filter(|s| s.status == SessionStatus::Waiting).count();
    let waiting_input_total = idle_count + waiting_count;

    debug!("[get_running_sessions] 完成: 总数={}, busy={}, idle={}, waiting={}, 等待输入总数={}",
           sessions.len(), busy_count, idle_count, waiting_count, waiting_input_total);
    sessions
}

/// 应用启动时初始化运行中 session 列表（扫描 sessions 目录）
pub fn init_running_sessions() -> Result<Vec<RunningSession>, String> {
    info!("[init_running_sessions] 开始初始化运行中 session 列表");
    let start = Instant::now();

    let sessions_dir = get_sessions_dir();
    debug!("[init_running_sessions] sessions 目录: {}", sessions_dir.display());

    if !sessions_dir.exists() {
        info!("[init_running_sessions] sessions 目录不存在，返回空列表");
        return Ok(Vec::new());
    }

    // 清空现有状态
    debug!("[init_running_sessions] 清空 RUNNING_SESSIONS");
    RUNNING_SESSIONS.lock().unwrap().clear();
    // 清空旧的窗口缓存
    clear_window_cache();

    // 扫描 sessions 目录下的所有 JSON 文件
    let entries = fs::read_dir(&sessions_dir)
        .map_err(|e| {
            error!("[init_running_sessions] 读取目录失败: {}", e);
            format!("读取 sessions 目录失败: {}", e)
        })?;

    let mut total_files = 0;
    let mut json_files = 0;
    let mut added_count = 0;
    let mut parse_failed = 0;
    let mut process_dead = 0;

    for entry in entries {
        let file_path = entry.map_err(|e| {
            warn!("[init_running_sessions] 读取条目失败: {}", e);
            format!("读取条目失败: {}", e)
        })?.path();

        total_files += 1;

        if file_path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        json_files += 1;
        debug!("[init_running_sessions] 检查文件 #{}: {}", json_files, file_path.display());

        // 解析文件内容
        if let Ok(content) = fs::read_to_string(&file_path) {
            if let Ok(session_content) = serde_json::from_str::<SessionFileContent>(&content) {
                debug!("[init_running_sessions] 解析成功: pid={}, sessionId={}, status={}",
                       session_content.pid, session_content.session_id, session_content.status);

                // 验证进程是否存活
                if !is_claude_process_running(session_content.pid) {
                    process_dead += 1;
                    debug!("[init_running_sessions] PID {} 进程已退出，跳过", session_content.pid);
                    continue;
                }

                // 添加到运行中列表
                if add_running_session_from_file(&session_content).is_ok() {
                    added_count += 1;
                }
            } else {
                parse_failed += 1;
                warn!("[init_running_sessions] 解析失败: {}", file_path.display());
            }
        } else {
            parse_failed += 1;
            warn!("[init_running_sessions] 读取文件失败: {}", file_path.display());
        }
    }

    info!("[init_running_sessions] 统计: 总文件={}, json={}, 解析失败={}, 进程已退出={}, 成功添加={}",
          total_files, json_files, parse_failed, process_dead, added_count);

    // 启动时立即扫描所有 session 的 jsonl
    info!("[init_running_sessions] 开始扫描 jsonl 文件获取 away_summary 和 last_user_input");
    scan_away_summaries();

    // 后台缓存所有运行中 session 的窗口信息（不阻塞主线程）
    {
        let pids: Vec<u32> = RUNNING_SESSIONS.lock().unwrap().keys().cloned().collect();
        if !pids.is_empty() {
            info!("[init_running_sessions] 后台缓存 {} 个 session 的窗口信息", pids.len());
            populate_window_cache_parallel(&pids);
        }
    }

    let result = get_running_sessions();
    let elapsed = start.elapsed();
    info!("[init_running_sessions] 完成，运行中 session 数量: {}，耗时: {}ms", result.len(), elapsed.as_millis());
    Ok(result)
}

/// 并行检查多个 PID 的进程存活状态
/// 返回 (存活 PID 列表, 已退出 PID 列表)
pub fn check_processes_parallel(pids: Vec<u32>) -> (Vec<u32>, Vec<u32>) {
    if pids.is_empty() {
        return (Vec::new(), Vec::new());
    }

    info!("[check_processes_parallel] 开始并行检查 {} 个进程", pids.len());
    let start = Instant::now();

    let pid_count = pids.len();

    // 使用 channel 收集结果
    let (tx, rx) = mpsc::channel();
    let tx = Arc::new(tx);

    // 为每个 PID 创建检查线程
    for pid in pids {
        let tx_clone = Arc::clone(&tx);
        thread::spawn(move || {
            let running = is_claude_process_running(pid);
            let _ = tx_clone.send((pid, running));
        });
    }

    // 收集所有结果
    let mut alive = Vec::new();
    let mut dead = Vec::new();

    for _ in 0..pid_count {
        if let Ok((pid, running)) = rx.recv_timeout(Duration::from_secs(5)) {
            if running {
                alive.push(pid);
            } else {
                dead.push(pid);
            }
        }
    }

    let elapsed = start.elapsed();
    info!("[check_processes_parallel] 完成: 存活={}, 已退出={}, 耗时={}ms",
          alive.len(), dead.len(), elapsed.as_millis());
    (alive, dead)
}

/// 并行获取多个 PID 的窗口标题
/// 返回 PID -> 窗口标题 映射
pub fn get_window_titles_parallel(pids: Vec<u32>) -> HashMap<u32, Option<String>> {
    if pids.is_empty() {
        return HashMap::new();
    }

    info!("[get_window_titles_parallel] 开始并行获取 {} 个窗口标题", pids.len());
    let start = Instant::now();

    let pid_count = pids.len();

    let (tx, rx) = mpsc::channel();
    let tx = Arc::new(tx);

    for pid in pids {
        let tx_clone = Arc::clone(&tx);
        thread::spawn(move || {
            let title = get_window_title_by_pid_chain(pid);
            let _ = tx_clone.send((pid, title));
        });
    }

    let mut results = HashMap::new();
    for _ in 0..pid_count {
        if let Ok((pid, title)) = rx.recv_timeout(Duration::from_secs(3)) {
            results.insert(pid, title);
        }
    }

    let elapsed = start.elapsed();
    info!("[get_window_titles_parallel] 完成: 获取 {} 个标题, 耗时={}ms", results.len(), elapsed.as_millis());
    results
}

/// 检查 session 是否有自定义名称（从文件读取）
fn has_custom_name(pid: u32) -> bool {
    let sessions_dir = get_sessions_dir();
    let file_path = sessions_dir.join(format!("{}.json", pid));

    if let Ok(content) = fs::read_to_string(&file_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            return json.get("name")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false);
        }
    }
    false
}

/// 刷新所有运行中 session 的名称
/// 用于轮询中检测窗口标题变化
pub fn refresh_session_names() {
    info!("[refresh_session_names] 开始刷新名称");
    let start = Instant::now();

    // 获取所有 PID 和当前信息
    let sessions_info: Vec<(u32, String, String)> = RUNNING_SESSIONS
        .lock()
        .unwrap()
        .iter()
        .map(|(pid, s)| (*pid, s.name.clone(), s.cwd.clone()))
        .collect();

    if sessions_info.is_empty() {
        debug!("[refresh_session_names] 无 session，跳过");
        return;
    }

    // 并行获取窗口标题（优先从缓存读取，单次 GetWindowTextW 调用）
    let pids: Vec<u32> = sessions_info.iter().map(|(pid, _, _)| *pid).collect();
    let mut titles: HashMap<u32, Option<String>> = HashMap::new();
    let mut uncached_pids: Vec<u32> = Vec::new();

    for &pid in &pids {
        // Windows Terminal 的 pseudo console 窗口无标题，且父链会拿到 WT 主窗口标题
        // （反映当前活动 tab，随切换抖动），直接用文件夹名，跳过父链标题查询
        if is_cached_console_window(pid) {
            titles.insert(pid, None);
            continue;
        }
        match get_cached_window_title(pid) {
            Some(title) => { titles.insert(pid, Some(title)); }
            None => { uncached_pids.push(pid); }
        }
    }

    if !uncached_pids.is_empty() {
        debug!("[refresh_session_names] {} 个 PID 缓存未命中，回退到完整查找", uncached_pids.len());
        let fallback_titles = get_window_titles_parallel(uncached_pids);
        titles.extend(fallback_titles);
    }

    // 更新名称（如果窗口标题变化且无自定义名称）
    let mut updated_count = 0;
    {
        let mut sessions = RUNNING_SESSIONS.lock().unwrap();
        for (pid, _current_name, cwd) in sessions_info {
            if let Some(session) = sessions.get_mut(&pid) {
                // 检查是否有自定义名称（从文件读取）
                if has_custom_name(pid) {
                    debug!("[refresh_session_names] PID {} 有自定义名称，跳过", pid);
                    continue;
                }

                // 获取窗口标题
                let window_title = titles.get(&pid).and_then(|t| t.clone());

                // 计算新名称
                let new_name = if let Some(title) = window_title {
                    // 检查是否为默认标题
                    let title_lower = title.trim().to_lowercase();
                    let is_default = title_lower.ends_with("claude code")
                        || title_lower.ends_with("claude-code");

                    if is_default {
                        // 使用文件夹名
                        get_path_name(&cwd)
                    } else {
                        // 使用窗口标题
                        title
                    }
                } else {
                    // 无窗口标题，使用文件夹名
                    get_path_name(&cwd)
                };

                // 检查名称是否变化
                if session.name != new_name {
                    info!("[refresh_session_names] PID {} 名称变化: {} -> {}",
                          pid, session.name, new_name);
                    session.name = new_name;
                    updated_count += 1;
                }
            }
        }
    }

    let elapsed = start.elapsed();
    info!("[refresh_session_names] 完成: 更新 {} 个名称, 耗时={}ms", updated_count, elapsed.as_millis());
}

/// 启动定时轮询（检测意外退出、刷新名称）
pub fn start_polling(app_handle: tauri::AppHandle) {
    info!("[start_polling] 开始启动轮询服务");

    if POLLING_RUNNING.load(Ordering::SeqCst) {
        warn!("[start_polling] 轮询服务已在运行，跳过启动");
        return;
    }

    info!("[start_polling] 设置运行标志并启动线程（30秒间隔）");
    POLLING_RUNNING.store(true, Ordering::SeqCst);

    let app_handle_clone = app_handle.clone();

    thread::spawn(move || {
        info!("[polling_thread] 轮询线程启动");
        let mut poll_count = 0;

        loop {
            if !POLLING_RUNNING.load(Ordering::SeqCst) {
                info!("[polling_thread] 收到停止信号，退出线程（共轮询 {} 次）", poll_count);
                break;
            }

            thread::sleep(Duration::from_secs(30));
            poll_count += 1;

            let poll_start = Instant::now();
            info!("[polling_thread] 轮询 #{} 开始", poll_count);

            // 获取当前所有 PID
            let pids: Vec<u32> = RUNNING_SESSIONS.lock().unwrap().keys().cloned().collect();
            let session_count = pids.len();

            debug!("[polling_thread] 当前 session 数量: {}", session_count);

            if session_count == 0 {
                debug!("[polling_thread] 无 session，跳过");
                continue;
            }

            // 1. 并行检查进程存活状态
            info!("[polling_thread] 并行检查 {} 个进程", session_count);
            let (_, dead_pids) = check_processes_parallel(pids);

            // 移除已退出的 session
            if !dead_pids.is_empty() {
                info!("[polling_thread] 检测到 {} 个意外退出的 session", dead_pids.len());
                let mut sessions = RUNNING_SESSIONS.lock().unwrap();
                for pid in &dead_pids {
                    info!("[polling_thread] 移除 PID: {}", pid);
                    sessions.remove(pid);
                    // 清除已退出进程的窗口缓存
                    invalidate_window_cache(*pid);
                }
            }

            // 1b. 后台缓存尚未缓存的 session 窗口信息
            {
                let uncached_pids: Vec<u32> = {
                    let sessions = RUNNING_SESSIONS.lock().unwrap();
                    sessions.keys()
                        .filter(|&&pid| get_cached_window(pid).is_none())
                        .cloned()
                        .collect()
                };
                if !uncached_pids.is_empty() {
                    debug!("[polling_thread] 为 {} 个未缓存的 PID 填充窗口缓存", uncached_pids.len());
                    populate_window_cache_parallel(&uncached_pids);
                }
            }

            // 2. 刷新 session 名称（检测窗口标题变化）
            info!("[polling_thread] 刷新 session 名称");
            refresh_session_names();

            // 3. 扫描 away_summary
            info!("[polling_thread] 扫描 away_summary");
            scan_away_summaries();

            // 4. 发送事件通知前端
            let sessions = get_running_sessions();

            if let Err(e) = app_handle_clone.emit("running_sessions_changed", sessions) {
                error!("[polling_thread] 发送事件失败: {}", e);
            } else {
                info!("[polling_thread] 事件发送成功");
            }

            let elapsed = poll_start.elapsed();
            info!("[polling_thread] 轮询 #{} 完成，耗时: {}ms", poll_count, elapsed.as_millis());
        }

        info!("[polling_thread] 轮询线程退出");
    });

    info!("[start_polling] 完成，轮询线程已启动");
}

/// 停止轮询
pub fn stop_polling() {
    info!("[stop_polling] 开始停止轮询服务");
    POLLING_RUNNING.store(false, Ordering::SeqCst);
    info!("[stop_polling] 完成，运行标志已设置为 false");
}

/// 强制扫描单个 session 的 jsonl（忽略缓存）
/// 用于启动时和状态变化时的实时扫描
pub fn scan_session_jsonl_force(pid: u32) {
    debug!("[scan_session_jsonl_force] 强制扫描 session pid={}", pid);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // 获取 session 信息
    let (session_id, cwd) = {
        let sessions = RUNNING_SESSIONS.lock().unwrap();
        match sessions.get(&pid) {
            Some(s) => (s.session_id.clone(), s.cwd.clone()),
            None => {
                warn!("[scan_session_jsonl_force] 未找到 pid={} 的 session", pid);
                return;
            }
        }
    };

    // 扫描 away_summary 和 last_user_input
    let away_result = extract_away_summary(&session_id, &cwd);
    let user_input = extract_last_user_input(&session_id, &cwd);

    // 更新缓存和 RunningSession
    {
        let mut cache = AWAY_SUMMARY_CACHE.lock().unwrap();
        let mut sessions = RUNNING_SESSIONS.lock().unwrap();

        if let Some(session) = sessions.get_mut(&pid) {
            match &away_result {
                Some((content, timestamp)) => {
                    info!("[scan_session_jsonl_force] 发现 away_summary: session={}, length={}", session_id, content.len());
                    session.away_summary = Some(content.clone());
                    session.away_summary_at = Some(timestamp.clone());
                    cache.insert(session_id.clone(), (Some(content.clone()), Some(timestamp.clone()), now));
                }
                None => {
                    debug!("[scan_session_jsonl_force] 无 away_summary: session={}", session_id);
                    session.away_summary = None;
                    session.away_summary_at = None;
                    cache.insert(session_id.clone(), (None, None, now));
                }
            }

            // 更新 last_user_input
            if let Some(input) = user_input {
                debug!("[scan_session_jsonl_force] 更新 last_user_input: session={}", session_id);
                session.last_user_input = Some(input);
            } else {
                session.last_user_input = None;
            }
        }
    }
}

/// 扫描所有 session 的 away_summary（带缓存）
/// 用于定时轮询
pub fn scan_away_summaries() {
    debug!("[scan_away_summaries] 开始扫描");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // 获取所有 session 列表（不管状态）
    let sessions_to_scan: Vec<(u32, String, String)> = RUNNING_SESSIONS
        .lock()
        .unwrap()
        .iter()
        .map(|(pid, session)| (pid.clone(), session.session_id.clone(), session.cwd.clone()))
        .collect();

    debug!("[scan_away_summaries] 需要扫描 {} 个 session", sessions_to_scan.len());

    for (pid, session_id, cwd) in sessions_to_scan {
        // 检查缓存是否有效（最近 60 秒内已扫描）
        let should_scan = {
            let cache = AWAY_SUMMARY_CACHE.lock().unwrap();
            match cache.get(&session_id) {
                Some((_, _, checked_at)) => {
                    // 如果 60 秒内已扫描，跳过
                    now - checked_at > 60
                }
                None => true,
            }
        };

        if !should_scan {
            debug!("[scan_away_summaries] 跳过 {} (缓存有效)", session_id);
            continue;
        }

        // 扫描 away_summary 和 last_user_input
        let away_result = extract_away_summary(&session_id, &cwd);
        let user_input = extract_last_user_input(&session_id, &cwd);

        // 更新缓存和 RunningSession
        {
            let mut cache = AWAY_SUMMARY_CACHE.lock().unwrap();
            let mut sessions = RUNNING_SESSIONS.lock().unwrap();

            if let Some(session) = sessions.get_mut(&pid) {
                match &away_result {
                    Some((content, timestamp)) => {
                        info!("[scan_away_summaries] 发现 away_summary: session={}, length={}", session_id, content.len());
                        session.away_summary = Some(content.clone());
                        session.away_summary_at = Some(timestamp.clone());
                        cache.insert(session_id.clone(), (Some(content.clone()), Some(timestamp.clone()), now));
                    }
                    None => {
                        debug!("[scan_away_summaries] 无 away_summary: session={}", session_id);
                        session.away_summary = None;
                        session.away_summary_at = None;
                        cache.insert(session_id.clone(), (None, None, now));
                    }
                }

                // 更新 last_user_input
                if let Some(input) = user_input {
                    debug!("[scan_away_summaries] 更新 last_user_input: session={}", session_id);
                    session.last_user_input = Some(input);
                } else {
                    session.last_user_input = None;
                }
            }
        }
    }

    debug!("[scan_away_summaries] 完成");
}