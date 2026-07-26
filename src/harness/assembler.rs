//! Assembler flavor for harness envelopes (NASM + GAS).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Which assembler dialect the agent is expected to emit.
///
/// - [`Self::Nasm`]: supported for x86_64 Win64 / SysV (SemASM `nasm-intel`).
/// - [`Self::Gas`]: supported for AArch64 / RISC-V Linux (SemASM `gas-unified`);
///   still **fail-closed** on x86_64 where SemASM remains NASM-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AssemblerFlavor {
    /// NASM Intel syntax (`.asm`) — supported for Win64 / SysV x86_64 today.
    #[default]
    Nasm,
    /// GNU assembler AT&T / unified (`.S`) — AArch64 / RISC-V Linux.
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

    /// True when this flavor matches SemASM's dialect for `target`.
    #[must_use]
    pub fn is_supported_for(self, target: &str) -> bool {
        match self {
            Self::Nasm => is_x86_64_target(target),
            Self::Gas => is_gas_native_target(target),
        }
    }

    /// Fail closed when flavor/target pairing is not wired.
    pub fn ensure_supported_for(self, target: &str) -> Result<(), String> {
        if self.is_supported_for(target) {
            Ok(())
        } else {
            Err(match self {
                Self::Nasm => format!(
                    "assembler flavor `nasm` is only supported for x86_64 targets \
                     (got `{target}`; use `--assembler gas` for aarch64/riscv64)"
                ),
                Self::Gas => format!(
                    "assembler flavor `gas` is supported for aarch64/riscv64 Linux targets only \
                     (got `{target}`; x86_64 remains NASM/Intel in SemASM+VAA)"
                ),
            })
        }
    }

    /// Back-compat: NASM always; GAS only when no target is known yet.
    #[must_use]
    pub const fn is_supported(self) -> bool {
        matches!(self, Self::Nasm)
    }

    /// Fail closed without a target (treat GAS as reserved).
    pub fn ensure_supported(self) -> Result<(), String> {
        if self.is_supported() {
            Ok(())
        } else {
            Err("assembler flavor `gas` requires a gas-native target \
                 (aarch64-unknown-linux-gnu or riscv64gc-unknown-linux-gnu); \
                 pass the locked task so the harness can validate the pairing"
                .into())
        }
    }

    /// Default assembler program on PATH for this flavor/target.
    #[must_use]
    pub fn default_program(self, target: &str) -> PathBuf {
        match self {
            Self::Nasm => PathBuf::from("nasm"),
            Self::Gas => PathBuf::from(gas_program_for_target(target)),
        }
    }

    /// CLI args to assemble `source` → `object` for this flavor/target.
    #[must_use]
    pub fn assemble_args(self, source: &Path, object: &Path, target: &str) -> Vec<String> {
        match self {
            Self::Nasm => vec![
                "-f".to_owned(),
                crate::build::nasm_format_for_target(target).to_owned(),
                "-o".to_owned(),
                object.to_string_lossy().into_owned(),
                source.to_string_lossy().into_owned(),
            ],
            Self::Gas => vec![
                source.to_string_lossy().into_owned(),
                "-o".to_owned(),
                object.to_string_lossy().into_owned(),
            ],
        }
    }

    /// Seed stub for prepare when no `--seed` is given.
    #[must_use]
    pub fn seed_stub(self, symbol: &str, target: &str) -> String {
        match self {
            Self::Nasm => format!(
                "; TODO: implement `{symbol}` for target {target} (assembler=nasm)\n\
                 bits 64\ndefault rel\nsection .text\nglobal {symbol}\n{symbol}:\n    ret\n"
            ),
            Self::Gas => format!(
                "// TODO: implement `{symbol}` for target {target} (assembler=gas)\n\
                 .text\n.global {symbol}\n.type {symbol}, %function\n{symbol}:\n    ret\n"
            ),
        }
    }
}

fn is_x86_64_target(target: &str) -> bool {
    let t = target.to_ascii_lowercase();
    t.contains("x86_64") || t == "win64" || t == "elf64" || t == "sysv64"
}

// Dialect wiring only: which `as` speaks this target's syntax. This is NOT a
// claim that VAA can agent-verify the target end-to-end. AArch64 is proven by
// the GAS harness gate; riscv64 stays fail-closed at the capability layer
// (`TargetCapabilities::for_target` → Unknown) until a proving gate exists.
fn is_gas_native_target(target: &str) -> bool {
    let t = target.to_ascii_lowercase();
    t.contains("aarch64") || t.contains("riscv64")
}

fn gas_program_for_target(target: &str) -> &'static str {
    let t = target.to_ascii_lowercase();
    if t.contains("aarch64") {
        "aarch64-linux-gnu-as"
    } else if t.contains("riscv64") {
        "riscv64-linux-gnu-as"
    } else {
        "as"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nasm_x86_supported_gas_x86_closed() {
        assert!(AssemblerFlavor::Nasm
            .ensure_supported_for("x86_64-pc-windows-msvc")
            .is_ok());
        assert!(AssemblerFlavor::Gas
            .ensure_supported_for("x86_64-unknown-linux-gnu")
            .is_err());
    }

    #[test]
    fn gas_aarch64_and_riscv_supported() {
        assert!(AssemblerFlavor::Gas
            .ensure_supported_for("aarch64-unknown-linux-gnu")
            .is_ok());
        assert!(AssemblerFlavor::Gas
            .ensure_supported_for("riscv64gc-unknown-linux-gnu")
            .is_ok());
        assert!(AssemblerFlavor::Nasm
            .ensure_supported_for("aarch64-unknown-linux-gnu")
            .is_err());
    }

    #[test]
    fn serde_round_trip() {
        let json = serde_json::to_string(&AssemblerFlavor::Gas).unwrap();
        assert_eq!(json, "\"gas\"");
        let back: AssemblerFlavor = serde_json::from_str(&json).unwrap();
        assert_eq!(back, AssemblerFlavor::Gas);
    }

    #[test]
    fn gas_assemble_args_match_semasm_shape() {
        let args = AssemblerFlavor::Gas.assemble_args(
            Path::new("exit.S"),
            Path::new("exit.o"),
            "aarch64-unknown-linux-gnu",
        );
        assert_eq!(args, vec!["exit.S", "-o", "exit.o"]);
        assert_eq!(
            AssemblerFlavor::Gas.default_program("aarch64-unknown-linux-gnu"),
            PathBuf::from("aarch64-linux-gnu-as")
        );
    }
}
