//! Task loading and validation errors.

use std::path::PathBuf;

/// Errors produced while parsing or validating a task file.
#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    /// Filesystem read failed.
    #[error("failed to read task file `{path}`: {source}")]
    Io {
        /// Path that could not be read.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// TOML syntax or structural deserialize error.
    #[error("invalid task TOML in `{path}`: {message}")]
    Parse {
        /// Path being parsed.
        path: PathBuf,
        /// Human-readable parse message.
        message: String,
    },

    /// Schema or semantic validation failure.
    #[error("task validation failed: {0}")]
    Validation(String),

    /// Multiple validation issues.
    #[error("task validation failed with {count} issue(s):\n{details}")]
    ValidationMany {
        /// Number of issues.
        count: usize,
        /// Multi-line detail list.
        details: String,
    },
}

impl TaskError {
    /// Build a multi-diagnostic validation error.
    #[must_use]
    pub fn from_diagnostics(diagnostics: &[String]) -> Self {
        if diagnostics.len() == 1 {
            return Self::Validation(diagnostics[0].clone());
        }
        let details = diagnostics
            .iter()
            .enumerate()
            .map(|(i, d)| format!("  {}. {d}", i + 1))
            .collect::<Vec<_>>()
            .join("\n");
        Self::ValidationMany {
            count: diagnostics.len(),
            details,
        }
    }
}
