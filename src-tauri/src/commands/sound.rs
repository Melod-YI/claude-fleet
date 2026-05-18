use std::fs;
use std::path::PathBuf;
use tracing::{info, warn, debug, error};

// 生产模式：包含编译时生成的嵌入音频数据
#[cfg(not(debug_assertions))]
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_sounds.rs"));
}

/// 音频文件信息
#[derive(Debug, serde::Serialize)]
pub struct SoundInfo {
    /// 显示名称（文件名去掉扩展名）
    pub name: String,
    /// 文件名（包含扩展名）
    pub filename: String,
}

/// 获取可用音频列表
#[tauri::command]
pub fn get_available_sounds(app: tauri::AppHandle) -> Result<Vec<SoundInfo>, String> {
    info!("[get_available_sounds] 获取可用音频列表");

    let mut sounds = Vec::new();

    #[cfg(not(debug_assertions))]
    {
        // 生产模式：优先使用嵌入的音频
        for (filename, _) in embedded::EMBEDDED_SOUNDS {
            if let Some(stem) = PathBuf::from(filename).file_stem() {
                let name = stem.to_string_lossy().to_string();
                if name.to_lowercase() != "default" {
                    debug!("[get_available_sounds] 嵌入音频: {} ({})", name, filename);
                    sounds.push(SoundInfo { name, filename: filename.to_string() });
                }
            }
        }
    }

    // 同时检查外部 sounds 目录（用于用户自定义音频）
    let sounds_dir = get_sounds_dir(&app);
    if sounds_dir.exists() {
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
                            if name.to_lowercase() != "default" {
                                // 检查是否已存在（避免重复）
                                if !sounds.iter().any(|s: &SoundInfo| s.filename == filename) {
                                    debug!("[get_available_sounds] 外部音频: {} ({})", name, filename);
                                    sounds.push(SoundInfo { name, filename });
                                }
                            }
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

    // 生产模式：优先从嵌入数据获取
    #[cfg(not(debug_assertions))]
    {
        for (embedded_name, data) in embedded::EMBEDDED_SOUNDS {
            if embedded_name == &filename {
                let base64 = base64_encode(data);
                let data_uri = format!("data:{};base64,{}", mime_type, base64);
                info!("[get_sound_data] 从嵌入数据读取音频，{} 字节", data.len());
                return Ok(data_uri);
            }
        }
    }

    // 从外部文件获取
    let sounds_dir = get_sounds_dir(&app);
    let sound_path = sounds_dir.join(&filename);

    if !sound_path.exists() {
        warn!("[get_sound_data] 音频文件不存在: {:?}", sound_path);
        return Err(format!("音频文件不存在: {}", filename));
    }

    match fs::read(&sound_path) {
        Ok(data) => {
            let base64 = base64_encode(&data);
            let data_uri = format!("data:{};base64,{}", mime_type, base64);
            info!("[get_sound_data] 从外部文件读取音频，{} 字节", data.len());
            Ok(data_uri)
        }
        Err(e) => {
            error!("[get_sound_data] 读取文件失败: {}", e);
            Err(format!("读取音频文件失败: {}", e))
        }
    }
}

/// 获取音频目录路径（用于外部音频文件）
#[allow(unused_variables)]
fn get_sounds_dir(app: &tauri::AppHandle) -> PathBuf {
    #[cfg(debug_assertions)]
    {
        // 开发模式：使用 src-tauri/sounds
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        if cwd.ends_with("src-tauri") {
            cwd.join("sounds")
        } else {
            cwd.join("src-tauri").join("sounds")
        }
    }

    #[cfg(not(debug_assertions))]
    {
        // 生产模式：exe 所在目录的 sounds 子目录
        use tauri::Manager;
        app.path().resource_dir()
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