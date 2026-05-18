# Claude Fleet 功能增强实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现三个功能（Session 命名、Workspace 分组、对话截断修复）并重构存储系统到 SQLite。

**Architecture:** 
- 后端新增 `db` 模块处理 SQLite 数据库，包含 schema、CRUD 操作和 localStorage 迁移逻辑
- 前端移除 Zustand persist，改为通过 Tauri invoke 调用后端 API
- Session 命名在显示层合并 custom_name 到现有 title 字段

**Tech Stack:** Rust (rusqlite), TypeScript (Zustand), Tauri 2.0, React

---

## 文件结构

### 后端新增文件
```
src-tauri/src/
  db/
    mod.rs           -- 数据库模块入口，导出所有子模块
    schema.rs        -- 表结构定义、初始化、获取数据库连接
    sessions_meta.rs -- sessions_meta 表 CRUD
    favorites.rs     -- favorites 表 CRUD
    favorite_paths.rs -- favorite_paths 表 CRUD
    settings.rs      -- app_settings 表 CRUD
    migration.rs     -- localStorage 迁移逻辑
```

### 后端修改文件
```
src-tauri/
  Cargo.toml                 -- 添加 rusqlite 依赖
  src/lib.rs                 -- 注册新命令，setup 中调用迁移
  src/utils/session_types.rs -- SessionMeta 增加 custom_name 字段
  src/utils/running_sessions.rs -- RunningSession 增加 custom_name 字段
  src/commands/session_commands.rs -- list_sessions_optimized 增加 custom_name 查询
  src/commands/session.rs    -- list_running 增加 custom_name 查询
```

### 前端修改文件
```
src/
  types/session.ts           -- SessionMeta 增加 customName 字段
  stores/favoriteStore.ts    -- 移除 persist，改为调用后端
  stores/settingsStore.ts    -- 移除 persist，改为调用后端
  components/management/SessionDetail.tsx -- 修复截断 + 添加编辑名称
  components/management/SessionListItem.tsx -- 添加双击/右键编辑名称
  components/running/SessionCard.tsx -- SessionCardNew 添加双击/右键编辑名称
  services/index.ts          -- 新增 db 服务导出
```

### 前端新增文件
```
src/
  services/dbService.ts      -- 数据库操作服务封装
  components/management/GroupedSessionList.tsx -- 分组视图容器
  components/management/WorkspaceGroupItem.tsx -- workspace 分组项
  components/common/EditableName.tsx -- 可编辑名称组件（复用）
  components/common/ContextMenu.tsx -- 右键菜单组件
```

---

## Phase 1: SQLite 数据库基础设施

### Task 1.1: 添加 rusqlite 依赖

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 添加 rusqlite 依赖到 Cargo.toml**

在 `[dependencies]` 部分添加：

```toml
rusqlite = { version = "0.32", features = ["bundled"] }
```

- [ ] **Step 2: 运行 cargo check 验证依赖**

Run: `cd src-tauri && cargo check`
Expected: 编译成功，无错误

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore: 添加 rusqlite SQLite 数据库依赖"
```

---

### Task 1.2: 创建数据库模块结构和 schema

**Files:**
- Create: `src-tauri/src/db/mod.rs`
- Create: `src-tauri/src/db/schema.rs`

- [ ] **Step 1: 创建 db/mod.rs 模块入口**

```rust
// src-tauri/src/db/mod.rs

pub mod schema;
pub mod sessions_meta;
pub mod favorites;
pub mod favorite_paths;
pub mod settings;
pub mod migration;

// 导出常用类型和函数
pub use schema::{get_db_path, init_database, get_connection};
pub use sessions_meta::{set_session_name, get_session_name, delete_session_name};
pub use favorites::{add_favorite, remove_favorite, is_favorite, get_all_favorites};
pub use favorite_paths::{record_path_usage, remove_favorite_path, get_sorted_favorite_paths};
pub use settings::{get_setting, set_setting, get_all_settings};
pub use migration::migrate_from_localstorage;
```

- [ ] **Step 2: 创建 db/schema.rs 表结构定义**

```rust
// src-tauri/src/db/schema.rs

use rusqlite::{Connection, Result};
use std::path::PathBuf;
use tracing::{info, error};

/// 获取数据库文件路径 ~/.claude-fleet/data/claude-fleet.db
pub fn get_db_path() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户目录")
        .join(".claude-fleet")
        .join("data")
        .join("claude-fleet.db")
}

/// 获取数据库连接
pub fn get_connection() -> Result<Connection> {
    let db_path = get_db_path();
    
    // 确保 data 目录存在
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| rusqlite::Error::InvalidPath(parent.to_path_buf()))?;
            info!("[get_connection] 创建数据目录: {}", parent.display());
        }
    }
    
    Connection::open(&db_path)
}

/// 初始化数据库表结构
pub fn init_database() -> Result<()> {
    info!("[init_database] 开始初始化数据库");
    let conn = get_connection()?;
    
    // Session 自定义名称表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions_meta (
            session_id    TEXT PRIMARY KEY,
            custom_name   TEXT,
            created_at    INTEGER,
            updated_at    INTEGER
        )",
        [],
    )?;
    
    // 收藏列表表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS favorites (
            session_id    TEXT PRIMARY KEY,
            added_at      INTEGER
        )",
        [],
    )?;
    
    // 常用路径表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS favorite_paths (
            path          TEXT PRIMARY KEY,
            use_count     INTEGER DEFAULT 1,
            last_used_at  INTEGER
        )",
        [],
    )?;
    
    // 应用设置表（KV 存储）
    conn.execute(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key           TEXT PRIMARY KEY,
            value         TEXT
        )",
        [],
    )?;
    
    info!("[init_database] 数据库初始化完成，路径: {}", get_db_path().display());
    Ok(())
}
```

- [ ] **Step 3: 在 src-tauri/src/utils/mod.rs 中添加 db 模块声明**

在 `src-tauri/src/utils/mod.rs` 末尾添加：

```rust
// 注意：db 模块在 src-tauri/src/ 下，不在 utils/ 下
// 所以需要在 lib.rs 中声明，此处无需修改
```

- [ ] **Step 4: 在 lib.rs 中添加 db 模块声明**

在 `src-tauri/src/lib.rs` 顶部 `mod utils;` 后添加：

```rust
mod utils;
mod commands;
mod db;  // 新增
```

- [ ] **Step 5: 运行 cargo check 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 编译成功

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/db/mod.rs src-tauri/src/db/schema.rs src-tauri/src/lib.rs
git commit -m "feat(db): 创建 SQLite 数据库模块结构和 schema"
```

---

### Task 1.3: 实现 sessions_meta CRUD

**Files:**
- Create: `src-tauri/src/db/sessions_meta.rs`

- [ ] **Step 1: 创建 sessions_meta.rs CRUD 实现**

```rust
// src-tauri/src/db/sessions_meta.rs

use rusqlite::{Connection, Result};
use tracing::{info, error};
use crate::db::schema::get_connection;

/// 设置 session 自定义名称
pub fn set_session_name(session_id: &str, name: &str) -> Result<()> {
    info!("[set_session_name] 设置名称: session_id={}, name={}", session_id, name);
    let conn = get_connection()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    
    // 使用 INSERT OR REPLACE（upsert）
    conn.execute(
        "INSERT OR REPLACE INTO sessions_meta (session_id, custom_name, created_at, updated_at)
         VALUES (?1, ?2, COALESCE((SELECT created_at FROM sessions_meta WHERE session_id = ?1), ?3), ?3)",
        [session_id, name, &now.to_string()],
    )?;
    
    info!("[set_session_name] 成功设置名称");
    Ok(())
}

/// 获取 session 自定义名称
pub fn get_session_name(session_id: &str) -> Result<Option<String>> {
    let conn = get_connection()?;
    
    let result = conn.query_row(
        "SELECT custom_name FROM sessions_meta WHERE session_id = ?1",
        [session_id],
        |row| row.get::<_, Option<String>>(0),
    );
    
    match result {
        Ok(name) => {
            info!("[get_session_name] 查询结果: session_id={}, name={:?}", session_id, name);
            Ok(name)
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => {
            error!("[get_session_name] 查询失败: {}", e);
            Err(e)
        }
    }
}

/// 删除 session 自定义名称
pub fn delete_session_name(session_id: &str) -> Result<()> {
    info!("[delete_session_name] 删除名称: session_id={}", session_id);
    let conn = get_connection()?;
    
    conn.execute(
        "DELETE FROM sessions_meta WHERE session_id = ?1",
        [session_id],
    )?;
    
    info!("[delete_session_name] 成功删除名称");
    Ok(())
}

/// 批量获取多个 session 的自定义名称
pub fn get_session_names(session_ids: &[String]) -> Result<Vec<(String, Option<String>)>> {
    if session_ids.is_empty() {
        return Ok(Vec::new());
    }
    
    let conn = get_connection()?;
    let mut results = Vec::new();
    
    for session_id in session_ids {
        let name = conn.query_row(
            "SELECT custom_name FROM sessions_meta WHERE session_id = ?1",
            [session_id],
            |row| row.get::<_, Option<String>>(0),
        ).ok().flatten();
        results.push((session_id.clone(), name));
    }
    
    Ok(results)
}
```

- [ ] **Step 2: 运行 cargo check**

Run: `cd src-tauri && cargo check`
Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/db/sessions_meta.rs
git commit -m "feat(db): 实现 sessions_meta 表 CRUD 操作"
```

---

### Task 1.4: 实现 favorites CRUD

**Files:**
- Create: `src-tauri/src/db/favorites.rs`

- [ ] **Step 1: 创建 favorites.rs CRUD 实现**

```rust
// src-tauri/src/db/favorites.rs

use rusqlite::{Connection, Result};
use tracing::{info, error};
use crate::db::schema::get_connection;

/// 添加收藏
pub fn add_favorite(session_id: &str) -> Result<()> {
    info!("[add_favorite] 添加收藏: session_id={}", session_id);
    let conn = get_connection()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    
    conn.execute(
        "INSERT OR IGNORE INTO favorites (session_id, added_at) VALUES (?1, ?2)",
        [session_id, &now.to_string()],
    )?;
    
    info!("[add_favorite] 成功添加收藏");
    Ok(())
}

/// 移除收藏
pub fn remove_favorite(session_id: &str) -> Result<()> {
    info!("[remove_favorite] 移除收藏: session_id={}", session_id);
    let conn = get_connection()?;
    
    conn.execute(
        "DELETE FROM favorites WHERE session_id = ?1",
        [session_id],
    )?;
    
    info!("[remove_favorite] 成功移除收藏");
    Ok(())
}

/// 检查是否收藏
pub fn is_favorite(session_id: &str) -> Result<bool> {
    let conn = get_connection()?;
    
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM favorites WHERE session_id = ?1",
        [session_id],
        |row| row.get(0),
    )?;
    
    Ok(count > 0)
}

/// 获取所有收藏的 session ID
pub fn get_all_favorites() -> Result<Vec<String>> {
    info!("[get_all_favorites] 获取所有收藏");
    let conn = get_connection()?;
    
    let mut stmt = conn.prepare("SELECT session_id FROM favorites ORDER BY added_at DESC")?;
    let session_ids = stmt.query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<String>>>()?;
    
    info!("[get_all_favorites] 共 {} 个收藏", session_ids.len());
    Ok(session_ids)
}
```

- [ ] **Step 2: 运行 cargo check**

Run: `cd src-tauri && cargo check`
Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/db/favorites.rs
git commit -m "feat(db): 实现 favorites 表 CRUD 操作"
```

---

### Task 1.5: 实现 favorite_paths CRUD

**Files:**
- Create: `src-tauri/src/db/favorite_paths.rs`

- [ ] **Step 1: 创建 favorite_paths.rs CRUD 实现**

```rust
// src-tauri/src/db/favorite_paths.rs

use rusqlite::{Connection, Result, Row};
use tracing::{info};
use crate::db::schema::get_connection;
use serde::{Deserialize, Serialize};

/// 常用路径数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoritePath {
    pub path: String,
    pub use_count: i64,
    pub last_used_at: i64,
}

/// 记录路径使用
pub fn record_path_usage(path: &str) -> Result<()> {
    info!("[record_path_usage] 记录路径使用: {}", path);
    let conn = get_connection()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    
    // 检查是否已存在
    let existing: Option<(i64, i64)> = conn.query_row(
        "SELECT use_count, last_used_at FROM favorite_paths WHERE path = ?1",
        [path],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    ).ok();
    
    if let Some((count, _)) = existing {
        // 更新：增加计数和时间
        conn.execute(
            "UPDATE favorite_paths SET use_count = ?1, last_used_at = ?2 WHERE path = ?3",
            [&(count + 1).to_string(), &now.to_string(), path],
        )?;
    } else {
        // 新增
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
    
    info!("[remove_favorite_path] 成功移除");
    Ok(())
}

/// 获取排序后的常用路径（按综合分数排序）
pub fn get_sorted_favorite_paths() -> Result<Vec<FavoritePath>> {
    info!("[get_sorted_favorite_paths] 获取排序后的常用路径");
    let conn = get_connection()?;
    
    // 排序权重配置（与前端保持一致）
    const RECENCY_WEIGHT: f64 = 0.6;
    const FREQUENCY_WEIGHT: f64 = 0.4;
    const RECENCY_DECAY_DAYS: f64 = 30.0;
    const MAX_DISPLAY: i64 = 10;
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    
    let mut stmt = conn.prepare(
        "SELECT path, use_count, last_used_at FROM favorite_paths"
    )?;
    
    let paths = stmt.query_map([], |row| {
        Ok(FavoritePath {
            path: row.get::<_, String>(0)?,
            use_count: row.get::<_, i64>(1)?,
            last_used_at: row.get::<_, i64>(2)?,
        })
    })?.collect::<Result<Vec<FavoritePath>>>()?;
    
    // 计算分数并排序
    let mut scored_paths: Vec<(FavoritePath, f64)> = paths
        .into_iter()
        .map(|p| {
            let days_since_last_use = (now - p.last_used_at) as f64 / (1000.0 * 60.0 * 60.0 * 24.0);
            let recency_factor = std::exp(-days_since_last_use / RECENCY_DECAY_DAYS);
            let frequency_factor = std::log10(p.use_count as f64 + 1.0) / std::log10(100.0);
            let score = recency_factor * RECENCY_WEIGHT + frequency_factor * FREQUENCY_WEIGHT;
            (p, score)
        })
        .collect();
    
    scored_paths.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    let result: Vec<FavoritePath> = scored_paths
        .into_iter()
        .take(MAX_DISPLAY as usize)
        .map(|(p, _)| p)
        .collect();
    
    info!("[get_sorted_favorite_paths] 返回 {} 个路径", result.len());
    Ok(result)
}
```

- [ ] **Step 2: 运行 cargo check**

Run: `cd src-tauri && cargo check`
Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/db/favorite_paths.rs
git commit -m "feat(db): 实现 favorite_paths 表 CRUD 操作"
```

---

### Task 1.6: 实现 settings CRUD

**Files:**
- Create: `src-tauri/src/db/settings.rs`

- [ ] **Step 1: 创建 settings.rs CRUD 实现**

```rust
// src-tauri/src/db/settings.rs

use rusqlite::{Connection, Result};
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

/// 删除单个设置
pub fn delete_setting(key: &str) -> Result<()> {
    info!("[delete_setting] 删除设置: key={}", key);
    let conn = get_connection()?;
    
    conn.execute(
        "DELETE FROM app_settings WHERE key = ?1",
        [key],
    )?;
    
    Ok(())
}
```

- [ ] **Step 2: 运行 cargo check**

Run: `cd src-tauri && cargo check`
Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/db/settings.rs
git commit -m "feat(db): 实现 app_settings 表 CRUD 操作"
```

---

### Task 1.7: 实现 localStorage 迁移逻辑

**Files:**
- Create: `src-tauri/src/db/migration.rs`

- [ ] **Step 1: 创建 migration.rs 迁移逻辑**

```rust
// src-tauri/src/db/migration.rs

use tracing::{info, warn, error};
use crate::db::schema::init_database;
use crate::db::favorites::{add_favorite, get_all_favorites};
use crate::db::favorite_paths::record_path_usage;
use crate::db::settings::set_setting;

/// 执行 localStorage 到 SQLite 的迁移
/// 返回是否执行了迁移（true 表示有数据迁移，false 表示无需迁移）
pub fn migrate_from_localstorage() -> bool {
    info!("[migrate_from_localstorage] 开始检查是否需要迁移");
    
    // 1. 初始化数据库
    if let Err(e) = init_database() {
        error!("[migrate_from_localstorage] 数据库初始化失败: {}", e);
        return false;
    }
    
    // 2. 检查是否已有数据（如果有，说明已经迁移过了）
    let existing_favorites = get_all_favorites().unwrap_or_default();
    if !existing_favorites.is_empty() {
        info!("[migrate_from_localstorage] 已有 {} 个收藏，跳过迁移", existing_favorites.len());
        return false;
    }
    
    info!("[migrate_from_localstorage] 检测到需要迁移，开始读取 localStorage 数据");
    
    // 注意：localStorage 是前端存储，后端无法直接读取
    // 迁移逻辑需要前端调用后端 API 来完成
    // 这里只返回一个标志，表示需要迁移
    // 实际迁移在前端启动时完成
    
    // 返回 true 表示需要前端执行迁移
    true
}

/// 检查是否需要迁移
pub fn needs_migration() -> bool {
    // 检查数据库是否有数据
    let existing_favorites = get_all_favorites().unwrap_or_default();
    existing_favorites.is_empty()
}
```

- [ ] **Step 2: 运行 cargo check**

Run: `cd src-tauri && cargo check`
Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/db/migration.rs
git commit -m "feat(db): 实现 localStorage 迁移检查逻辑"
```

---

### Task 1.8: 注册数据库 Tauri 命令

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/db/sessions_meta.rs`
- Modify: `src-tauri/src/db/favorites.rs`
- Modify: `src-tauri/src/db/favorite_paths.rs`
- Modify: `src-tauri/src/db/settings.rs`
- Modify: `src-tauri/src/db/migration.rs`

- [ ] **Step 1: 在 favorites.rs 添加 Tauri 命令包装**

在 `src-tauri/src/db/favorites.rs` 末尾添加：

```rust
/// Tauri 命令：添加收藏
#[tauri::command]
pub fn add_favorite_cmd(session_id: String) -> Result<(), String> {
    add_favorite(&session_id).map_err(|e| format!("添加收藏失败: {}", e))
}

/// Tauri 命令：移除收藏
#[tauri::command]
pub fn remove_favorite_cmd(session_id: String) -> Result<(), String> {
    remove_favorite(&session_id).map_err(|e| format!("移除收藏失败: {}", e))
}

/// Tauri 命令：检查是否收藏
#[tauri::command]
pub fn is_favorite_cmd(session_id: String) -> Result<bool, String> {
    is_favorite(&session_id).map_err(|e| format!("检查收藏失败: {}", e))
}

/// Tauri 命令：获取所有收藏
#[tauri::command]
pub fn get_all_favorites_cmd() -> Result<Vec<String>, String> {
    get_all_favorites().map_err(|e| format!("获取收藏列表失败: {}", e))
}
```

- [ ] **Step 2: 在 favorite_paths.rs 添加 Tauri 命令包装**

在 `src-tauri/src/db/favorite_paths.rs` 末尾添加：

```rust
/// Tauri 命令：记录路径使用
#[tauri::command]
pub fn record_path_usage_cmd(path: String) -> Result<(), String> {
    record_path_usage(&path).map_err(|e| format!("记录路径失败: {}", e))
}

/// Tauri 命令：移除常用路径
#[tauri::command]
pub fn remove_favorite_path_cmd(path: String) -> Result<(), String> {
    remove_favorite_path(&path).map_err(|e| format!("移除路径失败: {}", e))
}

/// Tauri 命令：获取排序后的常用路径
#[tauri::command]
pub fn get_sorted_favorite_paths_cmd() -> Result<Vec<FavoritePath>, String> {
    get_sorted_favorite_paths().map_err(|e| format!("获取路径失败: {}", e))
}
```

- [ ] **Step 3: 在 settings.rs 添加 Tauri 命令包装**

在 `src-tauri/src/db/settings.rs` 末尾添加：

```rust
/// Tauri 命令：获取设置
#[tauri::command]
pub fn get_setting_cmd(key: String) -> Result<Option<String>, String> {
    get_setting(&key).map_err(|e| format!("获取设置失败: {}", e))
}

/// Tauri 命令：设置值
#[tauri::command]
pub fn set_setting_cmd(key: String, value: String) -> Result<(), String> {
    set_setting(&key, &value).map_err(|e| format!("设置失败: {}", e))
}

/// Tauri 命令：获取所有设置
#[tauri::command]
pub fn get_all_settings_cmd() -> Result<HashMap<String, String>, String> {
    get_all_settings().map_err(|e| format!("获取设置失败: {}", e))
}
```

- [ ] **Step 4: 在 migration.rs 添加 Tauri 命令包装**

在 `src-tauri/src/db/migration.rs` 末尾添加：

```rust
/// Tauri 命令：检查是否需要迁移
#[tauri::command]
pub fn needs_migration_cmd() -> Result<bool, String> {
    Ok(needs_migration())
}
```

- [ ] **Step 5: 修改 lib.rs 导入和注册命令**

在 `src-tauri/src/lib.rs` 中：

```rust
// 在现有导入后添加（约第5-25行）
mod db;

// 在导入 commands 部分添加
use db::sessions_meta::{set_session_name_cmd, get_session_name_cmd, delete_session_name_cmd};
use db::favorites::{add_favorite_cmd, remove_favorite_cmd, is_favorite_cmd, get_all_favorites_cmd};
use db::favorite_paths::{record_path_usage_cmd, remove_favorite_path_cmd, get_sorted_favorite_paths_cmd};
use db::settings::{get_setting_cmd, set_setting_cmd, get_all_settings_cmd};
use db::migration::needs_migration_cmd;
```

- [ ] **Step 6: 在 invoke_handler 中注册命令**

修改 `src-tauri/src/lib.rs` 的 `invoke_handler` 部分：

```rust
.invoke_handler(tauri::generate_handler![
    // 现有命令保持不变
    list_sessions_optimized,
    get_session_messages_optimized,
    delete_session_optimized,
    init_running,
    list_running,
    start_polling_cmd,
    stop_polling_cmd,
    get_conversation,
    refresh_sessions,
    start_new_session,
    start_sessions_watcher,
    stop_sessions_watcher,
    start_hooks,
    stop_hooks,
    delete_session_cmd,
    jump_to_terminal,
    jump_to_terminal_by_pid,
    smart_jump_to_terminal,
    resume_in_terminal,
    open_directory,
    open_in_vscode,
    get_available_sounds,
    get_sound_data,
    
    // 数据库命令 - 新增
    set_session_name_cmd,
    get_session_name_cmd,
    delete_session_name_cmd,
    add_favorite_cmd,
    remove_favorite_cmd,
    is_favorite_cmd,
    get_all_favorites_cmd,
    record_path_usage_cmd,
    remove_favorite_path_cmd,
    get_sorted_favorite_paths_cmd,
    get_setting_cmd,
    set_setting_cmd,
    get_all_settings_cmd,
    needs_migration_cmd,
])
```

- [ ] **Step 7: 运行 cargo check**

Run: `cd src-tauri && cargo check`
Expected: 编译成功，无错误

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/db/*.rs
git commit -m "feat(db): 注册所有数据库 Tauri 命令"
```

---

### Task 1.9: 前端创建 dbService 服务

**Files:**
- Create: `src/services/dbService.ts`

- [ ] **Step 1: 创建 dbService.ts 封装数据库调用**

```typescript
// src/services/dbService.ts

import { invoke } from '@tauri-apps/api/core'

// ========== Sessions Meta ==========

export async function setSessionName(sessionId: string, name: string): Promise<void> {
  await invoke('set_session_name_cmd', { sessionId, name })
}

export async function getSessionName(sessionId: string): Promise<string | null> {
  return await invoke('get_session_name_cmd', { sessionId })
}

export async function deleteSessionName(sessionId: string): Promise<void> {
  await invoke('delete_session_name_cmd', { sessionId })
}

// ========== Favorites ==========

export async function addFavorite(sessionId: string): Promise<void> {
  await invoke('add_favorite_cmd', { sessionId })
}

export async function removeFavorite(sessionId: string): Promise<void> {
  await invoke('remove_favorite_cmd', { sessionId })
}

export async function isFavorite(sessionId: string): Promise<boolean> {
  return await invoke('is_favorite_cmd', { sessionId })
}

export async function getAllFavorites(): Promise<string[]> {
  return await invoke('get_all_favorites_cmd')
}

// ========== Favorite Paths ==========

export async function recordPathUsage(path: string): Promise<void> {
  await invoke('record_path_usage_cmd', { path })
}

export async function removeFavoritePath(path: string): Promise<void> {
  await invoke('remove_favorite_path_cmd', { path })
}

export interface FavoritePath {
  path: string
  useCount: number
  lastUsedAt: number
}

export async function getSortedFavoritePaths(): Promise<FavoritePath[]> {
  return await invoke('get_sorted_favorite_paths_cmd')
}

// ========== Settings ==========

export async function getSetting(key: string): Promise<string | null> {
  return await invoke('get_setting_cmd', { key })
}

export async function setSetting(key: string, value: string): Promise<void> {
  await invoke('set_setting_cmd', { key, value })
}

export async function getAllSettings(): Promise<Record<string, string>> {
  return await invoke('get_all_settings_cmd')
}

// ========== Migration ==========

export async function needsMigration(): Promise<boolean> {
  return await invoke('needs_migration_cmd')
}
```

- [ ] **Step 2: 在 services/index.ts 中导出**

修改 `src/services/index.ts`：

```typescript
export * from './claudeSession'
export * from './terminalService'
export * from './notificationService'
export * from './soundService'
export * from './dbService'  // 新增
```

- [ ] **Step 3: Commit**

```bash
git add src/services/dbService.ts src/services/index.ts
git commit -m "feat(frontend): 创建 dbService 封装数据库调用"
```

---

### Task 1.10: 重构 favoriteStore 移除 persist

**Files:**
- Modify: `src/stores/favoriteStore.ts`

- [ ] **Step 1: 重写 favoriteStore.ts**

```typescript
// src/stores/favoriteStore.ts

import { create } from 'zustand'
import { addFavorite, removeFavorite, getAllFavorites, isFavorite } from '@/services/dbService'

interface FavoriteState {
  favorites: Set<string>
  initialized: boolean

  // Actions
  initialize: () => Promise<void>
  addFavorite: (sessionId: string) => Promise<void>
  removeFavorite: (sessionId: string) => Promise<void>
  toggleFavorite: (sessionId: string) => Promise<void>
  isFavorite: (sessionId: string) => boolean
}

export const useFavoriteStore = create<FavoriteState>()((set, get) => ({
  favorites: new Set<string>(),
  initialized: false,

  initialize: async () => {
    try {
      const favoriteIds = await getAllFavorites()
      set({ favorites: new Set(favoriteIds), initialized: true })
    } catch (e) {
      console.error('初始化收藏列表失败:', e)
      set({ favorites: new Set(), initialized: true })
    }
  },

  addFavorite: async (sessionId: string) => {
    await addFavorite(sessionId)
    set((state) => {
      const newFavorites = new Set(state.favorites)
      newFavorites.add(sessionId)
      return { favorites: newFavorites }
    })
  },

  removeFavorite: async (sessionId: string) => {
    await removeFavorite(sessionId)
    set((state) => {
      const newFavorites = new Set(state.favorites)
      newFavorites.delete(sessionId)
      return { favorites: newFavorites }
    })
  },

  toggleFavorite: async (sessionId: string) => {
    const state = get()
    if (state.favorites.has(sessionId)) {
      await state.removeFavorite(sessionId)
    } else {
      await state.addFavorite(sessionId)
    }
  },

  isFavorite: (sessionId: string) => {
    return get().favorites.has(sessionId)
  },
}))
```

- [ ] **Step 2: Commit**

```bash
git add src/stores/favoriteStore.ts
git commit -m "refactor(stores): 重构 favoriteStore 移除 localStorage persist"
```

---

### Task 1.11: 重构 settingsStore 移除 persist

**Files:**
- Modify: `src/stores/settingsStore.ts`

- [ ] **Step 1: 重写 settingsStore.ts**

```typescript
// src/stores/settingsStore.ts

import { create } from 'zustand'
import { getSetting, setSetting, getAllSettings, recordPathUsage, removeFavoritePath, getSortedFavoritePaths, FavoritePath } from '@/services/dbService'
import type { AppSettings, TerminalType } from '@/types'
import { FAVORITE_PATH_CONFIG } from '@/types'

interface SettingsState extends AppSettings {
  initialized: boolean

  // Actions
  initialize: () => Promise<void>
  recordPathUsage: (path: string) => Promise<void>
  removeFavoritePath: (path: string) => Promise<void>
  setDefaultTimeRange: (range: '3d' | '7d' | '30d' | 'all') => Promise<void>
  setNotificationSound: (enabled: boolean) => Promise<void>
  setNotificationDesktop: (enabled: boolean) => Promise<void>
  setNotificationSoundFile: (filename: string) => Promise<void>
  setTheme: (theme: 'light' | 'dark' | 'system') => Promise<void>
  setTerminalType: (type: TerminalType) => Promise<void>
  getSortedFavoritePaths: () => FavoritePath[]
}

const DEFAULT_SETTINGS: AppSettings = {
  favoritePaths: { paths: [] },
  defaultTimeRange: '30d',
  notificationSound: true,
  notificationDesktop: true,
  notificationSoundFile: '',
  theme: 'system',
  terminalType: 'wezterm',
}

export const useSettingsStore = create<SettingsState>()((set, get) => ({
  ...DEFAULT_SETTINGS,
  initialized: false,

  initialize: async () => {
    try {
      const settings = await getAllSettings()
      
      // 解析设置
      const parsed: Partial<AppSettings> = {}
      
      if (settings['defaultTimeRange']) {
        parsed.defaultTimeRange = settings['defaultTimeRange'] as '3d' | '7d' | '30d' | 'all'
      }
      if (settings['notificationSound']) {
        parsed.notificationSound = settings['notificationSound'] === 'true'
      }
      if (settings['notificationDesktop']) {
        parsed.notificationDesktop = settings['notificationDesktop'] === 'true'
      }
      if (settings['notificationSoundFile']) {
        parsed.notificationSoundFile = settings['notificationSoundFile']
      }
      if (settings['theme']) {
        parsed.theme = settings['theme'] as 'light' | 'dark' | 'system'
      }
      if (settings['terminalType']) {
        parsed.terminalType = settings['terminalType'] as TerminalType
      }
      
      // 获取常用路径
      const paths = await getSortedFavoritePaths()
      parsed.favoritePaths = { paths }
      
      set({ ...DEFAULT_SETTINGS, ...parsed, initialized: true })
    } catch (e) {
      console.error('初始化设置失败:', e)
      set({ ...DEFAULT_SETTINGS, initialized: true })
    }
  },

  recordPathUsage: async (path: string) => {
    const normalized = normalizePath(path)
    await recordPathUsage(normalized)
    // 重新获取排序后的路径
    const paths = await getSortedFavoritePaths()
    set({ favoritePaths: { paths } })
  },

  removeFavoritePath: async (path: string) => {
    const normalized = normalizePath(path)
    await removeFavoritePath(normalized)
    const paths = await getSortedFavoritePaths()
    set({ favoritePaths: { paths } })
  },

  setDefaultTimeRange: async (range) => {
    await setSetting('defaultTimeRange', range)
    set({ defaultTimeRange: range })
  },

  setNotificationSound: async (enabled) => {
    await setSetting('notificationSound', enabled.toString())
    set({ notificationSound: enabled })
  },

  setNotificationDesktop: async (enabled) => {
    await setSetting('notificationDesktop', enabled.toString())
    set({ notificationDesktop: enabled })
  },

  setNotificationSoundFile: async (filename) => {
    await setSetting('notificationSoundFile', filename)
    set({ notificationSoundFile: filename })
  },

  setTheme: async (theme) => {
    await setSetting('theme', theme)
    set({ theme })
  },

  setTerminalType: async (type) => {
    await setSetting('terminalType', type)
    set({ terminalType: type })
  },

  getSortedFavoritePaths: () => {
    return get().favoritePaths.paths
  },
}))

// 路径标准化函数（保留）
function normalizePath(path: string): string {
  let normalized = path.trim()
  if (normalized.length > 3) {
    normalized = normalized.replace(/[\\\/]+$/, '')
  }
  if (normalized.match(/^[a-zA-Z]:/)) {
    normalized = normalized[0].toUpperCase() + normalized.slice(1)
  }
  return normalized
}
```

- [ ] **Step 2: Commit**

```bash
git add src/stores/settingsStore.ts
git commit -m "refactor(stores): 重构 settingsStore 移除 localStorage persist"
```

---

### Task 1.12: 实现前端迁移和初始化逻辑

**Files:**
- Modify: `src/App.tsx` 或入口文件

- [ ] **Step 1: 查找应用入口文件确定初始化位置**

Run: `grep -l "initialize" src/*.tsx src/**/*.tsx` 或查看项目入口

假设入口在 `src/App.tsx`，添加初始化逻辑：

```typescript
// 在 App.tsx 或合适的入口组件中添加

import { useEffect } from 'react'
import { useFavoriteStore, useSettingsStore } from '@/stores'
import { needsMigration, addFavorite, setSetting, recordPathUsage } from '@/services/dbService'

function App() {
  const { initialize: initFavorites } = useFavoriteStore()
  const { initialize: initSettings } = useSettingsStore()

  useEffect(() => {
    async function initializeApp() {
      // 检查是否需要迁移 localStorage 数据
      const shouldMigrate = await needsMigration()
      
      if (shouldMigrate) {
        // 执行迁移
        await migrateFromLocalStorage()
      }
      
      // 初始化 stores
      await initFavorites()
      await initSettings()
    }
    
    initializeApp()
  }, [])

  // ... 其余组件代码
}

// 迁移函数
async function migrateFromLocalStorage() {
  // 迁移收藏
  const favoritesStr = localStorage.getItem('claude-fleet-favorites')
  if (favoritesStr) {
    try {
      const data = JSON.parse(favoritesStr)
      const favorites = data.state?.favorites || []
      for (const sessionId of favorites) {
        await addFavorite(sessionId)
      }
      localStorage.removeItem('claude-fleet-favorites')
    } catch (e) {
      console.error('迁移收藏失败:', e)
    }
  }
  
  // 迁移设置
  const settingsStr = localStorage.getItem('claude-fleet-settings')
  if (settingsStr) {
    try {
      const data = JSON.parse(settingsStr)
      const state = data.state || {}
      
      // 迁移各个设置
      if (state.defaultTimeRange) {
        await setSetting('defaultTimeRange', state.defaultTimeRange)
      }
      if (state.notificationSound !== undefined) {
        await setSetting('notificationSound', state.notificationSound.toString())
      }
      if (state.notificationDesktop !== undefined) {
        await setSetting('notificationDesktop', state.notificationDesktop.toString())
      }
      if (state.notificationSoundFile) {
        await setSetting('notificationSoundFile', state.notificationSoundFile)
      }
      if (state.theme) {
        await setSetting('theme', state.theme)
      }
      if (state.terminalType) {
        await setSetting('terminalType', state.terminalType)
      }
      
      // 迁移常用路径
      const paths = state.favoritePaths?.paths || []
      for (const p of paths) {
        await recordPathUsage(p.path)
      }
      
      localStorage.removeItem('claude-fleet-settings')
    } catch (e) {
      console.error('迁移设置失败:', e)
    }
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/App.tsx
git commit -m "feat(frontend): 实现应用启动时的迁移和初始化逻辑"
```

---

## Phase 2: 对话截断修复（可并行）

### Task 2.1: 修复 ConversationView 底部截断

**Files:**
- Modify: `src/components/management/SessionDetail.tsx`

- [ ] **Step 1: 修改 SessionDetail.tsx 的 ConversationView 容器**

找到第 152-168 行的布局代码，修改为：

```tsx
{/* 对话历史 */}
<div className="flex-1 min-h-0 min-w-0 overflow-hidden">
  {/* 对话记录标题 */}
  <div className="flex items-center justify-between px-4 py-2 border-b bg-white min-w-0">
    <h3 className="text-sm font-medium text-gray-700 truncate">对话记录</h3>
    <Button
      variant="ghost"
      size="sm"
      onClick={onRefresh}
      className="h-7"
    >
      <RefreshCw className="w-4 h-4" />
    </Button>
  </div>
  
  {/* ConversationView - 添加 flex-1 min-h-0 包裹 */}
  <div className="flex-1 min-h-0 min-w-0">
    <ConversationView
      messages={conversationMessages}
      loading={messagesLoading}
    />
  </div>
</div>
```

- [ ] **Step 2: 验证修复效果**

Run: `npm run tauri dev`
手动测试：打开一个 session 详情，滚动到底部，确认时间戳和边框完整可见

- [ ] **Step 3: Commit**

```bash
git add src/components/management/SessionDetail.tsx
git commit -m "fix(ui): 修复对话记录底部截断问题，添加 flex-1 容器"
```

---

## Phase 3: Session 命名功能

### Task 3.1: 后端类型增加 custom_name 字段

**Files:**
- Modify: `src-tauri/src/utils/session_types.rs`
- Modify: `src-tauri/src/utils/running_sessions.rs`

- [ ] **Step 1: 修改 session_types.rs SessionMeta 结构**

在 `SessionMeta` 结构体中添加 `custom_name` 字段：

```rust
// src-tauri/src/utils/session_types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub provider_id: String,
    pub session_id: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub project_dir: Option<String>,
    pub created_at: Option<i64>,
    pub last_active_at: Option<i64>,
    pub source_path: Option<String>,
    pub resume_command: Option<String>,
    // 新增
    pub custom_name: Option<String>,  // Claude Fleet 自定义名称
}
```

- [ ] **Step 2: 修改 running_sessions.rs RunningSession 结构**

在 `RunningSession` 结构体中添加 `custom_name` 字段：

```rust
// src-tauri/src/utils/running_sessions.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningSession {
    pub session_id: String,
    pub pid: u32,
    pub status: SessionStatus,
    pub cwd: String,
    pub name: String,
    pub updated_at: u64,
    pub away_summary: Option<String>,
    pub away_summary_at: Option<u64>,
    pub last_user_input: Option<String>,
    // 新增
    pub custom_name: Option<String>,  // Claude Fleet 自定义名称
}
```

- [ ] **Step 3: 运行 cargo check**

Run: `cd src-tauri && cargo check`
Expected: 编译成功

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/utils/session_types.rs src-tauri/src/utils/running_sessions.rs
git commit -m "feat(types): 后端类型增加 custom_name 字段"
```

---

### Task 3.2: 修改 session_commands 查询合并 custom_name

**Files:**
- Modify: `src-tauri/src/commands/session_commands.rs`

- [ ] **Step 1: 修改 list_sessions_optimized 函数**

```rust
// src-tauri/src/commands/session_commands.rs

use crate::utils::session_types::{SessionMeta, SessionMessage};
use crate::utils::claude_session::{scan_sessions, get_session_messages, delete_session};
use crate::db::sessions_meta::get_session_names;
use tracing::info;

/// List all sessions - optimized version for management tab
#[tauri::command]
pub fn list_sessions_optimized() -> Result<Vec<SessionMeta>, String> {
    info!("[list_sessions_optimized] Scanning sessions");
    let sessions = scan_sessions();
    
    // 获取所有 session 的自定义名称
    let session_ids: Vec<String> = sessions.iter().map(|s| s.session_id.clone()).collect();
    let custom_names = get_session_names(&session_ids)
        .map_err(|e| format!("获取自定义名称失败: {}", e))?;
    
    // 合并 custom_name 到 session
    let mut result = sessions;
    for session in &mut result {
        for (id, name) in &custom_names {
            if session.session_id == *id {
                session.custom_name = name.clone();
                break;
            }
        }
    }
    
    info!("[list_sessions_optimized] Found {} sessions", result.len());
    Ok(result)
}

// get_session_messages_optimized 和 delete_session_optimized 保持不变
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/commands/session_commands.rs
git commit -m "feat(commands): list_sessions_optimized 合并 custom_name 查询"
```

---

### Task 3.3: 修改 running session 查询合并 custom_name

**Files:**
- Modify: `src-tauri/src/commands/session.rs`

- [ ] **Step 1: 修改 list_running 函数**

找到 `list_running` 函数，修改为：

```rust
use crate::db::sessions_meta::get_session_names;

#[tauri::command]
pub fn list_running() -> Result<Vec<RunningSession>, String> {
    info!("[list_running] Getting running sessions");
    let sessions = get_running_sessions();
    
    // 获取所有 running session 的自定义名称
    let session_ids: Vec<String> = sessions.iter().map(|s| s.session_id.clone()).collect();
    let custom_names = get_session_names(&session_ids)
        .map_err(|e| format!("获取自定义名称失败: {}", e))?;
    
    // 合并 custom_name
    let mut result = sessions;
    for session in &mut result {
        for (id, name) in &custom_names {
            if session.session_id == *id {
                session.custom_name = name.clone();
                break;
            }
        }
    }
    
    info!("[list_running] Returning {} running sessions", result.len());
    Ok(result)
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/commands/session.rs
git commit -m "feat(commands): list_running 合并 custom_name 查询"
```

---

### Task 3.4: 前端类型增加 customName 字段

**Files:**
- Modify: `src/types/session.ts`

- [ ] **Step 1: 修改 SessionMeta 类型**

```typescript
// src/types/session.ts

export interface SessionMeta {
  providerId: string;
  sessionId: string;
  title?: string;
  summary?: string;
  projectDir?: string;
  createdAt?: number;
  lastActiveAt?: number;
  sourcePath?: string;
  resumeCommand?: string;
  isFavorite?: boolean;
  // 新增
  customName?: string;  // Claude Fleet 自定义名称
}

// RunningSession 类型也需要添加
export interface RunningSession {
  session_id: string;
  pid: number;
  status: 'busy' | 'idle' | 'waiting';
  cwd: string;
  name: string;
  updated_at: number;
  away_summary?: string;
  away_summary_at?: number;
  last_user_input?: string;
  // 新增
  custom_name?: string;  // Claude Fleet 自定义名称
}
```

- [ ] **Step 2: Commit**

```bash
git add src/types/session.ts
git commit -m "feat(types): 前端类型增加 customName 字段"
```

---

### Task 3.5: 创建 getDisplayName 工具函数

**Files:**
- Modify: `src/utils/index.ts`

- [ ] **Step 1: 在 utils/index.ts 或新建文件添加函数**

```typescript
// src/utils/index.ts 或 src/utils/displayUtils.ts

import type { SessionMeta, RunningSession } from '@/types'

/**
 * 获取 session 显示名称（优先级：customName > title > projectDir > sessionId）
 */
export function getDisplayName(session: SessionMeta | RunningSession): string {
  // RunningSession 使用下划线命名，SessionMeta 使用驼峰命名
  const customName = 'customName' in session ? session.customName : session.custom_name
  const title = 'title' in session ? session.title : undefined
  const projectDir = 'projectDir' in session ? session.projectDir : session.cwd
  const sessionId = 'sessionId' in session ? session.sessionId : session.session_id
  
  return customName
    || title
    || projectDir?.split(/[\\/]/).pop()
    || sessionId.slice(0, 8)
}
```

- [ ] **Step 2: 在 utils/index.ts 中导出**

```typescript
export * from './displayUtils'
```

- [ ] **Step 3: Commit**

```bash
git add src/utils/index.ts
git commit -m "feat(utils): 添加 getDisplayName 工具函数"
```

---

### Task 3.6: 创建 EditableName 组件

**Files:**
- Create: `src/components/common/EditableName.tsx`

- [ ] **Step 1: 创建 EditableName.tsx 可编辑名称组件**

```tsx
// src/components/common/EditableName.tsx

import { useState, useRef, useEffect } from 'react'
import { cn } from '@/lib/utils'
import { Pencil, X } from 'lucide-react'

interface EditableNameProps {
  name: string
  onSave: (newName: string) => Promise<void>
  className?: string
  onDoubleClick?: () => void
}

export function EditableName({ name, onSave, className, onDoubleClick }: EditableNameProps) {
  const [isEditing, setIsEditing] = useState(false)
  const [editValue, setEditValue] = useState(name)
  const [saving, setSaving] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus()
      inputRef.current.select()
    }
  }, [isEditing])

  const handleStartEdit = () => {
    setEditValue(name)
    setIsEditing(true)
  }

  const handleCancel = () => {
    setIsEditing(false)
    setEditValue(name)
  }

  const handleSave = async () => {
    if (saving) return
    
    const trimmedValue = editValue.trim()
    
    // 如果值为空或与原值相同，取消
    if (!trimmedValue || trimmedValue === name) {
      handleCancel()
      return
    }
    
    setSaving(true)
    try {
      await onSave(trimmedValue)
      setIsEditing(false)
    } catch (e) {
      console.error('保存名称失败:', e)
      // 恢复原值
      setEditValue(name)
    } finally {
      setSaving(false)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSave()
    } else if (e.key === 'Escape') {
      handleCancel()
    }
  }

  const handleBlur = () => {
    handleSave()
  }

  if (isEditing) {
    return (
      <div className="flex items-center gap-1">
        <input
          ref={inputRef}
          type="text"
          value={editValue}
          onChange={(e) => setEditValue(e.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={handleBlur}
          disabled={saving}
          className={cn(
            "px-2 py-1 text-sm border rounded focus:outline-none focus:ring-2 focus:ring-violet-500",
            saving && "opacity-50 cursor-not-allowed",
            className
          )}
        />
        <button
          onClick={handleCancel}
          className="p-1 hover:bg-gray-100 rounded"
          type="button"
        >
          <X className="w-4 h-4 text-gray-400" />
        </button>
      </div>
    )
  }

  return (
    <div 
      className={cn("flex items-center gap-1 group cursor-pointer", className)}
      onDoubleClick={(e) => {
        e.stopPropagation()
        handleStartEdit()
        onDoubleClick?.()
      }}
    >
      <span className="truncate">{name}</span>
      <button
        onClick={(e) => {
          e.stopPropagation()
          handleStartEdit()
        }}
        className="p-1 opacity-0 group-hover:opacity-100 hover:bg-gray-100 rounded transition-opacity"
        type="button"
      >
        <Pencil className="w-3.5 h-3.5 text-gray-400" />
      </button>
    </div>
  )
}
```

- [ ] **Step 2: 在 common/index.ts 中导出**

```typescript
export * from './EditableName'
```

- [ ] **Step 3: Commit**

```bash
git add src/components/common/EditableName.tsx src/components/common/index.ts
git commit -m "feat(components): 创建 EditableName 可编辑名称组件"
```

---

### Task 3.7: 修改 SessionListItem 使用 EditableName

**Files:**
- Modify: `src/components/management/SessionListItem.tsx`

- [ ] **Step 1: 修改 SessionListItem 使用 getDisplayName 和 EditableName**

```tsx
// src/components/management/SessionListItem.tsx

import { cn } from "@/lib/utils"
import type { SessionMeta } from "@/types"
import { Clock, Star, Pencil } from "lucide-react"
import { formatRelativeTime } from "@/utils"
import { getDisplayName } from "@/utils"
import { EditableName } from "@/components/common/EditableName"
import { setSessionName, deleteSessionName } from "@/services/dbService"

interface SessionListItemProps {
  session: SessionMeta
  selected: boolean
  onClick: () => void
  onToggleFavorite: () => void
  onRename?: () => void  // 重命名后的回调
}

export function SessionListItem({ session, selected, onClick, onToggleFavorite, onRename }: SessionListItemProps) {
  const displayName = getDisplayName(session)
  const lastActive = session.lastActiveAt || session.createdAt

  const handleSaveName = async (newName: string) => {
    await setSessionName(session.sessionId, newName)
    onRename?.()
  }

  return (
    <div
      onClick={onClick}
      onContextMenu={(e) => {
        e.preventDefault()
        // 右键菜单逻辑将在 Task 3.9 实现
      }}
      className={cn(
        "p-3 rounded-lg cursor-pointer transition-all border min-w-0",
        selected
          ? "bg-violet-50 border-violet-200 shadow-sm"
          : "bg-white border-gray-100 hover:bg-gray-50 hover:border-gray-200"
      )}
    >
      {/* 标题和收藏 */}
      <div className="flex items-start justify-between gap-2 mb-2">
        <EditableName
          name={displayName}
          onSave={handleSaveName}
          className={cn(
            "font-medium text-sm leading-snug min-w-0",
            selected ? "text-violet-900" : "text-gray-900"
          )}
        />
        <button
          onClick={(e) => {
            e.stopPropagation()
            onToggleFavorite()
          }}
          className="p-1 rounded hover:bg-gray-100 shrink-0"
        >
          <Star
            className={cn(
              "w-4 h-4",
              session.isFavorite
                ? "fill-amber-400 text-amber-400"
                : "text-gray-300 hover:text-gray-400"
            )}
          />
        </button>
      </div>

      {/* 路径 */}
      {session.projectDir && (
        <p className="text-xs text-gray-500 truncate mb-2">
          {session.projectDir}
        </p>
      )}

      {/* 时间 */}
      <div className="flex items-center gap-1.5 text-xs text-gray-400">
        <Clock className="w-3.5 h-3.5" />
        <span>
          {lastActive ? formatRelativeTime(new Date(lastActive).toISOString()) : "未知时间"}
        </span>
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/management/SessionListItem.tsx
git commit -m "feat(components): SessionListItem 支持双击编辑名称"
```

---

### Task 3.8: 修改 SessionCardNew 使用 EditableName

**Files:**
- Modify: `src/components/running/SessionCard.tsx`

- [ ] **Step 1: 修改 SessionCardNew 组件**

```tsx
// 在 SessionCardNewProps 后添加

import { getDisplayName } from "@/utils"
import { EditableName } from "@/components/common/EditableName"
import { setSessionName } from "@/services/dbService"

interface SessionCardNewProps {
  session: RunningSession
  onJumpToTerminal: (session: RunningSession) => void
  onToggleFavorite?: (sessionId: string) => void
  onRename?: () => void  // 重命名后的回调
  compact?: boolean
}

export function SessionCardNew({ 
  session, 
  onJumpToTerminal, 
  onToggleFavorite, 
  onRename,
  compact = true 
}: SessionCardNewProps) {
  const displayName = getDisplayName(session)
  
  const handleSaveName = async (newName: string) => {
    await setSessionName(session.session_id, newName)
    onRename?.()
  }

  return (
    <div
      onContextMenu={(e) => {
        e.preventDefault()
        // 右键菜单逻辑将在 Task 3.9 实现
      }}
      className={cn(
        "rounded-lg p-4 flex justify-between items-center",
        "border transition-all",
        isWaitingInput
          ? "border-amber-400 bg-amber-50 shadow-sm"
          : "border-gray-200 bg-white hover:border-gray-300"
      )}
    >
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <EditableName
            name={displayName}
            onSave={handleSaveName}
            className="font-semibold text-gray-900"
          />
          <StatusBadge status={session.status} />
        </div>
        
        {/* ... 其他内容保持不变 ... */}
      </div>
      
      {/* ... 其他内容保持不变 ... */}
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/running/SessionCard.tsx
git commit -m "feat(components): SessionCardNew 支持双击编辑名称"
```

---

### Task 3.9: 修改 SessionDetail 使用 EditableName

**Files:**
- Modify: `src/components/management/SessionDetail.tsx`

- [ ] **Step 1: 修改 SessionDetail 的标题部分**

```tsx
// 在导入部分添加
import { getDisplayName } from "@/utils"
import { EditableName } from "@/components/common/EditableName"
import { setSessionName } from "@/services/dbService"

// 修改组件内部
export function SessionDetail({
  session,
  messages,
  messagesLoading,
  onDelete,
  onRefresh,
}: SessionDetailProps) {
  const displayName = getDisplayName(session)
  
  const handleSaveName = async (newName: string) => {
    await setSessionName(session.sessionId, newName)
    // 触发重新加载（可选，取决于数据流设计）
  }
  
  // ... 其他代码 ...
  
  return (
    <div className="flex flex-col h-full min-w-0 overflow-hidden">
      {/* 头部信息栏 */}
      <div className="px-4 py-3 border-b bg-gray-50/50">
        {/* 标题行 */}
        <div className="flex items-center gap-3 mb-2 min-w-0">
          <EditableName
            name={displayName}
            onSave={handleSaveName}
            className="text-lg font-semibold text-gray-900 min-w-0"
          />
          {/* ... 其他按钮保持不变 ... */}
        </div>
        
        {/* ... 其他内容保持不变 ... */}
      </div>
      
      {/* ... 其他内容保持不变 ... */}
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/management/SessionDetail.tsx
git commit -m "feat(components): SessionDetail 支持双击编辑名称"
```

---

### Task 3.10: 修改搜索逻辑支持 customName

**Files:**
- Modify: `src/hooks/useSessionSearch.ts`

- [ ] **Step 1: 修改搜索逻辑**

```typescript
// src/hooks/useSessionSearch.ts

import { useMemo } from 'react'
import type { SessionMeta } from '@/types'
import { fuzzyMatch } from '@/utils/fuzzySearch'

export function useSessionSearch(
  sessions: SessionMeta[],
  query: string
): SessionMeta[] {
  return useMemo(() => {
    if (!query.trim()) return sessions
    
    const lowerQuery = query.toLowerCase()
    
    return sessions.filter(session => {
      // 搜索范围：customName + title + projectDir + sessionId
      const searchFields = [
        session.customName,
        session.title,
        session.projectDir?.split(/[\\/]/).pop(),
        session.sessionId,
      ].filter(Boolean) as string[]
      
      // 模糊匹配任一字段
      return searchFields.some(field => 
        fuzzyMatch(lowerQuery, field.toLowerCase())
      )
    })
  }, [sessions, query])
}
```

- [ ] **Step 2: Commit**

```bash
git add src/hooks/useSessionSearch.ts
git commit -m "feat(search): 搜索逻辑支持 customName 字段"
```

---

## Phase 4: Workspace 分组功能

### Task 4.1: 创建 WorkspaceGroupItem 组件

**Files:**
- Create: `src/components/management/WorkspaceGroupItem.tsx`

- [ ] **Step 1: 创建 WorkspaceGroupItem.tsx 分组项组件**

```tsx
// src/components/management/WorkspaceGroupItem.tsx

import { useState } from 'react'
import { cn } from '@/lib/utils'
import { ChevronDown, ChevronRight, Folder } from 'lucide-react'
import { SessionListItem } from './SessionListItem'

interface WorkspaceGroupItemProps {
  workspaceName: string
  sessions: Array<{
    session: any
    selected: boolean
    onToggleFavorite: () => void
    onRename?: () => void
  }>
  onSelectSession: (sessionId: string) => void
  defaultExpanded?: boolean
}

export function WorkspaceGroupItem({
  workspaceName,
  sessions,
  onSelectSession,
  defaultExpanded = true,
}: WorkspaceGroupItemProps) {
  const [expanded, setExpanded] = useState(defaultExpanded)

  return (
    <div className="border rounded-lg mb-3 overflow-hidden">
      {/* 分组头部 */}
      <div
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-2 px-4 py-3 bg-gray-50 hover:bg-gray-100 cursor-pointer transition-colors"
      >
        {expanded ? (
          <ChevronDown className="w-4 h-4 text-gray-500" />
        ) : (
          <ChevronRight className="w-4 h-4 text-gray-500" />
        )}
        <Folder className="w-4 h-4 text-violet-500" />
        <span className="font-medium text-gray-700 truncate">
          {workspaceName}
        </span>
        <span className="text-sm text-gray-500 ml-auto shrink-0">
          ({sessions.length})
        </span>
      </div>

      {/* 分组内容 */}
      {expanded && (
        <div className="p-2 space-y-2 bg-white">
          {sessions.map(({ session, selected, onToggleFavorite, onRename }) => (
            <SessionListItem
              key={session.sessionId}
              session={session}
              selected={selected}
              onClick={() => onSelectSession(session.sessionId)}
              onToggleFavorite={onToggleFavorite}
              onRename={onRename}
            />
          ))}
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/management/WorkspaceGroupItem.tsx
git commit -m "feat(components): 创建 WorkspaceGroupItem 分组项组件"
```

---

### Task 4.2: 创建 GroupedSessionList 组件

**Files:**
- Create: `src/components/management/GroupedSessionList.tsx`

- [ ] **Step 1: 创建 GroupedSessionList.tsx 分组视图容器**

```tsx
// src/components/management/GroupedSessionList.tsx

import { useMemo } from 'react'
import type { SessionMeta } from '@/types'
import { WorkspaceGroupItem } from './WorkspaceGroupItem'

interface GroupedSessionListProps {
  sessions: SessionMeta[]
  selectedSessionId: string | null
  onSelectSession: (sessionId: string) => void
  onToggleFavorite: (sessionId: string) => void
  onRename?: (sessionId: string) => void
}

/**
 * 从 sourcePath 提取 workspace 名称
 * sourcePath 格式: ~/.claude/projects/<project-name>/<session-id>.jsonl
 */
function extractWorkspaceName(sourcePath?: string): string {
  if (!sourcePath) return '未知项目'
  
  const parts = sourcePath.split(/[\\/]/)
  // 找到 projects 后的下一部分
  const projectsIndex = parts.findIndex(p => p === 'projects')
  if (projectsIndex >= 0 && parts.length > projectsIndex + 1) {
    return parts[projectsIndex + 1]
  }
  
  return '未知项目'
}

export function GroupedSessionList({
  sessions,
  selectedSessionId,
  onSelectSession,
  onToggleFavorite,
  onRename,
}: GroupedSessionListProps) {
  // 按 workspace 分组
  const grouped = useMemo(() => {
    const groups: Map<string, SessionMeta[]> = new Map()
    
    for (const session of sessions) {
      const workspace = extractWorkspaceName(session.sourcePath)
      if (!groups.has(workspace)) {
        groups.set(workspace, [])
      }
      groups.get(workspace)!.push(session)
    }
    
    // 转换为数组并排序（按名称）
    return Array.from(groups.entries())
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([workspace, sessionList]) => ({
        workspaceName: workspace,
        sessions: sessionList.sort((a, b) => 
          (b.lastActiveAt || 0) - (a.lastActiveAt || 0)
        ),
      }))
  }, [sessions])

  return (
    <div className="space-y-3">
      {grouped.map(({ workspaceName, sessions: groupSessions }) => (
        <WorkspaceGroupItem
          key={workspaceName}
          workspaceName={workspaceName}
          sessions={groupSessions.map(session => ({
            session,
            selected: selectedSessionId === session.sessionId,
            onToggleFavorite: () => onToggleFavorite(session.sessionId),
            onRename: () => onRename?.(session.sessionId),
          }))}
          onSelectSession={onSelectSession}
          defaultExpanded={true}
        />
      ))}
      
      {grouped.length === 0 && (
        <div className="text-center py-8 text-gray-500">
          没有收藏的 session
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/management/GroupedSessionList.tsx
git commit -m "feat(components): 创建 GroupedSessionList 分组视图容器"
```

---

### Task 4.3: 修改 ManagementTab 添加视图切换

**Files:**
- Modify: `src/components/management/ManagementTab.tsx`

- [ ] **Step 1: 导入新组件和图标**

在 ManagementTab.tsx 导入部分添加：

```tsx
import { GroupedSessionList } from './GroupedSessionList'
import { List, FolderTree } from 'lucide-react'
import { Button } from '@/components/ui/button'
```

- [ ] **Step 2: 添加 viewMode state**

在组件内的 state 部分添加：

```tsx
const [viewMode, setViewMode] = useState<'list' | 'grouped'>('list')
```

- [ ] **Step 3: 在工具栏添加视图切换按钮**

找到工具栏区域（通常在搜索框附近），添加切换按钮：

```tsx
{/* 视图切换按钮 - 仅在收藏模式下显示 */}
{showFavoritesOnly && (
  <div className="flex items-center gap-1 ml-auto">
    <Button
      variant={viewMode === 'list' ? 'default' : 'ghost'}
      size="sm"
      onClick={() => setViewMode('list')}
      className="h-8"
      title="列表视图"
    >
      <List className="w-4 h-4" />
    </Button>
    <Button
      variant={viewMode === 'grouped' ? 'default' : 'ghost'}
      size="sm"
      onClick={() => setViewMode('grouped')}
      className="h-8"
      title="分组视图"
    >
      <FolderTree className="w-4 h-4" />
    </Button>
  </div>
)}
```

- [ ] **Step 4: 根据模式渲染不同视图**

修改 session 列表渲染部分：

```tsx
{/* Session 列表 */}
{viewMode === 'list' || !showFavoritesOnly ? (
  // 现有的列表视图
  <div className="space-y-2">
    {filteredSessions.map(session => (
      <SessionListItem
        key={session.sessionId}
        session={session}
        selected={selectedSessionId === session.sessionId}
        onClick={() => setSelectedSessionId(session.sessionId)}
        onToggleFavorite={() => handleToggleFavorite(session.sessionId)}
        onRename={() => handleRefresh()}
      />
    ))}
    {filteredSessions.length === 0 && (
      <div className="text-center py-8 text-gray-500">
        {searchQuery ? '没有匹配的 session' : '没有 session'}
      </div>
    )}
  </div>
) : (
  // 分组视图（仅收藏模式）
  <GroupedSessionList
    sessions={filteredSessions}
    selectedSessionId={selectedSessionId}
    onSelectSession={setSelectedSessionId}
    onToggleFavorite={handleToggleFavorite}
    onRename={() => handleRefresh()}
  />
)}
```

- [ ] **Step 5: 确保 handleRefresh 函数存在**

如果组件中没有 handleRefresh，添加一个刷新函数：

```tsx
const handleRefresh = () => {
  // 触发重新加载 session 列表
  // 根据实际数据流实现，可能需要调用 queryClient.invalidateQueries
}
```

- [ ] **Step 3: Commit**

```bash
git add src/components/management/ManagementTab.tsx
git commit -m "feat(ManagementTab): 添加收藏模式的分组视图切换"
```

---

## Phase 5: 集成测试和发布

### Task 5.1: 运行完整构建测试

- [ ] **Step 1: 运行前端构建**

Run: `npm run build`
Expected: 构建成功，无 TypeScript 错误

- [ ] **Step 2: 运行 Rust 构建**

Run: `cd src-tauri && cargo build --release`
Expected: 编译成功

- [ ] **Step 3: 运行开发模式测试**

Run: `npm run tauri dev`
手动测试所有功能：
- 收藏功能是否正常
- 常用路径是否正常
- Session 命名功能（双击编辑）
- 分组视图切换
- 对话记录底部显示完整

- [ ] **Step 4: 测试迁移逻辑**

清理 localStorage 后重新启动应用，验证数据迁移

---

### Task 5.2: 最终 Commit 和发布准备

- [ ] **Step 1: 确认所有功能正常后提交**

```bash
git add -A
git commit -m "feat: 完成功能增强 - SQLite 存储、Session 命名、Workspace 分组、对话截断修复"
```

- [ ] **Step 2: 运行发布构建**

Run: `npm run tauri build`
Expected: 生成安装包

---

## Spec Coverage Checklist

| Spec Section | Covered by Task |
|-------------|-----------------|
| 存储重构 - SQLite 位置 | Task 1.2 |
| 存储重构 - 表结构 | Task 1.2 |
| 存储重构 - 迁移策略 | Task 1.7, Task 1.12 |
| 存储重构 - 后端模块 | Tasks 1.2-1.8 |
| 存储重构 - 前端调整 | Tasks 1.9-1.11 |
| Session 命名 - 优先级 | Task 3.5 |
| Session 命名 - 编辑入口 | Tasks 3.6-3.9 |
| Session 命名 - 搜索支持 | Task 3.10 |
| Workspace 分组 - 分组依据 | Task 4.2 |
| Workspace 分组 - 视图切换 | Task 4.3 |
| Workspace 分组 - 树形 UI | Tasks 4.1-4.2 |
| 对话截断修复 | Task 2.1 |