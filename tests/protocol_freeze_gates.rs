//! Protocol freeze gates: published schemas, Rust structs, and golden
//! fixtures must not drift apart (mirrors SemASM's schema drift gate).
//!
//! The published schemas use `additionalProperties: false`, so a struct field
//! that is missing from the schema would reject real envelopes downstream.

use serde_json::Value;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(rel: &str) -> Value {
    let path = root().join(rel);
    let raw =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

/// Every key of `instance` must be declared in `schema.properties`,
/// recursing into nested objects that declare their own properties.
fn assert_keys_declared(schema: &Value, instance: &Value, ctx: &str) {
    let props = schema["properties"]
        .as_object()
        .unwrap_or_else(|| panic!("{ctx}: schema has no properties object"));
    for (key, val) in instance.as_object().into_iter().flatten() {
        assert!(
            props.contains_key(key),
            "{ctx}: key `{key}` is serialized but missing from the schema; \
             update the schema and golden fixture together"
        );
        if val.is_object() && props[key].get("properties").is_some() {
            assert_keys_declared(&props[key], val, &format!("{ctx}.{key}"));
        }
    }
}

/// Every `required` key of the schema must be present in the fixture.
fn assert_required_present(schema: &Value, instance: &Value, ctx: &str) {
    for req in schema["required"].as_array().into_iter().flatten() {
        let key = req.as_str().expect("required entries are strings");
        assert!(
            instance.get(key).is_some(),
            "{ctx}: schema-required key `{key}` missing from fixture"
        );
    }
}

#[test]
fn agent_envelope_fixture_round_trips_exactly() {
    let fixture = read_json("schemas/fixtures/agent-envelope.direct_nasm.json");
    let env: vaa::AgentEnvelope =
        serde_json::from_value(fixture.clone()).expect("golden fixture parses as AgentEnvelope");
    assert_eq!(env.schema_version, vaa::AGENT_ENVELOPE_SCHEMA_VERSION);
    let back = serde_json::to_value(&env).expect("serialize");
    assert_eq!(
        back, fixture,
        "AgentEnvelope serialization drifted from the golden fixture; \
         update schemas/fixtures/agent-envelope.direct_nasm.json and the schema together"
    );
}

#[test]
fn agent_envelope_schema_declares_every_serialized_field() {
    let schema = read_json("schemas/agent-envelope.schema.json");
    assert_eq!(
        schema["properties"]["schema_version"]["const"],
        Value::from(vaa::AGENT_ENVELOPE_SCHEMA_VERSION),
        "schema const must track AGENT_ENVELOPE_SCHEMA_VERSION"
    );

    // Fully populated envelope: every optional field serialized, so any new
    // struct field that lags the published schema fails here.
    let mut env = vaa::AgentEnvelope::direct_nasm(
        "x86_64-pc-windows-msvc",
        "freeze-gate",
        vaa::AgentCommands {
            doctor: Some("vaa doctor --format json".into()),
            verify: "semasm agent verify candidate.asm contract.sem.toml".into(),
            verify_gate2: Some("… --allow-execution".into()),
            regenerate: Some("regen".into()),
            suite: Some("suite".into()),
        },
        vaa::AgentBudget::default(),
    );
    env.abi = Some("win64".into());
    env.run_id = Some("run-1".into());
    env.remaining_attempts = Some(3);
    env.latest_failure = Some("UNSUPPORTED_SHAPE".into());
    env.digests = vaa::AgentDigests {
        task: Some("sha256:aa".into()),
        contract: Some("sha256:bb".into()),
        candidate: Some("sha256:cc".into()),
    };
    env.semasm_packet_path = Some("task-packet.json".into());
    env.repair_packet_path = Some("repair-packet.json".into());
    env.workspace_dir = Some("ws".into());
    env.prompt_markdown_path = Some("prompt.md".into());
    env.events_path = Some("events.jsonl".into());

    let full = serde_json::to_value(&env).expect("serialize");
    assert_keys_declared(&schema, &full, "AgentEnvelope");

    let fixture = read_json("schemas/fixtures/agent-envelope.direct_nasm.json");
    assert_keys_declared(&schema, &fixture, "AgentEnvelope fixture");
    assert_required_present(&schema, &fixture, "AgentEnvelope fixture");
}

#[test]
fn repair_packet_fixture_round_trips_exactly() {
    let fixture = read_json("schemas/fixtures/repair-packet.golden.json");
    let packet: vaa::generator::RepairPacket =
        serde_json::from_value(fixture.clone()).expect("golden fixture parses as RepairPacket");
    assert_eq!(
        packet.schema_version,
        vaa::generator::REPAIR_PACKET_SCHEMA_VERSION
    );
    let back = serde_json::to_value(&packet).expect("serialize");
    assert_eq!(
        back, fixture,
        "RepairPacket serialization drifted from the golden fixture; \
         update schemas/fixtures/repair-packet.golden.json and the schema together"
    );
}

#[test]
fn repair_packet_schema_declares_every_serialized_field() {
    let schema = read_json("schemas/repair-packet.schema.json");
    assert_eq!(
        schema["properties"]["schema_version"]["const"],
        Value::from(vaa::generator::REPAIR_PACKET_SCHEMA_VERSION),
        "schema const must track REPAIR_PACKET_SCHEMA_VERSION"
    );

    let fixture = read_json("schemas/fixtures/repair-packet.golden.json");
    let mut packet: vaa::generator::RepairPacket =
        serde_json::from_value(fixture.clone()).expect("fixture parses");
    // Populate the remaining optional field so schema lag fails here.
    packet.failure.instruction_offset = Some("0x2f".into());
    let full = serde_json::to_value(&packet).expect("serialize");
    assert_keys_declared(&schema, &full, "RepairPacket");
    assert_keys_declared(&schema, &fixture, "RepairPacket fixture");
    assert_required_present(&schema, &fixture, "RepairPacket fixture");
}

/// Cross-repo contract: the SemASM golden `agent_failure` envelope must parse
/// into VAA's structured error with its stable code intact.
#[test]
fn semasm_agent_failure_golden_parses_to_structured_error() {
    let raw = std::fs::read_to_string(
        root().join("schemas/fixtures/agent-failure.unsupported_shape.semasm.json"),
    )
    .expect("read semasm agent-failure fixture");
    let err = vaa::SemasmVerify::parse_report(&raw)
        .expect_err("agent_failure must not parse as a VerificationReport");
    assert_eq!(err.failure_code(), Some("UNSUPPORTED_SHAPE"));
    match err {
        vaa::VerifyError::AgentFailure {
            code,
            message,
            stage,
            retryability,
            ..
        } => {
            assert_eq!(code, "UNSUPPORTED_SHAPE");
            assert!(!message.is_empty());
            assert_eq!(stage.as_deref(), Some("unsupported_shape"));
            assert_eq!(retryability.as_deref(), Some("never"));
        }
        other => panic!("expected VerifyError::AgentFailure, got {other:?}"),
    }
}

#[test]
fn harness_submit_result_fixture_round_trips_exactly() {
    let fixture = read_json("schemas/fixtures/harness-submit-result.accepted.json");
    let result: vaa::HarnessSubmitResult = serde_json::from_value(fixture.clone())
        .expect("golden fixture parses as HarnessSubmitResult");
    assert_eq!(result.schema_version, vaa::HARNESS_SUBMIT_SCHEMA_VERSION);
    let back = serde_json::to_value(&result).expect("serialize");
    assert_eq!(
        back, fixture,
        "HarnessSubmitResult serialization drifted from the golden fixture; \
         update schemas/fixtures/harness-submit-result.accepted.json and the schema together"
    );
}

#[test]
fn harness_submit_result_schema_declares_every_serialized_field() {
    let schema = read_json("schemas/harness-submit-result.schema.json");
    assert_eq!(
        schema["properties"]["schema_version"]["const"],
        Value::from(vaa::HARNESS_SUBMIT_SCHEMA_VERSION),
        "schema const must track HARNESS_SUBMIT_SCHEMA_VERSION"
    );

    let full = vaa::HarnessSubmitResult {
        schema_version: vaa::HARNESS_SUBMIT_SCHEMA_VERSION.to_owned(),
        class: vaa::HarnessOutcomeClass::Failed,
        next_action: vaa::HarnessNextAction::Abort,
        evidence_status: "failed".into(),
        raw_status: Some("assemble_failed".into()),
        exit_code: 2,
        message: "structured failure".into(),
        failure_code: Some("ASSEMBLE_FAILED".into()),
        candidate_digest: Some("sha256:aa".into()),
        run_dir: Some("runs/r1".into()),
        run_id: Some("r1".into()),
        candidate_index: Some(0),
        candidate_dir: Some("runs/r1/candidates/0000".into()),
        seal_digest: Some("sha256:bb".into()),
        patch_evidence_path: Some("patch-evidence.json".into()),
        assembler: Some("nasm".into()),
        may_auto_retry: false,
    };
    let value = serde_json::to_value(&full).expect("serialize");
    assert_keys_declared(&schema, &value, "HarnessSubmitResult");

    let fixture = read_json("schemas/fixtures/harness-submit-result.accepted.json");
    assert_keys_declared(&schema, &fixture, "HarnessSubmitResult fixture");
    assert_required_present(&schema, &fixture, "HarnessSubmitResult fixture");
}

/// The adapter loop output is produced by Python, so the golden fixtures are
/// generated by `scripts/tests/harness_adapter_dryrun.py`. This gate keeps the
/// schema and those fixtures from drifting apart in-tree.
#[test]
fn harness_loop_result_fixtures_match_schema() {
    let schema = read_json("schemas/harness-loop-result.schema.json");
    let step_schema = &schema["properties"]["steps"]["items"];

    for rel in [
        "schemas/fixtures/harness-loop-result.direct_accepted.json",
        "schemas/fixtures/harness-loop-result.generator_repair_accepted.json",
    ] {
        let fixture = read_json(rel);
        assert_keys_declared(&schema, &fixture, rel);
        assert_required_present(&schema, &fixture, rel);
        assert_eq!(fixture["kind"], Value::from("agent_harness_loop"), "{rel}");

        let steps = fixture["steps"]
            .as_array()
            .unwrap_or_else(|| panic!("{rel}: steps must be an array"));
        assert!(!steps.is_empty(), "{rel}: loop fixture needs steps");
        for (i, step) in steps.iter().enumerate() {
            let ctx = format!("{rel} steps[{i}]");
            assert_keys_declared(step_schema, step, &ctx);
            assert_required_present(step_schema, step, &ctx);
        }
    }
}

/// One classification vocabulary across protocols: the loop result must not
/// invent classes or next actions that `HarnessSubmitResult` cannot produce.
#[test]
fn harness_loop_result_shares_submit_vocabulary() {
    let loop_schema = read_json("schemas/harness-loop-result.schema.json");
    let submit_schema = read_json("schemas/harness-submit-result.schema.json");

    for (def, field) in [("outcome_class", "class"), ("next_action", "next_action")] {
        let mut loop_values: Vec<&Value> = loop_schema["$defs"][def]["enum"]
            .as_array()
            .unwrap_or_else(|| panic!("loop schema $defs.{def}.enum missing"))
            .iter()
            .filter(|v| !v.is_null())
            .collect();
        let mut submit_values: Vec<&Value> = submit_schema["properties"][field]["enum"]
            .as_array()
            .unwrap_or_else(|| panic!("submit schema {field}.enum missing"))
            .iter()
            .collect();
        loop_values.sort_by_key(|v| v.as_str().unwrap_or_default());
        submit_values.sort_by_key(|v| v.as_str().unwrap_or_default());
        assert_eq!(
            loop_values, submit_values,
            "`{field}` vocabulary drifted between harness-loop-result and \
             harness-submit-result schemas"
        );
    }
}
