use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::run::run_id::RunId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunDirPaths {
    pub root: PathBuf,
    pub task_dir: PathBuf,
    pub target_dir: PathBuf,
    pub candidates_dir: PathBuf,
    pub accepted_dir: PathBuf,
    pub evidence_dir: PathBuf,
    pub event_log_path: PathBuf,
}

#[derive(Debug)]
pub struct RunDir {
    paths: RunDirPaths,
}

#[derive(Debug, thiserror::Error)]
pub enum RunDirError {
    #[error("failed to create run directory `{path}`: {source}")]
    Create {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write file `{path}`: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("path traversal detected in `{path}`")]
    PathTraversal { path: PathBuf },

    #[error("candidate index overflow")]
    CandidateOverflow,
}

impl RunDir {
    pub fn create(base: &Path, id: &RunId) -> Result<Self, RunDirError> {
        let root = Self::safe_join(base, id.dir_name())?;

        let paths = RunDirPaths {
            task_dir: root.join("task"),
            target_dir: root.join("target"),
            candidates_dir: root.join("candidates"),
            accepted_dir: root.join("accepted"),
            evidence_dir: root.join("evidence"),
            event_log_path: root.join("events.jsonl"),
            root,
        };

        let dirs = [
            &paths.root,
            &paths.task_dir,
            &paths.target_dir,
            &paths.candidates_dir,
            &paths.accepted_dir,
            &paths.evidence_dir,
        ];

        for dir in &dirs {
            fs::create_dir_all(dir).map_err(|source| RunDirError::Create {
                path: (*dir).clone(),
                source,
            })?;
        }

        Ok(Self { paths })
    }

    #[must_use]
    pub fn paths(&self) -> &RunDirPaths {
        &self.paths
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.paths.root
    }

    #[must_use]
    pub fn event_log_path(&self) -> &Path {
        &self.paths.event_log_path
    }

    pub fn candidate_dir(&self, index: u32) -> Result<PathBuf, RunDirError> {
        if index > 9999 {
            return Err(RunDirError::CandidateOverflow);
        }
        let name = format!("{index:04}");
        Ok(self.paths.candidates_dir.join(name))
    }

    pub fn create_candidate_dir(&self, index: u32) -> Result<PathBuf, RunDirError> {
        if index > 9999 {
            return Err(RunDirError::CandidateOverflow);
        }
        let name = format!("{index:04}");
        let dir = self.paths.candidates_dir.join(&name);
        fs::create_dir_all(&dir).map_err(|source| RunDirError::Create {
            path: dir.clone(),
            source,
        })?;

        let marker = dir.join(".immutable");
        if let Err(source) = fs::write(&marker, b"") {
            return Err(RunDirError::Write {
                path: marker,
                source,
            });
        }

        Ok(dir)
    }

    pub fn write_atomic(&self, path: &Path, data: &[u8]) -> Result<(), RunDirError> {
        if !path.starts_with(&self.paths.root) {
            return Err(RunDirError::PathTraversal {
                path: path.to_path_buf(),
            });
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| RunDirError::Create {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let tmp_path = path.with_extension("tmp");
        {
            let mut tmp = tempfile(path).map_err(|source| RunDirError::Write {
                path: tmp_path.clone(),
                source,
            })?;
            tmp.write_all(data).map_err(|source| RunDirError::Write {
                path: tmp_path.clone(),
                source,
            })?;
            tmp.flush().map_err(|source| RunDirError::Write {
                path: tmp_path.clone(),
                source,
            })?;
        }

        fs::rename(&tmp_path, path).map_err(|source| RunDirError::Write {
            path: path.to_path_buf(),
            source,
        })?;

        Ok(())
    }

    pub fn write_task_file(&self, filename: &str, data: &[u8]) -> Result<PathBuf, RunDirError> {
        let safe_name = sanitize_filename(filename);
        let path = self.paths.task_dir.join(safe_name);
        self.write_atomic(&path, data)?;
        Ok(path)
    }

    pub fn write_evidence_file(&self, filename: &str, data: &[u8]) -> Result<PathBuf, RunDirError> {
        let safe_name = sanitize_filename(filename);
        let path = self.paths.evidence_dir.join(safe_name);
        self.write_atomic(&path, data)?;
        Ok(path)
    }

    fn safe_join(base: &Path, component: &str) -> Result<PathBuf, RunDirError> {
        let sanitized: String = component
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        if sanitized.is_empty() {
            return Err(RunDirError::PathTraversal {
                path: base.to_path_buf(),
            });
        }

        let joined = base.join(&sanitized);
        if !joined.starts_with(base) {
            return Err(RunDirError::PathTraversal { path: joined });
        }
        Ok(joined)
    }
}

fn tempfile(path: &Path) -> Result<fs::File, std::io::Error> {
    let tmp = path.with_extension("tmp");
    fs::File::create(&tmp)
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_base() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vaa_rundir_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn creates_full_directory_tree() {
        let base = temp_base();
        let id = RunId::generate();
        let rundir = RunDir::create(&base, &id).expect("create run dir");

        assert!(rundir.root().exists());
        assert!(rundir.paths.task_dir.exists());
        assert!(rundir.paths.target_dir.exists());
        assert!(rundir.paths.candidates_dir.exists());
        assert!(rundir.paths.accepted_dir.exists());
        assert!(rundir.paths.evidence_dir.exists());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn create_candidate_dir_creates_immutable_marker() {
        let base = temp_base();
        let id = RunId::generate();
        let rundir = RunDir::create(&base, &id).expect("create run dir");

        let candidate = rundir.create_candidate_dir(1).expect("create candidate");
        assert!(candidate.exists());
        assert!(candidate.join(".immutable").exists());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn candidate_dir_overflow_rejected() {
        let base = temp_base();
        let id = RunId::generate();
        let rundir = RunDir::create(&base, &id).expect("create run dir");

        let result = rundir.create_candidate_dir(10_000);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RunDirError::CandidateOverflow
        ));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn atomic_write_writes_content() {
        let base = temp_base();
        let id = RunId::generate();
        let rundir = RunDir::create(&base, &id).expect("create run dir");

        let path = rundir.root().join("test.txt");
        rundir
            .write_atomic(&path, b"hello atomic")
            .expect("atomic write");

        let content = fs::read_to_string(&path).expect("read");
        assert_eq!(content, "hello atomic");

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn atomic_write_outside_root_rejected() {
        let base = temp_base();
        let id = RunId::generate();
        let rundir = RunDir::create(&base, &id).expect("create run dir");

        let outside = PathBuf::from(r"\\?\C:\Windows\system32\evil.dll");
        let result = rundir.write_atomic(&outside, b"evil");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RunDirError::PathTraversal { .. }
        ));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn safe_join_rejects_path_traversal() {
        let base = PathBuf::from(r"C:\safe");
        let result = RunDir::safe_join(&base, "..\\..\\Windows\\system32");
        assert!(result.is_ok());
        let joined = result.unwrap();
        assert!(
            !joined.starts_with(r"C:\Windows"),
            "traversal should be sanitized"
        );
    }

    #[test]
    fn safe_join_rejects_empty_component() {
        let base = PathBuf::from(r"C:\safe");
        let result = RunDir::safe_join(&base, "");
        assert!(result.is_err());
    }

    #[test]
    fn candidate_dir_naming() {
        let base = temp_base();
        let id = RunId::generate();
        let rundir = RunDir::create(&base, &id).expect("create run dir");

        let c1 = rundir.candidate_dir(0).expect("candidate 0");
        assert!(c1.to_string_lossy().ends_with("\\0000"));

        let c2 = rundir.candidate_dir(9999).expect("candidate 9999");
        assert!(c2.to_string_lossy().ends_with("\\9999"));

        let _ = fs::remove_dir_all(&base);
    }
}
