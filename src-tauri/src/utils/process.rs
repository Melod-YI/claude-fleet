// src-tauri/src/utils/process.rs
// 后台进程创建工具
//
// Windows 上 `Command::new()` 默认会弹出控制台窗口。
// 使用 `process::command()` 替代，自动在 Windows 上添加 CREATE_NO_WINDOW 标志。
// 其他平台等同于 `Command::new()`。
//
// 用法:
//   use crate::utils::process;
//   let output = process::command("tasklist")
//       .args(["/FI", "PID eq 1234"])
//       .output()?;
//
// 注意：需要窗口可见的场景（如启动终端）不要使用此函数，直接用 `Command::new()`。

use std::process::Command;

/// 创建后台进程命令。Windows 上自动隐藏控制台窗口。
///
/// 等同于 `Command::new(program)` + Windows `CREATE_NO_WINDOW` 标志。
/// 适用于所有不需要显示窗口的后台命令（git、tasklist、wmic、code 等）。
pub fn command(program: &str) -> Command {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = Command::new(program);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new(program)
    }
}
