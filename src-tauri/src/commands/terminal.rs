use std::process::Command;
use crate::utils::window_manager::{
    find_terminal_window,
    find_window_by_pid_chain,
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

/// 通过进程 ID 精确跳转到终端窗口（使用父进程链查找）
#[tauri::command]
pub fn jump_to_terminal_by_pid(process_id: u32) -> Result<(), String> {
    info!("[jump_to_terminal_by_pid] 开始，PID: {}", process_id);

    #[cfg(target_os = "windows")]
    {
        use crate::utils::window_manager::{get_cached_window, resolve_and_cache_window};

        // 快速路径：从缓存获取 HWND（微秒级验证）
        if let Some(hwnd) = get_cached_window(process_id) {
            info!("[jump_to_terminal_by_pid] 缓存命中，pid={}", process_id);
            activate_window(hwnd)?;
            info!("[jump_to_terminal_by_pid] 完成（缓存命中）");
            return Ok(());
        }

        // 慢速路径：缓存未命中，执行完整的 PID 链查找
        debug!("[jump_to_terminal_by_pid] 缓存未命中，执行 PID 链查找");
        if let Some(hwnd) = find_window_by_pid_chain(process_id) {
            // 顺便更新缓存，下次跳转可直接命中
            let _ = resolve_and_cache_window(process_id);
            info!("[jump_to_terminal_by_pid] 找到窗口，激活");
            activate_window(hwnd)?;
            info!("[jump_to_terminal_by_pid] 完成（PID 链查找）");
            Ok(())
        } else {
            warn!("[jump_to_terminal_by_pid] 未找到进程 {} 或其父进程对应的终端窗口", process_id);
            Err(format!("未找到进程 {} 或其父进程对应的终端窗口", process_id))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        warn!("[jump_to_terminal_by_pid] 非 Windows 平台，不支持");
        let _ = process_id;
        Err("仅支持 Windows 平台".to_string())
    }
}

/// 智能跳转：通过 PID 链向上查找父进程直到找到有窗口的进程
#[tauri::command]
pub fn smart_jump_to_terminal(working_directory: String, process_id: Option<u32>) -> Result<(), String> {
    info!("[smart_jump_to_terminal] 开始，工作目录: {}, PID: {:?}",
          working_directory, process_id);

    #[cfg(target_os = "windows")]
    {
        use crate::utils::window_manager::{get_cached_window, resolve_and_cache_window};

        if let Some(pid) = process_id {
            if pid > 0 {
                // 快速路径：从缓存获取 HWND（微秒级验证）
                if let Some(hwnd) = get_cached_window(pid) {
                    info!("[smart_jump_to_terminal] 缓存命中，pid={}", pid);
                    activate_window(hwnd)?;
                    info!("[smart_jump_to_terminal] 完成（缓存命中）");
                    return Ok(());
                }

                // 慢速路径：缓存未命中，执行完整的 PID 链查找
                info!("[smart_jump_to_terminal] 缓存未命中，执行 PID 链查找: {}", pid);
                if let Some(hwnd) = find_window_by_pid_chain(pid) {
                    // 顺便更新缓存，下次跳转可直接命中
                    let _ = resolve_and_cache_window(pid);
                    info!("[smart_jump_to_terminal] PID 链查找成功，激活窗口");
                    activate_window(hwnd)?;
                    info!("[smart_jump_to_terminal] 完成（通过 PID 链）");
                    return Ok(());
                }
                warn!("[smart_jump_to_terminal] PID {} 链查找失败，未找到窗口", pid);
                return Err(format!("未找到进程 {} 或其父进程对应的终端窗口", pid));
            } else {
                warn!("[smart_jump_to_terminal] PID 为 0，无法查找");
                return Err("无效的进程 ID".to_string());
            }
        } else {
            warn!("[smart_jump_to_terminal] 无 PID 信息");
            return Err("缺少进程 ID 信息".to_string());
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
pub fn resume_in_terminal(working_directory: String, session_id: String, terminal_type: String) -> Result<(), String> {
    info!("[resume_in_terminal] 开始，工作目录: {}, session_id: {}, 终端: {}",
          working_directory, session_id, terminal_type);

    let result = start_terminal_with_resume(&working_directory, &session_id, &terminal_type);

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

/// 打开目录（Windows 用 explorer）
#[tauri::command]
pub fn open_directory(path: String) -> Result<(), String> {
    info!("[open_directory] 开始，路径: {}", path);

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}。请确保路径存在且有效", e))?;
        info!("[open_directory] 完成");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        warn!("[open_directory] 非 Windows 平台，不支持");
        let _ = path;
        Err("仅支持 Windows 平台".to_string())
    }
}

/// 在 VSCode 中打开目录
#[tauri::command]
pub fn open_in_vscode(path: String) -> Result<(), String> {
    info!("[open_in_vscode] 开始，路径: {}", path);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        // CREATE_NO_WINDOW = 0x08000000，完全隐藏进程窗口
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // 使用 cmd.exe 执行 code 命令，确保能找到 PATH 中的 code
        // code 命令会启动 VSCode 后自动退出
        Command::new("cmd.exe")
            .args(["/C", "code", &path])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("打开 VSCode 失败: {}。请确保 VSCode 已安装且 'code' 命令在 PATH 中", e))?;
        info!("[open_in_vscode] 完成");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        warn!("[open_in_vscode] 非 Windows 平台，不支持");
        let _ = path;
        Err("仅支持 Windows 平台".to_string())
    }
}