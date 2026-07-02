# Worktree 基准分支按仓库记忆

## 背景

新建 worktree 对话框（`CreateWorktreeDialog`）会记忆用户上次选择的"基于分支 / ref"（baseRef），便于下次默认选中。当前实现把单个全局字符串存于 `app_settings.lastBaseRef`，不区分仓库。因此在 A 仓库创建后再到 B 仓库创建，A 的记忆就被 B 覆盖；回到 A 时不记得上次选择。

## 目标

- 按仓库粒度记忆上次选中的 baseRef。
- 跨 claude-fleet 重启持久化（已有 SQLite `app_settings` 表天然满足）。
- 清除旧的全局 `lastBaseRef` 数据。

## 非目标

- 不迁移旧 `lastBaseRef` 值（无仓库信息，迁移无意义）。
- 不为该字段新建专用 DB 表（一仓一字符串，KV 足够）。

## 设计

### 存储

复用现有 `app_settings` KV 表，key 形如：

```
worktree.baseRef.<normalizedRepoPath>
```

`normalizedRepoPath` 由 `settingsStore` 中已有的 `normalizePath` 生成（去末尾斜杠、盘符字母大写），保证同一仓库不同写法落到同一 key。`getSetting`/`setSetting` 命令已存在，无需新建读写命令。

### 后端改动（src-tauri）

`db/settings.rs`：新增 `delete_setting(key)` 与 `#[tauri::command] delete_setting_cmd`，并在 `lib.rs` 的 `invoke_handler` 注册。仅用于清除旧 `lastBaseRef` 行，同时是通用能力。

### 前端改动

- `services/dbService.ts`：新增 `deleteSetting(key)`。
- `stores/settingsStore.ts`：
  - 移除 `AppSettings.lastBaseRef`、`setLastBaseRef` action、`DEFAULT_SETTINGS.lastBaseRef`、`initialize` 中对 `lastBaseRef` 的解析。
  - `initialize` 中若 `settings['lastBaseRef']` 存在则调用 `deleteSetting('lastBaseRef')` 清除（幂等，清完不再触发）。
  - 导出 `normalizePath`（当前为模块私有函数）。
- `components/worktree/CreateWorktreeDialog.tsx`：
  - 移除 `useSettingsStore` 的 `lastBaseRef` / `setLastBaseRef` 依赖。
  - 对话框打开时 `getSetting("worktree.baseRef." + normalizePath(repoPath))`，有值则作为初始 `baseRef`。
  - `handleCreate` 成功后 `setSetting(同 key, effectiveBaseRef)`。
  - 既有 `repoInfo` 校验逻辑保留：记忆值在当前仓库不存在时回退到 `upstream/<default>` → `origin/<default>` → `default`。

### 数据流

```
打开对话框 → getSetting(worktree.baseRef.<repo>) → 有值且有效 → 设为 baseRef
                                                            无值/无效 → 走 repoInfo 默认回退
创建成功   → setSetting(worktree.baseRef.<repo>, effectiveBaseRef)
```

## 测试

- 后端：`db/settings.rs` 新增 `#[cfg(test)]` 测试，覆盖 `delete_setting` 的往返（set → delete → get 返回 None）。`cargo test` 运行。
- 前端：项目无前端测试运行器（package.json 未含 vitest/jest）。key 生成依赖 `normalizePath` 纯函数，靠 `npx tsc --noEmit` 类型检查 + 上述后端测试兜底；不引入新测试框架（超出本任务范围）。

## 清理

移除本次改动产生的孤儿：`lastBaseRef` 字段、`setLastBaseRef` action、对话框中的相关引用。DB 中残留的旧 `lastBaseRef` 行由 `initialize` 主动删除。
