# Claude Fleet 钩子验证指南

## 验证步骤

### 1. 启动 Claude Fleet 应用

```bash
cd C:\workspace\claude-fleet-sp
npm run tauri dev
```

应用启动后会：
- 创建 `~/.claude-fleet/events/` 目录
- 开始监听该目录的文件变化
- 清理历史事件文件

### 2. 在测试项目目录创建钩子配置

创建一个测试项目目录，放置项目级 `settings.json`：

```bash
mkdir -p ~/test-hooks-project/.claude
cp docs/hooks/settings.example.json ~/test-hooks-project/.claude/settings.json
```

### 3. 在测试项目启动 Claude Code

```bash
cd ~/test-hooks-project
claude
```

### 4. 触发事件验证

**验证 SessionStart 事件：**
- Claude Code 启动时，应自动触发 `start` 事件
- Claude Fleet 应用会收到并刷新 session 列表

**验证 idle_prompt 事件：**
- 让 Claude 执行一个任务，完成后等待
- 或手动触发空闲状态
- Claude Fleet 应用应收到 `idle` 事件并发送通知

**验证 Stop 事件：**
- Claude 完成响应时触发
- Claude Fleet 收到后刷新 session 状态

**验证 SessionEnd 事件：**
- 退出 Claude Code 时触发
- Claude Fleet 收到后更新 session 状态

### 5. 检查事件目录

```powershell
# 查看是否有事件文件（正常情况下应该已被消费删除）
ls $env:USERPROFILE\.claude-fleet\events
```

如果目录为空，说明事件已被成功处理。

## 事件格式

每个事件文件是 JSON 格式：

```json
{
  "event": "idle",
  "session_id": "abc123-def456",
  "cwd": "C:\\workspace\\test-project"
}
```

## 手动测试（可选）

可以手动创建事件文件测试通知功能：

```powershell
$dir="$env:USERPROFILE\.claude-fleet\events"
echo '{"event":"idle","session_id":"test-manual","cwd":"C:\\test"}' > "$dir\manual_test.json"
```

Claude Fleet 应立即收到并触发通知。

## 环境变量说明

Claude Code 钩子提供的环境变量：
- `CLAUDE_CODE_SESSION_ID`: 当前 session 的唯一标识
- `CLAUDE_CODE_PROJECT_DIR`: 项目目录路径

## 注意事项

1. 钩子命令使用 PowerShell，确保系统已安装
2. 事件文件在处理后立即删除，防止堆积
3. 应用退出时会清空事件目录
4. 应用未启动时，钩子写入的文件会堆积，启动后自动清理