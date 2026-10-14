//! Collects only this function's declarations, value references and statements.

use oxc_ast::ast::{
    ArrowFunctionExpression, BindingIdentifier, Class, Function, IdentifierReference, Statement,
    TSExternalModuleDeclaration, TSGlobalDeclaration, TSInterfaceDeclaration,
    TSNamespaceDeclaration, TSType, TSTypeAliasDeclaration, TSTypeAnnotation,
    TSTypeParameterDeclaration, TSTypeParameterInstantiation, VariableDeclaration,
    VariableDeclarationKind,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolFlags;

use super::{Local, Locality};

impl<'a> Visit<'a> for Locality<'a, '_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if !matches!(
            it,
            Statement::BlockStatement(_)
                | Statement::EmptyStatement(_)
                | Statement::LabeledStatement(_)
        ) {
            self.statements.push(it.span());
        }
        walk::walk_statement(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        let eligible = matches!(
            it.kind,
            VariableDeclarationKind::Const
                | VariableDeclarationKind::Let
                | VariableDeclarationKind::Var
        ) && !it.declare;
        for declaration in &it.declarations {
            self.declaring = eligible;
            self.visit_binding_pattern(&declaration.id);
            self.declaring = false;
            if let Some(init) = &declaration.init {
                self.visit_expression(init);
            }
        }
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if self.declaring
            && let Some(symbol) = it.symbol_id.get()
        {
            let declaration = self.scoping.symbol_span(symbol);
            let excluded = SymbolFlags::Function
                | SymbolFlags::Class
                | SymbolFlags::CatchVariable
                | SymbolFlags::Ambient;
            if declaration.start < self.body_span.start
                || declaration.end > self.body_span.end
                || self.scoping.symbol_flags(symbol).intersects(excluded)
            {
                return;
            }
            let local = self.locals.entry(symbol).or_insert_with(|| Local {
                name: it.name.as_str(),
                declaration: it.span,
                occurrences: Vec::new(),
            });
            local.occurrences.push(it.span);
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(reference) = it.reference_id.get()
            && let Some(symbol) = self.scoping.get_reference(reference).symbol_id()
        {
            self.references.push((symbol, it.span));
        }
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}
    fn visit_class(&mut self, _it: &Class<'a>) {}
    fn visit_ts_namespace_declaration(&mut self, _it: &TSNamespaceDeclaration<'a>) {}
    fn visit_ts_external_module_declaration(&mut self, _it: &TSExternalModuleDeclaration<'a>) {}
    fn visit_ts_global_declaration(&mut self, _it: &TSGlobalDeclaration<'a>) {}
    fn visit_ts_type(&mut self, _it: &TSType<'a>) {}
    fn visit_ts_type_alias_declaration(&mut self, _it: &TSTypeAliasDeclaration<'a>) {}
    fn visit_ts_interface_declaration(&mut self, _it: &TSInterfaceDeclaration<'a>) {}
    fn visit_ts_type_annotation(&mut self, _it: &TSTypeAnnotation<'a>) {}
    fn visit_ts_type_parameter_declaration(&mut self, _it: &TSTypeParameterDeclaration<'a>) {}
    fn visit_ts_type_parameter_instantiation(&mut self, _it: &TSTypeParameterInstantiation<'a>) {}
}
