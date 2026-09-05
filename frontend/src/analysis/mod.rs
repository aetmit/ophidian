pub mod declarations;
pub mod function;
pub mod globals;
pub mod hir;
pub mod ids;
pub mod lowerer;
pub mod resolution;
pub mod types;

use crate::analysis::declarations::Collecter;
use crate::analysis::function::{Function, FunctionAnalyzer};
use crate::analysis::globals::GlobalVarAnalyzer;
use crate::analysis::ids::{FunctionId, GlobalVarId, LocalVarId, VariableId};
use crate::analysis::lowerer::Lowerer;
use crate::analysis::resolution::{GlobalScope, GlobalVarDecl, Scope};
use crate::analysis::types::{Conversion, Type};
use crate::diagnostics::Diagnostic;
use crate::parse::ast::{self, NodeId};
use std::collections::HashMap;

pub struct SemanticAnalyzer<'diag> {
    diagnostics: &'diag mut Vec<Diagnostic>,
}

impl<'diag> SemanticAnalyzer<'diag> {
    pub fn new(diagnostics: &'diag mut Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    pub fn analyze(&mut self, program: ast::Program) -> Result<hir::Program, ()> {
        let mut ctx = AnalysisCtx::new(self.diagnostics);

        let mut collecter = Collecter::new();
        collecter.collect(&program, &mut ctx);

        let mut function_analyzer = FunctionAnalyzer::new();
        function_analyzer.analyze(&program, &mut ctx);

        if !ctx.diagnostics.is_empty() {
            return Err(());
        }

        let mut global_analyzer = GlobalVarAnalyzer::new();
        global_analyzer.analyze_globals(&program, &mut ctx);

        for (id, value) in &ctx.types {
            if let Some(conversion_type) = ctx.conversions.get(id) {
                match conversion_type {
                    Conversion::IntToDouble => {
                        ctx.converted_types.insert(*id, Type::Double);
                    }
                }
            } else {
                ctx.converted_types.insert(*id, *value);
            }
        }

        let mut lowerer = Lowerer::new();
        let hir = lowerer.lower(program, &mut ctx);
        Ok(hir)
    }
}

pub struct AnalysisCtx<'diag> {
    pub scopes: Vec<Scope>,
    pub global_scope: GlobalScope,
    pub types: HashMap<NodeId, Type>,
    pub variables: HashMap<NodeId, VariableId>,
    pub var_types: HashMap<VariableId, Type>,

    pub conversions: HashMap<NodeId, Conversion>,

    pub calls: HashMap<NodeId, FunctionId>,

    pub functions: HashMap<NodeId, FunctionId>,
    pub signatures: HashMap<FunctionId, Function>,

    pub function_scope: HashMap<Vec<u8>, FunctionId>,

    pub diagnostics: &'diag mut Vec<Diagnostic>,

    pub global_vars: HashMap<NodeId, GlobalVarId>,
    pub globalvar_data: HashMap<GlobalVarId, GlobalVarDecl>,

    pub converted_types: HashMap<NodeId, Type>,

    // current var id
    // local to a function
    varid: LocalVarId,
}

impl<'diag> AnalysisCtx<'diag> {
    pub fn alloc_varid(&mut self) -> LocalVarId {
        let id = self.varid;
        self.varid += 1;
        id
    }

    pub fn varid_set(&mut self, i: usize) {
        self.varid = LocalVarId(i);
    }
}

impl<'diag> AnalysisCtx<'diag> {
    fn new(diagnostics: &'diag mut Vec<Diagnostic>) -> Self {
        Self {
            scopes: Vec::new(),
            types: HashMap::new(),
            variables: HashMap::new(),
            var_types: HashMap::new(),
            conversions: HashMap::new(),
            converted_types: HashMap::new(),
            functions: HashMap::new(),
            signatures: HashMap::new(),
            global_vars: HashMap::new(),
            globalvar_data: HashMap::new(),
            global_scope: GlobalScope::new(),
            calls: HashMap::new(),
            function_scope: HashMap::new(),
            diagnostics,
            varid: LocalVarId(0),
        }
    }
}
