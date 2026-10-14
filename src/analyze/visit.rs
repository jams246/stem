//! Syntax tree traversal for [`Collector`]; each hook delegates to the module
//! that owns the rule.

use oxc_ast::ast::{
    ArrowFunctionExpression, AssignmentExpression, AssignmentTarget, BindingIdentifier,
    BindingProperty, BreakStatement, CallExpression, CatchClause, Class, ConditionalExpression,
    ContinueStatement, DoWhileStatement, ExportDeclaration, ExportDefaultDeclaration,
    ForInStatement, ForOfStatement, ForStatement, Function, FunctionType, IdentifierReference,
    IfStatement, JSXIdentifier, LogicalExpression, MethodDefinition, ObjectExpression,
    ObjectProperty, PrivateFieldExpression, PropertyDefinition, StaticMemberExpression,
    SwitchStatement, TSClassImplements, TSInterfaceDeclaration, TSThisParameter, TSType,
    TSTypeAliasDeclaration, TSTypeAnnotation, TSTypeParameterDeclaration,
    TSTypeParameterInstantiation, VariableDeclaration, WhileStatement,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;

use super::collector::{Collector, FunctionStart, Pending};
use super::hash;
use super::naming::function_start;

fn count(params: &oxc_ast::ast::FormalParameters<'_>) -> u32 {
    let total = params.items.len() + usize::from(params.rest.is_some());
    u32::try_from(total).unwrap_or(u32::MAX)
}

impl<'a> Visit<'a> for Collector<'a> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        // Bodiless signatures (overloads, `declare`, `abstract`) are not measured.
        if it.body.is_none() {
            return;
        }
        let id = it.id.as_ref().map(|id| id.name.as_str());
        if it.r#type == FunctionType::FunctionDeclaration
            && let Some(id) = id
        {
            self.add_name(id);
        }
        self.begin_function(FunctionStart {
            span: it.span,
            id,
            parameters: count(&it.params),
            hash: hash::function_hash(it),
        });
        walk::walk_function(self, it, flags);
        self.end_function();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.begin_function(FunctionStart {
            span: it.span,
            id: None,
            parameters: count(&it.params),
            hash: hash::arrow_hash(it),
        });
        walk::walk_arrow_function_expression(self, it);
        self.end_function();
    }

    fn visit_method_definition(&mut self, it: &MethodDefinition<'a>) {
        if it.value.body.is_none() {
            return;
        }
        self.name_method(it);
        walk::walk_method_definition(self, it);
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        self.name_class_field(it);
        walk::walk_property_definition(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if !it.computed
            && let oxc_ast::ast::PropertyKey::StaticIdentifier(id) = &it.key
        {
            self.add_name(id.name.as_str());
        }
        self.name_object_property(it);
        walk::walk_object_property(self, it);
    }

    fn visit_object_expression(&mut self, it: &ObjectExpression<'a>) {
        let name = match &self.pending_object {
            Some((target, _)) if *target == it.span.start => self.pending_object.take(),
            _ => None,
        };
        self.object_names.push(name.map(|(_, name)| name));
        walk::walk_object_expression(self, it);
        self.object_names.pop();
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        let own_start = self.owner_start(it.span.start);
        for (i, declarator) in it.declarations.iter().enumerate() {
            let start = if i == 0 {
                own_start
            } else {
                declarator.span.start
            };
            self.name_variable(start, declarator);
            self.visit_variable_declarator(declarator);
        }
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left
            && let Some(target) = function_start(&it.right)
        {
            self.pending = Some(Pending {
                target,
                segment: id.name.as_str().to_string(),
                own_start: it.span.start,
                line_pos: it.span.start,
                self_name: Some(id.name.as_str()),
                this_name: None,
            });
        }
        walk::walk_assignment_expression(self, it);
    }

    fn visit_export_declaration(&mut self, it: &ExportDeclaration<'a>) {
        self.export_start = Some((it.declaration.span().start, it.span.start));
        walk::walk_export_declaration(self, it);
    }

    fn visit_export_default_declaration(&mut self, it: &ExportDefaultDeclaration<'a>) {
        self.name_export_default(it);
        walk::walk_export_default_declaration(self, it);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        let segment = self.class_segment(it);
        let prefix = self.qualify(&segment);
        if let Some(id) = &it.id {
            self.add_name(id.name.as_str());
        }
        self.visit_decorators(&it.decorators);
        self.push_scope(prefix);
        if let Some(heritage) = &it.heritage {
            self.visit_expression(&heritage.expression);
        }
        self.visit_class_body(&it.body);
        self.pop_scope();
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        self.if_chain(it, false);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        self.conditional(it);
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        self.switch(it);
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        self.for_loop(it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.for_in(it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.for_of(it);
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.while_loop(it);
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.do_while(it);
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        self.catch_clause(it);
    }

    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        self.logical(it);
    }

    fn visit_break_statement(&mut self, it: &BreakStatement<'a>) {
        self.jump(it.span.start, it.label.as_ref(), "labeled break");
    }

    fn visit_continue_statement(&mut self, it: &ContinueStatement<'a>) {
        self.jump(it.span.start, it.label.as_ref(), "labeled continue");
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.call(it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.add_name(it.name.as_str());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.add_name(it.name.as_str());
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        if !it.computed
            && let oxc_ast::ast::PropertyKey::StaticIdentifier(id) = &it.key
        {
            self.add_name(id.name.as_str());
        }
        walk::walk_binding_property(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.visit_expression(&it.object);
        self.add_name(it.property.name.as_str());
    }

    fn visit_private_field_expression(&mut self, it: &PrivateFieldExpression<'a>) {
        self.visit_expression(&it.object);
        self.add_name(it.field.name.as_str());
    }

    fn visit_jsx_identifier(&mut self, it: &JSXIdentifier<'a>) {
        self.add_name(it.name.as_str());
    }

    // Type positions never contribute names, increments or functions.
    fn visit_ts_type(&mut self, _it: &TSType<'a>) {}
    fn visit_ts_type_annotation(&mut self, _it: &TSTypeAnnotation<'a>) {}
    fn visit_ts_type_parameter_declaration(&mut self, _it: &TSTypeParameterDeclaration<'a>) {}
    fn visit_ts_type_parameter_instantiation(&mut self, _it: &TSTypeParameterInstantiation<'a>) {}
    fn visit_ts_type_alias_declaration(&mut self, _it: &TSTypeAliasDeclaration<'a>) {}
    fn visit_ts_interface_declaration(&mut self, _it: &TSInterfaceDeclaration<'a>) {}
    fn visit_ts_class_implements(&mut self, _it: &TSClassImplements<'a>) {}
    fn visit_ts_this_parameter(&mut self, _it: &TSThisParameter<'a>) {}
}
