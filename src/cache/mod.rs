//! Local content-addressed cache (PR-020). Layout: `.vaa/cache/`.
//!
//! Verification and build stores only — never prompt-only binary reuse.

mod keys;
mod policy;
mod store;

pub use keys::{
    args_fingerprint, build_cache_key, verification_cache_key, BuildKeyMaterials,
    VerificationKeyMaterials,
};
pub use policy::{may_reuse_build, may_reuse_verification, CacheReuseDecision};
pub use store::{
    default_cache_root, resolve_cache_root, BuildCacheArtifacts, BuildCacheRecord, CacheError,
    CacheStats, CacheStore, VerificationCacheRecord, CACHE_SCHEMA_VERSION,
};
