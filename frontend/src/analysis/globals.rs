use crate::analysis::AnalysisCtx;
use crate::analysis::types::Type;
use crate::diagnostics::{Diagnostic, Severity};
use crate::parse::ast::{Expr, ExprKind, GlobalVarDecl, Item, LitKind, Program};
use crate::span::Span;

pub struct GlobalVarAnalyzer;

impl GlobalVarAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn analyze_globals(&mut self, program: &Program, ctx: &mut AnalysisCtx) {
        for item in &program.items {
            match item {
                Item::Function(_) => {
                    continue;
                }
                Item::GlobalVarDecl(decl) => {
                    self.analyze_globaldecl(decl, ctx);
                }
            }
        }
    }

    fn error<T: Into<String>>(&self, message: T, span: Span, ctx: &mut AnalysisCtx) {
        ctx.diagnostics
            .push(Diagnostic::new(message.into(), span, Severity::Error));
    }

    fn analyze_globaldecl(&mut self, decl: &GlobalVarDecl, ctx: &mut AnalysisCtx) {
        // let id = ctx.global_vars.get(&decl.id).unwrap();

        let init = match &decl.init {
            Some(init) => init,
            None => {
                self.error(
                    "global variable declaration must have initializer",
                    decl.span,
                    ctx,
                );
                return;
            }
        };
        let ty = self.analyze_init(init, ctx);

        if ty == Type::Error {
            return;
        }

        if ty != decl.type_annotation {
            self.error("type mismatch", decl.span, ctx);
            return;
        }
    }

    fn analyze_init(&mut self, expr: &Expr, ctx: &mut AnalysisCtx) -> Type {
        if !self.is_constant(expr) {
            self.error(
                "cannot initialize global variable with non constant expression",
                expr.span,
                ctx,
            );
            return Type::Error;
        }

        todo!()
    }

    fn get_expr_type(&self, expr: &Expr) -> Type {
        match &expr.kind {
            ExprKind::Literal(lit) => match lit {
                LitKind::Bool(..) => return Type::Bool,
                LitKind::Float(..) => return Type::Double,
                LitKind::Int(..) => return Type::Int,
            },
            ExprKind::Call(callee, args) => {
                todo!()
            }
            _ => todo!(),
        }
    }

    fn is_constant(&self, expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::BinaryOp(.., lhs, rhs) => self.is_constant(&lhs) && self.is_constant(&rhs),
            ExprKind::Literal(_) => true,
            ExprKind::UnaryOp(.., operand) => self.is_constant(&operand),
            ExprKind::Call(..) => false,
            ExprKind::Variable(_name) => {
                // check if variable is constant
                todo!()
            }
            _ => false,
        }
    }
}
