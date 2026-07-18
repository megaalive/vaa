//! Process exit codes for the VAA CLI.
//!
//! These match the architecture plan §19.3. Commands that have not been
//! implemented yet must not invent success for missing evidence.

/// Stable CLI exit codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitCode {
    /// Accepted / command succeeded.
    Success = 0,
    /// Invalid user input or task schema.
    InvalidInput = 2,
    /// Contract or policy violated.
    Violated = 3,
    /// Evidence incomplete.
    Incomplete = 4,
    /// Tool or pipeline failure.
    ToolFailure = 5,
    /// Dependency incompatible.
    DependencyIncompatible = 6,
    /// Budget exhausted.
    BudgetExhausted = 7,
    /// Security block.
    SecurityBlock = 8,
    /// Internal invariant failure.
    Internal = 9,
}

impl ExitCode {
    /// Convert to a process exit code.
    #[must_use]
    pub fn as_std(self) -> std::process::ExitCode {
        std::process::ExitCode::from(self as u8)
    }

    /// Create from a raw exit code value.
    #[must_use]
    pub fn from_raw(code: u8) -> Self {
        match code {
            0 => Self::Success,
            2 => Self::InvalidInput,
            3 => Self::Violated,
            4 => Self::Incomplete,
            5 => Self::ToolFailure,
            6 => Self::DependencyIncompatible,
            7 => Self::BudgetExhausted,
            8 => Self::SecurityBlock,
            _ => Self::Internal,
        }
    }
}
