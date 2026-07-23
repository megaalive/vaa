//! Same-host reproducibility comparison (PR-021).
//!
//! Compares canonical build fields only — excludes timestamps, absolute paths,
//! PIDs, and run ids (§16.3). Not a cross-host bit-identical claim.

use serde::{Deserialize, Serialize};

use super::pipeline::{BuildOutcome, BuildPipeline, PipelineConfig};
use crate::evidence::sha256_digest_prefixed;
use crate::process::{ProcessConfig, ProcessRunner};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Canonical snapshot used for reproducibility comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CanonicalBuildView {
    pub source_digest: String,
    pub object_digest: Option<String>,
    pub binary_digest: Option<String>,
    pub assembler_digest: Option<String>,
    pub linker_digest: Option<String>,
    pub target: String,
    pub assembler_args: Vec<String>,
    pub linker_args: Vec<String>,
}

/// Result of comparing two builds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReproReport {
    pub matched: bool,
    pub mismatches: Vec<String>,
    pub canonical_a: CanonicalBuildView,
    pub canonical_b: CanonicalBuildView,
}

impl CanonicalBuildView {
    /// Build a path-stripped view from a successful (or partial) build outcome.
    #[must_use]
    pub fn from_outcome(outcome: &BuildOutcome, source_bytes: &[u8], target: &str) -> Self {
        let object_digest = std::fs::read(&outcome.manifest.object_path)
            .ok()
            .map(|b| sha256_digest_prefixed(&b));
        let binary_digest = std::fs::read(&outcome.manifest.binary_path)
            .ok()
            .map(|b| sha256_digest_prefixed(&b));
        Self {
            source_digest: sha256_digest_prefixed(source_bytes),
            object_digest,
            binary_digest,
            assembler_digest: outcome.manifest.assembler_digest.clone(),
            linker_digest: outcome.manifest.linker_digest.clone(),
            target: target.to_owned(),
            // Drop absolute path args for canonical compare: keep flags only-ish
            // by stripping entries that look like filesystem paths.
            assembler_args: strip_path_args(&outcome.manifest.assembler_args),
            linker_args: strip_path_args(&outcome.manifest.linker_args),
        }
    }
}

fn strip_path_args(args: &[String]) -> Vec<String> {
    args.iter()
        .filter(|a| {
            let p = Path::new(a.as_str());
            if p.is_absolute() || a.contains('/') || a.contains('\\') {
                return false;
            }
            !p.extension().is_some_and(|ext| {
                ext.eq_ignore_ascii_case("asm")
                    || ext.eq_ignore_ascii_case("o")
                    || ext.eq_ignore_ascii_case("obj")
                    || ext.eq_ignore_ascii_case("bin")
                    || ext.eq_ignore_ascii_case("exe")
            })
        })
        .cloned()
        .collect()
}

/// Compare two canonical views; collect field mismatches.
#[must_use]
pub fn compare_canonical(a: &CanonicalBuildView, b: &CanonicalBuildView) -> ReproReport {
    let mut mismatches = Vec::new();
    if a.source_digest != b.source_digest {
        mismatches.push(format!(
            "source_digest: {} vs {}",
            a.source_digest, b.source_digest
        ));
    }
    if a.object_digest != b.object_digest {
        mismatches.push(format!(
            "object_digest: {:?} vs {:?}",
            a.object_digest, b.object_digest
        ));
    }
    if a.binary_digest != b.binary_digest {
        mismatches.push(format!(
            "binary_digest: {:?} vs {:?}",
            a.binary_digest, b.binary_digest
        ));
    }
    if a.assembler_digest != b.assembler_digest {
        mismatches.push(format!(
            "assembler_digest: {:?} vs {:?}",
            a.assembler_digest, b.assembler_digest
        ));
    }
    if a.linker_digest != b.linker_digest {
        mismatches.push(format!(
            "linker_digest: {:?} vs {:?}",
            a.linker_digest, b.linker_digest
        ));
    }
    if a.target != b.target {
        mismatches.push(format!("target: {} vs {}", a.target, b.target));
    }
    if a.assembler_args != b.assembler_args {
        mismatches.push(format!(
            "assembler_args: {:?} vs {:?}",
            a.assembler_args, b.assembler_args
        ));
    }
    if a.linker_args != b.linker_args {
        mismatches.push(format!(
            "linker_args: {:?} vs {:?}",
            a.linker_args, b.linker_args
        ));
    }
    ReproReport {
        matched: mismatches.is_empty(),
        mismatches,
        canonical_a: a.clone(),
        canonical_b: b.clone(),
    }
}

/// Build twice into sibling temp dirs and compare canonical digests.
pub fn check_reproducible(config: &PipelineConfig) -> Result<ReproReport, String> {
    let source_bytes =
        std::fs::read(&config.source_path).map_err(|e| format!("read source: {e}"))?;
    let base = std::env::temp_dir().join(format!(
        "vaa_repro_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let dir_a = base.join("a");
    let dir_b = base.join("b");
    std::fs::create_dir_all(&dir_a).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir_b).map_err(|e| e.to_string())?;

    let mut cfg_a = config.clone();
    cfg_a.output_dir = dir_a;
    let mut cfg_b = config.clone();
    cfg_b.output_dir = dir_b;

    let out_a = BuildPipeline::build(&cfg_a);
    let out_b = BuildPipeline::build(&cfg_b);
    if !out_a.success {
        let _ = std::fs::remove_dir_all(&base);
        return Err(format!(
            "first build failed: {}",
            out_a.assembler_stderr.trim()
        ));
    }
    if !out_b.success {
        let _ = std::fs::remove_dir_all(&base);
        return Err(format!(
            "second build failed: {}",
            out_b.assembler_stderr.trim()
        ));
    }

    let view_a = CanonicalBuildView::from_outcome(&out_a, &source_bytes, &config.target);
    let view_b = CanonicalBuildView::from_outcome(&out_b, &source_bytes, &config.target);
    let report = compare_canonical(&view_a, &view_b);
    let _ = std::fs::remove_dir_all(&base);
    Ok(report)
}

/// Digests of object bytes for evidence when `require_reproducible_build` is set.
///
/// Assembles twice (object only) when NASM is available; returns `(matched, details)`.
/// Same-host claim only — not cross-CI bit-identical.
#[must_use]
pub fn reproducible_build_check(source_path: &Path, target: &str) -> (bool, String) {
    let tmp = std::env::temp_dir().join(format!(
        "vaa_repro_ev_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let dir_a = tmp.join("a");
    let dir_b = tmp.join("b");
    if std::fs::create_dir_all(&dir_a).is_err() || std::fs::create_dir_all(&dir_b).is_err() {
        return (false, "repro temp dirs failed".into());
    }
    let fmt = nasm_format(target);
    let obj_a = dir_a.join("c.o");
    let obj_b = dir_b.join("c.o");
    let run = |out: &Path| {
        let cfg = ProcessConfig {
            program: PathBuf::from("nasm"),
            args: vec![
                "-f".to_owned(),
                fmt.to_owned(),
                "-o".to_owned(),
                out.to_string_lossy().to_string(),
                source_path.to_string_lossy().to_string(),
            ],
            timeout: Duration::from_secs(60),
            max_output_bytes: 1_048_576,
            ..ProcessConfig::default()
        };
        ProcessRunner::run(&cfg)
    };
    let a = run(&obj_a);
    let b = run(&obj_b);
    let result = match (a, b) {
        (Ok(oa), Ok(ob)) if oa.exit_code == Some(0) && ob.exit_code == Some(0) => {
            match (std::fs::read(&obj_a), std::fs::read(&obj_b)) {
                (Ok(ba), Ok(bb)) => {
                    let da = sha256_digest_prefixed(&ba);
                    let db = sha256_digest_prefixed(&bb);
                    if da == db {
                        (true, format!("twin assemble matched ({da})"))
                    } else {
                        (false, format!("object_digest: {da} vs {db}"))
                    }
                }
                _ => (false, "could not read twin objects".into()),
            }
        }
        (Ok(oa), _) if oa.exit_code != Some(0) => {
            (false, format!("first nasm failed: {}", oa.stderr.trim()))
        }
        (_, Ok(ob)) if ob.exit_code != Some(0) => {
            (false, format!("second nasm failed: {}", ob.stderr.trim()))
        }
        (Err(e), _) | (_, Err(e)) => (false, format!("nasm invoke failed: {e}")),
        _ => (false, "twin assemble failed".into()),
    };
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

fn nasm_format(target: &str) -> &str {
    if target.contains("windows") {
        "win64"
    } else {
        "elf64"
    }
}

/// Helper for unit tests: synthesize views without invoking NASM.
#[cfg(test)]
pub fn view_for_test(
    source: &str,
    object: Option<&[u8]>,
    binary: Option<&[u8]>,
    as_digest: &str,
) -> CanonicalBuildView {
    CanonicalBuildView {
        source_digest: sha256_digest_prefixed(source.as_bytes()),
        object_digest: object.map(sha256_digest_prefixed),
        binary_digest: binary.map(sha256_digest_prefixed),
        assembler_digest: Some(as_digest.to_owned()),
        linker_digest: Some("sha256:ld".into()),
        target: "elf64".into(),
        assembler_args: vec!["-f".into(), "elf64".into()],
        linker_args: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_views_match() {
        let a = view_for_test("src", Some(b"obj"), Some(b"bin"), "sha256:as");
        let b = a.clone();
        let r = compare_canonical(&a, &b);
        assert!(r.matched);
        assert!(r.mismatches.is_empty());
    }

    #[test]
    fn object_mismatch_is_reported() {
        let a = view_for_test("src", Some(b"obj1"), Some(b"bin"), "sha256:as");
        let b = view_for_test("src", Some(b"obj2"), Some(b"bin"), "sha256:as");
        let r = compare_canonical(&a, &b);
        assert!(!r.matched);
        assert!(r.mismatches.iter().any(|m| m.contains("object_digest")));
    }

    #[test]
    fn strip_path_args_drops_paths() {
        let args = vec![
            "-f".into(),
            "elf64".into(),
            "-o".into(),
            "/tmp/out.o".into(),
            "foo.asm".into(),
        ];
        let stripped = strip_path_args(&args);
        assert_eq!(
            stripped,
            vec!["-f".to_owned(), "elf64".to_owned(), "-o".to_owned()]
        );
    }
}
