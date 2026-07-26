//! Release C author gates: init draft, lock admission fail-closed, admitted lock.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn vaa_bin() -> PathBuf {
    env!("CARGO_BIN_EXE_vaa").into()
}

fn tmp(prefix: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn author_init_creates_draft_state() {
    let out = tmp("vaa-author-gate-init");
    let status = Command::new(vaa_bin())
        .args([
            "author",
            "init",
            "--template",
            "pure-int-binary",
            "--name",
            "max_i64",
            "--target",
            "x86_64-pc-windows-msvc",
            "--out",
            out.to_str().unwrap(),
            "--format",
            "json",
        ])
        .current_dir(root())
        .output()
        .expect("run author init");
    assert!(
        status.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&status.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&status.stdout).expect("json");
    assert_eq!(body["ok"], true);
    assert_eq!(body["state"]["state"], "draft");
    assert_eq!(body["state"]["experimental"], false);
    let case = PathBuf::from(body["case_dir"].as_str().unwrap());
    assert!(case.join("AUTHOR_STATE.toml").is_file());
    assert!(case.join("task.vaa.toml").is_file());
    assert!(!case.join("LOCKED").is_file());

    let validate = Command::new(vaa_bin())
        .args(["validate", case.join("task.vaa.toml").to_str().unwrap()])
        .output()
        .expect("validate");
    assert!(
        validate.status.success(),
        "validate failed: {}",
        String::from_utf8_lossy(&validate.stderr)
    );
}

#[test]
fn author_lock_without_admission_fails() {
    let out = tmp("vaa-author-gate-noadm");
    let init = Command::new(vaa_bin())
        .args([
            "author",
            "init",
            "--template",
            "pure-int-binary",
            "--name",
            "not_admitted_leaf",
            "--target",
            "x86_64-pc-windows-msvc",
            "--out",
            out.to_str().unwrap(),
        ])
        .current_dir(root())
        .output()
        .expect("init");
    assert!(init.status.success());
    let case = out.join("not_admitted_leaf");
    let lock = Command::new(vaa_bin())
        .args(["author", "lock", case.to_str().unwrap()])
        .current_dir(root())
        .output()
        .expect("lock");
    assert!(
        !lock.status.success(),
        "lock must fail without admission; stdout={}",
        String::from_utf8_lossy(&lock.stdout)
    );
    let err = String::from_utf8_lossy(&lock.stderr);
    assert!(
        err.contains("not admitted") || err.contains("review issues"),
        "unexpected stderr: {err}"
    );
    assert!(!case.join("LOCKED").is_file());
}

#[test]
fn author_lock_admitted_max_i64_writes_locked_marker() {
    let out = tmp("vaa-author-gate-lock");
    let init = Command::new(vaa_bin())
        .args([
            "author",
            "init",
            "--template",
            "pure-int-binary",
            "--name",
            "max_i64",
            "--target",
            "x86_64-pc-windows-msvc",
            "--out",
            out.to_str().unwrap(),
            "--format",
            "json",
        ])
        .current_dir(root())
        .output()
        .expect("init");
    assert!(
        init.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&init.stderr)
    );
    let case = out.join("max_i64");
    let lock = Command::new(vaa_bin())
        .args([
            "author",
            "lock",
            case.to_str().unwrap(),
            "--format",
            "json",
        ])
        .current_dir(root())
        .output()
        .expect("lock");
    assert!(
        lock.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&lock.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&lock.stdout).expect("json");
    assert_eq!(body["ok"], true);
    assert_eq!(body["state"]["state"], "locked");
    assert_ne!(body["acceptance"], "sealed_acceptance");
    assert!(Path::new(body["locked_marker"].as_str().unwrap()).is_file());
    let marker = std::fs::read_to_string(case.join("LOCKED")).unwrap();
    assert!(marker.contains("task_digest"));
    assert!(marker.contains("capability_snapshot_digest"));
}
