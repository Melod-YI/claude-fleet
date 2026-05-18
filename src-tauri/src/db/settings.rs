// src-tauri/src/db/settings.rs
// 应用设置 KV 存储

use rusqlite::Result;
use tracing::{info, error};
use crate::db::schema::get_connection;
use std::collections::HashMap;

/// 获取单个设置值
pub fn get_setting(key: &str) -> Result<Option<String>> {
    let conn = get_connection()?;

    let result = conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        [key],
        |row| row.get::<_, String>(0),
    );

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => {
            error!("[get_setting] 查询失败: key={}, error={}", key, e);
            Err(e)
        }
    }
}

/// 设置单个值
pub fn set_setting(key: &str, value: &str) -> Result<()> {
    info!("[set_setting] 设置: key={}, value={}", key, value);
    let conn = get_connection()?;

    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
        [key, value],
    )?;

    Ok(())
}

/// 获取所有设置
pub fn get_all_settings() -> Result<HashMap<String, String>> {
    info!("[get_all_settings] 获取所有设置");
    let conn = get_connection()?;

    let mut stmt = conn.prepare("SELECT key, value FROM app_settings")?;
    let settings = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?.collect::<Result<HashMap<String, String>>>()?;

    info!("[get_all_settings] 共 {} 个设置", settings.len());
    Ok(settings)
}

// Tauri 命令包装

#[tauri::command]
pub fn get_setting_cmd(key: String) -> Result<Option<String>, String> {
    get_setting(&key).map_err(|e| format!("获取设置失败: {}", e))
}

#[tauri::command]
pub fn set_setting_cmd(key: String, value: String) -> Result<(), String> {
    set_setting(&key, &value).map_err(|e| format!("设置失败: {}", e))
}

#[tauri::command]
pub fn get_all_settings_cmd() -> Result<HashMap<String, String>, String> {
    get_all_settings().map_err(|e| format!("获取设置失败: {}", e))
}