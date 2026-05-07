pub mod session_types;
pub mod session_utils;
pub mod claude_session;
pub mod claude_data;
pub mod sessions_watcher;
pub mod logger;
pub mod running_sessions;
pub mod window_manager;

// Re-export main types
pub use session_types::{SessionMeta, SessionMessage, RunningSessionMetadata};
pub use claude_session::{scan_sessions, get_session_messages, delete_session, get_projects_dir};