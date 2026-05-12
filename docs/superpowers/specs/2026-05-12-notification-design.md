# 通知功能设计

## 背景

当前应用的通知功能存在以下问题：
- 提示音文件不存在（`public/sounds/` 目录为空），导致播放失败
- Rust 端 `send_notification` 命令未实现，只是占位符
- 设置 UI 中缺少通知开关的展示

## 设计目标

实现可靠的通知功能：
1. 简单提示音（base64 内嵌）
2. Windows 系统通知（Tauri 插件）
3. 设置中可独立开关

## 状态模型

Claude Code 定义三种状态：`busy`、`idle`、`waiting`
- `busy`：Agent 正在运行/处理
- `idle`：等待用户输入（任务完成后）
- `waiting`：等待用户输入（运行中被阻断）

我们的应用将 `idle` 和 `waiting` 合并为同一个"等待输入"状态：
- 显示统一：StatusBadge 都显示"等待输入"
- 通知统一：从 `busy → idle/waiting` 时触发通知，行为一致

## 技术方案

### 1. 提示音实现

**方案**：内嵌 base64 编码的短提示音

```
前端代码
  ↓
HTMLAudioElement + base64 data URI
  ↓
播放短提示音（约 1-2 秒）
```

- 音频数据直接编码在 TypeScript 中
- 无需外部文件，加载可靠
- 默认音量 0.5

### 2. Windows 系统通知

**方案**：使用 `tauri-plugin-notification` 插件

```
Rust 端
  ↓
tauri-plugin-notification 初始化
  ↓
前端调用 @tauri-apps/plugin-notification
  ↓
Windows Toast 通知
```

依赖添加：
- Rust: `tauri-plugin-notification` crate
- 前端: `@tauri-apps/plugin-notification` npm 包

### 3. 设置 UI

在 `SettingsDialog.tsx` 中增加通知设置区域：

```tsx
{/* 通知设置 */}
<div className="space-y-3 border-t pt-4">
  <label className="text-sm font-medium">通知设置</label>

  <div className="flex items-center justify-between">
    <div className="space-y-0.5">
      <Label htmlFor="notification-sound">提示音</Label>
      <p className="text-xs text-muted-foreground">
        Session 进入等待状态时播放提示音
      </p>
    </div>
    <Switch
      id="notification-sound"
      checked={notificationSound}
      onCheckedChange={setNotificationSound}
    />
  </div>

  <div className="flex items-center justify-between">
    <div className="space-y-0.5">
      <Label htmlFor="notification-desktop">桌面通知</Label>
      <p className="text-xs text-muted-foreground">
        发送 Windows 系统通知
      </p>
    </div>
    <Switch
      id="notification-desktop"
      checked={notificationDesktop}
      onCheckedChange={setNotificationDesktop}
    />
  </div>
</div>
```

使用 `Switch` 组件（滑动开关）而非 Checkbox，更适合二元设置项。两个开关独立控制。

### 4. 数据流

```
Claude Code 更新 session 文件 (status 变化)
    ↓
Rust sessions_watcher 检测文件变化
    ↓
判断: busy → idle/waiting?
    ↓ (是)
发送事件 session_waiting_input
    ↓
前端 useNotification hook 接收事件
    ↓
根据设置决定:
  - 仅提示音 → playNotificationSound()
  - 仅桌面通知 → sendNotification()
  - 两者都开启 → 同时触发
```

现有代码已实现事件发送逻辑，只需修复提示音和桌面通知的实际功能。

## 文件改动

### Rust 端

| 文件 | 改动 |
|------|------|
| `src-tauri/Cargo.toml` | 添加 `tauri-plugin-notification` 依赖 |
| `src-tauri/src/lib.rs` | 初始化 notification 插件 |
| `src-tauri/src/commands/session.rs` | 移除或重写 `send_notification` 命令 |

### 前端

| 文件 | 改动 |
|------|------|
| `package.json` | 添加 `@tauri-apps/plugin-notification` |
| `src/services/notificationService.ts` | 重写：base64 提示音 + Tauri 通知插件调用 |
| `src/components/dialogs/SettingsDialog.tsx` | 添加通知设置 UI |
| `src/components/ui/switch.tsx` | 新增 Switch 组件（shadcn/ui） |
| `src/components/ui/label.tsx` | 新增 Label 组件（shadcn/ui） |

## 验收标准

1. 提示音能在 session 进入等待状态时正常播放
2. Windows Toast 通知能正常弹出，显示在通知中心
3. 设置中可独立开关提示音和桌面通知
4. 关闭开关后对应通知不再触发