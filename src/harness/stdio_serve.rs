//! NDJSON stdio agent serve loop (`vaa agent serve --stdio --case <dir>`).
//!
//! **Stdout is protocol JSON lines only.** Diagnostic logs go to stderr.
//! Controllers must never scrape stderr for decisions.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{json, Value};

use crate::harness::assembler::AssemblerFlavor;
use crate::harness::idioms::write_idioms_json;
use crate::harness::session::{
    prepare_direct_nasm, resume_status, submit_direct_nasm, HarnessError, PrepareDirectRequest,
    SubmitDirectRequest, VerifyLevel,
};
use crate::task::load_locked_task;

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
}

/// Mutable session state for one `serve` process.
pub struct AgentServeSession {
    case_dir: PathBuf,
    workspace: PathBuf,
    task: PathBuf,
    contract: PathBuf,
    assembler: AssemblerFlavor,
    started: bool,
    finished: bool,
    allow_execution: bool,
    allow_under_preconditions: bool,
    run_base: Option<PathBuf>,
    run_dir: Option<PathBuf>,
    last_feedback: Option<Value>,
    id_cache: HashMap<String, Value>,
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
        let workspace = case_dir.join(".vaa-agent");
        let assembler = AssemblerFlavor::Nasm;
        Ok(Self {
            case_dir,
            workspace,
            task,
            contract,
            assembler,
            started: false,
            finished: false,
            allow_execution: false,
            allow_under_preconditions: false,
            run_base: None,
            run_dir: None,
            last_feedback: None,
            id_cache: HashMap::new(),
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
            "candidate.submit" => self.candidate_submit(&incoming),
            "feedback.get" => self.feedback_get(),
            "session.status" => self.session_status(),
            "session.finish" => self.session_finish(),
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

        let seed = {
            let asm = self.case_dir.join("seed.asm");
            let s = self.case_dir.join("seed.S");
            if asm.is_file() {
                Some(asm)
            } else if s.is_file() {
                Some(s)
            } else {
                None
            }
        };

        // Resume hint: if a prior run dir exists under run_base, surface via status later.
        if let Some(base) = &self.run_base {
            if let Ok(rd) = find_latest_run_dir(base) {
                self.run_dir = Some(rd);
            }
        }

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

        let locked = load_locked_task(&self.task).map_err(|e| (-32001, e.to_string()))?;
        let _ = write_idioms_json(
            &self.workspace,
            &locked.task().target,
            Some(locked.task().entry.symbol.as_str()),
        );

        self.started = true;
        Ok(json!({
            "ok": true,
            "workspace": self.workspace.display().to_string(),
            "case": self.case_dir.display().to_string(),
            "task_id": env.task_id,
            "target": env.target,
            "assembler": env.assembler.as_str(),
            "work_packet_path": env.work_packet_path,
            "run_dir": self.run_dir.as_ref().map(|p| p.display().to_string()),
            "note": "stdout carries protocol JSON only; stderr is noise",
        }))
    }

    fn candidate_submit(&mut self, req: &Incoming) -> Result<Value, (i32, String)> {
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

        let mut allow_execution = self.allow_execution;
        if let Some(v) = req.allow_execution {
            allow_execution = v;
        }

        // Pass session run paths; `resolve_verify_policy` strips them for Fast/Full
        // and preserves default seal-when-run when `level` is None.
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
            idempotency_key: Some(format!("serve-{}", self.id_cache.len())),
            level,
        })
        .map_err(|e| (-32003, e.to_string()))?;

        if let Some(rd) = &result.run_dir {
            self.run_dir = Some(PathBuf::from(rd));
        }
        let value = serde_json::to_value(&result).map_err(|e| (-32003, e.to_string()))?;
        self.last_feedback = Some(value.clone());
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
                    "workspace": self.workspace.display().to_string(),
                    "run": status,
                }));
            }
        }
        Ok(json!({
            "started": self.started,
            "finished": self.finished,
            "workspace": self.workspace.display().to_string(),
            "run_dir": self.run_dir.as_ref().map(|p| p.display().to_string()),
            "has_feedback": self.last_feedback.is_some()
                || self.workspace.join("feedback.json").is_file(),
        }))
    }

    fn session_finish(&mut self) -> Result<Value, (i32, String)> {
        self.finished = true;
        Ok(json!({
            "ok": true,
            "finished": true,
            "workspace": self.workspace.display().to_string(),
            "run_dir": self.run_dir.as_ref().map(|p| p.display().to_string()),
        }))
    }
}

fn error_response(id: &str, code: i32, message: &str) -> Value {
    json!({
        "id": id,
        "error": { "code": code, "message": message }
    })
}

fn find_latest_run_dir(base: &Path) -> Result<PathBuf, ServeError> {
    if !base.is_dir() {
        return Err(ServeError::Message("run base missing".into()));
    }
    let mut dirs: Vec<_> = std::fs::read_dir(base)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.join("events.jsonl").is_file())
        .collect();
    dirs.sort();
    dirs.pop()
        .ok_or_else(|| ServeError::Message("no prior run".into()))
}

/// Run the NDJSON serve loop on the given readers/writers.
pub fn serve_stdio_with<R: BufRead, W: Write>(
    case_dir: &Path,
    mut input: R,
    mut output: W,
) -> Result<(), ServeError> {
    let mut session = AgentServeSession::open(case_dir)?;
    eprintln!(
        "vaa agent serve: case={} workspace={} (protocol on stdout)",
        session.case_dir.display(),
        session.workspace.display()
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
        // Minimal case files so open() succeeds.
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
        // finish before start → error, but cached
        let a = session.handle_value(&req);
        // Force started so second call would differ if not cached
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

        // Avoid prepare (needs write + optional semasm): drive status/finish only via handle_value.
        let mut session = AgentServeSession::open(&tmp).unwrap();
        session.started = true;
        let status = session.handle_value(&json!({"id":"s1","method":"session.status"}));
        assert_eq!(status["id"], "s1");
        assert!(status.get("result").is_some());
        let finish = session.handle_value(&json!({"id":"s2","method":"session.finish"}));
        assert_eq!(finish["result"]["finished"], true);

        // Also exercise line loop with Cursor (no prepare).
        let input = Cursor::new(
            r#"{"id":"a","method":"session.status"}
{"id":"b","method":"session.finish"}
"#,
        );
        let mut out = Vec::new();
        // Re-open clean session for line loop — status without start is ok.
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
