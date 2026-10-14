//! `stem compare` text output.

use std::fmt::Write as _;

use super::compare::{Comparison, Entry, Status};
use super::format::{LEARN_LINE, column, location, metric_line, row};
use crate::metric::Metric;
use crate::score::{signed_tenths, tenths};

const LIST_LIMIT: usize = 10;

fn score_pair(before: Option<u32>, after: Option<u32>) -> (String, String) {
    let show = |v: Option<u32>| v.map_or_else(|| "n/a".to_string(), tenths);
    let values = format!("{} -> {}", show(before), show(after));
    let change = match (before, after) {
        (Some(b), Some(a)) => format!("({})", signed_tenths(a, b)),
        _ => String::new(),
    };
    (values, change)
}

fn count_change(before: usize, after: usize) -> String {
    if after >= before {
        format!("(+{})", after - before)
    } else {
        format!("(-{})", before - after)
    }
}

fn write_entry(out: &mut String, entry: &Entry<'_>, c: &Comparison<'_>) {
    let new = if entry.status == Status::Added {
        "  (new)"
    } else {
        ""
    };
    let _ = writeln!(out, "  {}{new}", location(entry.function));
    for change in &entry.changes {
        let line = metric_line(change.metric, change.before, change.after, &c.thresholds);
        let _ = writeln!(out, "    {line}");
    }
    if let Some(hint) = &entry.hint {
        let _ = writeln!(out, "    {}\n    {}", hint.found, hint.action);
    }
}

fn write_list(out: &mut String, title: &str, entries: &[Entry<'_>], c: &Comparison<'_>) {
    if entries.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n{title} ({})", entries.len());
    for entry in entries.iter().take(LIST_LIMIT) {
        write_entry(out, entry, c);
    }
    if entries.len() > LIST_LIMIT {
        let _ = writeln!(out, "  and {} more", entries.len() - LIST_LIMIT);
    }
}

/// Renders the comparison. `baseline` is the path as the user gave it.
pub fn compare_text(c: &Comparison<'_>, baseline: &str) -> String {
    let mut out = format!("stem compare: {baseline}\n\n");
    let (values, change) = score_pair(c.changed_before, c.changed_after);
    let counts = format!(
        "{} changed, {} added, {} removed",
        c.changed, c.added, c.removed
    );
    let _ = writeln!(out, "{}", row("Changed code", &values, &change, &counts));
    let (values, change) = score_pair(Some(c.before.score10), Some(c.after.score10));
    let grades = format!("grade {} -> {}", c.before.grade(), c.after.grade());
    let _ = writeln!(out, "{}", row("Overall", &values, &change, &grades));
    for metric in Metric::ALL {
        let before = c.before.metric(metric).score10;
        let after = c.after.metric(metric).score10;
        let (values, change) = score_pair(Some(before), Some(after));
        let line = format!("  {:<22}{}{change}", metric.label(), column(&values, 16));
        let _ = writeln!(out, "{}", line.trim_end());
    }
    let values = format!("{} -> {}", c.before.functions, c.after.functions);
    let change = count_change(c.before.functions, c.after.functions);
    let _ = writeln!(out, "{}", row("Functions", &values, &change, ""));

    write_list(&mut out, "Harder to follow", &c.harder, c);
    write_list(&mut out, "Easier to follow", &c.easier, c);
    if !c.parse_errors.is_empty() {
        let _ = writeln!(out, "\nCould not parse ({})", c.parse_errors.len());
        for e in c.parse_errors {
            let _ = writeln!(out, "  {}: {}", e.path, e.message);
        }
    }
    if !c.notes.is_empty() {
        out.push_str("\nNotes\n");
        for note in &c.notes {
            let _ = writeln!(out, "  {note}");
        }
    }
    out.push('\n');
    match c.reasons.split_first() {
        Some((first, rest)) => {
            let _ = writeln!(out, "Result: harder to follow. {first}");
            for reason in rest {
                let _ = writeln!(out, "  {reason}");
            }
        }
        None if c.harder.is_empty() && !c.easier.is_empty() => {
            out.push_str("Result: easier to follow. No regression.\n");
        }
        None => out.push_str("Result: no regression.\n"),
    }
    let _ = writeln!(out, "{LEARN_LINE}");
    out
}
