use crate::utils::window_manager::{
    find_terminal_window,
    find_window_by_pid,
    activate_window,
    start_terminal_with_resume,
};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

/// 通过工作目录跳转到终端窗口
#[tauri::command]
pub fn jump_to_terminal(working_directory: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(hwnd) = find_terminal_window(&working_directory) {
            activate_window(hwnd)?;
            Ok(())
        } else {
            Err("未找到对应的终端窗口".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = working_directory;
        Err("仅支持 Windows 平台".to_string())
    }
}

/// 通过进程 ID 精确跳转到终端窗口
#[tauri::command]
pub fn jump_to_terminal_by_pid(process_id: u32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(hwnd) = find_window_by_pid(process_id) {
            activate_window(hwnd)?;
            Ok(())
        } else {
            Err(format!("未找到进程 {} 对应的终端窗口", process_id))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = process_id;
        Err("仅支持 Windows 平台".to_string())
    }
}

/// 智能跳转：先尝试 PID，失败则尝试路径
#[tauri::command]
pub fn smart_jump_to_terminal(working_directory: String, process_id: Option<u32>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // 先尝试 PID 精确匹配
        if let Some(pid) = process_id {
            if pid > 0 {
                if let Some(hwnd) = find_window_by_pid(pid) {
                    return activate_window(hwnd);
                }
            }
        }

        // PID 匹配失败，尝试路径匹配
        if let Some(hwnd) = find_terminal_window(&working_directory) {
            activate_window(hwnd)?;
            Ok(())
        } else {
            Err("未找到对应的终端窗口".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (working_directory, process_id);
        Err("仅支持 Windows 平台".to_string())
    }
}

/// 在终端中恢复 session
#[tauri::command]
pub fn resume_in_terminal(working_directory: String, session_id: String) -> Result<(), String> {
    start_terminal_with_resume(&working_directory, &session_id)
}