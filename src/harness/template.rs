use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessConfig {
    pub entry_symbol: String,
    pub abi: String,
    pub test_values: Vec<Vec<u8>>,
    pub expected_outputs: Vec<Vec<u8>>,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessTemplate {
    pub source: String,
    pub config: HarnessConfig,
    pub abi_preserved_regs: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    #[error("test count mismatch: {values} values vs {outputs} outputs")]
    CountMismatch { values: usize, outputs: usize },
    #[error("unsupported ABI: {0}")]
    UnsupportedAbi(String),
}

impl HarnessTemplate {
    pub fn new(config: HarnessConfig) -> Result<Self, HarnessError> {
        if config.test_values.len() != config.expected_outputs.len() {
            return Err(HarnessError::CountMismatch {
                values: config.test_values.len(),
                outputs: config.expected_outputs.len(),
            });
        }

        let (abi_preserved_regs, abi_decl) = match config.abi.as_str() {
            "sysv64" => (
                vec!["rbx".to_owned(), "rbp".to_owned(), "r12".to_owned(),
                     "r13".to_owned(), "r14".to_owned(), "r15".to_owned()],
                "default rel".to_owned(),
            ),
            "win64" => (
                vec!["rbx".to_owned(), "rbp".to_owned(), "rdi".to_owned(),
                     "rsi".to_owned(), "r12".to_owned(), "r13".to_owned(),
                     "r14".to_owned(), "r15".to_owned()],
                "default rel".to_owned(),
            ),
            other => return Err(HarnessError::UnsupportedAbi(other.to_owned())),
        };

        let source = Self::generate(&config, &abi_preserved_regs, &abi_decl);

        Ok(Self {
            source,
            config,
            abi_preserved_regs,
        })
    }

    fn generate(config: &HarnessConfig, preserved: &[String], _abi_decl: &str) -> String {
        let mut s = String::new();
        s.push_str("; Harness for ");
        s.push_str(&config.entry_symbol);
        s.push('\n');

        for (i, (_val, _exp)) in config.test_values.iter().zip(config.expected_outputs.iter()).enumerate() {
            s.push_str(&format!("; test[{}]\n", i));
            s.push_str("mov rdi, ");
            s.push_str(&format!("0x{:x}", i));
            s.push('\n');

            for reg in preserved {
                s.push_str(&format!("push {reg}\n"));
            }
            s.push_str(&format!("call {}\n", config.entry_symbol));
            for reg in preserved.iter().rev() {
                s.push_str(&format!("pop {reg}\n"));
            }
            s.push('\n');
        }

        s.push_str("ret\n");
        s
    }

    pub fn render_with(&self, _vars: &HashMap<String, String>) -> String {
        self.source.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_sysv64_creates_template() {
        let config = HarnessConfig {
            entry_symbol: "sum_i64".to_owned(),
            abi: "sysv64".to_owned(),
            test_values: vec![vec![1, 2], vec![3, 4]],
            expected_outputs: vec![vec![3], vec![7]],
            timeout_ms: 1000,
        };
        let tpl = HarnessTemplate::new(config).expect("create");
        assert!(!tpl.source.is_empty());
        assert!(tpl.source.contains("sum_i64"));
        assert!(tpl.abi_preserved_regs.contains(&"rbx".to_owned()));
    }

    #[test]
    fn harness_mismatch_returns_error() {
        let config = HarnessConfig {
            entry_symbol: "f".to_owned(),
            abi: "sysv64".to_owned(),
            test_values: vec![vec![1]],
            expected_outputs: vec![],
            timeout_ms: 1000,
        };
        let result = HarnessTemplate::new(config);
        assert!(matches!(result, Err(HarnessError::CountMismatch { .. })));
    }

    #[test]
    fn unsupported_abi_returns_error() {
        let config = HarnessConfig {
            entry_symbol: "f".to_owned(),
            abi: "arm32".to_owned(),
            test_values: vec![],
            expected_outputs: vec![],
            timeout_ms: 1000,
        };
        let result = HarnessTemplate::new(config);
        assert!(matches!(result, Err(HarnessError::UnsupportedAbi(_))));
    }
}
