use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

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

        // Windows: prefer an empty pipe we close immediately over Stdio::null(),
        // which has hung CPython stdin.buffer.read under CI load.
        if config.stdin_null {
            if cfg!(windows) {
                cmd.stdin(Stdio::piped());
            } else {
                cmd.stdin(Stdio::null());
            }
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

        let mut child = cmd.spawn().map_err(|e| ProcessError::Spawn {
            program: program_str.clone(),
            detail: e.to_string(),
        })?;

        if config.stdin_null {
            if let Some(stdin) = child.stdin.take() {
                drop(stdin);
            }
        }

        let overflow = Arc::new(AtomicBool::new(false));
        let total_bytes = Arc::new(AtomicU64::new(0));
        let limit = config.max_output_bytes;

        let stdout_pipe = child.stdout.take().expect("stdout piped");
        let stderr_pipe = child.stderr.take().expect("stderr piped");

        let (stdout_tx, stdout_rx) = mpsc::channel::<Vec<u8>>();
        let (stderr_tx, stderr_rx) = mpsc::channel::<Vec<u8>>();

        let overflow_out = Arc::clone(&overflow);
        let total_out = Arc::clone(&total_bytes);
        let stdout_handle = thread::spawn(move || {
            let _ = stdout_tx.send(drain_capped(stdout_pipe, limit, &total_out, &overflow_out));
        });
        let overflow_err = Arc::clone(&overflow);
        let total_err = Arc::clone(&total_bytes);
        let stderr_handle = thread::spawn(move || {
            let _ = stderr_tx.send(drain_capped(stderr_pipe, limit, &total_err, &overflow_err));
        });

        let deadline = Instant::now() + config.timeout;
        let mut timed_out = false;
        let exit_code;

        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    exit_code = status.code();
                    break;
                }
                Ok(None) => {}
                Err(error) => {
                    kill_child(&mut child);
                    let _ = child.wait();
                    let _ = stdout_handle.join();
                    let _ = stderr_handle.join();
                    return Err(ProcessError::Spawn {
                        program: program_str,
                        detail: error.to_string(),
                    });
                }
            }

            if overflow.load(Ordering::Relaxed) {
                kill_child(&mut child);
                let _ = child.wait();
                let _ = stdout_handle.join();
                let _ = stderr_handle.join();
                let _ = stdout_rx.recv();
                let _ = stderr_rx.recv();
                return Err(ProcessError::OutputOverflow { limit });
            }

            if Instant::now() >= deadline {
                kill_child(&mut child);
                timed_out = true;
                let _ = child.wait();
                exit_code = None;
                break;
            }

            thread::sleep(Duration::from_millis(5));
        }

        let stdout = stdout_rx.recv().unwrap_or_default();
        let stderr = stderr_rx.recv().unwrap_or_default();
        let _ = stdout_handle.join();
        let _ = stderr_handle.join();

        if overflow.load(Ordering::Relaxed) {
            return Err(ProcessError::OutputOverflow { limit });
        }

        if timed_out {
            return Err(ProcessError::Timeout {
                duration: config.timeout,
            });
        }

        Ok(ProcessOutput {
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            exit_code,
            timed_out: false,
        })
    }
}

fn drain_capped(
    mut reader: impl Read,
    limit: u64,
    total: &AtomicU64,
    overflow: &AtomicBool,
) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8 * 1024];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let so_far = total.fetch_add(n as u64, Ordering::Relaxed);
                if so_far.saturating_add(n as u64) > limit {
                    overflow.store(true, Ordering::Relaxed);
                    // Keep draining so the child does not block on a full pipe.
                    continue;
                }
                buf.extend_from_slice(&chunk[..n]);
            }
        }
    }
    buf
}

fn kill_child(child: &mut std::process::Child) {
    // Until R3 (process-group / Job Object at spawn), Unix children share the
    // parent's PGID. Do not `kill -<pid>` (process *group*): that silently fails
    // and leaves flood/timeout paths blocked forever on `child.wait()`.
    #[cfg(unix)]
    {
        let _ = child.kill();
    }
    #[cfg(windows)]
    {
        let pid = child.id();
        let _ = Command::new("taskkill")
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
            timeout: Duration::from_millis(200),
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

    fn python_program() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from("python")
        } else {
            PathBuf::from("python3")
        }
    }

    fn python_allowed_env() -> Vec<String> {
        vec![
            "PATH".to_owned(),
            "HOME".to_owned(),
            "USER".to_owned(),
            "SYSTEMROOT".to_owned(),
            "WINDIR".to_owned(),
            "SYSTEMDRIVE".to_owned(),
            "PATHEXT".to_owned(),
            "COMSPEC".to_owned(),
        ]
    }

    #[test]
    fn streaming_cap_rejects_flood_before_exit() {
        let cfg = ProcessConfig {
            program: python_program(),
            args: vec![
                "-c".to_owned(),
                "import sys\nwhile True:\n sys.stdout.write('x'*4096); sys.stdout.flush()"
                    .to_owned(),
            ],
            timeout: Duration::from_secs(10),
            max_output_bytes: 64 * 1024,
            allowed_env: python_allowed_env(),
            ..ProcessConfig::default()
        };
        let result = ProcessRunner::run(&cfg);
        assert!(
            matches!(result, Err(ProcessError::OutputOverflow { .. })),
            "expected OutputOverflow, got {result:?}"
        );
    }

    #[test]
    fn null_stdin_yields_immediate_eof() {
        let cfg = ProcessConfig {
            program: python_program(),
            args: vec![
                "-c".to_owned(),
                "import sys; data=sys.stdin.buffer.read(); print(len(data))".to_owned(),
            ],
            timeout: Duration::from_secs(10),
            max_output_bytes: 1_048_576,
            allowed_env: python_allowed_env(),
            stdin_null: true,
            ..ProcessConfig::default()
        };
        let output = ProcessRunner::run(&cfg).expect("python stdin EOF");
        assert_eq!(output.stdout.trim(), "0");
    }
}
