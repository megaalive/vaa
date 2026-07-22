//! Windows process-tree ownership using a Job Object.

use std::os::windows::io::AsRawHandle;
use std::process::{Child, Command, Stdio};

use win32job::{ExtendedLimitInfo, Job};

pub(crate) fn configure(_command: &mut Command) {}

pub(crate) struct ProcessTree {
    job: Option<Job>,
}

impl ProcessTree {
    pub(crate) fn attach(child: &Child) -> Result<Self, String> {
        let mut limits = ExtendedLimitInfo::new();
        limits.limit_kill_on_job_close();
        let job = Job::create_with_limit_info(&limits).map_err(|error| error.to_string())?;
        if let Err(error) = job.assign_process(child.as_raw_handle() as isize) {
            // Assignment failure leaves the process outside the Job Object.
            let _ = Command::new("taskkill")
                .args(["/PID", &child.id().to_string(), "/T", "/F"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            return Err(error.to_string());
        }
        Ok(Self { job: Some(job) })
    }

    pub(crate) fn terminate(&mut self, child: &mut Child) {
        let pid = child.id();
        // Kill the process tree first. Closing a Job with KILL_ON_JOB_CLOSE while
        // descendants are still running has been observed to block CloseHandle for
        // the lifetime of a long-running grandchild (e.g. ping -n 60).
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = child.kill();
        let _ = child.wait();
        if let Some(job) = self.job.take() {
            drop(job);
        }
    }
}

impl Drop for ProcessTree {
    fn drop(&mut self) {
        if let Some(job) = self.job.take() {
            drop(job);
        }
    }
}
