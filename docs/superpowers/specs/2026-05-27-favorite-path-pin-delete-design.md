# 常用路径 Pin/Delete 功能设计

## 概述

在新建 Session 对话框中，为常用路径列表增加置顶和删除功能，允许用户自定义路径排序和管理。

## 数据结构变更

### FavoritePath 新增字段

```typescript
interface FavoritePath {
  path: string           // 标准化路径
  useCount: number       // 使用次数
  lastUsedAt: number     // 最近使用时间戳（毫秒）
  pinned: boolean        // 是否置顶
  pinnedAt: number | null // 置顶时间戳（毫秒），未置顶时为 null
}
```

### SQLite 表结构变更

```sql
ALTER TABLE favorite_paths ADD COLUMN pinned INTEGER DEFAULT 0;
ALTER TABLE favorite_paths ADD COLUMN pinned_at INTEGER DEFAULT NULL;
```

## 排序规则

```
结果 = [置顶路径（按 pinnedAt 降序）] + [非置顶路径（按原算法）]
```

- **置顶路径**：按 `pinnedAt` 降序排列，最新置顶的排在最前
- **非置顶路径**：按原有算法排序（`recency × 0.6 + frequency × 0.4`）
- 置顶路径不参与原有算法计算，始终固定在列表顶部

## UI 设计

### 路径卡片结构

```
[书签按钮] [路径名称]
```

每个路径显示为一个横向卡片，左侧为书签按钮，右侧为路径名称。

### 书签按钮

- **图标**：书签图标（bookmark）
- **未置顶状态**：灰色空心图标，hover 变紫色
- **已置顶状态**：紫色填充图标，卡片整体紫色边框+紫色背景
- **Tooltip**：未置顶显示"置顶"，已置顶显示"取消置顶"
- **点击行为**：切换置顶状态

### 路径名称

- **显示内容**：只显示路径最后一级目录名（如 `claude-fleet-sp`）
- **点击行为**：填入工作目录输入框（原有行为保持）

### 右键菜单

右键点击卡片任意位置弹出菜单，包含：
- **删除此路径**：点击直接删除，无需确认对话框
- **复制完整路径**：可选功能，方便用户获取完整路径

### 样式规范

**置顶状态样式：**
- 边框：`border-violet-500`（2px）
- 背景：`bg-violet-50`
- 书签按钮背景：`bg-violet-200`
- 书签图标颜色：`text-violet-600`

**非置顶状态样式：**
- 边框：`border-gray-200`（1px）
- 背景：`bg-white`，hover `bg-gray-50`
- 书签按钮背景：`bg-gray-50`，hover `bg-violet-100`
- 书签图标颜色：`text-gray-400`，hover `text-violet-600`

## 后端 API

### 新增命令

```rust
// 切换路径置顶状态
#[tauri::command]
fn toggle_pin_path_cmd(path: String) -> Result<FavoritePath, String>
```

**逻辑：**
- 查询路径是否存在
- 若 `pinned = false`：设置 `pinned = true`，`pinned_at = 当前时间戳`
- 若 `pinned = true`：设置 `pinned = false`，`pinned_at = null`
- 返回更新后的 FavoritePath

### 修改命令

```rust
// 获取排序后的常用路径（返回包含 pinned 和 pinnedAt 字段）
#[tauri::command]
fn get_sorted_favorite_paths_cmd() -> Result<Vec<FavoritePath>, String>
```

**排序逻辑：**
1. 置顶路径：`WHERE pinned = 1 ORDER BY pinned_at DESC`
2. 非置顶路径：`WHERE pinned = 0` + 原有算法排序
3. 合并两部分，最多返回 10 条

### 复用命令

```rust
// 删除路径记录（已有）
#[tauri::command]
fn remove_favorite_path_cmd(path: String) -> Result<(), String>
```

## 前端实现

### dbService.ts 新增

```typescript
export async function togglePinPath(path: string): Promise<FavoritePath> {
  return await invoke('toggle_pin_path_cmd', { path })
}
```

### settingsStore.ts 新增

```typescript
togglePinPath: async (path: string) => {
  await togglePinPath(path)
  const paths = await getSortedFavoritePaths()
  set({ favoritePaths: { paths } })
}
```

### NewSessionDialog.tsx 修改

**组件结构：**

```tsx
<div className="flex flex-wrap gap-2">
  {favoritePaths.map((fp) => (
    <PathCard
      key={fp.path}
      path={fp}
      onPinToggle={() => handlePinToggle(fp.path)}
      onDelete={() => handleDelete(fp.path)}
      onSelect={() => handleSelectPath(fp.path)}
    />
  ))}
</div>
```

**PathCard 组件：**
- 左侧书签按钮，右侧路径名称
- 右键菜单（使用 Radix UI DropdownMenu 或自定义实现）
- hover tooltip 显示操作提示

**处理函数：**
- `handlePinToggle(path)`：调用 `settingsStore.togglePinPath`
- `handleDelete(path)`：调用 `settingsStore.removeFavoritePath`
- `handleSelectPath(path)`：填入工作目录

## 实现步骤

1. **后端**：修改 `favorite_paths.rs`，添加 `pinned` 和 `pinned_at` 字段，实现 `toggle_pin_path` 命令
2. **后端**：修改排序逻辑，置顶路径优先
3. **数据库迁移**：添加 ALTER TABLE 语句（在初始化时检测并添加列）
4. **前端**：dbService.ts 添加 `togglePinPath` 函数
5. **前端**：settingsStore.ts 添加 `togglePinPath` action
6. **前端**：修改 NewSessionDialog.tsx，实现 PathCard 组件和右键菜单

## 测试要点

- 置顶/取消置顶功能正常
- 置顶路径始终排在非置顶路径前面
- 最新置顶的路径排在置顶区域最前
- 删除路径后列表正确更新
- 右键菜单正确触发
- 路径名称显示最后一级目录名
- 点击路径名称填入工作目录正常