# Claude Fleet 功能增强设计文档

**日期**: 2026-05-18
**版本**: 1.0
**状态**: 待审阅

## 概述

本设计文档涵盖三个功能需求和一个存储重构：

1. **功能一**：Session 自定义命名能力
2. **功能二**：按 Workspace 分组查看收藏 Session
3. **功能三**：对话记录底部截断问题修复
4. **存储重构**：localStorage 迁移至 SQLite

## 一、存储重构

### 1.1 背景

当前 Claude Fleet 使用 localStorage 存储收藏列表和应用设置，存在以下问题：

- 大小限制约 5MB
- 数据分散在前端，不便于后端统一管理
- 迁移和备份不友好
- 无法支持更复杂的数据关系

### 1.2 数据库位置

```
~/.claude-fleet/data/claude-fleet.db
```

与现有日志目录 `~/.claude-fleet/logs/` 结构一致。

### 1.3 表结构设计

```sql
-- Session 自定义名称
CREATE TABLE sessions_meta (
  session_id    TEXT PRIMARY KEY,
  custom_name   TEXT,
  created_at    INTEGER,  -- 毫秒时间戳
  updated_at    INTEGER   -- 毫秒时间戳
);

-- 收藏列表
CREATE TABLE favorites (
  session_id    TEXT PRIMARY KEY,
  added_at      INTEGER   -- 收藏时间戳
);

-- 常用路径
CREATE TABLE favorite_paths (
  path          TEXT PRIMARY KEY,
  use_count     INTEGER DEFAULT 1,
  last_used_at  INTEGER   -- 毫秒时间戳
);

-- 应用设置（KV 存储）
CREATE TABLE app_settings (
  key           TEXT PRIMARY KEY,
  value         TEXT      -- JSON 格式或简单值
);
```

### 1.4 迁移策略

**首次启动检测**：
- 应用启动时检查 SQLite 数据库是否存在
- 若不存在，创建数据库和表结构
- 检查 localStorage 中是否有旧数据
- 若有，读取并写入 SQLite，然后清除 localStorage

**数据迁移映射**：
| localStorage Key | SQLite 表 |
|-----------------|----------|
| `claude-fleet-favorites` (Set<string>) | `favorites` |
| `claude-fleet-settings.favoritePaths.paths` | `favorite_paths` |
| `claude-fleet-settings.*` (其他字段) | `app_settings` |

### 1.5 后端实现

**Rust 依赖**：
- `rusqlite` - SQLite 操作库

**模块结构**：
```
src-tauri/src/
  db/
    mod.rs           -- 数据库模块入口
    schema.rs        -- 表结构定义和初始化
    sessions_meta.rs -- sessions_meta CRUD
    favorites.rs     -- favorites CRUD
    favorite_paths.rs -- favorite_paths CRUD
    settings.rs      -- app_settings CRUD
    migration.rs     -- localStorage 迁移逻辑
```

**Tauri 命令**：
```rust
// sessions_meta
fn set_session_name(session_id: String, name: String) -> Result<(), String>
fn get_session_name(session_id: String) -> Result<Option<String>, String>
fn delete_session_name(session_id: String) -> Result<(), String>

// favorites
fn add_favorite(session_id: String) -> Result<(), String>
fn remove_favorite(session_id: String) -> Result<(), String>
fn is_favorite(session_id: String) -> Result<bool, String>
fn get_all_favorites() -> Result<Vec<String>, String>

// favorite_paths
fn record_path_usage(path: String) -> Result<(), String>
fn remove_favorite_path(path: String) -> Result<(), String>
fn get_sorted_favorite_paths() -> Result<Vec<FavoritePath>, String>

// settings
fn get_setting(key: String) -> Result<Option<String>, String>
fn set_setting(key: String, value: String) -> Result<(), String>
```

### 1.6 前端调整

**移除 Zustand persist**：
- `favoriteStore.ts` 改为普通 Zustand store，数据通过 Tauri invoke 获取
- `settingsStore.ts` 同样移除 persist，改为调用后端

**初始化流程**：
```typescript
// App 启动时
async function initializeStores() {
  const favorites = await invoke<string[]>('get_all_favorites')
  favoriteStore.setState({ favorites: new Set(favorites) })

  const settings = await invoke<Record<string, string>>('get_all_settings')
  settingsStore.setState(parseSettings(settings))
}
```

---

## 二、功能一：Session 自定义命名

### 2.1 功能描述

为 Claude Code session 提供自定义命名能力，作为 Claude Code 自身命名功能的补充。

### 2.2 名称显示优先级

当显示 session 名称时，按以下优先级确定：

1. **Claude Fleet 自定义名称**（最高优先级）- 来自 `sessions_meta.custom_name`
2. **Claude Code 自定义名称** - 来自 JSONL 文件的 `type: "custom-title"` 条目
3. **第一条用户消息** - session 的第一条 user 消息内容
4. **工作目录名称** - `projectDir` 路径的最后一部分
5. **Session ID**（最低优先级）- UUID 的前几位

### 2.3 数据流

```
前端请求 session 列表
    ↓
后端读取 Claude Code JSONL 文件获取 title
    ↓
后端查询 sessions_meta 表获取 custom_name
    ↓
合并数据，custom_name 覆盖 title（若存在）
    ↓
返回给前端显示
```

### 2.4 编辑入口

| 位置 | 触发方式 | 操作 |
|-----|---------|-----|
| Running Tab SessionCard | 双击名称 / 右键菜单 | 编辑名称 |
| Management Tab SessionListItem | 双击名称 / 右键菜单 | 编辑名称 |
| SessionDetail 详情页 | 双击名称 / 右键菜单 | 编辑名称 |

### 2.5 UI 交互设计

**编辑模式**：
- 双击名称后，名称文本变为可编辑的 input 框
- 输入框自动聚焦，选中全部文本
- 按 Enter 保存，按 Escape 取消
- 点击输入框外区域保存

**右键菜单项**：
- "重命名" - 进入编辑模式
- "删除名称" - 清除 custom_name，恢复默认显示

**空名称处理**：
- 若用户清空输入并保存，则删除 custom_name 记录
- 显示时回退到下一优先级的名称

### 2.6 搜索支持

`custom_name` 作为搜索字段之一：
- 搜索匹配范围：title + custom_name + projectDir + sessionId
- 在搜索结果中，若 session 有 custom_name，优先显示

### 2.7 后端实现要点

**修改现有命令**：
- `list_sessions_optimized` - 返回结果中增加 `custom_name` 字段
- `list_running` - 返回结果中增加 `custom_name` 字段

**新增命令**：
```rust
#[tauri::command]
fn set_session_name(session_id: String, name: String) -> Result<(), String>

#[tauri::command]
fn delete_session_name(session_id: String) -> Result<(), String>
```

### 2.8 前端实现要点

**类型修改**：
```typescript
// session.ts
interface SessionMeta {
  // 现有字段...
  customName?: string  // Claude Fleet 自定义名称
}
```

**显示逻辑**：
```typescript
function getDisplayName(session: SessionMeta | RunningSession): string {
  return session.customName
    || session.title
    || session.projectDir?.split(/[\\/]/).pop()
    || session.sessionId.slice(0, 8)
}
```

**组件修改**：
- `SessionCardNew.tsx` - 使用 getDisplayName，添加双击/右键编辑
- `SessionListItem.tsx` - 同上
- `SessionDetail.tsx` - 同上

---

## 三、功能二：按 Workspace 分组查看

### 3.1 功能描述

在 Session 管理的"仅收藏模式"下，提供按 workspace 分组的树形视图。

### 3.2 分组依据

从 session 文件路径推断 workspace：
```
~/.claude/projects/<project-name>/<session-id>.jsonl
```

`<project-name>` 作为分组标识。

**project-name 解码**：
- Claude Code 对项目路径进行编码生成 project-name
- 若无法解码为原始路径，直接显示 project-name 作为分组名称

**无 projectDir 的 session**：
- 尝试从 `sourcePath` 字段提取 project-name
- 若仍无法获取，归入"未知项目"分组

### 3.3 视图切换

**入口**：
- 仅在"仅收藏模式"下显示切换按钮
- 按钮位于列表顶部工具栏

**两种视图**：
- **列表视图**（默认）- 现有的扁平列表
- **分组视图** - 按 workspace 分组的树形结构

**切换按钮样式**：
- 使用图标按钮（列表图标 / 树形图标）
- 当前视图高亮显示

### 3.4 树形结构 UI

```
┌─ workspace-group-item ──────────────────┐
│ [展开/折叠图标] workspace-name    (3)   │
├─────────────────────────────────────────┤
│   ┌─ session-item ──────────────┐       │
│   │ session-name  ●收藏  时间   │       │
│   └─────────────────────────────┘       │
│   ┌─ session-item ──────────────┐       │
│   │ session-name  ●收藏  时间   │       │
│   └─────────────────────────────┘       │
└─────────────────────────────────────────┘
```

**展开/折叠**：
- 点击 workspace 行切换展开/折叠状态
- 展开状态：显示下属 session 列表
- 折叠状态：仅显示 workspace 行和 session 数量

**状态持久化**：
- 展开/折叠状态存储在组件 state，不持久化
- 切换视图或刷新后恢复默认（全部展开）

### 3.5 数据获取

**后端新增命令**：
```rust
#[tauri::command]
fn get_favorites_grouped_by_workspace() -> Result<Vec<WorkspaceGroup>, String>

struct WorkspaceGroup {
  workspace_name: String,    // project-name 或解码后的路径
  sessions: Vec<SessionMeta>,
}
```

**前端也可自行分组**：
- 获取收藏 session 列表后，前端提取 sourcePath 进行分组
- 后端提供辅助函数解码 project-name

### 3.6 前端组件

**新增组件**：
```
src/components/management/
  GroupedSessionList.tsx    -- 分组视图容器
  WorkspaceGroupItem.tsx    -- workspace 分组项（可展开/折叠）
```

**修改组件**：
- `ManagementTab.tsx` - 添加视图切换逻辑

---

## 四、功能三：对话记录底部截断修复

### 4.1 问题描述

Session 详情页的对话记录部分，最后一条消息的底部边框和时间戳被截断显示不全，即使扩大窗口也无法解决。

### 4.2 问题根源

`SessionDetail.tsx` 第 152-168 行布局：
```tsx
<div className="flex-1 min-h-0 min-w-0 overflow-hidden">
  {/* 对话记录标题 - 占用了一部分高度 */}
  <div className="flex items-center justify-between px-4 py-2 border-b bg-white min-w-0">
    <h3>对话记录</h3>
    <Button>刷新</Button>
  </div>

  {/* ConversationView - 没有明确高度声明 */}
  <ConversationView messages={...} loading={...} />
</div>
```

`ConversationView` 内的 `ScrollArea` 使用 `h-full`，但父容器是 flex 子元素且没有 `flex-1` 或 `h-full`，导致 ScrollArea 无法正确计算高度。

### 4.3 修复方案

给 `ConversationView` 的包裹容器添加 `flex-1 min-h-0`：

```tsx
<div className="flex-1 min-h-0 min-w-0 overflow-hidden">
  {/* 对话记录标题 */}
  <div className="flex items-center justify-between px-4 py-2 border-b bg-white min-w-0">
    ...
  </div>

  {/* ConversationView - 添加 flex-1 min-h-0 */}
  <div className="flex-1 min-h-0 min-w-0">
    <ConversationView messages={...} loading={...} />
  </div>
</div>
```

### 4.4 验证方法

- 滚动到最后一条消息
- 确认时间戳和底部边框完整可见
- 缩小窗口高度，确认滚动功能正常

---

## 五、实现优先级

| 序号 | 功能 | 优先级 | 说明 |
|-----|-----|-------|-----|
| 1 | 存储重构 | P0 | 其他功能依赖 SQLite 基础设施 |
| 2 | 对话截断修复 | P0 | 独立 bug，可并行实现 |
| 3 | Session 命名 | P1 | 核心功能，依赖存储重构完成 |
| 4 | Workspace 分组 | P2 | 依赖收藏功能（存储重构）和命名功能（显示优化） |

---

## 六、风险和注意事项

### 6.1 数据迁移风险

- localStorage 数据可能损坏或不完整
- 需要添加容错处理，确保迁移失败不影响应用启动
- 建议保留迁移日志，便于用户手动恢复

### 6.2 性能考虑

- SQLite 操作应在后端异步执行
- 大量 session 时，分组视图需要考虑虚拟滚动
- 搜索性能：custom_name 字段需纳入现有搜索逻辑

### 6.3 兼容性

- 旧版本用户升级时自动迁移数据
- 新版本用户直接使用 SQLite
- 不支持降级（SQLite 数据无法回迁到 localStorage）