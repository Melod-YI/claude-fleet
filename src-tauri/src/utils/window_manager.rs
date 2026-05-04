use std::process::Command;
use tracing::{info, debug, warn, error};

#[cfg(target_os = "windows")]
use windows::{
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
};

/// 用于传递给 EnumWindows 回调的数据
#[cfg(target_os = "windows")]
struct EnumWindowsData {
    target_pid: u32,
    found_window: Option<HWND>,
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
        info!("[find_window_by_pid] 找到窗口: HWND={}", data.found_window.unwrap().0);
    } else {
        warn!("[find_window_by_pid] 未找到窗口");
    }

    data.found_window
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_by_pid_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data = &mut *(lparam.0 as *mut EnumWindowsData);

    // 获取窗口进程 ID
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    debug!("[enum_windows_by_pid_callback] 检查窗口: HWND={}, PID={} (目标: {})",
           hwnd.0, pid, data.target_pid);

    // 检查是否是目标 PID 且是可见窗口
    if pid == data.target_pid && IsWindowVisible(hwnd).as_bool() {
        debug!("[enum_windows_by_pid_callback] PID 匹配且窗口可见");

        // 还需要检查是否是顶层窗口（有标题）
        let mut title: [u16; 256] = [0; 256];
        let title_len = GetWindowTextW(hwnd, &mut title);

        if title_len > 0 {
            let title_str = String::from_utf16_lossy(&title[..title_len as usize]);
            debug!("[enum_windows_by_pid_callback] 窗口标题: \"{}\"", title_str);

            // 检查是否是 Windows Terminal 或相关终端窗口
            if title_str.contains("Windows Terminal")
                || title_str.contains("Terminal")
                || title_str.contains("Command Prompt")
                || title_str.contains("PowerShell")
                || title_str.contains("claude")
            {
                info!("[enum_windows_by_pid_callback] 找到终端窗口: HWND={}, 标题=\"{}\"", hwnd.0, title_str);
                data.found_window = Some(hwnd);
                return false.into(); // 停止枚举
            } else {
                debug!("[enum_windows_by_pid_callback] 标题不匹配终端类型");
            }
        } else {
            debug!("[enum_windows_by_pid_callback] 窗口无标题，跳过");
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
        debug!("[enum_windows_by_title_callback] 检查窗口: HWND={}, 标题=\"{}\"", hwnd.0, title_str);

        // Windows Terminal 标题格式: "Directory - Windows Terminal"
        // 尝试匹配工作目录（标题中包含目录名）
        if title_str.contains("Windows Terminal") {
            debug!("[enum_windows_by_title_callback] 找到 Windows Terminal 窗口");

            // 检查标题是否包含目标路径
            let path_segment = get_last_path_segment(&data.target_path);

            if title_str.contains(&data.target_path) {
                info!("[enum_windows_by_title_callback] 完整路径匹配: \"{}\" 包含 \"{}\"", title_str, data.target_path);
                data.found_window = Some(hwnd);
                return false.into(); // 停止枚举
            } else if title_str.contains(&path_segment) {
                info!("[enum_windows_by_title_callback] 路径段匹配: \"{}\" 包含 \"{}\"", title_str, path_segment);
                data.found_window = Some(hwnd);
                return false.into(); // 停止枚举
            } else {
                debug!("[enum_windows_by_title_callback] 路径不匹配: 目标=\"{}\", 段=\"{}\"",
                       data.target_path, path_segment);

                // 如果还没有找到任何窗口，记录第一个作为备用
                if data.found_window.is_none() {
                    debug!("[enum_windows_by_title_callback] 记录备用窗口: HWND={}", hwnd.0);
                    data.found_window = Some(hwnd);
                }
            }
        } else {
            debug!("[enum_windows_by_title_callback] 不是 Windows Terminal 窗口");
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
#[cfg(target_os = "windows")]
pub fn activate_window(hwnd: HWND) -> Result<(), String> {
    info!("[activate_window] 开始激活窗口: HWND={}", hwnd.0);

    unsafe {
        // 先尝试显示窗口
        debug!("[activate_window] 调用 ShowWindow(SW_SHOW)");
        let _ = ShowWindow(hwnd, SW_SHOW);

        // 尝试设置前台窗口
        debug!("[activate_window] 调用 SetForegroundWindow");
        let foreground_result = SetForegroundWindow(hwnd);
        if !foreground_result.as_bool() {
            warn!("[activate_window] SetForegroundWindow 失败");
        }

        // BringWindowToTop 也可能有用
        debug!("[activate_window] 调用 BringWindowToTop");
        let _ = BringWindowToTop(hwnd);
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

/// 启动新终端窗口并恢复 session
pub fn start_terminal_with_resume(working_directory: &str, session_id: &str) -> Result<(), String> {
    info!("[start_terminal_with_resume] 开始启动终端，工作目录: {}, session_id: {}", working_directory, session_id);

    #[cfg(target_os = "windows")]
    {
        debug!("[start_terminal_with_resume] Windows 平台，使用 wt 命令");
        let args = [
            "-d", working_directory,
            "claude",
            "--resume", session_id,
        ];
        info!("[start_terminal_with_resume] 命令: wt {}", args.join(" "));

        Command::new("wt")
            .args(args)
            .spawn()
            .map_err(|e| {
                error!("[start_terminal_with_resume] 启动失败: {}", e);
                format!("启动终端失败: {}", e)
            })?;

        info!("[start_terminal_with_resume] 终端启动成功");
    }

    #[cfg(target_os = "macos")]
    {
        debug!("[start_terminal_with_resume] macOS 平台，使用 open 命令");
        info!("[start_terminal_with_resume] 命令: open -a Terminal \"{}\"", working_directory);

        Command::new("open")
            .args(["-a", "Terminal", working_directory])
            .spawn()
            .map_err(|e| {
                error!("[start_terminal_with_resume] 启动失败: {}", e);
                format!("启动终端失败: {}", e)
            })?;

        info!("[start_terminal_with_resume] 终端启动成功");
    }

    #[cfg(target_os = "linux")]
    {
        debug!("[start_terminal_with_resume] Linux 平台，使用 gnome-terminal");
        info!("[start_terminal_with_resume] 命令: gnome-terminal --working-directory=\"{}\" -e \"claude --resume {}\"",
              working_directory, session_id);

        Command::new("gnome-terminal")
            .args([
                "--working-directory", working_directory,
                "-e", format!("claude --resume {}", session_id),
            ])
            .spawn()
            .map_err(|e| {
                error!("[start_terminal_with_resume] 启动失败: {}", e);
                format!("启动终端失败: {}", e)
            })?;

        info!("[start_terminal_with_resume] 终端启动成功");
    }

    Ok(())
}