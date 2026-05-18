// src-tauri/src/db/settings.rs
// 应用设置 KV 存储

use rusqlite::Result;

/// 获取设置值
pub fn get_setting(_key: &str) -> Result<Option<String>> {
    // TODO: 实现
    Ok(None)
}

/// 设置值
pub fn set_setting(_key: &str, _value: &str) -> Result<()> {
    // TODO: 实现
    Ok(())
}

/// 获取所有设置
pub fn get_all_settings() -> Result<Vec<(String, String)>> {
    // TODO: 实现
    Ok(vec![])
}