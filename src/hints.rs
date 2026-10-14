//! Fixed-template hints: what was found, then what to do and why.

use crate::analyze::{FunctionMetrics, Increment};
use crate::metric::{Band, Metric, Thresholds};

/// A two-sentence hint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hint {
    /// First sentence: what was found, with numbers and line ranges.
    pub found: String,
    /// Second sentence: what to do and why.
    pub action: &'static str,
}

impl Hint {
    /// Both sentences on one line.
    pub fn text(&self) -> String {
        format!("{} {}", self.found, self.action)
    }
}

/// The metric in the highest band; ties go to the earlier metric.
pub fn worst_metric(function: &FunctionMetrics, thresholds: &Thresholds) -> (Metric, Band) {
    let mut worst = (Metric::CognitiveComplexity, Band::Low);
    let mut first = true;
    for metric in Metric::ALL {
        let band = thresholds.band(metric, function.value(metric));
        if first || band > worst.1 {
            worst = (metric, band);
            first = false;
        }
    }
    worst
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    Nesting,
    Logical,
    Branching,
    Loops,
    Other,
}

const GROUPS: [Group; 5] = [
    Group::Nesting,
    Group::Logical,
    Group::Branching,
    Group::Loops,
    Group::Other,
];

fn base_group(kind: &str) -> Group {
    match kind {
        "&&" | "||" => Group::Logical,
        "if" | "else if" | "else" | "switch" | "ternary" => Group::Branching,
        "for" | "for...in" | "for...of" | "while" | "do...while" => Group::Loops,
        _ => Group::Other,
    }
}

/// Points of one group and the increments that carry them.
fn group_points(increments: &[Increment], group: Group) -> (u32, Vec<u32>) {
    let mut points = 0;
    let mut lines = Vec::new();
    for increment in increments {
        let share = if group == Group::Nesting {
            increment.nesting
        } else if base_group(&increment.kind) == group {
            increment.base
        } else {
            0
        };
        if share > 0 {
            points += share;
            lines.push(increment.line);
        }
    }
    (points, lines)
}

fn line_range(lines: &[u32]) -> String {
    let first = lines.iter().min().copied().unwrap_or(0);
    let last = lines.iter().max().copied().unwrap_or(0);
    if first == last {
        format!("at line {first}")
    } else {
        format!("at lines {first}-{last}")
    }
}

fn cognitive_hint(function: &FunctionMetrics) -> Hint {
    let total = function.value(Metric::CognitiveComplexity);
    let mut best = (Group::Nesting, 0, Vec::new());
    for group in GROUPS {
        let (points, lines) = group_points(&function.increments, group);
        if points > best.1 {
            best = (group, points, lines);
        }
    }
    let (group, n, lines) = best;
    let at = line_range(&lines);
    let (found, action) = match group {
        Group::Nesting => (
            format!("Nesting adds {n} of the {total} points, {at}."),
            "Return early instead of nesting, so the main path is not indented.",
        ),
        Group::Logical => (
            format!("Chains of && and || add {n} of the {total} points, {at}."),
            "Move each condition into a function whose name says what it checks.",
        ),
        Group::Branching => (
            format!("if, else and switch branches add {n} of the {total} points, {at}."),
            "Replace the chain with a lookup object, or move each decision into its own well-named function.",
        ),
        Group::Loops => (
            format!("Loops add {n} of the {total} points, {at}."),
            "Move the loop body into a well-named function.",
        ),
        Group::Other => (
            format!("catch blocks, recursion and labels add {n} of the {total} points."),
            "Split the function into smaller, well-named steps.",
        ),
    };
    Hint { found, action }
}

/// Hint for the function's worst metric, when that metric is above low.
pub fn hint(function: &FunctionMetrics, thresholds: &Thresholds) -> Option<Hint> {
    let (metric, band) = worst_metric(function, thresholds);
    if band == Band::Low {
        return None;
    }
    let value = function.value(metric);
    Some(match metric {
        Metric::CognitiveComplexity => cognitive_hint(function),
        Metric::Length => Hint {
            found: format!(
                "The function is {value} lines long, so a reader must scroll and remember earlier parts."
            ),
            action: "Move a section that does one job into a well-named function.",
        },
        Metric::Parameters => Hint {
            found: format!("Every caller must pass {value} values in the right order."),
            action: "Group related parameters into one options object.",
        },
        Metric::Vocabulary => Hint {
            found: format!("The reader must keep track of {value} different names."),
            action: "Split the function, or group related values into an object.",
        },
        Metric::UsageGap => Hint {
            found: format!("The largest gap leaves {value} statements between references."),
            action: "Review whether the declaration can move closer to its use, or related work can be grouped while preserving execution order.",
        },
    })
}
