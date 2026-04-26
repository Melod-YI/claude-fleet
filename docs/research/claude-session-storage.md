# Claude Code Session 存储结构

## 存储位置

Claude Code 的数据存储在用户目录下：

- **Windows**: `C:\Users\<username>\.claude\`
- **macOS**: `~/.claude/`
- **Linux**: `~/.claude/`

## 目录结构

```
~/.claude/
├── projects/                    # 项目数据
│   ├── C--workspace-project1/   # 项目目录（路径编码）
│   │   ├── <session-id>.jsonl   # session 数据文件
│   │   └── <session-id>/        # session 子目录
│   │       └── subagents/       # 子代理数据
│   └── C--workspace-project2/
│       └── ...
├── sessions/                    # 运行中 session 元数据
│   ├── 41200.json               # 按 PID 命名
│   └── 59172.json
├── history.jsonl                # 命令历史
├── settings.json                # 用户设置
└── ...
```

## 项目哈希计算

项目目录名是将工作目录路径进行编码转换：

- 原始路径: `C:\workspace\claude-fleet-sp`
- 编码后: `C--workspace-claude-fleet-sp`

转换规则：
1. 去除盘符后的冒号（`C:` → `C`）
2. 将路径分隔符替换为 `-`（`\` 或 `/` → `-`）
3. 连续的分隔符合并为单个 `-`

## Session 文件格式

### 运行中 Session 元数据 (`sessions/<pid>.json`)

```json
{
  "pid": 41200,
  "sessionId": "b02bb3fd-55d8-44c6-8aeb-b3bddeabaf09",
  "cwd": "C:\\workspace\\claude-fleet-sp",
  "startedAt": 1777124967120,
  "kind": "interactive",
  "entrypoint": "cli"
}
```

字段说明：
- `pid`: 进程 ID
- `sessionId`: session 唯一标识符 (UUID)
- `cwd`: 当前工作目录
- `startedAt`: 启动时间戳（毫秒）
- `kind`: session 类型（interactive）
- `entrypoint`: 入口点（cli）

### Session 数据文件 (`projects/<project>/<session-id>.jsonl`)

JSONL 格式，每行是一个 JSON 对象。消息类型包括：

#### 1. 权限模式设置

```json
{
  "type": "permission-mode",
  "permissionMode": "bypassPermissions",
  "sessionId": "uuid"
}
```

#### 2. 文件历史快照

```json
{
  "type": "file-history-snapshot",
  "messageId": "uuid",
  "snapshot": { ... },
  "isSnapshotUpdate": false
}
```

#### 3. 用户消息

```json
{
  "parentUuid": null,
  "isSidechain": false,
  "type": "user",
  "message": {
    "role": "user",
    "content": "..."
  },
  "uuid": "uuid",
  "timestamp": "2026-04-25T13:49:32.731Z",
  "userType": "external",
  "entrypoint": "cli",
  "cwd": "C:\\workspace\\claude-fleet-sp",
  "sessionId": "uuid",
  "version": "2.1.91",
  "gitBranch": "HEAD"
}
```

#### 4. 助手消息

```json
{
  "parentUuid": "uuid",
  "isSidechain": false,
  "message": {
    "model": "glm-5",
    "id": "msg_uuid",
    "role": "assistant",
    "type": "message",
    "content": [...],
    "usage": { "input_tokens": 66724, "output_tokens": 0 }
  },
  "type": "assistant",
  "uuid": "uuid",
  "timestamp": "2026-04-25T13:49:39.694Z",
  "userType": "external",
  "entrypoint": "cli",
  "cwd": "C:\\workspace\\claude-fleet-sp",
  "sessionId": "uuid",
  "version": "2.1.91",
  "gitBranch": "HEAD"
}
```

## 数据提取策略

### 1. 获取所有 Session 列表

遍历 `projects/` 目录下的所有项目目录，读取每个 `.jsonl` 文件：

1. 遍历 `projects/` 目录
2. 对每个项目目录，遍历 `sessions/` 子目录（如果存在）或直接读取 `.jsonl` 文件
3. 解析每个 JSONL 文件，提取 session 元数据

### 2. 检测运行中的 Session

读取 `sessions/` 目录下的 `<pid>.json` 文件：

1. 获取所有 `sessions/*.json` 文件
2. 检查 PID 是否仍在运行（通过进程检查）
3. 合并运行状态到 session 列表

### 3. 获取 Session 详情

从 JSONL 文件中提取：

- Session ID（文件名）
- 工作目录（从消息的 `cwd` 字段）
- 创建时间（第一条消息的 timestamp）
- 最后活动时间（最后一条消息的 timestamp）
- 消息数量（行数统计）

### 4. 获取对话内容

解析 JSONL 文件，提取 `type: user` 和 `type: assistant` 的消息。

## 字段映射

| 应用字段 | 来源 |
|---------|------|
| `id` | JSONL 文件名（不含扩展名） |
| `name` | 需要用户自定义，默认使用 ID 或工作目录名 |
| `workingDirectory` | 消息中的 `cwd` 字段 |
| `status` | 检测 PID 是否运行 |
| `createdAt` | 第一条消息的 `timestamp` |
| `lastActivityAt` | 最后一条消息的 `timestamp` |
| `conversationCount` | user + assistant 消息数 |
| `isFavorite` | 用户自定义，存储在应用本地 |
| `terminalWindowId` | 需要通过进程检测 |
| `processId` | `sessions/<pid>.json` 文件名 |

## 注意事项

1. **路径编码**: 项目目录名需要反向解码才能得到原始路径
2. **并发访问**: 读取 JSONL 时可能有写入，需要处理文件锁
3. **大文件处理**: JSONL 文件可能很大，建议流式读取
4. **编码问题**: Windows 路径使用反斜杠，需要正确处理