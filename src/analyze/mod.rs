//! Per-file measurement: parse, find functions, compute the four metrics.

mod collector;
mod flow;
mod hash;
pub mod lines;
mod naming;
mod visit;

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_parser::{ParseOptions, Parser};
use oxc_span::SourceType;

use crate::metric::Metric;
use collector::Collector;
use lines::LineIndex;

/// One Cognitive Complexity increment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Increment {
    pub line: u32,
    pub kind: String,
    pub base: u32,
    pub nesting: u32,
}

/// Measurements of one function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionMetrics {
    /// `<path>::<qualified name>`.
    pub id: String,
    pub line: u32,
    /// Values in [`Metric::ALL`] order.
    pub values: [u32; 4],
    pub increments: Vec<Increment>,
    pub hash: String,
}

impl FunctionMetrics {
    pub fn value(&self, metric: Metric) -> u32 {
        self.values[metric.index()]
    }

    /// Scoring weight: `max(1, length)`.
    pub fn weight(&self) -> u64 {
        u64::from(self.value(Metric::Length).max(1))
    }

    /// Path part of the ID.
    pub fn path(&self) -> &str {
        self.id
            .split_once("::")
            .map_or(self.id.as_str(), |(p, _)| p)
    }

    /// Qualified-name part of the ID.
    pub fn name(&self) -> &str {
        self.id
            .split_once("::")
            .map_or(self.id.as_str(), |(_, n)| n)
    }
}

/// Result of measuring one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileOutcome {
    Functions(Vec<FunctionMetrics>),
    ParseError(String),
}

/// Strips a UTF-8 BOM and converts CRLF to LF.
pub fn normalize(raw: &str) -> Cow<'_, str> {
    let text = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    if text.contains("\r\n") {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

/// Measures every function in one TypeScript file.
///
/// `path` is the project-relative path; it decides the dialect (`.tsx`
/// enables JSX) and prefixes every function ID.
pub fn analyze_source(path: &str, raw: &str) -> FileOutcome {
    let source = normalize(raw);
    let source = source.as_ref();
    let source_type = SourceType::from_path(Path::new(path)).unwrap_or_else(|_| SourceType::ts());
    let allocator = Allocator::default();
    let options = ParseOptions {
        preserve_parens: false,
        ..ParseOptions::default()
    };
    let parsed = Parser::new(&allocator, source, source_type)
        .with_options(options)
        .parse();
    let index = LineIndex::new(source);
    if let Some(first) = parsed.diagnostics.errors().next() {
        let message = match first.labels.first() {
            Some(label) => {
                let pos = label.offset();
                format!("line {}: {}", index.line(pos), first.message)
            }
            None => first.message.to_string(),
        };
        return FileOutcome::ParseError(message);
    }
    if parsed.fatal_error {
        return FileOutcome::ParseError("the file could not be parsed".to_string());
    }
    let mut collector = Collector::new(&index);
    collector.visit_program(&parsed.program);
    let records = collector.records;

    let mut comments: Vec<(u32, u32)> = parsed
        .program
        .comments
        .iter()
        .map(|c| (c.span.start, c.span.end))
        .collect();
    comments.sort_unstable();
    let offsets = lines::first_code_offsets(source, &comments);
    let spans: Vec<(u32, u32)> = records.iter().map(|r| (r.own_start, r.own_end)).collect();
    let lengths = lines::owned_line_counts(&offsets, &spans);

    let mut seen: HashMap<String, u32> = HashMap::new();
    let functions = records
        .into_iter()
        .zip(lengths)
        .map(|(record, length)| {
            let count = seen.entry(record.name.clone()).or_insert(0);
            *count += 1;
            let name = if *count == 1 {
                record.name
            } else {
                format!("{}~{}", record.name, count)
            };
            FunctionMetrics {
                id: format!("{path}::{name}"),
                line: record.line,
                values: [
                    record.cognitive,
                    length,
                    record.parameters,
                    record.vocabulary,
                ],
                increments: record.increments,
                hash: record.hash,
            }
        })
        .collect();
    FileOutcome::Functions(functions)
}
