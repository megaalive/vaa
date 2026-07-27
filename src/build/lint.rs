//! Hosted / asm source lints (heuristic; never seals).

use serde::{Deserialize, Serialize};

/// One lint finding for controllers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceLintFinding {
    pub code: String,
    pub severity: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

/// Scan NASM Intel source for common hosted foot-guns.
#[must_use]
pub fn lint_nasm_source(source: &str, target: &str) -> Vec<SourceLintFinding> {
    let mut out = Vec::new();
    let sysv = target.contains("linux")
        || target.contains("gnu")
        || target.contains("elf")
        || target == "elf64";

    for (idx, line) in source.lines().enumerate() {
        let line_no = (idx + 1) as u32;
        let trimmed = line.trim();
        if trimmed.starts_with(';') {
            continue;
        }
        // RIP_INDEX: [rel sym+reg] or [rel sym+reg*scale]
        if let Some(pos) = trimmed.find("[rel ") {
            let rest = &trimmed[pos..];
            if rest.contains('+') {
                out.push(SourceLintFinding {
                    code: "RIP_INDEX".into(),
                    severity: "error".into(),
                    message: "RIP-relative memory with index/displacement register \
                              (NASM warning → typical AV); use lea + [base+index]"
                        .into(),
                    line: Some(line_no),
                });
            }
        }
        // Soft note for SysV: live counter in rsi/edi across call is a common bug;
        // full CALLER_SAVED needs SemASM analyze — emit advisory when both appear.
        if sysv && (trimmed.contains("call ") || trimmed.contains("syscall")) && idx > 0 {
            // Look back a few lines for esi/edi counters — advisory only.
            let prior: Vec<&str> = source.lines().take(idx).collect();
            let window = prior
                .iter()
                .rev()
                .take(8)
                .copied()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n")
                .to_ascii_lowercase();
            if window.contains("esi")
                || window.contains("edi")
                || window.contains("rsi")
                || window.contains("rdi")
            {
                // Only once per file to avoid noise.
                if !out.iter().any(|f| f.code == "CALLER_SAVED") {
                    out.push(SourceLintFinding {
                        code: "CALLER_SAVED".into(),
                        severity: "warning".into(),
                        message: "possible SysV volatile live across call/syscall \
                                  (prefer rbx/r12+ for counters); confirm with semasm abi"
                            .into(),
                        line: Some(line_no),
                    });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rip_index_detected() {
        let src = "mov al, [rel inbuf+rbx]\n";
        let f = lint_nasm_source(src, "win64");
        assert!(f.iter().any(|x| x.code == "RIP_INDEX"));
    }
}
