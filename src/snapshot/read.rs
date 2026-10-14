//! Reading a baseline snapshot.

use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::analyze::{FunctionMetrics, Increment, UsageGapEvidence};
use crate::config::{self, Config};
use crate::error::StemError;
use crate::{METRICS_VERSION, SCHEMA_VERSION};

/// The parts of a baseline that `compare` uses.
#[derive(Clone, Debug)]
pub struct Baseline {
    pub config: Config,
    /// Functions sorted by ID in byte order.
    pub functions: Vec<FunctionMetrics>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawIncrement {
    line: u32,
    kind: String,
    base: u32,
    nesting: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFunction {
    id: String,
    line: u32,
    cognitive_complexity: u32,
    length: u32,
    parameters: u32,
    vocabulary: u32,
    usage_gap: u32,
    #[serde(deserialize_with = "Option::deserialize")]
    usage_gap_evidence: Option<UsageGapEvidence>,
    increments: Vec<RawIncrement>,
    hash: String,
}

fn version(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

/// Reads and checks a baseline file. `shown` names it in messages.
pub fn read_baseline(path: &Path, shown: &str) -> Result<Baseline, StemError> {
    let fail = |message: String| StemError::Baseline {
        path: shown.to_string(),
        message,
    };
    let text = fs::read_to_string(path).map_err(|e| StemError::read(shown, e))?;
    let value: Value =
        serde_json::from_str(&text).map_err(|e| fail(format!("not a stem snapshot ({e})")))?;
    for (key, ours) in [
        ("schemaVersion", SCHEMA_VERSION),
        ("metricsVersion", METRICS_VERSION),
    ] {
        match version(&value, key) {
            Some(theirs) if theirs == u64::from(ours) => {}
            Some(theirs) => {
                return Err(fail(format!(
                    "it has {key} {theirs} but this stem uses {ours}; run stem scan again to make a new baseline"
                )));
            }
            None => return Err(fail(format!("not a stem snapshot (no {key})"))),
        }
    }
    let config_value = value
        .get("config")
        .ok_or_else(|| fail("not a stem snapshot (no config)".to_string()))?;
    let config = config::from_value(config_value, shown)?;
    let raw: Vec<RawFunction> = value
        .get("functions")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| fail(format!("invalid functions list ({e})")))?
        .ok_or_else(|| fail("not a stem snapshot (no functions)".to_string()))?;
    let mut functions: Vec<FunctionMetrics> = raw
        .into_iter()
        .map(|f| FunctionMetrics {
            id: f.id,
            line: f.line,
            values: [
                f.cognitive_complexity,
                f.length,
                f.parameters,
                f.vocabulary,
                f.usage_gap,
            ],
            usage_gap_evidence: f.usage_gap_evidence,
            increments: f
                .increments
                .into_iter()
                .map(|i| Increment {
                    line: i.line,
                    kind: i.kind,
                    base: i.base,
                    nesting: i.nesting,
                })
                .collect(),
            hash: f.hash,
        })
        .collect();
    functions.sort_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));
    Ok(Baseline { config, functions })
}
