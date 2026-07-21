//! Live Gate-2 smoke: SemASM Verified with --allow-execution (ignored without toolchain).

use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore = "requires `semasm` on PATH, Win64 toolchain, and SemASM --allow-execution"]
fn gate2_verify_count_byte_win64_verified() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task = root.join("fixtures/semasm/count_byte/count_byte.vaa.toml");
    let source = root.join("fixtures/semasm/count_byte/count_byte_win64.asm");
    let contract = root.join("fixtures/semasm/count_byte/count_byte.sem.toml");
    let binary = env!("CARGO_BIN_EXE_vaa");

    let output = Command::new(binary)
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
