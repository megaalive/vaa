//! Candidate submission protocol with digest dedup and attempt budgets.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_CANDIDATE_SIZE: u64 = 1_048_576;
const MAX_CANDIDATES_PER_TASK: u32 = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateSubmission {
    pub source_path: PathBuf,
    pub source_content: String,
    pub source_digest: String,
    pub target: String,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RejectionReason {
    SizeExceeded { actual: u64, limit: u64 },
    RepeatedHash { previous_index: u32, digest: String },
    IndexOverflow { index: u32, max: u32 },
    AttemptExhausted { attempts: u32, max: u32 },
    TargetMismatch { expected: String, actual: String },
    InvalidSource(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionOutcome {
    pub accepted: bool,
    pub index: u32,
    pub digest: String,
    pub rejection: Option<RejectionReason>,
}

#[derive(Debug, thiserror::Error)]
pub enum CandidateError {
    #[error("candidate too large: {actual} > {limit}")]
    SizeExceeded { actual: u64, limit: u64 },
    #[error("duplicate candidate (hash collision at index {previous_index})")]
    Duplicate { previous_index: u32, digest: String },
    #[error("index overflow: {index} >= {max}")]
    IndexOverflow { index: u32, max: u32 },
    #[error("attempt budget exhausted: {attempts} >= {max}")]
    AttemptExhausted { attempts: u32, max: u32 },
    #[error("target mismatch: expected {expected}, got {actual}")]
    TargetMismatch { expected: String, actual: String },
    #[error("invalid source: {0}")]
    InvalidSource(String),
}

pub struct CandidateProtocol {
    /// Digest → accepted candidate index.
    seen_hashes: HashMap<String, u32>,
    /// Next accepted-candidate index.
    next_index: u32,
    /// Every submit attempt (accepted or rejected).
    attempt_count: u32,
    target: String,
    max_candidates: u32,
}

impl CandidateProtocol {
    #[must_use]
    pub fn new(target: &str) -> Self {
        Self::with_max(target, MAX_CANDIDATES_PER_TASK)
    }

    #[must_use]
    pub fn with_max(target: &str, max: u32) -> Self {
        Self {
            seen_hashes: HashMap::new(),
            next_index: 0,
            attempt_count: 0,
            target: target.to_owned(),
            max_candidates: max.min(MAX_CANDIDATES_PER_TASK),
        }
    }

    /// Submit a candidate for `candidate_target` (must match protocol target).
    pub fn submit(
        &mut self,
        source: &str,
        source_path: &Path,
        candidate_target: &str,
    ) -> SubmissionOutcome {
        self.attempt_count = self.attempt_count.saturating_add(1);
        let index = self.next_index;

        match self.validate(source, source_path, candidate_target, index) {
            Ok(digest) => {
                self.seen_hashes.insert(digest.clone(), index);
                self.next_index += 1;
                SubmissionOutcome {
                    accepted: true,
                    index,
                    digest,
                    rejection: None,
                }
            }
            Err(e) => SubmissionOutcome {
                accepted: false,
                index,
                digest: String::new(),
                rejection: Some(Self::error_to_rejection(&e)),
            },
        }
    }

    #[must_use]
    pub fn submission_count(&self) -> u32 {
        self.next_index
    }

    #[must_use]
    pub fn attempt_count(&self) -> u32 {
        self.attempt_count
    }

    #[must_use]
    pub fn is_exhausted(&self) -> bool {
        self.attempt_count >= self.max_candidates || self.next_index >= self.max_candidates
    }

    fn validate(
        &self,
        source: &str,
        source_path: &Path,
        candidate_target: &str,
        index: u32,
    ) -> Result<String, CandidateError> {
        if self.attempt_count > self.max_candidates {
            return Err(CandidateError::AttemptExhausted {
                attempts: self.attempt_count,
                max: self.max_candidates,
            });
        }

        if candidate_target != self.target {
            return Err(CandidateError::TargetMismatch {
                expected: self.target.clone(),
                actual: candidate_target.to_owned(),
            });
        }

        if index >= self.max_candidates {
            return Err(CandidateError::IndexOverflow {
                index,
                max: self.max_candidates,
            });
        }

        let size = source.len() as u64;
        if size > MAX_CANDIDATE_SIZE {
            return Err(CandidateError::SizeExceeded {
                actual: size,
                limit: MAX_CANDIDATE_SIZE,
            });
        }

        let digest = Self::hash_source(source);

        if let Some(&prev) = self.seen_hashes.get(&digest) {
            return Err(CandidateError::Duplicate {
                previous_index: prev,
                digest: digest.clone(),
            });
        }

        if source.trim().is_empty() {
            return Err(CandidateError::InvalidSource("source is empty".to_owned()));
        }

        if !source_path.exists() {
            return Err(CandidateError::InvalidSource(format!(
                "path does not exist: {}",
                source_path.display()
            )));
        }

        Ok(digest)
    }

    fn hash_source(source: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(source.as_bytes());
        let hash = hasher.finalize();
        format!("sha256:{}", hex_encode(&hash))
    }

    fn error_to_rejection(error: &CandidateError) -> RejectionReason {
        match error {
            CandidateError::SizeExceeded { actual, limit } => RejectionReason::SizeExceeded {
                actual: *actual,
                limit: *limit,
            },
            CandidateError::Duplicate {
                previous_index,
                digest,
            } => RejectionReason::RepeatedHash {
                previous_index: *previous_index,
                digest: digest.clone(),
            },
            CandidateError::IndexOverflow { index, max } => RejectionReason::IndexOverflow {
                index: *index,
                max: *max,
            },
            CandidateError::AttemptExhausted { attempts, max } => {
                RejectionReason::AttemptExhausted {
                    attempts: *attempts,
                    max: *max,
                }
            }
            CandidateError::TargetMismatch { expected, actual } => {
                RejectionReason::TargetMismatch {
                    expected: expected.clone(),
                    actual: actual.clone(),
                }
            }
            CandidateError::InvalidSource(msg) => RejectionReason::InvalidSource(msg.clone()),
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_submission_accepted() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_first_submission.asm");
        std::fs::write(&path, "section .text\nglobal _start\n").expect("write");
        let mut proto = CandidateProtocol::new("x86_64-unknown-linux-gnu");
        let outcome = proto.submit(
            "section .text\nglobal _start\n",
            &path,
            "x86_64-unknown-linux-gnu",
        );
        assert!(outcome.accepted);
        assert!(outcome.digest.starts_with("sha256:"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn duplicate_submission_reports_original_index() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_duplicate.asm");
        std::fs::write(&path, "mov eax, 1\n").expect("write");
        let mut proto = CandidateProtocol::new("x86_64-unknown-linux-gnu");
        let first = proto.submit("mov eax, 1\n", &path, "x86_64-unknown-linux-gnu");
        assert!(first.accepted);
        let other = dir.join("test_other.asm");
        std::fs::write(&other, "mov eax, 2\n").expect("write");
        assert!(
            proto
                .submit("mov eax, 2\n", &other, "x86_64-unknown-linux-gnu")
                .accepted
        );
        let second = proto.submit("mov eax, 1\n", &path, "x86_64-unknown-linux-gnu");
        assert!(!second.accepted);
        match second.rejection {
            Some(RejectionReason::RepeatedHash {
                previous_index,
                digest,
            }) => {
                assert_eq!(previous_index, 0);
                assert!(digest.starts_with("sha256:"));
            }
            other => panic!("unexpected rejection: {other:?}"),
        }
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&other);
    }

    #[test]
    fn target_mismatch_rejected() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_target.asm");
        std::fs::write(&path, "nop\n").expect("write");
        let mut proto = CandidateProtocol::new("x86_64-pc-windows-msvc");
        let outcome = proto.submit("nop\n", &path, "x86_64-unknown-linux-gnu");
        assert!(!outcome.accepted);
        assert!(matches!(
            outcome.rejection,
            Some(RejectionReason::TargetMismatch { .. })
        ));
        assert_eq!(proto.attempt_count(), 1);
        assert_eq!(proto.submission_count(), 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn empty_source_rejected() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_empty.asm");
        std::fs::write(&path, "").expect("write");
        let mut proto = CandidateProtocol::new("x86_64-unknown-linux-gnu");
        let outcome = proto.submit("", &path, "x86_64-unknown-linux-gnu");
        assert!(!outcome.accepted);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rejected_attempts_consume_budget() {
        let dir = std::env::temp_dir();
        let mut proto = CandidateProtocol::with_max("x86_64-unknown-linux-gnu", 2);
        for i in 0..2 {
            let path = dir.join(format!("test_reject_{i}.asm"));
            std::fs::write(&path, "").expect("write");
            let outcome = proto.submit("", &path, "x86_64-unknown-linux-gnu");
            assert!(!outcome.accepted);
            let _ = std::fs::remove_file(&path);
        }
        assert!(proto.is_exhausted());
    }

    #[test]
    fn max_candidates_enforced() {
        let dir = std::env::temp_dir();
        let mut proto = CandidateProtocol::with_max("x86_64-unknown-linux-gnu", 3);
        for i in 0..3 {
            let path = dir.join(format!("test_max_{i}.asm"));
            std::fs::write(&path, format!("content {i}")).expect("write");
            let outcome = proto.submit(&format!("content {i}"), &path, "x86_64-unknown-linux-gnu");
            assert!(outcome.accepted);
            let _ = std::fs::remove_file(&path);
        }
        assert!(proto.is_exhausted());
    }
}
