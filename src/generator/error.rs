//! Errors for stack lock and external generator specs.

use std::path::PathBuf;

/// Errors produced while parsing or validating generator bridge documents.
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    /// Filesystem read failed.
    #[error("failed to read `{path}`: {source}")]
    Io {
        /// Path that could not be read.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// TOML syntax or structural deserialize error.
    #[error("invalid TOML in `{path}`: {message}")]
    Parse {
        /// Path being parsed.
        path: PathBuf,
        /// Human-readable parse message.
        message: String,
    },

    /// Schema or semantic validation failure.
    #[error("validation failed: {0}")]
    Validation(String),

    /// Multiple validation issues.
    #[error("validation failed with {count} issue(s):\n{details}")]
    ValidationMany {
        /// Number of issues.
        count: usize,
        /// Multi-line detail list.
        details: String,
    },
}

impl GeneratorError {
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
