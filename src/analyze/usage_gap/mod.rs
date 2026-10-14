//! Local binding references and the largest statement gap between them.

mod intervals;
mod visit;

use std::collections::HashMap;

use oxc_ast::ast::FunctionBody;
use oxc_ast_visit::Visit;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use serde::Deserialize;

use super::lines::LineIndex;

/// One pair of references explaining a function's positive usage gap.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageGapEvidence {
    /// Local binding's source name.
    pub name: String,
    /// First declaration's 1-based line.
    pub declaration_line: u32,
    /// Earlier reference's 1-based line.
    pub from_line: u32,
    /// Later reference's 1-based line.
    pub to_line: u32,
}

struct Local<'a> {
    name: &'a str,
    declaration: Span,
    occurrences: Vec<Span>,
}

struct Locality<'a, 's> {
    scoping: &'s Scoping,
    body_span: Span,
    locals: HashMap<SymbolId, Local<'a>>,
    references: Vec<(SymbolId, Span)>,
    statements: Vec<Span>,
    declaring: bool,
}

struct Gap<'a> {
    local: &'a Local<'a>,
    from: Span,
    to: Span,
}

impl Gap<'_> {
    fn order(&self) -> (u32, u32, u32, &str) {
        (
            self.from.start,
            self.to.start,
            self.local.declaration.start,
            self.local.name,
        )
    }
}

pub(super) fn measure(
    body: Option<&FunctionBody<'_>>,
    scoping: &Scoping,
    lines: &LineIndex,
) -> (u32, Option<UsageGapEvidence>) {
    let Some(body) = body else { return (0, None) };
    let mut visitor = Locality {
        scoping,
        body_span: body.span,
        locals: HashMap::new(),
        references: Vec::new(),
        statements: Vec::new(),
        declaring: false,
    };
    visitor.visit_function_body(body);
    for (symbol, span) in visitor.references {
        if let Some(local) = visitor.locals.get_mut(&symbol) {
            local.occurrences.push(span);
        }
    }
    for local in visitor.locals.values_mut() {
        local.occurrences.sort_unstable_by_key(|s| (s.start, s.end));
        local.occurrences.dedup();
    }
    let gaps: Vec<_> = visitor
        .locals
        .values()
        .flat_map(|local| {
            local.occurrences.windows(2).map(|pair| Gap {
                local,
                from: pair[0],
                to: pair[1],
            })
        })
        .collect();
    let ranges: Vec<_> = gaps.iter().map(|g| (g.from.end, g.to.start)).collect();
    let counts = intervals::count(&mut visitor.statements, &ranges);
    let best = gaps
        .iter()
        .zip(counts)
        .filter(|(_, count)| *count > 0)
        .min_by(|(a, ac), (b, bc)| bc.cmp(ac).then_with(|| a.order().cmp(&b.order())));
    match best {
        None => (0, None),
        Some((gap, count)) => (
            count,
            Some(UsageGapEvidence {
                name: gap.local.name.to_string(),
                declaration_line: lines.line(gap.local.declaration.start),
                from_line: lines.line(gap.from.start),
                to_line: lines.line(gap.to.start),
            }),
        ),
    }
}
