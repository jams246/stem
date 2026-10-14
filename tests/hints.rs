//! Every hint template is rendered by a fixture with real numbers and lines.

mod common;

use common::{TempProject, fixture, stdout};

fn hint_lines(file: &str) -> String {
    let p = TempProject::new(&format!("hint-{file}"));
    p.write(
        file,
        &std::fs::read_to_string(fixture(&format!("hints/{file}"))).unwrap(),
    );
    stdout(&p.stem(&["scan"]))
}

fn assert_hint(file: &str, found: &str, action: &str) {
    let text = hint_lines(file);
    let expected = format!("    {found}\n    {action}\n");
    assert!(text.contains(&expected), "{file}:\n{text}");
}

#[test]
fn nesting_hint_should_name_points_and_lines() {
    assert_hint(
        "nesting.ts",
        "Nesting adds 6 of the 10 points, at lines 3-5.",
        "Return early instead of nesting, so the main path is not indented.",
    );
}

#[test]
fn logical_hint_should_name_points_and_lines() {
    assert_hint(
        "logical.ts",
        "Chains of && and || add 10 of the 10 points, at lines 2-5.",
        "Move each condition into a function whose name says what it checks.",
    );
}

#[test]
fn branching_hint_should_name_points_and_lines() {
    assert_hint(
        "branching.ts",
        "if, else and switch branches add 6 of the 7 points, at lines 2-11.",
        "Replace the chain with a lookup object, or move each decision into its own well-named function.",
    );
}

#[test]
fn loops_hint_should_name_points_and_lines() {
    assert_hint(
        "loops.ts",
        "Loops add 7 of the 7 points, at lines 3-11.",
        "Move the loop body into a well-named function.",
    );
}

#[test]
fn other_hint_should_name_points() {
    assert_hint(
        "other.ts",
        "catch blocks, recursion and labels add 6 of the 7 points.",
        "Split the function into smaller, well-named steps.",
    );
}

#[test]
fn length_hint_should_name_the_length() {
    assert_hint(
        "length.ts",
        "The function is 41 lines long, so a reader must scroll and remember earlier parts.",
        "Move a section that does one job into a well-named function.",
    );
}

#[test]
fn parameters_hint_should_name_the_count() {
    assert_hint(
        "parameters.ts",
        "Every caller must pass 5 values in the right order.",
        "Group related parameters into one options object.",
    );
}

#[test]
fn vocabulary_hint_should_name_the_count() {
    assert_hint(
        "vocabulary.ts",
        "The reader must keep track of 26 different names.",
        "Split the function, or group related values into an object.",
    );
}

#[test]
fn hints_should_be_plain_ascii() {
    let all: String = [
        "nesting.ts",
        "logical.ts",
        "branching.ts",
        "loops.ts",
        "other.ts",
    ]
    .iter()
    .map(|f| hint_lines(f))
    .collect();
    assert!(all.is_ascii());
}
