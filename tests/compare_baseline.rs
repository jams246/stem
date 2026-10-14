//! Compare: baseline checks, read-only behavior and long lists.

mod common;

use common::{TempProject, code, flat_ifs, stderr, stdout};

const MODERATE: &str = "export function process(x: number) {\n  if (x) {}\n}\n";

fn setup(tag: &str, files: &[(&str, &str)]) -> TempProject {
    let p = TempProject::new(tag);
    for (path, content) in files {
        p.write(path, content);
    }
    p.baseline("base.json", &[]);
    p
}

fn names(report: &serde_json::Value, list: &str) -> usize {
    report[list].as_array().unwrap().len()
}

#[test]
fn baseline_with_another_metrics_version_should_exit_2() {
    let p = setup("cmp-version", &[("a.ts", MODERATE)]);
    let edited = p
        .read("base.json")
        .replace("\"metricsVersion\": 1", "\"metricsVersion\": 2");
    p.write("base.json", &edited);
    let output = p.stem(&["compare", "base.json"]);
    assert_eq!(
        (code(&output), stderr(&output).contains("metricsVersion")),
        (2, true)
    );
}

#[test]
fn baseline_with_another_schema_version_should_exit_2() {
    let p = setup("cmp-schema", &[("a.ts", MODERATE)]);
    let edited = p
        .read("base.json")
        .replace("\"schemaVersion\": 1", "\"schemaVersion\": 7");
    p.write("base.json", &edited);
    assert_eq!(code(&p.stem(&["compare", "base.json"])), 2);
}

#[test]
fn unreadable_baseline_should_exit_2() {
    let p = setup("cmp-unreadable", &[("a.ts", MODERATE)]);
    p.write("broken.json", "{ not json");
    let codes = (
        code(&p.stem(&["compare", "missing.json"])),
        code(&p.stem(&["compare", "broken.json"])),
    );
    assert_eq!(codes, (2, 2));
}

#[test]
fn ignore_flag_on_compare_should_exit_2() {
    let p = setup("cmp-flag", &[("a.ts", MODERATE)]);
    assert_eq!(code(&p.stem(&["compare", "base.json", "--ignore", "x"])), 2);
}

#[test]
fn compare_should_write_nothing() {
    let p = setup("cmp-readonly", &[("a.ts", MODERATE)]);
    let before = p.read("base.json");
    p.write("a.ts", &flat_ifs("process", 3));
    p.stem(&["compare", "base.json"]);
    let entries = std::fs::read_dir(&p.root).unwrap().count();
    assert_eq!((p.read("base.json"), entries), (before, 2));
}

#[test]
fn long_lists_should_show_ten_entries_and_a_count() {
    let files: Vec<(String, String)> = (0..12)
        .map(|i| (format!("f{i:02}.ts"), flat_ifs(&format!("f{i:02}"), 1)))
        .collect();
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let p = setup("cmp-long", &refs);
    for i in 0..12 {
        p.write(&format!("f{i:02}.ts"), &flat_ifs(&format!("f{i:02}"), 2));
    }
    let text = stdout(&p.stem(&["compare", "base.json"]));
    let (_, r) = p.compare_json("base.json");
    assert_eq!(
        (text.contains("  and 2 more\n"), names(&r, "harder")),
        (true, 12)
    );
}

#[test]
fn absolute_baseline_path_should_not_appear_in_any_output() {
    let p = setup("cmp-absolute", &[("a.ts", MODERATE)]);
    p.write("a.ts", &flat_ifs("process", 3));
    let absolute = p.path("base.json");
    let absolute = absolute.to_str().unwrap();
    let text = stdout(&p.stem(&["compare", absolute]));
    let json = stdout(&p.stem(&["compare", absolute, "--json"]));
    let root = p.root.to_str().unwrap();
    assert_eq!(
        (
            text.contains(root),
            json.contains(root),
            text.starts_with("stem compare: base.json\n")
        ),
        (false, false, true)
    );
}

#[test]
fn baseline_spelling_should_not_change_the_report() {
    let p = setup("cmp-spelling", &[("a.ts", MODERATE)]);
    let absolute = p.path("base.json");
    let relative = stdout(&p.stem(&["compare", "base.json"]));
    let dotted = stdout(&p.stem(&["compare", "./base.json"]));
    assert_eq!(
        (
            stdout(&p.stem(&["compare", absolute.to_str().unwrap()])),
            dotted
        ),
        (relative.clone(), relative)
    );
}
