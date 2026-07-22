// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 子命令分发：`claude-fleet.exe maximize-window` —— 启动终端时由终端命令前置调用，
    // 在本进程内最大化当前/父终端窗口后退出，Tauri 主进程不进入。
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "maximize-window" {
        // 初始化 helper 专用日志（写独立 maximize.log），便于取证最大化轮询行为。
        // 必须在 maximize_current_process_window 之前：之后 helper 会 FreeConsole 使
        // stdout/stderr 失效，但文件写入不受影响。
        claude_fleet::init_helper_logging();
        if let Err(e) = claude_fleet::maximize_current_process_window() {
            eprintln!("[maximize-window] 失败: {}", e);
        }
        // best-effort：无论成败都 exit 0，绝不阻塞 claude 启动
        std::process::exit(0);
    }
    claude_fleet::run()
}
