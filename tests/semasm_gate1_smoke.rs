//! Live Gate-1 smoke: SemASM Incomplete + ingest seal chain (ignored without toolchain).

use std::path::PathBuf;
use std::process::Command;

fn vaa_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_vaa"))
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
}
