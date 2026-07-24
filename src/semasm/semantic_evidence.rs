//! Thin SemASM semantic-evidence projection (Sei P1).
//!
//! Parses selected fields from the **raw** VerificationReport JSON without
//! replacing it. Hashing / seals continue to use the intact `raw_json` blob.

use serde::Deserialize;

/// Summary of SemASM semantic evidence slices VAA may require.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SemanticEvidenceSummary {
    pub alias: Option<AliasEvidenceSummary>,
    pub region_access: Option<RegionAccessSummary>,
    pub contract_expressions: Option<ContractExpressionSummary>,
    pub obligations: Vec<VerificationObligationSummary>,
}

/// Alias / region-relation slice (`alias_analysis`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasEvidenceSummary {
    pub model: String,
    pub status: String,
    pub unknown_memory_accesses: u64,
    pub obligation_count: usize,
}

/// Region-access slice (`region_access`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionAccessSummary {
    pub model: String,
    pub status: String,
    pub accesses_unknown: u64,
}

/// Contract-expression slice (`contract_expressions`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractExpressionSummary {
    pub model: String,
    pub status: String,
}

/// One unresolved caller obligation from `alias_analysis.unresolved_obligations`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationObligationSummary {
    pub kind: String,
    pub left: String,
    pub right: String,
    pub owner: String,
}

#[derive(Debug, Deserialize)]
struct ReportProjection {
    #[serde(default)]
    alias_analysis: Option<AliasProjection>,
    #[serde(default)]
    region_access: Option<RegionAccessProjection>,
    #[serde(default)]
    contract_expressions: Option<ContractExprProjection>,
}

#[derive(Debug, Deserialize)]
struct AliasProjection {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    unknown_memory_accesses: u64,
    #[serde(default)]
    unresolved_obligations: Vec<ObligationProjection>,
}

#[derive(Debug, Deserialize)]
struct ObligationProjection {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    left: String,
    #[serde(default)]
    right: String,
    #[serde(default)]
    owner: String,
}

#[derive(Debug, Deserialize)]
struct RegionAccessProjection {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    accesses_unknown: u64,
}

#[derive(Debug, Deserialize)]
struct ContractExprProjection {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

/// Project semantic evidence from intact SemASM report JSON.
#[must_use]
pub fn project_semantic_evidence(raw_json: &str) -> SemanticEvidenceSummary {
    let Ok(proj) = serde_json::from_str::<ReportProjection>(raw_json) else {
        return SemanticEvidenceSummary::default();
    };

    let mut obligations = Vec::new();
    let alias = proj.alias_analysis.map(|a| {
        let alias_obligations: Vec<_> = a
            .unresolved_obligations
            .into_iter()
            .map(|o| VerificationObligationSummary {
                kind: o.kind,
                left: o.left,
                right: o.right,
                owner: o.owner,
            })
            .collect();
        let obligation_count = alias_obligations.len();
        obligations = alias_obligations;
        AliasEvidenceSummary {
            model: a.model.unwrap_or_default(),
            status: a.status.unwrap_or_default(),
            unknown_memory_accesses: a.unknown_memory_accesses,
            obligation_count,
        }
    });

    let region_access = proj.region_access.map(|r| RegionAccessSummary {
        model: r.model.unwrap_or_default(),
        status: r.status.unwrap_or_default(),
        accesses_unknown: r.accesses_unknown,
    });

    let contract_expressions = proj
        .contract_expressions
        .map(|c| ContractExpressionSummary {
            model: c.model.unwrap_or_default(),
            status: c.status.unwrap_or_default(),
        });

    SemanticEvidenceSummary {
        alias,
        region_access,
        contract_expressions,
        obligations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_alias_and_obligations() {
        let json = r#"{
            "status": "verified_under_preconditions",
            "schema_version": "0.5",
            "alias_analysis": {
                "model": "region-affine-v1",
                "status": "passed_under_preconditions",
                "unknown_memory_accesses": 0,
                "unresolved_obligations": [
                    {"kind":"regions_disjoint","left":"src","right":"dst","owner":"caller"}
                ]
            },
            "region_access": {
                "model": "region-access-affine-v1",
                "status": "incomplete",
                "accesses_unknown": 2
            }
        }"#;
        let s = project_semantic_evidence(json);
        let alias = s.alias.expect("alias");
        assert_eq!(alias.model, "region-affine-v1");
        assert_eq!(alias.status, "passed_under_preconditions");
        assert_eq!(alias.obligation_count, 1);
        assert_eq!(s.obligations[0].owner, "caller");
        let ra = s.region_access.expect("region_access");
        assert_eq!(ra.model, "region-access-affine-v1");
        assert_eq!(ra.accesses_unknown, 2);
    }

    #[test]
    fn missing_slices_are_none() {
        let s = project_semantic_evidence(r#"{"status":"verified","schema_version":"0.5"}"#);
        assert!(s.alias.is_none());
        assert!(s.region_access.is_none());
        assert!(s.contract_expressions.is_none());
        assert!(s.obligations.is_empty());
    }
}
