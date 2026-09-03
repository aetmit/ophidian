use crate::parse::ast;
use crate::analysis::hir;
use crate::analysis::AnalysisCtx;
use crate::analysis::ids::HirId;

pub struct Lowerer {
    curr_hirid: HirId,
}

impl Lowerer {
    pub fn new() -> Self {
        Self {
            curr_hirid: HirId(0),
        }
    }

    pub fn lower(&mut self, program: &ast::Program, ctx: &mut AnalysisCtx) -> hir::Program {
        let mut items = Vec::new();
        for item in &program.items {
            match item {
                ast::Item::Function(f) => {
                    let item = hir::Item::Function(self.lower_function(f, ctx));
                    items.push(item);
                }
                ast::Item::GlobalVarDecl(decl) => {
                    let item = hir::Item::GlobalVarDecl(self.lower_globalvardecl(decl, ctx))
                    items.push(item);
                }
            }
        }
        let program = hir::Program::new(items);
        program
    }

    fn lower_function(&mut self, function: &ast::Function, ctx: &mut AnalysisCtx) -> hir::Function {
        todo!() 
    }

    fn lower_globalvardecl(&mut self, decl: &ast::GlobalVarDecl, ctx: &mut AnalysisCtx) -> hir::GlobalVarDecl {
        todo!()
    }

    fn lower_stmt(&mut self, stmt: &ast::Stmt, ctx: &mut AnalysisCtx) -> hir::Stmt {
        todo!()
    }

    fn lower_expr(&mut self, expr: &ast::Expr, ctx: &mut AnalysisCtx) -> hir::Expr {
        todo!()
    }

    fn next_hirid(&mut self) -> HirId {
        let id = self.curr_hirid;
        self.curr_hirid += 1;
        id
    }
}