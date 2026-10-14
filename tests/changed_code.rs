//! Empty projects and the changed-code score, end to end.

mod common;

use common::TempProject;

#[test]
fn empty_project_should_score_100_with_a_note() {
    let p = TempProject::new("empty-project");
    p.write(
        "types.ts",
        "export type A = { a: number };\nexport const x = 1;\n",
    );
    let report = p.scan_json(&[]);
    let summary = &report["summary"];
    let scores: Vec<f64> = ["cognitiveComplexity", "length", "parameters", "vocabulary"]
        .iter()
        .map(|k| summary["metrics"][k]["score"].as_f64().unwrap())
        .chain([summary["score"].as_f64().unwrap()])
        .collect();
    assert_eq!(scores, vec![100.0; 5]);
    assert_eq!(report["notes"], serde_json::json!(["No functions found."]));
}

#[test]
fn empty_project_text_should_say_no_functions_were_found() {
    let p = TempProject::new("empty-project-text");
    p.write("types.ts", "export type A = number;\n");
    let text = common::stdout(&p.stem(&["scan"]));
    assert!(text.contains("Overall        100.0  grade A") && text.contains("No functions found."));
}

const LOW_F: &str = "export function f() {\n  return 1;\n}\n";
const LOW_G: &str = "export function g() {\n  return 2;\n}\n";
const LOW_H: &str = "export function h() {\n  return 3;\n}\n";
const MODERATE_F: &str = "export function f(x: number) {\n  if (x) {}\n  if (x) {}\n  if (x) {}\n  if (x) {}\n  if (x) {}\n  if (x) {}\n}\n";

fn changed_code(report: &serde_json::Value) -> (Option<f64>, Option<f64>, [u64; 3]) {
    let c = &report["changedCode"];
    let counts = ["changed", "added", "removed"].map(|k| c[k].as_u64().unwrap());
    (c["before"].as_f64(), c["after"].as_f64(), counts)
}

#[test]
fn changed_code_with_only_additions_should_have_no_before() {
    let p = TempProject::new("cc-additions");
    p.write("f.ts", LOW_F);
    p.baseline("base.json", &[]);
    p.write("h.ts", LOW_H);
    let (_, report) = p.compare_json("base.json");
    assert_eq!(changed_code(&report), (None, Some(100.0), [0, 1, 0]));
}

#[test]
fn changed_code_with_only_removals_should_have_no_after() {
    let p = TempProject::new("cc-removals");
    p.write("f.ts", LOW_F).write("g.ts", LOW_G);
    p.baseline("base.json", &[]);
    p.remove("g.ts");
    let (_, report) = p.compare_json("base.json");
    assert_eq!(changed_code(&report), (Some(100.0), None, [0, 0, 1]));
}

#[test]
fn changed_code_with_both_should_score_each_side() {
    let p = TempProject::new("cc-both");
    p.write("f.ts", LOW_F).write("g.ts", LOW_G);
    p.baseline("base.json", &[]);
    p.write("f.ts", MODERATE_F).write("h.ts", LOW_H);
    p.remove("g.ts");
    let (status, report) = p.compare_json("base.json");
    // After: f has cognitive 6 (moderate) over 8 of 11 weighted lines:
    // 1000 - ceil(500 * 8 / 11) = 636, overall (636 + 3000 + 2) / 4 = 909.
    assert_eq!(
        (status, changed_code(&report)),
        (1, (Some(100.0), Some(90.9), [1, 1, 1]))
    );
}
