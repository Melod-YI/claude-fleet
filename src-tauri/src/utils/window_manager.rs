use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use once_cell::sync::Lazy;
use tracing::{info, debug, warn};

#[cfg(target_os = "windows")]
use windows::{
    core::PWSTR,
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::UI::Input::KeyboardAndMouse::*,
    Win32::System::Diagnostics::ToolHelp::*,
    Win32::System::Threading::*,
    Win32::System::Console::*,
};

// --- 窗口 HWND 缓存 ---
// 缓存已解析的窗口句柄，使跳转终端可以跳过昂贵的 PID 链遍历
// （10 次 EnumWindows + 10 次 wmic 子进程 → 2 次 Win32 调用验证）

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
pub struct WindowCacheEntry {
    pub hwnd_raw: isize,      // HWND 的原始值（isize 可安全跨线程传递）
    pub owner_pid: u32,       // 实际拥有窗口的 PID（可能是父进程）
    pub resolved_at: Instant, // 缓存写入时间
    /// 该 HWND 是否为 Windows Terminal 的不可见 pseudo-console 宿主窗口。
    /// 这类窗口无标题、激活需走 activate_console_window（切 tab），
    /// 且 refresh_session_names 应跳过父链标题查询、直接用文件夹名。
    pub is_console_window: bool,
}

#[cfg(target_os = "windows")]
static WINDOW_HWND_CACHE: Lazy<Mutex<HashMap<u32, WindowCacheEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// 从缓存获取指定 PID 的窗口句柄，验证 HWND 仍然有效且归属于同一进程
/// 验证仅需 IsWindow + GetWindowThreadProcessId（微秒级），vs PID 链遍历（200-500ms）
#[cfg(target_os = "windows")]
pub fn get_cached_window(pid: u32) -> Option<HWND> {
    let cache = WINDOW_HWND_CACHE.lock().unwrap();
    let entry = cache.get(&pid)?;

    let hwnd = HWND(entry.hwnd_raw as *mut _);

    unsafe {
        if !IsWindow(hwnd).as_bool() {
            debug!("[window_cache] HWND 失效（IsWindow=false），pid={}", pid);
            return None;
        }
        let mut current_pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut current_pid));
        if current_pid != entry.owner_pid {
            debug!(
                "[window_cache] HWND 归属变化，pid={}（缓存 owner={}，当前={}）",
                pid, entry.owner_pid, current_pid
            );
            return None;
        }
    }

    debug!(
        "[window_cache] 命中，pid={}，hwnd={}，缓存年龄={}ms",
        pid,
        entry.hwnd_raw,
        entry.resolved_at.elapsed().as_millis()
    );
    Some(hwnd)
}

/// 判断缓存的 HWND 是否为 Windows Terminal 的 pseudo-console 宿主窗口。
/// refresh_session_names 据此跳过父链标题查询（pseudo 无标题，且父链会拿到
/// WT 主窗口的活动 tab 标题导致多 session 串扰）。
#[cfg(target_os = "windows")]
pub fn is_cached_console_window(pid: u32) -> bool {
    WINDOW_HWND_CACHE
        .lock()
        .unwrap()
        .get(&pid)
        .map(|e| e.is_console_window)
        .unwrap_or(false)
}

/// 删除指定 PID 的缓存条目（session 移除时调用）
#[cfg(target_os = "windows")]
pub fn invalidate_window_cache(pid: u32) {
    if WINDOW_HWND_CACHE.lock().unwrap().remove(&pid).is_some() {
        info!("[window_cache] 已清除 pid={} 的缓存条目", pid);
    }
}

/// 清空整个缓存（应用重新初始化时调用）
#[cfg(target_os = "windows")]
pub fn clear_window_cache() {
    let mut cache = WINDOW_HWND_CACHE.lock().unwrap();
    let len = cache.len();
    cache.clear();
    info!("[window_cache] 已清空 {} 条缓存", len);
}

// --- console attach 串行锁 ---
// AttachConsole/FreeConsole/GetConsoleWindow 操作调用进程的 console 状态（进程级，
// 同一进程同一时刻只能 attach 到一个 console），故 attach 序列必须跨线程串行。
// 持此锁期间禁止获取任何其它 mutex（如 WINDOW_HWND_CACHE），否则存在 AB-BA 死锁风险。
#[cfg(target_os = "windows")]
static CONSOLE_ATTACH_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// 取进程主可执行文件名（basename，大小写保留）。
/// 用于判断 AttachConsole 返回的 console HWND 是否归属 WindowsTerminal.exe。
/// 仅需 PROCESS_QUERY_LIMITED_INFORMATION（0x1000），对同用户进程普遍可用；
/// 提权或跨会话场景 OpenProcess 失败时返回 None（调用方据此回退父链）。
#[cfg(target_os = "windows")]
fn get_process_image_basename(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 260];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut size as *mut u32,
        );
        let _ = CloseHandle(handle);
        if !ok.is_ok() {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..size as usize]);
        path.rsplit(['\\', '/']).next().map(|s| s.to_string())
    }
}

/// 取 console HWND 的 owner 进程可执行文件名（basename）。
/// 仅需 PROCESS_QUERY_LIMITED_INFORMATION；提权/跨会话 OpenProcess 失败时返回 None。
#[cfg(target_os = "windows")]
fn get_console_owner_name(hwnd: HWND) -> Option<String> {
    let mut owner_pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut owner_pid)) };
    if owner_pid == 0 {
        return None;
    }
    get_process_image_basename(owner_pid)
}

/// 判断 HWND 是否归属 Windows Terminal 进程（其 owner 进程名含 "WindowsTerminal"，
/// 大小写不敏感）。用于区分激活路径（WT pseudo 走 activate_console_window）与
/// refresh_session_names 的标题抖动规避。
#[cfg(target_os = "windows")]
fn is_windows_terminal_window(hwnd: HWND) -> bool {
    get_console_owner_name(hwnd)
        .map(|n| n.to_lowercase().contains("windowsterminal"))
        .unwrap_or(false)
}

/// attach 拿到的 console 窗口是否应被采用（纯决策，便于单测）。
/// - WT pseudo 窗口不可见，但 activate_console_window 经 GetAncestor 取真实 WT 主窗口激活 → 采用
/// - 其余（conhost / cmd 自持 / …）：可见才采用；不可见（wezterm 的 OpenConsole，
///   不响应前台）则丢弃，回退父链
fn should_use_console_window(is_wt: bool, is_visible: bool) -> bool {
    is_wt || is_visible
}

/// AttachConsole + GetConsoleWindow 的核心序列，返回目标进程所在 console 的窗口 HWND
/// （不区分 owner 是 conhost 还是 WindowsTerminal）。失败/无 console 返回 None。
///
/// 串行：attach 序列操作进程级 console 状态，持 CONSOLE_ATTACH_MUTEX 串行执行。
#[cfg(target_os = "windows")]
fn raw_console_window_for_pid(pid: u32) -> Option<HWND> {
    // 不能 AttachConsole 到自身
    if pid == unsafe { GetCurrentProcessId() } {
        return None;
    }

    let hwnd = {
        // 中毒安全恢复：避免持锁线程 panic 后所有调用连锁 panic
        let _guard = CONSOLE_ATTACH_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        // 先清理自身可能残留的 attach（Tauri GUI subsystem 通常无 console，
        // FreeConsole 返回 false 不影响后续）
        unsafe { let _ = FreeConsole(); }
        let attached = unsafe { AttachConsole(pid) };
        if !attached.is_ok() {
            // 失败原因常见：目标进程无 console（Git Bash/pty/wezterm GUI）、目标已退出、跨会话拒绝。
            return None;
        }
        let hwnd = unsafe { GetConsoleWindow() };
        // 立即释放 attach，避免影响本进程后续 attach 及其它线程
        unsafe { let _ = FreeConsole(); }
        hwnd
    };

    // 出锁后再做校验与日志
    if hwnd.0.is_null() {
        debug!("[raw_console_window_for_pid] GetConsoleWindow 返回空，pid={}", pid);
        return None;
    }
    if unsafe { !IsWindow(hwnd).as_bool() } {
        debug!("[raw_console_window_for_pid] console HWND 已失效，pid={}", pid);
        return None;
    }
    Some(hwnd)
}

/// 通过 AttachConsole + GetConsoleWindow 定位进程所在 console 的窗口 HWND。
///
/// 采用判据（`should_use_console_window`）：
/// - owner 为 WindowsTerminal（WT pseudo，不可见但 per-tab 唯一）→ 采用，激活经
///   activate_console_window + GetAncestor 取真实 WT 主窗口并切到正确 tab。
/// - 其余 owner（conhost / cmd 自持 / …）→ **窗口可见才采用**：cmd/conhost 等真实
///   终端窗口可见，直接 activate_window 即可，不再依赖父链（父链需启动 launcher 仍
///   存活，launcher 退出即断）。
/// - 不可见且非 WT（wezterm 的 OpenConsole，不响应前台）→ 丢弃，回退父链解析 wezterm GUI 窗口。
/// - Git Bash（mintty 无真实 console，AttachConsole 失败）→ None，回退父链。
///
/// 串行：attach 序列操作进程级 console 状态，持 CONSOLE_ATTACH_MUTEX 串行执行。
#[cfg(target_os = "windows")]
pub fn find_window_by_console_attach(pid: u32) -> Option<HWND> {
    let hwnd = raw_console_window_for_pid(pid)?;
    let owner_name = get_console_owner_name(hwnd);
    let is_wt = owner_name
        .as_ref()
        .map(|n| n.to_lowercase().contains("windowsterminal"))
        .unwrap_or(false);
    let is_visible = unsafe { IsWindowVisible(hwnd).as_bool() };
    if !should_use_console_window(is_wt, is_visible) {
        debug!(
            "[find_window_by_console_attach] console 窗口丢弃（is_wt={}, visible={}, owner={}），pid={}",
            is_wt, is_visible, owner_name.unwrap_or_default(), pid
        );
        return None;
    }
    info!(
        "[find_window_by_console_attach] 命中 console 窗口（is_wt={}, visible={}, owner={}），pid={}",
        is_wt, is_visible, owner_name.unwrap_or_default(), pid
    );
    Some(hwnd)
}

/// 统一解析入口：优先 attach 快路径（仅 WT），未命中回退父链慢路径。
/// 返回 (HWND, is_console_window)：is_console_window=true 表示是 WT pseudo 窗口，
/// 激活需走 activate_console_window。
#[cfg(target_os = "windows")]
pub fn resolve_window_for_pid(pid: u32) -> Option<(HWND, bool)> {
    if let Some(hwnd) = find_window_by_console_attach(pid) {
        // is_console=true 仅 WT（走 activate_console_window + refresh_session_names 标题规避）；
        // conhost 窗口是普通可见顶层窗口，走 activate_window。
        let is_console = is_windows_terminal_window(hwnd);
        return Some((hwnd, is_console));
    }
    if let Some(hwnd) = find_window_by_pid_chain(pid) {
        return Some((hwnd, false));
    }
    None
}

/// 执行完整的窗口解析并将结果写入缓存
/// 线程安全，可从后台线程调用
#[cfg(target_os = "windows")]
pub fn resolve_and_cache_window(pid: u32) -> Option<HWND> {
    let (hwnd, is_console) = resolve_window_for_pid(pid)?;

    let mut owner_pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut owner_pid));
    }

    WINDOW_HWND_CACHE.lock().unwrap().insert(
        pid,
        WindowCacheEntry {
            hwnd_raw: hwnd.0 as isize,
            owner_pid,
            resolved_at: Instant::now(),
            is_console_window: is_console,
        },
    );
    info!(
        "[window_cache] 已缓存 hwnd={} owner_pid={} is_console={} → session pid={}",
        hwnd.0 as isize, owner_pid, is_console, pid
    );
    Some(hwnd)
}

/// 并行解析多个 PID 的窗口信息并写入缓存
/// 为每个 PID 启动一个独立线程，立即返回（fire-and-forget）
#[cfg(target_os = "windows")]
pub fn populate_window_cache_parallel(pids: &[u32]) {
    info!("[window_cache] 开始并行缓存 {} 个 PID 的窗口信息", pids.len());
    for &pid in pids {
        std::thread::spawn(move || {
            let _ = resolve_and_cache_window(pid);
        });
    }
}

/// 从缓存的 HWND 快速读取窗口标题（单次 GetWindowTextW 调用）
/// 缓存未命中时返回 None
#[cfg(target_os = "windows")]
pub fn get_cached_window_title(pid: u32) -> Option<String> {
    let hwnd = get_cached_window(pid)?;

    unsafe {
        let mut buf: [u16; 256] = [0; 256];
        let len = GetWindowTextW(hwnd, &mut buf);
        if len > 0 {
            let title = String::from_utf16_lossy(&buf[..len as usize]);
            debug!("[window_cache] pid={} 的窗口标题: {}", pid, title);
            return Some(title);
        }
    }
    None
}

/// 用于传递给 EnumWindows 回调的数据
#[cfg(target_os = "windows")]
struct EnumWindowsData {
    target_pid: u32,
    found_window: Option<HWND>,
}

/// 用于获取窗口标题的数据
#[cfg(target_os = "windows")]
struct EnumWindowsTitleData {
    target_pid: u32,
    found_title: Option<String>,
}

/// 获取父进程 PID（使用 Win32 CreateToolhelp32Snapshot API，避免 spawn wmic 进程）
#[cfg(target_os = "windows")]
fn get_parent_pid(pid: u32) -> Option<u32> {
    debug!("[get_parent_pid] 查询 PID {} 的父进程", pid);

    let snapshot = match unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) } {
        Ok(handle) => handle,
        Err(e) => {
            warn!("[get_parent_pid] CreateToolhelp32Snapshot 失败: {}", e);
            return None;
        }
    };

    let mut entry = PROCESSENTRY32 {
        dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
        ..Default::default()
    };

    let mut parent_pid = None;

    if unsafe { Process32First(snapshot, &mut entry) }.is_ok() {
        loop {
            if entry.th32ProcessID == pid {
                parent_pid = Some(entry.th32ParentProcessID);
                info!("[get_parent_pid] PID {} 的父进程是 {}", pid, entry.th32ParentProcessID);
                break;
            }
            if unsafe { Process32Next(snapshot, &mut entry) }.is_err() {
                break;
            }
        }
    }

    let _ = unsafe { CloseHandle(snapshot) };

    if parent_pid.is_none() {
        warn!("[get_parent_pid] 未在进程快照中找到 PID {}", pid);
    }
    parent_pid
}

/// 通过进程 ID 向上查找父进程链，直到找到持有窗口的进程
#[cfg(target_os = "windows")]
pub fn find_window_by_pid_chain(start_pid: u32) -> Option<HWND> {
    info!("[find_window_by_pid_chain] 开始查找，起始 PID: {}", start_pid);

    let mut current_pid = start_pid;
    let mut depth = 0;
    const MAX_DEPTH: u32 = 10;  // 限制查找深度，避免无限循环

    while depth < MAX_DEPTH {
        debug!("[find_window_by_pid_chain] 第 {} 层，检查 PID {}", depth + 1, current_pid);

        // 先尝试直接匹配当前 PID 的窗口
        if let Some(hwnd) = find_window_by_pid(current_pid) {
            info!("[find_window_by_pid_chain] 在第 {} 层找到窗口，PID {}", depth + 1, current_pid);
            return Some(hwnd);
        }

        // 获取父进程 PID
        let parent_pid = match get_parent_pid(current_pid) {
            Some(pid) => pid,
            None => {
                debug!("[find_window_by_pid_chain] 无法获取 PID {} 的父进程，停止查找", current_pid);
                break;
            }
        };

        // 检查是否到达根进程（父进程为自身或 0）
        if parent_pid == 0 || parent_pid == current_pid {
            debug!("[find_window_by_pid_chain] 到达根进程，停止查找");
            break;
        }

        current_pid = parent_pid;
        depth += 1;
    }

    if depth >= MAX_DEPTH {
        warn!("[find_window_by_pid_chain] 达到最大查找深度 {}，停止查找", MAX_DEPTH);
    }

    info!("[find_window_by_pid_chain] 未找到窗口（共检查 {} 层）", depth);
    None
}

/// 通过进程 ID 精确查找窗口
#[cfg(target_os = "windows")]
pub fn find_window_by_pid(target_pid: u32) -> Option<HWND> {
    info!("[find_window_by_pid] 开始查找，PID: {}", target_pid);

    let mut data = EnumWindowsData {
        target_pid,
        found_window: None,
    };

    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_by_pid_callback),
            LPARAM(&mut data as *mut _ as isize),
        );
    }

    if data.found_window.is_some() {
        info!("[find_window_by_pid] 找到窗口: HWND={:p}", data.found_window.unwrap().0);
    } else {
        debug!("[find_window_by_pid] 未找到窗口");
    }

    data.found_window
}

/// 通过进程 ID 获取窗口标题（向上查找父进程链）
#[cfg(target_os = "windows")]
pub fn get_window_title_by_pid_chain(start_pid: u32) -> Option<String> {
    info!("[get_window_title_by_pid_chain] 开始查找，起始 PID: {}", start_pid);

    let mut current_pid = start_pid;
    let mut depth = 0;
    const MAX_DEPTH: u32 = 10;

    while depth < MAX_DEPTH {
        debug!("[get_window_title_by_pid_chain] 第 {} 层，检查 PID {}", depth + 1, current_pid);

        // 尝试获取当前 PID 的窗口标题
        if let Some(title) = get_window_title_by_pid(current_pid) {
            info!("[get_window_title_by_pid_chain] 在第 {} 层找到窗口标题: \"{}\"", depth + 1, title);
            return Some(title);
        }

        // 获取父进程 PID
        let parent_pid = match get_parent_pid(current_pid) {
            Some(pid) => pid,
            None => {
                debug!("[get_window_title_by_pid_chain] 无法获取 PID {} 的父进程，停止查找", current_pid);
                break;
            }
        };

        if parent_pid == 0 || parent_pid == current_pid {
            debug!("[get_window_title_by_pid_chain] 到达根进程，停止查找");
            break;
        }

        current_pid = parent_pid;
        depth += 1;
    }

    info!("[get_window_title_by_pid_chain] 未找到窗口标题（共检查 {} 层）", depth);
    None
}

/// 通过进程 ID 获取窗口标题
#[cfg(target_os = "windows")]
pub fn get_window_title_by_pid(target_pid: u32) -> Option<String> {
    info!("[get_window_title_by_pid] 开始查找，PID: {}", target_pid);

    let mut data = EnumWindowsTitleData {
        target_pid,
        found_title: None,
    };

    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_title_callback),
            LPARAM(&mut data as *mut _ as isize),
        );
    }

    if data.found_title.is_some() {
        info!("[get_window_title_by_pid] 找到窗口标题: \"{}\"", data.found_title.as_ref().unwrap());
    } else {
        debug!("[get_window_title_by_pid] 未找到窗口");
    }

    data.found_title
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_title_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data = &mut *(lparam.0 as *mut EnumWindowsTitleData);

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    if pid == data.target_pid && IsWindowVisible(hwnd).as_bool() {
        let mut title: [u16; 256] = [0; 256];
        let title_len = GetWindowTextW(hwnd, &mut title);

        if title_len > 0 {
            let title_str = String::from_utf16_lossy(&title[..title_len as usize]);
            info!("[enum_windows_title_callback] 找到可见窗口: PID={}, 标题=\"{}\"", pid, title_str);
            data.found_title = Some(title_str);
            return false.into();
        }
    }

    true.into()
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_by_pid_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data = &mut *(lparam.0 as *mut EnumWindowsData);

    // 获取窗口进程 ID
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    // 检查是否是目标 PID 且是可见窗口
    if pid == data.target_pid && IsWindowVisible(hwnd).as_bool() {
        // 检查是否有标题（确保是顶层窗口）
        let mut title: [u16; 256] = [0; 256];
        let title_len = GetWindowTextW(hwnd, &mut title);

        if title_len > 0 {
            let title_str = String::from_utf16_lossy(&title[..title_len as usize]);
            info!("[enum_windows_by_pid_callback] 找到可见窗口: HWND={}, PID={}, 标题=\"{}\"",
                   hwnd.0 as usize, pid, title_str);
            data.found_window = Some(hwnd);
            return false.into(); // 停止枚举，找到第一个有窗口的进程即可
        }
    }

    true.into()
}

/// 通过工作目录查找 Windows Terminal 窗口（标题匹配）
#[cfg(target_os = "windows")]
pub fn find_terminal_window(working_directory: &str) -> Option<HWND> {
    info!("[find_terminal_window] 开始查找，工作目录: {}", working_directory);

    let mut data = FindByTitleData {
        target_path: working_directory.to_string(),
        found_window: None,
    };

    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_by_title_callback),
            LPARAM(&mut data as *mut _ as isize),
        );
    }

    if data.found_window.is_some() {
        info!("[find_terminal_window] 找到窗口");
    } else {
        warn!("[find_terminal_window] 未找到匹配的终端窗口");
    }

    data.found_window
}

/// 用于标题匹配的数据
#[cfg(target_os = "windows")]
struct FindByTitleData {
    target_path: String,
    found_window: Option<HWND>,
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_by_title_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data = &mut *(lparam.0 as *mut FindByTitleData);

    // 获取窗口标题
    let mut title: [u16; 512] = [0; 512];
    let title_len = GetWindowTextW(hwnd, &mut title);

    if title_len > 0 {
        let title_str = String::from_utf16_lossy(&title[..title_len as usize]);
        debug!("[enum_windows_by_title_callback] 检查窗口: HWND={}, 标题=\"{}\"", hwnd.0 as usize, title_str);

        // Windows Terminal 标题格式: "Directory - Windows Terminal"
        if title_str.contains("Windows Terminal") {
            debug!("[enum_windows_by_title_callback] 找到 Windows Terminal 窗口");

            let path_segment = get_last_path_segment(&data.target_path);

            if title_str.contains(&data.target_path) {
                info!("[enum_windows_by_title_callback] 完整路径匹配: \"{}\" 包含 \"{}\"", title_str, data.target_path);
                data.found_window = Some(hwnd);
                return false.into();
            } else if title_str.contains(&path_segment) {
                info!("[enum_windows_by_title_callback] 路径段匹配: \"{}\" 包含 \"{}\"", title_str, path_segment);
                data.found_window = Some(hwnd);
                return false.into();
            } else {
                debug!("[enum_windows_by_title_callback] 路径不匹配: 目标=\"{}\", 段=\"{}\"",
                       data.target_path, path_segment);

                // 记录第一个作为备用
                if data.found_window.is_none() {
                    debug!("[enum_windows_by_title_callback] 记录备用窗口: HWND={}", hwnd.0 as usize);
                    data.found_window = Some(hwnd);
                }
            }
        }
    }

    true.into()
}

/// 获取路径的最后一段
fn get_last_path_segment(path: &str) -> String {
    let parts: Vec<&str> = path.split(|c| c == '\\' || c == '/').filter(|s| !s.is_empty()).collect();
    parts.last().unwrap_or(&path).to_string()
}

/// 激活窗口（置顶）
/// Windows 限制非前台进程直接调用 SetForegroundWindow，需要模拟 Alt 键绕过
#[cfg(target_os = "windows")]
pub fn activate_window(hwnd: HWND) -> Result<(), String> {
    info!("[activate_window] 开始激活窗口: HWND={}", hwnd.0 as usize);

    unsafe {
        // 检查窗口状态，避免破坏最大化状态
        let is_minimized = IsIconic(hwnd).as_bool();
        let is_maximized = IsZoomed(hwnd).as_bool();
        info!("[activate_window] 窗口状态: 最小化={}, 最大化={}", is_minimized, is_maximized);

        // 先显示窗口
        let _ = ShowWindow(hwnd, SW_SHOW);

        // 模拟 Alt 键按下释放，绕过 Windows 的 SetForegroundWindow 限制
        // 这是 Windows 安全机制要求的：只有前台进程或有输入事件的进程才能抢夺焦点
        keybd_event(VK_LMENU.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_LMENU.0 as u8, 0, KEYEVENTF_KEYUP, 0);

        // 短暂延迟让系统处理 Alt 键事件
        std::thread::sleep(std::time::Duration::from_millis(50));

        // 现在应该可以成功设置前台窗口
        let foreground_result = SetForegroundWindow(hwnd);
        if !foreground_result.as_bool() {
            warn!("[activate_window] SetForegroundWindow 失败");
        }

        // 将窗口置顶
        let _ = BringWindowToTop(hwnd);

        // 仅在窗口最小化时恢复，避免破坏最大化状态
        if is_minimized {
            info!("[activate_window] 窗口最小化，恢复显示");
            let _ = ShowWindow(hwnd, SW_RESTORE);
        } else if is_maximized {
            info!("[activate_window] 窗口最大化，保持状态");
            // 最大化窗口不需要 SW_RESTORE，保持最大化
        }
    }

    info!("[activate_window] 完成");
    Ok(())
}

/// 激活 Windows Terminal 的 pseudo-console 宿主窗口（不可见，per-tab 唯一）。
/// 不复用 activate_window：其 ShowWindow(hwnd, SW_SHOW) 会打到不可见 pseudo 窗口，
/// WT <1.14 可能产生 0-size 残影、≥1.14 会重复触发 show 传播。
///
/// 流程：用 GetAncestor(GA_ROOTOWNER) 取真实 WT 顶层窗口，必要时恢复/显示，
/// 再 Alt trick 绕过前台限制 + SetForegroundWindow(pseudo)（WT 路由到正确 tab，
/// 关键：必须对 pseudo 调用而非 root，否则只激活主窗口不切 tab）+ BringWindowToTop。
#[cfg(target_os = "windows")]
pub fn activate_console_window(hwnd: HWND) -> Result<(), String> {
    info!("[activate_console_window] 开始，pseudo HWND={}", hwnd.0 as usize);

    unsafe {
        // 取真实 WT 顶层窗口（pseudo 窗口的 owner 链根）
        let root = GetAncestor(hwnd, GA_ROOTOWNER);
        if !root.0.is_null() {
            let is_minimized = IsIconic(root).as_bool();
            let is_visible = IsWindowVisible(root).as_bool();
            info!(
                "[activate_console_window] root HWND={} minimized={} visible={}",
                root.0 as usize, is_minimized, is_visible
            );
            if is_minimized {
                let _ = ShowWindow(root, SW_RESTORE);
            } else if !is_visible {
                // WT 主窗口非最小化但不可见（隐藏/被遮挡），显式显示
                let _ = ShowWindow(root, SW_SHOW);
            }
        } else {
            debug!("[activate_console_window] 无 root owner，直接激活 pseudo");
        }

        // 模拟 Alt 键绕过 SetForegroundWindow 前台限制
        keybd_event(VK_LMENU.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_LMENU.0 as u8, 0, KEYEVENTF_KEYUP, 0);
        std::thread::sleep(std::time::Duration::from_millis(50));

        // 对 pseudo 调用：WT v1.14+ 传播到主窗口并切到正确 tab
        let fg = SetForegroundWindow(hwnd);
        if !fg.as_bool() {
            warn!("[activate_console_window] SetForegroundWindow 失败");
        }
        let _ = BringWindowToTop(hwnd);
    }

    info!("[activate_console_window] 完成");
    Ok(())
}

/// 取 pseudo console 窗口经 owner 链解析到的真实顶层窗口，仅当该 root 非 null、
/// 不等于 pseudo 自身、可见、且有非空标题时返回。否则 None（调用方回退按 PID 枚举）。
#[cfg(target_os = "windows")]
fn visible_titled_root_owner(hwnd: HWND) -> Option<HWND> {
    unsafe {
        let root = GetAncestor(hwnd, GA_ROOTOWNER);
        if root.0.is_null() || root.0 == hwnd.0 {
            return None;
        }
        if !IsWindowVisible(root).as_bool() {
            return None;
        }
        let mut buf: [u16; 256] = [0; 256];
        if GetWindowTextW(root, &mut buf) <= 0 {
            return None;
        }
        Some(root)
    }
}

/// 非 Windows 平台的备用实现
#[cfg(not(target_os = "windows"))]
pub fn find_window_by_pid(target_pid: u32) -> Option<u64> {
    warn!("[find_window_by_pid] 非 Windows 平台不支持，PID: {}", target_pid);
    let _ = target_pid;
    None
}

#[cfg(not(target_os = "windows"))]
pub fn find_window_by_pid_chain(start_pid: u32) -> Option<u64> {
    warn!("[find_window_by_pid_chain] 非 Windows 平台不支持，PID: {}", start_pid);
    let _ = start_pid;
    None
}

#[cfg(not(target_os = "windows"))]
pub fn get_window_title_by_pid_chain(start_pid: u32) -> Option<String> {
    warn!("[get_window_title_by_pid_chain] 非 Windows 平台不支持，PID: {}", start_pid);
    let _ = start_pid;
    None
}

#[cfg(not(target_os = "windows"))]
pub fn get_window_title_by_pid(target_pid: u32) -> Option<String> {
    warn!("[get_window_title_by_pid] 非 Windows 平台不支持，PID: {}", target_pid);
    let _ = target_pid;
    None
}

#[cfg(not(target_os = "windows"))]
pub fn find_terminal_window(working_directory: &str) -> Option<u64> {
    warn!("[find_terminal_window] 非 Windows 平台不支持，工作目录: {}", working_directory);
    let _ = working_directory;
    None
}

#[cfg(not(target_os = "windows"))]
pub fn activate_window(window_id: u64) -> Result<(), String> {
    warn!("[activate_window] 非 Windows 平台不支持，window_id: {}", window_id);
    let _ = window_id;
    Err("仅支持 Windows 平台".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn invalidate_window_cache(_pid: u32) {}

#[cfg(not(target_os = "windows"))]
pub fn clear_window_cache() {}

#[cfg(not(target_os = "windows"))]
pub fn populate_window_cache_parallel(_pids: &[u32]) {}

#[cfg(not(target_os = "windows"))]
pub fn find_window_by_console_attach(_pid: u32) -> Option<u64> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn resolve_window_for_pid(_pid: u32) -> Option<(u64, bool)> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn is_cached_console_window(_pid: u32) -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn activate_console_window(_window_id: u64) -> Result<(), String> {
    Err("仅支持 Windows 平台".to_string())
}

/// CLI 子命令 `maximize-window` 的实现：在**当前 helper 进程**内最大化所在终端窗口。
///
/// 供 launch 构造的终端命令前置调用（如 `cmd /K "claude-fleet.exe maximize-window && claude ..."`）：
/// helper 进程短命，AttachConsole 污染随进程退出消亡，Tauri 主进程永不调用 AttachConsole，
/// 杜绝 os error 50 与点击陷阱。
///
/// 流程：
/// 1. AttachConsole(ATTACH_PARENT_PROCESS) 挂到父进程（终端进程）的 console。
/// 2. GetConsoleWindow 取 console 窗口 → 解析【可见+有标题】目标（见 resolve_console_target）
///    → ShowWindow(SW_MAXIMIZE)。
/// 3. 始终返回 Ok（best-effort，未命中也不阻塞 claude）。
///
/// 不做父链兜底：cmd/ps/ps7 的 console 路径已全覆盖（WT 宿主走 GetAncestor 取宿主主窗、
/// 经典 conhost 直接用可见 hwnd）；父链兜底对手动开的终端有误最大化 explorer 等风险，且
/// wezterm（原设想用例）已不支持最大化，故不引入。
#[cfg(target_os = "windows")]
pub fn maximize_current_process_window() -> Result<(), String> {
    let pid = unsafe { GetCurrentProcessId() };
    info!("[maximize_current_process_window] 开始，pid={}", pid);

    // attach 父 console → console 路径
    if let Some(target) = resolve_console_target() {
        unsafe {
            let ok = ShowWindow(target, SW_MAXIMIZE).as_bool();
            info!(
                "[maximize_current_process_window] 命中 target={} ShowWindow(SW_MAXIMIZE)={}",
                target.0 as usize, ok
            );
        }
        return Ok(());
    }

    warn!(
        "[maximize_current_process_window] 未命中可见终端窗口，跳过最大化，pid={}",
        pid
    );
    Ok(())
}

/// AttachConsole(ATTACH_PARENT_PROCESS) + GetConsoleWindow 解析【可见且有非空标题】的终端窗口。
///
/// 解析顺序（任一命中即返回）：
/// 1. `visible_titled_root_owner(hwnd)`：GetAncestor(GA_ROOTOWNER) 沿**窗口 owner 链**取
///    可见+有标题的宿主主窗口。WT 宿主下 GetConsoleWindow 返回的是 ConPTY 伪宿主窗口，
///    其 owner 进程报为 cmd/powershell.exe（非 WindowsTerminal），但沿窗口 owner 链
///    GetAncestor 可达真实 WT 主窗口（owner=WindowsTerminal.exe，可见有标题）。
/// 2. console hwnd 本身可见：经典 conhost 下 GetAncestor 返回自身（root==hwnd），
///    此时 conhost 窗口即真实可见终端窗口，直接采用。
/// 3. find_window_by_pid(owner_pid)：枚举 owner 进程的可见+有标题窗口兜底。
///
/// 失败/无可见目标返回 None（调用方走阶段3 父链兜底）。
#[cfg(target_os = "windows")]
fn resolve_console_target() -> Option<HWND> {
    unsafe {
        // 清理自身可能残留的 attach（helper 通常无 console，FreeConsole 无副作用）
        let _ = FreeConsole();
        if AttachConsole(ATTACH_PARENT_PROCESS).is_err() {
            debug!("[resolve_console_target] AttachConsole(父) 失败（父无 console），走父链兜底");
            return None;
        }
        let hwnd = GetConsoleWindow();
        // 立即释放 attach，避免影响后续
        let _ = FreeConsole();

        if hwnd.0.is_null() {
            debug!("[resolve_console_target] GetConsoleWindow 返回空");
            return None;
        }
        // WT 宿主：沿窗口 owner 链取真实 WT 主窗口
        if let Some(root) = visible_titled_root_owner(hwnd) {
            debug!("[resolve_console_target] GetAncestor 命中可见有标题 root");
            return Some(root);
        }
        // 经典 conhost：窗口可见即真实终端窗口，直接采用
        if IsWindowVisible(hwnd).as_bool() {
            debug!("[resolve_console_target] console 窗口可见，直接采用");
            return Some(hwnd);
        }
        // 兜底：枚举 owner 进程的可见+有标题窗口
        let mut owner_pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut owner_pid));
        if owner_pid != 0 {
            if let Some(real) = find_window_by_pid(owner_pid) {
                debug!(
                    "[resolve_console_target] owner_pid={} 兜底枚举命中可见有标题窗口",
                    owner_pid
                );
                return Some(real);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_use_console_window_rule() {
        // WT pseudo（不可见）→ 采用：activate_console_window 经 GetAncestor 激活真实 WT 主窗口
        assert!(should_use_console_window(true, false));
        assert!(should_use_console_window(true, true));
        // conhost / cmd 自持等可见窗口 → 采用
        assert!(should_use_console_window(false, true));
        // wezterm OpenConsole（不可见且非 WT，不响应前台）→ 丢弃回退父链
        assert!(!should_use_console_window(false, false));
    }
}
