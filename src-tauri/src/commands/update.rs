use tracing::{info, warn};

use crate::utils::update_checker;

/// 读取当前更新状态（无更新返回 null）。
#[tauri::command]
pub fn get_update_status() -> Option<update_checker::UpdateInfo> {
    update_checker::get_status()
}

/// 在默认浏览器中打开 release 页面。
#[tauri::command]
pub fn open_release_page(url: String) -> Result<(), String> {
    info!("[open_release_page] 打开: {}", url);

    #[cfg(target_os = "windows")]
    {
        // cmd /C start "" "<url>" ：用默认浏览器打开 URL
        let mut cmd = crate::utils::process::command("cmd");
        cmd.args(["/C", "start", "", &url]);
        crate::utils::process::spawn(&mut cmd)
            .map_err(|e| {
                warn!("[open_release_page] 打开失败: {}", e);
                format!("打开下载页面失败: {}", e)
            })?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        warn!("[open_release_page] 非 Windows 平台，不支持");
        let _ = url;
        Err("仅支持 Windows 平台".to_string())
    }
}
