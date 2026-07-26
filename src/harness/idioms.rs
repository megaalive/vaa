//! Verified idiom catalog v0 — guidance snippets for agent authoring.
//!
//! Snippets are **not** acceptance authority. Sealed SemASM evidence decides
//! acceptance; idioms only reduce common ABI / leaf repair mistakes.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::session::HarnessError;

/// Catalog schema version written into `idioms.json` / CLI output.
pub const IDIOM_CATALOG_SCHEMA_VERSION: &str = "0.1";

/// How strongly an idiom has been exercised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdiomEvidenceLevel {
    /// Appears in CI-proven repaired fixtures / corpus.
    VerifiedInCi,
    /// Authoring hint only — not CI-proven as a standalone leaf.
    Guidance,
}

impl IdiomEvidenceLevel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VerifiedInCi => "verified_in_ci",
            Self::Guidance => "guidance",
        }
    }
}

/// One authoring idiom entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdiomEntry {
    pub idiom_id: String,
    pub target: String,
    pub dialect: String,
    pub snippet: String,
    pub evidence_level: IdiomEvidenceLevel,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known_limits: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_effects: Vec<String>,
    /// Optional leaf / shape tags used for selection (`max_i64`, `count_byte`, …).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shapes: Vec<String>,
}

/// Catalog document returned by `vaa agent idioms` / written at prepare.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdiomCatalog {
    pub schema_version: String,
    /// Honesty banner for controllers.
    pub note: String,
    pub idioms: Vec<IdiomEntry>,
}

const CATALOG_NOTE: &str = "Idiom snippets are guidance only — not acceptance authority. \
SemASM sealed evidence decides verified/accepted.";

/// Embedded v0 catalog (3–5 snippets).
#[must_use]
pub fn embedded_catalog() -> Vec<IdiomEntry> {
    vec![
        IdiomEntry {
            idiom_id: "win64_preserve_rbx_v1".into(),
            target: "x86_64-pc-windows-msvc".into(),
            dialect: "nasm".into(),
            snippet: "\
; Win64: RBX is callee-saved — preserve around use
    push rbx
    ; … use rbx …
    pop rbx
"
            .into(),
            evidence_level: IdiomEvidenceLevel::Guidance,
            known_limits: vec![
                "Does not cover full Win64 home-space / shadow space layout".into(),
                "Guidance only — not a sealed leaf".into(),
            ],
            semantic_effects: vec!["abi.callee_saved.rbx".into()],
            shapes: vec![],
        },
        IdiomEntry {
            idiom_id: "win64_framed_leaf_v1".into(),
            target: "x86_64-pc-windows-msvc".into(),
            dialect: "nasm".into(),
            snippet: "\
; Win64 framed leaf: balanced prologue/epilogue
    push rbp
    mov rbp, rsp
    sub rsp, 32          ; shadow space when calling; omit if pure leaf
    ; … body …
    mov rsp, rbp
    pop rbp
    ret
"
            .into(),
            evidence_level: IdiomEvidenceLevel::Guidance,
            known_limits: vec![
                "Stack allocation size is task-specific".into(),
                "Must keep RSP 16-byte aligned before calls".into(),
            ],
            semantic_effects: vec!["abi.frame".into(), "abi.stack_balance".into()],
            shapes: vec![],
        },
        IdiomEntry {
            idiom_id: "x86_signed_max_cmov_v1".into(),
            target: "x86_64-pc-windows-msvc".into(),
            dialect: "nasm".into(),
            snippet: "\
; max_i64 — signed max via cmovg (Win64: rcx=a, rdx=b → rax)
    mov rax, rcx
    cmp rdx, rax
    cmovg rax, rdx
    ret
"
            .into(),
            evidence_level: IdiomEvidenceLevel::VerifiedInCi,
            known_limits: vec![
                "Requires SemASM modeling cmov* as Select".into(),
                "Win64 arg regs; SysV uses rdi/rsi".into(),
            ],
            semantic_effects: vec!["arith.signed_max".into()],
            shapes: vec!["max_i64".into()],
        },
        IdiomEntry {
            idiom_id: "x86_byte_scan_v1".into(),
            target: "x86_64-pc-windows-msvc".into(),
            dialect: "nasm".into(),
            snippet: "\
; Byte scan loop skeleton (Win64: rcx=buf, rdx=len, r8=needle)
    xor eax, eax
    test rdx, rdx
    jz .done
.loop:
    cmp byte [rcx], r8b
    ; je / jne depending on find vs count
    inc rcx
    dec rdx
    jnz .loop
.done:
    ret
"
            .into(),
            evidence_level: IdiomEvidenceLevel::VerifiedInCi,
            known_limits: vec![
                "Does not encode length-return vs count semantics".into(),
                "Memory leaves typically land verified_under_preconditions".into(),
            ],
            semantic_effects: vec!["mem.byte_scan".into()],
            shapes: vec![
                "count_byte".into(),
                "find_first_byte".into(),
                "find_last_byte".into(),
                "replace_byte".into(),
            ],
        },
        IdiomEntry {
            idiom_id: "sysv_preserve_rbx_v1".into(),
            target: "x86_64-unknown-linux-gnu".into(),
            dialect: "nasm".into(),
            snippet: "\
; SysV: RBX is callee-saved — preserve around use
    push rbx
    ; … use rbx …
    pop rbx
"
            .into(),
            evidence_level: IdiomEvidenceLevel::Guidance,
            known_limits: vec!["Optional SysV note — not a sealed leaf".into()],
            semantic_effects: vec!["abi.callee_saved.rbx".into()],
            shapes: vec![],
        },
    ]
}

/// Select ≤5 idioms for a target (and optional leaf/shape symbol).
#[must_use]
pub fn select_idioms(target: &str, shape: Option<&str>) -> Vec<IdiomEntry> {
    let all = embedded_catalog();
    let mut selected: Vec<IdiomEntry> = Vec::new();

    if let Some(shape) = shape {
        for entry in &all {
            if entry.shapes.iter().any(|s| s == shape) && target_matches(entry, target) {
                selected.push(entry.clone());
            }
        }
    }

    for entry in &all {
        if selected.len() >= 5 {
            break;
        }
        if !target_matches(entry, target) {
            continue;
        }
        if selected.iter().any(|e| e.idiom_id == entry.idiom_id) {
            continue;
        }
        // Prefer target-specific ABI guidance + shape matches already added.
        if entry.shapes.is_empty() || shape.is_some_and(|s| entry.shapes.iter().any(|x| x == s)) {
            selected.push(entry.clone());
        }
    }

    // If still empty (unknown target), return guidance-only global hints capped.
    if selected.is_empty() {
        selected.extend(
            all.into_iter()
                .filter(|e| matches!(e.evidence_level, IdiomEvidenceLevel::Guidance))
                .take(3),
        );
    }

    selected.truncate(5);
    selected
}

fn target_matches(entry: &IdiomEntry, target: &str) -> bool {
    entry.target == target
        || (target.starts_with("x86_64")
            && entry.target.starts_with("x86_64")
            && entry.shapes.iter().any(|s| {
                matches!(
                    s.as_str(),
                    "max_i64"
                        | "count_byte"
                        | "find_first_byte"
                        | "find_last_byte"
                        | "replace_byte"
                )
            }))
}

/// Build a catalog document for CLI / prepare output.
#[must_use]
pub fn catalog_for(target: &str, shape: Option<&str>) -> IdiomCatalog {
    IdiomCatalog {
        schema_version: IDIOM_CATALOG_SCHEMA_VERSION.to_owned(),
        note: CATALOG_NOTE.to_owned(),
        idioms: select_idioms(target, shape),
    }
}

/// Write `idioms.json` into a prepare workspace (≤5 selected entries).
pub fn write_idioms_json(
    workspace: &Path,
    target: &str,
    shape: Option<&str>,
) -> Result<std::path::PathBuf, HarnessError> {
    let catalog = catalog_for(target, shape);
    let path = workspace.join("idioms.json");
    fs::write(&path, serde_json::to_string_pretty(&catalog)?)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_required_ids() {
        let ids: Vec<_> = embedded_catalog().into_iter().map(|e| e.idiom_id).collect();
        assert!(ids.contains(&"win64_preserve_rbx_v1".into()));
        assert!(ids.contains(&"win64_framed_leaf_v1".into()));
        assert!(ids.contains(&"x86_signed_max_cmov_v1".into()));
        assert!(ids.contains(&"x86_byte_scan_v1".into()));
    }

    #[test]
    fn select_max_i64_prefers_cmov_idiom() {
        let selected = select_idioms("x86_64-pc-windows-msvc", Some("max_i64"));
        assert!(selected
            .iter()
            .any(|e| e.idiom_id == "x86_signed_max_cmov_v1"));
        assert!(selected.len() <= 5);
    }

    #[test]
    fn note_denies_acceptance_authority() {
        let cat = catalog_for("x86_64-pc-windows-msvc", None);
        assert!(cat.note.contains("not acceptance authority"));
    }
}
