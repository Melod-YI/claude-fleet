// src-tauri/src/db/mod.rs

pub mod schema;
pub mod sessions_meta;
pub mod favorites;
pub mod favorite_paths;
pub mod settings;
pub mod migration;

// 导出常用类型和函数
pub use schema::{get_db_path, init_database, get_connection};
pub use sessions_meta::{set_session_name, get_session_name, delete_session_name};
pub use favorites::{add_favorite, remove_favorite, is_favorite, get_all_favorites};
pub use favorite_paths::{record_path_usage, remove_favorite_path, get_sorted_favorite_paths};
pub use settings::{get_setting, set_setting, get_all_settings};
pub use migration::migrate_from_localstorage;