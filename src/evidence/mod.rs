pub mod bundle;
pub mod report;
pub mod seal;
pub mod status;

pub use bundle::{
    materialize_bundle_files, verify_bundle, BUNDLE_CONTRACT, BUNDLE_EVIDENCE, BUNDLE_REPORT,
    BUNDLE_SEAL, BUNDLE_SOURCE, BUNDLE_TASK,
};
pub use report::{
    schema_version_compatible, sha256_digest_prefixed, CheckOutcome, EvidenceAggregator,
    EvidenceExpect, EvidenceReport,
};
pub use seal::{
    acceptance_digest_of, build_seal_envelope, envelope_digest_of, seal_envelope, verify_seal,
    write_final_sealed_evidence, write_sealed_evidence, AcceptanceBody, GeneratorMeta,
    ProvenanceBody, SealBuildInput, SealEnvelope, SealError, SEAL_SCHEMA_VERSION,
};
pub use status::EvidenceStatus;
