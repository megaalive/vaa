//! Fail-closed reuse policy for cache hits (§17.2 / §C-011).

use crate::evidence::EvidenceStatus;

use super::store::{BuildCacheRecord, VerificationCacheRecord};

/// Outcome of asking whether a cache record may be reused for the current run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheReuseDecision {
    Allow,
    Deny(&'static str),
}

/// Verification reuse: never promote Incomplete/Failed to Verified; require intact record.
#[must_use]
pub fn may_reuse_verification(
    record: &VerificationCacheRecord,
    require_verified: bool,
) -> CacheReuseDecision {
    if record.schema_version != super::store::CACHE_SCHEMA_VERSION {
        return CacheReuseDecision::Deny("schema incompatible");
    }
    if record.report_blob_digest.is_empty() {
        return CacheReuseDecision::Deny("missing report blob digest");
    }
    let status = parse_status(&record.final_status);
    if matches!(status, EvidenceStatus::Failed) {
        return CacheReuseDecision::Deny("previous terminal status was failed");
    }
    if require_verified && !matches!(status, EvidenceStatus::Verified) {
        return CacheReuseDecision::Deny("incomplete evidence is not reused as verified");
    }
    CacheReuseDecision::Allow
}

/// Build reuse: schema + blob pointers must be present.
#[must_use]
pub fn may_reuse_build(record: &BuildCacheRecord) -> CacheReuseDecision {
    if record.schema_version != super::store::CACHE_SCHEMA_VERSION {
        return CacheReuseDecision::Deny("schema incompatible");
    }
    if record.object_blob_digest.is_empty() {
        return CacheReuseDecision::Deny("missing object blob");
    }
    CacheReuseDecision::Allow
}

fn parse_status(s: &str) -> EvidenceStatus {
    match s {
        "Verified" => EvidenceStatus::Verified,
        "Violated" => EvidenceStatus::Violated,
        "Incomplete" => EvidenceStatus::Incomplete,
        _ => EvidenceStatus::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::store::CACHE_SCHEMA_VERSION;

    fn ver_rec(status: &str) -> VerificationCacheRecord {
        VerificationCacheRecord {
            schema_version: CACHE_SCHEMA_VERSION.to_owned(),
            key_digest: "sha256:k".into(),
            final_status: status.into(),
            report_blob_digest: "sha256:r".into(),
            raw_status: Some("verified".into()),
        }
    }

    #[test]
    fn rejects_failed_and_incomplete_as_verified() {
        assert!(matches!(
            may_reuse_verification(&ver_rec("Failed"), true),
            CacheReuseDecision::Deny(_)
        ));
        assert!(matches!(
            may_reuse_verification(&ver_rec("Incomplete"), true),
            CacheReuseDecision::Deny(_)
        ));
        assert_eq!(
            may_reuse_verification(&ver_rec("Verified"), true),
            CacheReuseDecision::Allow
        );
        assert_eq!(
            may_reuse_verification(&ver_rec("Incomplete"), false),
            CacheReuseDecision::Allow
        );
    }

    #[test]
    fn rejects_build_without_object_blob() {
        let rec = BuildCacheRecord {
            schema_version: CACHE_SCHEMA_VERSION.to_owned(),
            key_digest: "sha256:k".into(),
            object_blob_digest: String::new(),
            binary_blob_digest: None,
            manifest_json: "{}".into(),
        };
        assert!(matches!(may_reuse_build(&rec), CacheReuseDecision::Deny(_)));
    }
}
