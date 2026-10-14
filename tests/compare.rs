//! Compare scenarios: report contents and exit codes.

mod common;

use common::{TempProject, flat_ifs, stdout};
use serde_json::Value;

const MODERATE: &str = "\
export function process(items: number[]) {
  for (const item of items) {
    if (item > 0) {
      log(item);
    }
  }
  if (items.length > 10) {}
  if (items.length > 20) {}
  if (items.length > 30) {}
}
";

const NESTED: &str = "\
export function process(items: number[]) {
  for (const item of items) {
    if (item > 0) {
      if (item > 1) {
        if (item > 2) {
          log(item);
        }
      }
    }
  }
  if (items.length > 10) {}
}
";

fn names(report: &Value, list: &str) -> Vec<String> {
    report[list]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect()
}

fn setup(tag: &str, files: &[(&str, &str)]) -> TempProject {
    let p = TempProject::new(tag);
    for (path, content) in files {
        p.write(path, content);
    }
    p.baseline("base.json", &[]);
    p
}

#[test]
fn no_code_change_should_pass_with_empty_lists() {
    let p = setup("cmp-none", &[("a.ts", MODERATE)]);
    let (status, r) = p.compare_json("base.json");
    let lists = (names(&r, "harder").len(), names(&r, "easier").len());
    assert_eq!(
        (status, lists, r["overall"]["before"].clone()),
        (0, (0, 0), r["overall"]["after"].clone())
    );
}

#[test]
fn moderate_function_entering_high_should_regress_first_with_a_nesting_hint() {
    let p = setup(
        "cmp-nesting",
        &[("a.ts", MODERATE), ("b.ts", &flat_ifs("aaa", 1))],
    );
    p.write("a.ts", NESTED).write("b.ts", &flat_ifs("aaa", 2));
    let (status, r) = p.compare_json("base.json");
    let first = &r["harder"][0];
    assert_eq!(
        (
            status,
            names(&r, "harder"),
            first["regression"].clone(),
            first["hint"].clone()
        ),
        (
            1,
            vec!["process".to_string(), "aaa".to_string()],
            Value::Bool(true),
            Value::from(
                "Nesting adds 6 of the 11 points, at lines 3-5. \
                 Return early instead of nesting, so the main path is not indented."
            ),
        )
    );
}

#[test]
fn moderate_function_entering_high_should_explain_itself_in_text() {
    let p = setup("cmp-nesting-text", &[("a.ts", MODERATE)]);
    p.write("a.ts", NESTED);
    let output = p.stem(&["compare", "base.json"]);
    let text = stdout(&output);
    let expected = "Harder to follow (1)\n  process  a.ts:1\n    \
        cognitive complexity 6 -> 11 (high, aim for 10 or less): how hard the logic is to follow\n    \
        length 10 -> 12 (low): lines of code in the function\n    \
        Nesting adds 6 of the 11 points, at lines 3-5.\n    \
        Return early instead of nesting, so the main path is not indented.\n";
    assert!(text.contains(expected), "{text}");
    assert!(text.contains(
        "Result: harder to follow. process entered the high band for cognitive complexity."
    ));
}

#[test]
fn new_low_function_should_pass() {
    let p = setup("cmp-new-low", &[("a.ts", MODERATE)]);
    p.write("b.ts", &flat_ifs("small", 2));
    assert_eq!(p.compare_json("base.json").0, 0);
}

#[test]
fn new_high_function_should_regress() {
    let p = setup("cmp-new-high", &[("a.ts", MODERATE)]);
    p.write("b.ts", &flat_ifs("big", 12));
    let (status, r) = p.compare_json("base.json");
    assert_eq!(
        (status, r["harder"][0]["status"].clone()),
        (1, Value::from("added"))
    );
}

#[test]
fn very_high_function_dropping_to_high_should_pass_as_easier() {
    let p = setup("cmp-drop", &[("a.ts", &flat_ifs("huge", 26))]);
    p.write("a.ts", &flat_ifs("huge", 20));
    let (status, r) = p.compare_json("base.json");
    assert_eq!((status, names(&r, "easier")), (0, vec!["huge".to_string()]));
}

#[test]
fn high_function_rising_within_high_should_regress() {
    let p = setup("cmp-rise", &[("a.ts", &flat_ifs("busy", 12))]);
    p.write("a.ts", &flat_ifs("busy", 14));
    let (status, r) = p.compare_json("base.json");
    assert_eq!(
        (status, r["reasons"][0].clone()),
        (
            1,
            Value::from(
                "busy rose from 12 to 14 for cognitive complexity, staying in the high band."
            )
        )
    );
}

#[test]
fn falling_changed_code_score_without_high_functions_should_regress() {
    let p = setup("cmp-fall", &[("a.ts", &flat_ifs("calm", 1))]);
    p.write("a.ts", &flat_ifs("calm", 7));
    let (status, r) = p.compare_json("base.json");
    let reason = r["reasons"][0].as_str().unwrap().to_string();
    assert_eq!(
        (status, reason.starts_with("The changed-code score fell")),
        (1, true)
    );
}

#[test]
fn loosened_thresholds_should_add_a_note_and_not_change_the_result() {
    let p = setup("cmp-loosen", &[("a.ts", &flat_ifs("busy", 12))]);
    p.write(
        "stem.config.json",
        r#"{ "thresholds": { "cognitiveComplexity": [20, 30, 40] } }"#,
    )
    .write("a.ts", &flat_ifs("busy", 14));
    let (status, r) = p.compare_json("base.json");
    assert_eq!(
        (status, r["configChanged"].clone(), r["notes"][0].clone()),
        (
            1,
            Value::Bool(true),
            Value::from("The config changed since the baseline; run stem scan again to adopt it.")
        )
    );
}

#[test]
fn ignore_added_after_baseline_should_still_compare_those_files() {
    let p = setup(
        "cmp-ignore",
        &[
            ("src/legacy/old.ts", &flat_ifs("old", 3)),
            ("src/a.ts", MODERATE),
        ],
    );
    p.write("stem.config.json", r#"{ "ignore": ["src/legacy"] }"#)
        .write("src/legacy/old.ts", &flat_ifs("old", 12));
    let (status, r) = p.compare_json("base.json");
    assert_eq!(
        (status, names(&r, "harder"), r["configChanged"].clone()),
        (1, vec!["old".to_string()], Value::Bool(true))
    );
}
