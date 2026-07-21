pub mod report;
pub mod seal;
pub mod status;

pub use report::{
    schema_version_compatible, sha256_digest_prefixed, CheckOutcome, EvidenceAggregator,
    EvidenceExpect, EvidenceReport,
};
pub use seal::{
    build_seal_payload, seal_digest_of, seal_envelope, verify_seal, write_sealed_evidence,
    GeneratorMeta, SealEnvelope, SealError, SealPayloadV01, SEAL_SCHEMA_VERSION,
};
pub use status::EvidenceStatus;
