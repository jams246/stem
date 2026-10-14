//! Ignoring files: config patterns, `--ignore` flags and built-ins.

mod common;

use common::{TempProject, code, paths, stderr};

const FN: &str = "export function f() {\n  return 1;\n}\n";

fn project(tag: &str, files: &[&str]) -> TempProject {
    let p = TempProject::new(tag);
    for file in files {
        p.write(file, FN);
    }
    p
}

#[test]
fn folder_pattern_should_ignore_everything_inside_it() {
    let p = project(
        "ig-folder",
        &["src/legacy/a.ts", "src/legacy/deep/b.ts", "src/c.ts"],
    );
    p.write("stem.config.json", r#"{ "ignore": ["src/legacy"] }"#);
    assert_eq!(paths(&p.snapshot(&[])), vec!["src/c.ts"]);
}

#[test]
fn rooted_pattern_should_not_match_a_nested_folder() {
    let p = project("ig-rooted", &["src/legacy/a.ts", "legacy/b.ts"]);
    let snap = p.snapshot(&["--ignore", "legacy/**"]);
    assert_eq!(paths(&snap), vec!["src/legacy/a.ts"]);
}

#[test]
fn leading_double_star_should_match_a_nested_folder() {
    let p = project("ig-anywhere", &["src/legacy/a.ts", "legacy/b.ts", "c.ts"]);
    let snap = p.snapshot(&["--ignore", "**/legacy/**"]);
    assert_eq!(paths(&snap), vec!["c.ts"]);
}

#[test]
fn matching_should_be_case_sensitive() {
    let p = project("ig-case", &["src/a.ts"]);
    let report = p.scan_json(&["--ignore", "Src/"]);
    assert_eq!(report["files"]["scanned"], 1);
}

#[test]
fn braces_and_character_classes_should_work() {
    let p = project("ig-glob", &["a.gen.ts", "b.auto.ts", "x.ts", "y.ts"]);
    let snap = p.snapshot(&["--ignore", "*.{gen,auto}.ts", "--ignore", "[xz].ts"]);
    assert_eq!(paths(&snap), vec!["y.ts"]);
}

#[test]
fn invalid_pattern_should_exit_2_and_name_it() {
    let p = project("ig-invalid", &["a.ts"]);
    let output = p.stem(&["scan", "--ignore", "src/[oops"]);
    assert_eq!(
        (code(&output), stderr(&output).contains("src/[oops")),
        (2, true)
    );
}

#[test]
fn invalid_config_pattern_should_exit_2() {
    let p = project("ig-invalid-config", &["a.ts"]);
    p.write("stem.config.json", r#"{ "ignore": ["{a,b"] }"#);
    assert_eq!(code(&p.stem(&["scan"])), 2);
}

#[test]
fn pattern_matching_nothing_should_add_a_note() {
    let p = project("ig-note", &["a.ts"]);
    let report = p.scan_json(&["--ignore", "src/legasy"]);
    let notes = report["notes"].to_string();
    assert!(notes.contains("src/legasy"), "{notes}");
}

#[test]
fn ignore_flags_should_be_stored_sorted_in_the_snapshot() {
    let p = project("ig-stored", &["a.ts", "b.ts", "c.ts", "d.ts"]);
    p.write("stem.config.json", r#"{ "ignore": ["c.ts"] }"#);
    let snap = p.snapshot(&["--ignore", "b.ts", "--ignore", "a.ts"]);
    assert_eq!(
        snap["config"]["ignore"],
        serde_json::json!(["a.ts", "b.ts", "c.ts"])
    );
}

#[test]
fn scanned_and_ignored_counts_should_be_correct() {
    let p = project(
        "ig-counts",
        &["src/a.ts", "src/b.ts", "src/old/c.ts", "src/x.test.ts"],
    );
    let report = p.scan_json(&["--ignore", "src/old"]);
    assert_eq!(
        (
            report["files"]["scanned"].as_u64(),
            report["files"]["ignored"].as_u64()
        ),
        (Some(2), Some(2))
    );
}

#[test]
fn scan_text_should_print_the_counts() {
    let p = project("ig-text", &["src/a.ts", "src/old/c.ts"]);
    let text = common::stdout(&p.stem(&["scan", "--ignore", "src/old"]));
    assert!(text.contains("1 scanned, 1 ignored"), "{text}");
}

#[test]
fn builtin_ignores_should_apply_even_to_explicit_tsconfig_files() {
    let p = project("ig-builtin", &["a.ts", "a.test.ts", "dist/b.ts"]);
    p.write(
        "tsconfig.json",
        r#"{ "files": ["a.ts", "a.test.ts", "dist/b.ts"], "exclude": [] }"#,
    );
    assert_eq!(paths(&p.snapshot(&[])), vec!["a.ts"]);
}

#[test]
fn builtin_ignores_should_not_be_configurable() {
    let p = project("ig-builtin-config", &["a.ts"]);
    p.write("stem.config.json", r#"{ "builtinIgnores": false }"#);
    assert_eq!(code(&p.stem(&["scan"])), 2);
}

#[test]
fn negation_should_be_rejected() {
    let p = project("ig-negation", &["a.ts"]);
    assert_eq!(code(&p.stem(&["scan", "--ignore", "!a.ts"])), 2);
}
