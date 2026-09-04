use crate::analysis::AnalysisCtx;
use crate::analysis::function::{Function, Param};
use crate::analysis::ids::{FunctionId, GlobalVarId, VariableId};
use crate::analysis::resolution::GlobalVarDecl;
use crate::analysis::types::Type;
use crate::diagnostics::{Diagnostic, Severity};
use crate::parse::ast;
use crate::span::Span;
use std::collections::HashSet;

pub struct Collecter<'a> {
    identifiers: HashSet<&'a [u8]>,
    curr_function_id: FunctionId,
    curr_globalvar_id: GlobalVarId,
}

impl<'a> Collecter<'a> {
    pub fn new() -> Self {
        Self {
            identifiers: HashSet::new(),
            curr_function_id: FunctionId(0),
            curr_globalvar_id: GlobalVarId(0),
        }
    }

    pub fn collect(&mut self, program: &'a ast::Program, ctx: &mut AnalysisCtx) {
        for item in &program.items {
            match item {
                ast::Item::Function(function) => {
                    self.collect_fn(function, ctx);
                }
                ast::Item::GlobalVarDecl(decl) => {
                    self.collect_global_var_decl(decl, ctx);
                }
            }
        }
    }

    fn alloc_fn_id(&mut self) -> FunctionId {
        let id = self.curr_function_id;
        self.curr_function_id += 1;
        id
    }

    fn alloc_globalvar_id(&mut self) -> GlobalVarId {
        let id = self.curr_globalvar_id;
        self.curr_globalvar_id += 1;
        id
    }

    fn error<T: Into<String>>(&self, msg: T, span: Span, ctx: &mut AnalysisCtx) {
        ctx.diagnostics
            .push(Diagnostic::new(msg.into(), span, Severity::Error));
    }

    fn collect_fn(&mut self, function: &'a ast::Function, ctx: &mut AnalysisCtx) {
        ctx.varid_set(0);
        if !self.identifiers.insert(&function.name) {
            self.error(
                format!(
                    "redeclaration of identifier: '{}'",
                    std::str::from_utf8(&function.name).expect("trust user provides valid utf8")
                ),
                function.span,
                ctx,
            );
        }
        let fn_id = self.alloc_fn_id();
        ctx.functions.insert(function.id, fn_id);
        let return_type = match function.return_type {
            Some(ty) => ty,
            None => Type::Void,
        };

        let params: Vec<Param> = function
            .params
            .iter()
            .map(|param| {
                let id = ctx.alloc_varid();
                let ty = param.ty;
                ctx.variables.insert(param.id, VariableId::Local(id));
                ctx.var_types.insert(VariableId::Local(id), ty);
                Param::new(id, ty)
            })
            .collect();

        ctx.function_scope.insert(function.name.clone(), fn_id);

        let function = Function::new(return_type, params);
        ctx.signatures.insert(fn_id, function);
    }

    fn collect_global_var_decl(&mut self, vardecl: &'a ast::GlobalVarDecl, ctx: &mut AnalysisCtx) {
        if !self.identifiers.insert(&vardecl.name) {
            self.error(
                format!(
                    "redeclaration of identifier: '{}'",
                    std::str::from_utf8(&vardecl.name).expect("trust user provides valid utf8")
                ),
                vardecl.span,
                ctx,
            );
        }

        let decl_id = self.alloc_globalvar_id();
        ctx.global_scope
            .globals
            .insert(vardecl.name.clone(), decl_id);
        ctx.global_vars.insert(vardecl.id, decl_id);

        let type_annotation = vardecl.type_annotation;
        let init = vardecl.init.clone();

        let globalvardecl = GlobalVarDecl::new(decl_id, type_annotation, init);
        ctx.globalvar_data.insert(decl_id, globalvardecl);
    }
}
