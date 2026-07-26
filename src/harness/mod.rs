//! Agent harness façade: prepare / submit / resume / status envelopes.
//!
//! SemASM remains the verifier; VAA owns session, seals, and path policy.
//! This module is a thin orchestration layer over existing CLI surfaces.
//! Assembler flavor defaults to NASM; GAS is reserved and fail-closed until
//! VAA's build/inspect path is dialect-aware.

pub mod assembler;
pub mod envelope;
pub mod feedback;
pub mod session;
pub mod target_profile;
pub mod template;

pub use assembler::AssemblerFlavor;
pub use envelope::{
    default_allowed_operations, AgentBudget, AgentCommands, AgentDigests, AgentEnvelope, AgentMode,
    AGENT_ENVELOPE_SCHEMA_VERSION,
};
pub use feedback::{
    classify_outcome, enrich_repair_feedback, stage_for_failure_code, CandidateDelta,
    FailureDetail, FailureLocation, HarnessNextAction, HarnessOutcomeClass, HarnessSubmitResult,
    HARNESS_SUBMIT_SCHEMA_VERSION,
};
pub use session::{
    prepare_direct_nasm, prepare_generator_repair, resume_status, submit_direct_nasm,
    submit_generator_repair, HarnessError, PrepareDirectRequest, PrepareGeneratorRequest,
    SubmitDirectRequest, SubmitGeneratorRequest,
};
pub use target_profile::{
    embedded_profile, resolve_target_profile, write_target_profile, ResolvedTargetProfile,
};
pub use template::{HarnessConfig, HarnessTemplate};
