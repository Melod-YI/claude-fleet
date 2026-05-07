use tracing_subscriber::fmt;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::prelude::*;
use tracing::info;
use std::path::PathBuf;

/// 获取日志目录路径
pub fn get_log_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户目录")
        .join(".claude-fleet")
        .join("logs")
}

/// 初始化日志系统（分离 stdout 和文件输出，避免颜色码污染文件）
pub fn init_logging() {
    let log_dir = get_log_dir();

    // 确保日志目录存在
    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir).ok();
    }

    // 创建文件日志 appender（每天滚动）
    let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "claude-fleet.log");

    // 使用 Layer 方式分离 stdout 和文件输出
    // stdout: 启用颜色，无文件路径信息（简洁）
    // file: 禁用颜色，包含完整信息（文件路径、行号）

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)           // stdout 启用颜色
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_line_number(false)
        .with_file(false);

    // 文件 Layer：无颜色，完整信息
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);
    // guard 必须保持存活，否则 non-blocking writer 会停止工作
    std::mem::forget(guard);

    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false)          // 文件禁用颜色
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_line_number(true)
        .with_file(true);

    // 组合两个 Layer
    let subscriber = tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("无法设置全局日志订阅者");

    info!("日志系统初始化完成，日志目录: {}", log_dir.display());
}