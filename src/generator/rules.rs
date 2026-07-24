//! Agent/editor rules and command templates (plan §15–§16).
//!
//! Renders a project-rule document from the generator spec so agents and
//! interactive editors get one authoritative, generator-specific brief:
//! allowed/forbidden paths, fixed commands, and acceptance policy. The
//! editor stays a code-reasoning surface; acceptance is `vaa patch
//! evidence-verify` — never agent self-report.

use std::path::Path;

use crate::generator::error::GeneratorError;
use crate::generator::spec::GeneratorSpec;

/// Fixed command templates surfaced to agents/editors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleCommands {
    /// Locked build command (defaults to the spec build argv).
    pub build: String,
    /// One-case regeneration command template.
    pub regenerate: String,
    /// Single per-case verification command template.
    pub verify: String,
    /// Full regression suite command.
    pub suite: String,
}

/// Render agent + editor rules Markdown from a generator spec.
#[must_use]
pub fn render_agent_rules(spec: &GeneratorSpec, commands: &RuleCommands) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Repair rules for generator `{}`\n\n\
These rules bind any coding agent or interactive editor working on this\n\
generator. Acceptance authority is VAA/SemASM evidence, never agent output.\n\n\
## Scope\n\n\
- repository: `{}` at `{}`\n\
- fix the **generator source**, never the generated assembly.\n\n\
## Allowed paths (editable)\n",
        spec.generator_id, spec.repository.path, spec.repository.expected_revision
    );
    if spec.patch_policy.allowed_paths.is_empty() {
        out.push_str("- (none declared — extend `patch_policy.allowed_paths` first)\n");
    } else {
        for p in &spec.patch_policy.allowed_paths {
            let _ = writeln!(out, "- `{p}`");
        }
    }
    out.push_str("\n## Forbidden paths (never edit)\n\n");
    for p in &spec.patch_policy.forbidden_paths {
        let _ = writeln!(out, "- `{p}`");
    }
    let _ = writeln!(
        out,
        "\nAdditionally forbidden everywhere: generated candidate assembly,\n\
SemASM contracts, VAA tasks, authoritative vectors, `stack.lock.toml`,\n\
and evidence files.\n\n\
## Fixed commands\n\n\
| Step | Command |\n\
|---|---|\n\
| build | `{}` |\n\
| regenerate one case | `{}` |\n\
| verify one case | `{}` |\n\
| full regression suite | `{}` |\n\n\
## Loop (after each change)\n\n\
1. rebuild the generator;\n\
2. regenerate candidate assembly from the locked generator input;\n\
3. run the supplied case verification command;\n\
4. run the required regression suite before completion.\n\n\
## Acceptance\n\n\
- output is accepted only after patch evidence verifies\n\
  (`vaa patch evidence-verify`); agent self-report is not acceptance;\n\
- `Incomplete` / `verified_under_preconditions` is **not** `Verified` and\n\
  is not, by itself, a generator defect (run `vaa generator triage`);\n\
- never weaken contracts, vectors, or the stack lock to make a case pass.",
        commands.build, commands.regenerate, commands.verify, commands.suite
    );
    out
}

/// Write rules Markdown to disk.
pub fn write_agent_rules(
    path: &Path,
    spec: &GeneratorSpec,
    commands: &RuleCommands,
) -> Result<(), GeneratorError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| GeneratorError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(path, render_agent_rules(spec, commands)).map_err(|source| GeneratorError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::spec::parse_generator_spec;

    fn sample_spec() -> GeneratorSpec {
        let text = r#"
schema_version = "0.1"
generator_id = "hlax64"
[repository]
path = "../hlax64"
expected_revision = "git:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
[build]
command = ["cargo", "build", "--release"]
[generation]
command = ["{generator}", "compile", "{input}", "--output", "{output}"]
[patch_policy]
allowed_paths = ["src/backend/**"]
forbidden_paths = ["**/stack.lock.toml"]
"#;
        parse_generator_spec(Path::new("mem"), text).expect("spec")
    }

    fn sample_commands() -> RuleCommands {
        RuleCommands {
            build: "cargo build --release".into(),
            regenerate: "vaa generator-run --skip-verify --spec generator.spec.toml".into(),
            verify: "vaa generator-run --spec generator.spec.toml".into(),
            suite: "vaa suite run suites/smoke.vaa-suite.toml".into(),
        }
    }

    #[test]
    fn rules_include_paths_commands_and_acceptance() {
        let text = render_agent_rules(&sample_spec(), &sample_commands());
        assert!(text.contains("`src/backend/**`"));
        assert!(text.contains("`**/stack.lock.toml`"));
        assert!(text.contains("vaa suite run suites/smoke.vaa-suite.toml"));
        assert!(text.contains("vaa patch evidence-verify"));
        assert!(text.contains("**not** `Verified`"));
        assert!(text.contains("never the generated assembly"));
    }

    #[test]
    fn empty_allowlist_is_called_out() {
        let mut spec = sample_spec();
        spec.patch_policy.allowed_paths.clear();
        let text = render_agent_rules(&spec, &sample_commands());
        assert!(text.contains("none declared"));
    }

    #[test]
    fn write_creates_file() {
        let dir = std::env::temp_dir().join(format!(
            "vaa-rules-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("agent-rules.md");
        write_agent_rules(&path, &sample_spec(), &sample_commands()).expect("write");
        let body = std::fs::read_to_string(&path).expect("read");
        assert!(body.contains("Repair rules for generator `hlax64`"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
