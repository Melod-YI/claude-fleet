# 日志系统增强设计

## 目标

为 Claude Fleet 项目添加详细的日志记录，用于开发调试，能够：
1. 体现各个处理环节、逻辑分支
2. 通过入口/出口日志时间戳评估耗时
3. 所有日志记录到文件（开发时不依赖控制台）

## 现状分析

| 模块 | 现状 | 问题 |
|------|------|------|
| `logger.rs` | 已有文件+控制台日志框架 | 基础完善，无需改动 |
| `claude_data.rs` | 使用 `println!` | 不写入文件，缺少分支日志 |
| `hooks.rs` | 有基础 tracing 日志 | 缺少监听细节、事件处理分支 |
| `running_sessions.rs` | 有基础 tracing 日志 | 缺少状态转换细节 |
| `terminal.rs` | 无日志 | 完全缺失 |
| `session.rs` | 无日志 | 完全缺失 |
| `window_manager.rs` | 无日志 | 完全缺失 |
| `lib.rs` | 有启动日志 | 可略微增强 |

## 日志级别约定

| 级别 | 使用场景 | 示例 |
|------|----------|------|
| `info` | 函数入口/出口、关键业务事件、状态变更 | "开始解析 session 文件"、"session 添加成功" |
| `debug` | 中间步骤、数据详情、分支判断、耗时测量 | "检查文件: xxx"、"耗时: 50ms" |
| `warn` | 非致命错误、降级处理、未找到预期数据 | "文件解析失败，跳过"、"session 未找到" |
| `error` | 操作失败、异常情况 | "读取目录失败"、"进程启动失败" |

## 耗时追踪规范

1. **函数边界日志**：每个关键函数必须有入口和出口日志
   - 入口：`info!("[函数名] 开始 - 关键参数")`
   - 出口：`info!("[函数名] 完成 - 结果摘要")`

2. **耗时操作标记**：对于明显耗时操作，记录耗时值
   ```rust
   let start = std::time::Instant::now();
   // ... 操作 ...
   debug!("[操作名] 完成，耗时: {}ms", start.elapsed().as_millis());
   ```

3. **中间步骤日志**：循环、分支判断等用 `debug` 级别

---

## 模块增强详情

### 1. claude_data.rs

**改动**：将 `println!` 替换为 tracing 日志，添加详细分支日志

**增强点**：

| 函数 | 入口日志 | 中间日志 | 出口日志 |
|------|----------|----------|----------|
| `get_all_sessions` | 记录开始 | 目录遍历数量、每个项目文件数、解析成功/失败数 | 总数、耗时 |
| `get_running_sessions` | 记录开始 | 每个文件解析结果 | 数量 |
| `get_running_sessions_list` | 记录开始 | 进程检测结果 | 数量、状态分布 |
| `parse_session_file` | 文件路径 | 每行解析类型、消息提取 | session ID、消息数 |
| `get_session_conversation` | session_id | 查找路径 | 找到/未找到 |
| `delete_session` | session_id | 查找路径 | 删除成功/未找到 |
| `decode_project_path` | 项目名 | - | 解码结果 |
| `is_process_running` | PID | 命令输出 | 结果 |
| `is_claude_process_running` | PID | 命令输出、进程名匹配 | 结果 |

**分支日志**：
- 目录不存在分支
- 文件扩展名过滤
- JSON 解析成功/失败
- 进程名匹配结果
- 状态判断（running/idle/waiting_input）

---

### 2. hooks.rs

**改动**：添加文件监听各阶段、事件处理详细分支日志

**增强点**：

| 函数 | 入口日志 | 中间日志 | 出口日志 |
|------|----------|----------|----------|
| `start_hook_receiver` | 开始 | 监听器创建、目录监听 | 成功/失败 |
| `stop_hook_receiver` | 开始 | - | 完成 |
| `cleanup_events_dir` | 目录路径 | 清理文件数 | 完成 |
| `process_file_event` | 事件类型 | 文件过滤、延迟等待、解析结果 | 处理完成 |
| `handle_hook_event_incremental` | 事件类型+session_id | 各事件类型分支处理 | 状态更新结果 |
| `emit_sessions_changed` | session 数量 | emit 结果 | 完成 |
| `trigger_hook_event` | 事件详情 | emit 结果 | 完成 |

**分支日志**：
- 事件类型判断（Modify/其他）
- 文件扩展名过滤
- HookEvent 类型分支（SessionStart/Notification/Stop/SessionEnd/未知）
- 添加 session 成功/失败
- emit 成功/失败

---

### 3. running_sessions.rs

**改动**：添加状态转换、进程检测、轮询细节日志

**增强点**：

| 函数 | 入口日志 | 中间日志 | 出口日志 |
|------|----------|----------|----------|
| `init_running_sessions` | 开始 | 事件文件数、解析成功数、session 分组数、每个 session 分析结果 | 运行中数量、耗时 |
| `add_running_session` | session_id | 元数据查找、PID 验证、进程检测、状态判断 | 成功/失败原因 |
| `update_session_status` | session_id、新旧状态 | 锁获取 | 完成/未找到警告 |
| `remove_running_session` | session_id | - | 完成 |
| `get_running_sessions` | - | 数量 | 返回 |
| `parse_hook_event` | 文件路径 | 内容读取、JSON 解析 | 成功/失败 |
| `read_session_metadata` | session_id | 目录遍历、每个文件检查 | 找到/未找到 |
| `start_polling` | 开始 | - | 线程启动确认 |
| `stop_polling` | 开始 | - | 完成 |

**轮询线程内部**：
- 每次轮询开始时间
- 检查的 PID 列表
- 发现退出的 PID
- emit 结果

**分支日志**：
- 目录不存在分支
- SessionEnd 判断（跳过）
- SessionStart 判断（添加）
- Notification 判断（更新状态）
- 进程检测结果

---

### 4. terminal.rs

**改动**：添加命令入口、参数、结果日志

**增强点**：

| 函数 | 入口日志 | 中间日志 | 出口日志 |
|------|----------|----------|----------|
| `jump_to_terminal` | 工作目录 | 窗口查找结果 | 成功/失败原因 |
| `jump_to_terminal_by_pid` | PID | 窗口查找结果 | 成功/失败原因 |
| `smart_jump_to_terminal` | 目录+PID | PID 匹配尝试、路径匹配尝试 | 成功/失败原因 |
| `resume_in_terminal` | 目录+session_id | 命令执行 | 成功/失败 |

**分支日志**：
- PID > 0 判断
- PID 匹配成功/失败
- 路径匹配成功/失败
- 非 Windows 平台分支

---

### 5. session.rs

**改动**：添加命令入口、参数、结果日志

**增强点**：

| 函数 | 入口日志 | 中间日志 | 出口日志 |
|------|----------|----------|----------|
| `list_sessions` | 开始 | - | 数量 |
| `init_running` | 开始 | - | 数量 |
| `list_running` | 开始 | - | 数量 |
| `start_polling_cmd` | 开始 | - | 完成 |
| `stop_polling_cmd` | 开始 | - | 完成 |
| `get_conversation` | session_id | - | 消息数 |
| `refresh_sessions` | 开始 | - | 数量 |
| `delete_session_cmd` | session_id | - | 完成 |
| `start_new_session` | 目录+名称 | 命令构建、平台判断、执行结果 | 成功/消息 |
| `start_hooks` | 开始 | - | 完成 |
| `stop_hooks` | 开始 | - | 完成 |
| `receive_hook_event` | 事件类型 | 处理结果 | 完成 |
| `send_notification` | 标题+内容 | - | 完成 |

**分支日志**：
- Windows/macOS/Linux 平台分支
- 命令执行成功/失败

---

### 6. window_manager.rs

**改动**：添加窗口查找、激活各阶段日志

**增强点**：

| 函数 | 入口日志 | 中间日志 | 出口日志 |
|------|----------|----------|----------|
| `find_window_by_pid` | PID | 遍历窗口、PID 匹配、标题获取、终端类型判断 | 找到/未找到 |
| `find_terminal_window` | 工作目录 | 遍历窗口、标题解析、路径匹配 | 找到/未找到 |
| `activate_window` | HWND | ShowWindow/SetForegroundWindow 调用 | 完成 |
| `start_terminal_with_resume` | 目录+session_id | 命令参数、平台分支 | 成功/失败 |

**回调函数内部**：
- 窗口遍历计数
- PID 匹配结果
- 标题内容
- 终端类型匹配
- 路径匹配结果

**分支日志**：
- 窗口可见性判断
- 标题长度判断
- 终端类型判断（Windows Terminal/Terminal/Command Prompt/PowerShell/claude）
- 路径匹配结果
- 备用窗口选择
- 平台分支（Windows/macOS/Linux）

---

### 7. lib.rs

**改动**：增强启动日志细节

**增强点**：
- 记录各初始化步骤耗时
- 记录初始化失败时的详细错误

---

## 文件改动清单

| 文件 | 改动类型 |
|------|----------|
| `claude_data.rs` | 替换 println + 增强日志 |
| `hooks.rs` | 增强日志 |
| `running_sessions.rs` | 增强日志 |
| `terminal.rs` | 新增日志 |
| `session.rs` | 新增日志 |
| `window_manager.rs` | 新增日志 |
| `lib.rs` | 增强日志 |

## 不改动

| 文件 | 原因 |
|------|------|
| `logger.rs` | 已完善，保持现状 |
| 前端 TypeScript | 保持 console 方式 |