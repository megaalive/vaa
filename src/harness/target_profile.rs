//! Target authoring profiles embedded into the harness workspace at prepare.
//!
//! Prefer live `semasm target profile <target> --format json` when SemASM is
//! discoverable; otherwise use a small embedded Win64 / SysV / AArch64 fallback
//! with enough ABI register and stack facts for agent authoring. Profiles are
//! guidance — not acceptance authority.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use serde_json::Value;

use crate::process::{ProcessConfig, ProcessRunner};
use crate::semasm::doctor::{semasm_subprocess_allowed_env, SemasmDoctor, ENV_SEMASM_BIN};
use crate::sha256_digest_prefixed;

/// Resolved target profile plus its content digest.
#[derive(Debug, Clone)]
pub struct ResolvedTargetProfile {
    pub profile: Value,
    pub digest: String,
    pub source: &'static str,
}

/// Resolve a target authoring profile (live SemASM, else embedded fallback).
#[must_use]
pub fn resolve_target_profile(target: &str) -> ResolvedTargetProfile {
    if let Some(profile) = try_live_semasm_profile(target) {
        let bytes = serde_json::to_vec(&profile).unwrap_or_default();
        return ResolvedTargetProfile {
            digest: sha256_digest_prefixed(&bytes),
            profile,
            source: "semasm_cli",
        };
    }
    let profile = embedded_profile(target).unwrap_or_else(|| minimal_unknown_profile(target));
    let bytes = serde_json::to_vec(&profile).unwrap_or_default();
    ResolvedTargetProfile {
        digest: sha256_digest_prefixed(&bytes),
        profile,
        source: "vaa_embedded",
    }
}

/// Write `target-profile.json` under `workspace` and return path + digest.
pub fn write_target_profile(
    workspace: &Path,
    target: &str,
) -> std::io::Result<(std::path::PathBuf, String)> {
    let resolved = resolve_target_profile(target);
    let path = workspace.join("target-profile.json");
    let pretty = serde_json::to_string_pretty(&resolved.profile)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, pretty)?;
    Ok((path, resolved.digest))
}

fn try_live_semasm_profile(target: &str) -> Option<Value> {
    let binary = std::env::var_os(ENV_SEMASM_BIN)
        .map(std::path::PathBuf::from)
        .or_else(|| SemasmDoctor::run().binary_path)
        .or_else(|| {
            Command::new("semasm")
                .arg("--version")
                .output()
                .ok()
                .and_then(|_| SemasmDoctor::run().binary_path)
        })?;

    let config = ProcessConfig {
        program: binary,
        args: vec![
            "target".into(),
            "profile".into(),
            target.to_owned(),
            "--format".into(),
            "json".into(),
        ],
        timeout: Duration::from_secs(30),
        max_output_bytes: 1_048_576,
        allowed_env: semasm_subprocess_allowed_env(),
        ..ProcessConfig::default()
    };
    let output = ProcessRunner::run(&config).ok()?;
    let stdout = output.stdout.trim();
    if stdout.is_empty() {
        return None;
    }
    serde_json::from_str(stdout).ok()
}

/// Embedded profiles for the three CI-proven agent-verify targets.
#[must_use]
pub fn embedded_profile(target: &str) -> Option<Value> {
    let raw = match target {
        "x86_64-pc-windows-msvc" => EMBEDDED_WIN64,
        "x86_64-unknown-linux-gnu" => EMBEDDED_SYSV,
        "aarch64-unknown-linux-gnu" => EMBEDDED_AARCH64,
        _ => return None,
    };
    serde_json::from_str(raw).ok()
}

fn minimal_unknown_profile(target: &str) -> Value {
    serde_json::json!({
        "profile_id": format!("{target}@unknown"),
        "target": target,
        "syntax": "unknown",
        "dialect": "unknown",
        "file_template": "; unknown target — no embedded authoring profile\n",
        "symbol_rules": {
            "export_directive": "global",
            "text_section": "section .text",
            "leading_underscore": false,
            "source_extension": ".asm"
        },
        "abi": {
            "parameter_registers": [],
            "return_register": "",
            "preserved_registers": [],
            "volatile_registers": [],
            "stack_alignment": 16,
            "shadow_space_bytes": 0,
            "red_zone_bytes": 0
        },
        "modeled_addressing": [],
        "supported_loop_idioms": [],
        "known_incomplete_patterns": ["target has no embedded VAA authoring profile"],
        "object_format": "unknown"
    })
}

const EMBEDDED_WIN64: &str = r#"{
  "profile_id": "x86_64-pc-windows-msvc@nasm-intel",
  "target": "x86_64-pc-windows-msvc",
  "syntax": "nasm-intel",
  "dialect": "nasm-intel",
  "file_template": "; {routine} — agent candidate skeleton\nBITS 64\nDEFAULT REL\n\nglobal {routine}\n\nsection .text\n{routine}:\n\t; TODO: implement\n\tret\n",
  "symbol_rules": {
    "export_directive": "global",
    "text_section": "section .text",
    "leading_underscore": false,
    "source_extension": ".asm"
  },
  "abi": {
    "parameter_registers": ["rcx", "rdx", "r8", "r9"],
    "return_register": "rax",
    "preserved_registers": ["rbx", "rbp", "rdi", "rsi", "r12", "r13", "r14", "r15"],
    "volatile_registers": ["rax", "rcx", "rdx", "r8", "r9", "r10", "r11"],
    "stack_alignment": 16,
    "shadow_space_bytes": 32,
    "red_zone_bytes": 0
  },
  "modeled_addressing": ["base+disp", "base+index*scale+disp", "rip-relative", "rsp/rbp frame locals"],
  "supported_loop_idioms": ["count-up induction with constant stride", "countdown (dec) induction with exclusive bound", "affine index patterns on single buffer leaves"],
  "known_incomplete_patterns": ["arbitrary control-flow invariants", "general alias analysis / formal memory safety", "full-ISA decode completeness certificate"],
  "object_format": "pe-coff"
}"#;

const EMBEDDED_SYSV: &str = r#"{
  "profile_id": "x86_64-unknown-linux-gnu@nasm-intel",
  "target": "x86_64-unknown-linux-gnu",
  "syntax": "nasm-intel",
  "dialect": "nasm-intel",
  "file_template": "; {routine} — agent candidate skeleton\nBITS 64\nDEFAULT REL\n\nglobal {routine}\n\nsection .text\n{routine}:\n\t; TODO: implement\n\tret\n",
  "symbol_rules": {
    "export_directive": "global",
    "text_section": "section .text",
    "leading_underscore": false,
    "source_extension": ".asm"
  },
  "abi": {
    "parameter_registers": ["rdi", "rsi", "rdx", "rcx", "r8", "r9"],
    "return_register": "rax",
    "preserved_registers": ["rbx", "rbp", "r12", "r13", "r14", "r15"],
    "volatile_registers": ["rax", "rcx", "rdx", "rsi", "rdi", "r8", "r9", "r10", "r11"],
    "stack_alignment": 16,
    "shadow_space_bytes": 0,
    "red_zone_bytes": 128
  },
  "modeled_addressing": ["base+disp", "base+index*scale+disp", "rip-relative", "rsp/rbp frame locals"],
  "supported_loop_idioms": ["count-up induction with constant stride", "countdown (dec) induction with exclusive bound", "affine index patterns on single buffer leaves"],
  "known_incomplete_patterns": ["arbitrary control-flow invariants", "general alias analysis / formal memory safety", "full-ISA decode completeness certificate"],
  "object_format": "elf"
}"#;

const EMBEDDED_AARCH64: &str = r#"{
  "profile_id": "aarch64-unknown-linux-gnu@gas-unified",
  "target": "aarch64-unknown-linux-gnu",
  "syntax": "gas-unified",
  "dialect": "gas-unified",
  "file_template": "// {routine} — agent candidate skeleton (AArch64)\n.global {routine}\n.text\n{routine}:\n\t// TODO: implement\n\tret\n",
  "symbol_rules": {
    "export_directive": ".global",
    "text_section": ".text",
    "leading_underscore": false,
    "source_extension": ".S"
  },
  "abi": {
    "parameter_registers": ["x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7"],
    "return_register": "x0",
    "preserved_registers": ["x19", "x20", "x21", "x22", "x23", "x24", "x25", "x26", "x27", "x28"],
    "volatile_registers": ["x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7", "x8", "x9", "x10", "x11", "x12", "x13", "x14", "x15", "x16", "x17"],
    "stack_alignment": 16,
    "shadow_space_bytes": 0,
    "red_zone_bytes": 0
  },
  "modeled_addressing": ["base+imm", "base+index", "pre/post-index", "pc-relative adr/adrp"],
  "supported_loop_idioms": ["count-up induction with constant stride", "countdown (dec) induction with exclusive bound", "affine index patterns on single buffer leaves"],
  "known_incomplete_patterns": ["arbitrary control-flow invariants", "general alias analysis / formal memory safety", "full-ISA decode completeness certificate"],
  "object_format": "elf"
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_win64_has_shadow_space() {
        let p = embedded_profile("x86_64-pc-windows-msvc").expect("win64");
        assert_eq!(p["abi"]["shadow_space_bytes"], 32);
        assert_eq!(p["abi"]["parameter_registers"][0], "rcx");
    }

    #[test]
    fn embedded_sysv_has_red_zone() {
        let p = embedded_profile("x86_64-unknown-linux-gnu").expect("sysv");
        assert_eq!(p["abi"]["red_zone_bytes"], 128);
        assert_eq!(p["abi"]["parameter_registers"][0], "rdi");
    }

    #[test]
    fn resolve_always_returns_profile() {
        let r = resolve_target_profile("x86_64-pc-windows-msvc");
        assert!(r.digest.starts_with("sha256:"));
        assert_eq!(r.profile["target"], "x86_64-pc-windows-msvc");
    }
}
