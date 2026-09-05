use crate::analysis::AnalysisCtx;
use crate::analysis::hir;
use crate::analysis::ids::HirId;
use crate::parse::ast;

pub struct Lowerer {
    curr_hirid: HirId,
}

impl Lowerer {
    pub fn new() -> Self {
        Self {
            curr_hirid: HirId(0),
        }
    }

    pub fn lower(&mut self, program: ast::Program, ctx: &mut AnalysisCtx) -> hir::Program {
        let mut items = Vec::new();
        for item in program.items {
            match item {
                ast::Item::Function(f) => {
                    let item = hir::Item::Function(self.lower_function(f, ctx));
                    items.push(item);
                }
                ast::Item::GlobalVarDecl(decl) => {
                    let item = hir::Item::GlobalVarDecl(self.lower_globalvardecl(decl, ctx));
                    items.push(item);
                }
            }
        }
        let program = hir::Program::new(items);
        program
    }

    fn lower_function(&mut self, function: ast::Function, ctx: &mut AnalysisCtx) -> hir::Function {
        todo!()
    }

    fn lower_globalvardecl(
        &mut self,
        decl: ast::GlobalVarDecl,
        ctx: &mut AnalysisCtx,
    ) -> hir::GlobalVarDecl {
        todo!()
    }

    fn lower_stmt(&mut self, stmt: ast::Stmt, ctx: &mut AnalysisCtx) -> hir::Stmt {
        todo!()
    }

    fn lower_expr(&mut self, expr: ast::Expr, ctx: &mut AnalysisCtx) -> hir::Expr {
        let id = self.next_hirid();
        match expr.kind {
            ast::ExprKind::BinaryOp(op, lhs, rhs) => {
                let binopkind = op.node.into();
                let lhs = self.lower_expr(*lhs, ctx);
                let rhs = self.lower_expr(*rhs, ctx);
                let binop = hir::BinaryOp::new(binopkind, Box::new(lhs), Box::new(rhs));
                let kind = hir::ExprKind::BinaryOp(binop);

                let ty = ctx.types.get(&expr.id).unwrap();

                return hir::Expr::new(kind, id, *ty);
            }
            ast::ExprKind::Call(callee, args) => {
                let expr_hir = self.lower_expr(*callee, ctx);


                let hir_args: Vec<hir::Expr> = args.into_iter().map(|arg| {
                    self.lower_expr(arg, ctx)
                }).collect();

                let call = hir::Call::new(Box::new(expr_hir), hir_args);

                let kind = hir::ExprKind::Call(call);

                let ty = *ctx.types.get(&expr.id).unwrap();

                hir::Expr::new(kind, self.next_hirid(), ty)
            }
            ast::ExprKind::Error => {
                unreachable!()
            }
            ast::ExprKind::Literal(litkind) => {
                let lit_kind = litkind.into();
                let literal = hir::Literal::new(lit_kind);

                let ty = *ctx.types.get(&expr.id).unwrap();

                let kind = hir::ExprKind::Literal(literal);

                return hir::Expr::new(kind, self.next_hirid(), ty);
            }
            ast::ExprKind::UnaryOp(op, operand) => {
                let unaryopkind = op.node.into();
                let rhs = Box::new(self.lower_expr(*operand, ctx));
                let unaryop = hir::UnaryOp::new(unaryopkind, rhs);
                let kind = hir::ExprKind::UnaryOp(unaryop);

                let ty = *ctx.types.get(&expr.id).unwrap();

                return hir::Expr::new(kind, self.next_hirid(), ty)
            }
            ast::ExprKind::VarAssign(target, value) => {
                let target = self.lower_expr(*target, ctx);
                let value = self.lower_expr(*value, ctx);

                let assign = hir::VarAssign::new(Box::new(target), Box::new(value));

                let kind = hir::ExprKind::VarAssign(assign);

                let ty = *ctx.types.get(&expr.id).unwrap();

                hir::Expr::new(kind, self.next_hirid(), ty)
            }
            ast::ExprKind::Variable(..) => {
                let varid = *ctx.variables.get(&expr.id).unwrap();

                let var = hir::Variable::new(varid);
                let kind = hir::ExprKind::Variable(var);

                let ty = *ctx.var_types.get(&varid).unwrap();

                hir::Expr::new(kind, self.next_hirid(), ty)
            }
        }
    }

    fn next_hirid(&mut self) -> HirId {
        let id = self.curr_hirid;
        self.curr_hirid += 1;
        id
    }
}

impl From<ast::UnaryOpKind> for hir::UnaryOpKind {
    fn from(value: ast::UnaryOpKind) -> Self {
        match value {
            ast::UnaryOpKind::Negate => {
                Self::Negate
            }
            ast::UnaryOpKind::PostDecrement => {
                Self::PostDecrement
            }
            ast::UnaryOpKind::PostIncrement => {
                Self::PostIncrement
            }
            ast::UnaryOpKind::PreDecrement => {
                Self::PreDecrement
            }
            ast::UnaryOpKind::PreIncrement =>{ 
                Self::PreIncrement
            }
        }
    }
}

impl From<ast::LitKind> for hir::LiteralKind {
    fn from(value: ast::LitKind) -> Self {
        match value {
            ast::LitKind::Bool(b) => {
                Self::Bool(b)
            }
            ast::LitKind::Float(f) => {
                Self::Float(f)
            }
            ast::LitKind::Int(i) => { 
                todo!()
            }
        }
    }
}

impl From<ast::BinOpKind> for hir::BinaryOpKind {
    fn from(value: ast::BinOpKind) -> Self {
        match value {
            ast::BinOpKind::Add => Self::Add,
            ast::BinOpKind::And => Self::And,
            ast::BinOpKind::BangEq => Self::BangEq,
            ast::BinOpKind::Div => Self::Div,
            ast::BinOpKind::EqEq => Self::EqEq,
            ast::BinOpKind::GreaterEq => Self::GreaterEq,
            ast::BinOpKind::GreaterThan => Self::GreaterThan,
            ast::BinOpKind::LessEq => Self::LessEq,
            ast::BinOpKind::LessThan => Self::LessThan,
            ast::BinOpKind::Mul => Self::Mul,
            ast::BinOpKind::Or => Self::Or,
            ast::BinOpKind::Sub => Self::Sub,
        }
    }
}
