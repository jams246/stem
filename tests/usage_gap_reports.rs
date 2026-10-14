//! Usage gap participates in scoring, reports, config and baseline contracts.

mod common;

use common::{TempProject, code, stdout};
use serde_json::Value;

const CONFIG: &str = r#"{"thresholds":{"usageGap":[0,1,3]}}"#;
const GROUPED: &str = "function total() {\none();\ntwo();\nlet amount = subtotal;\namount -= discount;\nreturn amount;\n}\n";
const SEPARATED: &str = "function total() {\nlet amount = subtotal;\none();\ntwo();\namount -= discount;\nreturn amount;\n}\n";

#[test]
fn snapshot_should_round_trip_usage_gap_and_evidence() {
    let p = TempProject::new("gap-snapshot");
    p.write("a.ts", SEPARATED).write("stem.config.json", CONFIG);
    let snapshot = p.snapshot(&[]);
    let f = &snapshot["functions"][0];
    assert_eq!(f["usageGap"], 2);
    assert_eq!(
        f["usageGapEvidence"],
        serde_json::json!({
            "name":"amount", "declarationLine":2, "fromLine":2, "toLine":5,
        })
    );
    assert_eq!(
        (
            snapshot["schemaVersion"].clone(),
            snapshot["metricsVersion"].clone()
        ),
        (Value::from(2), Value::from(3))
    );
    p.baseline("base.json", &[]);
    assert_eq!(p.compare_json("base.json").0, 0);
}

#[test]
fn locality_only_change_should_affect_score_regression_and_text() {
    let p = TempProject::new("gap-compare");
    p.write("a.ts", GROUPED).write("stem.config.json", CONFIG);
    p.baseline("base.json", &[]);
    p.write("a.ts", SEPARATED);
    let (status, report) = p.compare_json("base.json");
    assert_eq!(status, 1);
    assert_eq!(
        report["metrics"]["usageGap"],
        serde_json::json!({"before":100.0,"after":0.0})
    );
    assert_eq!(report["overall"]["after"], 80.0);
    assert_eq!(report["harder"][0]["metrics"].as_object().unwrap().len(), 1);
    assert_eq!(report["harder"][0]["usageGapEvidence"]["name"], "amount");
    let text = stdout(&p.stem(&["compare", "base.json"]));
    assert!(
        text.contains("usage gap 0 -> 2 (high, aim for 1 or less)"),
        "{text}"
    );
    assert!(
        text.contains("amount: references at lines 2 and 5; declared at line 2."),
        "{text}"
    );
    assert!(text.contains("preserving execution order"), "{text}");
    let scan = p.scan_json(&[]);
    assert_eq!(scan["top"][0]["metrics"]["usageGap"]["value"], 2);
    assert_eq!(
        scan["top"][0]["usageGapEvidence"],
        report["harder"][0]["usageGapEvidence"]
    );
}

#[test]
fn comparison_should_use_baseline_thresholds_and_report_improvement() {
    let p = TempProject::new("gap-improvement");
    p.write("a.ts", SEPARATED).write("stem.config.json", CONFIG);
    p.baseline("base.json", &[]);
    let before = p.read("base.json");
    p.write("a.ts", GROUPED);
    p.write(
        "stem.config.json",
        r#"{"thresholds":{"usageGap":[10,20,30]}}"#,
    );
    let (status, report) = p.compare_json("base.json");
    assert_eq!(
        (status, report["configChanged"].clone()),
        (0, Value::Bool(true))
    );
    assert_eq!(
        report["easier"][0]["metrics"]["usageGap"]["bandBefore"],
        "high"
    );
    assert!(report["easier"][0]["usageGapEvidence"].is_null());
    assert_eq!(p.read("base.json"), before);
}

#[test]
fn incomplete_new_baselines_should_be_rejected_instead_of_assuming_zero() {
    for field in ["usageGap", "usageGapEvidence"] {
        let p = TempProject::new(field);
        p.write("a.ts", SEPARATED);
        let mut snapshot = p.snapshot(&[]);
        snapshot["functions"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        p.write("bad.json", &snapshot.to_string());
        assert_eq!(code(&p.stem(&["compare", "bad.json"])), 2, "{field}");
    }
}

#[test]
fn scan_should_be_deterministic_including_evidence() {
    let p = TempProject::new("gap-determinism");
    p.write(
        "a.ts",
        "function f() { let a = 0, b = 0; one(); two(); use(a, b); }",
    );
    p.write("stem.config.json", CONFIG);
    let first = p.scan_json(&[]);
    for _ in 0..5 {
        assert_eq!(p.scan_json(&[]), first);
    }
}
