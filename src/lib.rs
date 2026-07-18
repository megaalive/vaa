//! VAA library surface for the offline controller.
//!
//! The binary crate reuses these modules so unit tests can exercise parsing,
//! validation, and digests without spawning a process.

#![forbid(unsafe_code)]

pub mod exit_code;
pub mod run;
pub mod task;

pub use exit_code::ExitCode as VaaExitCode;
pub use run::{EventKind, EventLog, RunDir, RunDirPaths, RunId};
pub use task::{load_locked_task, load_task_file, LockedTask, Task, TaskError};

/// Package version embedded at compile time.
pub const VAA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maturity label for the current tree. Keep this honest.
pub const MATURITY: &str = "experimental";

/// Supported task schema major.minor for this release of the crate.
pub const TASK_SCHEMA_VERSION: &str = "0.1";
