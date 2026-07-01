# GitHub Release 更新检测 设计文档

- 日期：2026-07-01
- 分支：update-detect
- 当前版本：0.8.2

## 1. 目标

应用运行期间，每隔数小时检测 GitHub 上是否存在比当前版本更新的正式 release。若存在，在 UI 上以常驻红点形式提示用户；用户进入设置后可查看新版本号、发布时间、release notes 摘要，并点击"前往下载"跳转到 GitHub Release 页面。

非目标：不实现自动下载、替换 exe、重启；不接入 tauri-plugin-updater；不做按版本忽略；不做总开关（始终检查）。

## 2. 约束与现状

- 仓库：`Melod-YI/claude-fleet`
- Release 形态：CI 仅产出便携版 exe `claude-fleet-<version>-x64.exe` 挂在 GitHub Release 上，无签名、无 update JSON。
- CSP：`connect-src` 限定 `self` + ipc，**前端无法直接访问 GitHub**，必须由 Rust 后端发起请求。
- Cargo.toml 目前无 HTTP client 依赖。
- 运行时当前版本：通过 `app.package_info().version` 获取（与 package.json / tauri.conf.json / Cargo.toml 三处一致），无需前端硬编码。
- 已有 `tauri-plugin-shell`（`shell:allow-open`），可直接用于打开浏览器。
- GitHub API 未认证限流 60 req/hour/IP，6h 间隔完全足够。

## 3. 架构与数据流

```
GitHub Releases API  ──HTTPS GET──►  Rust 后端 (utils/update_checker.rs)
                                        │ 解析 latest release (prerelease=false)
                                        │ semver 比较 vs app 当前版本
                                        ▼
                                  全局状态 (once_cell Mutex<Option<UpdateInfo>>)
                                        │
                          ┌─────────────┼──────────────┐
                          ▼             ▼              ▼
                  emit("update_available")  cmd: get_update_status  cmd: open_release_page
                                        │
                                        ▼
                            前端 Zustand updateStore ─► 设置图标红点 + 设置对话框详情
```

- **检测触发**：① `setup()` 启动后延迟 10s 跑一次；② 之后每 6h 循环一次；二者调用同一个内部函数 `run_check(app)`。
- **状态归属**：后端持有唯一真值 `Option<UpdateInfo>`，前端只读取，避免前后端轮询不一致。

## 4. 后端组件

### 4.1 `src-tauri/src/utils/update_checker.rs`（新增）

- `UpdateInfo` 结构体，`#[serde(rename_all = "camelCase")]`：
  - `latest_version: String`
  - `release_url: String`
  - `release_notes: Option<String>`
  - `published_at: String`
- 常量：
  - `RELEASES_API = "https://api.github.com/repos/Melod-YI/claude-fleet/releases/latest"`
  - `USER_AGENT = "claude-fleet/<version>"`
- `pub async fn check_for_updates(app: &AppHandle) -> Result<Option<UpdateInfo>>`：
  - 用 `tauri::async_runtime::spawn_blocking` 包阻塞 `ureq` 请求。
  - 解析 JSON；若 `prerelease == true`，视为无更新，返回 `None`。
  - `is_newer_version(current, latest)` 比较；更大才返回 `Some(UpdateInfo)`。
- 全局状态：`static STATE: Lazy<Mutex<Option<UpdateInfo>>>`。检测完成后写入，并通过 `app.emit("update_available", &info)` 推送。
- `pub fn start_update_loop(app: AppHandle)`：在 `setup()` 里调用，`tauri::async_runtime::spawn` 一个循环：先 sleep 10s 检查一次，之后每 6h 检查一次；每次失败只 `warn!` 日志，不中断循环。

### 4.2 纯函数（可单测）

- `pub fn is_newer_version(current: &str, latest: &str) -> bool`：去掉 `v` 前缀，按 `major.minor.patch` 数值比较；解析失败时按字符串比较兜底。
- `pub fn parse_latest_release(json: &str) -> Option<RawRelease>`：提取 `tag_name`、`html_url`、`body`、`prerelease`、`published_at`。

### 4.3 Tauri 命令（`commands/update.rs` 新增）

- `get_update_status() -> Option<UpdateInfo>`：读全局状态。
- `open_release_page(url: String) -> Result<()>`：调用 `tauri-plugin-shell` 的 open 打开浏览器。

### 4.4 依赖

Cargo.toml 新增：
```toml
ureq = { version = "2", features = ["json", "tls"] }
```

### 4.5 接线

- `utils/mod.rs` 加 `pub mod update_checker;`
- `commands/mod.rs` 加 `pub mod update;`，`lib.rs` 的 `invoke_handler!` 注册两个命令。
- `lib.rs` 的 `setup()` 中调用 `update_checker::start_update_loop(app_handle)`。

## 5. 前端组件

- **类型**：`src/types/` 加 `UpdateInfo`（`latestVersion`, `releaseUrl`, `releaseNotes?`, `publishedAt`），并在 `index.ts` 导出。
- **Store**：`src/stores/updateStore.ts`（Zustand）：`updateInfo: UpdateInfo | null`、`setUpdateInfo`。
- **Hook**：`src/hooks/useUpdateChecker.ts`：挂载时 `invoke('get_update_status')` 初始化，监听 `update_available` 事件更新 store。在 `App.tsx` 初始化流程里调用一次。参考 `useNotification` / `useRunningSessions` 的事件监听写法。
- **UI**：
  - 设置图标按钮加红点（绝对定位小圆点，`cn()` 控制 visible），条件 `updateInfo != null`。
  - `SettingsDialog` 新增"更新"区块：显示"发现新版本 vX.Y.Z"、发布时间、可选 release notes 摘要、"前往下载"按钮（点击 `invoke('open_release_page', { url })`）。无新版本时不显示。

## 6. 错误处理

- 网络失败 / API 限流（403/429）/ 解析失败：`warn!` 日志，状态保持上次的值（不清空，避免红点闪烁）。启动首次失败则状态为 `None`，红点不亮。
- semver 解析失败（标签非标准格式）：`warn!`，按字符串比较兜底。
- 全程不影响主业务，任何错误都不阻塞应用。

## 7. 测试

- `is_newer_version`：覆盖 `0.8.2 < 0.9.0`、`0.8.2 = 0.8.2`（返回 false）、`0.8.2 > 0.8.1`、带 `v` 前缀、不同位数（`0.8.2` vs `0.8.10`）。
- `parse_latest_release`：用 fixture JSON 字符串测字段提取 + prerelease 过滤。
- 网络层不写测试（外部依赖）。

## 8. 日志

- `[update_checker] 开始检查更新` / `[update_checker] 发现新版本: <v>` / `[update_checker] 当前已是最新: <v>`
- `[update_checker] 检查失败: <error>`（warn）
- 启动循环入口：`[update_checker] 启动更新检测循环，间隔 6h`
