//! Cognitive Complexity increments for control flow.

use oxc_ast::ast::{
    CallExpression, CatchClause, ConditionalExpression, DoWhileStatement, Expression,
    ForInStatement, ForOfStatement, ForStatement, IfStatement, LabelIdentifier, LogicalExpression,
    Statement, SwitchStatement, WhileStatement,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::GetSpan;
use oxc_syntax::operator::LogicalOperator;

use super::collector::Collector;

fn collect_operators(expr: &LogicalExpression<'_>, out: &mut Vec<(LogicalOperator, u32)>) {
    if let Expression::LogicalExpression(left) = &expr.left {
        collect_operators(left, out);
    }
    out.push((expr.operator, expr.right.span().start));
    if let Expression::LogicalExpression(right) = &expr.right {
        collect_operators(right, out);
    }
}

impl<'a> Collector<'a> {
    pub fn if_chain(&mut self, it: &IfStatement<'a>, else_if: bool) {
        if else_if {
            self.add_flat_increment(it.span.start, "else if");
        } else {
            self.add_nested_increment(it.span.start, "if");
        }
        self.visit_expression(&it.test);
        self.nested(|c| c.visit_statement(&it.consequent));
        match &it.alternate {
            Some(Statement::IfStatement(alternate)) => self.if_chain(alternate, true),
            Some(alternate) => {
                self.add_flat_increment(alternate.span().start, "else");
                self.nested(|c| c.visit_statement(alternate));
            }
            None => {}
        }
    }

    pub fn conditional(&mut self, it: &ConditionalExpression<'a>) {
        self.add_nested_increment(it.span.start, "ternary");
        self.visit_expression(&it.test);
        self.nested(|c| {
            c.visit_expression(&it.consequent);
            c.visit_expression(&it.alternate);
        });
    }

    pub fn switch(&mut self, it: &SwitchStatement<'a>) {
        self.add_nested_increment(it.span.start, "switch");
        self.visit_expression(&it.discriminant);
        self.nested(|c| c.visit_switch_cases(&it.cases));
    }

    pub fn for_loop(&mut self, it: &ForStatement<'a>) {
        self.add_nested_increment(it.span.start, "for");
        if let Some(init) = &it.init {
            self.visit_for_statement_init(init);
        }
        if let Some(test) = &it.test {
            self.visit_expression(test);
        }
        if let Some(update) = &it.update {
            self.visit_expression(update);
        }
        self.nested(|c| c.visit_statement(&it.body));
    }

    pub fn for_in(&mut self, it: &ForInStatement<'a>) {
        self.add_nested_increment(it.span.start, "for...in");
        self.visit_for_statement_left(&it.left);
        self.visit_expression(&it.right);
        self.nested(|c| c.visit_statement(&it.body));
    }

    pub fn for_of(&mut self, it: &ForOfStatement<'a>) {
        self.add_nested_increment(it.span.start, "for...of");
        self.visit_for_statement_left(&it.left);
        self.visit_expression(&it.right);
        self.nested(|c| c.visit_statement(&it.body));
    }

    pub fn while_loop(&mut self, it: &WhileStatement<'a>) {
        self.add_nested_increment(it.span.start, "while");
        self.visit_expression(&it.test);
        self.nested(|c| c.visit_statement(&it.body));
    }

    pub fn do_while(&mut self, it: &DoWhileStatement<'a>) {
        self.add_nested_increment(it.span.start, "do...while");
        self.nested(|c| c.visit_statement(&it.body));
        self.visit_expression(&it.test);
    }

    pub fn catch_clause(&mut self, it: &CatchClause<'a>) {
        self.add_nested_increment(it.span.start, "catch");
        if let Some(param) = &it.param {
            self.visit_catch_parameter(param);
        }
        self.nested(|c| c.visit_block_statement(&it.body));
    }

    /// Counts one point per run of the same `&&` or `||` operator.
    pub fn logical(&mut self, it: &LogicalExpression<'a>) {
        if !self.logical_child {
            let mut operators = Vec::new();
            collect_operators(it, &mut operators);
            let mut previous = None;
            for (operator, pos) in operators {
                let kind = match operator {
                    LogicalOperator::And => "&&",
                    LogicalOperator::Or => "||",
                    LogicalOperator::Coalesce => continue,
                };
                if previous != Some(operator) {
                    self.add_flat_increment(pos, kind);
                }
                previous = Some(operator);
            }
        }
        for side in [&it.left, &it.right] {
            let saved = self.logical_child;
            self.logical_child = matches!(side, Expression::LogicalExpression(_));
            self.visit_expression(side);
            self.logical_child = saved;
        }
    }

    /// Labeled `break` and `continue` add one point each.
    pub fn jump(&mut self, pos: u32, label: Option<&LabelIdentifier<'a>>, kind: &'static str) {
        if label.is_some() {
            self.add_flat_increment(pos, kind);
        }
    }

    /// Direct recursion adds one point, then the call is walked as usual.
    pub fn call(&mut self, it: &CallExpression<'a>) {
        let recursive = self.frames.last().is_some_and(|frame| match &it.callee {
            Expression::Identifier(id) => frame.self_names.contains(&id.name.as_str()),
            Expression::StaticMemberExpression(member) => {
                matches!(member.object, Expression::ThisExpression(_))
                    && frame.this_name == Some(member.property.name.as_str())
            }
            _ => false,
        });
        if recursive {
            self.add_flat_increment(it.span.start, "recursion");
        }
        walk::walk_call_expression(self, it);
    }
}
