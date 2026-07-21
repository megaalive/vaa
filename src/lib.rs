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

pub use build::{BuildManifest, BuildOutcome, BuildPipeline, PipelineConfig};
pub use candidate::{CandidateProtocol, CandidateSubmission, SubmissionOutcome};
pub use canonical_json::{CANONICALIZATION_ID, DIGEST_ALGORITHM_ID};
pub use evidence::{
    sha256_digest_prefixed, verify_bundle, verify_chain, verify_seal, write_sealed_evidence,
    ChainIdentity, CheckOutcome, EvidenceAggregator, EvidenceExpect, EvidenceReport,
    EvidenceStatus, GeneratorMeta, SealEnvelope, SealError, VerifyChainReport,
};
pub use exit_code::ExitCode as VaaExitCode;
pub use harness::{HarnessConfig, HarnessTemplate};
pub use inspect::{ArtifactInfo, ArtifactInspector};
pub use model::{FixtureModelAdapter, ModelAdapter, ModelResponse};
pub use orchestrate::{MachineState, Orchestrator, StateTransition};
pub use process::{ProcessConfig, ProcessError, ProcessOutput, ProcessRunner};
pub use run::{
    ingest_candidate, run_fixture_loop, EventKind, EventLog, RunConfig, RunDir, RunDirPaths,
    RunError, RunId, RunOutcome, VerifySealOutcome,
};
pub use sandbox::{ExecutionSandbox, LocalBackend, SandboxBackend, SandboxConfig};
pub use semasm::{
    match_task_requirements, CapabilityMatch, DoctorReport, DoctorStatus, SemasmDoctor,
    SemasmVerify, TargetCapabilities, VerifyError, VerifyReport,
};
pub use task::{load_locked_task, load_task_file, LockedTask, Task, TaskError};

pub const VAA_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MATURITY: &str = "experimental";
pub const TASK_SCHEMA_VERSION: &str = "0.1";
