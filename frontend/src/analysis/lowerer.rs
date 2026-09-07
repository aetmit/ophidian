use crate::analysis::AnalysisCtx;
use crate::analysis::hir::{self, Hir};
use crate::analysis::ids::{HirId, VariableId};
use crate::analysis::types::{Conversion, Type};
use crate::parse::ast::{self, Ast, NodeId};

pub struct Lowerer {
    curr_hirid: HirId,
}

struct LowerState<'ctx, 'instance, 'diag> {
    instance: &'instance mut Lowerer,
    ctx: &'ctx mut AnalysisCtx<'diag>,
    id: Option<NodeId>,
}

impl<'ctx, 'instance, 'diag> LowerState<'ctx, 'instance, 'diag> {
    pub fn new(
        instance: &'instance mut Lowerer,
        ctx: &'ctx mut AnalysisCtx<'diag>,
        id: Option<NodeId>,
    ) -> Self {
        Self { instance, ctx, id }
    }
}

trait LowerTo<T>
where
    Self: Ast,
    T: Hir,
{
    fn lower(self, state: &mut LowerState) -> T;
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
        let mut state = LowerState::new(self, ctx, Some(function.id));

        let function = function.lower(&mut state);

        function
    }

    fn lower_globalvardecl(
        &mut self,
        decl: ast::GlobalVarDecl,
        ctx: &mut AnalysisCtx,
    ) -> hir::GlobalVarDecl {
        todo!()
    }

    fn lower_stmt(&mut self, stmt: ast::Stmt, ctx: &mut AnalysisCtx) -> hir::Stmt {
        let mut state = LowerState::new(self, ctx, Some(stmt.id));
        match stmt.kind {
            ast::StmtKind::Block(body) => {
                let block = body.lower(&mut state);

                let kind = hir::StmtKind::Block(block);

                return hir::Stmt::new(kind, self.next_hirid());
            }
            ast::StmtKind::Break => {
                let kind = hir::StmtKind::Break;

                return hir::Stmt::new(kind, self.next_hirid());
            }
            ast::StmtKind::Continue => {
                let kind = hir::StmtKind::Continue;

                return hir::Stmt::new(kind, self.next_hirid());
            }
            ast::StmtKind::Error => {
                unreachable!()
            }
            ast::StmtKind::ExprStmt(expr) => {
                let expr = expr.lower(&mut state);

                let kind = hir::StmtKind::ExprStmt(expr);

                return hir::Stmt::new(kind, self.next_hirid());
            }
            ast::StmtKind::For(for_loop) => {
                let for_loop = for_loop.lower(&mut state);

                let kind = hir::StmtKind::For(for_loop);

                return hir::Stmt::new(kind, self.next_hirid());
            }
            ast::StmtKind::If(if_stmt) => {
                let if_stmt = if_stmt.lower(&mut state);

                let kind = hir::StmtKind::If(if_stmt);
                return hir::Stmt::new(kind, self.next_hirid());
            }
            ast::StmtKind::Print(print) => {
                let print = print.lower(&mut state);
                let kind = hir::StmtKind::Print(print);

                return hir::Stmt::new(kind, self.next_hirid());
            }
            ast::StmtKind::Return(ret) => {
                let ret = ret.lower(&mut state);

                let kind = hir::StmtKind::Return(ret);

                return hir::Stmt::new(kind, self.next_hirid());
            }
            ast::StmtKind::VarDecl(decl) => {
                let decl = decl.lower(&mut state);

                let kind = hir::StmtKind::VarDecl(decl);

                return hir::Stmt::new(kind, self.next_hirid());
            }
            ast::StmtKind::While(while_loop) => {
                let while_loop = while_loop.lower(&mut state);

                let kind = hir::StmtKind::While(while_loop);

                return hir::Stmt::new(kind, self.next_hirid());
            }
        }
    }

    fn lower_expr(&mut self, expr: ast::Expr, ctx: &mut AnalysisCtx) -> hir::Expr {
        let mut state = LowerState::new(self, ctx, Some(expr.id));
        let hir_expr = match expr.kind {
            ast::ExprKind::BinaryOp(op, lhs, rhs) => {
                let binopkind = op.node.lower(&mut state);
                let lhs = self.lower_expr(*lhs, ctx);
                let rhs = self.lower_expr(*rhs, ctx);
                let binop = hir::BinaryOp::new(binopkind, Box::new(lhs), Box::new(rhs));
                let kind = hir::ExprKind::BinaryOp(binop);

                let ty = ctx.types.get(&expr.id).unwrap();

                return hir::Expr::new(kind, self.next_hirid(), *ty);
            }
            ast::ExprKind::Call(callee, args) => {
                let expr_hir = self.lower_expr(*callee, ctx);

                let hir_args: Vec<hir::Expr> = args
                    .into_iter()
                    .map(|arg| self.lower_expr(arg, ctx))
                    .collect();

                let function = *ctx.calls.get(&expr.id).unwrap();

                let call = hir::Call::new(Box::new(expr_hir), function, hir_args);

                let kind = hir::ExprKind::Call(call);

                let ty = *ctx.types.get(&expr.id).unwrap();

                hir::Expr::new(kind, self.next_hirid(), ty)
            }
            ast::ExprKind::Error => {
                unreachable!()
            }
            ast::ExprKind::Literal(litkind) => {
                let lit_kind = litkind.lower(&mut state);
                let literal = hir::Literal::new(lit_kind);

                let ty = *ctx.types.get(&expr.id).unwrap();

                let kind = hir::ExprKind::Literal(literal);

                hir::Expr::new(kind, self.next_hirid(), ty)
            }
            ast::ExprKind::UnaryOp(op, operand) => {
                let unaryopkind = op.node.lower(&mut state);
                let rhs = Box::new(self.lower_expr(*operand, ctx));
                let unaryop = hir::UnaryOp::new(unaryopkind, rhs);
                let kind = hir::ExprKind::UnaryOp(unaryop);

                let ty = *ctx.types.get(&expr.id).unwrap();

                hir::Expr::new(kind, self.next_hirid(), ty)
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
        };

        if let Some(conversion) = ctx.conversions.get(&expr.id) {
            match conversion {
                Conversion::IntToDouble => {
                    let hir_conversion =
                        hir::Conversion::new(hir::ConversionKind::IntToDouble, Box::new(hir_expr));
                    let kind = hir::ExprKind::Conversion(hir_conversion);

                    return hir::Expr::new(
                        kind,
                        self.next_hirid(),
                        self.get_conversion_type(conversion),
                    );
                }
            }
        }

        hir_expr
    }

    fn get_conversion_type(&self, conversion: &Conversion) -> Type {
        match conversion {
            Conversion::IntToDouble => Type::Double,
        }
    }

    fn next_hirid(&mut self) -> HirId {
        let id = self.curr_hirid;
        self.curr_hirid += 1;
        id
    }
}

impl LowerTo<hir::Function> for ast::Function {
    fn lower(self, state: &mut LowerState) -> hir::Function {
        let nodeid = state.id.unwrap_or_else(|| panic!("internal error"));
        let fn_id = *state.ctx.functions.get(&nodeid).unwrap();

        let sig = state.ctx.signatures.get(&fn_id).unwrap();
        let return_type = sig.return_type;

        let params = self
            .params
            .into_iter()
            .map(|param| {
                let mut state = LowerState::new(state.instance, state.ctx, Some(param.id));
                param.lower(&mut state)
            })
            .collect();

        let body = self.body.node.lower(state);

        return hir::Function::new(
            state.instance.next_hirid(),
            fn_id,
            return_type,
            params,
            body,
        );
    }
}

impl LowerTo<hir::Param> for ast::Param {
    fn lower(self, state: &mut LowerState) -> hir::Param {
        let nodeid = state.id.unwrap_or_else(|| panic!("internal error"));
        let varid = *state.ctx.variables.get(&nodeid).unwrap();

        let id = match varid {
            VariableId::Global(_) => {
                unreachable!()
            }
            VariableId::Local(i) => i,
        };

        let ty = *state.ctx.var_types.get(&varid).unwrap();

        return hir::Param::new(id, ty);
    }
}

impl LowerTo<hir::While> for ast::While {
    fn lower(self, state: &mut LowerState) -> hir::While {
        let condition = state.instance.lower_expr(*self.condition, state.ctx);

        let body = state.instance.lower_stmt(*self.body, state.ctx);

        return hir::While::new(condition, Box::new(body));
    }
}

impl LowerTo<hir::Return> for ast::Return {
    fn lower(self, state: &mut LowerState) -> hir::Return {
        let expr = if let Some(expr) = self.expr {
            Some(state.instance.lower_expr(expr, state.ctx))
        } else {
            None
        };

        return hir::Return::new(expr);
    }
}

impl LowerTo<hir::Print> for ast::Print {
    fn lower(self, state: &mut LowerState) -> hir::Print {
        let expr = state.instance.lower_expr(*self.expr, state.ctx);

        return hir::Print::new(expr);
    }
}

impl LowerTo<hir::If> for ast::If {
    fn lower(self, state: &mut LowerState) -> hir::If {
        let condition = state.instance.lower_expr(*self.condition, state.ctx);

        let body = state.instance.lower_stmt(*self.body, state.ctx);

        let else_clause = if let Some(else_clause) = self.else_body {
            Some(Box::new(state.instance.lower_stmt(*else_clause, state.ctx)))
        } else {
            None
        };

        return hir::If::new(condition, Box::new(body), else_clause);
    }
}

impl LowerTo<hir::For> for ast::For {
    fn lower(self, state: &mut LowerState) -> hir::For {
        let condition = if let Some(condition) = self.condition {
            Some(state.instance.lower_expr(condition, state.ctx))
        } else {
            None
        };

        let increment = if let Some(increment) = self.increment {
            Some(state.instance.lower_expr(increment, state.ctx))
        } else {
            None
        };

        let init = if let Some(init) = self.init {
            let kind = init.kind.lower(state);
            let id = state.instance.next_hirid();
            Some(hir::ForInit::new(kind, id))
        } else {
            None
        };

        let body = state.instance.lower_stmt(*self.body, state.ctx);

        return hir::For::new(init, condition, increment, Box::new(body));
    }
}

impl LowerTo<hir::ForInitKind> for ast::ForInitKind {
    fn lower(self, state: &mut LowerState) -> hir::ForInitKind {
        match self {
            ast::ForInitKind::Decl(decl) => {
                let decl = decl.node.lower(state);
                return hir::ForInitKind::Decl(decl);
            }
            ast::ForInitKind::Expr(expr) => {
                let expr = state.instance.lower_expr(expr, state.ctx);
                return hir::ForInitKind::Expr(expr);
            }
        }
    }
}

impl LowerTo<hir::VarDecl> for ast::VarDecl {
    fn lower(self, state: &mut LowerState) -> hir::VarDecl {
        let nodeid = state.id.unwrap_or_else(|| unreachable!());
        let id = *state.ctx.variables.get(&nodeid).unwrap();
        let id = match id {
            VariableId::Global(_) => {
                unreachable!()
            }
            VariableId::Local(id) => id,
        };

        let ty = *state.ctx.types.get(&nodeid).unwrap();

        let init = if let Some(init) = self.init {
            Some(state.instance.lower_expr(init, state.ctx))
        } else {
            None
        };

        return hir::VarDecl::new(id, ty, init);
    }
}

impl LowerTo<hir::ExprStmt> for ast::ExprStmt {
    fn lower(self, state: &mut LowerState) -> hir::ExprStmt {
        let expr = state.instance.lower_expr(*self.expr, state.ctx);

        return hir::ExprStmt::new(expr);
    }
}

impl LowerTo<hir::Block> for ast::Block {
    fn lower(self, state: &mut LowerState) -> hir::Block {
        let body: Vec<_> = self
            .body
            .into_iter()
            .map(|s| state.instance.lower_stmt(s, state.ctx))
            .collect();

        return hir::Block::new(body);
    }
}

impl LowerTo<hir::UnaryOpKind> for ast::UnaryOpKind {
    fn lower(self, _state: &mut LowerState) -> hir::UnaryOpKind {
        match self {
            ast::UnaryOpKind::Negate => hir::UnaryOpKind::Negate,
            ast::UnaryOpKind::PostDecrement => hir::UnaryOpKind::PostDecrement,
            ast::UnaryOpKind::PostIncrement => hir::UnaryOpKind::PostIncrement,
            ast::UnaryOpKind::PreDecrement => hir::UnaryOpKind::PreDecrement,
            ast::UnaryOpKind::PreIncrement => hir::UnaryOpKind::PreIncrement,
        }
    }
}

impl LowerTo<hir::LiteralKind> for ast::LitKind {
    fn lower(self, _state: &mut LowerState) -> hir::LiteralKind {
        match self {
            Self::Bool(b) => hir::LiteralKind::Bool(b),
            Self::Float(f) => hir::LiteralKind::Float(f),
            Self::Int(i) => hir::LiteralKind::Int(i as i32),
        }
    }
}

impl LowerTo<hir::BinaryOpKind> for ast::BinOpKind {
    fn lower(self, _state: &mut LowerState) -> hir::BinaryOpKind {
        match self {
            ast::BinOpKind::Add => hir::BinaryOpKind::Add,
            ast::BinOpKind::And => hir::BinaryOpKind::And,
            ast::BinOpKind::BangEq => hir::BinaryOpKind::BangEq,
            ast::BinOpKind::Div => hir::BinaryOpKind::Div,
            ast::BinOpKind::EqEq => hir::BinaryOpKind::EqEq,
            ast::BinOpKind::GreaterEq => hir::BinaryOpKind::GreaterEq,
            ast::BinOpKind::GreaterThan => hir::BinaryOpKind::GreaterThan,
            ast::BinOpKind::LessEq => hir::BinaryOpKind::LessEq,
            ast::BinOpKind::LessThan => hir::BinaryOpKind::LessThan,
            ast::BinOpKind::Mul => hir::BinaryOpKind::Mul,
            ast::BinOpKind::Or => hir::BinaryOpKind::Or,
            ast::BinOpKind::Sub => hir::BinaryOpKind::Sub,
        }
    }
}
