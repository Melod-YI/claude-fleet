use crate::utils::window_manager::{
    find_terminal_window,
    activate_window,
    start_terminal_with_resume,
};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

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

#[tauri::command]
pub fn resume_in_terminal(working_directory: String, session_id: String) -> Result<(), String> {
    start_terminal_with_resume(&working_directory, &session_id)
}