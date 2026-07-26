//! Assembler flavor for harness envelopes (extensible beyond NASM).

use serde::{Deserialize, Serialize};

/// Which assembler dialect the agent is expected to emit.
///
/// SemASM already understands gas dialects for AArch64/RISC-V. VAA's build /
/// object-inspect path is still NASM-hardcoded for x86_64, so [`Self::Gas`] is
/// recognized but **fail-closed** until that wiring lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AssemblerFlavor {
    /// NASM Intel syntax (`.asm`) — supported for Win64 / SysV x86_64 today.
    #[default]
    Nasm,
    /// GNU assembler AT&T / unified (`.S`) — reserved; not yet a VAA harness path.
    Gas,
}

impl AssemblerFlavor {
    /// Stable snake_case label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Nasm => "nasm",
            Self::Gas => "gas",
        }
    }

    /// Candidate source filename for this flavor.
    #[must_use]
    pub const fn candidate_filename(self) -> &'static str {
        match self {
            Self::Nasm => "candidate.asm",
            Self::Gas => "candidate.S",
        }
    }

    /// Whether the VAA harness may drive assemble/inspect for this flavor.
    #[must_use]
    pub const fn is_supported(self) -> bool {
        matches!(self, Self::Nasm)
    }

    /// Fail closed when the flavor is not yet wired through VAA.
    pub fn ensure_supported(self) -> Result<(), String> {
        if self.is_supported() {
            Ok(())
        } else {
            Err(format!(
                "assembler flavor `{}` is reserved but not yet supported by the VAA harness \
                 (supported today: nasm; SemASM gas dialects exist for non-x86 targets, \
                 but VAA build/inspect still assumes NASM for x86_64)",
                self.as_str()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nasm_is_default_and_supported() {
        assert_eq!(AssemblerFlavor::default(), AssemblerFlavor::Nasm);
        assert!(AssemblerFlavor::Nasm.ensure_supported().is_ok());
        assert!(AssemblerFlavor::Gas.ensure_supported().is_err());
    }

    #[test]
    fn serde_round_trip() {
        let json = serde_json::to_string(&AssemblerFlavor::Gas).unwrap();
        assert_eq!(json, "\"gas\"");
        let back: AssemblerFlavor = serde_json::from_str(&json).unwrap();
        assert_eq!(back, AssemblerFlavor::Gas);
    }
}
