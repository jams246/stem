//! `stem compare --json` output.

use super::compare::{Comparison, Entry};
use crate::SCHEMA_VERSION;
use crate::json::Json;
use crate::metric::Metric;
use crate::snapshot::{parse_errors_json, usage_gap_json};

fn entry_json(entry: &Entry<'_>) -> Json {
    let metrics = entry
        .changes
        .iter()
        .map(|c| {
            let value = Json::obj([
                ("before", c.before.map_or(Json::Null, Json::int)),
                (
                    "bandBefore",
                    c.band_before.map_or(Json::Null, |b| Json::str(b.key())),
                ),
                ("after", Json::int(c.after)),
                ("bandAfter", Json::str(c.band_after.key())),
            ]);
            (c.metric.key().to_string(), value)
        })
        .collect();
    let f = entry.function;
    Json::obj([
        ("id", Json::str(&f.id)),
        ("path", Json::str(f.path())),
        ("name", Json::str(f.name())),
        ("line", Json::int(f.line)),
        ("status", Json::str(entry.status.key())),
        ("metrics", Json::Obj(metrics)),
        (
            "usageGapEvidence",
            usage_gap_json(f.usage_gap_evidence.as_ref()),
        ),
        ("regression", Json::Bool(entry.regression)),
        (
            "hint",
            entry
                .hint
                .as_ref()
                .map_or(Json::Null, |h| Json::str(h.text())),
        ),
    ])
}

pub fn compare_json(c: &Comparison<'_>) -> Json {
    let metrics = Metric::ALL
        .iter()
        .map(|&m| {
            let value = Json::obj([
                ("before", Json::tenths(c.before.metric(m).score10)),
                ("after", Json::tenths(c.after.metric(m).score10)),
            ]);
            (m.key().to_string(), value)
        })
        .collect();
    let result = if c.regression() { "regression" } else { "pass" };
    Json::obj([
        ("schemaVersion", Json::int(SCHEMA_VERSION)),
        ("result", Json::str(result)),
        (
            "reasons",
            Json::Arr(c.reasons.iter().map(Json::str).collect()),
        ),
        (
            "changedCode",
            Json::obj([
                ("before", Json::opt_tenths(c.changed_before)),
                ("after", Json::opt_tenths(c.changed_after)),
                ("changed", Json::int(c.changed)),
                ("added", Json::int(c.added)),
                ("removed", Json::int(c.removed)),
            ]),
        ),
        (
            "overall",
            Json::obj([
                ("before", Json::tenths(c.before.score10)),
                ("after", Json::tenths(c.after.score10)),
                ("gradeBefore", Json::str(c.before.grade())),
                ("gradeAfter", Json::str(c.after.grade())),
            ]),
        ),
        ("metrics", Json::Obj(metrics)),
        (
            "functions",
            Json::obj([
                ("before", Json::int(c.before.functions)),
                ("after", Json::int(c.after.functions)),
            ]),
        ),
        (
            "harder",
            Json::Arr(c.harder.iter().map(entry_json).collect()),
        ),
        (
            "easier",
            Json::Arr(c.easier.iter().map(entry_json).collect()),
        ),
        ("configChanged", Json::Bool(c.config_changed)),
        ("notes", Json::Arr(c.notes.iter().map(Json::str).collect())),
        ("parseErrors", parse_errors_json(c.parse_errors)),
    ])
}
