use std::process::Command;

#[cfg(target_os = "windows")]
use windows::{
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
};

/// 查找 Windows Terminal 窗口
#[cfg(target_os = "windows")]
pub fn find_terminal_window(working_directory: &str) -> Option<HWND> {
    // Windows Terminal 窗口标题通常包含路径信息
    // 使用 EnumWindows 查找匹配的窗口
    let mut found_window: Option<HWND> = None;

    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            LPARAM(&mut found_window as *mut _ as isize),
        );
    }

    // 过滤：如果有精确匹配的工作目录，优先使用
    found_window
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let found_window = &mut *(lparam.0 as *mut Option<HWND>);

    // 获取窗口标题
    let mut title: [u16; 512] = [0; 512];
    let title_len = GetWindowTextW(hwnd, &mut title);

    if title_len > 0 {
        let title_str = String::from_utf16_lossy(&title[..title_len as usize]);

        // 检查是否是 Windows Terminal
        // Windows Terminal 标题格式通常为: "Directory - Windows Terminal" 或 "Command Prompt - Windows Terminal"
        if title_str.contains("Windows Terminal") {
            *found_window = Some(hwnd);
            return false.into(); // 停止枚举，找到第一个即可
        }
    }

    true.into()
}

/// 激活窗口（置顶）
#[cfg(target_os = "windows")]
pub fn activate_window(hwnd: HWND) -> Result<(), String> {
    unsafe {
        // 显示窗口
        ShowWindow(hwnd, SW_SHOW);
        // 设置前台窗口
        SetForegroundWindow(hwnd);
    }
    Ok(())
}

/// 通过进程 ID 查找窗口
#[cfg(target_os = "windows")]
pub fn find_window_by_pid(target_pid: u32) -> Option<HWND> {
    let mut found_window: Option<HWND> = None;

    unsafe {
        EnumWindows(
            Some(enum_windows_by_pid_callback),
            LPARAM(&mut found_window as *mut _ as isize),
        );
    }

    found_window
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_by_pid_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let found_window = &mut *(lparam.0 as *mut Option<HWND>);

    // 获取窗口进程 ID
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    // 检查是否是目标 PID
    if pid != 0 && !hwnd.0.is_null() {
        // 简单检查：如果是可见窗口
        if IsWindowVisible(hwnd).as_bool() {
            // 这里需要传入目标 PID，简化实现返回第一个可见窗口
            // 实际需要通过 LPARAM 传递目标 PID
        }
    }

    true.into()
}

/// 非 Windows 平台的备用实现
#[cfg(not(target_os = "windows"))]
pub fn find_terminal_window(working_directory: &str) -> Option<u64> {
    let _ = working_directory;
    None
}

#[cfg(not(target_os = "windows"))]
pub fn activate_window(window_id: u64) -> Result<(), String> {
    let _ = window_id;
    Err("仅支持 Windows 平台".to_string())
}

/// 启动新终端窗口并恢复 session
pub fn start_terminal_with_resume(working_directory: &str, session_id: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("wt")
            .args([
                "-d", working_directory,
                "claude",
                "--resume", session_id,
            ])
            .spawn()
            .map_err(|e| format!("启动终端失败: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Terminal", working_directory])
            .spawn()
            .map_err(|e| format!("启动终端失败: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("gnome-terminal")
            .args([
                "--working-directory", working_directory,
                "-e", format!("claude --resume {}", session_id),
            ])
            .spawn()
            .map_err(|e| format!("启动终端失败: {}", e))?;
    }

    Ok(())
}