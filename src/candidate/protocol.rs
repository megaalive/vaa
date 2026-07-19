use std::collections::HashSet;
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
    Duplicate { previous_index: u32 },
    #[error("index overflow: {index} >= {max}")]
    IndexOverflow { index: u32, max: u32 },
    #[error("target mismatch: expected {expected}, got {actual}")]
    TargetMismatch { expected: String, actual: String },
    #[error("invalid source: {0}")]
    InvalidSource(String),
}

#[allow(dead_code)]
pub struct CandidateProtocol {
    seen_hashes: HashSet<String>,
    next_index: u32,
    target: String,
    max_candidates: u32,
}

impl CandidateProtocol {
    #[must_use]
    pub fn new(target: &str) -> Self {
        Self {
            seen_hashes: HashSet::new(),
            next_index: 0,
            target: target.to_owned(),
            max_candidates: MAX_CANDIDATES_PER_TASK,
        }
    }

    #[must_use]
    pub fn with_max(target: &str, max: u32) -> Self {
        Self {
            seen_hashes: HashSet::new(),
            next_index: 0,
            target: target.to_owned(),
            max_candidates: max.min(MAX_CANDIDATES_PER_TASK),
        }
    }

    pub fn submit(&mut self, source: &str, source_path: &Path) -> SubmissionOutcome {
        let index = self.next_index;

        match self.validate(source, source_path, index) {
            Ok(digest) => {
                self.seen_hashes.insert(digest.clone());
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
    pub fn is_exhausted(&self) -> bool {
        self.next_index >= self.max_candidates
    }

    fn validate(
        &mut self,
        source: &str,
        source_path: &Path,
        index: u32,
    ) -> Result<String, CandidateError> {
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

        if let Some(prev) = self.find_previous(&digest) {
            return Err(CandidateError::Duplicate {
                previous_index: prev,
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

    fn find_previous(&self, digest: &str) -> Option<u32> {
        if self.seen_hashes.contains(digest) {
            Some(self.seen_hashes.len() as u32 - 1)
        } else {
            None
        }
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
            CandidateError::Duplicate { previous_index } => RejectionReason::RepeatedHash {
                previous_index: *previous_index,
                digest: String::new(),
            },
            CandidateError::IndexOverflow { index, max } => RejectionReason::IndexOverflow {
                index: *index,
                max: *max,
            },
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
        let outcome = proto.submit("section .text\nglobal _start\n", &path);
        assert!(outcome.accepted);
        assert!(outcome.digest.starts_with("sha256:"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn duplicate_submission_rejected() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_duplicate.asm");
        std::fs::write(&path, "mov eax, 1\n").expect("write");
        let mut proto = CandidateProtocol::new("x86_64-unknown-linux-gnu");
        let first = proto.submit("mov eax, 1\n", &path);
        assert!(first.accepted);
        let second = proto.submit("mov eax, 1\n", &path);
        assert!(!second.accepted);
        assert!(matches!(
            second.rejection,
            Some(RejectionReason::RepeatedHash { .. })
        ));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn empty_source_rejected() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_empty.asm");
        std::fs::write(&path, "").expect("write");
        let mut proto = CandidateProtocol::new("x86_64-unknown-linux-gnu");
        let outcome = proto.submit("", &path);
        assert!(!outcome.accepted);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn max_candidates_enforced() {
        let dir = std::env::temp_dir();
        let mut proto = CandidateProtocol::with_max("x86_64-unknown-linux-gnu", 3);
        for i in 0..3 {
            let path = dir.join(format!("test_max_{i}.asm"));
            std::fs::write(&path, format!("content {i}")).expect("write");
            let outcome = proto.submit(&format!("content {i}"), &path);
            assert!(outcome.accepted);
            let _ = std::fs::remove_file(&path);
        }
        assert!(proto.is_exhausted());
    }
}
