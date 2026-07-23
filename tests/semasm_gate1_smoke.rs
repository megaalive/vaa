//! Live Gate-1 smoke: SemASM Incomplete + ingest seal chain (ignored without toolchain).

use std::path::{Path, PathBuf};
use std::process::Command;

fn vaa_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_vaa"))
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn signing_key_configured() -> bool {
    std::env::var_os("VAA_SEAL_SIGNING_KEY").is_some_and(|v| !v.is_empty())
}

/// When CI sets `VAA_SEAL_SIGNING_KEY`, seals must carry Ed25519 authenticity.
fn assert_seal_signature_if_signing(run_dir: &Path) {
    if !signing_key_configured() {
        return;
    }
    let seal_path = run_dir.join("evidence").join("final.seal.json");
    let raw = std::fs::read_to_string(&seal_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", seal_path.display()));
    let value: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse seal: {e}"));
    let sig = value
        .get("signature")
        .unwrap_or_else(|| panic!("expected signature on {}", seal_path.display()));
    assert_eq!(
        sig.get("alg").and_then(|v| v.as_str()),
        Some("ed25519"),
        "signature.alg: {sig}"
    );
    assert_eq!(
        sig.get("signed_over").and_then(|v| v.as_str()),
        Some("acceptance_digest"),
        "signature.signed_over: {sig}"
    );
    assert!(
        sig.get("public_key_b64")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty()),
        "signature.public_key_b64 missing"
    );
    assert!(
        sig.get("sig_b64")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty()),
        "signature.sig_b64 missing"
    );
}

#[test]
fn hlax64_sum_i64_candidate_is_framed_win64() {
    let src = include_str!("../fixtures/ingest/hlax64_sum_i64/candidate.asm");
    assert!(
        src.contains("push rbp"),
        "HlaX64 framed leaf must open a frame"
    );
    assert!(
        src.contains("mov rsp, rbp") || src.contains("mov rsp,rbp"),
        "HlaX64 framed leaf must restore rsp via rbp (SemASM T1 carve-out)"
    );
    assert!(
        src.contains("[rbp-"),
        "HlaX64 framed leaf must spill args to [rbp-disp]"
    );
}

#[test]
fn hlax64_find_last_byte_candidate_is_framed_win64() {
    let src = include_str!("../fixtures/ingest/hlax64_find_last_byte/candidate.asm");
    assert!(
        src.contains("global find_last_byte"),
        "HlaX64 find_last_byte leaf must export the SemASM symbol"
    );
    assert!(
        src.contains("push rbp"),
        "HlaX64 framed leaf must open a frame"
    );
    assert!(
        src.contains("mov rsp, rbp") || src.contains("mov rsp,rbp"),
        "HlaX64 framed leaf must restore rsp via rbp (SemASM T1 carve-out)"
    );
    assert!(
        src.contains("[rbp-"),
        "HlaX64 framed leaf must spill args to [rbp-disp]"
    );
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_verify_count_byte_win64_incomplete() {
    let task = root().join("fixtures/semasm/count_byte/count_byte.vaa.toml");
    let source = root().join("fixtures/semasm/count_byte/count_byte_win64.asm");
    let contract = root().join("fixtures/semasm/count_byte/count_byte.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected evidence JSON ({e}): stdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    });

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping: SemASM unavailable\n{value}");
        return;
    }

    assert_eq!(
        value["final_status"], "Incomplete",
        "Gate-1 expects Incomplete: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "execution_denied");
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires `semasm` on PATH and a Linux assemble/link toolchain"]
fn gate1_verify_count_byte_linux_incomplete() {
    let task = root().join("fixtures/semasm/count_byte/count_byte_linux.vaa.toml");
    let source = root().join("fixtures/semasm/count_byte/count_byte_linux.asm");
    let contract = root().join("fixtures/semasm/count_byte/count_byte.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected evidence JSON ({e}): stdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    });

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping: SemASM unavailable\n{value}");
        return;
    }

    assert_eq!(
        value["final_status"], "Incomplete",
        "Gate-1 Linux expects Incomplete: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "execution_denied");
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires `semasm` on PATH and a Linux assemble/link toolchain"]
fn gate1_verify_sum_i64_linux_incomplete() {
    let task = root().join("fixtures/semasm/sum_i64/sum_i64_linux.vaa.toml");
    let source = root().join("fixtures/semasm/sum_i64/sum_i64_linux.asm");
    let contract = root().join("fixtures/semasm/sum_i64/sum_i64.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected evidence JSON ({e}): stdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    });

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping: SemASM unavailable\n{value}");
        return;
    }

    assert_eq!(
        value["final_status"], "Incomplete",
        "Gate-1 Linux sum_i64 expects Incomplete: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "execution_denied");
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_verify_sum_i64_win64_incomplete() {
    let task = root().join("fixtures/semasm/sum_i64/sum_i64.vaa.toml");
    let source = root().join("fixtures/semasm/sum_i64/sum_i64_win64.asm");
    let contract = root().join("fixtures/semasm/sum_i64/sum_i64.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected evidence JSON ({e}): stdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    });

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping: SemASM unavailable\n{value}");
        return;
    }

    assert_eq!(
        value["final_status"], "Incomplete",
        "Gate-1 sum_i64 expects Incomplete: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "execution_denied");
    let raw_json = value["verify_report"]["raw_json"]
        .as_str()
        .expect("verify_report.raw_json");
    let raw: serde_json::Value = serde_json::from_str(raw_json).expect("raw_json parse");
    assert_eq!(
        raw["behavior_oracle"]["id"],
        "builtin.buffer.wrapping_sum_i64"
    );
    assert_eq!(raw["behavior_oracle"]["version"], 2);
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires `semasm` on PATH and a Linux assemble/link toolchain"]
fn gate1_verify_min_usize_linux_incomplete() {
    let task = root().join("fixtures/semasm/min_usize/min_usize_linux.vaa.toml");
    let source = root().join("fixtures/semasm/min_usize/min_usize_linux.asm");
    let contract = root().join("fixtures/semasm/min_usize/min_usize.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected evidence JSON ({e}): stdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    });

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping: SemASM unavailable\n{value}");
        return;
    }

    assert_eq!(
        value["final_status"], "Incomplete",
        "Gate-1 Linux min_usize expects Incomplete: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "execution_denied");
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_verify_min_usize_win64_incomplete() {
    let task = root().join("fixtures/semasm/min_usize/min_usize.vaa.toml");
    let source = root().join("fixtures/semasm/min_usize/min_usize_win64.asm");
    let contract = root().join("fixtures/semasm/min_usize/min_usize.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected evidence JSON ({e}): stdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    });

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping: SemASM unavailable\n{value}");
        return;
    }

    assert_eq!(
        value["final_status"], "Incomplete",
        "Gate-1 min_usize expects Incomplete: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "execution_denied");
    let raw_json = value["verify_report"]["raw_json"]
        .as_str()
        .expect("verify_report.raw_json");
    let raw: serde_json::Value = serde_json::from_str(raw_json).expect("raw_json parse");
    assert_eq!(
        raw["behavior_oracle"]["id"],
        "builtin.pure_int.binary_usize"
    );
    assert_eq!(raw["behavior_oracle"]["version"], 2);
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires `semasm` on PATH and a Linux assemble/link toolchain"]
fn gate1_verify_max_usize_linux_incomplete() {
    let task = root().join("fixtures/semasm/max_usize/max_usize_linux.vaa.toml");
    let source = root().join("fixtures/semasm/max_usize/max_usize_linux.asm");
    let contract = root().join("fixtures/semasm/max_usize/max_usize.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected evidence JSON ({e}): stdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    });

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping: SemASM unavailable\n{value}");
        return;
    }

    assert_eq!(
        value["final_status"], "Incomplete",
        "Gate-1 Linux max_usize expects Incomplete: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "execution_denied");
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_verify_max_usize_win64_incomplete() {
    let task = root().join("fixtures/semasm/max_usize/max_usize.vaa.toml");
    let source = root().join("fixtures/semasm/max_usize/max_usize_win64.asm");
    let contract = root().join("fixtures/semasm/max_usize/max_usize.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected evidence JSON ({e}): stdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    });

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping: SemASM unavailable\n{value}");
        return;
    }

    assert_eq!(
        value["final_status"], "Incomplete",
        "Gate-1 max_usize expects Incomplete: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "execution_denied");
    let raw_json = value["verify_report"]["raw_json"]
        .as_str()
        .expect("verify_report.raw_json");
    let raw: serde_json::Value = serde_json::from_str(raw_json).expect("raw_json parse");
    assert_eq!(
        raw["behavior_oracle"]["id"],
        "builtin.pure_int.binary_usize"
    );
    assert_eq!(raw["behavior_oracle"]["version"], 2);
    assert!(
        raw["behavior_oracle"]["claim"]
            .as_str()
            .unwrap_or("")
            .contains("max(a, b)"),
        "max_usize claim must name max: {}",
        raw["behavior_oracle"]["claim"]
    );
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires `semasm` on PATH and a Linux assemble/link toolchain"]
fn gate1_verify_find_first_byte_linux_incomplete() {
    let task = root().join("fixtures/semasm/find_first_byte/find_first_byte_linux.vaa.toml");
    let source = root().join("fixtures/semasm/find_first_byte/find_first_byte_linux.asm");
    let contract = root().join("fixtures/semasm/find_first_byte/find_first_byte.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected evidence JSON ({e}): stdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    });

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping: SemASM unavailable\n{value}");
        return;
    }

    assert_eq!(
        value["final_status"], "Incomplete",
        "Gate-1 Linux find_first_byte expects Incomplete: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "execution_denied");
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_verify_find_first_byte_win64_incomplete() {
    let task = root().join("fixtures/semasm/find_first_byte/find_first_byte.vaa.toml");
    let source = root().join("fixtures/semasm/find_first_byte/find_first_byte_win64.asm");
    let contract = root().join("fixtures/semasm/find_first_byte/find_first_byte.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected evidence JSON ({e}): stdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    });

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping: SemASM unavailable\n{value}");
        return;
    }

    assert_eq!(
        value["final_status"], "Incomplete",
        "Gate-1 find_first_byte expects Incomplete: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "execution_denied");
    let raw_json = value["verify_report"]["raw_json"]
        .as_str()
        .expect("verify_report.raw_json");
    let raw: serde_json::Value = serde_json::from_str(raw_json).expect("raw_json parse");
    assert_eq!(raw["behavior_oracle"]["id"], "builtin.buffer.find_first_u8");
    assert_eq!(raw["behavior_oracle"]["version"], 1);
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires `semasm` on PATH and a Linux assemble/link toolchain"]
fn gate1_verify_find_last_byte_linux_incomplete() {
    let task = root().join("fixtures/semasm/find_last_byte/find_last_byte_linux.vaa.toml");
    let source = root().join("fixtures/semasm/find_last_byte/find_last_byte_linux.asm");
    let contract = root().join("fixtures/semasm/find_last_byte/find_last_byte.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected evidence JSON ({e}): stdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    });

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping: SemASM unavailable\n{value}");
        return;
    }

    assert_eq!(
        value["final_status"], "Incomplete",
        "Gate-1 Linux find_last_byte expects Incomplete: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "execution_denied");
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_verify_find_last_byte_win64_incomplete() {
    let task = root().join("fixtures/semasm/find_last_byte/find_last_byte.vaa.toml");
    let source = root().join("fixtures/semasm/find_last_byte/find_last_byte_win64.asm");
    let contract = root().join("fixtures/semasm/find_last_byte/find_last_byte.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected evidence JSON ({e}): stdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    });

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping: SemASM unavailable\n{value}");
        return;
    }

    assert_eq!(
        value["final_status"], "Incomplete",
        "Gate-1 find_last_byte expects Incomplete: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "execution_denied");
    let raw_json = value["verify_report"]["raw_json"]
        .as_str()
        .expect("verify_report.raw_json");
    let raw: serde_json::Value = serde_json::from_str(raw_json).expect("raw_json parse");
    assert_eq!(raw["behavior_oracle"]["id"], "builtin.buffer.find_last_u8");
    assert_eq!(raw["behavior_oracle"]["version"], 1);
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires `semasm` on PATH and a Linux assemble/link toolchain"]
fn gate1_verify_memcmp_linux_incomplete() {
    let task = root().join("fixtures/semasm/memcmp/memcmp_linux.vaa.toml");
    let source = root().join("fixtures/semasm/memcmp/memcmp_linux.asm");
    let contract = root().join("fixtures/semasm/memcmp/memcmp.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected evidence JSON ({e}): stdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    });

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping: SemASM unavailable\n{value}");
        return;
    }

    assert_eq!(
        value["final_status"], "Incomplete",
        "Gate-1 Linux memcmp expects Incomplete: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "execution_denied");
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_verify_memcmp_win64_incomplete() {
    let task = root().join("fixtures/semasm/memcmp/memcmp.vaa.toml");
    let source = root().join("fixtures/semasm/memcmp/memcmp_win64.asm");
    let contract = root().join("fixtures/semasm/memcmp/memcmp.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected evidence JSON ({e}): stdout={stdout}\nstderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    });

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping: SemASM unavailable\n{value}");
        return;
    }

    assert_eq!(
        value["final_status"], "Incomplete",
        "Gate-1 memcmp expects Incomplete: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "execution_denied");
    let raw_json = value["verify_report"]["raw_json"]
        .as_str()
        .expect("verify_report.raw_json");
    let raw: serde_json::Value = serde_json::from_str(raw_json).expect("raw_json parse");
    assert_eq!(raw["behavior_oracle"]["id"], "builtin.buffer.memcmp_i8");
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_ingest_and_verify_chain() {
    let task = root().join("fixtures/ingest/count_byte/count_byte.vaa.toml");
    let source = root().join("fixtures/ingest/count_byte/candidate.asm");
    let contract = root().join("fixtures/ingest/count_byte/count_byte.sem.toml");
    let run_base = root().join("target/vaa-gate1-runs");
    let _ = std::fs::remove_dir_all(&run_base);
    std::fs::create_dir_all(&run_base).unwrap();

    let output = Command::new(vaa_bin())
        .args([
            "ingest",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--generator",
            "ci-gate1",
            "--run-dir",
            run_base.to_str().unwrap(),
            "--format",
            "terminal",
        ])
        .output()
        .expect("run vaa ingest");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.contains("semasm unavailable")
        || stderr.contains("semasm unavailable")
        || (stdout.contains("SemASM") && stdout.contains("not found"))
    {
        eprintln!("skipping: SemASM unavailable\nstdout={stdout}\nstderr={stderr}");
        return;
    }

    assert!(
        output.status.success() || stdout.contains("Incomplete") || stdout.contains("final_status"),
        "ingest failed: status={:?}\nstdout={stdout}\nstderr={stderr}",
        output.status
    );

    // Find the run directory created under run_base.
    let run_dir = std::fs::read_dir(&run_base)
        .expect("read run base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("expected a run directory after ingest");

    let chain = Command::new(vaa_bin())
        .args(["evidence", "verify-chain", run_dir.to_str().unwrap()])
        .output()
        .expect("verify-chain");
    let chain_out = String::from_utf8_lossy(&chain.stdout);
    let chain_err = String::from_utf8_lossy(&chain.stderr);
    assert!(
        chain.status.success(),
        "verify-chain failed: stdout={chain_out}\nstderr={chain_err}"
    );
    assert!(
        chain_out.contains("seal chain verified") || chain_out.contains("ok:"),
        "unexpected chain output: {chain_out}"
    );
    assert_seal_signature_if_signing(&run_dir);
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires `semasm` on PATH and a Linux assemble/link toolchain"]
fn gate1_ingest_count_byte_linux_verify_chain() {
    let task = root().join("fixtures/ingest/count_byte_linux/count_byte.vaa.toml");
    let source = root().join("fixtures/ingest/count_byte_linux/candidate.asm");
    let contract = root().join("fixtures/ingest/count_byte_linux/count_byte.sem.toml");
    let run_base = root().join("target/vaa-gate1-linux-runs");
    let _ = std::fs::remove_dir_all(&run_base);
    std::fs::create_dir_all(&run_base).unwrap();

    let output = Command::new(vaa_bin())
        .args([
            "ingest",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--generator",
            "ci-gate1-linux",
            "--run-dir",
            run_base.to_str().unwrap(),
            "--format",
            "terminal",
        ])
        .output()
        .expect("run vaa ingest linux");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.contains("semasm unavailable")
        || stderr.contains("semasm unavailable")
        || (stdout.contains("SemASM") && stdout.contains("not found"))
    {
        eprintln!("skipping: SemASM unavailable\nstdout={stdout}\nstderr={stderr}");
        return;
    }

    assert!(
        output.status.success() || stdout.contains("Incomplete") || stdout.contains("final_status"),
        "linux ingest failed: status={:?}\nstdout={stdout}\nstderr={stderr}",
        output.status
    );

    let run_dir = std::fs::read_dir(&run_base)
        .expect("read run base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("expected a run directory after linux ingest");

    let chain = Command::new(vaa_bin())
        .args(["evidence", "verify-chain", run_dir.to_str().unwrap()])
        .output()
        .expect("verify-chain");
    let chain_out = String::from_utf8_lossy(&chain.stdout);
    let chain_err = String::from_utf8_lossy(&chain.stderr);
    assert!(
        chain.status.success(),
        "verify-chain failed: stdout={chain_out}\nstderr={chain_err}"
    );
    assert!(
        chain_out.contains("seal chain verified") || chain_out.contains("ok:"),
        "unexpected chain output: {chain_out}"
    );
    assert_seal_signature_if_signing(&run_dir);
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_run_count_byte_multi_candidate_verify_chain() {
    let task = root().join("fixtures/run/count_byte/count_byte.vaa.toml");
    let contract = root().join("fixtures/run/count_byte/count_byte.sem.toml");
    let wrong = root().join("fixtures/run/count_byte/01_wrong.asm");
    let repaired = root().join("fixtures/run/count_byte/02_repaired.asm");
    let run_base = root().join("target/vaa-gate1-run-smokes");
    let _ = std::fs::remove_dir_all(&run_base);
    std::fs::create_dir_all(&run_base).unwrap();

    let output = Command::new(vaa_bin())
        .args([
            "run",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--wrong",
            wrong.to_str().unwrap(),
            "--repaired",
            repaired.to_str().unwrap(),
            "--run-dir",
            run_base.to_str().unwrap(),
            "--format",
            "terminal",
        ])
        .output()
        .expect("run vaa run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.contains("semasm unavailable")
        || stderr.contains("semasm unavailable")
        || (stdout.contains("SemASM") && stdout.contains("not found"))
    {
        eprintln!("skipping: SemASM unavailable\nstdout={stdout}\nstderr={stderr}");
        return;
    }

    assert!(
        output.status.success()
            || stdout.contains("Incomplete")
            || stdout.contains("Candidates accepted")
            || stdout.contains("final_status"),
        "vaa run failed: status={:?}\nstdout={stdout}\nstderr={stderr}",
        output.status
    );

    let run_dir = std::fs::read_dir(&run_base)
        .expect("read run base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("expected a run directory after vaa run");

    let cand0 = run_dir.join("candidates").join("0000");
    let cand1 = run_dir.join("candidates").join("0001");
    assert!(
        cand0.is_dir() && cand1.is_dir(),
        "R7 expects ≥2 sealed candidates under {}",
        run_dir.display()
    );

    let seal_log = run_dir.join("evidence").join("seal-log.jsonl");
    assert!(
        seal_log.is_file(),
        "seal-log.jsonl missing under {}",
        run_dir.display()
    );
    let seal_body = std::fs::read_to_string(&seal_log).expect("read seal-log");
    assert!(
        seal_body.lines().filter(|l| !l.trim().is_empty()).count() >= 2,
        "seal-log should have ≥2 entries: {seal_body}"
    );

    let events = run_dir.join("events.jsonl");
    assert!(
        events.is_file()
            && !std::fs::read_to_string(&events)
                .unwrap_or_default()
                .is_empty(),
        "events.jsonl should be non-empty"
    );

    let chain = Command::new(vaa_bin())
        .args(["evidence", "verify-chain", run_dir.to_str().unwrap()])
        .output()
        .expect("verify-chain");
    let chain_out = String::from_utf8_lossy(&chain.stdout);
    let chain_err = String::from_utf8_lossy(&chain.stderr);
    assert!(
        chain.status.success(),
        "verify-chain failed: stdout={chain_out}\nstderr={chain_err}"
    );
    assert!(
        chain_out.contains("seal chain verified") || chain_out.contains("ok:"),
        "unexpected chain output: {chain_out}"
    );
    assert_seal_signature_if_signing(&run_dir);
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_run_find_first_byte_multi_candidate_verify_chain() {
    let task = root().join("fixtures/run/find_first_byte/find_first_byte.vaa.toml");
    let contract = root().join("fixtures/run/find_first_byte/find_first_byte.sem.toml");
    let wrong = root().join("fixtures/run/find_first_byte/01_wrong.asm");
    let repaired = root().join("fixtures/run/find_first_byte/02_repaired.asm");
    let run_base = root().join("target/vaa-gate1-run-ffb-smokes");
    let _ = std::fs::remove_dir_all(&run_base);
    std::fs::create_dir_all(&run_base).unwrap();

    let output = Command::new(vaa_bin())
        .args([
            "run",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--wrong",
            wrong.to_str().unwrap(),
            "--repaired",
            repaired.to_str().unwrap(),
            "--run-dir",
            run_base.to_str().unwrap(),
            "--format",
            "terminal",
        ])
        .output()
        .expect("run vaa run find_first_byte");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.contains("semasm unavailable")
        || stderr.contains("semasm unavailable")
        || (stdout.contains("SemASM") && stdout.contains("not found"))
    {
        eprintln!("skipping: SemASM unavailable\nstdout={stdout}\nstderr={stderr}");
        return;
    }

    assert!(
        output.status.success()
            || stdout.contains("Incomplete")
            || stdout.contains("Candidates accepted")
            || stdout.contains("final_status"),
        "vaa run find_first_byte failed: status={:?}\nstdout={stdout}\nstderr={stderr}",
        output.status
    );

    let run_dir = std::fs::read_dir(&run_base)
        .expect("read run base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("expected a run directory after vaa run find_first_byte");

    let cand0 = run_dir.join("candidates").join("0000");
    let cand1 = run_dir.join("candidates").join("0001");
    assert!(
        cand0.is_dir() && cand1.is_dir(),
        "Q1 expects ≥2 sealed candidates under {}",
        run_dir.display()
    );

    let seal_log = run_dir.join("evidence").join("seal-log.jsonl");
    assert!(
        seal_log.is_file(),
        "seal-log.jsonl missing under {}",
        run_dir.display()
    );
    let seal_body = std::fs::read_to_string(&seal_log).expect("read seal-log");
    assert!(
        seal_body.lines().filter(|l| !l.trim().is_empty()).count() >= 2,
        "seal-log should have ≥2 entries: {seal_body}"
    );

    let events = run_dir.join("events.jsonl");
    assert!(
        events.is_file()
            && !std::fs::read_to_string(&events)
                .unwrap_or_default()
                .is_empty(),
        "events.jsonl should be non-empty"
    );

    let chain = Command::new(vaa_bin())
        .args(["evidence", "verify-chain", run_dir.to_str().unwrap()])
        .output()
        .expect("verify-chain");
    let chain_out = String::from_utf8_lossy(&chain.stdout);
    let chain_err = String::from_utf8_lossy(&chain.stderr);
    assert!(
        chain.status.success(),
        "verify-chain failed: stdout={chain_out}\nstderr={chain_err}"
    );
    assert!(
        chain_out.contains("seal chain verified") || chain_out.contains("ok:"),
        "unexpected chain output: {chain_out}"
    );
    assert_seal_signature_if_signing(&run_dir);
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_run_find_last_byte_multi_candidate_verify_chain() {
    let task = root().join("fixtures/run/find_last_byte/find_last_byte.vaa.toml");
    let contract = root().join("fixtures/run/find_last_byte/find_last_byte.sem.toml");
    let wrong = root().join("fixtures/run/find_last_byte/01_wrong.asm");
    let repaired = root().join("fixtures/run/find_last_byte/02_repaired.asm");
    let run_base = root().join("target/vaa-gate1-run-flb-smokes");
    let _ = std::fs::remove_dir_all(&run_base);
    std::fs::create_dir_all(&run_base).unwrap();

    let output = Command::new(vaa_bin())
        .args([
            "run",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--wrong",
            wrong.to_str().unwrap(),
            "--repaired",
            repaired.to_str().unwrap(),
            "--run-dir",
            run_base.to_str().unwrap(),
            "--format",
            "terminal",
        ])
        .output()
        .expect("run vaa run find_last_byte");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.contains("semasm unavailable")
        || stderr.contains("semasm unavailable")
        || (stdout.contains("SemASM") && stdout.contains("not found"))
    {
        eprintln!("skipping: SemASM unavailable\nstdout={stdout}\nstderr={stderr}");
        return;
    }

    assert!(
        output.status.success()
            || stdout.contains("Incomplete")
            || stdout.contains("Candidates accepted")
            || stdout.contains("final_status"),
        "vaa run find_last_byte failed: status={:?}\nstdout={stdout}\nstderr={stderr}",
        output.status
    );

    let run_dir = std::fs::read_dir(&run_base)
        .expect("read run base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("expected a run directory after vaa run find_last_byte");

    let cand0 = run_dir.join("candidates").join("0000");
    let cand1 = run_dir.join("candidates").join("0001");
    assert!(
        cand0.is_dir() && cand1.is_dir(),
        "S2 expects >=2 sealed candidates under {}",
        run_dir.display()
    );

    let seal_log = run_dir.join("evidence").join("seal-log.jsonl");
    assert!(
        seal_log.is_file(),
        "seal-log.jsonl missing under {}",
        run_dir.display()
    );
    let seal_body = std::fs::read_to_string(&seal_log).expect("read seal-log");
    assert!(
        seal_body.lines().filter(|l| !l.trim().is_empty()).count() >= 2,
        "seal-log should have >=2 entries: {seal_body}"
    );

    let events = run_dir.join("events.jsonl");
    assert!(
        events.is_file()
            && !std::fs::read_to_string(&events)
                .unwrap_or_default()
                .is_empty(),
        "events.jsonl should be non-empty"
    );

    let chain = Command::new(vaa_bin())
        .args(["evidence", "verify-chain", run_dir.to_str().unwrap()])
        .output()
        .expect("verify-chain");
    let chain_out = String::from_utf8_lossy(&chain.stdout);
    let chain_err = String::from_utf8_lossy(&chain.stderr);
    assert!(
        chain.status.success(),
        "verify-chain failed: stdout={chain_out}\nstderr={chain_err}"
    );
    assert!(
        chain_out.contains("seal chain verified") || chain_out.contains("ok:"),
        "unexpected chain output: {chain_out}"
    );
    assert_seal_signature_if_signing(&run_dir);
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_run_memcmp_multi_candidate_verify_chain() {
    let task = root().join("fixtures/run/memcmp/memcmp.vaa.toml");
    let contract = root().join("fixtures/run/memcmp/memcmp.sem.toml");
    let wrong = root().join("fixtures/run/memcmp/01_wrong.asm");
    let repaired = root().join("fixtures/run/memcmp/02_repaired.asm");
    let run_base = root().join("target/vaa-gate1-run-memcmp-smokes");
    let _ = std::fs::remove_dir_all(&run_base);
    std::fs::create_dir_all(&run_base).unwrap();

    let output = Command::new(vaa_bin())
        .args([
            "run",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--wrong",
            wrong.to_str().unwrap(),
            "--repaired",
            repaired.to_str().unwrap(),
            "--run-dir",
            run_base.to_str().unwrap(),
            "--format",
            "terminal",
        ])
        .output()
        .expect("run vaa run memcmp");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.contains("semasm unavailable")
        || stderr.contains("semasm unavailable")
        || (stdout.contains("SemASM") && stdout.contains("not found"))
    {
        eprintln!("skipping: SemASM unavailable\nstdout={stdout}\nstderr={stderr}");
        return;
    }

    assert!(
        output.status.success()
            || stdout.contains("Incomplete")
            || stdout.contains("Candidates accepted")
            || stdout.contains("final_status"),
        "vaa run memcmp failed: status={:?}\nstdout={stdout}\nstderr={stderr}",
        output.status
    );

    let run_dir = std::fs::read_dir(&run_base)
        .expect("read run base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("expected a run directory after vaa run memcmp");

    let cand0 = run_dir.join("candidates").join("0000");
    let cand1 = run_dir.join("candidates").join("0001");
    assert!(
        cand0.is_dir() && cand1.is_dir(),
        "U expects >=2 sealed candidates under {}",
        run_dir.display()
    );

    let seal_log = run_dir.join("evidence").join("seal-log.jsonl");
    assert!(
        seal_log.is_file(),
        "seal-log.jsonl missing under {}",
        run_dir.display()
    );
    let seal_body = std::fs::read_to_string(&seal_log).expect("read seal-log");
    assert!(
        seal_body.lines().filter(|l| !l.trim().is_empty()).count() >= 2,
        "seal-log should have >=2 entries: {seal_body}"
    );

    let events = run_dir.join("events.jsonl");
    assert!(
        events.is_file()
            && !std::fs::read_to_string(&events)
                .unwrap_or_default()
                .is_empty(),
        "events.jsonl should be non-empty"
    );

    let chain = Command::new(vaa_bin())
        .args(["evidence", "verify-chain", run_dir.to_str().unwrap()])
        .output()
        .expect("verify-chain");
    let chain_out = String::from_utf8_lossy(&chain.stdout);
    let chain_err = String::from_utf8_lossy(&chain.stderr);
    assert!(
        chain.status.success(),
        "verify-chain failed: stdout={chain_out}\nstderr={chain_err}"
    );
    assert!(
        chain_out.contains("seal chain verified") || chain_out.contains("ok:"),
        "unexpected chain output: {chain_out}"
    );
    assert_seal_signature_if_signing(&run_dir);
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
#[allow(clippy::too_many_lines)]
fn gate1_resume_second_candidate_verify_chain() {
    let task = root().join("fixtures/run/count_byte/count_byte.vaa.toml");
    let contract = root().join("fixtures/run/count_byte/count_byte.sem.toml");
    let wrong = root().join("fixtures/run/count_byte/01_wrong.asm");
    let repaired = root().join("fixtures/run/count_byte/02_repaired.asm");
    let run_base = root().join("target/vaa-gate1-resume-smokes");
    let _ = std::fs::remove_dir_all(&run_base);
    std::fs::create_dir_all(&run_base).unwrap();

    let ingest = Command::new(vaa_bin())
        .args([
            "ingest",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--source",
            wrong.to_str().unwrap(),
            "--generator",
            "ci-e1b",
            "--run-dir",
            run_base.to_str().unwrap(),
            "--format",
            "terminal",
        ])
        .output()
        .expect("ingest seed");

    let stdout = String::from_utf8_lossy(&ingest.stdout);
    let stderr = String::from_utf8_lossy(&ingest.stderr);
    if stdout.contains("semasm unavailable")
        || stderr.contains("semasm unavailable")
        || (stdout.contains("SemASM") && stdout.contains("not found"))
    {
        eprintln!("skipping: SemASM unavailable\nstdout={stdout}\nstderr={stderr}");
        return;
    }
    assert!(
        ingest.status.success()
            || stdout.contains("Incomplete")
            || stdout.contains("Violated")
            || stdout.contains("Failed")
            || stdout.contains("final_status"),
        "seed ingest failed: {:?}\n{stdout}\n{stderr}",
        ingest.status
    );

    let run_dir = std::fs::read_dir(&run_base)
        .expect("read run base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("run dir after ingest");

    assert!(
        run_dir.join("candidates/0000/evidence.seal.json").is_file(),
        "seed ingest did not seal candidates/0000; stdout={stdout}\nstderr={stderr}"
    );
    let seal0: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("candidates/0000/evidence.seal.json")).unwrap(),
    )
    .unwrap();
    let d0 = seal0["envelope_digest"].as_str().unwrap().to_owned();

    let resume = Command::new(vaa_bin())
        .args([
            "run",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--wrong",
            wrong.to_str().unwrap(),
            "--repaired",
            repaired.to_str().unwrap(),
            "--resume",
            run_dir.to_str().unwrap(),
            "--format",
            "terminal",
        ])
        .output()
        .expect("vaa run --resume");

    let r_out = String::from_utf8_lossy(&resume.stdout);
    let r_err = String::from_utf8_lossy(&resume.stderr);
    if r_out.contains("semasm unavailable") || r_err.contains("semasm unavailable") {
        eprintln!("skipping resume: SemASM unavailable\n{r_out}\n{r_err}");
        return;
    }
    assert!(
        resume.status.success()
            || r_out.contains("Incomplete")
            || r_out.contains("Candidates accepted"),
        "resume failed: {:?}\n{r_out}\n{r_err}",
        resume.status
    );

    assert!(
        run_dir.join("candidates/0001").is_dir(),
        "expected candidates/0001 after resume"
    );
    let seal1: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("candidates/0001/evidence.seal.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        seal1["provenance"]["previous_seal_digest"].as_str(),
        Some(d0.as_str())
    );

    let chain = Command::new(vaa_bin())
        .args(["evidence", "verify-chain", run_dir.to_str().unwrap()])
        .output()
        .expect("verify-chain");
    assert!(
        chain.status.success(),
        "verify-chain failed: {} {}",
        String::from_utf8_lossy(&chain.stdout),
        String::from_utf8_lossy(&chain.stderr)
    );
    assert_seal_signature_if_signing(&run_dir);
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_ingest_hlax64_sum_i64_verify_chain() {
    let task = root().join("fixtures/ingest/hlax64_sum_i64/sum_i64.vaa.toml");
    let source = root().join("fixtures/ingest/hlax64_sum_i64/candidate.asm");
    let contract = root().join("fixtures/ingest/hlax64_sum_i64/sum_i64.sem.toml");
    let run_base = root().join("target/vaa-gate1-hlax64-runs");
    let _ = std::fs::remove_dir_all(&run_base);
    std::fs::create_dir_all(&run_base).unwrap();

    let output = Command::new(vaa_bin())
        .args([
            "ingest",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--generator",
            "hlax64",
            "--run-dir",
            run_base.to_str().unwrap(),
            "--format",
            "terminal",
        ])
        .output()
        .expect("run vaa ingest hlax64");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.contains("semasm unavailable")
        || stderr.contains("semasm unavailable")
        || (stdout.contains("SemASM") && stdout.contains("not found"))
    {
        eprintln!("skipping: SemASM unavailable\nstdout={stdout}\nstderr={stderr}");
        return;
    }

    assert!(
        output.status.success() || stdout.contains("Incomplete") || stdout.contains("final_status"),
        "hlax64 ingest failed: status={:?}\nstdout={stdout}\nstderr={stderr}",
        output.status
    );

    let run_dir = std::fs::read_dir(&run_base)
        .expect("read run base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("expected a run directory after hlax64 ingest");

    let chain = Command::new(vaa_bin())
        .args(["evidence", "verify-chain", run_dir.to_str().unwrap()])
        .output()
        .expect("verify-chain");
    let chain_out = String::from_utf8_lossy(&chain.stdout);
    let chain_err = String::from_utf8_lossy(&chain.stderr);
    assert!(
        chain.status.success(),
        "verify-chain failed: stdout={chain_out}\nstderr={chain_err}"
    );
    assert!(
        chain_out.contains("seal chain verified") || chain_out.contains("ok:"),
        "unexpected chain output: {chain_out}"
    );
    assert_seal_signature_if_signing(&run_dir);
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_ingest_hlax64_find_last_byte_verify_chain() {
    let task = root().join("fixtures/ingest/hlax64_find_last_byte/find_last_byte.vaa.toml");
    let source = root().join("fixtures/ingest/hlax64_find_last_byte/candidate.asm");
    let contract = root().join("fixtures/ingest/hlax64_find_last_byte/find_last_byte.sem.toml");
    let run_base = root().join("target/vaa-gate1-hlax64-find-last-runs");
    let _ = std::fs::remove_dir_all(&run_base);
    std::fs::create_dir_all(&run_base).unwrap();

    let output = Command::new(vaa_bin())
        .args([
            "ingest",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--generator",
            "hlax64",
            "--run-dir",
            run_base.to_str().unwrap(),
            "--format",
            "terminal",
        ])
        .output()
        .expect("run vaa ingest hlax64 find_last_byte");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.contains("semasm unavailable")
        || stderr.contains("semasm unavailable")
        || (stdout.contains("SemASM") && stdout.contains("not found"))
    {
        eprintln!("skipping: SemASM unavailable\nstdout={stdout}\nstderr={stderr}");
        return;
    }

    assert!(
        output.status.success() || stdout.contains("Incomplete") || stdout.contains("final_status"),
        "hlax64 find_last_byte ingest failed: status={:?}\nstdout={stdout}\nstderr={stderr}",
        output.status
    );

    let run_dir = std::fs::read_dir(&run_base)
        .expect("read run base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("expected a run directory after hlax64 find_last_byte ingest");

    let chain = Command::new(vaa_bin())
        .args(["evidence", "verify-chain", run_dir.to_str().unwrap()])
        .output()
        .expect("verify-chain");
    let chain_out = String::from_utf8_lossy(&chain.stdout);
    let chain_err = String::from_utf8_lossy(&chain.stderr);
    assert!(
        chain.status.success(),
        "verify-chain failed: stdout={chain_out}\nstderr={chain_err}"
    );
    assert!(
        chain_out.contains("seal chain verified") || chain_out.contains("ok:"),
        "unexpected chain output: {chain_out}"
    );
    assert_seal_signature_if_signing(&run_dir);
}

#[test]
#[ignore = "Gate smoke: offline vaa search staging (no SemASM required)"]
fn gate1_search_nop_slide_stages() {
    let task = root().join("fixtures/run/find_first_byte/find_first_byte.vaa.toml");
    let seed = root().join("fixtures/run/find_first_byte/02_repaired.asm");
    let run_base = root().join("target/vaa-gate1-search-smokes");
    let _ = std::fs::remove_dir_all(&run_base);
    std::fs::create_dir_all(&run_base).unwrap();

    let output = Command::new(vaa_bin())
        .args([
            "search",
            task.to_str().unwrap(),
            seed.to_str().unwrap(),
            "--run-dir",
            run_base.to_str().unwrap(),
            "--budget",
            "3",
            "--mutator",
            "nop-slide",
        ])
        .output()
        .expect("run vaa search");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "vaa search failed: status={:?}\nstdout={stdout}\nstderr={stderr}",
        output.status
    );
    assert!(
        stdout.contains("search: attempts=3") || stdout.contains("attempts=3"),
        "expected 3 search attempts: {stdout}"
    );
    assert!(
        stdout.contains("verified=false"),
        "search staging must not claim Verified: {stdout}"
    );
    assert!(
        stdout.contains("not formal superoptimization") || stdout.contains("CryptOpt-like staging"),
        "expected honesty note: {stdout}"
    );

    let run_dir = std::fs::read_dir(&run_base)
        .expect("read run base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("expected a run directory after vaa search");

    let staging = run_dir.join("staging");
    assert!(
        staging.is_dir(),
        "staging/ missing under {}",
        run_dir.display()
    );
    for i in 0..3u32 {
        let staged = staging.join(format!("search-{i:04}.asm"));
        assert!(
            staged.is_file(),
            "expected staged mutator output {}",
            staged.display()
        );
        let body = std::fs::read_to_string(&staged).expect("read staged");
        assert!(
            body.contains("vaa-search nop-slide"),
            "staged file missing nop-slide marker: {}",
            staged.display()
        );
    }
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
#[allow(clippy::too_many_lines)]
fn gate1_search_then_ingest_staged_verify_chain() {
    let task = root().join("fixtures/run/find_first_byte/find_first_byte.vaa.toml");
    let contract = root().join("fixtures/run/find_first_byte/find_first_byte.sem.toml");
    let seed = root().join("fixtures/run/find_first_byte/02_repaired.asm");
    let search_base = root().join("target/vaa-gate1-search-ingest-search");
    let ingest_base = root().join("target/vaa-gate1-search-ingest-runs");
    let _ = std::fs::remove_dir_all(&search_base);
    let _ = std::fs::remove_dir_all(&ingest_base);
    std::fs::create_dir_all(&search_base).unwrap();
    std::fs::create_dir_all(&ingest_base).unwrap();

    let search = Command::new(vaa_bin())
        .args([
            "search",
            task.to_str().unwrap(),
            seed.to_str().unwrap(),
            "--run-dir",
            search_base.to_str().unwrap(),
            "--budget",
            "1",
            "--mutator",
            "nop-slide",
        ])
        .output()
        .expect("run vaa search");
    let search_out = String::from_utf8_lossy(&search.stdout);
    let search_err = String::from_utf8_lossy(&search.stderr);
    assert!(
        search.status.success(),
        "vaa search failed: {:?}\n{search_out}\n{search_err}",
        search.status
    );
    assert!(
        search_out.contains("verified=false"),
        "search must not claim Verified: {search_out}"
    );

    let search_run = std::fs::read_dir(&search_base)
        .expect("read search base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("expected search run directory");
    // Index 0 only appends the nop-slide comment marker (no trailing NOPs after
    // `ret`, which would fail SemASM decode/control). Still proves staging→ingest.
    let staged = search_run.join("staging").join("search-0000.asm");
    assert!(
        staged.is_file(),
        "expected staged candidate {}",
        staged.display()
    );
    let staged_body = std::fs::read_to_string(&staged).expect("read staged");
    assert!(
        staged_body.contains("vaa-search nop-slide"),
        "staged candidate missing mutator marker"
    );

    let ingest = Command::new(vaa_bin())
        .args([
            "ingest",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--source",
            staged.to_str().unwrap(),
            "--generator",
            "vaa-search-nop-slide",
            "--run-dir",
            ingest_base.to_str().unwrap(),
            "--format",
            "terminal",
        ])
        .output()
        .expect("run vaa ingest staged");

    let stdout = String::from_utf8_lossy(&ingest.stdout);
    let stderr = String::from_utf8_lossy(&ingest.stderr);
    if stdout.contains("semasm unavailable")
        || stderr.contains("semasm unavailable")
        || (stdout.contains("SemASM") && stdout.contains("not found"))
    {
        eprintln!("skipping: SemASM unavailable\nstdout={stdout}\nstderr={stderr}");
        return;
    }

    assert!(
        ingest.status.success() || stdout.contains("Incomplete") || stdout.contains("final_status"),
        "ingest staged failed: {:?}\n{stdout}\n{stderr}",
        ingest.status
    );

    let run_dir = std::fs::read_dir(&ingest_base)
        .expect("read ingest base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("expected ingest run directory");

    let chain = Command::new(vaa_bin())
        .args(["evidence", "verify-chain", run_dir.to_str().unwrap()])
        .output()
        .expect("verify-chain");
    let chain_out = String::from_utf8_lossy(&chain.stdout);
    let chain_err = String::from_utf8_lossy(&chain.stderr);
    assert!(
        chain.status.success(),
        "verify-chain failed: stdout={chain_out}\nstderr={chain_err}"
    );
    assert!(
        chain_out.contains("seal chain verified") || chain_out.contains("ok:"),
        "unexpected chain output: {chain_out}"
    );
    assert_seal_signature_if_signing(&run_dir);
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
#[allow(clippy::too_many_lines)]
fn gate1_search_ingest_nop_before_ret_stops_on_incomplete() {
    let task = root().join("fixtures/run/find_last_byte/find_last_byte.vaa.toml");
    let contract = root().join("fixtures/run/find_last_byte/find_last_byte.sem.toml");
    let seed = root().join("fixtures/run/find_last_byte/02_repaired.asm");
    let run_base = root().join("target/vaa-gate1-search-ingest-incomplete");
    let _ = std::fs::remove_dir_all(&run_base);
    std::fs::create_dir_all(&run_base).unwrap();

    let output = Command::new(vaa_bin())
        .args([
            "search",
            task.to_str().unwrap(),
            seed.to_str().unwrap(),
            "--run-dir",
            run_base.to_str().unwrap(),
            "--budget",
            "3",
            "--mutator",
            "nop-before-ret",
            "--ingest",
            "--contract",
            contract.to_str().unwrap(),
        ])
        .output()
        .expect("run vaa search --ingest");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.contains("semasm unavailable")
        || stderr.contains("semasm unavailable")
        || (stdout.contains("SemASM") && stdout.contains("not found"))
    {
        eprintln!("skipping: SemASM unavailable\nstdout={stdout}\nstderr={stderr}");
        return;
    }

    assert!(
        output.status.success(),
        "search --ingest failed: {:?}\n{stdout}\n{stderr}",
        output.status
    );
    assert!(
        stdout.contains("verified=false"),
        "Incomplete must not claim Verified: {stdout}"
    );
    assert!(
        stdout.contains("reason=incomplete_accepted") || stdout.contains("incomplete_accepted"),
        "expected incomplete_accepted stop: {stdout}"
    );
    assert!(
        stdout.contains("incomplete"),
        "expected incomplete attempt status: {stdout}"
    );
    assert!(
        !stdout.to_lowercase().contains("cryptopt verified"),
        "honesty: must not claim CryptOpt Verified: {stdout}"
    );
    assert!(
        stdout.contains("Incomplete")
            || stdout.contains("not formal superoptimization")
            || stdout.contains("CryptOpt-like"),
        "expected honesty note: {stdout}"
    );

    let run_dir = std::fs::read_dir(&run_base)
        .expect("read run base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("expected search-ingest run directory");

    let chain = Command::new(vaa_bin())
        .args(["evidence", "verify-chain", run_dir.to_str().unwrap()])
        .output()
        .expect("verify-chain");
    let chain_out = String::from_utf8_lossy(&chain.stdout);
    let chain_err = String::from_utf8_lossy(&chain.stderr);
    assert!(
        chain.status.success(),
        "verify-chain failed: stdout={chain_out}\nstderr={chain_err}"
    );
    assert_seal_signature_if_signing(&run_dir);
}

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn gate1_search_ingest_skips_violated_budget() {
    let task = root().join("fixtures/run/find_last_byte/find_last_byte.vaa.toml");
    let contract = root().join("fixtures/run/find_last_byte/find_last_byte.sem.toml");
    let seed = root().join("fixtures/run/find_last_byte/00_write_broken.asm");
    let run_base = root().join("target/vaa-gate1-search-ingest-violated");
    let _ = std::fs::remove_dir_all(&run_base);
    std::fs::create_dir_all(&run_base).unwrap();

    let output = Command::new(vaa_bin())
        .args([
            "search",
            task.to_str().unwrap(),
            seed.to_str().unwrap(),
            "--run-dir",
            run_base.to_str().unwrap(),
            "--budget",
            "2",
            "--mutator",
            "nop-before-ret",
            "--ingest",
            "--contract",
            contract.to_str().unwrap(),
        ])
        .output()
        .expect("run vaa search --ingest violated seed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.contains("semasm unavailable")
        || stderr.contains("semasm unavailable")
        || (stdout.contains("SemASM") && stdout.contains("not found"))
    {
        eprintln!("skipping: SemASM unavailable\nstdout={stdout}\nstderr={stderr}");
        return;
    }

    assert!(
        output.status.success(),
        "search --ingest violated seed failed: {:?}\n{stdout}\n{stderr}",
        output.status
    );
    assert!(
        stdout.contains("verified=false"),
        "violated budget must not claim Verified: {stdout}"
    );
    assert!(
        stdout.contains("violated"),
        "expected skipped violated attempts: {stdout}"
    );
    assert!(
        stdout.contains("reason=budget_exhausted") || stdout.contains("budget_exhausted"),
        "expected budget_exhausted after skipping Violated: {stdout}"
    );
}
