use std::fs;
use std::io::{BufWriter, Write};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    RunStarted {
        task_id: String,
        task_digest: String,
    },
    CandidateSubmitted {
        index: u32,
        source_path: String,
    },
    CandidateAccepted {
        index: u32,
    },
    CandidateRejected {
        index: u32,
        reason: String,
    },
    SemasmCheck {
        version: String,
        compatible: bool,
    },
    BuildStarted {
        index: u32,
    },
    BuildCompleted {
        index: u32,
        success: bool,
    },
    VerificationStarted,
    VerificationCompleted {
        outcome: String,
    },
    RunFinished {
        outcome: String,
        candidate_count: u32,
    },
    Error {
        message: String,
    },
    Warning {
        message: String,
    },
    Info {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub timestamp: String,
    pub kind: EventKind,
}

impl Event {
    pub fn new(kind: EventKind) -> Self {
        Self {
            timestamp: iso_timestamp(),
            kind,
        }
    }
}

const DEFAULT_MAX_EVENTS: usize = 10_000;
const DEFAULT_MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug)]
pub struct EventLog {
    path: std::path::PathBuf,
    max_events: usize,
    max_file_bytes: u64,
    count: usize,
    byte_estimate: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum EventLogError {
    #[error("failed to write event: {0}")]
    Io(#[from] std::io::Error),
    #[error("event log size limit reached ({count} events, ~{bytes} bytes)")]
    SizeLimit {
        count: usize,
        bytes: u64,
    },
    #[error("failed to serialize event: {0}")]
    Serialize(#[from] serde_json::Error),
}

impl EventLog {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self {
            path,
            max_events: DEFAULT_MAX_EVENTS,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            count: 0,
            byte_estimate: 0,
        }
    }

    pub fn with_limits(mut self, max_events: usize, max_file_bytes: u64) -> Self {
        self.max_events = max_events;
        self.max_file_bytes = max_file_bytes;
        self
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn record(&mut self, kind: EventKind) -> Result<(), EventLogError> {
        if self.count >= self.max_events {
            return Err(EventLogError::SizeLimit {
                count: self.count,
                bytes: self.byte_estimate,
            });
        }

        let event = Event::new(kind);
        let line = serde_json::to_string(&event)?;
        let line_bytes = line.len() as u64 + 1;

        if self.byte_estimate + line_bytes > self.max_file_bytes {
            return Err(EventLogError::SizeLimit {
                count: self.count,
                bytes: self.byte_estimate,
            });
        }

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let mut writer = BufWriter::new(&mut file);
        writer.write_all(line.as_bytes())?;
        writer.write_all(b"\n")?;
        writer.flush()?;

        self.count += 1;
        self.byte_estimate += line_bytes;
        Ok(())
    }
}

fn iso_timestamp() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch");
    let secs = dur.as_secs();
    let subsec_nanos = dur.subsec_nanos();

    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    let (y, m, d) = civil_from_days(days as i64);
    let subsec = format_subsec(subsec_nanos);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{subsec}Z",
        y, m, d, hours, minutes, seconds
    )
}

fn format_subsec(nanos: u32) -> String {
    let ms = nanos / 1_000_000;
    format!("{:03}", ms)
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    static TEST_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    fn temp_event_log() -> (EventLog, PathBuf) {
        let count = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("vaa_event_test_{}_{}", std::process::id(), count));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("events.jsonl");
        let log = EventLog::new(path.clone());
        (log, path)
    }

    use std::path::Path;

    fn cleanup(path: &Path) {
        let _ = fs::remove_file(path);
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir(parent);
        }
    }

    #[test]
    fn records_events_to_jsonl() {
        let (mut log, path) = temp_event_log();

        log.record(EventKind::RunStarted {
            task_id: "test-v1".to_owned(),
            task_digest: "sha256:abc".to_owned(),
        })
        .expect("record run started");

        log.record(EventKind::Info {
            message: "hello".to_owned(),
        })
        .expect("record info");

        assert_eq!(log.count(), 2);

        let content = fs::read_to_string(&path).expect("read event log");
        let lines: Vec<&str> = content.trim().lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("run_started"));
        assert!(lines[1].contains("info"));

        cleanup(&path);
    }

    #[test]
    fn event_log_respects_max_events() {
        let (mut log, path) = temp_event_log();
        log.max_events = 2;

        log.record(EventKind::Info {
            message: "first".to_owned(),
        })
        .expect("first event");
        log.record(EventKind::Info {
            message: "second".to_owned(),
        })
        .expect("second event");

        let result = log.record(EventKind::Info {
            message: "third".to_owned(),
        });
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), EventLogError::SizeLimit { .. }));

        cleanup(&path);
    }

    #[test]
    fn event_log_respects_max_file_bytes() {
        let (mut log, path) = temp_event_log();
        log.max_file_bytes = 1;

        let result = log.record(EventKind::Info {
            message: "any".to_owned(),
        });
        assert!(result.is_err());

        cleanup(&path);
    }

    #[test]
    fn event_serializes_with_kind_tag() {
        let event = Event::new(EventKind::Warning {
            message: "test warning".to_owned(),
        });
        let json = serde_json::to_string(&event).expect("serialize");
        assert!(json.contains(r#""kind":"warning""#));
        assert!(json.contains(r#""message":"test warning""#));
    }
}
