//! Agent harness façade gates: prepare / submit+seal / resume / generator policy.

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

fn tmp(prefix: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn harness_prepare_direct_nasm_emits_envelope() {
    let tmp = tmp("vaa-harness-prep");
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
            "--assembler",
            "nasm",
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
    assert_eq!(value["assembler"], "nasm");
    assert_eq!(value["schema_version"], "0.1");
    assert!(tmp.join("agent-envelope.json").is_file());
    assert!(tmp.join("candidate.asm").is_file());
    assert!(tmp.join("prompt.md").is_file());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn harness_prepare_gas_is_fail_closed() {
    let tmp = tmp("vaa-harness-gas");
    let task = root().join("fixtures/semasm/count_byte/count_byte.vaa.toml");
    let contract = root().join("fixtures/semasm/count_byte/count_byte.sem.toml");
    let out = Command::new(vaa_bin())
        .args([
            "harness",
            "prepare",
            "--mode",
            "direct",
            "--task",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--workspace",
            tmp.to_str().unwrap(),
            "--assembler",
            "gas",
            "--format",
            "json",
        ])
        .output()
        .expect("prepare gas");
    assert!(
        !out.status.success(),
        "gas prepare must fail-closed; stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn harness_prepare_generator_repair_from_fixture() {
    let tmp = tmp("vaa-harness-gen");
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
fn harness_submit_wrong_then_repaired_with_seal_and_chain() {
    if !semasm_available() {
        eprintln!("skipping harness seal gate: semasm unavailable");
        return;
    }

    let run_base = tmp("vaa-harness-seal-base");
    let task = root().join("fixtures/run/count_byte/count_byte.vaa.toml");
    let contract = root().join("fixtures/run/count_byte/count_byte.sem.toml");
    let wrong = root().join("fixtures/run/count_byte/01_wrong.asm");
    let repaired = root().join("fixtures/run/count_byte/02_repaired.asm");

    let wrong_out = Command::new(vaa_bin())
        .args([
            "harness",
            "submit",
            "--mode",
            "direct-nasm",
            "--task",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--source",
            wrong.to_str().unwrap(),
            "--allow-execution",
            "--run-base",
            run_base.to_str().unwrap(),
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
    if class == "toolchain_retryable" {
        eprintln!("skipping seal gate: toolchain retryable ({wrong_json})");
        let _ = std::fs::remove_dir_all(&run_base);
        return;
    }
    assert!(
        matches!(class, "violated_repairable" | "failed" | "incomplete_coverage" | "accepted"),
        "unexpected class={class} {wrong_json}"
    );
    let run_dir = wrong_json["run_dir"]
        .as_str()
        .expect("sealed submit must return run_dir")
        .to_owned();
    assert!(
        wrong_json["seal_digest"].as_str().is_some(),
        "expected seal_digest: {wrong_json}"
    );

    let ok_out = Command::new(vaa_bin())
        .args([
            "harness",
            "submit",
            "--mode",
            "direct-nasm",
            "--task",
            task.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--source",
            repaired.to_str().unwrap(),
            "--allow-execution",
            "--allow-under-preconditions",
            "--run-dir",
            &run_dir,
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
    assert_eq!(ok_json["candidate_index"], 1);
    assert!(ok_json["seal_digest"].as_str().is_some());

    // Resume must not reseal index 0; cursor advances.
    let status = Command::new(vaa_bin())
        .args(["harness", "status", "--run-dir", &run_dir, "--format", "json"])
        .output()
        .expect("status");
    let status_json: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&status.stdout).trim()).expect("status json");
    assert_eq!(status_json["next_candidate_index"], 2);
    assert!(status_json["events_path"].as_str().is_some());

    let chain = Command::new(vaa_bin())
        .args(["evidence", "verify-chain", &run_dir])
        .output()
        .expect("verify-chain");
    assert!(
        chain.status.success(),
        "verify-chain failed: stdout={}\nstderr={}",
        String::from_utf8_lossy(&chain.stdout),
        String::from_utf8_lossy(&chain.stderr)
    );

    let _ = std::fs::remove_dir_all(&run_base);
}

#[test]
fn harness_generator_submit_rejects_authority_mutation() {
    let tmp = tmp("vaa-harness-auth");
    let packet = root().join(
        "fixtures/repair/hlax64-stack-balance-win64-live-worktree/repair-packet.json",
    );
    let suite_ev = root().join("fixtures/repair/echoasm-passthrough/suite-evidence.accepted.json");

    let out = Command::new(vaa_bin())
        .args([
            "harness",
            "submit",
            "--mode",
            "generator-repair",
            "--repair-packet",
            packet.to_str().unwrap(),
            "--workspace",
            tmp.to_str().unwrap(),
            "--changed-file",
            "integrations/hlax64/cases/stack_local_i64/task.vaa.toml",
            "--patched-revision",
            "git:deadbeef",
            "--suite-evidence",
            suite_ev.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("generator submit");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|_| {
        panic!(
            "expected JSON; got {stdout}\nstderr={}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    assert_eq!(json["class"], "policy_blocked");
    assert_eq!(json["failure_code"], "FORBIDDEN_PATH");
    assert_eq!(out.status.code(), Some(8));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn harness_generator_submit_accepted_from_suite_evidence() {
    let tmp = tmp("vaa-harness-patch-ok");
    let packet = root().join(
        "fixtures/repair/hlax64-stack-balance-win64-live-worktree/repair-packet.json",
    );
    let suite_ev = root().join("fixtures/repair/echoasm-passthrough/suite-evidence.accepted.json");

    let out = Command::new(vaa_bin())
        .args([
            "harness",
            "submit",
            "--mode",
            "generator-repair",
            "--repair-packet",
            packet.to_str().unwrap(),
            "--workspace",
            tmp.to_str().unwrap(),
            "--changed-file",
            "src/HlaX64.Backend.Nasm/Emit.cs",
            "--patched-revision",
            "git:cafebabe",
            "--suite-evidence",
            suite_ev.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("generator submit ok");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|_| {
        panic!(
            "expected JSON; got {stdout}\nstderr={}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    assert_eq!(json["class"], "accepted", "{json}");
    assert!(
        json["patch_evidence_path"].as_str().is_some(),
        "expected patch evidence path: {json}"
    );
    assert!(tmp.join("patch-evidence.json").is_file());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn harness_resume_status_on_empty_run_dir_is_structured() {
    let tmp = tmp("vaa-harness-resume");
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
    let tmp = tmp("vaa-harness-badnasm");
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
        panic!(
            "expected JSON; got {stdout}\nstderr={}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    let class = json["class"].as_str().unwrap_or_default();
    assert!(
        matches!(class, "failed" | "toolchain_retryable" | "violated_repairable"),
        "unexpected class={class} {json}"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[allow(dead_code)]
fn path_exists(p: &Path) -> bool {
    p.exists()
}
