//! Output contracts: wording rules, metric lines, explain coverage, golden reports.

mod common;

use common::{TempProject, code, fixture, stderr, stdout};

const BROKEN: &str = "export function broken( {\n";

fn shop(tag: &str) -> TempProject {
    let p = TempProject::new(tag);
    p.copy_from(&fixture("projects/shop"));
    p.copy_from(&fixture("hints"));
    p.write("tsconfig.json", r#"{ "include": ["src", "*.ts"] }"#);
    p.write("src/broken.ts", BROKEN);
    p
}

fn report_texts(tag: &str) -> Vec<String> {
    let p = shop(tag);
    let scan = stdout(&p.stem(&["scan"]));
    p.baseline("base.json", &[]);
    p.copy_from(&fixture("agent-loop/harder"));
    let harder = stdout(&p.stem(&["compare", "base.json"]));
    p.copy_from(&fixture("agent-loop/fix"));
    let fixed = stdout(&p.stem(&["compare", "base.json"]));
    vec![scan, harder, fixed]
}

#[test]
fn reports_should_use_no_rule_ids_and_no_failure_words() {
    for text in report_texts("contract-words") {
        let lower = text.to_lowercase();
        let bad = ["error", "warning", "cognitive-complexity", "rule"];
        assert!(bad.iter().all(|w| !lower.contains(w)), "{text}");
        assert!(text.ends_with("Learn what a metric measures: stem explain <metric>\n"));
    }
}

#[test]
fn real_failures_should_say_error() {
    let p = TempProject::new("contract-failure");
    let output = p.stem(&["scan", "missing-folder"]);
    assert_eq!(
        (code(&output), stderr(&output).contains("error")),
        (2, true)
    );
}

#[test]
fn every_metric_line_should_carry_name_value_band_target_and_description() {
    let descriptions: Vec<(&str, &str)> = vec![
        ("cognitive complexity", "how hard the logic is to follow"),
        ("length", "lines of code in the function"),
        ("parameters", "values every caller must pass, in order"),
        (
            "vocabulary",
            "different names the reader must keep track of",
        ),
        (
            "usage gap",
            "most statements between consecutive references to a local variable",
        ),
    ];
    let mut checked = 0;
    for text in report_texts("contract-lines") {
        for line in text.lines().map(str::trim_start) {
            let Some((label, description)) = descriptions.iter().find(|(l, _)| {
                line.strip_prefix(&format!("{l} ")).is_some_and(|rest| {
                    rest.starts_with(|c: char| c.is_ascii_digit()) && rest.contains("): ")
                })
            }) else {
                continue;
            };
            let band_part = line
                .split(" (")
                .nth(1)
                .unwrap()
                .split("): ")
                .next()
                .unwrap();
            let band = band_part.split(',').next().unwrap();
            let has_target = band_part.contains(", aim for ") && band_part.ends_with(" or less");
            assert!(
                ["low", "moderate", "high", "very high"].contains(&band),
                "{line}"
            );
            assert_eq!(has_target, band != "low", "{line}");
            assert!(
                line.ends_with(&format!("): {description}")),
                "{line} / {label}"
            );
            checked += 1;
        }
    }
    assert!(checked >= 10, "only {checked} metric lines");
}

#[test]
fn every_metric_and_band_name_in_reports_should_appear_in_explain() {
    let p = TempProject::new("contract-explain");
    let explained: String = [
        "",
        "cognitive-complexity",
        "length",
        "parameters",
        "vocabulary",
        "usage-gap",
        "score",
    ]
    .iter()
    .map(|t| {
        if t.is_empty() {
            stdout(&p.stem(&["explain"]))
        } else {
            stdout(&p.stem(&["explain", t]))
        }
    })
    .collect::<String>()
    .to_lowercase();
    let reports = report_texts("contract-explain-reports").concat();
    let names = [
        "cognitive complexity",
        "length",
        "parameters",
        "vocabulary",
        "low",
        "moderate",
        "high",
        "very high",
        "grade",
        "changed code",
        "usage gap",
        "score",
    ];
    for name in names {
        if reports.to_lowercase().contains(name) {
            assert!(explained.contains(name), "{name} is not explained");
        }
    }
}

#[test]
fn reports_should_match_their_golden_output() {
    let texts = report_texts("contract-golden");
    insta::assert_snapshot!("scan_shop", texts[0]);
    insta::assert_snapshot!("compare_harder", texts[1]);
    insta::assert_snapshot!("compare_fixed", texts[2]);
}
