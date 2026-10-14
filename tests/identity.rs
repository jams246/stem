//! Function identity: one fixture per row of the naming table.

use std::fs;
use std::path::Path;

use stem::analyze::{FileOutcome, analyze_source};

fn ids(name: &str, source: &str) -> Vec<String> {
    match analyze_source(name, source) {
        FileOutcome::Functions(functions) => functions
            .into_iter()
            .map(|f| f.id.split_once("::").unwrap().1.to_string())
            .collect(),
        FileOutcome::ParseError(message) => panic!("{name}: {message}"),
    }
}

#[test]
fn every_identity_fixture_should_produce_its_expected_ids() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/identity");
    let mut sources: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("ts"))
        .collect();
    sources.sort();
    let mut failures = Vec::new();
    for source in &sources {
        let name = source.file_name().unwrap().to_str().unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(source.with_extension("json")).unwrap())
                .unwrap();
        let want: Vec<String> = expected["ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        let got = ids(name, &fs::read_to_string(source).unwrap());
        if got != want {
            failures.push(format!("{name}: expected {want:?}, got {got:?}"));
        }
    }
    assert!(sources.len() >= 10, "fixtures missing");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn ids_should_not_change_when_lines_are_inserted_above() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/identity");
    let source = fs::read_to_string(dir.join("anonymous.ts")).unwrap();
    let shifted = format!("// header\n\nconst unrelated = 1;\n\n{source}");
    assert_eq!(ids("a.ts", &source), ids("a.ts", &shifted));
}

#[test]
fn line_numbers_should_follow_inserted_lines() {
    let lines = |src: &str| match analyze_source("a.ts", src) {
        FileOutcome::Functions(f) => f[0].line,
        FileOutcome::ParseError(m) => panic!("{m}"),
    };
    assert_eq!(lines("\n\nfunction a() {}\n"), 3);
}
