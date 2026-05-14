# 工作目录悬浮增强设计

## 概述

在工作目录悬浮组件中增加"打开目录"和"打开 VSCode"按钮，并将该组件统一应用到 Session 管理页详情和运行中页面卡片。

## 背景

当前实现：
- SessionDetail.tsx 有悬浮层显示完整路径 + 复制按钮
- SessionCard.tsx 仅用 `title` 属性显示完整路径，无悬浮层

用户希望：
- 悬浮层增加快速操作按钮（打开目录、打开 VSCode）
- 两处使用一致的交互体验

## 需求范围

- 仅涉及工作目录悬浮显示功能
- 仅支持 Windows 平台（explorer 打开目录、code 命令打开 VSCode）
- 按钮样式：纯图标，与现有复制按钮风格统一
- 错误处理：Toast 提示

## 设计方案

### 方案选择：抽取共享组件

创建 `PathHoverDisplay` 共享组件，SessionDetail 和 SessionCard 都使用该组件。优点：
- 代码复用，一处修改多处生效
- 保证两个页面的交互体验完全一致
- 更易于维护和扩展

---

## 后端设计

### 新增 Tauri 命令

**文件：`src-tauri/src/commands/terminal.rs`**

新增两个命令：

```rust
/// 打开目录（Windows 用 explorer）
#[tauri::command]
pub fn open_directory(path: String) -> Result<(), String> {
    info!("[open_directory] 开始，路径: {}", path);

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
        info!("[open_directory] 完成");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Err("仅支持 Windows 平台".to_string())
    }
}

/// 在 VSCode 中打开目录
#[tauri::command]
pub fn open_in_vscode(path: String) -> Result<(), String> {
    info!("[open_in_vscode] 开始，路径: {}", path);

    #[cfg(target_os = "windows")]
    {
        Command::new("code")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开 VSCode 失败: {}。请确保 VSCode 已安装且 'code' 命令在 PATH 中", e))?;
        info!("[open_in_vscode] 完成");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Err("仅支持 Windows 平台".to_string())
    }
}
```

### 注册命令

**文件：`src-tauri/src/lib.rs`**

在 `invoke_handler` 中注册新命令：

```rust
.invoke_handler(tauri::generate_handler![
    // ... 现有命令
    open_directory,
    open_in_vscode,
])
```

---

## 前端设计

### 服务层

**文件：`src/services/terminalService.ts`**

新增两个服务函数：

```typescript
/**
 * 打开目录（Windows 资源管理器）
 */
export async function openDirectory(path: string): Promise<void> {
  try {
    await invoke('open_directory', { path })
  } catch (error) {
    throw new Error(String(error))
  }
}

/**
 * 在 VSCode 中打开目录
 */
export async function openInVSCode(path: string): Promise<void> {
  try {
    await invoke('open_in_vscode', { path })
  } catch (error) {
    throw new Error(String(error))
  }
}
```

### 共享组件

**文件：`src/components/common/PathHoverDisplay.tsx`**

组件 Props：

```typescript
interface PathHoverDisplayProps {
  path: string           // 完整路径
  displayName?: string   // 显示名称（默认取路径最后一段）
  className?: string     // 外层容器样式
  showOnHover?: boolean  // 是否悬浮显示（默认 true）
}
```

组件结构：

```tsx
<div className="relative group min-w-0 max-w-[400px]">
  {/* 基础显示 */}
  <div className="flex items-center gap-1.5 cursor-default">
    <FolderOpen className="w-4 h-4 shrink-0" />
    <span className="truncate">{displayName || path.split(/[\\/]/).pop()}</span>
  </div>

  {/* 悬浮层 */}
  <div className="absolute left-0 top-full mt-1 hidden group-hover:flex items-center gap-2 bg-white border rounded-md px-3 py-1.5 shadow-md z-10">
    <span className="text-xs text-gray-600 truncate max-w-[300px]">{path}</span>
    <div className="flex items-center gap-1 shrink-0">
      {/* 复制 */}
      <Button variant="ghost" size="sm" onClick={() => navigator.clipboard.writeText(path)}>
        <Copy className="w-3 h-3" />
      </Button>
      {/* 打开目录 */}
      <Button variant="ghost" size="sm" onClick={handleOpenDirectory}>
        <FolderOpen className="w-3 h-3" />
      </Button>
      {/* 打开 VSCode */}
      <Button variant="ghost" size="sm" onClick={handleOpenVSCode}>
        <Code className="w-3 h-3" />
      </Button>
    </div>
  </div>
</div>
```

按钮图标：
- 复制：`Copy`（lucide-react）
- 打开目录：`FolderOpen`（lucide-react）
- 打开 VSCode：`Code`（lucide-react）

### 集成点

**SessionDetail.tsx（第116-137行）**

替换现有悬浮路径实现：

```tsx
// 替换为：
{session.projectDir && (
  <PathHoverDisplay path={session.projectDir} />
)}
```

**SessionCard.tsx（第148-155行）**

替换 cwd 显示：

```tsx
// 替换为：
<PathHoverDisplay
  path={session.cwd}
  className="max-w-[200px]"
/>
```

---

## 错误处理

### 后端错误

- 目录不存在：explorer 会显示错误窗口，不返回错误
- VSCode 未安装：返回 `"打开 VSCode 失败: {error}。请确保 VSCode 已安装且 'code' 命令在 PATH 中"`

### 前端处理

- 捕获错误后使用 `toast.error()` 显示提示
- 用户可从 Toast 了解失败原因

---

## 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src-tauri/src/commands/terminal.rs` | 修改 | 新增 `open_directory`、`open_in_vscode` 命令 |
| `src-tauri/src/lib.rs` | 修改 | 注册新命令 |
| `src/services/terminalService.ts` | 修改 | 新增 `openDirectory`、`openInVSCode` 函数 |
| `src/services/index.ts` | 修改 | 导出新函数 |
| `src/components/common/PathHoverDisplay.tsx` | 新增 | 共享悬浮路径组件 |
| `src/components/management/SessionDetail.tsx` | 修改 | 使用 PathHoverDisplay 组件 |
| `src/components/running/SessionCard.tsx` | 修改 | 使用 PathHoverDisplay 组件 |

---

## 测试要点

1. SessionDetail 悬浮路径显示完整路径 + 三个按钮
2. SessionCard 悬浮路径显示完整路径 + 三个按钮
3. 点击复制按钮，路径复制到剪贴板
4. 点击打开目录，Windows 资源管理器打开对应目录
5. 点击打开 VSCode，VSCode 打开对应目录
6. VSCode 未安装时显示 Toast 错误提示