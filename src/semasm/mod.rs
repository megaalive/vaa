pub mod admission;
pub mod capabilities;
pub mod doctor;
pub mod semantic_evidence;
pub mod status;
pub mod verify;

pub use admission::{
    admit_leaf, load_snapshot, map_acceptance_level, snapshot_digest, AdmissionEntry,
    AdmissionTier, CapabilitiesSnapshot, SnapshotAdmission, ADMISSION_SOURCE,
    CAPABILITY_SNAPSHOT_DIGEST,
};
pub use capabilities::{
    match_task_requirements, CapabilityMatch, TargetCapabilities, CAPABILITY_SOURCE,
};
pub use doctor::{
    probe_live_for_target, semasm_subprocess_allowed_env, DoctorReport, DoctorStatus,
    EvidencePolicy, LiveProbeSummary, SemasmDoctor, ENV_SEMASM_BIN,
};
pub use semantic_evidence::{
    project_semantic_evidence, AliasEvidenceSummary, ContractExpressionSummary,
    RegionAccessSummary, SemanticEvidenceSummary, VerificationObligationSummary,
};
pub use status::{
    compare_live_status, parse_status_json, CompareOutcome, LiveStatusCompare, SemasmStatusDocument,
};
pub use verify::{SemasmVerify, VerifyError, VerifyReport};
