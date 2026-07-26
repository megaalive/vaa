//! Agent harness façade: prepare / submit / resume / status envelopes.
//!
//! SemASM remains the verifier; VAA owns session, seals, and path policy.
//! This module is a thin orchestration layer over existing CLI surfaces.
//! Assembler flavor defaults to NASM; GAS is reserved and fail-closed until
//! VAA's build/inspect path is dialect-aware.

pub mod assembler;
pub mod envelope;
pub mod feedback;
pub mod idioms;
pub mod session;
pub mod stdio_serve;
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
pub use idioms::{
    catalog_for, embedded_catalog, select_idioms, write_idioms_json, IdiomCatalog, IdiomEntry,
    IdiomEvidenceLevel, IDIOM_CATALOG_SCHEMA_VERSION,
};
pub use session::{
    enforce_level_honesty, prepare_direct_nasm, prepare_generator_repair, resolve_verify_policy,
    resume_status, submit_direct_nasm, submit_generator_repair, HarnessError, PrepareDirectRequest,
    PrepareGeneratorRequest, ResolvedVerifyPolicy, SubmitDirectRequest, SubmitGeneratorRequest,
    VerifyLevel,
};
pub use stdio_serve::{
    parse_request_line, serve_stdio, serve_stdio_with, AgentServeSession, ServeError,
};
pub use target_profile::{
    embedded_profile, resolve_target_profile, write_target_profile, ResolvedTargetProfile,
};
pub use template::{HarnessConfig, HarnessTemplate};
