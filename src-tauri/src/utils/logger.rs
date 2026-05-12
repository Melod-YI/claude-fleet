use tracing_subscriber::fmt;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing::{info, debug};
use std::path::PathBuf;
use std::fs;

/// 获取日志目录路径
pub fn get_log_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户目录")
        .join(".claude-fleet")
        .join("logs")
}

/// 清理过期日志文件（保留最近 7 天）
fn cleanup_old_logs(log_dir: &PathBuf) {
    let keep_days = 7;
    let now = chrono::Local::now();
    let cutoff_date = now - chrono::Duration::days(keep_days);

    if let Ok(entries) = fs::read_dir(log_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "log").unwrap_or(false) {
                // 尝试从文件名解析日期：claude-fleet-YYYY-MM-DD.log
                let filename = path.file_name().unwrap().to_string_lossy();
                if let Some(date_str) = filename.strip_prefix("claude-fleet-").and_then(|s| s.strip_suffix(".log")) {
                    if let Ok(file_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                        if file_date < cutoff_date.date_naive() {
                            debug!("删除过期日志: {}", path.display());
                            fs::remove_file(path).ok();
                        }
                    }
                }
            }
        }
    }
}

/// 初始化日志系统
///
/// 日志级别可通过环境变量 RUST_LOG 控制：
/// - 默认：INFO 级别（不输出 DEBUG）
/// - 开启 DEBUG：设置 RUST_LOG=debug 或 RUST_LOG=claude_fleet=debug
pub fn init_logging() {
    let log_dir = get_log_dir();

    // 确保日志目录存在
    if !log_dir.exists() {
        fs::create_dir_all(&log_dir).ok();
    }

    // 清理过期日志
    cleanup_old_logs(&log_dir);

    // 创建文件日志 appender（每天滚动）
    // Rotation::DAILY 会生成 claude-fleet.log.YYYY-MM-DD 格式
    // 我们需要自定义格式，使用 builder 方式
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("claude-fleet")
        .filename_suffix("log")
        .max_log_files(7)  // 最多保留 7 个日志文件
        .build(&log_dir)
        .expect("无法创建日志文件");

    // 日志级别过滤：默认 INFO，可通过 RUST_LOG 环境变量覆盖
    // 示例：RUST_LOG=debug 开启全部 debug 日志
    //       RUST_LOG=claude_fleet=debug 只开启本应用的 debug 日志
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // stdout Layer：简洁格式，启用颜色
    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_line_number(false)
        .with_file(false);

    // 文件 Layer：完整信息，禁用颜色
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);
    std::mem::forget(guard);

    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_line_number(true)
        .with_file(true);

    // 组合所有 Layer
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("无法设置全局日志订阅者");

    info!("日志系统初始化完成，日志目录: {}", log_dir.display());
    info!("日志级别: INFO (可通过 RUST_LOG=debug 开启 DEBUG 日志)");
}