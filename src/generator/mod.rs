//! External generator bridge types (stack lock + generator spec).
//!
//! Generator-agnostic VAA core for the verified-repair bridge plan.
//! HlaX64 is an instance pack under `integrations/`, not a VAA core dependency.

mod error;
mod repo_guard;
mod spec;
mod stack_lock;

pub use error::GeneratorError;
pub use repo_guard::{
    check_repository, glob_match, path_policy_violations, resolve_repository_path, RepoGuardConfig,
    RepoGuardReport,
};
pub use spec::{
    load_generator_spec, parse_generator_spec, validate_generator_spec, BuildSpec, GenerationSpec,
    GeneratorRepository, GeneratorSpec, IdentityPolicy, PatchPolicy, GENERATOR_SPEC_SCHEMA_VERSION,
};
pub use stack_lock::{
    load_stack_lock, parse_stack_lock, stack_lock_digest, validate_stack_lock, ComponentPin,
    GeneratorPin, StackLock, StackLockDigest, ToolchainPin, STACK_LOCK_SCHEMA_VERSION,
};
