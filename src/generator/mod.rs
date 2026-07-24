//! External generator bridge types (stack lock + generator spec).
//!
//! Generator-agnostic VAA core for the verified-repair bridge plan.
//! HlaX64 is an instance pack under `integrations/`, not a VAA core dependency.

mod error;
mod generate;
mod identity;
mod repo_guard;
mod run;
mod spec;
mod stack_lock;
mod suite;

pub use error::GeneratorError;
pub use generate::{
    expand_generation_command, generate_candidate, GenerationOutcome, GenerationRequest,
};
pub use identity::{
    build_and_identify, build_generator, establish_binary_identity, resolve_generator_binary,
    GeneratorBinaryIdentity,
};
pub use repo_guard::{
    check_repository, glob_match, path_policy_violations, resolve_repository_path, RepoGuardConfig,
    RepoGuardReport,
};
pub use run::{
    resolve_maybe_relative, run_generator_case, GeneratorRunConfig, GeneratorRunError,
    GeneratorRunOutcome, VerifySummary,
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
