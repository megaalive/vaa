use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ProcessConfig {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub allowed_env: Vec<String>,
    pub timeout: Duration,
    pub max_output_bytes: u64,
    pub stdin_null: bool,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            program: PathBuf::new(),
            args: Vec::new(),
            working_dir: None,
            allowed_env: vec!["PATH".to_owned(), "HOME".to_owned(), "USER".to_owned()],
            timeout: Duration::from_secs(30),
            max_output_bytes: 1_048_576,
            stdin_null: true,
        }
    }
}

#[derive(Debug)]
pub struct ProcessOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("failed to start process `{program}`: {detail}")]
    Spawn { program: String, detail: String },
    #[error("process timed out after {duration:?}")]
    Timeout { duration: Duration },
    #[error("output exceeded {limit} bytes")]
    OutputOverflow { limit: u64 },
}

pub struct ProcessRunner;

impl ProcessRunner {
    pub fn run(config: &ProcessConfig) -> Result<ProcessOutput, ProcessError> {
        let program_str = config.program.to_string_lossy().to_string();
        let mut cmd = Command::new(&config.program);
        cmd.args(&config.args);

        if config.stdin_null {
            cmd.stdin(Stdio::null());
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(wd) = &config.working_dir {
            cmd.current_dir(wd);
        }

        cmd.env_clear();
        for var in &config.allowed_env {
            if let Ok(val) = std::env::var(var) {
                cmd.env(var, val);
            }
        }

        let child = cmd
            .spawn()
            .map_err(|e| ProcessError::Spawn {
                program: program_str.clone(),
                detail: e.to_string(),
            })?;

        let pid = child.id();
        let (tx, rx) = mpsc::channel();
        let _ = std::thread::spawn(move || {
            let _ = tx.send(child.wait_with_output());
        });

        match rx.recv_timeout(config.timeout) {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                let total_bytes = stdout.len() + stderr.len();
                if total_bytes as u64 > config.max_output_bytes {
                    return Err(ProcessError::OutputOverflow {
                        limit: config.max_output_bytes,
                    });
                }

                Ok(ProcessOutput {
                    stdout,
                    stderr,
                    exit_code: output.status.code(),
                    timed_out: false,
                })
            }
            Ok(Err(e)) => Err(ProcessError::Spawn {
                program: program_str,
                detail: e.to_string(),
            }),
            Err(_) => {
                kill_process(config.program.to_str().unwrap_or("?"), pid);
                Err(ProcessError::Timeout {
                    duration: config.timeout,
                })
            }
        }
    }
}

#[allow(unused_variables)]
fn kill_process(name: &str, pid: u32) {
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .arg("-TERM")
            .arg(format!("-{pid}"))
            .output();
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID"])
            .arg(pid.to_string())
            .output();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn timeout_kills_child() {
        let cfg = ProcessConfig {
            program: if cfg!(windows) {
                PathBuf::from("ping")
            } else {
                PathBuf::from("sleep")
            },
            args: if cfg!(windows) {
                vec!["-n".to_owned(), "60".to_owned(), "127.0.0.1".to_owned()]
            } else {
                vec!["60".to_owned()]
            },
            timeout: Duration::from_millis(10),
            max_output_bytes: 1_048_576,
            ..ProcessConfig::default()
        };
        let result = ProcessRunner::run(&cfg);
        assert!(matches!(result, Err(ProcessError::Timeout { .. })));
    }

    #[test]
    fn binary_not_found() {
        let cfg = ProcessConfig {
            program: PathBuf::from("nonexistent_tool_xyz"),
            ..ProcessConfig::default()
        };
        let result = ProcessRunner::run(&cfg);
        assert!(matches!(result, Err(ProcessError::Spawn { .. })));
    }
}
