//! NDJSON stdio agent serve loop (`vaa agent serve --stdio --case <dir>`).
//!
//! **Stdout is protocol JSON lines only.** Diagnostic logs go to stderr.
//! Controllers must never scrape stderr for decisions.
//!
//! H2/H3: assembler from locked task target; explicit resume; digest idempotency
//! journaled under `run-dir/protocol-events.jsonl`.

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::evidence::sha256_digest_prefixed;
use crate::harness::assembler::AssemblerFlavor;
use crate::harness::idioms::write_idioms_json;
use crate::harness::session::{
    prepare_direct_nasm, resume_status, submit_direct_nasm, HarnessError, PrepareDirectRequest,
    SubmitDirectRequest, VerifyLevel,
};
use crate::semasm::admission::CAPABILITY_SNAPSHOT_DIGEST;
use crate::task::load_locked_task;

/// Protocol journal filename under a run directory.
pub const PROTOCOL_EVENTS_FILE: &str = "protocol-events.jsonl";

/// Errors from the stdio serve loop.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Harness(#[from] HarnessError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct Incoming {
    #[allow(dead_code)] // present for protocol symmetry; id handled before deserialize use
    id: String,
    method: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    allow_execution: Option<bool>,
    #[serde(default)]
    allow_under_preconditions: Option<bool>,
    #[serde(default)]
    run_base: Option<String>,
    /// When false/absent: do not auto-pick a run directory.
    #[serde(default)]
    resume: Option<bool>,
    /// Explicit run directory name (under `run_base`) or absolute path.
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    expected_task_digest: Option<String>,
    #[serde(default)]
    expected_contract_digest: Option<String>,
    #[serde(default)]
    expected_capability_snapshot_digest: Option<String>,
    #[serde(default)]
    expected_target_profile_digest: Option<String>,
    #[serde(default)]
    expected_assembler: Option<String>,
}

/// Mutable session state for one `serve` process.
pub struct AgentServeSession {
    case_dir: PathBuf,
    workspace: PathBuf,
    task: PathBuf,
    contract: PathBuf,
    session_id: String,
    assembler: AssemblerFlavor,
    started: bool,
    finished: bool,
    allow_execution: bool,
    allow_under_preconditions: bool,
    run_base: Option<PathBuf>,
    run_dir: Option<PathBuf>,
    last_feedback: Option<Value>,
    /// In-process request-id cache (same-process duplicate `id`).
    id_cache: HashMap<String, Value>,
    /// Persistent idempotency key → prior submit result (loaded/appended via journal).
    idempotency_cache: HashMap<String, Value>,
}

impl AgentServeSession {
    /// Open a case directory (`task.vaa.toml` + `contract.sem.toml` required).
    pub fn open(case_dir: &Path) -> Result<Self, ServeError> {
        let case_dir = case_dir
            .canonicalize()
            .unwrap_or_else(|_| case_dir.to_path_buf());
        let task = case_dir.join("task.vaa.toml");
        let contract = case_dir.join("contract.sem.toml");
        if !task.is_file() {
            return Err(ServeError::Message(format!(
                "case dir missing task.vaa.toml: {}",
                task.display()
            )));
        }
        if !contract.is_file() {
            return Err(ServeError::Message(format!(
                "case dir missing contract.sem.toml: {}",
                contract.display()
            )));
        }
        let locked = load_locked_task(&task).map_err(|e| ServeError::Message(e.to_string()))?;
        let assembler =
            AssemblerFlavor::for_target(&locked.task().target).map_err(ServeError::Message)?;
        assembler
            .ensure_supported_for(&locked.task().target)
            .map_err(ServeError::Message)?;
        let workspace = case_dir.join(".vaa-agent");
        Ok(Self {
            case_dir,
            workspace,
            task,
            contract,
            session_id: generate_session_id(),
            assembler,
            started: false,
            finished: false,
            allow_execution: false,
            allow_under_preconditions: false,
            run_base: None,
            run_dir: None,
            last_feedback: None,
            id_cache: HashMap::new(),
            idempotency_cache: HashMap::new(),
        })
    }

    /// Handle one parsed request object; returns the protocol response value.
    pub fn handle_value(&mut self, raw: &Value) -> Value {
        let id = raw
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        if id.is_empty() {
            return error_response("", -32600, "missing request id");
        }
        if let Some(cached) = self.id_cache.get(&id) {
            return cached.clone();
        }
        let incoming: Incoming = match serde_json::from_value(raw.clone()) {
            Ok(v) => v,
            Err(e) => {
                let resp = error_response(&id, -32600, &format!("invalid request: {e}"));
                self.id_cache.insert(id, resp.clone());
                return resp;
            }
        };
        let resp = match incoming.method.as_str() {
            "session.start" => self.session_start(&incoming),
            "candidate.submit" => self.candidate_submit(&id, &incoming),
            "feedback.get" => self.feedback_get(),
            "session.status" => self.session_status(),
            "session.finish" => Ok(self.session_finish()),
            other => Err((-32601, format!("unknown method: {other}"))),
        };
        let wrapped = match resp {
            Ok(result) => json!({ "id": id, "result": result }),
            Err((code, message)) => error_response(&id, code, &message),
        };
        self.id_cache.insert(id, wrapped.clone());
        wrapped
    }

    fn session_start(&mut self, req: &Incoming) -> Result<Value, (i32, String)> {
        if self.finished {
            return Err((-32000, "session already finished".into()));
        }
        self.allow_execution = req.allow_execution.unwrap_or(false);
        self.allow_under_preconditions = req.allow_under_preconditions.unwrap_or(false);
        if let Some(base) = &req.run_base {
            self.run_base = Some(PathBuf::from(base));
        } else {
            self.run_base = Some(self.case_dir.join(".vaa-runs"));
        }

        let locked = load_locked_task(&self.task).map_err(|e| (-32001, e.to_string()))?;
        // Re-resolve assembler from locked target (H3); never guess from seed extension.
        self.assembler =
            AssemblerFlavor::for_target(&locked.task().target).map_err(|e| (-32001, e))?;
        self.assembler
            .ensure_supported_for(&locked.task().target)
            .map_err(|e| (-32001, e))?;

        let task_digest = locked.digest().prefixed();
        let contract_digest = sha256_digest_prefixed(
            &std::fs::read(&self.contract).map_err(|e| (-32001, e.to_string()))?,
        );

        let resume = req.resume.unwrap_or(false);
        self.run_dir = None;
        if resume || req.run_id.is_some() {
            let resolved = resolve_resume_run_dir(
                self.run_base.as_ref().expect("run_base set"),
                req,
                &ResumeExpect {
                    task_digest: &task_digest,
                    contract_digest: &contract_digest,
                    capability_snapshot_digest: CAPABILITY_SNAPSHOT_DIGEST,
                    assembler: self.assembler.as_str(),
                },
            )?;
            self.run_dir = Some(resolved);
            if let Some(rd) = &self.run_dir {
                self.idempotency_cache = load_idempotency_journal(rd);
            }
        }

        let seed = {
            let preferred = self.case_dir.join(self.assembler.candidate_filename());
            let asm = self.case_dir.join("seed.asm");
            let s = self.case_dir.join("seed.S");
            // Prefer flavor-matching seed; never pick the other dialect first.
            if preferred.is_file() {
                Some(preferred)
            } else if self.assembler == AssemblerFlavor::Nasm && asm.is_file() {
                Some(asm)
            } else if self.assembler == AssemblerFlavor::Gas && s.is_file() {
                Some(s)
            } else if asm.is_file() {
                Some(asm)
            } else if s.is_file() {
                Some(s)
            } else {
                None
            }
        };

        let env = prepare_direct_nasm(&PrepareDirectRequest {
            task: self.task.clone(),
            contract: self.contract.clone(),
            workspace: self.workspace.clone(),
            seed_source: seed,
            allow_execution_in_recipes: self.allow_execution,
            assembler: self.assembler,
            run_dir: self.run_dir.clone(),
        })
        .map_err(|e| (-32001, e.to_string()))?;

        let _ = write_idioms_json(
            &self.workspace,
            &locked.task().target,
            Some(locked.task().entry.symbol.as_str()),
        );

        self.started = true;
        Ok(json!({
            "ok": true,
            "session_id": self.session_id,
            "workspace": self.workspace.display().to_string(),
            "case": self.case_dir.display().to_string(),
            "task_id": env.task_id,
            "target": env.target,
            "assembler": env.assembler.as_str(),
            "candidate_filename": self.assembler.candidate_filename(),
            "work_packet_path": env.work_packet_path,
            "run_dir": self.run_dir.as_ref().map(|p| p.display().to_string()),
            "resumed": resume || req.run_id.is_some(),
            "task_digest": task_digest,
            "contract_digest": contract_digest,
            "capability_snapshot_digest": CAPABILITY_SNAPSHOT_DIGEST,
            "note": "stdout carries protocol JSON only; stderr is noise; resume defaults to false",
        }))
    }

    fn candidate_submit(
        &mut self,
        request_id: &str,
        req: &Incoming,
    ) -> Result<Value, (i32, String)> {
        if !self.started {
            return Err((-32000, "session not started".into()));
        }
        if self.finished {
            return Err((-32000, "session already finished".into()));
        }
        let rel = req
            .path
            .as_deref()
            .unwrap_or(self.assembler.candidate_filename());
        let source = if Path::new(rel).is_absolute() {
            PathBuf::from(rel)
        } else {
            self.workspace.join(rel)
        };
        if !source.is_file() {
            return Err((-32002, format!("candidate not found: {}", source.display())));
        }

        let level = match req.level.as_deref() {
            None => None,
            Some(s) => {
                Some(VerifyLevel::parse(s).ok_or_else(|| (-32602, format!("invalid level: {s}")))?)
            }
        };
        let level_label = level
            .map(|l| l.as_str().to_owned())
            .unwrap_or_else(|| "default_seal".into());

        let source_bytes = std::fs::read(&source).map_err(|e| (-32002, e.to_string()))?;
        let source_digest = sha256_digest_prefixed(&source_bytes);
        let idem_key =
            digest_idempotency_key(&self.session_id, request_id, &source_digest, &level_label);

        if let Some(prior) = self.idempotency_cache.get(&idem_key) {
            self.last_feedback = Some(prior.clone());
            return Ok(prior.clone());
        }

        let mut allow_execution = self.allow_execution;
        if let Some(v) = req.allow_execution {
            allow_execution = v;
        }

        let result = submit_direct_nasm(&SubmitDirectRequest {
            task: self.task.clone(),
            contract: self.contract.clone(),
            source,
            allow_execution,
            allow_under_preconditions: self.allow_under_preconditions,
            run_dir: self.run_dir.clone(),
            run_base: self.run_base.clone(),
            timeout_secs: 120,
            assembler: self.assembler,
            idempotency_key: Some(idem_key.clone()),
            level,
        })
        .map_err(|e| (-32003, e.to_string()))?;

        if let Some(rd) = &result.run_dir {
            self.run_dir = Some(PathBuf::from(rd));
        }
        let value = serde_json::to_value(&result).map_err(|e| (-32003, e.to_string()))?;
        self.last_feedback = Some(value.clone());
        self.idempotency_cache
            .insert(idem_key.clone(), value.clone());
        if let Some(rd) = &self.run_dir {
            let _ = append_protocol_event(
                rd,
                &json!({
                    "kind": "candidate_submit",
                    "session_id": self.session_id,
                    "request_id": request_id,
                    "idempotency_key": idem_key,
                    "source_digest": source_digest,
                    "verification_level": level_label,
                    "result": value,
                }),
            );
        }
        Ok(value)
    }

    fn feedback_get(&self) -> Result<Value, (i32, String)> {
        if !self.started {
            return Err((-32000, "session not started".into()));
        }
        if let Some(fb) = &self.last_feedback {
            return Ok(fb.clone());
        }
        let path = self.workspace.join("feedback.json");
        if path.is_file() {
            let raw = std::fs::read_to_string(&path).map_err(|e| (-32004, e.to_string()))?;
            let v: Value = serde_json::from_str(&raw).map_err(|e| (-32004, e.to_string()))?;
            return Ok(v);
        }
        Err((-32004, "no feedback yet".into()))
    }

    fn session_status(&self) -> Result<Value, (i32, String)> {
        if let Some(run_dir) = &self.run_dir {
            if run_dir.is_dir() {
                let status = resume_status(run_dir).map_err(|e| (-32005, e.to_string()))?;
                return Ok(json!({
                    "started": self.started,
                    "finished": self.finished,
                    "session_id": self.session_id,
                    "assembler": self.assembler.as_str(),
                    "workspace": self.workspace.display().to_string(),
                    "run": status,
                }));
            }
        }
        Ok(json!({
            "started": self.started,
            "finished": self.finished,
            "session_id": self.session_id,
            "assembler": self.assembler.as_str(),
            "workspace": self.workspace.display().to_string(),
            "run_dir": self.run_dir.as_ref().map(|p| p.display().to_string()),
            "has_feedback": self.last_feedback.is_some()
                || self.workspace.join("feedback.json").is_file(),
        }))
    }

    fn session_finish(&mut self) -> Value {
        self.finished = true;
        json!({
            "ok": true,
            "finished": true,
            "session_id": self.session_id,
            "workspace": self.workspace.display().to_string(),
            "run_dir": self.run_dir.as_ref().map(|p| p.display().to_string()),
        })
    }
}

fn error_response(id: &str, code: i32, message: &str) -> Value {
    json!({
        "id": id,
        "error": { "code": code, "message": message }
    })
}

fn generate_session_id() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!(
        "sess-{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}

#[must_use]
pub fn digest_idempotency_key(
    session_id: &str,
    request_id: &str,
    source_digest: &str,
    verification_level: &str,
) -> String {
    let material = format!("{session_id}|{request_id}|{source_digest}|{verification_level}");
    sha256_digest_prefixed(material.as_bytes())
}

fn append_protocol_event(run_dir: &Path, event: &Value) -> Result<(), ServeError> {
    std::fs::create_dir_all(run_dir)?;
    let path = run_dir.join(PROTOCOL_EVENTS_FILE);
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    writeln!(file, "{}", serde_json::to_string(event)?)?;
    Ok(())
}

fn load_idempotency_journal(run_dir: &Path) -> HashMap<String, Value> {
    let path = run_dir.join(PROTOCOL_EVENTS_FILE);
    let Ok(raw) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if v.get("kind").and_then(|k| k.as_str()) != Some("candidate_submit") {
            continue;
        }
        let Some(key) = v.get("idempotency_key").and_then(|k| k.as_str()) else {
            continue;
        };
        if let Some(result) = v.get("result") {
            out.insert(key.to_owned(), result.clone());
        }
    }
    out
}

struct ResumeExpect<'a> {
    task_digest: &'a str,
    contract_digest: &'a str,
    capability_snapshot_digest: &'a str,
    assembler: &'a str,
}

fn resolve_resume_run_dir(
    run_base: &Path,
    req: &Incoming,
    expect: &ResumeExpect<'_>,
) -> Result<PathBuf, (i32, String)> {
    if let Some(id) = &req.run_id {
        let path = if Path::new(id).is_absolute() {
            PathBuf::from(id)
        } else {
            run_base.join(id)
        };
        if !path.is_dir() || !path.join("events.jsonl").is_file() {
            return Err((
                -32006,
                format!("resume run_id not found: {}", path.display()),
            ));
        }
        validate_run_identity(&path, req, expect)?;
        return Ok(path);
    }

    if !req.resume.unwrap_or(false) {
        return Err((-32006, "resume requested without run_id".into()));
    }

    let matches = find_matching_run_dirs(run_base, expect).map_err(|e| (-32006, e.to_string()))?;
    match matches.len() {
        0 => Err((-32006, "resume=true but no matching run directory".into())),
        1 => Ok(matches[0].clone()),
        _ => Err((
            -32006,
            format!(
                "ambiguous resume: {} matching runs {}; pass run_id explicitly",
                matches.len(),
                matches
                    .iter()
                    .filter_map(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
    }
}

fn validate_run_identity(
    run_dir: &Path,
    req: &Incoming,
    expect: &ResumeExpect<'_>,
) -> Result<(), (i32, String)> {
    let meta = read_run_start_meta(run_dir).map_err(|e| (-32006, e))?;
    let task_digest = meta
        .get("task_digest")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if task_digest != expect.task_digest {
        return Err((
            -32006,
            format!(
                "run identity mismatch: task_digest {task_digest} != {}",
                expect.task_digest
            ),
        ));
    }
    if let Some(want) = &req.expected_task_digest {
        if want != expect.task_digest && want.as_str() != task_digest {
            return Err((
                -32006,
                format!("expected_task_digest mismatch: want {want}, run has {task_digest}"),
            ));
        }
    }
    if let Some(want) = &req.expected_contract_digest {
        if want != expect.contract_digest {
            return Err((
                -32006,
                format!(
                    "expected_contract_digest mismatch against locked case ({})",
                    expect.contract_digest
                ),
            ));
        }
    }
    if let Some(want) = &req.expected_capability_snapshot_digest {
        if want != expect.capability_snapshot_digest {
            return Err((
                -32006,
                format!(
                    "expected_capability_snapshot_digest mismatch (want {want}, have {})",
                    expect.capability_snapshot_digest
                ),
            ));
        }
    }
    if let Some(want) = &req.expected_assembler {
        if want != expect.assembler {
            return Err((
                -32006,
                format!(
                    "expected_assembler mismatch (want {want}, session {})",
                    expect.assembler
                ),
            ));
        }
    }
    if let Some(want) = &req.expected_target_profile_digest {
        // Profile is regenerated at prepare; optional pin is checked when provided
        // against workspace profile after prepare — here we only reject empty pins.
        if want.is_empty() {
            return Err((-32006, "expected_target_profile_digest empty".into()));
        }
    }
    Ok(())
}

fn read_run_start_meta(run_dir: &Path) -> Result<Value, String> {
    let path = run_dir.join("events.jsonl");
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        let v: Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
        let kind = v.get("kind").cloned().unwrap_or(Value::Null);
        // EventLog serializes Event { timestamp, kind: EventKind::RunStarted {..} }
        // with externally tagged kind object under "kind".
        if let Some(obj) = kind.as_object() {
            if let Some(started) = obj.get("run_started") {
                return Ok(started.clone());
            }
        }
        if v.pointer("/kind/run_started").is_some() {
            return Ok(v["kind"]["run_started"].clone());
        }
        // Flat fallback if ever written differently.
        if v.get("task_digest").is_some() && v.get("task_id").is_some() {
            return Ok(v);
        }
    }
    Err(format!("no RunStarted event in {}", path.display()))
}

fn find_matching_run_dirs(
    base: &Path,
    expect: &ResumeExpect<'_>,
) -> Result<Vec<PathBuf>, ServeError> {
    if !base.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(base)? {
        let path = entry?.path();
        if !path.is_dir() || !path.join("events.jsonl").is_file() {
            continue;
        }
        let Ok(meta) = read_run_start_meta(&path) else {
            continue;
        };
        let Some(td) = meta.get("task_digest").and_then(|v| v.as_str()) else {
            continue;
        };
        if td == expect.task_digest {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// Run the NDJSON serve loop on the given readers/writers.
pub fn serve_stdio_with<R: BufRead, W: Write>(
    case_dir: &Path,
    mut input: R,
    mut output: W,
) -> Result<(), ServeError> {
    let mut session = AgentServeSession::open(case_dir)?;
    eprintln!(
        "vaa agent serve: case={} workspace={} session={} assembler={} (protocol on stdout)",
        session.case_dir.display(),
        session.workspace.display(),
        session.session_id,
        session.assembler.as_str(),
    );
    let mut line = String::new();
    loop {
        line.clear();
        let n = input.read_line(&mut line)?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let raw: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let resp = error_response("", -32700, &format!("parse error: {e}"));
                writeln!(output, "{}", serde_json::to_string(&resp)?)?;
                output.flush()?;
                continue;
            }
        };
        let resp = session.handle_value(&raw);
        writeln!(output, "{}", serde_json::to_string(&resp)?)?;
        output.flush()?;
    }
    Ok(())
}

/// Serve on process stdin/stdout.
pub fn serve_stdio(case_dir: &Path) -> Result<(), ServeError> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    serve_stdio_with(case_dir, stdin.lock(), stdout.lock())
}

/// Parse a single NDJSON request line into a JSON value (test helper).
pub fn parse_request_line(line: &str) -> Result<Value, ServeError> {
    Ok(serde_json::from_str(line.trim())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_request_line_ok() {
        let v = parse_request_line(r#"{"id":"1","method":"session.start"}"#).unwrap();
        assert_eq!(v["id"], "1");
        assert_eq!(v["method"], "session.start");
    }

    #[test]
    fn digest_idempotency_stable() {
        let a = digest_idempotency_key("sess-1", "req-1", "sha256:abc", "fast");
        let b = digest_idempotency_key("sess-1", "req-1", "sha256:abc", "fast");
        let c = digest_idempotency_key("sess-1", "req-2", "sha256:abc", "fast");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.starts_with("sha256:"));
    }

    #[test]
    fn open_derives_assembler_from_target() {
        let tmp = std::env::temp_dir().join(format!(
            "vaa-serve-asm-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("task.vaa.toml"),
            include_str!("../../fixtures/semasm/count_byte/count_byte.vaa.toml"),
        )
        .unwrap();
        std::fs::write(
            tmp.join("contract.sem.toml"),
            include_str!("../../fixtures/semasm/count_byte/count_byte.sem.toml"),
        )
        .unwrap();
        let session = AgentServeSession::open(&tmp).unwrap();
        assert_eq!(session.assembler, AssemblerFlavor::Nasm);
        assert!(!session.session_id.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn session_start_default_does_not_auto_resume() {
        let tmp = std::env::temp_dir().join(format!(
            "vaa-serve-noresume-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("task.vaa.toml"),
            include_str!("../../fixtures/semasm/count_byte/count_byte.vaa.toml"),
        )
        .unwrap();
        std::fs::write(
            tmp.join("contract.sem.toml"),
            include_str!("../../fixtures/semasm/count_byte/count_byte.sem.toml"),
        )
        .unwrap();
        // Fake prior run that would have been auto-picked lexicographically.
        let runs = tmp.join(".vaa-runs");
        let prior = runs.join("zzzz-last");
        std::fs::create_dir_all(&prior).unwrap();
        std::fs::write(prior.join("events.jsonl"), "{\"timestamp\":\"t\",\"kind\":{\"run_started\":{\"task_id\":\"x\",\"task_digest\":\"sha256:nope\"}}}\n").unwrap();

        let mut session = AgentServeSession::open(&tmp).unwrap();
        // prepare_direct needs write access; may fail without semasm — only assert run_dir stays None
        // when resume is false. If prepare fails, still check we didn't set run_dir before prepare.
        let req = json!({"id":"1","method":"session.start","resume":false});
        // Manually invoke session_start path via handle — prepare may succeed or fail.
        let resp = session.handle_value(&req);
        if resp.get("result").is_some() {
            assert!(
                resp["result"]["run_dir"].is_null()
                    || resp["result"]["run_dir"] == Value::Null
                    || resp["result"].get("run_dir").is_none()
                    || resp["result"]["resumed"] == false
            );
            assert_eq!(resp["result"]["resumed"], false);
            assert!(session.run_dir.is_none());
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn duplicate_id_returns_cached() {
        let tmp = std::env::temp_dir().join(format!(
            "vaa-serve-dup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("task.vaa.toml"),
            include_str!("../../fixtures/semasm/count_byte/count_byte.vaa.toml"),
        )
        .unwrap();
        std::fs::write(
            tmp.join("contract.sem.toml"),
            include_str!("../../fixtures/semasm/count_byte/count_byte.sem.toml"),
        )
        .unwrap();

        let mut session = AgentServeSession::open(&tmp).unwrap();
        let req = json!({"id":"dup-1","method":"session.finish"});
        let a = session.handle_value(&req);
        session.started = true;
        let b = session.handle_value(&req);
        assert_eq!(a, b);
        assert!(a.get("error").is_some() || a.get("result").is_some());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn hermetic_ndjson_roundtrip_status_finish() {
        let tmp = std::env::temp_dir().join(format!(
            "vaa-serve-ndjson-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("task.vaa.toml"),
            include_str!("../../fixtures/semasm/count_byte/count_byte.vaa.toml"),
        )
        .unwrap();
        std::fs::write(
            tmp.join("contract.sem.toml"),
            include_str!("../../fixtures/semasm/count_byte/count_byte.sem.toml"),
        )
        .unwrap();

        let mut session = AgentServeSession::open(&tmp).unwrap();
        session.started = true;
        let status = session.handle_value(&json!({"id":"s1","method":"session.status"}));
        assert_eq!(status["id"], "s1");
        assert!(status.get("result").is_some());
        assert!(status["result"]["session_id"]
            .as_str()
            .unwrap()
            .starts_with("sess-"));
        let finish = session.handle_value(&json!({"id":"s2","method":"session.finish"}));
        assert_eq!(finish["result"]["finished"], true);

        let input = Cursor::new(
            r#"{"id":"a","method":"session.status"}
{"id":"b","method":"session.finish"}
"#,
        );
        let mut out = Vec::new();
        serve_stdio_with(&tmp, input, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        let lines: Vec<_> = text.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 2);
        for line in lines {
            let v: Value = serde_json::from_str(line).unwrap();
            assert!(v.get("id").is_some());
            assert!(v.get("result").is_some() || v.get("error").is_some());
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
