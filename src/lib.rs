#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::struct_excessive_bools,
    clippy::too_many_lines
)]

pub mod build;
pub mod cache;
pub mod candidate;
pub mod canonical_json;
pub mod evidence;
pub mod exit_code;
pub mod harness;
pub mod inspect;
pub mod model;
pub mod orchestrate;
pub mod process;
pub mod run;
pub mod sandbox;
pub mod search;
pub mod semasm;
pub mod task;

pub use build::{
    check_reproducible, compare_canonical, probe_container_runtime, remap_host_args_to_container,
    reproducible_build_check, tool_digest, BuildManifest, BuildOutcome, BuildPipeline,
    CanonicalBuildView, ContainerBuildOpts, PipelineConfig, ReproReport, DEFAULT_CONTAINER_IMAGE,
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
pub use harness::{HarnessConfig, HarnessTemplate};
pub use inspect::{ArtifactInfo, ArtifactInspector};
#[cfg(feature = "live-model")]
pub use model::{build_generation_prompt, LiveModelConfig, OpenAiCompatibleAdapter};
pub use model::{
    ArgvExternalGenerator, FixtureModelAdapter, GeneratorJailOpts, ModelAdapter, ModelError,
    ModelResponse, DEFAULT_STAGING_OUTPUT,
};
pub use orchestrate::{MachineState, Orchestrator, StateTransition};
pub use process::{ProcessConfig, ProcessError, ProcessOutput, ProcessRunner};
pub use run::{
    assemble_and_inspect, ingest_candidate, run_fixture_loop, EventKind, EventLog, ResumeCursor,
    RunConfig, RunDir, RunDirPaths, RunError, RunId, RunOutcome, VerifySealOutcome,
};
pub use sandbox::{
    probe_rootless_runtime, write_default_seccomp_profile, ContainerBackend, ExecutionSandbox,
    LocalBackend, SandboxBackend, SandboxConfig, DEFAULT_SECCOMP_PROFILE_JSON,
};
pub use search::{mutate_nop_slide, run_search, SearchAttempt, SearchError, SearchReport};
pub use semasm::{
    compare_live_status, match_task_requirements, parse_status_json, probe_live_for_target,
    CapabilityMatch, CompareOutcome, DoctorReport, DoctorStatus, EvidencePolicy, LiveProbeSummary,
    LiveStatusCompare, SemasmDoctor, SemasmStatusDocument, SemasmVerify, TargetCapabilities,
    VerifyError, VerifyReport, CAPABILITY_SOURCE,
};
pub use task::{load_locked_task, load_task_file, LockedTask, Task, TaskError};

pub const VAA_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MATURITY: &str = "experimental";
pub const TASK_SCHEMA_VERSION: &str = "0.1";
