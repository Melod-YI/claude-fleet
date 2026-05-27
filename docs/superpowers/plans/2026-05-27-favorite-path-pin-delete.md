# Favorite Path Pin/Delete 功能实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在新建 Session 对话框中，为常用路径列表增加置顶和删除功能。

**Architecture:** 后端新增 pinned/pinned_at 字段和 toggle_pin_path 命令，前端创建 PathCard 组件实现书签按钮和右键菜单交互。

**Tech Stack:** Rust/Tauri (后端), React/TypeScript/Tailwind/shadcn/ui (前端)

---

## 文件结构

**后端修改：**
- `src-tauri/src/db/schema.rs` - 添加 pinned/pinned_at 列迁移
- `src-tauri/src/db/favorite_paths.rs` - 更新结构体、实现 toggle_pin、修改排序
- `src-tauri/src/lib.rs` - 注册新命令

**前端修改：**
- `src/types/settings.ts` - 更新 FavoritePath 类型
- `src/services/dbService.ts` - 添加 togglePinPath 函数
- `src/stores/settingsStore.ts` - 添加 togglePinPath action
- `src/components/dialogs/PathCard.tsx` - 新建路径卡片组件
- `src/components/dialogs/NewSessionDialog.tsx` - 重构常用路径部分

---

### Task 1: 后端 - 数据库迁移

**Files:**
- Modify: `src-tauri/src/db/schema.rs`

- [ ] **Step 1: 在 init_tables 函数中添加列迁移逻辑**

在 `init_tables()` 函数末尾添加检测并添加缺失列的逻辑：

```rust
/// 初始化数据库表（创建缺失的表）
pub fn init_tables() -> Result<()> {
    info!("[init_tables] 开始初始化数据库表");
    let conn = get_connection()?;

    // 使用 IF NOT EXISTS 确保只创建缺失的表，已存在的表不受影响
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS favorites (
            session_id TEXT PRIMARY KEY,
            added_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT
        );
        CREATE TABLE IF NOT EXISTS sessions_meta (
            session_id TEXT PRIMARY KEY,
            custom_name TEXT,
            created_at INTEGER,
            updated_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS favorite_paths (
            path TEXT PRIMARY KEY,
            use_count INTEGER,
            last_used_at INTEGER
        );"
    )?;

    // 迁移：为 favorite_paths 表添加 pinned 和 pinned_at 列（如果不存在）
    let pinned_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('favorite_paths') WHERE name='pinned'",
        [],
        |row| row.get::<_, i64>(0),
    )? > 0;

    if !pinned_exists {
        conn.execute("ALTER TABLE favorite_paths ADD COLUMN pinned INTEGER DEFAULT 0", [])?;
        conn.execute("ALTER TABLE favorite_paths ADD COLUMN pinned_at INTEGER DEFAULT NULL", [])?;
        info!("[init_tables] 添加 pinned 和 pinned_at 列");
    }

    info!("[init_tables] 数据库表初始化完成");
    Ok(())
}
```

- [ ] **Step 2: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 无编译错误

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/db/schema.rs
git commit -m "feat(db): 添加 favorite_paths 表 pinned/pinned_at 列迁移"
```

---

### Task 2: 后端 - 更新 FavoritePath 结构体

**Files:**
- Modify: `src-tauri/src/db/favorite_paths.rs:10-14`

- [ ] **Step 1: 更新 FavoritePath 结构体添加新字段**

修改 FavoritePath 结构体：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoritePath {
    pub path: String,
    pub use_count: i64,
    pub last_used_at: i64,
    pub pinned: bool,
    pub pinned_at: Option<i64>,
}
```

- [ ] **Step 2: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 编译错误（后续步骤会修复）

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/db/favorite_paths.rs
git commit -m "feat(db): FavoritePath 结构体添加 pinned/pinned_at 字段"
```

---

### Task 3: 后端 - 实现 toggle_pin_path 函数

**Files:**
- Modify: `src-tauri/src/db/favorite_paths.rs`

- [ ] **Step 1: 实现 toggle_pin_path 函数**

在 `favorite_paths.rs` 中添加新函数：

```rust
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
            [&now.to_string(), path],
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
```

- [ ] **Step 2: 添加 Tauri 命令包装**

在文件末尾添加：

```rust
#[tauri::command]
pub fn toggle_pin_path_cmd(path: String) -> Result<FavoritePath, String> {
    toggle_pin_path(&path).map_err(|e| format!("切换置顶状态失败: {}", e))
}
```

- [ ] **Step 3: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 无编译错误

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db/favorite_paths.rs
git commit -m "feat(db): 实现 toggle_pin_path 命令"
```

---

### Task 4: 后端 - 修改排序逻辑

**Files:**
- Modify: `src-tauri/src/db/favorite_paths.rs:61-98`

- [ ] **Step 1: 重写 get_sorted_favorite_paths 函数**

替换原有的 `get_sorted_favorite_paths` 函数：

```rust
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

    // 1. 获取置顶路径（按 pinned_at 降序）
    let mut stmt = conn.prepare(
        "SELECT path, use_count, last_used_at, pinned, pinned_at
         FROM favorite_paths WHERE pinned = 1
         ORDER BY pinned_at DESC"
    )?;

    let pinned_paths: Vec<FavoritePath> = stmt.query_map([], |row| {
        Ok(FavoritePath {
            path: row.get::<_, String>(0)?,
            use_count: row.get::<_, i64>(1)?,
            last_used_at: row.get::<_, i64>(2)?,
            pinned: true,
            pinned_at: row.get::<_, Option<i64>>(4)?,
        })
    })?.collect::<Result<Vec<FavoritePath>>>()?;

    info!("[get_sorted_favorite_paths] 置顶路径数量: {}", pinned_paths.len());

    // 2. 获取非置顶路径并计算分数排序
    let mut stmt = conn.prepare(
        "SELECT path, use_count, last_used_at, pinned, pinned_at
         FROM favorite_paths WHERE pinned = 0"
    )?;

    let unpinned_paths = stmt.query_map([], |row| {
        Ok(FavoritePath {
            path: row.get::<_, String>(0)?,
            use_count: row.get::<_, i64>(1)?,
            last_used_at: row.get::<_, i64>(2)?,
            pinned: false,
            pinned_at: None,
        })
    })?.collect::<Result<Vec<FavoritePath>>>()?;

    let mut scored: Vec<(FavoritePath, f64)> = unpinned_paths
        .into_iter()
        .map(|p| {
            let days = (now - p.last_used_at) as f64 / (1000.0 * 60.0 * 60.0 * 24.0);
            let recency = (-days / RECENCY_DECAY_DAYS).exp();
            let freq = (p.use_count as f64 + 1.0).log10() / 100.0_f64.log10();
            (p, recency * RECENCY_WEIGHT + freq * FREQUENCY_WEIGHT)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let unpinned_sorted: Vec<FavoritePath> = scored.into_iter().map(|(p, _)| p).collect();

    info!("[get_sorted_favorite_paths] 非置顶路径数量: {}", unpinned_sorted.len());

    // 3. 合并两部分，最多返回 MAX_DISPLAY 条
    let mut result = pinned_paths;
    result.extend(unpinned_sorted);
    result.truncate(MAX_DISPLAY);

    info!("[get_sorted_favorite_paths] 返回路径数量: {}", result.len());
    Ok(result)
}
```

- [ ] **Step 2: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 无编译错误

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/db/favorite_paths.rs
git commit -m "feat(db): 修改排序逻辑，置顶路径优先"
```

---

### Task 5: 后端 - 注册新命令

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 导入新命令**

修改 import 行：

```rust
use db::favorite_paths::{record_path_usage_cmd, remove_favorite_path_cmd, get_sorted_favorite_paths_cmd, toggle_pin_path_cmd};
```

- [ ] **Step 2: 在 invoke_handler 中注册命令**

在 `invoke_handler` 中添加 `toggle_pin_path_cmd`：

```rust
.invoke_handler(tauri::generate_handler![
    // ... 其他命令
    get_sorted_favorite_paths_cmd,
    toggle_pin_path_cmd,  // 新增
    get_setting_cmd,
    // ...
])
```

- [ ] **Step 3: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 无编译错误

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(tauri): 注册 toggle_pin_path_cmd 命令"
```

---

### Task 6: 前端 - 更新类型定义

**Files:**
- Modify: `src/types/settings.ts`

- [ ] **Step 1: 更新 FavoritePath 类型**

修改 FavoritePath 接口：

```typescript
export interface FavoritePath {
  path: string           // 标准化的路径
  useCount: number       // 使用次数
  lastUsedAt: number     // 最近使用时间戳（毫秒）
  pinned: boolean        // 是否置顶
  pinnedAt: number | null // 置顶时间戳，未置顶时为 null
}
```

- [ ] **Step 2: TypeScript 编译验证**

Run: `npm run build`
Expected: 无 TypeScript 错误（可能有其他文件报错，后续修复）

- [ ] **Step 3: Commit**

```bash
git add src/types/settings.ts
git commit -m "feat(types): FavoritePath 添加 pinned/pinnedAt 字段"
```

---

### Task 7: 前端 - 添加 dbService 函数

**Files:**
- Modify: `src/services/dbService.ts`

- [ ] **Step 1: 更新 FavoritePath 接口定义**

修改 dbService.ts 中的 FavoritePath 接口：

```typescript
export interface FavoritePath {
  path: string
  useCount: number
  lastUsedAt: number
  pinned: boolean
  pinnedAt: number | null
}
```

- [ ] **Step 2: 添加 togglePinPath 函数**

在 `getSortedFavoritePaths` 函数后添加：

```typescript
export async function togglePinPath(path: string): Promise<FavoritePath> {
  return await invoke('toggle_pin_path_cmd', { path })
}
```

- [ ] **Step 3: TypeScript 编译验证**

Run: `npm run build`
Expected: 无 TypeScript 错误

- [ ] **Step 4: Commit**

```bash
git add src/services/dbService.ts
git commit -m "feat(service): 添加 togglePinPath 函数"
```

---

### Task 8: 前端 - 添加 settingsStore action

**Files:**
- Modify: `src/stores/settingsStore.ts`

- [ ] **Step 1: 导入 togglePinPath**

修改 import 行：

```typescript
import {
  setSetting,
  getAllSettings,
  recordPathUsage,
  removeFavoritePath,
  getSortedFavoritePaths,
  togglePinPath,
  FavoritePath,
} from '@/services/dbService'
```

- [ ] **Step 2: 在 SettingsState 接口中添加 action**

在 `SettingsState` 接口中添加：

```typescript
interface SettingsState extends AppSettings {
  initialized: boolean

  // Actions
  initialize: () => Promise<void>
  recordPathUsage: (path: string) => Promise<void>
  removeFavoritePath: (path: string) => Promise<void>
  togglePinPath: (path: string) => Promise<void>  // 新增
  // ... 其他 actions
}
```

- [ ] **Step 3: 实现 togglePinPath action**

在 store 实现中添加：

```typescript
togglePinPath: async (path: string) => {
  const normalized = normalizePath(path)
  await togglePinPath(normalized)
  const paths = await getSortedFavoritePaths()
  set({ favoritePaths: { paths } })
},
```

（在 `removeFavoritePath` 实现后添加）

- [ ] **Step 4: TypeScript 编译验证**

Run: `npm run build`
Expected: 无 TypeScript 错误

- [ ] **Step 5: Commit**

```bash
git add src/stores/settingsStore.ts
git commit -m "feat(store): settingsStore 添加 togglePinPath action"
```

---

### Task 9: 前端 - 创建 PathCard 组件

**Files:**
- Create: `src/components/dialogs/PathCard.tsx`

- [ ] **Step 1: 创建 PathCard 组件文件**

创建新文件 `src/components/dialogs/PathCard.tsx`：

```tsx
import { useState, useRef, useEffect } from "react"
import { cn } from "@/lib/utils"
import { Bookmark } from "lucide-react"
import type { FavoritePath } from "@/types"

interface PathCardProps {
  path: FavoritePath
  onPinToggle: () => void
  onDelete: () => void
  onSelect: () => void
}

export function PathCard({ path, onPinToggle, onDelete, onSelect }: PathCardProps) {
  const [showMenu, setShowMenu] = useState(false)
  const [menuPos, setMenuPos] = useState({ x: 0, y: 0 })
  const cardRef = useRef<HTMLDivElement>(null)

  // 提取最后一级目录名
  const displayName = path.path.split(/[/\\]/).filter(Boolean).pop() || path.path

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault()
    setMenuPos({ x: e.clientX, y: e.clientY })
    setShowMenu(true)
  }

  const handleClickOutside = (e: MouseEvent) => {
    if (cardRef.current && !cardRef.current.contains(e.target as Node)) {
      setShowMenu(false)
    }
  }

  useEffect(() => {
    if (showMenu) {
      document.addEventListener("click", handleClickOutside)
    }
    return () => {
      document.removeEventListener("click", handleClickOutside)
    }
  }, [showMenu])

  return (
    <div
      ref={cardRef}
      className={cn(
        "inline-flex items-center rounded overflow-hidden cursor-pointer",
        path.pinned
          ? "border-2 border-violet-500 bg-violet-50"
          : "border border-gray-200 bg-white hover:bg-gray-50"
      )}
      onContextMenu={handleContextMenu}
    >
      {/* 书签按钮 */}
      <button
        onClick={(e) => {
          e.stopPropagation()
          onPinToggle()
        }}
        className={cn(
          "p-1.5 border-r transition-colors",
          path.pinned
            ? "bg-violet-200 hover:bg-violet-300"
            : "bg-gray-50 hover:bg-violet-100"
        )}
        title={path.pinned ? "取消置顶" : "置顶"}
      >
        <Bookmark
          className={cn(
            "w-4 h-4",
            path.pinned
              ? "text-violet-600 fill-violet-600"
              : "text-gray-400 hover:text-violet-600"
          )}
        />
      </button>

      {/* 路径名称 */}
      <span
        onClick={onSelect}
        className="px-3 py-1 text-xs hover:underline"
      >
        {displayName}
      </span>

      {/* 右键菜单 */}
      {showMenu && (
        <div
          className="fixed bg-white border rounded-lg shadow-lg py-1 z-50"
          style={{
            left: menuPos.x,
            top: menuPos.y,
            minWidth: "120px"
          }}
        >
          <button
            onClick={() => {
              setShowMenu(false)
              onDelete()
            }}
            className="w-full px-3 py-2 text-sm text-left hover:bg-red-50 hover:text-red-600 flex items-center gap-2"
          >
            删除此路径
          </button>
          <button
            onClick={() => {
              setShowMenu(false)
              navigator.clipboard.writeText(path.path)
            }}
            className="w-full px-3 py-2 text-sm text-left hover:bg-gray-50 text-gray-600 flex items-center gap-2"
          >
            复制完整路径
          </button>
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: TypeScript 编译验证**

Run: `npm run build`
Expected: 无 TypeScript 错误

- [ ] **Step 3: Commit**

```bash
git add src/components/dialogs/PathCard.tsx
git commit -m "feat(ui): 创建 PathCard 组件实现置顶和右键删除"
```

---

### Task 10: 前端 - 修改 NewSessionDialog

**Files:**
- Modify: `src/components/dialogs/NewSessionDialog.tsx`

- [ ] **Step 1: 导入 PathCard 组件和 togglePinPath**

在文件顶部添加 import：

```tsx
import { PathCard } from "./PathCard"
import { useSettingsStore } from "@/stores/settingsStore"
```

- [ ] **Step 2: 添加 togglePinPath action**

在组件内获取 action：

```tsx
const togglePinPath = useSettingsStore((state) => state.togglePinPath)
```

- [ ] **Step 3: 替换常用路径显示部分**

将原有的常用路径显示部分（约 129-149 行）替换为：

```tsx
{/* 常用路径 */}
{favoritePaths.length > 0 && (
  <div className="flex flex-col gap-2">
    <label className="text-sm font-medium text-gray-700">常用路径</label>
    <div className="flex flex-wrap gap-2">
      {favoritePaths.map((fp) => (
        <PathCard
          key={fp.path}
          path={fp}
          onPinToggle={() => togglePinPath(fp.path)}
          onDelete={() => onRecordPathUsage && removeFavoritePath(fp.path)}
          onSelect={() => handleSelectFavoritePath(fp.path)}
        />
      ))}
    </div>
  </div>
)}
```

- [ ] **Step 4: 添加 removeFavoritePath 引用**

在组件顶部添加：

```tsx
const removeFavoritePath = useSettingsStore((state) => state.removeFavoritePath)
```

- [ ] **Step 5: TypeScript 编译验证**

Run: `npm run build`
Expected: 无 TypeScript 错误

- [ ] **Step 6: Commit**

```bash
git add src/components/dialogs/NewSessionDialog.tsx
git commit -m "feat(ui): NewSessionDialog 使用 PathCard 组件"
```

---

### Task 11: 集成测试验证

- [ ] **Step 1: 启动开发模式**

Run: `npm run tauri dev`
Expected: 应用正常启动

- [ ] **Step 2: 手动测试 - 置顶功能**

测试操作：
1. 打开新建 Session 对话框
2. 点击常用路径的书签按钮，验证路径变为紫色边框并移到顶部
3. 再次点击书签按钮，验证取消置顶，路径回到原位置

- [ ] **Step 3: 手动测试 - 删除功能**

测试操作：
1. 右键点击常用路径卡片
2. 点击"删除此路径"，验证路径从列表消失
3. 再次新建 session，验证删除的路径不再出现

- [ ] **Step 4: 手动测试 - 排序验证**

测试操作：
1. 置顶多个路径，验证按置顶时间降序排列（最新置顶在最前）
2. 验证置顶路径始终在非置顶路径前面

- [ ] **Step 5: 手动测试 - 复制路径功能**

测试操作：
1. 右键点击路径卡片
2. 点击"复制完整路径"
3. 验证剪贴板内容为完整路径

---

## Self-Review 检查清单

**Spec coverage:**
- ✓ 数据结构变更（pinned/pinnedAt 字段）
- ✓ 排序规则（置顶优先）
- ✓ UI 设计（书签按钮 + 右键菜单）
- ✓ 后端 API（toggle_pin_path_cmd）
- ✓ 前端实现（PathCard 组件）

**Placeholder scan:**
- 无 TBD/TODO
- 无 "add validation" 等模糊描述
- 所有代码步骤有完整实现

**Type consistency:**
- FavoritePath 结构体前后端字段名一致（path, useCount/use_count, lastUsedAt/last_used_at, pinned, pinnedAt/pinned_at）
- togglePinPath 函数签名前后端一致

**Missing coverage:**
- 需要检查 RunningTab.tsx 是否也传递了 favoritePaths（已在设计中说明）
- 需要确认 removeFavoritePath 在 Task 10 中的调用方式

---

## 执行后提交

实现完成后，创建一个汇总 commit：

```bash
git add -A
git commit -m "feat: 实现常用路径 Pin/Delete 功能

- 后端添加 pinned/pinned_at 字段和 toggle_pin_path 命令
- 前端创建 PathCard 组件实现书签按钮和右键菜单
- 置顶路径优先排序，最新置顶排最前
- 右键菜单支持删除和复制完整路径"
```