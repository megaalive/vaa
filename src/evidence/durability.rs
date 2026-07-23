//! Seal durability classification (P7-D).
//!
//! Extends D0 atomic seal-last publication with an explicit durability *class*
//! for the target filesystem. This is **not** a formal proof of FS correctness.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::seal::SealError;

/// How durable multi-file seal publication can claim to be on this path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DurabilityClass {
    /// Unix local disk: file + parent dir sync required and succeeded in probe.
    LocalDurable,
    /// Windows / network / restricted: best-effort sync only.
    BestEffort,
    /// Task/policy asked for hard durability and the FS failed the probe.
    RefuseVerified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DurabilityProbeReport {
    pub path: String,
    pub class: DurabilityClass,
    pub details: Vec<String>,
}

/// Env: when `1`/`true`, treat best-effort FS as `RefuseVerified` for Verified claims.
pub const ENV_REQUIRE_LOCAL_DURABLE: &str = "VAA_REQUIRE_LOCAL_DURABLE";

fn require_local_durable_env() -> bool {
    match std::env::var(ENV_REQUIRE_LOCAL_DURABLE) {
        Ok(v) => {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
}

/// Probe `dir` for rename + fsync behavior. Creates a temporary sibling file.
#[must_use]
pub fn probe_durability(dir: &Path) -> DurabilityProbeReport {
    let mut details = Vec::new();
    let path_s = dir.display().to_string();

    if let Err(e) = fs::create_dir_all(dir) {
        return DurabilityProbeReport {
            path: path_s,
            class: DurabilityClass::RefuseVerified,
            details: vec![format!("create_dir_all failed: {e}")],
        };
    }

    let marker = dir.join(format!(
        ".vaa-durability-probe-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let tmp = PathBuf::from(format!("{}.tmp", marker.display()));

    let write_ok = (|| -> Result<(), String> {
        let mut f = File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(b"vaa-durability-probe\n")
            .map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| format!("file sync_all: {e}"))?;
        drop(f);
        fs::rename(&tmp, &marker).map_err(|e| e.to_string())?;
        let f = OpenOptions::new()
            .write(true)
            .open(&marker)
            .map_err(|e| e.to_string())?;
        f.sync_all()
            .map_err(|e| format!("post-rename sync_all: {e}"))?;
        Ok(())
    })();

    let dir_sync = fsync_dir(dir);

    let _ = fs::remove_file(&marker);
    let _ = fs::remove_file(&tmp);

    match write_ok {
        Err(e) => {
            details.push(e);
            DurabilityProbeReport {
                path: path_s,
                class: DurabilityClass::RefuseVerified,
                details,
            }
        }
        Ok(()) => {
            #[cfg(windows)]
            {
                details.push("windows directory sync is best-effort".into());
                let class = if require_local_durable_env() {
                    details.push("VAA_REQUIRE_LOCAL_DURABLE set → refuse-verified".into());
                    DurabilityClass::RefuseVerified
                } else {
                    DurabilityClass::BestEffort
                };
                let _ = dir_sync;
                DurabilityProbeReport {
                    path: path_s,
                    class,
                    details,
                }
            }
            #[cfg(not(windows))]
            {
                match dir_sync {
                    Ok(()) => {
                        details.push("unix parent-dir sync_all ok".into());
                        DurabilityProbeReport {
                            path: path_s,
                            class: DurabilityClass::LocalDurable,
                            details,
                        }
                    }
                    Err(e) => {
                        details.push(format!("parent-dir sync failed: {e}"));
                        let class = if require_local_durable_env() {
                            DurabilityClass::RefuseVerified
                        } else {
                            DurabilityClass::BestEffort
                        };
                        DurabilityProbeReport {
                            path: path_s,
                            class,
                            details,
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::unnecessary_wraps)]
fn fsync_dir(dir: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        if let Ok(d) = OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(dir)
        {
            let _ = d.sync_all();
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let d = File::open(dir).map_err(|e| e.to_string())?;
        d.sync_all().map_err(|e| e.to_string())
    }
}

/// Map probe class through policy: Verified may be refused.
#[must_use]
pub fn may_claim_verified(class: DurabilityClass) -> bool {
    !matches!(class, DurabilityClass::RefuseVerified)
}

/// Multi-file publish helper used by tests: write N temps, sync, rename seal last.
pub fn publish_files_seal_last(
    files: &[(PathBuf, Vec<u8>)],
    seal_path: &Path,
    seal_bytes: &[u8],
) -> Result<DurabilityClass, SealError> {
    let parent = seal_path.parent().unwrap_or_else(|| Path::new("."));
    let probe = probe_durability(parent);
    if matches!(probe.class, DurabilityClass::RefuseVerified) {
        return Err(SealError::Io(format!(
            "durability refuse-verified: {}",
            probe.details.join("; ")
        )));
    }

    let mut tmps = Vec::new();
    for (path, bytes) in files {
        let tmp = PathBuf::from(format!("{}.tmp", path.display()));
        let mut f = File::create(&tmp).map_err(|e| SealError::Io(e.to_string()))?;
        f.write_all(bytes)
            .map_err(|e| SealError::Io(e.to_string()))?;
        f.sync_all().map_err(|e| SealError::Io(e.to_string()))?;
        tmps.push((tmp, path.clone()));
    }
    let seal_tmp = PathBuf::from(format!("{}.tmp", seal_path.display()));
    {
        let mut f = File::create(&seal_tmp).map_err(|e| SealError::Io(e.to_string()))?;
        f.write_all(seal_bytes)
            .map_err(|e| SealError::Io(e.to_string()))?;
        f.sync_all().map_err(|e| SealError::Io(e.to_string()))?;
    }

    for (tmp, final_path) in &tmps {
        fs::rename(tmp, final_path).map_err(|e| SealError::Io(e.to_string()))?;
        let f = OpenOptions::new()
            .write(true)
            .open(final_path)
            .map_err(|e| SealError::Io(e.to_string()))?;
        f.sync_all().map_err(|e| SealError::Io(e.to_string()))?;
    }
    fs::rename(&seal_tmp, seal_path).map_err(|e| SealError::Io(e.to_string()))?;
    let f = OpenOptions::new()
        .write(true)
        .open(seal_path)
        .map_err(|e| SealError::Io(e.to_string()))?;
    f.sync_all().map_err(|e| SealError::Io(e.to_string()))?;
    let _ = fsync_dir(parent);

    Ok(probe.class)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_temp_dir_not_refuse_without_env() {
        let dir = std::env::temp_dir().join(format!("vaa_dur_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let report = probe_durability(&dir);
        assert!(
            matches!(
                report.class,
                DurabilityClass::LocalDurable | DurabilityClass::BestEffort
            ),
            "{report:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn publish_seal_last_round_trip() {
        let dir = std::env::temp_dir().join(format!("vaa_pub_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let ev = dir.join("evidence.json");
        let seal = dir.join("evidence.seal.json");
        let class = publish_files_seal_last(
            &[(ev.clone(), b"{\"ok\":true}".to_vec())],
            &seal,
            b"{\"seal\":1}",
        )
        .expect("publish");
        assert!(may_claim_verified(class));
        assert_eq!(fs::read_to_string(&ev).unwrap(), "{\"ok\":true}");
        assert_eq!(fs::read_to_string(&seal).unwrap(), "{\"seal\":1}");
        let _ = fs::remove_dir_all(&dir);
    }
}
