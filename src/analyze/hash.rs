//! SHA-256 of a function's canonical form.
//!
//! The canonical form is the function's own syntax tree: node kinds, names,
//! literal values and operators. Comments, whitespace, quote style,
//! parentheses, semicolons and trailing commas never reach the tree, so
//! formatting changes keep the hash. Nested functions become one placeholder.

use std::fmt::Write as _;

use oxc_ast::AstKind;
use oxc_ast::ast::{
    ArrowFunctionExpression, Expression, Function, JSXExpression, JSXExpressionContainer, JSXText,
    PropertyKey, TSParenthesizedType,
};
use oxc_ast_visit::{Visit, walk};
use oxc_syntax::scope::ScopeFlags;
use sha2::{Digest, Sha256};

struct Hasher {
    sha: Sha256,
    buf: String,
}

impl Hasher {
    fn new() -> Self {
        Hasher {
            sha: Sha256::new(),
            buf: String::new(),
        }
    }

    fn emit(&mut self, text: &str) {
        self.sha.update(text.as_bytes());
        self.sha.update([0u8]);
    }

    fn emit_fmt(&mut self, args: std::fmt::Arguments<'_>) {
        self.buf.clear();
        // Writing to a String cannot fail.
        let _ = self.buf.write_fmt(args);
        self.sha.update(self.buf.as_bytes());
        self.sha.update([0u8]);
    }

    fn finish(self) -> String {
        let digest = self.sha.finalize();
        let mut out = String::with_capacity(7 + digest.len() * 2);
        out.push_str("sha256:");
        for byte in digest.iter() {
            let _ = write!(out, "{byte:02x}");
        }
        out
    }

    fn emit_values(&mut self, kind: AstKind<'_>) {
        match kind {
            AstKind::IdentifierName(x) => self.emit(x.name.as_str()),
            AstKind::IdentifierReference(x) => self.emit(x.name.as_str()),
            AstKind::BindingIdentifier(x) => self.emit(x.name.as_str()),
            AstKind::LabelIdentifier(x) => self.emit(x.name.as_str()),
            AstKind::PrivateIdentifier(x) => self.emit(x.name.as_str()),
            AstKind::JSXIdentifier(x) => self.emit(x.name.as_str()),
            AstKind::StringLiteral(x) => self.emit(x.value.as_str()),
            AstKind::NumericLiteral(x) => self.emit_fmt(format_args!("{:x}", x.value.to_bits())),
            AstKind::BigIntLiteral(x) => self.emit(x.value.as_str()),
            AstKind::BooleanLiteral(x) => self.emit(if x.value { "true" } else { "false" }),
            AstKind::RegExpLiteral(x) => {
                self.emit(x.regex.pattern.text.as_str());
                self.emit_fmt(format_args!("{:?}", x.regex.flags));
            }
            AstKind::TemplateElement(x) => self.emit(x.value.raw.as_str()),
            AstKind::Directive(x) => self.emit(x.directive.as_str()),
            AstKind::BinaryExpression(x) => self.emit(x.operator.as_str()),
            AstKind::LogicalExpression(x) => self.emit(x.operator.as_str()),
            AstKind::AssignmentExpression(x) => self.emit(x.operator.as_str()),
            AstKind::UnaryExpression(x) => self.emit(x.operator.as_str()),
            AstKind::UpdateExpression(x) => {
                self.emit_fmt(format_args!("{}{}", x.operator.as_str(), x.prefix));
            }
            AstKind::VariableDeclaration(x) => self.emit(x.kind.as_str()),
            AstKind::Function(x) => self.emit_fmt(format_args!("{}{}", x.r#async, x.generator)),
            AstKind::ArrowFunctionExpression(x) => self.emit_fmt(format_args!("{}", x.r#async)),
            AstKind::MethodDefinition(x) => {
                self.emit_fmt(format_args!("{:?}{}{}", x.kind, x.r#static, x.computed));
            }
            AstKind::PropertyDefinition(x) => {
                self.emit_fmt(format_args!("{}{}", x.r#static, x.computed));
            }
            AstKind::ObjectProperty(x) => self.emit_fmt(format_args!(
                "{:?}{}{}{}",
                x.kind, x.method, x.shorthand, x.computed
            )),
            AstKind::StaticMemberExpression(x) => self.emit_fmt(format_args!("{}", x.optional)),
            AstKind::ComputedMemberExpression(x) => self.emit_fmt(format_args!("{}", x.optional)),
            AstKind::CallExpression(x) => self.emit_fmt(format_args!("{}", x.optional)),
            AstKind::ForOfStatement(x) => self.emit_fmt(format_args!("{}", x.r#await)),
            AstKind::YieldExpression(x) => self.emit_fmt(format_args!("{}", x.delegate)),
            _ => {}
        }
    }
}

impl<'a> Visit<'a> for Hasher {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        self.emit_fmt(format_args!("{:?}", kind.ty()));
        self.emit_values(kind);
    }

    fn leave_node(&mut self, _kind: AstKind<'a>) {
        self.emit(")");
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {
        self.emit("<fn>");
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {
        self.emit("<fn>");
    }

    fn visit_property_key(&mut self, it: &PropertyKey<'a>) {
        // `a: 1` and `'a': 1` are the same key; formatters switch between them.
        match it {
            PropertyKey::StaticIdentifier(id) => {
                self.emit("Key");
                self.emit(id.name.as_str());
            }
            PropertyKey::StringLiteral(s) => {
                self.emit("Key");
                self.emit(s.value.as_str());
            }
            _ => walk::walk_property_key(self, it),
        }
    }

    fn visit_jsx_text(&mut self, it: &JSXText<'a>) {
        // Formatters reflow JSX text; only its words carry meaning here.
        let mut first = true;
        for word in it.value.as_str().split_whitespace() {
            if first {
                self.emit("JSXText");
                first = false;
            }
            self.emit(word);
        }
    }

    fn visit_jsx_expression_container(&mut self, it: &JSXExpressionContainer<'a>) {
        if let JSXExpression::StringLiteral(s) = &it.expression
            && s.value.as_str().trim().is_empty()
        {
            return;
        }
        walk::walk_jsx_expression_container(self, it);
    }

    fn visit_ts_parenthesized_type(&mut self, it: &TSParenthesizedType<'a>) {
        self.visit_ts_type(&it.type_annotation);
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Expression::ParenthesizedExpression(p) = it {
            self.visit_expression(&p.expression);
        } else {
            walk::walk_expression(self, it);
        }
    }
}

/// Hash of a function, method or function expression.
pub(crate) fn function_hash(it: &Function<'_>) -> String {
    let mut hasher = Hasher::new();
    walk::walk_function(&mut hasher, it, ScopeFlags::Function);
    hasher.finish()
}

/// Hash of an arrow function.
pub(crate) fn arrow_hash(it: &ArrowFunctionExpression<'_>) -> String {
    let mut hasher = Hasher::new();
    walk::walk_arrow_function_expression(&mut hasher, it);
    hasher.finish()
}
