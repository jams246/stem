//! The agent loop end to end: scan, apply a patch that makes one function
//! harder to follow, compare, apply a fix, compare again.

mod common;

use common::{TempProject, fixture};

fn names_and_hints(report: &serde_json::Value) -> Vec<(String, String)> {
    report["harder"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["name"].as_str().unwrap().to_string(),
                e["hint"].as_str().unwrap_or("").to_string(),
            )
        })
        .collect()
}

#[test]
fn harder_patch_should_regress_and_fix_patch_should_pass() {
    let p = TempProject::new("agent-loop");
    p.copy_from(&fixture("projects/shop"));
    p.baseline("baseline.json", &[]);

    p.copy_from(&fixture("agent-loop/harder"));
    let (status, report) = p.compare_json("baseline.json");
    let harder = names_and_hints(&report);
    assert_eq!(status, 1);
    assert_eq!(harder.len(), 1);
    assert_eq!(harder[0].0, "applyDiscounts");
    assert!(
        harder[0]
            .1
            .starts_with("Nesting adds 10 of the 18 points, at lines 12-19."),
        "{}",
        harder[0].1
    );

    p.copy_from(&fixture("agent-loop/fix"));
    let (status, report) = p.compare_json("baseline.json");
    assert_eq!((status, report["result"].as_str()), (0, Some("pass")));
}
