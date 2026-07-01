// src-tauri/src/utils/update_checker.rs
// GitHub Release 更新检测
//
// 周期性请求 GitHub Releases API，比较最新正式 release 与当前版本，
// 发现新版本时写入全局状态并向前端 emit 事件。

use tracing::{info, warn};

/// 比较版本号，判断 latest 是否比 current 更新。
///
/// 输入形如 "0.8.2" 或 "v0.9.0"，按 major.minor.patch 数值比较。
/// 解析失败时按字符串比较兜底。
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    todo!("Task 2 实现")
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
}
