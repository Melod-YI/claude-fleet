use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchSettings {
    pub terminal_id: String,
    pub claude_executable: String,
    pub claude_args: Vec<String>,
    pub wrapper: Option<CommandWrapper>,
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

    match request.settings.terminal_id.as_str() {
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
            current_dir: None,
            creation_flags: Some(DETACHED_PROCESS),
        }),
        "cmd" => Ok(SpawnPlan {
            command: "cmd.exe".to_string(),
            args: vec!["/K".to_string(), command_line(&process_argv)],
            current_dir: Some(request.working_directory.clone()),
            creation_flags: Some(CREATE_NEW_CONSOLE),
        }),
        "powershell" => Ok(SpawnPlan {
            command: "powershell.exe".to_string(),
            args: vec![
                "-Command".to_string(),
                command_line(&process_argv),
            ],
            current_dir: Some(request.working_directory.clone()),
            creation_flags: Some(CREATE_NEW_CONSOLE),
        }),
        "powershell7" => Ok(SpawnPlan {
            command: "pwsh.exe".to_string(),
            args: vec![
                "-Command".to_string(),
                command_line(&process_argv),
            ],
            current_dir: Some(request.working_directory.clone()),
            creation_flags: Some(CREATE_NEW_CONSOLE),
        }),
        other => Err(format!("不支持的终端类型: {}", other)),
    }
}

pub fn launch_session(request: &LaunchRequest) -> Result<(), String> {
    let plan = build_spawn_plan(request)?;
    spawn_plan(&plan)
}

pub fn spawn_plan(plan: &SpawnPlan) -> Result<(), String> {
    let mut command = Command::new(&plan.command);
    command.args(&plan.args);

    if let Some(current_dir) = &plan.current_dir {
        command.current_dir(current_dir);
    }

    #[cfg(target_os = "windows")]
    if let Some(flags) = plan.creation_flags {
        use std::os::windows::process::CommandExt;
        command.creation_flags(flags);
    }

    command
        .spawn()
        .map(|_| ())
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
            }
        );
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
