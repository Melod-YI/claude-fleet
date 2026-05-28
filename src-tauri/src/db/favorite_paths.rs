// src-tauri/src/db/favorite_paths.rs
// 常用路径管理

use rusqlite::Result;
use tracing::info;
use crate::db::schema::get_connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoritePath {
    pub path: String,
    pub use_count: i64,
    pub last_used_at: i64,
    pub pinned: bool,
    pub pinned_at: Option<i64>,
}

/// 记录路径使用
pub fn record_path_usage(path: &str) -> Result<()> {
    info!("[record_path_usage] 记录路径使用: {}", path);
    let conn = get_connection()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let existing: Option<(i64, i64)> = conn.query_row(
        "SELECT use_count, last_used_at FROM favorite_paths WHERE path = ?1",
        [path],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    ).ok();

    if let Some((count, _)) = existing {
        conn.execute(
            "UPDATE favorite_paths SET use_count = ?1, last_used_at = ?2 WHERE path = ?3",
            [&(count + 1).to_string(), &now.to_string(), path],
        )?;
    } else {
        conn.execute(
            "INSERT INTO favorite_paths (path, use_count, last_used_at) VALUES (?1, 1, ?2)",
            [path, &now.to_string()],
        )?;
    }

    info!("[record_path_usage] 成功记录");
    Ok(())
}

/// 移除常用路径
pub fn remove_favorite_path(path: &str) -> Result<()> {
    info!("[remove_favorite_path] 移除路径: {}", path);
    let conn = get_connection()?;

    conn.execute(
        "DELETE FROM favorite_paths WHERE path = ?1",
        [path],
    )?;

    Ok(())
}

/// 获取排序后的常用路径
pub fn get_sorted_favorite_paths() -> Result<Vec<FavoritePath>> {
    info!("[get_sorted_favorite_paths] 获取排序后的常用路径");
    let conn = get_connection()?;

    const RECENCY_WEIGHT: f64 = 0.6;
    const FREQUENCY_WEIGHT: f64 = 0.4;
    const RECENCY_DECAY_DAYS: f64 = 30.0;
    const MAX_DISPLAY: usize = 10;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let mut stmt = conn.prepare("SELECT path, use_count, last_used_at FROM favorite_paths")?;

    let paths = stmt.query_map([], |row| {
        Ok(FavoritePath {
            path: row.get::<_, String>(0)?,
            use_count: row.get::<_, i64>(1)?,
            last_used_at: row.get::<_, i64>(2)?,
        })
    })?.collect::<Result<Vec<FavoritePath>>>()?;

    let mut scored: Vec<(FavoritePath, f64)> = paths
        .into_iter()
        .map(|p| {
            let days = (now - p.last_used_at) as f64 / (1000.0 * 60.0 * 60.0 * 24.0);
            let recency = (-days / RECENCY_DECAY_DAYS).exp();
            let freq = (p.use_count as f64 + 1.0).log10() / 100.0_f64.log10();
            (p, recency * RECENCY_WEIGHT + freq * FREQUENCY_WEIGHT)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    Ok(scored.into_iter().take(MAX_DISPLAY).map(|(p, _)| p).collect())
}

/// 切换路径置顶状态
pub fn toggle_pin_path(path: &str) -> Result<FavoritePath> {
    info!("[toggle_pin_path] 切换置顶状态: {}", path);
    let conn = get_connection()?;

    // 查询当前状态
    let current: Option<(bool, Option<i64>)> = conn.query_row(
        "SELECT pinned, pinned_at FROM favorite_paths WHERE path = ?1",
        [path],
        |row| Ok((row.get::<_, i64>(0)? != 0, row.get::<_, Option<i64>>(1)?)),
    ).ok();

    if current.is_none() {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }

    let (is_pinned, _) = current.unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    if is_pinned {
        // 取消置顶
        conn.execute(
            "UPDATE favorite_paths SET pinned = 0, pinned_at = NULL WHERE path = ?1",
            [path],
        )?;
        info!("[toggle_pin_path] 取消置顶: {}", path);
    } else {
        // 置顶
        conn.execute(
            "UPDATE favorite_paths SET pinned = 1, pinned_at = ?1 WHERE path = ?2",
            rusqlite::params![&now, path],
        )?;
        info!("[toggle_pin_path] 置顶: {}", path);
    }

    // 返回更新后的记录
    get_favorite_path_by_path(path)
}

/// 根据 path 获取单个 FavoritePath
fn get_favorite_path_by_path(path: &str) -> Result<FavoritePath> {
    let conn = get_connection()?;
    conn.query_row(
        "SELECT path, use_count, last_used_at, pinned, pinned_at FROM favorite_paths WHERE path = ?1",
        [path],
        |row| Ok(FavoritePath {
            path: row.get::<_, String>(0)?,
            use_count: row.get::<_, i64>(1)?,
            last_used_at: row.get::<_, i64>(2)?,
            pinned: row.get::<_, i64>(3)? != 0,
            pinned_at: row.get::<_, Option<i64>>(4)?,
        }),
    )
}

// Tauri 命令包装

#[tauri::command]
pub fn record_path_usage_cmd(path: String) -> Result<(), String> {
    record_path_usage(&path).map_err(|e| format!("记录路径失败: {}", e))
}

#[tauri::command]
pub fn remove_favorite_path_cmd(path: String) -> Result<(), String> {
    remove_favorite_path(&path).map_err(|e| format!("移除路径失败: {}", e))
}

#[tauri::command]
pub fn get_sorted_favorite_paths_cmd() -> Result<Vec<FavoritePath>, String> {
    get_sorted_favorite_paths().map_err(|e| format!("获取路径失败: {}", e))
}

#[tauri::command]
pub fn toggle_pin_path_cmd(path: String) -> Result<FavoritePath, String> {
    toggle_pin_path(&path).map_err(|e| format!("切换置顶状态失败: {}", e))
}