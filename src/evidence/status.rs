use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceStatus {
    Verified,
    /// Accepted under explicitly recorded SemASM caller preconditions (Sei).
    ///
    /// Must not be silently promoted to [`Self::Verified`].
    VerifiedUnderPreconditions,
    Violated,
    Incomplete,
    Failed,
}

impl EvidenceStatus {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Verified | Self::VerifiedUnderPreconditions | Self::Violated | Self::Failed
        )
    }

    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Verified | Self::VerifiedUnderPreconditions)
    }

    #[must_use]
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Verified | Self::VerifiedUnderPreconditions => 0,
            Self::Violated => 3,
            Self::Incomplete => 4,
            Self::Failed => 5,
        }
    }
}
