use crate::analysis::AnalysisCtx;
use crate::diagnostics::{Diagnostic, Severity};
use crate::lex::token::TokenKind;
use crate::parse::ast::{
    BinOpKind, Expr, ExprKind, ForInit, ForInitKind, LitKind, NodeId, Stmt, StmtKind, UnaryOpKind,
    VarDecl,
};
use crate::span::{Span, Spanned};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Int,

    Bool,

    Double,

    Void,

    // The type that allows analysis to continue if it
    // encounters an error
    Error,
}

impl From<TokenKind> for Type {
    fn from(value: TokenKind) -> Self {
        match value {
            TokenKind::Int => Self::Int,
            TokenKind::Double => Self::Double,
            TokenKind::Bool => Self::Bool,
            TokenKind::Void => Self::Void,
            _ => Self::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conversion {
    IntToDouble,
}

pub struct TypeChecker {
    // true if the typechecker is currently type checking a function
    is_checking_fn: bool,
    fn_ret_type: Option<Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            is_checking_fn: false,
            fn_ret_type: None,
        }
    }

    pub fn check_fn(&mut self, stmts: &[Stmt], return_type: Type, ctx: &mut AnalysisCtx) {
        self.is_checking_fn = true;
        self.fn_ret_type = Some(return_type);

        for stmt in stmts {
            self.check_stmt(stmt, ctx);
        }
    }

    fn error<T: Into<String>>(&mut self, message: T, span: Span, ctx: &mut AnalysisCtx) {
        ctx.diagnostics
            .push(Diagnostic::new(message.into(), span, Severity::Error));
    }

    fn check_stmt(&mut self, stmt: &Stmt, ctx: &mut AnalysisCtx) {
        match &stmt.kind {
            StmtKind::Block(block) => {
                for stmt in &block.body {
                    self.check_stmt(&stmt, ctx);
                }
            }
            StmtKind::ExprStmt(e) => {
                self.check_expr(&e.expr, ctx);
            }
            StmtKind::Print(p) => {
                let expr_type = self.check_expr(&p.expr, ctx);

                // builtin checking for the types that print supports
                // because print is not a function yet
                match expr_type {
                    Type::Error => {
                        return;
                    }
                    Type::Void => {
                        self.error("invalid use of 'void' expression", p.expr.span, ctx);
                        return;
                    }
                    Type::Int | Type::Bool | Type::Double => {}
                }
            }
            StmtKind::VarDecl(decl) => {
                self.check_vardecl(stmt, decl, ctx);
            }
            StmtKind::If(i) => {
                let cond_ty = self.check_expr(&i.condition, ctx);

                if cond_ty != Type::Bool && cond_ty != Type::Error {
                    self.error(
                        "if statement condition must have type 'bool'",
                        i.condition.span,
                        ctx,
                    );
                    return;
                }

                self.check_stmt(&i.body, ctx);

                if let Some(e) = &i.else_body {
                    self.check_stmt(&e, ctx);
                }
            }
            StmtKind::While(w) => {
                let cond_ty = self.check_expr(&w.condition, ctx);

                if cond_ty != Type::Bool && cond_ty != Type::Error {
                    self.error(
                        "while statement condition must have type 'bool'",
                        w.condition.span,
                        ctx,
                    );
                    return;
                }

                self.check_stmt(&w.body, ctx);
            }
            StmtKind::Break => {
                // nothing to type check
            }
            StmtKind::Continue => {
                // nothing to type check
            }
            StmtKind::For(f) => {
                if let Some(init) = &f.init {
                    self.check_forinit(&**init, ctx);
                }

                if let Some(cond) = &f.condition {
                    let ty = self.check_expr(&cond, ctx);

                    if ty != Type::Bool && ty != Type::Error {
                        self.error("for loop condition must have type 'bool'", cond.span, ctx);
                        return;
                    }
                }

                if let Some(incre) = &f.increment {
                    self.check_expr(&incre, ctx);
                }

                self.check_stmt(&f.body, ctx);
            }
            StmtKind::Return(ret) => {
                if self.is_checking_fn {
                    let ty = if let Some(expr) = &ret.expr {
                        self.check_expr(&expr, ctx)
                    } else {
                        Type::Void
                    };

                    if ty != self.fn_ret_type.unwrap() {
                        self.error("mismatched return types", stmt.span, ctx);
                        return;
                    }
                }
            }
            StmtKind::Error => {
                unreachable!()
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr, ctx: &mut AnalysisCtx) -> Type {
        let ty = match &expr.kind {
            ExprKind::BinaryOp(op, left, right) => {
                let left_type = self.check_expr(left, ctx);
                let right_type = self.check_expr(right, ctx);

                let res = self.binary_result_type(
                    op.node, left_type, right_type, left.id, right.id, left.span, right.span, ctx,
                );
                if res == Type::Error {
                    self.error(
                        format!("invalid operands for binary operation: '{}'", op.node),
                        expr.span,
                        ctx,
                    );
                }

                res
            }
            ExprKind::Literal(lit) => match lit {
                LitKind::Int(i) => {
                    // TODO: because i32::MAX is less than i32::min when the sign is ignored,
                    // we need to somehow account for the +1 that a negative literal needs
                    if *i <= i32::MAX as u128 {
                        Type::Int
                    } else {
                        unimplemented!("larger integer types not yet implemented")
                    }
                }
                LitKind::Bool(_b) => Type::Bool,
                LitKind::Float(_f) => {
                    // no type checking needed here
                    Type::Double
                }
            },
            ExprKind::UnaryOp(op, right) => {
                let expr_type = self.check_expr(right, ctx);

                match op.node {
                    UnaryOpKind::Negate => {
                        // do nothin
                    }
                    UnaryOpKind::PostDecrement
                    | UnaryOpKind::PostIncrement
                    | UnaryOpKind::PreDecrement
                    | UnaryOpKind::PreIncrement => {
                        if !self.is_lvalue(right) {
                            self.error(
                                format!("cannot apply operator '{}' on non l-value", op.node),
                                expr.span,
                                ctx,
                            );
                            return Type::Error;
                        }
                    }
                }

                self.unary_result_type(op.node, expr_type, &expr.span, ctx)
            }
            ExprKind::VarAssign(target, rhs) => {
                let rhs_type = self.check_expr(rhs, ctx);
                let target_type = self.check_expr(target, ctx);

                if !self.can_assign(rhs_type, target_type, rhs, ctx) {
                    self.error("mismatched types", expr.span, ctx);
                    Type::Error
                } else if !self.is_lvalue(target) {
                    self.error("cannot assign to non-lvalue", expr.span, ctx);
                    Type::Error
                } else {
                    rhs_type
                }
            }
            ExprKind::Variable(..) => {
                // unwrap because we already know the variable exists
                // after name resolution
                let varid = ctx.variables.get(&expr.id).unwrap();
                // unwrap here because we know that this variable is already declared
                *ctx.var_types.get(varid).unwrap()
            }
            ExprKind::Call(callee, args) => {
                if !self.is_callable(callee) {}
                let funcid = *ctx.calls.get(&expr.id).unwrap();
                let function = ctx.signatures.get(&funcid).unwrap().clone();
                if args.len() != function.params.len() {
                    self.error("incorrect number of arguments", expr.span, ctx);
                    return Type::Error;
                }
                for i in 0..args.len() {
                    let ty = self.check_expr(&args[i], ctx);
                    let expected = function.params[i].ty;
                    if !self.can_assign(ty, expected, expr, ctx) {
                        self.error("mismatched types", args[i].span, ctx);
                        return Type::Error;
                    }
                }
                function.return_type
            }
            ExprKind::Error => {
                unreachable!()
            }
        };

        ctx.types.insert(expr.id, ty);

        ty
    }

    fn check_vardecl(&mut self, stmt: &Stmt, decl: &VarDecl, ctx: &mut AnalysisCtx) {
        match decl.type_annotation {
            Some(annotation) => {
                // unwrap because name resolution has already checked
                let varid = *ctx.variables.get(&stmt.id).unwrap();

                match &decl.init {
                    Some(init) => {
                        let initializer_type = self.check_expr(&init, ctx);

                        if !self.can_assign(initializer_type, annotation, &init, ctx) {
                            self.error("mismatched types", stmt.span, ctx);
                            return;
                        }

                        ctx.var_types.insert(varid, annotation);
                    }
                    None => {
                        ctx.var_types.insert(varid, annotation);
                    }
                }
            }
            None => match &decl.init {
                Some(init) => {
                    let initializer_type = self.check_expr(&init, ctx);

                    let varid = *ctx.variables.get(&stmt.id).unwrap();
                    ctx.var_types.insert(varid, initializer_type);
                }
                None => {
                    self.error("type annotation required", stmt.span, ctx);
                }
            },
        }
    }

    fn check_forinit(&mut self, init: &ForInit, ctx: &mut AnalysisCtx) {
        match &init.kind {
            ForInitKind::Decl(decl) => {
                let spanned = Spanned::new(init.clone(), decl.span);
                let stmt: Stmt = spanned.try_into().unwrap();
                self.check_vardecl(&stmt, &decl.node, ctx);
            }
            ForInitKind::Expr(expr) => {
                self.check_expr(&expr, ctx);
            }
        }
    }

    fn is_callable(&self, node: &Expr) -> bool {
        // so functions dont yet exist as first class objects therefore
        // there is no function type so this function cannot be implemented yet
        // just say true for now
        true
    }

    fn is_lvalue(&self, node: &Expr) -> bool {
        match node.kind {
            ExprKind::Variable(..) => true,
            _ => false,
        }
    }

    fn can_assign(&self, value: Type, target: Type, expr: &Expr, ctx: &mut AnalysisCtx) -> bool {
        if target == value {
            return true;
        }

        match (target, value) {
            (Type::Error, _) | (_, Type::Error) => {
                return true;
            }
            (Type::Double, Type::Int) => {
                ctx.conversions.insert(expr.id, Conversion::IntToDouble);
                return true;
            }
            _ => {
                return false;
            }
        }
    }

    fn unary_result_type(
        &mut self,
        op: UnaryOpKind,
        rhs: Type,
        span: &Span,
        ctx: &mut AnalysisCtx,
    ) -> Type {
        if rhs == Type::Error {
            return Type::Error;
        }

        if rhs == Type::Void {
            self.error("invalid use of void expression", *span, ctx);
            return Type::Error;
        }

        match (op, rhs) {
            (UnaryOpKind::Negate, Type::Int) => Type::Int,
            (UnaryOpKind::Negate, Type::Double) => Type::Double,
            (
                UnaryOpKind::Negate
                | UnaryOpKind::PostDecrement
                | UnaryOpKind::PostIncrement
                | UnaryOpKind::PreDecrement
                | UnaryOpKind::PreIncrement,
                Type::Bool,
            ) => Type::Error,
            (UnaryOpKind::PostDecrement, Type::Int) => Type::Int,
            (UnaryOpKind::PostIncrement, Type::Int) => Type::Int,
            (UnaryOpKind::PreDecrement, Type::Int) => Type::Int,
            (UnaryOpKind::PreIncrement, Type::Int) => Type::Int,
            (UnaryOpKind::PostDecrement, Type::Double) => Type::Double,
            (UnaryOpKind::PostIncrement, Type::Double) => Type::Double,
            (UnaryOpKind::PreDecrement, Type::Double) => Type::Double,
            (UnaryOpKind::PreIncrement, Type::Double) => Type::Double,
            (_, Type::Error) => unreachable!(),
            (_, Type::Void) => unreachable!(),
        }
    }

    fn binary_result_type(
        &mut self,
        op: BinOpKind,
        lhs: Type,
        rhs: Type,
        left_id: NodeId,
        right_id: NodeId,
        left_span: Span,
        right_span: Span,
        ctx: &mut AnalysisCtx,
    ) -> Type {
        if lhs == Type::Error || rhs == Type::Error {
            return Type::Error;
        }

        if lhs == Type::Void {
            self.error("invalid use of 'void' expression", left_span, ctx);
            return Type::Error;
        } else if rhs == Type::Void {
            self.error("invalid use of 'void' expression", right_span, ctx);
            return Type::Error;
        }

        match op {
            BinOpKind::Add | BinOpKind::Sub | BinOpKind::Mul | BinOpKind::Div => match (lhs, rhs) {
                (Type::Int, Type::Int) => {
                    return Type::Int;
                }
                (Type::Double, Type::Double) => {
                    return Type::Double;
                }
                (Type::Int, Type::Double) => {
                    ctx.conversions.insert(left_id, Conversion::IntToDouble);
                    return Type::Double;
                }
                (Type::Double, Type::Int) => {
                    ctx.conversions.insert(right_id, Conversion::IntToDouble);
                    return Type::Double;
                }
                (Type::Bool | Type::Error | Type::Void, _) => {
                    return Type::Error;
                }
                (_, Type::Bool | Type::Error | Type::Void) => {
                    return Type::Error;
                }
            },
            BinOpKind::BangEq | BinOpKind::EqEq => match (lhs, rhs) {
                (Type::Int, Type::Int) => {
                    return Type::Bool;
                }
                (Type::Double, Type::Double) => {
                    return Type::Bool;
                }
                (Type::Int, Type::Double) => {
                    ctx.conversions.insert(left_id, Conversion::IntToDouble);
                    return Type::Bool;
                }
                (Type::Double, Type::Int) => {
                    ctx.conversions.insert(right_id, Conversion::IntToDouble);
                    return Type::Bool;
                }
                (Type::Bool, Type::Bool) => {
                    return Type::Bool;
                }
                _ => {
                    return Type::Error;
                }
            },
            BinOpKind::GreaterEq
            | BinOpKind::GreaterThan
            | BinOpKind::LessEq
            | BinOpKind::LessThan => match (lhs, rhs) {
                (Type::Int, Type::Int) => {
                    return Type::Bool;
                }
                (Type::Double, Type::Double) => {
                    return Type::Bool;
                }
                (Type::Int, Type::Double) => {
                    ctx.conversions.insert(left_id, Conversion::IntToDouble);
                    return Type::Bool;
                }
                (Type::Double, Type::Int) => {
                    ctx.conversions.insert(right_id, Conversion::IntToDouble);
                    return Type::Bool;
                }
                _ => {
                    return Type::Error;
                }
            },
            BinOpKind::Or | BinOpKind::And => match (lhs, rhs) {
                (Type::Bool, Type::Bool) => {
                    return Type::Bool;
                }
                _ => {
                    return Type::Error;
                }
            },
        }
    }
}
