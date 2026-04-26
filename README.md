# Claude Fleet

一个管理多个 Claude Code session 的桌面应用工具。

## 功能

- **状态监控**: 实时显示运行中 session 状态，等待输入时主动提示
- **快速切换**: 一键跳转到对应的 Windows Terminal 窗口
- **历史管理**: 收藏、搜索、恢复历史 session

## 安装

下载最新的安装包: [Releases](https://github.com/xxx/claude-fleet/releases)

## 使用

### 查看运行中的 session

打开应用，默认显示"运行中" Tab，可以看到所有正在运行的 Claude Code session。

当 session 进入"等待输入"状态时，应用会发送声音和桌面通知提醒。

### 管理历史 session

切换到"Session 管理" Tab:
- 左侧列表显示收藏和历史 session
- 支持搜索（名称、路径、对话内容）
- 支持收藏过滤和时间筛选
- 支持目录视图（按路径树状展开）

### 新建 session

点击 "+" 按钮，选择工作目录，启动新的 Claude Code。

### 恢复 session

点击"恢复"按钮，自动打开新终端窗口并执行恢复命令。

### 跳转到终端

点击"跳转到终端"按钮，自动激活对应的 Windows Terminal 窗口。

## 配置 Claude Code 钩子

为了实时接收 session 状态变化，需要配置 Claude Code 钩子。详见 [docs/hooks-setup.md](docs/hooks-setup.md)。

## 开发

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建
npm run tauri build
```

## 技术栈

- Tauri 2.0
- React 18
- TypeScript 5
- Tailwind CSS 4
- shadcn/ui
- Zustand