//! Agent harness façade gates: prepare / submit / resume (direct + generator).

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn vaa_bin() -> PathBuf {
    env!("CARGO_BIN_EXE_vaa").into()
}

fn semasm_available() -> bool {
    Command::new("semasm")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        || std::env::var_os("SEMASM_BIN").is_some()
}

#[test]
fn harness_prepare_direct_nasm_emits_envelope() {
    let tmp = std::env::temp_dir().join(format!(
        "vaa-harness-prep-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let task = root().join("fixtures/semasm/count_byte/count_byte.vaa.toml");
    let contract = root().join("fixtures/semasm/count_byte/count_byte.sem.toml");
    let seed = root().join("fixtures/run/count_byte/01_wrong.asm");

    let out = Command::new(vaa_bin())
        .args([
            "harness",
            "prepare",
            "--mode",
            "direct-nasm",
            "--task",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--workspace",
            tmp.to_str().unwrap(),
            "--seed",
            seed.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("spawn vaa harness prepare");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "prepare failed: status={:?}\nstdout={stdout}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json stdout");
    assert_eq!(value["mode"], "direct_nasm");
    assert_eq!(value["schema_version"], "0.1");
    assert!(tmp.join("agent-envelope.json").is_file());
    assert!(tmp.join("candidate.asm").is_file());
    assert!(tmp.join("prompt.md").is_file());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn harness_prepare_generator_repair_from_fixture() {
    let tmp = std::env::temp_dir().join(format!(
        "vaa-harness-gen-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let packet = root().join(
        "fixtures/repair/hlax64-stack-balance-win64-live-worktree/repair-packet.json",
    );

    let out = Command::new(vaa_bin())
        .args([
            "harness",
            "prepare",
            "--mode",
            "generator-repair",
            "--repair-packet",
            packet.to_str().unwrap(),
            "--workspace",
            tmp.to_str().unwrap(),
            "--target",
            "x86_64-pc-windows-msvc",
            "--format",
            "json",
        ])
        .output()
        .expect("spawn prepare generator");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "prepare generator failed: {}\n{}",
        stdout,
        String::from_utf8_lossy(&out.stderr)
    );
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json");
    assert_eq!(value["mode"], "generator_repair");
    assert!(tmp.join("repair-packet.json").is_file());
    assert!(tmp.join("agent-envelope.json").is_file());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn harness_submit_wrong_then_repaired_count_byte_win64() {
    if !semasm_available() {
        eprintln!("skipping harness submit gate: semasm unavailable");
        return;
    }

    let task = root().join("fixtures/run/count_byte/count_byte.vaa.toml");
    let contract = root().join("fixtures/run/count_byte/count_byte.sem.toml");
    let wrong = root().join("fixtures/run/count_byte/01_wrong.asm");
    let repaired = root().join("fixtures/run/count_byte/02_repaired.asm");

    let wrong_out = Command::new(vaa_bin())
        .args([
            "harness",
            "submit",
            "--task",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--source",
            wrong.to_str().unwrap(),
            "--allow-execution",
            "--format",
            "json",
        ])
        .output()
        .expect("submit wrong");
    let wrong_stdout = String::from_utf8_lossy(&wrong_out.stdout);
    let wrong_json: serde_json::Value =
        serde_json::from_str(wrong_stdout.trim()).unwrap_or_else(|_| {
            panic!(
                "expected JSON stdout for wrong candidate; got: {wrong_stdout}\nstderr={}",
                String::from_utf8_lossy(&wrong_out.stderr)
            )
        });
    let class = wrong_json["class"].as_str().unwrap_or_default();
    assert!(
        matches!(class, "violated_repairable" | "failed" | "toolchain_retryable"),
        "wrong candidate unexpected class={class} body={wrong_json}"
    );
    if class == "toolchain_retryable" {
        eprintln!("skipping repair half: toolchain retryable ({wrong_json})");
        return;
    }

    let ok_out = Command::new(vaa_bin())
        .args([
            "harness",
            "submit",
            "--task",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--source",
            repaired.to_str().unwrap(),
            "--allow-execution",
            "--allow-under-preconditions",
            "--format",
            "json",
        ])
        .output()
        .expect("submit repaired");
    let ok_stdout = String::from_utf8_lossy(&ok_out.stdout);
    let ok_json: serde_json::Value = serde_json::from_str(ok_stdout.trim()).unwrap_or_else(|_| {
        panic!(
            "expected JSON stdout for repaired; got: {ok_stdout}\nstderr={}",
            String::from_utf8_lossy(&ok_out.stderr)
        )
    });
    assert_eq!(
        ok_json["class"], "accepted",
        "repaired candidate not accepted: {ok_json}"
    );
    assert_eq!(ok_json["next_action"], "done");
    assert_eq!(ok_out.status.code(), Some(0));
}

#[test]
fn harness_resume_status_on_empty_run_dir_is_structured() {
    let tmp = std::env::temp_dir().join(format!(
        "vaa-harness-resume-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    // Create a minimal run dir via `vaa ingest` would be heavy; just assert CLI
    // returns structured JSON on missing/incomplete dirs without panicking.
    std::fs::create_dir_all(&tmp).unwrap();
    let out = Command::new(vaa_bin())
        .args([
            "harness",
            "status",
            "--run-dir",
            tmp.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("status");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Empty dir is not a valid RunDir — expect tool failure JSON, still parseable.
    assert!(!stdout.trim().is_empty() || !out.status.success());
    if !stdout.trim().is_empty() {
        let _: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn harness_cli_help_lists_subcommands() {
    let out = Command::new(vaa_bin())
        .args(["harness", "--help"])
        .output()
        .expect("help");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("prepare"));
    assert!(text.contains("submit"));
    assert!(text.contains("resume"));
    assert!(text.contains("status"));
}

#[test]
fn invalid_nasm_structured_failure_when_semasm_present() {
    if !semasm_available() {
        eprintln!("skipping invalid NASM structured failure: semasm unavailable");
        return;
    }
    let tmp = std::env::temp_dir().join(format!(
        "vaa-harness-badnasm-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let bad = tmp.join("bad.asm");
    std::fs::write(&bad, "this is not valid nasm!!!!\n").unwrap();

    let task = root().join("fixtures/semasm/count_byte/count_byte.vaa.toml");
    let contract = root().join("fixtures/semasm/count_byte/count_byte.sem.toml");
    let out = Command::new(vaa_bin())
        .args([
            "harness",
            "submit",
            "--task",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--source",
            bad.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("submit bad");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|_| {
        panic!("expected JSON; got {stdout}\nstderr={}", String::from_utf8_lossy(&out.stderr))
    });
    let class = json["class"].as_str().unwrap_or_default();
    assert!(
        matches!(class, "failed" | "toolchain_retryable" | "violated_repairable"),
        "unexpected class={class} {json}"
    );
    // Prefer structured failure code when SemASM emits agent_failure.
    if let Some(code) = json["failure_code"].as_str() {
        assert!(!code.is_empty());
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

#[allow(dead_code)]
fn path_exists(p: &Path) -> bool {
    p.exists()
}
