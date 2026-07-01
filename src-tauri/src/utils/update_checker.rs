// src-tauri/src/utils/update_checker.rs
// GitHub Release 更新检测
//
// 周期性请求 GitHub Releases API，比较最新正式 release 与当前版本，
// 发现新版本时写入全局状态并向前端 emit 事件。

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tracing::{info, warn};

/// 对外暴露（前端 + 命令）的更新信息
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// 最新版本号（如 "0.9.0"，不含 v 前缀）
    pub latest_version: String,
    /// GitHub Release 页面 URL
    pub release_url: String,
    /// Release notes（markdown 原文，可能为空）
    pub release_notes: Option<String>,
    /// 发布时间（ISO 8601 字符串）
    pub published_at: String,
}

/// GitHub Releases API 的原始响应子集
#[derive(Debug, Deserialize)]
struct RawRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    prerelease: bool,
    published_at: String,
}

const RELEASES_API: &str = "https://api.github.com/repos/Melod-YI/claude-fleet/releases/latest";

/// 全局状态：检测到的最新更新信息（None 表示无更新或尚未检测）
static STATE: Lazy<Mutex<Option<UpdateInfo>>> = Lazy::new(|| Mutex::new(None));

/// 解析 GitHub Releases API 的 JSON 响应。
/// 若为预发布版本，返回 None。
pub fn parse_latest_release(json: &str) -> Option<RawRelease> {
    let raw: RawRelease = serde_json::from_str(json).ok()?;
    if raw.prerelease {
        return None;
    }
    Some(raw)
}

/// 比较版本号，判断 latest 是否比 current 更新。
///
/// 输入形如 "0.8.2" 或 "v0.9.0"，按 major.minor.patch 数值比较。
/// 解析失败时按字符串比较兜底。
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    fn parse(v: &str) -> Option<(u64, u64, u64)> {
        let v = v.trim_start_matches('v').trim_start_matches('V');
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some((
            parts[0].parse::<u64>().ok()?,
            parts[1].parse::<u64>().ok()?,
            parts[2].parse::<u64>().ok()?,
        ))
    }

    match (parse(current), parse(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => latest.trim_start_matches('v') > current.trim_start_matches('v'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_patch() {
        assert!(is_newer_version("0.8.2", "0.8.3"));
    }

    #[test]
    fn newer_minor() {
        assert!(is_newer_version("0.8.2", "0.9.0"));
    }

    #[test]
    fn newer_major() {
        assert!(is_newer_version("0.8.2", "1.0.0"));
    }

    #[test]
    fn equal_is_not_newer() {
        assert!(!is_newer_version("0.8.2", "0.8.2"));
    }

    #[test]
    fn older_is_not_newer() {
        assert!(!is_newer_version("0.9.0", "0.8.2"));
    }

    #[test]
    fn handles_v_prefix() {
        assert!(is_newer_version("v0.8.2", "v0.9.0"));
        assert!(is_newer_version("0.8.2", "v0.9.0"));
    }

    #[test]
    fn double_digit_segments() {
        // 0.8.10 应大于 0.8.2（数值比较，非字符串）
        assert!(is_newer_version("0.8.2", "0.8.10"));
        assert!(!is_newer_version("0.8.10", "0.8.9"));
    }

    const SAMPLE_RELEASE_JSON: &str = r###"{
        "tag_name": "v0.9.0",
        "html_url": "https://github.com/Melod-YI/claude-fleet/releases/tag/v0.9.0",
        "body": "## 新功能\n- 更新检测",
        "prerelease": false,
        "published_at": "2026-07-01T10:00:00Z"
    }"###;

    #[test]
    fn parse_extracts_fields() {
        let raw = parse_latest_release(SAMPLE_RELEASE_JSON).expect("应解析成功");
        assert_eq!(raw.tag_name, "v0.9.0");
        assert_eq!(
            raw.html_url,
            "https://github.com/Melod-YI/claude-fleet/releases/tag/v0.9.0"
        );
        assert_eq!(raw.body.as_deref(), Some("## 新功能\n- 更新检测"));
        assert!(!raw.prerelease);
        assert_eq!(raw.published_at, "2026-07-01T10:00:00Z");
    }

    #[test]
    fn parse_filters_prerelease() {
        let json = r#"{
            "tag_name": "v0.9.0-beta",
            "html_url": "https://example.com",
            "body": null,
            "prerelease": true,
            "published_at": "2026-07-01T10:00:00Z"
        }"#;
        assert!(parse_latest_release(json).is_none());
    }

    #[test]
    fn parse_invalid_json_returns_none() {
        assert!(parse_latest_release("not json").is_none());
    }
}

/// 获取当前应用版本字符串（如 "0.8.2"）。
fn current_version(app: &AppHandle) -> String {
    app.package_info().version.to_string()
}

/// 请求 GitHub，比较版本，更新全局状态。
/// 发现新版本时 emit("update_available", UpdateInfo)。
/// 任何错误只 warn 日志，不影响应用。
pub async fn check_for_updates(app: AppHandle) {
    let version = current_version(&app);
    let user_agent = format!("claude-fleet/{}", version);
    info!("[update_checker] 开始检查更新，当前版本: {}", version);

    let result = tauri::async_runtime::spawn_blocking(move || {
        let resp = ureq::get(RELEASES_API)
            .set("User-Agent", &user_agent)
            .call()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let body = resp.into_string()?;
        let raw = parse_latest_release(&body)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "无正式 release"))?;
        Ok::<_, std::io::Error>(raw)
    })
    .await;

    let raw = match result {
        Ok(Ok(raw)) => raw,
        Ok(Err(e)) => {
            warn!("[update_checker] 检查失败: {}", e);
            return;
        }
        Err(e) => {
            warn!("[update_checker] 检查任务异常: {}", e);
            return;
        }
    };

    let latest_version = raw.tag_name.trim_start_matches('v').to_string();

    if !is_newer_version(&version, &latest_version) {
        info!("[update_checker] 当前已是最新: {}", version);
        // 当前版本不落后：清除状态（例如用户升级后首次运行）
        if let Ok(mut st) = STATE.lock() {
            *st = None;
        }
        return;
    }

    let info = UpdateInfo {
        latest_version: latest_version.clone(),
        release_url: raw.html_url,
        release_notes: raw.body,
        published_at: raw.published_at,
    };
    info!("[update_checker] 发现新版本: {}", latest_version);

    if let Ok(mut st) = STATE.lock() {
        *st = Some(info.clone());
    }
    if let Err(e) = app.emit("update_available", &info) {
        warn!("[update_checker] 发送 update_available 事件失败: {}", e);
    }
}

/// 读取当前更新状态（供命令层调用）。
pub fn get_status() -> Option<UpdateInfo> {
    STATE.lock().ok().and_then(|st| st.clone())
}

/// 启动后台更新检测循环：启动后延迟 10s 检查一次，之后每 6h 检查一次。
/// 在 setup() 中调用。
pub fn start_update_loop(app: AppHandle) {
    info!("[update_checker] 启动更新检测循环，间隔 6h");
    tauri::async_runtime::spawn(async move {
        // 启动后延迟 10s，避免与初始化抢资源
        tauri::async_runtime::spawn_blocking(|| {
            std::thread::sleep(std::time::Duration::from_secs(10));
        })
        .await
        .ok();

        loop {
            check_for_updates(app.clone()).await;

            // 每 6 小时检查一次
            tauri::async_runtime::spawn_blocking(|| {
                std::thread::sleep(std::time::Duration::from_secs(6 * 60 * 60));
            })
            .await
            .ok();
        }
    });
}
