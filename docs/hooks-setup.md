# Claude Code 钩子配置指南

Claude Fleet 通过 Claude Code 的全局钩子功能接收 session 状态变化通知，实现声音和桌面通知提醒。

## 配置步骤

### 1. 编辑 Claude Code 配置文件

配置文件位置：
- Windows: `C:\Users\<username>\.claude\settings.json`
- macOS/Linux: `~/.claude/settings.json`

### 2. 添加钩子配置

在 `settings.json` 中添加以下内容：

```json
{
  "hooks": {
    "SessionStart": {
      "command": "curl -X POST http://localhost:9527/hook -H \"Content-Type: application/json\" -d \"{\\\"event_type\\\":\\\"session_start\\\",\\\"session_id\\\":\\\"$SESSION_ID\\\",\\\"working_directory\\\":\\\"$WORKING_DIR\\\",\\\"timestamp\\\":\\\"$TIMESTAMP\\\"}\""
    },
    "WaitingForInput": {
      "command": "curl -X POST http://localhost:9527/hook -H \"Content-Type: application/json\" -d \"{\\\"event_type\\\":\\\"waiting_input\\\",\\\"session_id\\\":\\\"$SESSION_ID\\\",\\\"working_directory\\\":\\\"$WORKING_DIR\\\",\\\"timestamp\\\":\\\"$TIMESTAMP\\\"}\""
    },
    "SessionEnd": {
      "command": "curl -X POST http://localhost:9527/hook -H \"Content-Type: application/json\" -d \"{\\\"event_type\\\":\\\"session_end\\\",\\\"session_id\\\":\\\"$SESSION_ID\\\",\\\"working_directory\\\":\\\"$WORKING_DIR\\\",\\\"timestamp\\\":\\\"$TIMESTAMP\\\"}\""
    }
  }
}
```

### 3. 启动 Claude Fleet

Claude Fleet 启动时会自动启动钩子接收服务，监听 session 状态变化。

## 环境变量

钩子命令可以使用以下环境变量：
- `$SESSION_ID`: 当前 session ID
- `$WORKING_DIR`: 工作目录路径
- `$TIMESTAMP`: 事件时间戳

## 备用方案

如果 Claude Code 钩子不可用或配置失败，Claude Fleet 会通过以下方式检测状态变化：

### 自动轮询检测

Claude Fleet 每 2 秒轮询检查 session 文件变化，检测 `waiting_input` 状态：
- 新的 running 状态 session 进入时触发通知
- Session 状态变化时更新通知状态

### 手动通知测试

在应用中可以测试通知功能是否正常工作：
```typescript
import { sendDesktopNotification } from '@/services'

// 发送测试通知
sendDesktopNotification({
  title: '测试通知',
  body: '这是一条测试消息',
  sound: true
})
```

## 通知设置

在 Claude Fleet 的设置中可以配置：
- **声音通知**: 启用/禁用提示音
- **桌面通知**: 启用/禁用系统桌面通知

## 注意事项

1. **Windows 平台**: 确保已安装 `curl` 或使用 PowerShell 的 `Invoke-WebRequest` 替代
2. **macOS/Linux**: curl 通常已预装
3. **通知权限**: 首次使用桌面通知时，浏览器会请求权限授权
4. **提示音文件**: 需要在 `public/sounds/notification.mp3` 放置音频文件

## PowerShell 替代方案（Windows）

如果没有 curl，可以使用 PowerShell：

```json
{
  "hooks": {
    "WaitingForInput": {
      "command": "powershell -Command \"Invoke-WebRequest -Uri 'http://localhost:9527/hook' -Method POST -ContentType 'application/json' -Body '{\"event_type\":\"waiting_input\",\"session_id\":\"$SESSION_ID\",\"working_directory\":\"$WORKING_DIR\",\"timestamp\":\"$TIMESTAMP\"}'\""
    }
  }
}
```

## 未来改进

后续版本计划：
- 内置 HTTP 服务器监听 9527 端口
- WebSocket 实时通信
- 自定义提示音选择
- 通知历史记录