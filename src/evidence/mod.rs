pub mod bundle;
pub mod chain;
pub mod durability;
#[cfg(feature = "pkcs11")]
pub mod pkcs11_softhsm;
pub mod rekor;
pub mod report;
pub mod seal;
pub mod seal_log;
pub mod seal_sign;
pub mod signer;
pub mod status;
pub mod transparency;

pub use bundle::{
    materialize_bundle_files, verify_bundle, BUNDLE_CONTRACT, BUNDLE_EVIDENCE, BUNDLE_REPORT,
    BUNDLE_SEAL, BUNDLE_SOURCE, BUNDLE_TASK,
};
pub use chain::{verify_chain, ChainIdentity, VerifyChainReport};
pub use durability::{
    may_claim_verified, probe_durability, publish_files_seal_last, DurabilityClass,
    DurabilityProbeReport, ENV_REQUIRE_LOCAL_DURABLE,
};
#[cfg(feature = "pkcs11")]
pub use pkcs11_softhsm::{
    provision_rsa_keypair, sign_acceptance_digest_rsa, verify_rsa_pkcs1_sha256_spki,
};
#[cfg(feature = "rekor")]
pub use rekor::UreqRekorTransport;
pub use rekor::{
    hashedrekord_from_dsse, publish_dsse, transparency_payload_bytes, verify_entry_matches_dsse,
    MockRekorTransport, RekorError, RekorPublishResult, RekorTransport,
};
pub use report::{
    schema_version_compatible, sha256_digest_prefixed, CheckOutcome, EvidenceAggregator,
    EvidenceExpect, EvidenceReport, ObjectInspectionOutcome, ReproducibleBuildOutcome,
};
pub use seal::{
    acceptance_digest_of, build_seal_envelope, envelope_digest_of, seal_envelope, verify_seal,
    write_final_sealed_evidence, write_sealed_evidence, AcceptanceBody, GeneratorMeta,
    ProvenanceBody, SealBuildInput, SealEnvelope, SealError, SEAL_SCHEMA_VERSION,
};
pub use seal_log::{
    append_seal_log, read_seal_log, verify_seal_log_against_digests, SealLogEntry, SEAL_LOG_NAME,
    SEAL_LOG_SCHEMA_VERSION,
};
pub use seal_sign::{
    keygen_seal, maybe_sign_envelope, verify_envelope_signature, SealSignature,
    ENV_REQUIRE_SEAL_SIGNATURE, ENV_SEAL_SIGNING_KEY, SIGNATURE_ALG,
    SIGNATURE_ALG_RSA_PKCS1_SHA256, SIGNED_OVER_ACCEPTANCE,
};
pub use signer::{
    dsse_pae, signer_from_env, verify_dsse_envelope, write_dsse_file, DsseEnvelope, DsseSignature,
    HsmPkcs11Signer, PracticeEd25519Signer, SealSigner, SignerKind, SigstoreDsseSigner,
    DSSE_PAYLOAD_TYPE_TRANSPARENCY,
};
pub use status::EvidenceStatus;
pub use transparency::{
    export_transparency, read_transparency_file, verify_transparency_against_run,
    write_transparency_file, TransparencyDocument, TRANSPARENCY_SCHEMA_VERSION,
};
