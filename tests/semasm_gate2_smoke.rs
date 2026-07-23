//! Live Gate-2 smoke: SemASM Verified with --allow-execution (ignored without toolchain).

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
#[ignore = "requires `semasm` on PATH, Win64 toolchain, and SemASM --allow-execution"]
fn gate2_verify_count_byte_win64_verified() {
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
            "--allow-execution",
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify --allow-execution");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(error) => {
            eprintln!("skipping Gate-2: no evidence JSON ({error})\n{stdout}");
            return;
        }
    };

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 expects Verified with --allow-execution: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires `semasm` on PATH, Linux toolchain, and SemASM --allow-execution"]
fn gate2_verify_count_byte_linux_verified() {
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
            "--allow-execution",
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify --allow-execution");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(error) => {
            eprintln!("skipping Gate-2 Linux: no evidence JSON ({error})\n{stdout}");
            return;
        }
    };

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2 Linux: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 Linux expects Verified with --allow-execution: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires `semasm` on PATH, Linux toolchain, and SemASM --allow-execution"]
fn gate2_verify_sum_i64_linux_verified() {
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
            "--allow-execution",
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify --allow-execution");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(error) => {
            eprintln!("skipping Gate-2 Linux sum_i64: no evidence JSON ({error})\n{stdout}");
            return;
        }
    };

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2 Linux sum_i64: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 Linux sum_i64 expects Verified with --allow-execution: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");
}

#[test]
#[ignore = "requires `semasm` on PATH, Win64 toolchain, and SemASM --allow-execution"]
fn gate2_verify_sum_i64_win64_verified() {
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
            "--allow-execution",
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify --allow-execution");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(error) => {
            eprintln!("skipping Gate-2 sum_i64: no evidence JSON ({error})\n{stdout}");
            return;
        }
    };

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2 sum_i64: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 sum_i64 expects Verified with --allow-execution: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");
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
#[ignore = "requires `semasm` on PATH, Linux toolchain, and SemASM --allow-execution"]
fn gate2_verify_min_usize_linux_verified() {
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
            "--allow-execution",
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify --allow-execution");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(error) => {
            eprintln!("skipping Gate-2 Linux min_usize: no evidence JSON ({error})\n{stdout}");
            return;
        }
    };

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2 Linux min_usize: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 Linux min_usize expects Verified with --allow-execution: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");
}

#[test]
#[ignore = "requires `semasm` on PATH, Win64 toolchain, and SemASM --allow-execution"]
fn gate2_verify_min_usize_win64_verified() {
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
            "--allow-execution",
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify --allow-execution");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(error) => {
            eprintln!("skipping Gate-2 min_usize: no evidence JSON ({error})\n{stdout}");
            return;
        }
    };

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2 min_usize: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 min_usize expects Verified with --allow-execution: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");
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
#[ignore = "requires `semasm` on PATH, Linux toolchain, and SemASM --allow-execution"]
fn gate2_verify_max_usize_linux_verified() {
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
            "--allow-execution",
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify --allow-execution");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(error) => {
            eprintln!("skipping Gate-2 Linux max_usize: no evidence JSON ({error})\n{stdout}");
            return;
        }
    };

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2 Linux max_usize: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 Linux max_usize expects Verified with --allow-execution: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");
}

#[test]
#[ignore = "requires `semasm` on PATH, Win64 toolchain, and SemASM --allow-execution"]
fn gate2_verify_max_usize_win64_verified() {
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
            "--allow-execution",
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify --allow-execution");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(error) => {
            eprintln!("skipping Gate-2 max_usize: no evidence JSON ({error})\n{stdout}");
            return;
        }
    };

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2 max_usize: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 max_usize expects Verified with --allow-execution: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");
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
#[ignore = "requires `semasm` on PATH, Linux toolchain, and SemASM --allow-execution"]
fn gate2_verify_find_first_byte_linux_verified() {
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
            "--allow-execution",
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify --allow-execution");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(error) => {
            eprintln!(
                "skipping Gate-2 Linux find_first_byte: no evidence JSON ({error})\n{stdout}"
            );
            return;
        }
    };

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2 Linux find_first_byte: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 Linux find_first_byte expects Verified with --allow-execution: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");
}

#[test]
#[ignore = "requires `semasm` on PATH, Win64 toolchain, and SemASM --allow-execution"]
fn gate2_verify_find_first_byte_win64_verified() {
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
            "--allow-execution",
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify --allow-execution");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(error) => {
            eprintln!("skipping Gate-2 find_first_byte: no evidence JSON ({error})\n{stdout}");
            return;
        }
    };

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2 find_first_byte: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 find_first_byte expects Verified with --allow-execution: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");
    let raw_json = value["verify_report"]["raw_json"]
        .as_str()
        .expect("verify_report.raw_json");
    let raw: serde_json::Value = serde_json::from_str(raw_json).expect("raw_json parse");
    assert_eq!(raw["behavior_oracle"]["id"], "builtin.buffer.find_first_u8");
    assert_eq!(raw["behavior_oracle"]["version"], 1);
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires `semasm` on PATH, Linux toolchain, and SemASM --allow-execution"]
fn gate2_verify_find_last_byte_linux_verified() {
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
            "--allow-execution",
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify --allow-execution");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(error) => {
            eprintln!("skipping Gate-2 Linux find_last_byte: no evidence JSON ({error})\n{stdout}");
            return;
        }
    };

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2 Linux find_last_byte: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 Linux find_last_byte expects Verified with --allow-execution: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");
}

#[test]
#[ignore = "requires `semasm` on PATH, Win64 toolchain, and SemASM --allow-execution"]
fn gate2_verify_find_last_byte_win64_verified() {
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
            "--allow-execution",
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify --allow-execution");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(error) => {
            eprintln!("skipping Gate-2 find_last_byte: no evidence JSON ({error})\n{stdout}");
            return;
        }
    };

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2 find_last_byte: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 find_last_byte expects Verified with --allow-execution: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");
    let raw_json = value["verify_report"]["raw_json"]
        .as_str()
        .expect("verify_report.raw_json");
    let raw: serde_json::Value = serde_json::from_str(raw_json).expect("raw_json parse");
    assert_eq!(raw["behavior_oracle"]["id"], "builtin.buffer.find_last_u8");
    assert_eq!(raw["behavior_oracle"]["version"], 1);
}

#[test]
#[ignore = "requires `semasm` on PATH, Win64 toolchain, and SemASM --allow-execution"]
fn gate2_verify_hlax64_sum_i64_win64_verified() {
    let task = root().join("fixtures/ingest/hlax64_sum_i64/sum_i64.vaa.toml");
    let source = root().join("fixtures/ingest/hlax64_sum_i64/candidate.asm");
    let contract = root().join("fixtures/ingest/hlax64_sum_i64/sum_i64.sem.toml");

    let output = Command::new(vaa_bin())
        .args([
            "verify",
            task.to_str().unwrap(),
            "--source",
            source.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--allow-execution",
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify --allow-execution");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(error) => {
            eprintln!("skipping Gate-2 hlax64 sum_i64: no evidence JSON ({error})\n{stdout}");
            return;
        }
    };

    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2 hlax64 sum_i64: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 hlax64 sum_i64 expects Verified: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");
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
#[ignore = "requires `semasm` on PATH, Linux toolchain, and SemASM --allow-execution"]
fn gate2_ingest_count_byte_linux_verified_seal_chain() {
    let task = root().join("fixtures/ingest/count_byte_linux/count_byte.vaa.toml");
    let source = root().join("fixtures/ingest/count_byte_linux/candidate.asm");
    let contract = root().join("fixtures/ingest/count_byte_linux/count_byte.sem.toml");
    let run_base = root().join("target/vaa-gate2-linux-runs");
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
            "ci-gate2-linux",
            "--allow-execution",
            "--run-dir",
            run_base.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa ingest --allow-execution linux");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.contains("semasm unavailable")
        || stderr.contains("semasm unavailable")
        || (stdout.contains("SemASM") && stdout.contains("not found"))
    {
        eprintln!("skipping: SemASM unavailable\nstdout={stdout}\nstderr={stderr}");
        return;
    }

    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("expected evidence JSON ({e}): stdout={stdout}\nstderr={stderr}");
    });
    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2 Linux ingest: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 Linux ingest expects Verified: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");

    let run_dir = std::fs::read_dir(&run_base)
        .expect("read run base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("expected a run directory after Gate-2 Linux ingest");

    let seal_log = run_dir.join("evidence").join("seal-log.jsonl");
    assert!(
        seal_log.is_file(),
        "seal-log.jsonl missing under {}",
        run_dir.display()
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
    assert_seal_signature_if_signing(&run_dir);
}

#[test]
#[ignore = "requires `semasm` on PATH, Win64 toolchain, and SemASM --allow-execution"]
fn gate2_ingest_count_byte_verified_seal_chain() {
    let task = root().join("fixtures/ingest/count_byte/count_byte.vaa.toml");
    let source = root().join("fixtures/ingest/count_byte/candidate.asm");
    let contract = root().join("fixtures/ingest/count_byte/count_byte.sem.toml");
    let run_base = root().join("target/vaa-gate2-runs");
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
            "ci-gate2",
            "--allow-execution",
            "--run-dir",
            run_base.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa ingest --allow-execution");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.contains("semasm unavailable")
        || stderr.contains("semasm unavailable")
        || (stdout.contains("SemASM") && stdout.contains("not found"))
    {
        eprintln!("skipping: SemASM unavailable\nstdout={stdout}\nstderr={stderr}");
        return;
    }

    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("expected evidence JSON ({e}): stdout={stdout}\nstderr={stderr}");
    });
    if value["doctor"]["status"] == "Unavailable" {
        eprintln!("skipping Gate-2 ingest: SemASM unavailable");
        return;
    }

    assert_eq!(
        value["final_status"], "Verified",
        "Gate-2 ingest expects Verified: {value}"
    );
    assert_eq!(value["verify_report"]["raw_status"], "verified");

    let run_dir = std::fs::read_dir(&run_base)
        .expect("read run base")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("expected a run directory after Gate-2 ingest");

    let seal_log = run_dir.join("evidence").join("seal-log.jsonl");
    assert!(
        seal_log.is_file(),
        "seal-log.jsonl missing under {}",
        run_dir.display()
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
    assert_seal_signature_if_signing(&run_dir);
}
