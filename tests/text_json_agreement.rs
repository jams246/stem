//! Text and JSON output of the same run carry the same numbers.

mod common;

use common::{TempProject, fixture, stdout};
use serde_json::Value;

const BROKEN: &str = "export function broken( {\n";

fn shop(tag: &str) -> TempProject {
    let p = TempProject::new(tag);
    p.copy_from(&fixture("projects/shop"));
    p.copy_from(&fixture("hints"));
    p.write("tsconfig.json", r#"{ "include": ["src", "*.ts"] }"#);
    p.write("src/broken.ts", BROKEN);
    p
}

fn score(v: &Value) -> String {
    v.as_f64()
        .map_or_else(|| "n/a".to_string(), |f| format!("{f:.1}"))
}

fn tokens(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

fn find<'a>(text: &'a str, prefix: &str) -> &'a str {
    text.lines()
        .find(|l| l.starts_with(prefix))
        .unwrap_or_else(|| panic!("no {prefix}"))
}

/// `label value (band...)` or `label a -> b (band...)` inside an entry.
fn metric_value(text_block: &str, label: &str) -> Option<String> {
    let line = text_block
        .lines()
        .find(|l| l.trim_start().starts_with(&format!("{label} ")))?;
    let rest = line.trim_start().strip_prefix(label)?.trim_start();
    Some(rest.split(" (").next()?.to_string())
}

const METRICS: [(&str, &str); 4] = [
    ("cognitiveComplexity", "cognitive complexity"),
    ("length", "length"),
    ("parameters", "parameters"),
    ("vocabulary", "vocabulary"),
];

#[test]
fn scan_text_and_json_should_carry_the_same_numbers() {
    let p = shop("contract-scan");
    let text = stdout(&p.stem(&["scan"]));
    let json = p.scan_json(&[]);
    let s = &json["summary"];
    assert_eq!(
        tokens(find(&text, "Overall")),
        vec![
            "Overall",
            &score(&s["score"]),
            "grade",
            s["grade"].as_str().unwrap()
        ]
    );
    for (key, label) in METRICS {
        let line = find(&text, &format!("  {label}  "));
        assert_eq!(
            tokens(line.trim_start().strip_prefix(label).unwrap())[0],
            score(&s["metrics"][key]["score"])
        );
    }
    assert_eq!(
        tokens(find(&text, "Functions"))[1],
        s["functions"].to_string()
    );
    let files = format!(
        "{} scanned, {} ignored",
        json["files"]["scanned"], json["files"]["ignored"]
    );
    assert!(find(&text, "Files").ends_with(&files));
    let errors = json["files"]["parseErrors"].as_array().unwrap();
    assert!(text.contains(&format!(
        "Could not parse ({})\n  {}: ",
        errors.len(),
        errors[0]["path"].as_str().unwrap()
    )));
    let top = json["top"].as_array().unwrap();
    assert!(text.contains(&format!("Most to gain ({})", top.len())));
    for entry in top {
        let heading = format!(
            "  {}  {}:{}\n",
            entry["name"].as_str().unwrap(),
            entry["path"].as_str().unwrap(),
            entry["line"]
        );
        let start = text
            .find(&heading)
            .unwrap_or_else(|| panic!("missing {heading}"));
        let block: String = text[start + heading.len()..]
            .lines()
            .take_while(|l| l.starts_with("    "))
            .map(|l| format!("{l}\n"))
            .collect();
        let block = block.as_str();
        for (key, label) in METRICS {
            let m = &entry["metrics"][key];
            let shown = metric_value(block, label);
            let expected = (m["band"] != "low").then(|| m["value"].to_string());
            assert_eq!(shown, expected, "{heading}{label}");
        }
        if let Some(hint) = entry["hint"].as_str() {
            assert!(block.replace("\n    ", " ").contains(hint), "{hint}");
        }
    }
}

#[test]
fn compare_text_and_json_should_carry_the_same_numbers() {
    let p = shop("contract-compare");
    p.baseline("base.json", &[]);
    p.copy_from(&fixture("agent-loop/harder"));
    p.write(
        "src/util/strings.ts",
        "export const capitalize = (s: string) => s;\n",
    );
    let text = stdout(&p.stem(&["compare", "base.json"]));
    let (_, json) = p.compare_json("base.json");
    let c = &json["changedCode"];
    let row = tokens(find(&text, "Changed code"));
    assert_eq!(
        &row[2..5],
        [
            score(&c["before"]).as_str(),
            "->",
            score(&c["after"]).as_str()
        ]
    );
    assert!(find(&text, "Changed code").ends_with(&format!(
        "{} changed, {} added, {} removed",
        c["changed"], c["added"], c["removed"]
    )));
    let o = &json["overall"];
    let row = tokens(find(&text, "Overall"));
    assert_eq!(
        (row[1], row[3], row[6], row[8]),
        (
            score(&o["before"]).as_str(),
            score(&o["after"]).as_str(),
            o["gradeBefore"].as_str().unwrap(),
            o["gradeAfter"].as_str().unwrap()
        )
    );
    for (key, label) in METRICS {
        let row = tokens(
            find(&text, &format!("  {label}  "))
                .trim_start()
                .strip_prefix(label)
                .unwrap(),
        );
        assert_eq!(
            (row[0].to_string(), row[2].to_string()),
            (
                score(&json["metrics"][key]["before"]),
                score(&json["metrics"][key]["after"])
            )
        );
    }
    let row = tokens(find(&text, "Functions"));
    assert_eq!(
        (row[1], row[3]),
        (
            json["functions"]["before"].to_string().as_str(),
            json["functions"]["after"].to_string().as_str()
        )
    );
    for list in ["harder", "easier"] {
        for entry in json[list].as_array().unwrap() {
            let heading = format!(
                "  {}  {}:{}",
                entry["name"].as_str().unwrap(),
                entry["path"].as_str().unwrap(),
                entry["line"]
            );
            let start = text
                .find(&heading)
                .unwrap_or_else(|| panic!("missing {heading}"));
            let block = &text[start..];
            for (key, label) in METRICS {
                let Some(m) = entry["metrics"].get(key) else {
                    continue;
                };
                let expected = match m["before"].as_u64() {
                    Some(b) => format!("{b} -> {}", m["after"]),
                    None => m["after"].to_string(),
                };
                assert_eq!(
                    metric_value(block, label),
                    Some(expected),
                    "{heading} {label}"
                );
            }
        }
    }
    let first_reason = json["reasons"][0].as_str().unwrap();
    assert!(text.contains(&format!("Result: harder to follow. {first_reason}")));
}
