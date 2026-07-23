//! CryptOpt-like search driver (P7-C): mutate → optional ingest, no embedded CryptOpt.

use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::evidence::sha256_digest_prefixed;
use crate::process::{ProcessConfig, ProcessRunner};
use crate::run::RunDir;
use crate::task::LockedTask;

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("io: {0}")]
    Io(String),
    #[error("mutator: {0}")]
    Mutator(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAttempt {
    pub index: u32,
    pub source_digest: String,
    pub status: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchReport {
    pub attempts: Vec<SearchAttempt>,
    pub verified: bool,
    pub stopped_reason: String,
}

/// Deterministic fixture mutator: append `times` NOPs.
#[must_use]
pub fn mutate_nop_slide(seed: &str, times: u32) -> String {
    let mut out = seed.to_owned();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    let _ = writeln!(out, "; vaa-search nop-slide x{times}");
    for _ in 0..times {
        out.push_str("    nop\n");
    }
    out
}

fn run_external_mutator(program: &Path, seed: &str, index: u32) -> Result<String, SearchError> {
    let dir = std::env::temp_dir().join(format!("vaa_mut_{}_{}", std::process::id(), index));
    std::fs::create_dir_all(&dir).map_err(|e| SearchError::Io(e.to_string()))?;
    let seed_path = dir.join("seed.asm");
    let out_path = dir.join("out.asm");
    std::fs::write(&seed_path, seed).map_err(|e| SearchError::Io(e.to_string()))?;
    let out = ProcessRunner::run(&ProcessConfig {
        program: program.to_path_buf(),
        args: vec![
            seed_path.to_string_lossy().into_owned(),
            out_path.to_string_lossy().into_owned(),
            index.to_string(),
        ],
        timeout: Duration::from_secs(30),
        max_output_bytes: 1_048_576,
        stdin_null: true,
        ..ProcessConfig::default()
    })
    .map_err(|e| SearchError::Mutator(e.to_string()))?;
    if out.exit_code != Some(0) {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(SearchError::Mutator(format!(
            "exit={:?} stderr={}",
            out.exit_code, out.stderr
        )));
    }
    let source = std::fs::read_to_string(&out_path).map_err(|e| {
        let _ = std::fs::remove_dir_all(&dir);
        SearchError::Mutator(format!("missing output: {e}"))
    })?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(source)
}

/// Bounded search campaign. Offline mode only mutates + stages (unit-friendly).
/// Live ingest is left to the CLI (`vaa ingest`) so Gate stays deterministic.
pub fn run_search(
    locked: &LockedTask,
    seed_asm: &str,
    run_base: &Path,
    budget: u32,
    mutator: &str,
    mutator_command: Option<&Path>,
    offline_fixture: bool,
) -> Result<SearchReport, SearchError> {
    let task_budget = locked.task().budgets.max_candidates;
    let limit = budget.min(task_budget).max(1);
    let rundir = RunDir::create(run_base, &crate::run::RunId::generate())
        .map_err(|e| SearchError::Io(e.to_string()))?;

    let mut attempts = Vec::new();
    let verified = false;
    let mut stopped_reason = "budget_exhausted".to_owned();

    for i in 0..limit {
        let source = if let Some(cmd) = mutator_command {
            run_external_mutator(cmd, seed_asm, i)?
        } else if mutator == "nop-slide" {
            mutate_nop_slide(seed_asm, i)
        } else {
            return Err(SearchError::Mutator(format!(
                "unknown mutator `{mutator}` (use nop-slide or --mutator-command)"
            )));
        };

        let digest = sha256_digest_prefixed(source.as_bytes());
        let rel = format!("search-{i:04}.asm");
        let written = rundir
            .write_staging(&rel, source.as_bytes())
            .map_err(|e| SearchError::Io(e.to_string()))?;

        let (status, notes) = if offline_fixture {
            (
                "mutated".to_owned(),
                vec![
                    "offline fixture — SemASM ingest skipped".into(),
                    format!("staged={}", written.display()),
                ],
            )
        } else {
            (
                "staged".to_owned(),
                vec![
                    format!("staged={}", written.display()),
                    "run `vaa ingest` on staged candidates for SemASM+seal".into(),
                ],
            )
        };

        attempts.push(SearchAttempt {
            index: i,
            source_digest: digest,
            status,
            notes,
        });
    }

    if attempts.is_empty() {
        stopped_reason = "empty".into();
    }

    Ok(SearchReport {
        attempts,
        verified,
        stopped_reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nop_slide_is_deterministic() {
        let a = mutate_nop_slide("mov rax, 1\n", 2);
        let b = mutate_nop_slide("mov rax, 1\n", 2);
        assert_eq!(a, b);
        assert!(a.contains("nop"));
        assert!(a.contains("vaa-search nop-slide x2"));
    }

    #[test]
    fn offline_search_respects_budget_cap() {
        let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/tasks/sum_i64.vaa.toml");
        let locked = crate::task::load_locked_task(&fixture).expect("task");
        let dir = std::env::temp_dir().join(format!(
            "vaa_search_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let report = run_search(
            &locked,
            "xor eax, eax\nret\n",
            &dir,
            3,
            "nop-slide",
            None,
            true,
        )
        .expect("search");
        assert_eq!(report.attempts.len(), 3);
        assert!(!report.verified);
        assert_eq!(report.stopped_reason, "budget_exhausted");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
