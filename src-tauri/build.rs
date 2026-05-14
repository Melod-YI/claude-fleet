// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "console")]

use std::fs;
use std::path::Path;

fn main() {
    // 生成嵌入音频的 Rust 代码
    generate_embedded_sounds();

    tauri_build::build()
}

/// 扫描 sounds 目录，生成包含 include_bytes! 的 Rust 代码
fn generate_embedded_sounds() {
    let sounds_dir = Path::new("sounds");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest_path = Path::new(&out_dir).join("embedded_sounds.rs");

    let mut code = String::new();
    code.push_str("/// 嵌入的音频文件列表\n");
    code.push_str("/// 路径相对于 OUT_DIR，需要回退到 src-tauri/sounds\n");
    code.push_str("pub static EMBEDDED_SOUNDS: &[(&str, &[u8])] = &[\n");

    if sounds_dir.exists() {
        for entry in fs::read_dir(sounds_dir).expect("Failed to read sounds directory") {
            let entry = entry.expect("Failed to read entry");
            let path = entry.path();

            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if ["mp3", "wav", "ogg"].contains(&ext.as_str()) {
                    let filename = path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    // OUT_DIR 在 target/release/build/<hash>/out/
                    // 需要回退到 src-tauri/sounds: ../../../sounds/<filename>
                    let include_path = format!("../../../sounds/{}", filename);

                    code.push_str(&format!("    (\"{}\", include_bytes!(\"{}\")),\n",
                        filename, include_path));

                    println!("cargo:warning=Embedding sound: {}", filename);
                }
            }
        }
    }

    code.push_str("];\n");

    // 告知 Cargo 当 sounds 目录变化时重新运行
    println!("cargo:rerun-if-changed=sounds");

    fs::write(&dest_path, code).expect("Failed to write embedded_sounds.rs");
    println!("cargo:warning=Generated embedded_sounds.rs at: {:?}", dest_path);
}