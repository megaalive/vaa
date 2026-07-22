#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::struct_excessive_bools,
    clippy::too_many_lines
)]

pub mod build;
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
pub mod semasm;
pub mod task;

pub use build::{
    probe_container_runtime, BuildManifest, BuildOutcome, BuildPipeline, ContainerBuildOpts,
    PipelineConfig, DEFAULT_CONTAINER_IMAGE,
};
pub use candidate::{CandidateProtocol, CandidateSubmission, SubmissionOutcome};
pub use canonical_json::{CANONICALIZATION_ID, DIGEST_ALGORITHM_ID};
pub use evidence::{
    export_transparency, keygen_seal, sha256_digest_prefixed, verify_bundle, verify_chain,
    verify_seal, verify_transparency_against_run, write_sealed_evidence, write_transparency_file,
    ChainIdentity, CheckOutcome, EvidenceAggregator, EvidenceExpect, EvidenceReport,
    EvidenceStatus, GeneratorMeta, SealEnvelope, SealError, SealSignature, TransparencyDocument,
    VerifyChainReport, TRANSPARENCY_SCHEMA_VERSION,
};
pub use exit_code::ExitCode as VaaExitCode;
pub use harness::{HarnessConfig, HarnessTemplate};
pub use inspect::{ArtifactInfo, ArtifactInspector};
pub use model::{
    ArgvExternalGenerator, FixtureModelAdapter, ModelAdapter, ModelError, ModelResponse,
    DEFAULT_STAGING_OUTPUT,
};
pub use orchestrate::{MachineState, Orchestrator, StateTransition};
pub use process::{ProcessConfig, ProcessError, ProcessOutput, ProcessRunner};
pub use run::{
    assemble_and_inspect, ingest_candidate, run_fixture_loop, EventKind, EventLog, ResumeCursor,
    RunConfig, RunDir, RunDirPaths, RunError, RunId, RunOutcome, VerifySealOutcome,
};
pub use sandbox::{
    ContainerBackend, ExecutionSandbox, LocalBackend, SandboxBackend, SandboxConfig,
};
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
