//! External generator bridge types (stack lock + generator spec).
//!
//! Generator-agnostic VAA core for the verified-repair bridge plan.
//! HlaX64 is an instance pack under `integrations/`, not a VAA core dependency.

mod diagnostics;
mod error;
mod generate;
mod identity;
mod patch;
mod path_policy;
mod repair;
mod repo_guard;
mod run;
mod source_map;
mod spec;
mod stack_lock;
mod suite;
mod triage;

pub use diagnostics::{
    diagnostics_by_category, is_well_formed_diagnostic_code, lookup_diagnostic,
    validate_diagnostic_code, DiagnosticEntry, DiagnosticView, DIAGNOSTIC_REGISTRY,
};
pub use error::GeneratorError;
pub use generate::{
    expand_generation_command, generate_candidate, GenerationOutcome, GenerationRequest,
};
pub use identity::{
    build_and_identify, build_generator, establish_binary_identity, resolve_generator_binary,
    GeneratorBinaryIdentity,
};
pub use patch::{
    build_patch_evidence, git_changed_files, load_patch_evidence, patch_evidence_digest,
    validate_patch_evidence, verify_patch_evidence_file, write_patch_evidence, PatchEvidence,
    PatchEvidenceInput, PatchStatus, PATCH_EVIDENCE_SCHEMA_VERSION,
};
pub use path_policy::{check_path_policy, check_paths_against_spec, PathPolicyReport};
pub use repair::{
    build_repair_packet, default_constraints, load_repair_packet, render_repair_markdown,
    write_repair_packet, RepairArtifact, RepairCommands, RepairFailure, RepairPacket,
    RepairPacketInput, RepairRepository, RepairSourceMapping, REPAIR_PACKET_SCHEMA_VERSION,
};
pub use repo_guard::{
    check_repository, glob_match, path_policy_violations, resolve_repository_path, RepoGuardConfig,
    RepoGuardReport,
};
pub use run::{
    resolve_maybe_relative, run_generator_case, GeneratorRunConfig, GeneratorRunError,
    GeneratorRunOutcome, VerifySummary,
};
pub use source_map::{
    entry_to_repair_mapping, join_by_assembly_line, join_by_offset, load_source_map, parse_offset,
    validate_source_map, SourceMap, SourceMapEntry, SOURCE_MAP_SCHEMA_VERSION,
};
pub use spec::{
    load_generator_spec, parse_generator_spec, validate_generator_spec, BuildSpec, GenerationSpec,
    GeneratorRepository, GeneratorSpec, IdentityPolicy, PatchPolicy, GENERATOR_SPEC_SCHEMA_VERSION,
};
pub use stack_lock::{
    load_stack_lock, parse_stack_lock, stack_lock_digest, validate_stack_lock, ComponentPin,
    GeneratorPin, StackLock, StackLockDigest, ToolchainPin, STACK_LOCK_SCHEMA_VERSION,
};
pub use suite::{
    aggregate_suite_status, load_suite_manifest, parse_suite_manifest, resolve_case_paths,
    run_suite, suite_manifest_digest, validate_suite_manifest, CasePaths, SuiteCaseResult,
    SuiteEvidence, SuiteGeneratorRef, SuiteManifest, SuitePolicy, SuiteRunConfig, SuiteRunReport,
    SuiteStatus, SUITE_SCHEMA_VERSION,
};
pub use triage::{should_export_repair_packet, triage_status, TriageClass, TriageDecision};
