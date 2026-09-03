use macros::Constructor;

use crate::analysis::types::{Type, TypeChecker};
use crate::analysis::resolution::Resolver;
use crate::analysis::ids::{LocalVarId, VariableId};
use crate::analysis::AnalysisCtx;
use crate::parse::ast::{Function as AstFunction, Item, Program};

#[derive(Debug, PartialEq, Eq, Clone, Constructor)]
pub struct Function {
    pub return_type: Type,
    pub params: Vec<Param>,
}

#[derive(Debug, PartialEq, Eq, Clone, Constructor)]
pub struct Param {
    pub id: LocalVarId,
    pub ty: Type,
}

pub struct FunctionAnalyzer {}

impl FunctionAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn analyze(&mut self, program: &Program, ctx: &mut AnalysisCtx) {
        for item in &program.items {
            match item {
                Item::Function(f) => {
                    self.analyze_function(f, ctx);
                }
                Item::GlobalVarDecl(_) => {
                    continue;
                }
            }
        }
    }

    fn analyze_function(&mut self, function: &AstFunction, ctx: &mut AnalysisCtx) {
        let mut resolver = Resolver::new();

        resolver.enter_scope(ctx);
        for param in &function.params {
            let id = *ctx.variables.get(&param.id).unwrap();
            match id {
                VariableId::Global(_)  => {
                    unreachable!()
                }
                VariableId::Local(id) => {
                    resolver.declare_existing_var(&param.name, id, ctx, param.span);
                }
            }
        }
        resolver.resolve_block(&function.body.node.body, ctx);
        resolver.exit_scope(ctx);

        if !ctx.diagnostics.is_empty() {
            // exit if name resolution produced any errors
            return;
        }

        let mut typechecker = TypeChecker::new();
        typechecker.check_fn(&function.body.node.body, function.return_type.unwrap_or(Type::Void), ctx);

    }
}
