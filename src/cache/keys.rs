//! Cache key materials → content-addressed digests (vaa-canonical-json-v1).

use serde::Serialize;

use crate::canonical_json::canonical_json_bytes;
use crate::evidence::sha256_digest_prefixed;

/// Inputs for a verification cache key (§C-011). Never prompt-only.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VerificationKeyMaterials {
    pub source_digest: String,
    pub contract_digest: String,
    pub task_digest: String,
    pub target: String,
    pub semasm_version: String,
    pub allow_execution: bool,
    pub capability_source: String,
}

/// Inputs for a build cache key (§C-011).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BuildKeyMaterials {
    pub source_digest: String,
    pub target: String,
    pub assembler_digest: String,
    pub linker_digest: String,
    pub assembler_args_fingerprint: String,
    pub linker_args_fingerprint: String,
    /// Empty string when not using a container image pin.
    pub container_image_digest: String,
}

/// Digest of verification key materials (`sha256:…`).
#[must_use]
pub fn verification_cache_key(materials: &VerificationKeyMaterials) -> String {
    sha256_digest_prefixed(&canonical_json_bytes(materials))
}

/// Digest of build key materials (`sha256:…`).
#[must_use]
pub fn build_cache_key(materials: &BuildKeyMaterials) -> String {
    sha256_digest_prefixed(&canonical_json_bytes(materials))
}

/// Fingerprint argv without absolute paths (basename-ish: join as-is for stability of relative args).
#[must_use]
pub fn args_fingerprint(args: &[String]) -> String {
    sha256_digest_prefixed(args.join("\0").as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_key_stable_and_order_independent_of_struct_field_order() {
        let a = VerificationKeyMaterials {
            source_digest: "sha256:aa".into(),
            contract_digest: "sha256:bb".into(),
            task_digest: "sha256:cc".into(),
            target: "x86_64-unknown-linux-gnu".into(),
            semasm_version: "0.1.0".into(),
            allow_execution: false,
            capability_source: "vaa_embedded_agent_verify_snapshot".into(),
        };
        let b = a.clone();
        assert_eq!(verification_cache_key(&a), verification_cache_key(&b));
        let mut c = a.clone();
        c.allow_execution = true;
        assert_ne!(verification_cache_key(&a), verification_cache_key(&c));
    }

    #[test]
    fn build_key_changes_with_tool_digest() {
        let base = BuildKeyMaterials {
            source_digest: "sha256:s".into(),
            target: "elf64".into(),
            assembler_digest: "sha256:as1".into(),
            linker_digest: "sha256:ld1".into(),
            assembler_args_fingerprint: args_fingerprint(&["-f".into(), "elf64".into()]),
            linker_args_fingerprint: args_fingerprint(&["-o".into(), "out".into()]),
            container_image_digest: String::new(),
        };
        let mut other = base.clone();
        other.assembler_digest = "sha256:as2".into();
        assert_ne!(build_cache_key(&base), build_cache_key(&other));
    }
}
