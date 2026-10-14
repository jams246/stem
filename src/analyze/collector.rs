//! Walks a parsed file and records one entry per measured function.

use std::collections::HashSet;

use oxc_semantic::Scoping;
use oxc_span::Span;

use super::Increment;
use super::UsageGapEvidence;
use super::lines::LineIndex;

/// Raw measurements of one function before lengths and IDs are finalized.
pub(crate) struct Record {
    pub name: String,
    pub line: u32,
    pub own_start: u32,
    pub own_end: u32,
    pub cognitive: u32,
    pub increments: Vec<Increment>,
    pub parameters: u32,
    pub vocabulary: u32,
    pub usage_gap: (u32, Option<UsageGapEvidence>),
    pub hash: String,
}

/// Naming details supplied by the construct that holds a function.
pub(crate) struct Pending<'a> {
    /// Start offset of the function node this applies to.
    pub target: u32,
    pub segment: String,
    pub own_start: u32,
    pub line_pos: u32,
    pub self_name: Option<&'a str>,
    pub this_name: Option<&'a str>,
}

/// State of a function currently being walked.
pub(crate) struct Frame<'a> {
    record: usize,
    pub nesting: u32,
    pub self_names: Vec<&'a str>,
    pub this_name: Option<&'a str>,
    names: HashSet<&'a str>,
}

/// A naming scope: the module, a function or a class.
pub(crate) struct Scope {
    pub prefix: String,
    anonymous: u32,
}

/// What `begin_function` needs to know about the function node.
pub(crate) struct FunctionStart<'a> {
    pub span: Span,
    pub id: Option<&'a str>,
    pub parameters: u32,
    pub hash: String,
    pub usage_gap: (u32, Option<UsageGapEvidence>),
}

pub(crate) struct Collector<'a> {
    pub lines: &'a LineIndex,
    pub scoping: &'a Scoping,
    pub records: Vec<Record>,
    pub frames: Vec<Frame<'a>>,
    pub scopes: Vec<Scope>,
    pub pending: Option<Pending<'a>>,
    pub pending_object: Option<(u32, String)>,
    pub pending_class: Option<(u32, String)>,
    pub object_names: Vec<Option<String>>,
    /// Maps the start of an exported declaration to the start of `export`.
    pub export_start: Option<(u32, u32)>,
    pub logical_child: bool,
}

impl<'a> Collector<'a> {
    pub fn new(lines: &'a LineIndex, scoping: &'a Scoping) -> Self {
        Collector {
            lines,
            scoping,
            records: Vec::new(),
            frames: Vec::new(),
            scopes: vec![Scope {
                prefix: String::new(),
                anonymous: 0,
            }],
            pending: None,
            pending_object: None,
            pending_class: None,
            object_names: Vec::new(),
            export_start: None,
            logical_child: false,
        }
    }

    /// Start offset to use for a construct, widened to a wrapping `export`.
    pub fn owner_start(&self, start: u32) -> u32 {
        match self.export_start {
            Some((decl, export)) if decl == start => export,
            _ => start,
        }
    }

    /// Joins `segment` onto the current naming scope.
    pub fn qualify(&self, segment: &str) -> String {
        match self.scopes.last() {
            Some(scope) if !scope.prefix.is_empty() => format!("{}.{segment}", scope.prefix),
            _ => segment.to_string(),
        }
    }

    /// Next `<anonymous N>` segment in the current scope.
    pub fn anonymous_segment(&mut self) -> String {
        let n = match self.scopes.last_mut() {
            Some(scope) => {
                scope.anonymous += 1;
                scope.anonymous
            }
            None => 1,
        };
        format!("<anonymous {n}>")
    }

    pub fn push_scope(&mut self, prefix: String) {
        self.scopes.push(Scope {
            prefix,
            anonymous: 0,
        });
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn begin_function(&mut self, start: FunctionStart<'a>) {
        let pending = match &self.pending {
            Some(p) if p.target == start.span.start => self.pending.take(),
            _ => None,
        };
        let (segment, own_start, line_pos, self_name, this_name) = match pending {
            Some(p) => (p.segment, p.own_start, p.line_pos, p.self_name, p.this_name),
            None => {
                let own_start = self.owner_start(start.span.start);
                let segment = match start.id {
                    Some(id) => id.to_string(),
                    None => self.anonymous_segment(),
                };
                (segment, own_start, own_start, None, None)
            }
        };
        let name = self.qualify(&segment);
        let mut self_names: Vec<&'a str> = start.id.into_iter().collect();
        self_names.extend(self_name);
        let nesting = self.frames.last().map_or(0, |parent| parent.nesting + 1);
        self.records.push(Record {
            name: name.clone(),
            line: self.lines.line(line_pos),
            own_start,
            own_end: start.span.end,
            cognitive: 0,
            increments: Vec::new(),
            parameters: start.parameters,
            vocabulary: 0,
            usage_gap: start.usage_gap,
            hash: start.hash,
        });
        self.frames.push(Frame {
            record: self.records.len() - 1,
            nesting,
            self_names,
            this_name,
            names: HashSet::new(),
        });
        self.push_scope(name);
    }

    pub fn end_function(&mut self) {
        self.pop_scope();
        if let Some(frame) = self.frames.pop() {
            let count = frame
                .names
                .iter()
                .filter(|name| !frame.self_names.contains(name) && frame.this_name != Some(name))
                .count();
            if let Some(record) = self.records.get_mut(frame.record) {
                record.vocabulary = u32::try_from(count).unwrap_or(u32::MAX);
            }
        }
    }

    /// Records a name used by the innermost function, if any.
    pub fn add_name(&mut self, name: &'a str) {
        if let Some(frame) = self.frames.last_mut() {
            frame.names.insert(name);
        }
    }

    /// Adds `1 + nesting` points for a structure that is affected by nesting.
    pub fn add_nested_increment(&mut self, pos: u32, kind: &'static str) {
        let nesting = self.frames.last().map_or(0, |f| f.nesting);
        self.add_increment(pos, kind, nesting);
    }

    /// Adds one point that ignores nesting.
    pub fn add_flat_increment(&mut self, pos: u32, kind: &'static str) {
        self.add_increment(pos, kind, 0);
    }

    fn add_increment(&mut self, pos: u32, kind: &'static str, nesting: u32) {
        let Some(frame) = self.frames.last() else {
            return;
        };
        let line = self.lines.line(pos);
        if let Some(record) = self.records.get_mut(frame.record) {
            record.cognitive += 1 + nesting;
            record.increments.push(Increment {
                line,
                kind: kind.to_string(),
                base: 1,
                nesting,
            });
        }
    }

    /// Runs `f` one nesting level deeper.
    pub fn nested(&mut self, f: impl FnOnce(&mut Self)) {
        if let Some(frame) = self.frames.last_mut() {
            frame.nesting += 1;
        }
        f(self);
        if let Some(frame) = self.frames.last_mut() {
            frame.nesting = frame.nesting.saturating_sub(1);
        }
    }
}
