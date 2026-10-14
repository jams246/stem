//! Metric fixtures: every `.ts`/`.tsx` file in `tests/fixtures/metrics` has a
//! JSON file beside it with the expected values.

use std::fs;
use std::path::Path;

use stem::analyze::{FileOutcome, FunctionMetrics, analyze_source};
use stem::metric::Metric;

fn measure(path: &Path) -> Vec<FunctionMetrics> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let source = fs::read_to_string(path).unwrap();
    match analyze_source(name, &source) {
        FileOutcome::Functions(functions) => functions,
        FileOutcome::ParseError(message) => panic!("{name}: {message}"),
    }
}

fn check_fixture(source: &Path) -> Vec<String> {
    let expected: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(source.with_extension("json")).unwrap()).unwrap();
    let functions = measure(source);
    let file = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let mut failures = Vec::new();
    if let Some(count) = expected["count"].as_u64()
        && functions.len() as u64 != count
    {
        let ids: Vec<&str> = functions.iter().map(|f| f.id.as_str()).collect();
        failures.push(format!("{file}: expected {count} functions, found {ids:?}"));
    }
    for (name, values) in expected["functions"].as_object().unwrap() {
        let id = format!("{file}::{name}");
        let Some(function) = functions.iter().find(|f| f.id == id) else {
            failures.push(format!("{file}: missing {name}"));
            continue;
        };
        for metric in Metric::ALL {
            if let Some(want) = values[metric.key()].as_u64() {
                let got = u64::from(function.value(metric));
                if got != want {
                    failures.push(format!("{id}: {} expected {want}, got {got}", metric.key()));
                }
            }
        }
        if let Some(want) = values["weight"].as_u64()
            && function.weight() != want
        {
            failures.push(format!(
                "{id}: weight expected {want}, got {}",
                function.weight()
            ));
        }
    }
    failures
}

#[test]
fn every_metric_fixture_should_match_its_expected_values() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/metrics");
    let mut sources: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("ts" | "tsx")))
        .collect();
    sources.sort();
    assert!(sources.len() >= 17, "fixtures missing");
    let failures: Vec<String> = sources.iter().flat_map(|s| check_fixture(s)).collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn reference_example_should_count_every_increment() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/metrics");
    let functions = measure(&dir.join("shipping-cost.ts"));
    let kinds: Vec<(u32, &str, u32)> = functions[0]
        .increments
        .iter()
        .map(|i| (i.line, i.kind.as_str(), i.nesting))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (2, "if", 0),
            (4, "for...of", 0),
            (5, "if", 1),
            (7, "else", 0),
            (11, "ternary", 0),
            (11, "&&", 0),
        ]
    );
}
