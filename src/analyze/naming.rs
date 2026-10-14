//! Decides the name segment of functions from the construct that holds them.

use oxc_ast::ast::{
    BindingPattern, Class, ExportDefaultDeclaration, ExportDefaultDeclarationKind, Expression,
    MethodDefinition, MethodDefinitionKind, ObjectProperty, PropertyDefinition, PropertyKey,
    PropertyKind, VariableDeclarator,
};
use oxc_span::GetSpan;

use super::collector::{Collector, Pending};

/// Start offset of `expr` when it is a function or arrow, looking through
/// parentheses and type assertions.
pub(crate) fn function_start(expr: &Expression<'_>) -> Option<u32> {
    match expr.get_inner_expression() {
        Expression::FunctionExpression(f) => Some(f.span.start),
        Expression::ArrowFunctionExpression(a) => Some(a.span.start),
        _ => None,
    }
}

fn object_start(expr: &Expression<'_>) -> Option<u32> {
    match expr.get_inner_expression() {
        Expression::ObjectExpression(o) => Some(o.span.start),
        _ => None,
    }
}

fn class_start(expr: &Expression<'_>) -> Option<u32> {
    match expr.get_inner_expression() {
        Expression::ClassExpression(c) => Some(c.span.start),
        _ => None,
    }
}

/// Name segment of a property key; `<computed>` for computed keys.
pub(crate) fn key_segment(key: &PropertyKey<'_>, computed: bool) -> String {
    if computed {
        return "<computed>".to_string();
    }
    match key {
        PropertyKey::PrivateIdentifier(p) => format!("#{}", p.name.as_str()),
        _ => key
            .static_name()
            .map_or_else(|| "<computed>".to_string(), |n| n.into_owned()),
    }
}

/// Identifier-like key text, used to recognize `this.name()` recursion.
fn key_ident<'a>(key: &PropertyKey<'a>, computed: bool) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(id) if !computed => Some(id.name.as_str()),
        _ => None,
    }
}

impl<'a> Collector<'a> {
    pub fn name_variable(&mut self, own_start: u32, it: &VariableDeclarator<'a>) {
        let BindingPattern::BindingIdentifier(id) = &it.id else {
            return;
        };
        let Some(init) = &it.init else {
            return;
        };
        let name = id.name.as_str();
        if let Some(target) = function_start(init) {
            self.pending = Some(Pending {
                target,
                segment: name.to_string(),
                own_start,
                line_pos: own_start,
                self_name: Some(name),
                this_name: None,
            });
        } else if let Some(target) = object_start(init) {
            self.pending_object = Some((target, name.to_string()));
        } else if let Some(target) = class_start(init) {
            self.pending_class = Some((target, name.to_string()));
        }
    }

    pub fn name_method(&mut self, it: &MethodDefinition<'a>) {
        let key = key_segment(&it.key, it.computed);
        let segment = match it.kind {
            MethodDefinitionKind::Constructor => "constructor".to_string(),
            MethodDefinitionKind::Get => format!("get:{key}"),
            MethodDefinitionKind::Set => format!("set:{key}"),
            MethodDefinitionKind::Method => key,
        };
        self.pending = Some(Pending {
            target: it.value.span.start,
            segment,
            own_start: it.span.start,
            line_pos: it.key.span().start,
            self_name: None,
            this_name: key_ident(&it.key, it.computed),
        });
    }

    pub fn name_class_field(&mut self, it: &PropertyDefinition<'a>) {
        let Some(target) = it.value.as_ref().and_then(function_start) else {
            return;
        };
        self.pending = Some(Pending {
            target,
            segment: key_segment(&it.key, it.computed),
            own_start: it.span.start,
            line_pos: it.key.span().start,
            self_name: None,
            this_name: key_ident(&it.key, it.computed),
        });
    }

    pub fn name_object_property(&mut self, it: &ObjectProperty<'a>) {
        let key = key_segment(&it.key, it.computed);
        let key = match it.kind {
            PropertyKind::Get => format!("get:{key}"),
            PropertyKind::Set => format!("set:{key}"),
            PropertyKind::Init => key,
        };
        let object = self.object_names.last().cloned().flatten();
        let segment = match object {
            Some(object) => format!("{object}.{key}"),
            None => key,
        };
        if let Some(target) = function_start(&it.value) {
            self.pending = Some(Pending {
                target,
                segment,
                own_start: it.span.start,
                line_pos: it.key.span().start,
                self_name: None,
                this_name: key_ident(&it.key, it.computed),
            });
        } else if let Some(target) = object_start(&it.value) {
            self.pending_object = Some((target, segment));
        }
    }

    pub fn name_export_default(&mut self, it: &ExportDefaultDeclaration<'a>) {
        let start = it.span.start;
        let target = match &it.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                if f.id.is_some() {
                    self.export_start = Some((f.span.start, start));
                    return;
                }
                f.span.start
            }
            ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                if c.id.is_none() {
                    self.pending_class = Some((c.span.start, "default".to_string()));
                }
                return;
            }
            other => {
                let Some(expr) = other.as_expression() else {
                    return;
                };
                if let Some(target) = class_start(expr) {
                    self.pending_class = Some((target, "default".to_string()));
                    return;
                }
                let Some(target) = function_start(expr) else {
                    return;
                };
                target
            }
        };
        self.pending = Some(Pending {
            target,
            segment: "default".to_string(),
            own_start: start,
            line_pos: start,
            self_name: None,
            this_name: None,
        });
    }

    /// Name segment of a class, consuming any name supplied by its holder.
    pub fn class_segment(&mut self, it: &Class<'a>) -> String {
        let pending = match &self.pending_class {
            Some((target, _)) if *target == it.span.start => self.pending_class.take(),
            _ => None,
        };
        match (pending, &it.id) {
            (Some((_, name)), _) => name,
            (None, Some(id)) => id.name.as_str().to_string(),
            (None, None) => self.anonymous_segment(),
        }
    }
}
