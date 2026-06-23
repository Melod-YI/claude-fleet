use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use once_cell::sync::Lazy;
use tracing::{info, debug, warn};

#[cfg(target_os = "windows")]
use windows::{
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::UI::Input::KeyboardAndMouse::*,
    Win32::System::Diagnostics::ToolHelp::*,
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

/// 执行完整的 PID 链查找并将结果写入缓存
/// 线程安全，可从后台线程调用
#[cfg(target_os = "windows")]
pub fn resolve_and_cache_window(pid: u32) -> Option<HWND> {
    let hwnd = find_window_by_pid_chain(pid)?;

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
        },
    );
    info!(
        "[window_cache] 已缓存 hwnd={} owner_pid={} → session pid={}",
        hwnd.0 as isize, owner_pid, pid
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
