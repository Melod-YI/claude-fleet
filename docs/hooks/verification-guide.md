# Claude Fleet 钩子配置

## 自动安装

应用启动时会自动在 `~/.claude-fleet/` 目录创建 `hook_writer.py` 脚本。

## 配置 Claude Code 钩子

将以下内容添加到 `~/.claude/settings.json`：

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python ~/.claude-fleet/hook_writer.py"
          }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "idle_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "python ~/.claude-fleet/hook_writer.py"
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python ~/.claude-fleet/hook_writer.py"
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python ~/.claude-fleet/hook_writer.py"
          }
        ]
      }
    ]
  }
}
```

## 钩子事件

Claude Code 通过 stdin 传递 JSON：

```json
{
  "session_id": "...",
  "hook_event_name": "SessionStart",
  "cwd": "...",
  "transcript_path": "...",
  "source": "startup",
  "model": "..."
}
```

事件类型：
- `SessionStart` - 启动
- `Notification` - 等待输入
- `Stop` - 响应完成
- `SessionEnd` - 结束