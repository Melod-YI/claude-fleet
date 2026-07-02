// src-tauri/src/db/settings.rs
// 应用设置 KV 存储

use rusqlite::{Connection, Result};
use tracing::{info, error};
use crate::db::schema::get_connection;
use std::collections::HashMap;

/// 获取单个设置值
pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
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
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    info!("[set_setting] 设置: key={}, value={}", key, value);
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
        [key, value],
    )?;
    Ok(())
}

/// 删除单个设置值（不存在时视为成功）
pub fn delete_setting(conn: &Connection, key: &str) -> Result<()> {
    info!("[delete_setting] 删除: key={}", key);
    conn.execute("DELETE FROM app_settings WHERE key = ?1", [key])?;
    Ok(())
}

/// 获取所有设置
pub fn get_all_settings(conn: &Connection) -> Result<HashMap<String, String>> {
    info!("[get_all_settings] 获取所有设置");
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
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    get_setting(&conn, &key).map_err(|e| format!("获取设置失败: {}", e))
}

#[tauri::command]
pub fn set_setting_cmd(key: String, value: String) -> Result<(), String> {
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    set_setting(&conn, &key, &value).map_err(|e| format!("设置失败: {}", e))
}

#[tauri::command]
pub fn delete_setting_cmd(key: String) -> Result<(), String> {
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    delete_setting(&conn, &key).map_err(|e| format!("删除设置失败: {}", e))
}

#[tauri::command]
pub fn get_all_settings_cmd() -> Result<HashMap<String, String>, String> {
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    get_all_settings(&conn).map_err(|e| format!("获取设置失败: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE app_settings (
                key TEXT PRIMARY KEY,
                value TEXT
            );"
        ).expect("create table");
        conn
    }

    #[test]
    fn set_get_round_trip() {
        let conn = setup_test_db();
        assert_eq!(get_setting(&conn, "k").unwrap(), None);
        set_setting(&conn, "k", "v").unwrap();
        assert_eq!(get_setting(&conn, "k").unwrap(), Some("v".to_string()));
    }

    #[test]
    fn set_replaces_existing() {
        let conn = setup_test_db();
        set_setting(&conn, "k", "v1").unwrap();
        set_setting(&conn, "k", "v2").unwrap();
        assert_eq!(get_setting(&conn, "k").unwrap(), Some("v2".to_string()));
    }

    #[test]
    fn delete_setting_round_trip() {
        let conn = setup_test_db();
        set_setting(&conn, "k", "v").unwrap();
        assert_eq!(get_setting(&conn, "k").unwrap(), Some("v".to_string()));

        delete_setting(&conn, "k").unwrap();
        assert_eq!(get_setting(&conn, "k").unwrap(), None);

        // 再次删除（不存在）不应报错
        delete_setting(&conn, "k").unwrap();
    }

    #[test]
    fn get_all_settings_returns_map() {
        let conn = setup_test_db();
        set_setting(&conn, "a", "1").unwrap();
        set_setting(&conn, "b", "2").unwrap();

        let all = get_all_settings(&conn).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all.get("a"), Some(&"1".to_string()));
        assert_eq!(all.get("b"), Some(&"2".to_string()));
    }
}
