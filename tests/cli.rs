//! Commands, flags and exit codes.

mod common;

use common::{TempProject, code, stderr, stdout};

const FN: &str = "export function f() {\n  return 1;\n}\n";

#[test]
fn version_should_print_and_exit_0() {
    let p = TempProject::new("cli-version");
    let output = p.stem(&["--version"]);
    assert_eq!(
        (code(&output), stdout(&output)),
        (0, "stem 0.1.0\n".to_string())
    );
}

#[test]
fn help_should_list_the_commands_and_exit_0() {
    let p = TempProject::new("cli-help");
    let output = p.stem(&["--help"]);
    let text = stdout(&output);
    let listed = ["scan", "compare", "explain"]
        .iter()
        .all(|c| text.contains(c));
    assert_eq!((code(&output), listed), (0, true));
}

#[test]
fn unknown_flag_should_exit_2() {
    let p = TempProject::new("cli-flag");
    p.write("a.ts", FN);
    assert_eq!(code(&p.stem(&["scan", "--nope"])), 2);
}

#[test]
fn missing_command_should_exit_2() {
    let p = TempProject::new("cli-none");
    assert_eq!(code(&p.stem(&[])), 2);
}

#[test]
fn output_flag_should_create_parent_folders() {
    let p = TempProject::new("cli-output");
    p.write("a.ts", FN);
    let output = p.stem(&["scan", "-o", "reports/nested/base.json"]);
    assert_eq!(
        (code(&output), p.path("reports/nested/base.json").is_file()),
        (0, true)
    );
}

#[test]
fn scan_without_output_should_write_nothing() {
    let p = TempProject::new("cli-readonly");
    p.write("a.ts", FN);
    p.stem(&["scan"]);
    assert_eq!(std::fs::read_dir(&p.root).unwrap().count(), 1);
}

#[test]
fn scan_should_take_a_folder_argument() {
    let p = TempProject::new("cli-dir");
    p.write("app/a.ts", FN);
    let output = p.stem(&["scan", "app", "--json"]);
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["summary"]["functions"], 1);
}

#[test]
fn parse_errors_should_be_reported_not_fatal() {
    let p = TempProject::new("cli-parse");
    p.write("a.ts", FN)
        .write("broken.ts", "export function (\n");
    let json = p.scan_json(&[]);
    let errors = json["files"]["parseErrors"].as_array().unwrap();
    assert_eq!(
        (
            errors.len(),
            errors[0]["path"].as_str(),
            json["summary"]["functions"].as_u64()
        ),
        (1, Some("broken.ts"), Some(1))
    );
}

#[test]
fn invalid_config_should_exit_2_naming_the_key() {
    let p = TempProject::new("cli-config");
    p.write("a.ts", FN).write(
        "stem.config.json",
        r#"{ "thresholds": { "length": [30, 20, 10] } }"#,
    );
    let output = p.stem(&["scan"]);
    assert_eq!(
        (code(&output), stderr(&output).contains("thresholds.length")),
        (2, true)
    );
}

#[test]
fn config_thresholds_should_be_used_and_stored() {
    let p = TempProject::new("cli-thresholds");
    p.write("a.ts", FN).write(
        "stem.config.json",
        "// tuned\n{ \"thresholds\": { \"length\": [1, 2, 3,], }, }\n",
    );
    let snap = p.snapshot(&[]);
    assert_eq!(
        (
            snap["config"]["thresholds"]["length"].clone(),
            snap["summary"]["metrics"]["length"]["lines"]["high"].clone()
        ),
        (serde_json::json!([1, 2, 3]), serde_json::json!(3))
    );
}

#[test]
fn scan_json_should_be_the_only_stdout() {
    let p = TempProject::new("cli-json");
    p.write("a.ts", FN);
    let output = p.stem(&["scan", "--json", "-o", "base.json"]);
    let parsed: Result<serde_json::Value, _> = serde_json::from_slice(&output.stdout);
    assert!(parsed.is_ok() && p.path("base.json").is_file());
}
