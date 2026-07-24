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
// 注意：需要窗口可见的场景（如启动终端）不要使用此函数，直接用 `Command::new()`，
// 但 spawn 统一走 `process::spawn()` 以获得句柄失效恢复能力。

use std::io;
use std::process::{Child, Command};
use tracing::warn;

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

/// Windows ERROR_INVALID_HANDLE 的错误码。
#[cfg(target_os = "windows")]
const ERROR_INVALID_HANDLE: i32 = 6;

/// Windows ERROR_NOT_SUPPORTED 的错误码。
#[cfg(target_os = "windows")]
const ERROR_NOT_SUPPORTED: i32 = 50;

/// 判断一个 io::Error 是否为"标准句柄被污染"类错误（os error 6 或 50）。
///
/// release 构建使用 `windows_subsystem = "windows"`，进程无控制台。spawn 控制台
/// 子进程（git、cmd 等）后，父进程标准句柄会被污染为"非 NULL 但已失效"状态，
/// 此后 `Command::spawn` 因 CreateProcess 继承到失效句柄而失败。依据句柄"伪有效"
/// 程度，CreateProcess 可能返回 ERROR_INVALID_HANDLE (6) 或 ERROR_NOT_SUPPORTED
/// (50)——二者同源，均需走 `reset_std_handles` 恢复路径。日志中 os error 50 表现为
/// "不支持该请求"，与 os error 6 一样会导致"启动终端/打开目录/打开 vscode"全部失败。
///
/// 抽取为独立函数以便单元测试锁定错误码与判定逻辑——它是 spawn 恢复分支的唯一
/// 触发条件，改动此处必须同步更新测试。
#[cfg(target_os = "windows")]
fn is_invalid_handle_error(e: &io::Error) -> bool {
    e.raw_os_error() == Some(ERROR_INVALID_HANDLE)
        || e.raw_os_error() == Some(ERROR_NOT_SUPPORTED)
}

#[cfg(not(target_os = "windows"))]
fn is_invalid_handle_error(_e: &io::Error) -> bool {
    false
}

/// 执行 spawn，遇到 Windows"句柄无效"(os error 6) 时自动恢复并重试一次。
///
/// 背景：release 构建使用 `windows_subsystem = "windows"`，进程无控制台。当 spawn
/// 控制台子进程（如 git、cmd）后，父进程的标准句柄可能被污染为"非 NULL 但已失效"，
/// 导致此后所有 `Command::spawn` 因 CreateProcess 传入失效句柄而返回 os error 6
/// （表现为"打开 claude code / vscode / 目录"全部失败，需重启才恢复）。
///
/// 恢复方式：`FreeConsole` + `SetStdHandle(NULL)` 清掉三个标准句柄，使其回到刚启动
/// 时的 NULL 状态——此时 Rust 不再设置 STARTF_USESTDHANDLES，CREATE_NEW_CONSOLE
/// 子进程会获得自己的新控制台，spawn 即可成功。等价于"不重启地恢复"。
///
/// 仅在出现该特定错误时触发，正常路径零开销。
pub fn spawn(cmd: &mut Command) -> io::Result<Child> {
    spawn_inner(cmd, |c| c.spawn(), reset_std_handles)
}

/// 可注入 spawn 与 recover 钩子的内部实现，便于单元测试重试编排而不触发真实
/// 操作系统副作用。
fn spawn_inner(
    cmd: &mut Command,
    mut do_spawn: impl FnMut(&mut Command) -> io::Result<Child>,
    mut on_recover: impl FnMut(),
) -> io::Result<Child> {
    match do_spawn(cmd) {
        Ok(child) => Ok(child),
        Err(e) if is_invalid_handle_error(&e) => {
            warn!(
                "[process::spawn] 检测到标准句柄失效 (os error 6 或 50)，清理标准句柄后重试: {}",
                e
            );
            on_recover();
            let retry = do_spawn(cmd);
            if let Err(ref re) = retry {
                warn!("[process::spawn] 重试仍失败: {}", re);
            }
            retry
        }
        Err(e) => Err(e),
    }
}

/// 清理当前进程的标准句柄：分离控制台 + 将三个标准句柄置 NULL。
///
/// best-effort：任一调用失败均忽略，下一步 spawn 会重新探测句柄状态。
#[cfg(target_os = "windows")]
fn reset_std_handles() {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Console::{
        FreeConsole, SetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };

    unsafe {
        let null = HANDLE(std::ptr::null_mut());
        let _ = FreeConsole();
        let _ = SetStdHandle(STD_INPUT_HANDLE, null);
        let _ = SetStdHandle(STD_OUTPUT_HANDLE, null);
        let _ = SetStdHandle(STD_ERROR_HANDLE, null);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // --- is_invalid_handle_error 判定逻辑 ---

    #[cfg(target_os = "windows")]
    #[test]
    fn detects_invalid_handle_os_error() {
        assert!(is_invalid_handle_error(&io::Error::from_raw_os_error(6)));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn detects_not_supported_os_error() {
        // os error 50 (ERROR_NOT_SUPPORTED) 与 os error 6 同源于标准句柄污染，
        // 必须同样触发恢复路径。
        assert!(is_invalid_handle_error(&io::Error::from_raw_os_error(50)));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn ignores_other_os_errors() {
        assert!(!is_invalid_handle_error(&io::Error::from_raw_os_error(5))); // access denied
        assert!(!is_invalid_handle_error(&io::Error::from_raw_os_error(2))); // file not found
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn ignores_non_os_error() {
        assert!(!is_invalid_handle_error(&io::Error::new(io::ErrorKind::Other, "boom")));
    }

    // --- spawn_inner 重试编排（注入 fake spawn，不触发真实 OS 副作用）---

    #[cfg(target_os = "windows")]
    #[test]
    fn retries_once_on_invalid_handle_error() {
        let recover_calls = AtomicUsize::new(0);
        let mut on_recover = || {
            let _ = recover_calls.fetch_add(1, Ordering::SeqCst);
        };

        // do_spawn 始终返回 os error 6
        let calls = AtomicUsize::new(0);
        let mut do_spawn = |_: &mut Command| -> io::Result<Child> {
            let _ = calls.fetch_add(1, Ordering::SeqCst);
            Err(io::Error::from_raw_os_error(6))
        };

        let mut cmd = Command::new("nonexistent_bin_for_test");
        let result = spawn_inner(&mut cmd, &mut do_spawn, &mut on_recover);

        assert!(result.is_err(), "重试仍失败时应返回 Err");
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "应在 os error 6 后重试一次（共调用 2 次）"
        );
        assert_eq!(
            recover_calls.load(Ordering::SeqCst),
            1,
            "恢复钩子应被调用一次"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn retries_once_on_not_supported_error() {
        // os error 50 应与 os error 6 走相同的清理+重试路径。
        let recover_calls = AtomicUsize::new(0);
        let mut on_recover = || {
            let _ = recover_calls.fetch_add(1, Ordering::SeqCst);
        };

        let calls = AtomicUsize::new(0);
        let mut do_spawn = |_: &mut Command| -> io::Result<Child> {
            let _ = calls.fetch_add(1, Ordering::SeqCst);
            Err(io::Error::from_raw_os_error(50))
        };

        let mut cmd = Command::new("nonexistent_bin_for_test");
        let result = spawn_inner(&mut cmd, &mut do_spawn, &mut on_recover);

        assert!(result.is_err(), "重试仍失败时应返回 Err");
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "应在 os error 50 后重试一次（共调用 2 次）"
        );
        assert_eq!(
            recover_calls.load(Ordering::SeqCst),
            1,
            "恢复钩子应被调用一次"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn does_not_retry_on_unrelated_os_error() {
        let mut do_spawn = |_: &mut Command| -> io::Result<Child> {
            Err(io::Error::from_raw_os_error(2)) // file not found
        };
        let mut on_recover = || panic!("非句柄失效错误不应触发恢复");

        let mut cmd = Command::new("nonexistent_bin_for_test");
        let result = spawn_inner(&mut cmd, &mut do_spawn, &mut on_recover);

        assert!(result.is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn does_not_retry_on_non_os_error() {
        let mut do_spawn = |_: &mut Command| -> io::Result<Child> {
            Err(io::Error::new(io::ErrorKind::Other, "boom"))
        };
        let mut on_recover = || panic!("非 OS 错误不应触发恢复");

        let mut cmd = Command::new("nonexistent_bin_for_test");
        let result = spawn_inner(&mut cmd, &mut do_spawn, &mut on_recover);

        assert!(result.is_err());
    }
}
