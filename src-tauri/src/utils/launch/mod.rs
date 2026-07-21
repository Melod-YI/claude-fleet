use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchSettings {
    pub terminal_id: String,
    pub claude_executable: String,
    pub claude_args: Vec<String>,
    pub wrapper: Option<CommandWrapper>,
    /// 启动终端后是否将窗口最大化（新建/恢复均生效）。前端旧数据无此字段时默认 false。
    #[serde(default)]
    pub maximize_window: bool,
}

impl LaunchSettings {
    pub fn legacy_default(terminal_id: &str) -> Self {
        Self {
            terminal_id: terminal_id.to_string(),
            claude_executable: "claude".to_string(),
            claude_args: vec![
                "--permission-mode".to_string(),
                "bypassPermissions".to_string(),
            ],
            wrapper: None,
            maximize_window: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandWrapper {
    pub enabled: bool,
    pub executable: String,
    pub args_before_agent: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LaunchMode {
    New { name: Option<String> },
    Resume { session_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequest {
    pub working_directory: String,
    pub mode: LaunchMode,
    pub settings: LaunchSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnPlan {
    pub command: String,
    pub args: Vec<String>,
    /// 需原样传入（不转义）的参数。cmd.exe 的 /K 命令串含原始 `"` 字符，普通 `.args()`
    /// 会把 `"` 转义为 `\"`（cmd 不认），故 helper 前缀的 cmd 命令走 raw_arg。
    pub raw_args: Vec<String>,
    pub current_dir: Option<String>,
    pub creation_flags: Option<u32>,
}

pub fn build_agent_argv(request: &LaunchRequest) -> Vec<String> {
    let executable = request.settings.claude_executable.trim();
    let mut argv = vec![if executable.is_empty() {
        "claude".to_string()
    } else {
        executable.to_string()
    }];

    match &request.mode {
        LaunchMode::New { .. } => {}
        LaunchMode::Resume { session_id } => {
            argv.push("--resume".to_string());
            argv.push(session_id.clone());
        }
    }

    argv.extend(request.settings.claude_args.clone());

    if let LaunchMode::New { name: Some(name) } = &request.mode {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            argv.push("--name".to_string());
            argv.push(trimmed.to_string());
        }
    }

    argv
}

pub fn build_process_argv(request: &LaunchRequest) -> Vec<String> {
    let agent_argv = build_agent_argv(request);

    // ccglass 在 wezterm 下存在兼容性问题，强制跳过
    if request.settings.terminal_id == "wezterm" {
        if let Some(wrapper) = &request.settings.wrapper {
            if wrapper.enabled {
                tracing::warn!(
                    "[build_process_argv] wezterm 不支持 ccglass wrapper，已忽略"
                );
            }
        }
        return agent_argv;
    }

    if let Some(wrapper) = &request.settings.wrapper {
        if wrapper.enabled && !wrapper.executable.trim().is_empty() {
            let mut argv = vec![wrapper.executable.clone()];
            argv.extend(wrapper.args_before_agent.clone());
            argv.extend(agent_argv);
            return argv;
        }
    }

    agent_argv
}

pub fn build_spawn_plan(request: &LaunchRequest) -> Result<SpawnPlan, String> {
    let process_argv = build_process_argv(request);
    if process_argv.is_empty() {
        return Err("启动命令为空".to_string());
    }
    let claude_cmdline = command_line(&process_argv);
    let maximize = request.settings.maximize_window;
    let terminal = request.settings.terminal_id.as_str();

    // wezterm 不支持最大化：即使 maximize=true 也不前缀 helper，warn 跳过
    if maximize && terminal == "wezterm" {
        tracing::warn!("[build_spawn_plan] wezterm 不支持最大化，已跳过 helper 前缀");
    }
    // helper exe 路径（maximize && 非 wezterm 时需要）。current_exe 失败则退化为不前缀 helper。
    let helper_exe: Option<String> = if maximize && terminal != "wezterm" {
        match std::env::current_exe() {
            Ok(p) => Some(p.display().to_string()),
            Err(e) => {
                tracing::warn!("[build_spawn_plan] current_exe 失败，退化为不前缀 helper: {}", e);
                None
            }
        }
    } else {
        None
    };

    match terminal {
        "wezterm" => Ok(SpawnPlan {
            command: "wezterm".to_string(),
            args: [
                vec![
                    "start".to_string(),
                    "--cwd".to_string(),
                    request.working_directory.clone(),
                    "-e".to_string(),
                ],
                process_argv,
            ]
            .concat(),
            raw_args: vec![],
            current_dir: None,
            creation_flags: Some(DETACHED_PROCESS),
        }),
        "cmd" => {
            // cmd.exe 自有命令行解析器，不识别 `\"` 转义。helper exe 路径含空格时
            // （如 `C:\Users\...\Claude Fleet\claude-fleet.exe`），用 raw_arg 把带引号的
            // 命令串原样传给 cmd，避免 Rust .args() 把 " 转义为 \" 破坏 cmd 解析。
            match &helper_exe {
                Some(exe) => {
                    let k_raw = format!("\"{}\" maximize-window && {}", exe, claude_cmdline);
                    Ok(SpawnPlan {
                        command: "cmd.exe".to_string(),
                        args: vec!["/K".to_string()],
                        raw_args: vec![k_raw],
                        current_dir: Some(request.working_directory.clone()),
                        creation_flags: Some(CREATE_NEW_CONSOLE),
                    })
                }
                None => Ok(SpawnPlan {
                    command: "cmd.exe".to_string(),
                    args: vec!["/K".to_string(), claude_cmdline],
                    raw_args: vec![],
                    current_dir: Some(request.working_directory.clone()),
                    creation_flags: Some(CREATE_NEW_CONSOLE),
                }),
            }
        }
        "powershell" | "powershell7" => {
            let exe_name = if terminal == "powershell" {
                "powershell.exe"
            } else {
                "pwsh.exe"
            };
            // powershell 用标准 argv 解析（识别 \"），普通 .args() 即可；
            // 用 & 调用操作符包裹带双引号的 exe 路径。
            let cmd_arg = match &helper_exe {
                Some(exe) => format!("& \"{}\" maximize-window; {}", exe, claude_cmdline),
                None => claude_cmdline,
            };
            Ok(SpawnPlan {
                command: exe_name.to_string(),
                args: vec!["-Command".to_string(), cmd_arg],
                raw_args: vec![],
                current_dir: Some(request.working_directory.clone()),
                creation_flags: Some(CREATE_NEW_CONSOLE),
            })
        }
        other => Err(format!("不支持的终端类型: {}", other)),
    }
}

pub fn launch_session(request: &LaunchRequest) -> Result<(), String> {
    let plan = build_spawn_plan(request)?;
    // 最大化由终端命令前置的 helper 子命令完成（build_spawn_plan 在 maximize_window=true
    // 时已构造 "<helper> maximize-window && claude..."），此处不再事后补最大化。
    // 丢 child 不影响终端进程存活（drop 仅关闭句柄，不杀进程）。
    let _child = spawn_plan(&plan)?;
    Ok(())
}

pub fn spawn_plan(plan: &SpawnPlan) -> Result<std::process::Child, String> {
    let mut command = Command::new(&plan.command);
    command.args(&plan.args);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // raw_args：原样追加（不转义）。cmd.exe 的 /K 命令串含原始 " 字符，
        // 普通 .args() 会把 " 转义为 \"（cmd 不认），故 helper 前缀的 cmd 命令走 raw_arg。
        for raw in &plan.raw_args {
            command.raw_arg(raw);
        }
        if let Some(flags) = plan.creation_flags {
            command.creation_flags(flags);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = &plan.raw_args;
        let _ = &plan.creation_flags;
    }

    if let Some(current_dir) = &plan.current_dir {
        command.current_dir(current_dir);
    }

    crate::utils::process::spawn(&mut command)
        .map_err(|e| format!("启动终端失败: {}", e))
}

fn command_line(argv: &[String]) -> String {
    argv.iter()
        .map(|arg| quote_command_arg(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote_command_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }

    if !arg.chars().any(|c| c.is_whitespace() || matches!(c, '"' | '\'')) {
        return arg.to_string();
    }

    format!("\"{}\"", arg.replace('"', "\\\""))
}

const DETACHED_PROCESS: u32 = 0x00000008;
const CREATE_NEW_CONSOLE: u32 = 0x00000010;

#[cfg(test)]
mod tests {
    use super::*;

    fn default_settings(terminal_id: &str) -> LaunchSettings {
        LaunchSettings {
            terminal_id: terminal_id.to_string(),
            claude_executable: "claude".to_string(),
            claude_args: vec![
                "--permission-mode".to_string(),
                "bypassPermissions".to_string(),
            ],
            wrapper: None,
            maximize_window: false,
        }
    }

    #[test]
    fn builds_resume_agent_argv_from_settings() {
        let request = LaunchRequest {
            working_directory: "C:\\workspace\\project".to_string(),
            mode: LaunchMode::Resume {
                session_id: "session-123".to_string(),
            },
            settings: default_settings("cmd"),
        };

        let argv = build_agent_argv(&request);

        assert_eq!(
            argv,
            vec![
                "claude",
                "--resume",
                "session-123",
                "--permission-mode",
                "bypassPermissions",
            ]
        );
    }

    #[test]
    fn builds_new_agent_argv_with_name() {
        let request = LaunchRequest {
            working_directory: "C:\\workspace\\project".to_string(),
            mode: LaunchMode::New {
                name: Some("demo".to_string()),
            },
            settings: default_settings("powershell"),
        };

        let argv = build_agent_argv(&request);

        assert_eq!(
            argv,
            vec!["claude", "--permission-mode", "bypassPermissions", "--name", "demo"]
        );
    }

    #[test]
    fn wrapper_becomes_process_entrypoint() {
        let mut settings = default_settings("cmd");
        settings.wrapper = Some(CommandWrapper {
            enabled: true,
            executable: "ccglass".to_string(),
            args_before_agent: vec![],
        });
        let request = LaunchRequest {
            working_directory: "C:\\workspace\\project".to_string(),
            mode: LaunchMode::Resume {
                session_id: "session-123".to_string(),
            },
            settings,
        };

        let argv = build_process_argv(&request);

        assert_eq!(
            argv,
            vec![
                "ccglass",
                "claude",
                "--resume",
                "session-123",
                "--permission-mode",
                "bypassPermissions",
            ]
        );
    }

    #[test]
    fn wezterm_skips_wrapper_even_when_enabled() {
        let mut settings = default_settings("wezterm");
        settings.wrapper = Some(CommandWrapper {
            enabled: true,
            executable: "ccglass".to_string(),
            args_before_agent: vec!["--some-flag".to_string()],
        });
        let request = LaunchRequest {
            working_directory: "C:\\workspace\\project".to_string(),
            mode: LaunchMode::New {
                name: Some("demo".to_string()),
            },
            settings,
        };

        let argv = build_process_argv(&request);

        // wrapper 被跳过，直接返回 agent argv
        assert_eq!(
            argv,
            vec![
                "claude",
                "--permission-mode",
                "bypassPermissions",
                "--name",
                "demo",
            ]
        );
    }

    #[test]
    fn terminal_registry_builds_wezterm_spawn_plan() {
        let request = LaunchRequest {
            working_directory: "C:\\workspace\\project".to_string(),
            mode: LaunchMode::Resume {
                session_id: "session-123".to_string(),
            },
            settings: default_settings("wezterm"),
        };

        let plan = build_spawn_plan(&request).unwrap();

        assert_eq!(plan.command, "wezterm");
        assert_eq!(
            plan.args,
            vec![
                "start",
                "--cwd",
                "C:\\workspace\\project",
                "-e",
                "claude",
                "--resume",
                "session-123",
                "--permission-mode",
                "bypassPermissions",
            ]
        );
        assert_eq!(plan.current_dir, None);
        assert_eq!(plan.creation_flags, Some(0x00000008));
    }

    #[test]
    fn terminal_registry_builds_cmd_spawn_plan() {
        let request = LaunchRequest {
            working_directory: "C:\\workspace\\project".to_string(),
            mode: LaunchMode::Resume {
                session_id: "session-123".to_string(),
            },
            settings: default_settings("cmd"),
        };

        let plan = build_spawn_plan(&request).unwrap();

        assert_eq!(plan.command, "cmd.exe");
        assert_eq!(plan.args[0], "/K");
        assert_eq!(
            plan.args[1],
            "claude --resume session-123 --permission-mode bypassPermissions"
        );
        assert_eq!(plan.current_dir, Some("C:\\workspace\\project".to_string()));
        assert_eq!(plan.creation_flags, Some(0x00000010));
    }

    #[test]
    fn terminal_registry_builds_powershell_spawn_plan() {
        let request = LaunchRequest {
            working_directory: "C:\\workspace\\project".to_string(),
            mode: LaunchMode::Resume {
                session_id: "session-123".to_string(),
            },
            settings: default_settings("powershell"),
        };

        let plan = build_spawn_plan(&request).unwrap();

        assert_eq!(plan.command, "powershell.exe");
        assert_eq!(plan.args[0], "-Command");
        assert_eq!(
            plan.args[1],
            "claude --resume session-123 --permission-mode bypassPermissions"
        );
        assert_eq!(plan.current_dir, Some("C:\\workspace\\project".to_string()));
        assert_eq!(plan.creation_flags, Some(0x00000010));
    }

    #[test]
    fn terminal_registry_builds_powershell7_spawn_plan() {
        let request = LaunchRequest {
            working_directory: "C:\\workspace\\project".to_string(),
            mode: LaunchMode::Resume {
                session_id: "session-123".to_string(),
            },
            settings: default_settings("powershell7"),
        };

        let plan = build_spawn_plan(&request).unwrap();

        assert_eq!(plan.command, "pwsh.exe");
        assert_eq!(plan.args[0], "-Command");
        assert_eq!(
            plan.args[1],
            "claude --resume session-123 --permission-mode bypassPermissions"
        );
        assert_eq!(plan.current_dir, Some("C:\\workspace\\project".to_string()));
        assert_eq!(plan.creation_flags, Some(0x00000010));
    }

    #[test]
    fn windows_terminal_rejected_as_terminal_type() {
        // windows-terminal 选项已移除：交由 Windows 默认终端决定，不再专门启动 wt.exe
        let request = LaunchRequest {
            working_directory: "C:\\workspace\\project".to_string(),
            mode: LaunchMode::Resume {
                session_id: "session-123".to_string(),
            },
            settings: default_settings("windows-terminal"),
        };

        let result = build_spawn_plan(&request);
        assert!(result.is_err(), "windows-terminal 应被拒绝");
    }

    #[test]
    fn legacy_default_preserves_existing_permission_args() {
        assert_eq!(
            LaunchSettings::legacy_default("powershell"),
            LaunchSettings {
                terminal_id: "powershell".to_string(),
                claude_executable: "claude".to_string(),
                claude_args: vec![
                    "--permission-mode".to_string(),
                    "bypassPermissions".to_string(),
                ],
                wrapper: None,
                maximize_window: false,
            }
        );
    }

    #[test]
    fn launch_settings_defaults_maximize_window_when_absent() {
        // 前端旧数据不含 maximizeWindow，应反序列化为 false（#[serde(default)]）
        let json = r#"{
            "terminalId": "cmd",
            "claudeExecutable": "claude",
            "claudeArgs": ["--permission-mode", "bypassPermissions"],
            "wrapper": null
        }"#;

        let settings: LaunchSettings = serde_json::from_str(json).unwrap();
        assert!(!settings.maximize_window);
    }

    #[test]
    fn launch_settings_reads_maximize_window_true() {
        let json = r#"{
            "terminalId": "wezterm",
            "claudeExecutable": "claude",
            "claudeArgs": [],
            "wrapper": null,
            "maximizeWindow": true
        }"#;

        let settings: LaunchSettings = serde_json::from_str(json).unwrap();
        assert!(settings.maximize_window);
    }

    #[test]
    fn blank_claude_executable_falls_back_to_claude() {
        let mut settings = default_settings("cmd");
        settings.claude_executable = "   ".to_string();
        let request = LaunchRequest {
            working_directory: "C:\\workspace\\project".to_string(),
            mode: LaunchMode::Resume {
                session_id: "session-123".to_string(),
            },
            settings,
        };

        let argv = build_agent_argv(&request);

        assert_eq!(argv[0], "claude");
    }

    #[test]
    fn launch_request_deserializes_frontend_camel_case_resume_mode() {
        let json = r#"{
            "workingDirectory": "C:\\workspace\\project",
            "mode": { "resume": { "sessionId": "session-123" } },
            "settings": {
                "terminalId": "cmd",
                "claudeExecutable": "claude",
                "claudeArgs": ["--permission-mode", "bypassPermissions"],
                "wrapper": null
            }
        }"#;

        let request: LaunchRequest = serde_json::from_str(json).unwrap();

        assert_eq!(
            request.mode,
            LaunchMode::Resume {
                session_id: "session-123".to_string()
            }
        );
    }
}
