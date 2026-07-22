//! Unix process-group setup and termination.

use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub(crate) fn configure(command: &mut Command) {
    // New process group so descendants share PGID == child PID and can be
    // signalled with `kill -<pid>` without touching the VAA process group.
    command.process_group(0);
}

pub(crate) struct ProcessTree {
    process_group: String,
}

impl ProcessTree {
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn attach(child: &Child) -> Result<Self, String> {
        Ok(Self {
            process_group: format!("-{}", child.id()),
        })
    }

    pub(crate) fn terminate(&mut self, child: &mut Child) {
        let _ = signal_group("-TERM", &self.process_group);
        let deadline = Instant::now() + Duration::from_millis(200);
        while Instant::now() < deadline && group_exists(&self.process_group) {
            thread::sleep(Duration::from_millis(10));
        }

        if group_exists(&self.process_group) {
            let _ = signal_group("-KILL", &self.process_group);
        }

        let _ = child.kill();
    }
}

fn signal_group(signal: &str, process_group: &str) -> bool {
    Command::new("kill")
        .args([signal, "--", process_group])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn group_exists(process_group: &str) -> bool {
    signal_group("-0", process_group)
}
