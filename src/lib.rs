#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::struct_excessive_bools,
    clippy::too_many_lines
)]

pub mod author;
pub mod build;
pub mod cache;
pub mod candidate;
pub mod canonical_json;
pub mod evidence;
pub mod exit_code;
pub mod generator;
pub mod harness;
pub mod inspect;
pub mod model;
pub mod optimize;
pub mod orchestrate;
pub mod process;
pub mod run;
pub mod sandbox;
pub mod search;
pub mod semasm;
pub mod task;

pub use author::{
    abi_for_target, author_init, author_lock, author_review, is_known_template, load_author_state,
    load_catalog, task_id_for, template_meta, AdmissionSummary, AuthorCaseState, AuthorError,
    AuthorState, InitResult, LockResult, ReviewResult, TemplateMeta, AUTHOR_STATE_FILE,
    AUTHOR_TEMPLATES_DIR, DEFAULT_AUTHOR_ASSEMBLER, LOCKED_MARKER_FILE,
};
pub use build::{
    check_reproducible, compare_canonical, nasm_format_for_target, probe_container_runtime,
    remap_host_args_to_container, reproducible_build_check, tool_digest, BuildManifest,
    BuildOutcome, BuildPipeline, CanonicalBuildView, ContainerBuildOpts, PipelineConfig,
    ReproReport, DEFAULT_CONTAINER_IMAGE,
};
pub use cache::{
    args_fingerprint, build_cache_key, default_cache_root, may_reuse_build, may_reuse_verification,
    resolve_cache_root, verification_cache_key, BuildCacheArtifacts, BuildCacheRecord,
    BuildKeyMaterials, CacheError, CacheReuseDecision, CacheStats, CacheStore,
    VerificationCacheRecord, VerificationKeyMaterials, CACHE_SCHEMA_VERSION,
};
pub use candidate::{CandidateProtocol, CandidateSubmission, SubmissionOutcome};
pub use canonical_json::{CANONICALIZATION_ID, DIGEST_ALGORITHM_ID};
#[cfg(feature = "fulcio")]
pub use evidence::UreqFulcioTransport;
#[cfg(feature = "rekor")]
pub use evidence::UreqRekorTransport;
pub use evidence::{
    dry_run_oidc_token, export_transparency, keygen_seal, keyless_sign_transparency,
    may_claim_verified, oidc_subject, probe_durability, publish_dsse, publish_files_seal_last,
    read_transparency_file, sha256_digest_prefixed, verify_bundle, verify_chain,
    verify_dsse_envelope, verify_entry_matches_dsse, verify_seal, verify_transparency_against_run,
    write_dsse_file, write_sealed_evidence, write_transparency_file, ChainIdentity, CheckOutcome,
    DsseEnvelope, DurabilityClass, DurabilityProbeReport, EvidenceAggregator, EvidenceExpect,
    EvidenceReport, EvidenceStatus, FulcioError, FulcioSigningResult, GeneratorMeta,
    HsmPkcs11Signer, MockFulcioTransport, MockRekorTransport, PracticeEd25519Signer,
    RekorPublishResult, ReproducibleBuildOutcome, SealEnvelope, SealError, SealSignature,
    SealSigner, SignerKind, SigstoreDsseSigner, TransparencyDocument, VerifyChainReport,
    DSSE_PAYLOAD_TYPE_TRANSPARENCY, ENV_REQUIRE_LOCAL_DURABLE, ENV_SEAL_SIGNING_KEY,
    TRANSPARENCY_SCHEMA_VERSION,
};
pub use exit_code::ExitCode as VaaExitCode;
pub use generator::{
    aggregate_suite_status, build_and_identify, build_generator, build_patch_evidence,
    check_path_policy, check_paths_against_spec, check_repository, establish_binary_identity,
    expand_generation_command, generate_candidate, git_changed_files, load_generator_spec,
    load_patch_evidence, load_stack_lock, load_suite_manifest, parse_generator_spec,
    parse_stack_lock, parse_suite_manifest, patch_evidence_digest, path_policy_violations,
    resolve_case_paths, resolve_generator_binary, resolve_maybe_relative, resolve_repository_path,
    run_generator_case, run_suite, should_export_repair_packet, stack_lock_digest,
    suite_manifest_digest, triage_status, validate_generator_spec, validate_patch_evidence,
    validate_stack_lock, validate_suite_manifest, verify_patch_evidence_file, write_patch_evidence,
    BuildSpec, CasePaths, ComponentPin, GenerationOutcome, GenerationRequest, GenerationSpec,
    GeneratorBinaryIdentity, GeneratorError, GeneratorPin, GeneratorRepository, GeneratorRunConfig,
    GeneratorRunError, GeneratorRunOutcome, GeneratorSpec, IdentityPolicy, PatchEvidence,
    PatchEvidenceInput, PatchPolicy, PatchStatus, PathPolicyReport, RepoGuardConfig,
    RepoGuardReport, StackLock, StackLockDigest, SuiteCaseResult, SuiteEvidence, SuiteGeneratorRef,
    SuiteManifest, SuitePolicy, SuiteRunConfig, SuiteRunReport, SuiteStatus, ToolchainPin,
    TriageClass, TriageDecision, VerifySummary, GENERATOR_SPEC_SCHEMA_VERSION,
    PATCH_EVIDENCE_SCHEMA_VERSION, STACK_LOCK_SCHEMA_VERSION, SUITE_SCHEMA_VERSION,
};
pub use harness::{
    catalog_for, classify_outcome, default_allowed_operations, enrich_repair_feedback,
    prepare_direct_nasm, prepare_generator_repair, resolve_verify_policy, resume_status,
    select_idioms, serve_stdio, stage_for_failure_code, submit_direct_nasm,
    submit_generator_repair, write_idioms_json, AgentBudget, AgentCommands, AgentDigests,
    AgentEnvelope, AgentMode, AssemblerFlavor, CandidateDelta, FailureDetail, FailureLocation,
    HarnessConfig, HarnessError, HarnessNextAction, HarnessOutcomeClass, HarnessSubmitResult,
    HarnessTemplate, IdiomCatalog, IdiomEntry, IdiomEvidenceLevel, PrepareDirectRequest,
    PrepareGeneratorRequest, ResolvedTargetProfile, ResolvedVerifyPolicy, SubmitDirectRequest,
    SubmitGeneratorRequest, VerifyLevel, AGENT_ENVELOPE_SCHEMA_VERSION,
    HARNESS_SUBMIT_SCHEMA_VERSION, IDIOM_CATALOG_SCHEMA_VERSION,
};
pub use inspect::{ArtifactInfo, ArtifactInspector};
#[cfg(feature = "live-model")]
pub use model::{build_generation_prompt, LiveModelConfig, OpenAiCompatibleAdapter};
pub use model::{
    ArgvExternalGenerator, FixtureModelAdapter, GeneratorJailOpts, ModelAdapter, ModelError,
    ModelResponse, DEFAULT_STAGING_OUTPUT,
};
pub use optimize::{
    collect_candidate_metrics, load_objective, metrics_for_candidate_dir, objective_digest,
    parse_objective_toml, rank_candidates, rank_run_dir, status_label, validate_objective,
    CandidateMetrics, Objective, ObjectiveMetric, OptimizeError, RejectedCandidate,
    SelectedObjectiveView, SelectionEvidence, OBJECTIVE_SCHEMA_VERSION, SELECTION_EVIDENCE_FILE,
};
pub use orchestrate::{MachineState, Orchestrator, StateTransition};
pub use process::{ProcessConfig, ProcessError, ProcessOutput, ProcessRunner};
pub use run::{
    assemble_and_inspect, ingest_candidate, run_fixture_loop, scan_resume_cursor, EventKind,
    EventLog, ResumeCursor, RunConfig, RunDir, RunDirPaths, RunError, RunId, RunOutcome,
    VerifySealOutcome,
};
pub use sandbox::{
    probe_rootless_runtime, write_default_seccomp_profile, ContainerBackend, ExecutionSandbox,
    LocalBackend, SandboxBackend, SandboxConfig, DEFAULT_SECCOMP_PROFILE_JSON,
};
pub use search::{
    mutate_nop_before_ret, mutate_nop_slide, run_search, SearchAttempt, SearchError,
    SearchIngestConfig, SearchReport,
};
pub use semasm::{
    admit_leaf, compare_live_status, load_snapshot, map_acceptance_level, match_task_requirements,
    parse_status_json, probe_live_for_target, snapshot_digest, AdmissionEntry, AdmissionTier,
    CapabilitiesSnapshot, CapabilityMatch, CompareOutcome, DoctorReport, DoctorStatus,
    EvidencePolicy, LiveProbeSummary, LiveStatusCompare, SemasmDoctor, SemasmStatusDocument,
    SemasmVerify, SnapshotAdmission, TargetCapabilities, VerifyError, VerifyReport,
    ADMISSION_SOURCE, CAPABILITY_SNAPSHOT_DIGEST, CAPABILITY_SOURCE,
};
pub use task::{load_locked_task, load_task_file, LockedTask, Task, TaskError};

pub const VAA_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MATURITY: &str = "experimental";
pub const TASK_SCHEMA_VERSION: &str = "0.1";
