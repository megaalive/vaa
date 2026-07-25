//! Repair packet export (plan §11): constrained instructions for fixing the
//! generator under test — never the generated assembly or authority files.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::generator::error::GeneratorError;
use crate::generator::spec::GeneratorSpec;
use crate::generator::triage::{triage_status, TriageClass};

/// Accepted repair packet schema version.
pub const REPAIR_PACKET_SCHEMA_VERSION: &str = "0.1";

/// Repository slice of a repair packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRepository {
    pub base_revision: String,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
}

/// Failure slice of a repair packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairFailure {
    /// Triage classification (`generator_*` / `verifier_*` vocabulary).
    pub classification: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic_code: Option<String>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruction_offset: Option<String>,
}

/// Generated artifact reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairArtifact {
    pub path: String,
    pub digest: String,
}

/// Optional source mapping context (assembly ↔ generator source).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairSourceMapping {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator_input: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ir_node: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator_source: Option<String>,
}

/// Fixed reproduction commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairCommands {
    pub build: String,
    pub regenerate: String,
    pub verify: String,
}

/// Full repair packet document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairPacket {
    pub schema_version: String,
    pub task_id: String,
    pub generator_id: String,
    pub repository: RepairRepository,
    pub failure: RepairFailure,
    pub generated_artifact: RepairArtifact,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_mapping: Option<RepairSourceMapping>,
    pub commands: RepairCommands,
    pub constraints: Vec<String>,
}

/// Standard constraints every packet carries (plan §11.1).
#[must_use]
pub fn default_constraints() -> Vec<String> {
    vec![
        "Do not edit generated assembly manually".to_owned(),
        "Do not edit contracts, vectors, task files, or stack.lock.toml".to_owned(),
        "Regenerate all candidates after generator changes".to_owned(),
        "Run the required regression suite before completion".to_owned(),
    ]
}

/// Inputs for building a repair packet.
#[derive(Debug, Clone)]
pub struct RepairPacketInput {
    pub task_id: String,
    pub status: String,
    pub message: String,
    pub diagnostic_code: Option<String>,
    pub instruction_offset: Option<String>,
    pub artifact_path: String,
    pub artifact_digest: String,
    pub source_mapping: Option<RepairSourceMapping>,
    pub commands: RepairCommands,
}

/// Build a repair packet from a generator spec + failure info.
///
/// Fails closed when triage says the failure is **not** a generator defect
/// (verifier-incomplete must route to a SemASM coverage issue instead).
pub fn build_repair_packet(
    spec: &GeneratorSpec,
    input: &RepairPacketInput,
) -> Result<RepairPacket, GeneratorError> {
    if let Some(code) = &input.diagnostic_code {
        crate::generator::diagnostics::validate_diagnostic_code(code)?;
    }

    let decision = triage_status(&input.status);
    if !decision.suggest_generator_repair {
        return Err(GeneratorError::Validation(format!(
            "status `{}` triaged as {:?} — not a generator defect; refusing to export a generator repair packet ({})",
            input.status, decision.class, decision.rationale
        )));
    }

    let classification = classification_label(decision.class);

    Ok(RepairPacket {
        schema_version: REPAIR_PACKET_SCHEMA_VERSION.to_owned(),
        task_id: input.task_id.clone(),
        generator_id: spec.generator_id.clone(),
        repository: RepairRepository {
            base_revision: spec.repository.expected_revision.clone(),
            allowed_paths: spec.patch_policy.allowed_paths.clone(),
            forbidden_paths: spec.patch_policy.forbidden_paths.clone(),
        },
        failure: RepairFailure {
            classification,
            diagnostic_code: input.diagnostic_code.clone(),
            message: input.message.clone(),
            instruction_offset: input.instruction_offset.clone(),
        },
        generated_artifact: RepairArtifact {
            path: input.artifact_path.clone(),
            digest: input.artifact_digest.clone(),
        },
        source_mapping: input.source_mapping.clone(),
        commands: input.commands.clone(),
        constraints: default_constraints(),
    })
}

fn classification_label(class: TriageClass) -> String {
    match class {
        TriageClass::SemanticRejected => "generator_candidate_violated".to_owned(),
        TriageClass::GeneratorDefect => "generator_build_failed".to_owned(),
        TriageClass::VerifierIncomplete | TriageClass::Unknown => "verifier_incomplete".to_owned(),
        TriageClass::ToolchainOrIdentity => "toolchain_unavailable".to_owned(),
        TriageClass::Accepted => "accepted".to_owned(),
    }
}

/// Render the packet as an agent-ready Markdown task prompt (plan §11.2).
#[must_use]
pub fn render_repair_markdown(packet: &RepairPacket) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Repair packet: `{}` (generator `{}`)\n",
        packet.task_id, packet.generator_id
    );
    let _ = writeln!(
        out,
        "Fix the **generator under test**, not the generated assembly.\n\n\
## Failure\n\n\
- classification: `{}`",
        packet.failure.classification
    );
    if let Some(code) = &packet.failure.diagnostic_code {
        let _ = writeln!(out, "- diagnostic_code: `{code}`");
    }
    let _ = writeln!(out, "- message: {}", packet.failure.message);
    if let Some(offset) = &packet.failure.instruction_offset {
        let _ = writeln!(out, "- instruction_offset: `{offset}`");
    }
    let _ = writeln!(
        out,
        "\n## Generated artifact\n\n- path: `{}`\n- digest: `{}`",
        packet.generated_artifact.path, packet.generated_artifact.digest
    );
    if let Some(map) = &packet.source_mapping {
        out.push_str("\n## Source mapping\n\n");
        if let Some(v) = &map.generator_input {
            let _ = writeln!(out, "- generator_input: `{v}`");
        }
        if let Some(v) = &map.ir_node {
            let _ = writeln!(out, "- ir_node: `{v}`");
        }
        if let Some(v) = &map.generator_source {
            let _ = writeln!(out, "- generator_source: `{v}`");
        }
    }
    let _ = writeln!(
        out,
        "\n## Repository\n\n- base_revision: `{}`\n\nAllowed paths:\n",
        packet.repository.base_revision
    );
    for p in &packet.repository.allowed_paths {
        let _ = writeln!(out, "- `{p}`");
    }
    out.push_str("\nForbidden paths (never edit):\n\n");
    for p in &packet.repository.forbidden_paths {
        let _ = writeln!(out, "- `{p}`");
    }
    let _ = writeln!(
        out,
        "\n## Commands\n\n\
1. build: `{}`\n\
2. regenerate: `{}`\n\
3. verify: `{}`\n\n\
## Constraints\n",
        packet.commands.build, packet.commands.regenerate, packet.commands.verify
    );
    for c in &packet.constraints {
        let _ = writeln!(out, "- {c}");
    }
    out
}

/// Write packet JSON (+ optional Markdown sibling).
pub fn write_repair_packet(
    path: &Path,
    packet: &RepairPacket,
    with_markdown: bool,
) -> Result<(), GeneratorError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| GeneratorError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let body = serde_json::to_vec_pretty(packet)
        .map_err(|error| GeneratorError::Validation(format!("serialize repair packet: {error}")))?;
    std::fs::write(path, body).map_err(|source| GeneratorError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if with_markdown {
        let md_path = path.with_extension("md");
        std::fs::write(&md_path, render_repair_markdown(packet)).map_err(|source| {
            GeneratorError::Io {
                path: md_path,
                source,
            }
        })?;
    }
    Ok(())
}

/// Load and structurally validate a repair packet JSON.
pub fn load_repair_packet(path: impl AsRef<Path>) -> Result<RepairPacket, GeneratorError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|source| GeneratorError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let packet: RepairPacket =
        serde_json::from_slice(&bytes).map_err(|error| GeneratorError::Parse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    if packet.schema_version != REPAIR_PACKET_SCHEMA_VERSION {
        return Err(GeneratorError::Validation(format!(
            "unsupported repair packet schema_version `{}`",
            packet.schema_version
        )));
    }
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::spec::{parse_generator_spec, GeneratorSpec};

    fn sample_spec() -> GeneratorSpec {
        let text = r#"
schema_version = "0.1"
generator_id = "hlax64"
[repository]
path = "../hlax64"
expected_revision = "git:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
[build]
command = ["cargo", "build", "--release"]
[generation]
command = ["{generator}", "compile", "{input}", "--output", "{output}"]
[patch_policy]
allowed_paths = ["src/backend/**"]
forbidden_paths = ["**/stack.lock.toml"]
"#;
        parse_generator_spec(Path::new("mem"), text).expect("spec")
    }

    fn sample_input(status: &str) -> RepairPacketInput {
        RepairPacketInput {
            task_id: "hlax64.min_i64.win64".into(),
            status: status.into(),
            message: "RBX modified but not restored".into(),
            diagnostic_code: Some("ABI_CALLEE_SAVED_001".into()),
            instruction_offset: Some("0x17".into()),
            artifact_path: "candidate.asm".into(),
            artifact_digest: "sha256:aa".into(),
            source_mapping: None,
            commands: RepairCommands {
                build: "cargo build --release".into(),
                regenerate: "vaa generator-run --skip-verify ...".into(),
                verify: "vaa generator-run ...".into(),
            },
        }
    }

    #[test]
    fn builds_packet_for_violated_status() {
        let packet = build_repair_packet(&sample_spec(), &sample_input("Violated")).unwrap();
        assert_eq!(
            packet.failure.classification,
            "generator_candidate_violated"
        );
        assert!(!packet.constraints.is_empty());
        let md = render_repair_markdown(&packet);
        assert!(md.contains("ABI_CALLEE_SAVED_001"));
        assert!(md.contains("stack.lock.toml"));
    }

    #[test]
    fn refuses_packet_for_incomplete() {
        let err = build_repair_packet(&sample_spec(), &sample_input("Incomplete")).unwrap_err();
        assert!(err.to_string().contains("not a generator defect"), "{err}");
    }

    #[test]
    fn roundtrip_write_load_with_markdown() {
        let dir = std::env::temp_dir().join(format!(
            "vaa-repair-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let packet = build_repair_packet(&sample_spec(), &sample_input("BehaviorFailed")).unwrap();
        let json_path = dir.join("repair.json");
        write_repair_packet(&json_path, &packet, true).unwrap();
        let loaded = load_repair_packet(&json_path).unwrap();
        assert_eq!(loaded, packet);
        assert!(json_path.with_extension("md").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn golden_repair_packet_fixture_loads() {
        let raw = include_str!("../../schemas/fixtures/repair-packet.golden.json");
        let packet: RepairPacket = serde_json::from_str(raw).expect("golden");
        assert_eq!(packet.schema_version, REPAIR_PACKET_SCHEMA_VERSION);
        assert!(!packet.repository.allowed_paths.is_empty());
        assert!(!packet.constraints.is_empty());
        assert!(!packet.commands.build.is_empty());
    }
}
