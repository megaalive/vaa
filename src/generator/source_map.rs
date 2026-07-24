//! Optional generator source mapping join (plan §13).
//!
//! Generators may emit `candidate.map.json` next to `candidate.asm`. VAA
//! joins a SemASM instruction offset (or assembly line) against the map so
//! repair packets can point at generator input, IR node, and backend source.
//!
//! Fallback is explicit (plan §13.3): a missing map never downgrades
//! verification status — the repair packet simply carries assembly context
//! only.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::generator::error::GeneratorError;
use crate::generator::repair::RepairSourceMapping;

/// Accepted source map schema version.
pub const SOURCE_MAP_SCHEMA_VERSION: &str = "0.1";

/// One assembly-to-source mapping entry.
///
/// Field aliases keep plan §13.1 HlaX64 pack keys (`hla_source`,
/// `compiler_source`) readable while VAA core stays generator-agnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMapEntry {
    /// 1-based line in the generated assembly file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assembly_line: Option<u64>,
    /// Instruction offset as emitted by the verifier (e.g. `0x31`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruction_offset: Option<String>,
    /// Generator input reference (e.g. `input.hlx:8:5`).
    #[serde(default, alias = "hla_source", skip_serializing_if = "Option::is_none")]
    pub generator_input: Option<String>,
    /// IR node reference (e.g. `StoreByte#17`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ir_node: Option<String>,
    /// Generator backend source reference (e.g. `src/backend/x64/store.rs:84`).
    #[serde(
        default,
        alias = "compiler_source",
        skip_serializing_if = "Option::is_none"
    )]
    pub generator_source: Option<String>,
}

/// Full `candidate.map.json` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMap {
    pub schema_version: String,
    /// Generator revision that produced the map (`git:…`).
    #[serde(
        default,
        alias = "compiler_revision",
        skip_serializing_if = "Option::is_none"
    )]
    pub generator_revision: Option<String>,
    #[serde(default)]
    pub entries: Vec<SourceMapEntry>,
}

/// Load a source map JSON from disk and validate.
pub fn load_source_map(path: impl AsRef<Path>) -> Result<SourceMap, GeneratorError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|source| GeneratorError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let map: SourceMap = serde_json::from_slice(&bytes).map_err(|error| GeneratorError::Parse {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let diagnostics = validate_source_map(&map);
    if !diagnostics.is_empty() {
        return Err(GeneratorError::from_diagnostics(&diagnostics));
    }
    Ok(map)
}

/// Structural validation; returns human-readable diagnostics.
#[must_use]
pub fn validate_source_map(map: &SourceMap) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if map.schema_version != SOURCE_MAP_SCHEMA_VERSION {
        diagnostics.push(format!(
            "unsupported source map schema_version `{}` (expected `{SOURCE_MAP_SCHEMA_VERSION}`)",
            map.schema_version
        ));
    }
    for (index, entry) in map.entries.iter().enumerate() {
        if entry.assembly_line.is_none() && entry.instruction_offset.is_none() {
            diagnostics.push(format!(
                "entries[{index}]: needs `assembly_line` or `instruction_offset` to be joinable"
            ));
        }
        if let Some(offset) = &entry.instruction_offset {
            if parse_offset(offset).is_none() {
                diagnostics.push(format!(
                    "entries[{index}]: instruction_offset `{offset}` is not a hex (`0x…`) or decimal offset"
                ));
            }
        }
        if entry.generator_input.is_none()
            && entry.ir_node.is_none()
            && entry.generator_source.is_none()
        {
            diagnostics.push(format!(
                "entries[{index}]: carries no source information (generator_input / ir_node / generator_source all absent)"
            ));
        }
    }
    diagnostics
}

/// Parse `0x…` hex or plain decimal offsets to a number for comparison.
#[must_use]
pub fn parse_offset(text: &str) -> Option<u64> {
    let trimmed = text.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16).ok();
    }
    trimmed.parse::<u64>().ok()
}

/// Join by instruction offset (hex/decimal tolerant). Returns the first match.
#[must_use]
pub fn join_by_offset<'map>(map: &'map SourceMap, offset: &str) -> Option<&'map SourceMapEntry> {
    let wanted = parse_offset(offset)?;
    map.entries.iter().find(|entry| {
        entry
            .instruction_offset
            .as_deref()
            .and_then(parse_offset)
            .is_some_and(|value| value == wanted)
    })
}

/// Join by 1-based assembly line. Returns the first match.
#[must_use]
pub fn join_by_assembly_line(map: &SourceMap, line: u64) -> Option<&SourceMapEntry> {
    map.entries
        .iter()
        .find(|entry| entry.assembly_line == Some(line))
}

/// Convert a joined entry into the repair packet source mapping slice.
#[must_use]
pub fn entry_to_repair_mapping(entry: &SourceMapEntry) -> RepairSourceMapping {
    RepairSourceMapping {
        generator_input: entry.generator_input.clone(),
        ir_node: entry.ir_node.clone(),
        generator_source: entry.generator_source.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_map() -> SourceMap {
        serde_json::from_str(
            r#"{
  "schema_version": "0.1",
  "compiler_revision": "git:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "entries": [
    {
      "assembly_line": 42,
      "instruction_offset": "0x31",
      "hla_source": "input.hlx:8:5",
      "ir_node": "StoreByte#17",
      "compiler_source": "src/backend/x64/store.rs:84"
    },
    {
      "assembly_line": 7,
      "instruction_offset": "0x17",
      "ir_node": "CompareSigned#12"
    }
  ]
}"#,
        )
        .expect("sample map")
    }

    #[test]
    fn plan_aliases_deserialize() {
        let map = sample_map();
        assert_eq!(
            map.generator_revision.as_deref(),
            Some("git:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert_eq!(
            map.entries[0].generator_input.as_deref(),
            Some("input.hlx:8:5")
        );
        assert_eq!(
            map.entries[0].generator_source.as_deref(),
            Some("src/backend/x64/store.rs:84")
        );
        assert!(validate_source_map(&map).is_empty());
    }

    #[test]
    fn join_by_offset_hex_and_decimal() {
        let map = sample_map();
        let entry = join_by_offset(&map, "0x31").expect("hex join");
        assert_eq!(entry.assembly_line, Some(42));
        let same = join_by_offset(&map, "49").expect("decimal join (0x31 == 49)");
        assert_eq!(same.assembly_line, Some(42));
        assert!(join_by_offset(&map, "0xFFFF").is_none());
    }

    #[test]
    fn join_by_line_works() {
        let map = sample_map();
        let entry = join_by_assembly_line(&map, 7).expect("line join");
        assert_eq!(entry.ir_node.as_deref(), Some("CompareSigned#12"));
        assert!(join_by_assembly_line(&map, 999).is_none());
    }

    #[test]
    fn entry_converts_to_repair_mapping() {
        let map = sample_map();
        let mapping = entry_to_repair_mapping(&map.entries[0]);
        assert_eq!(mapping.generator_input.as_deref(), Some("input.hlx:8:5"));
        assert_eq!(mapping.ir_node.as_deref(), Some("StoreByte#17"));
        assert_eq!(
            mapping.generator_source.as_deref(),
            Some("src/backend/x64/store.rs:84")
        );
    }

    #[test]
    fn validation_flags_unjoinable_and_empty_entries() {
        let map = SourceMap {
            schema_version: "0.1".into(),
            generator_revision: None,
            entries: vec![SourceMapEntry {
                assembly_line: None,
                instruction_offset: None,
                generator_input: None,
                ir_node: None,
                generator_source: None,
            }],
        };
        let diagnostics = validate_source_map(&map);
        assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    }

    #[test]
    fn validation_flags_bad_offset_and_schema() {
        let map = SourceMap {
            schema_version: "9.9".into(),
            generator_revision: None,
            entries: vec![SourceMapEntry {
                assembly_line: Some(1),
                instruction_offset: Some("zz".into()),
                generator_input: Some("input.hlx:1".into()),
                ir_node: None,
                generator_source: None,
            }],
        };
        let diagnostics = validate_source_map(&map);
        assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    }

    #[test]
    fn parse_offset_variants() {
        assert_eq!(parse_offset("0x17"), Some(0x17));
        assert_eq!(parse_offset("0X17"), Some(0x17));
        assert_eq!(parse_offset("23"), Some(23));
        assert_eq!(parse_offset(" 0x17 "), Some(0x17));
        assert_eq!(parse_offset("zz"), None);
    }
}
