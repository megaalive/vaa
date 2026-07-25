//! Agent harness façade: prepare / submit / resume / status envelopes.
//!
//! SemASM remains the verifier; VAA owns session, seals, and path policy.
//! This module is a thin orchestration layer over existing CLI surfaces.

pub mod envelope;
pub mod feedback;
pub mod session;
pub mod template;

pub use envelope::{
    AgentBudget, AgentCommands, AgentDigests, AgentEnvelope, AgentMode, AGENT_ENVELOPE_SCHEMA_VERSION,
};
pub use feedback::{
    classify_outcome, HarnessNextAction, HarnessOutcomeClass, HarnessSubmitResult,
    HARNESS_SUBMIT_SCHEMA_VERSION,
};
pub use session::{
    prepare_direct_nasm, prepare_generator_repair, resume_status, submit_direct_nasm, HarnessError,
    PrepareDirectRequest, PrepareGeneratorRequest, SubmitDirectRequest,
};
pub use template::{HarnessConfig, HarnessTemplate};
