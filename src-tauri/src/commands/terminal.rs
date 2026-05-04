use crate::utils::window_manager::{
    find_terminal_window,
    find_window_by_pid,
    activate_window,
    start_terminal_with_resume,
};
use tracing::{info, debug, warn, error};

/// 通过工作目录跳转到终端窗口
#[tauri::command]
pub fn jump_to_terminal(working_directory: String) -> Result<(), String> {
    info!("[jump_to_terminal] 开始，工作目录: {}", working_directory);

    #[cfg(target_os = "windows")]
    {
        debug!("[jump_to_terminal] Windows 平台，查找终端窗口");
        if let Some(hwnd) = find_terminal_window(&working_directory) {
            info!("[jump_to_terminal] 找到窗口，激活");
            activate_window(hwnd)?;
            info!("[jump_to_terminal] 完成");
            Ok(())
        } else {
            warn!("[jump_to_terminal] 未找到对应的终端窗口");
            Err("未找到对应的终端窗口".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        warn!("[jump_to_terminal] 非 Windows 平台，不支持");
        let _ = working_directory;
        Err("仅支持 Windows 平台".to_string())
    }
}

/// 通过进程 ID 精确跳转到终端窗口
#[tauri::command]
pub fn jump_to_terminal_by_pid(process_id: u32) -> Result<(), String> {
    info!("[jump_to_terminal_by_pid] 开始，PID: {}", process_id);

    #[cfg(target_os = "windows")]
    {
        debug!("[jump_to_terminal_by_pid] Windows 平台，查找窗口");
        if let Some(hwnd) = find_window_by_pid(process_id) {
            info!("[jump_to_terminal_by_pid] 找到窗口，激活");
            activate_window(hwnd)?;
            info!("[jump_to_terminal_by_pid] 完成");
            Ok(())
        } else {
            warn!("[jump_to_terminal_by_pid] 未找到进程 {} 对应的终端窗口", process_id);
            Err(format!("未找到进程 {} 对应的终端窗口", process_id))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        warn!("[jump_to_terminal_by_pid] 非 Windows 平台，不支持");
        let _ = process_id;
        Err("仅支持 Windows 平台".to_string())
    }
}

/// 智能跳转：先尝试 PID，失败则尝试路径
#[tauri::command]
pub fn smart_jump_to_terminal(working_directory: String, process_id: Option<u32>) -> Result<(), String> {
    info!("[smart_jump_to_terminal] 开始，工作目录: {}, PID: {:?}",
          working_directory, process_id);

    #[cfg(target_os = "windows")]
    {
        // 先尝试 PID 精确匹配
        if let Some(pid) = process_id {
            if pid > 0 {
                info!("[smart_jump_to_terminal] 尝试 PID 精确匹配: {}", pid);
                if let Some(hwnd) = find_window_by_pid(pid) {
                    info!("[smart_jump_to_terminal] PID 匹配成功，激活窗口");
                    activate_window(hwnd)?;
                    info!("[smart_jump_to_terminal] 完成（通过 PID）");
                    return Ok(());
                }
                warn!("[smart_jump_to_terminal] PID {} 匹配失败，尝试路径匹配", pid);
            } else {
                debug!("[smart_jump_to_terminal] PID 为 0，跳过 PID 匹配");
            }
        } else {
            debug!("[smart_jump_to_terminal] 无 PID，跳过 PID 匹配");
        }

        // PID 匹配失败，尝试路径匹配
        info!("[smart_jump_to_terminal] 尝试路径匹配: {}", working_directory);
        if let Some(hwnd) = find_terminal_window(&working_directory) {
            info!("[smart_jump_to_terminal] 路径匹配成功，激活窗口");
            activate_window(hwnd)?;
            info!("[smart_jump_to_terminal] 完成（通过路径）");
            Ok(())
        } else {
            warn!("[smart_jump_to_terminal] 未找到对应的终端窗口");
            Err("未找到对应的终端窗口".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        warn!("[smart_jump_to_terminal] 非 Windows 平台，不支持");
        let _ = (working_directory, process_id);
        Err("仅支持 Windows 平台".to_string())
    }
}

/// 在终端中恢复 session
#[tauri::command]
pub fn resume_in_terminal(working_directory: String, session_id: String) -> Result<(), String> {
    info!("[resume_in_terminal] 开始，工作目录: {}, session_id: {}", working_directory, session_id);

    let result = start_terminal_with_resume(&working_directory, &session_id);

    match result {
        Ok(_) => {
            info!("[resume_in_terminal] 完成");
            Ok(())
        }
        Err(e) => {
            error!("[resume_in_terminal] 失败: {}", e);
            Err(e)
        }
    }
}