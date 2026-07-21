//! Live smoke: `vaa verify` + SemASM count_byte Win64 (ignored without toolchain).

use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore = "requires `semasm` on PATH and a Win64 assemble/link toolchain"]
fn vaa_verify_count_byte_win64_execution_denied_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task = root.join("fixtures/semasm/count_byte/count_byte.vaa.toml");
    let source = root.join("fixtures/semasm/count_byte/count_byte_win64.asm");
    let contract = root.join("fixtures/semasm/count_byte/count_byte.sem.toml");
    let binary = env!("CARGO_BIN_EXE_vaa");

    let output = Command::new(binary)
        .args([
            "verify",
            task.to_str().expect("utf-8 task"),
            "--source",
            source.to_str().expect("utf-8 source"),
            "--contract",
            contract.to_str().expect("utf-8 contract"),
            "--format",
            "json",
        ])
        .output()
        .expect("run vaa verify");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let value: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(error) => {
            if stdout.contains("semasm binary not found")
                || stderr.contains("semasm")
                || stdout.is_empty()
            {
                eprintln!("skipping smoke: no usable evidence JSON ({error})\nstdout={stdout}\nstderr={stderr}");
                return;
            }
            panic!("expected evidence JSON ({error}): stdout={stdout}\nstderr={stderr}");
        }
    };

    if value["doctor"]["status"] == "Unavailable"
        || value["summary"]
            .as_str()
            .is_some_and(|s| s.contains("semasm binary not found"))
    {
        eprintln!("skipping smoke: SemASM unavailable\n{value}");
        return;
    }

    // Without --allow-execution, SemASM static-OK maps to Incomplete.
    assert_eq!(
        value["final_status"], "Incomplete",
        "expected Incomplete evidence; got {value}"
    );

    let verify = &value["verify_report"];
    assert!(!verify.is_null(), "expected verify_report present: {value}");
    assert_eq!(verify["outcome"], "Incomplete");
    assert_eq!(verify["raw_status"], "execution_denied");
    assert!(verify["tool_version"]
        .as_str()
        .is_some_and(|v| v.starts_with("semasm ")));
    assert!(verify["contract_digest"]
        .as_str()
        .is_some_and(|v| v.starts_with("sha256:")));
    assert!(verify["source_digest"]
        .as_str()
        .is_some_and(|v| v.starts_with("sha256:")));
}
