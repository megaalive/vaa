pub mod capabilities;
pub mod doctor;
pub mod semantic_evidence;
pub mod status;
pub mod verify;

pub use capabilities::{
    match_task_requirements, CapabilityMatch, TargetCapabilities, CAPABILITY_SOURCE,
};
pub use doctor::{
    probe_live_for_target, semasm_subprocess_allowed_env, DoctorReport, DoctorStatus,
    EvidencePolicy, LiveProbeSummary, SemasmDoctor,
};
pub use semantic_evidence::{
    project_semantic_evidence, AliasEvidenceSummary, ContractExpressionSummary,
    RegionAccessSummary, SemanticEvidenceSummary, VerificationObligationSummary,
};
pub use status::{
    compare_live_status, parse_status_json, CompareOutcome, LiveStatusCompare, SemasmStatusDocument,
};
pub use verify::{SemasmVerify, VerifyError, VerifyReport};
