use std::fs;
use std::path::PathBuf;
use tracing::{info, warn, debug, error};

/// 音频文件信息
#[derive(Debug, serde::Serialize)]
pub struct SoundInfo {
    /// 显示名称（文件名去掉扩展名）
    pub name: String,
    /// 文件名（包含扩展名）
    pub filename: String,
}

/// 获取可用音频列表（不包括内置默认）
#[tauri::command]
pub fn get_available_sounds(app: tauri::AppHandle) -> Result<Vec<SoundInfo>, String> {
    info!("[get_available_sounds] 获取可用音频列表");

    let sounds_dir = get_sounds_dir(&app);

    if !sounds_dir.exists() {
        warn!("[get_available_sounds] 音频目录不存在: {:?}", sounds_dir);
        return Ok(vec![]);
    }

    let mut sounds = Vec::new();

    if let Ok(entries) = fs::read_dir(&sounds_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if ["wav", "mp3", "ogg"].contains(&ext.as_str()) {
                    if let Some(stem) = path.file_stem() {
                        let name = stem.to_string_lossy().to_string();
                        let filename = path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        // 排除 default 文件（使用内置默认）
                        if name.to_lowercase() != "default" {
                            debug!("[get_available_sounds] 发现音频: {} ({})", name, filename);
                            sounds.push(SoundInfo { name, filename });
                        }
                    }
                }
            }
        }
    }

    // 按字母顺序排序
    sounds.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    info!("[get_available_sounds] 找到 {} 个音频文件", sounds.len());
    Ok(sounds)
}

/// 获取音频数据（返回 base64 data URI）
#[tauri::command]
pub fn get_sound_data(app: tauri::AppHandle, filename: String) -> Result<String, String> {
    info!("[get_sound_data] 读取音频文件: {}", filename);

    // 安全检查：防止路径遍历攻击
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        error!("[get_sound_data] 无效文件名（路径遍历攻击尝试）: {}", filename);
        return Err("无效文件名".to_string());
    }

    let sounds_dir = get_sounds_dir(&app);
    let sound_path = sounds_dir.join(&filename);

    if !sound_path.exists() {
        warn!("[get_sound_data] 音频文件不存在: {:?}", sound_path);
        return Err(format!("音频文件不存在: {}", filename));
    }

    // 根据扩展名确定 MIME 类型
    let mime_type = match PathBuf::from(&filename).extension() {
        Some(ext) => match ext.to_string_lossy().to_lowercase().as_str() {
            "wav" => "audio/wav",
            "mp3" => "audio/mpeg",
            "ogg" => "audio/ogg",
            _ => "audio/wav",
        },
        None => "audio/wav",
    };

    match fs::read(&sound_path) {
        Ok(data) => {
            let base64 = base64_encode(&data);
            let data_uri = format!("data:{};base64,{}", mime_type, base64);
            info!("[get_sound_data] 成功读取音频文件，{} 字节", data.len());
            Ok(data_uri)
        }
        Err(e) => {
            error!("[get_sound_data] 读取文件失败: {}", e);
            Err(format!("读取音频文件失败: {}", e))
        }
    }
}

/// 获取音频目录路径
/// 开发模式：src-tauri/sounds
/// 生产模式：resource_dir/sounds
fn get_sounds_dir(_app: &tauri::AppHandle) -> PathBuf {
    #[cfg(debug_assertions)]
    {
        // 开发模式：使用 src-tauri/sounds
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // 检查当前目录是否已经是 src-tauri
        if cwd.ends_with("src-tauri") {
            cwd.join("sounds")
        } else {
            cwd.join("src-tauri").join("sounds")
        }
    }

    #[cfg(not(debug_assertions))]
    {
        // 生产模式：使用 resource 目录
        use tauri::Manager;
        _app.path().resource_dir()
            .map(|p| p.join("sounds"))
            .unwrap_or_else(|_| PathBuf::from("sounds"))
    }
}

/// Base64 编码（无外部依赖）
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
            if chunk.len() > 2 {
                result.push(ALPHABET[b2 & 0x3f] as char);
            } else {
                result.push('=');
            }
        } else {
            result.push('=');
            result.push('=');
        }
    }

    result
}