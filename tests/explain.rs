//! `stem explain`: golden output, size limits and unknown topics.

mod common;

use common::{TempProject, code, stderr, stdout};

const TOPICS: [Option<&str>; 7] = [
    None,
    Some("cognitive-complexity"),
    Some("length"),
    Some("parameters"),
    Some("vocabulary"),
    Some("usage-gap"),
    Some("score"),
];

fn explain(p: &TempProject, topic: Option<&str>) -> String {
    let output = match topic {
        Some(topic) => p.stem(&["explain", topic]),
        None => p.stem(&["explain"]),
    };
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    stdout(&output)
}

fn name(topic: Option<&str>) -> &str {
    topic.unwrap_or("overview")
}

#[test]
fn every_topic_should_match_its_golden_output_with_default_bands() {
    let p = TempProject::new("explain-default");
    for topic in TOPICS {
        insta::assert_snapshot!(format!("default_{}", name(topic)), explain(&p, topic));
    }
}

#[test]
fn every_topic_should_show_the_project_bands_with_a_config() {
    let p = TempProject::new("explain-config");
    p.write(
        "stem.config.json",
        r#"{ "thresholds": { "cognitiveComplexity": [8, 16, 32], "length": [20, 40, 80], "parameters": [2, 3, 5], "vocabulary": [12, 24, 36], "usageGap": [2, 5, 9] } }"#,
    );
    for topic in TOPICS {
        insta::assert_snapshot!(format!("config_{}", name(topic)), explain(&p, topic));
    }
}

#[test]
fn bands_should_follow_the_config() {
    let p = TempProject::new("explain-bands");
    p.write(
        "stem.config.json",
        r#"{ "thresholds": { "length": [20, 40, 80] } }"#,
    );
    let text = explain(&p, Some("length"));
    assert!(
        text.contains("low 0-20, moderate 21-40, high 41-80, very high 81+"),
        "{text}"
    );
}

#[test]
fn every_topic_should_fit_20_lines_of_80_ascii_columns() {
    let p = TempProject::new("explain-size");
    for topic in TOPICS {
        let text = explain(&p, topic);
        let lines: Vec<&str> = text.lines().collect();
        let widest = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        assert!(
            lines.len() <= 20 && widest <= 80 && text.is_ascii(),
            "{}: {} lines, {widest} columns",
            name(topic),
            lines.len()
        );
    }
}

#[test]
fn metric_topics_should_have_the_same_five_parts() {
    let p = TempProject::new("explain-parts");
    for topic in [
        "cognitive-complexity",
        "length",
        "parameters",
        "vocabulary",
        "usage-gap",
    ] {
        let text = explain(&p, Some(topic));
        let first = text.lines().next().unwrap_or("");
        let parts = [
            "\nCounts\n",
            "\nBands for this project\n",
            "\nWhy it matters\n",
            "\nTo improve\n",
        ];
        assert!(
            first.contains(": ") && parts.iter().all(|s| text.contains(s)),
            "{topic}"
        );
    }
}

#[test]
fn unknown_topic_should_exit_2_and_list_the_topics() {
    let p = TempProject::new("explain-unknown");
    let output = p.stem(&["explain", "unknown"]);
    let message = stderr(&output);
    let lists = [
        "cognitive-complexity",
        "length",
        "parameters",
        "vocabulary",
        "usage-gap",
        "score",
    ]
    .iter()
    .all(|t| message.contains(t));
    assert_eq!((code(&output), lists), (2, true), "{message}");
}
