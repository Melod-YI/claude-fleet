use std::process::Command;

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

    data.found_window
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_by_pid_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data = &mut *(lparam.0 as *mut EnumWindowsData);

    // 获取窗口进程 ID
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    // 检查是否是目标 PID 且是可见窗口
    if pid == data.target_pid && IsWindowVisible(hwnd).as_bool() {
        // 还需要检查是否是顶层窗口（有标题）
        let mut title: [u16; 256] = [0; 256];
        let title_len = GetWindowTextW(hwnd, &mut title);

        if title_len > 0 {
            let title_str = String::from_utf16_lossy(&title[..title_len as usize]);

            // 检查是否是 Windows Terminal 或相关终端窗口
            if title_str.contains("Windows Terminal")
                || title_str.contains("Terminal")
                || title_str.contains("Command Prompt")
                || title_str.contains("PowerShell")
                || title_str.contains("claude")
            {
                data.found_window = Some(hwnd);
                return false.into(); // 停止枚举
            }
        }
    }

    true.into()
}

/// 通过工作目录查找 Windows Terminal 窗口（标题匹配）
#[cfg(target_os = "windows")]
pub fn find_terminal_window(working_directory: &str) -> Option<HWND> {
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

        // Windows Terminal 标题格式: "Directory - Windows Terminal"
        // 尝试匹配工作目录（标题中包含目录名）
        if title_str.contains("Windows Terminal") {
            // 检查标题是否包含目标路径
            if title_str.contains(&data.target_path)
                || title_str.contains(&get_last_path_segment(&data.target_path))
            {
                data.found_window = Some(hwnd);
                return false.into(); // 停止枚举
            }

            // 如果还没有找到任何窗口，记录第一个作为备用
            if data.found_window.is_none() {
                data.found_window = Some(hwnd);
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
#[cfg(target_os = "windows")]
pub fn activate_window(hwnd: HWND) -> Result<(), String> {
    unsafe {
        // 先尝试显示窗口
        let _ = ShowWindow(hwnd, SW_SHOW);

        // 尝试设置前台窗口
        let _ = SetForegroundWindow(hwnd);

        // BringWindowToTop 也可能有用
        let _ = BringWindowToTop(hwnd);
    }
    Ok(())
}

/// 非 Windows 平台的备用实现
#[cfg(not(target_os = "windows"))]
pub fn find_window_by_pid(target_pid: u32) -> Option<u64> {
    let _ = target_pid;
    None
}

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