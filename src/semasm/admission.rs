//! Capability admission freeze: checked-in SemASM capabilities snapshot.
//!
//! Agents / controllers consult this registry for which leaf×target×assembler
//! tuples VAA may admit. The skill allowlist
//! (`schemas/agent-leaf-allowlist.json`) remains the skill gate until admission
//! fully replaces it; protocol freeze requires both list the same leaf_names.

use serde::{Deserialize, Serialize};

/// Path of the checked-in SemASM capabilities JSON (relative to repo root).
pub const ADMISSION_SOURCE: &str = "fixtures/semasm/capabilities-snapshot.json";

/// Digest pinned from SemASM `capabilities --format json` at freeze time.
/// Must match the `digest` field inside [`ADMISSION_SOURCE`].
pub const CAPABILITY_SNAPSHOT_DIGEST: &str =
    "sha256:94bda0b92b69d12360e757451478b07c7fdcff317a50e7f3f8d3677f33d13e05";

const SNAPSHOT_JSON: &str = include_str!("../../fixtures/semasm/capabilities-snapshot.json");

/// VAA admission tiers mapped from SemASM acceptance / authoring levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionTier {
    /// Draft / author paths only — not agent-verify.
    AuthoringOnly,
    /// Static gates without behavioral acceptance.
    StaticAnalysis,
    /// Behavioral acceptance (`verified` / VUP) without seal authority claim.
    BehavioralAcceptance,
    /// Behavioral acceptance plus seal when controller policy allows.
    SealedAcceptance,
}

impl AdmissionTier {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AuthoringOnly => "authoring_only",
            Self::StaticAnalysis => "static_analysis",
            Self::BehavioralAcceptance => "behavioral_acceptance",
            Self::SealedAcceptance => "sealed_acceptance",
        }
    }
}

/// One `[[admission]]` row from the SemASM capabilities export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotAdmission {
    pub capability_id: String,
    #[serde(default)]
    pub leaf_names: Vec<String>,
    #[serde(default)]
    pub targets: Vec<String>,
    #[serde(default)]
    pub assemblers: Vec<String>,
    #[serde(default)]
    pub acceptance_level: String,
    #[serde(default)]
    pub authoring_level: String,
    #[serde(default)]
    pub oracles: Vec<String>,
    #[serde(default)]
    pub parameters: Vec<String>,
    #[serde(default)]
    pub returns: Vec<String>,
    #[serde(default)]
    pub required_gates: Vec<String>,
}

/// Admitted leaf entry with VAA tier attached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionEntry {
    #[serde(flatten)]
    pub snapshot: SnapshotAdmission,
    pub tier: AdmissionTier,
}

/// Full frozen SemASM capabilities document (subset used by VAA).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitiesSnapshot {
    pub name: String,
    pub version: String,
    pub capability_schema: String,
    pub digest: String,
    #[serde(default)]
    pub admission: Vec<SnapshotAdmission>,
    #[serde(default)]
    pub targets: Vec<serde_json::Value>,
    #[serde(default)]
    pub workspace_crates: Vec<String>,
}

/// Map SemASM `acceptance_level` → VAA [`AdmissionTier`].
///
/// `verified` / `verified_under_preconditions` become
/// [`AdmissionTier::BehavioralAcceptance`], or
/// [`AdmissionTier::SealedAcceptance`] when `allow_sealed` is true.
/// `authoring_only` stays authoring-only.
#[must_use]
pub fn map_acceptance_level(acceptance_level: &str, allow_sealed: bool) -> AdmissionTier {
    match acceptance_level {
        "static" | "static_analysis" => AdmissionTier::StaticAnalysis,
        "verified" | "verified_under_preconditions" => {
            if allow_sealed {
                AdmissionTier::SealedAcceptance
            } else {
                AdmissionTier::BehavioralAcceptance
            }
        }
        // `authoring_only` and unknown levels stay authoring-only (fail closed).
        _ => AdmissionTier::AuthoringOnly,
    }
}

/// Load the checked-in capabilities snapshot (compile-time include).
#[must_use]
pub fn load_snapshot() -> CapabilitiesSnapshot {
    serde_json::from_str(SNAPSHOT_JSON)
        .expect("fixtures/semasm/capabilities-snapshot.json must parse as CapabilitiesSnapshot")
}

/// Digest recorded on the frozen snapshot (must equal [`CAPABILITY_SNAPSHOT_DIGEST`]).
#[must_use]
pub fn snapshot_digest() -> &'static str {
    CAPABILITY_SNAPSHOT_DIGEST
}

/// Admit a leaf×target×assembler triple from the frozen snapshot.
///
/// Returns [`None`] when no admission row lists the leaf on that target with
/// the requested assembler. Tier defaults to behavioral (not sealed); callers
/// that seal may re-map via [`map_acceptance_level`].
#[must_use]
pub fn admit_leaf(name: &str, target: &str, assembler: &str) -> Option<AdmissionEntry> {
    let snap = load_snapshot();
    snap.admission.into_iter().find_map(|row| {
        let leaf_ok = row.leaf_names.iter().any(|n| n == name);
        let target_ok = row.targets.iter().any(|t| t == target);
        let asm_ok = row.assemblers.iter().any(|a| a == assembler);
        if leaf_ok && target_ok && asm_ok {
            let tier = map_acceptance_level(&row.acceptance_level, false);
            Some(AdmissionEntry {
                tier,
                snapshot: row,
            })
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_digest_matches_constant() {
        let snap = load_snapshot();
        assert_eq!(snap.digest, CAPABILITY_SNAPSHOT_DIGEST);
        assert_eq!(snapshot_digest(), CAPABILITY_SNAPSHOT_DIGEST);
        assert!(CAPABILITY_SNAPSHOT_DIGEST.starts_with("sha256:"));
        assert_eq!(CAPABILITY_SNAPSHOT_DIGEST.len(), "sha256:".len() + 64);
    }

    #[test]
    fn admit_max_i64_win64_nasm() {
        let entry = admit_leaf("max_i64", "x86_64-pc-windows-msvc", "nasm")
            .expect("max_i64 must be admitted");
        assert_eq!(entry.snapshot.acceptance_level, "verified");
        assert_eq!(entry.tier, AdmissionTier::BehavioralAcceptance);
        assert_eq!(
            map_acceptance_level("verified", true),
            AdmissionTier::SealedAcceptance
        );
    }

    #[test]
    fn admit_count_byte_linux() {
        let entry = admit_leaf("count_byte_linux", "x86_64-unknown-linux-gnu", "nasm")
            .expect("count_byte_linux must be admitted");
        assert_eq!(
            entry.snapshot.acceptance_level,
            "verified_under_preconditions"
        );
        assert_eq!(entry.tier, AdmissionTier::BehavioralAcceptance);
    }

    #[test]
    fn unknown_leaf_is_not_admitted() {
        assert!(admit_leaf("strlen", "x86_64-pc-windows-msvc", "nasm").is_none());
    }

    #[test]
    fn authoring_only_maps_to_authoring_tier() {
        assert_eq!(
            map_acceptance_level("authoring_only", true),
            AdmissionTier::AuthoringOnly
        );
    }
}
