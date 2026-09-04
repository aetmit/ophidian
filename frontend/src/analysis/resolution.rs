use macros::Constructor;

use crate::analysis::AnalysisCtx;
use crate::analysis::ids::{GlobalVarId, LocalVarId, VariableId};
use crate::analysis::types::Type;
use crate::diagnostics::{Diagnostic, Severity};
use crate::parse::ast::{Expr, ExprKind, ForInit, Stmt, StmtKind};
use crate::span::Span;
use std::collections::HashMap;

pub struct Scope {
    vars: HashMap<Vec<u8>, LocalVarId>,
}

pub struct GlobalScope {
    pub globals: HashMap<Vec<u8>, GlobalVarId>,
}

impl GlobalScope {
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
        }
    }
}

impl Scope {
    fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }
}

#[derive(Debug, PartialEq, Constructor)]
pub struct GlobalVarDecl {
    pub id: GlobalVarId,
    pub type_annotation: Type,
    pub init: Option<Expr>,
}

pub struct Resolver {
    curr_var_id: LocalVarId,
    loop_depth: usize,
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            curr_var_id: LocalVarId::from(0),
            loop_depth: 0,
        }
    }

    fn error<T: Into<String>>(&mut self, message: T, span: Span, ctx: &mut AnalysisCtx) {
        ctx.diagnostics
            .push(Diagnostic::new(message.into(), span, Severity::Error));
    }

    fn resolve_stmt(&mut self, stmt: &Stmt, ctx: &mut AnalysisCtx) {
        match &stmt.kind {
            StmtKind::VarDecl(decl) => {
                self.resolve_vardecl(stmt, &decl.name, &decl.init, ctx);
            }
            StmtKind::Block(b) => {
                self.resolve_block(&b.body, ctx);
            }
            StmtKind::ExprStmt(e) => {
                self.resolve_exprstmt(&e.expr, ctx);
            }
            StmtKind::Print(p) => {
                self.resolve_print(&p.expr, ctx);
            }
            StmtKind::If(i) => {
                self.resolve_if(&i.condition, &i.body, &i.else_body, ctx);
            }
            StmtKind::While(w) => {
                self.resolve_while(&w.condition, &w.body, ctx);
            }
            StmtKind::For(f) => {
                self.resolve_for(&f.init, &f.condition, &f.increment, &f.body, ctx);
            }
            StmtKind::Break => {
                self.resolve_break(stmt, ctx);
            }
            StmtKind::Continue => {
                self.resolve_continue(stmt, ctx);
            }
            StmtKind::Return(ret) => {
                if let Some(expr) = &ret.expr {
                    self.resolve_expr(&expr, ctx);
                }
            }
            StmtKind::Error => {
                unreachable!()
            }
        }
    }

    fn resolve_vardecl(
        &mut self,
        stmt: &Stmt,
        name: &[u8],
        init: &Option<Expr>,
        ctx: &mut AnalysisCtx,
    ) {
        if let Some(initializer) = init {
            self.resolve_expr(initializer, ctx);
        }

        let varid = self.declare_var(name, ctx, stmt.span);

        ctx.variables.insert(stmt.id, VariableId::Local(varid));
    }

    pub fn resolve_block(&mut self, body: &[Stmt], ctx: &mut AnalysisCtx) {
        self.enter_scope(ctx);

        for s in body {
            self.resolve_stmt(s, ctx);
        }

        self.exit_scope(ctx);
    }

    fn resolve_exprstmt(&mut self, expr: &Expr, ctx: &mut AnalysisCtx) {
        self.resolve_expr(expr, ctx);
    }

    fn resolve_print(&mut self, expr: &Expr, ctx: &mut AnalysisCtx) {
        self.resolve_expr(expr, ctx);
    }

    fn resolve_if(
        &mut self,
        cond: &Expr,
        body: &Stmt,
        else_body: &Option<Box<Stmt>>,
        ctx: &mut AnalysisCtx,
    ) {
        self.resolve_expr(cond, ctx);
        self.resolve_stmt(body, ctx);
        if let Some(e) = else_body {
            self.resolve_stmt(e, ctx);
        }
    }

    fn resolve_while(&mut self, cond: &Expr, body: &Stmt, ctx: &mut AnalysisCtx) {
        self.resolve_expr(cond, ctx);

        self.loop_depth += 1;
        self.resolve_stmt(body, ctx);
        self.loop_depth -= 1;
    }

    fn resolve_for(
        &mut self,
        init: &Option<Box<ForInit>>,
        cond: &Option<Expr>,
        incre: &Option<Expr>,
        body: &Stmt,
        ctx: &mut AnalysisCtx,
    ) {
        self.enter_scope(ctx);

        match init {
            Some(init) => match &**init {
                ForInit::Expr(expr) => {
                    self.resolve_expr(&expr, ctx);
                }
                ForInit::Decl(decl) => {
                    self.resolve_vardecl(&body, &decl.node.name, &decl.node.init, ctx);
                }
            },
            None => {
                // no name resolution needed here
            }
        }

        match cond {
            Some(cond) => {
                self.resolve_expr(cond, ctx);
            }
            None => {
                // no name resolution needed here
            }
        }

        match incre {
            Some(incre) => {
                self.resolve_expr(incre, ctx);
            }
            None => {
                // no name resolution needed here
            }
        }

        self.loop_depth += 1;

        self.resolve_stmt(body, ctx);

        self.loop_depth -= 1;

        self.exit_scope(ctx);
    }

    fn resolve_break(&mut self, stmt: &Stmt, ctx: &mut AnalysisCtx) {
        if self.loop_depth == 0 {
            self.error("use of 'break' statement outside loop", stmt.span, ctx);
        }
    }

    fn resolve_continue(&mut self, stmt: &Stmt, ctx: &mut AnalysisCtx) {
        if self.loop_depth == 0 {
            self.error("use of continue statement outside loop", stmt.span, ctx);
        }
    }

    fn resolve_expr(&mut self, expr: &Expr, ctx: &mut AnalysisCtx) {
        match &expr.kind {
            ExprKind::VarAssign(target, expr) => {
                self.resolve_expr(expr, ctx);
                self.resolve_expr(target, ctx);
            }
            ExprKind::Variable(name) => match self.lookup_var(name, ctx) {
                Some(id) => {
                    match id {
                        VariableId::Global(id) => {
                            ctx.variables.insert(expr.id, VariableId::Global(id));
                        }
                        VariableId::Local(id) => {
                            ctx.variables.insert(expr.id, VariableId::Local(id));
                        }
                    }
                }
                None => {
                    self.error(
                        format!(
                            "use of undeclared identifer: '{}'",
                            std::str::from_utf8(name).unwrap()
                        ),
                        expr.span,
                        ctx,
                    );
                }
            },
            ExprKind::BinaryOp(.., left, right) => {
                self.resolve_expr(left, ctx);
                self.resolve_expr(right, ctx);
            }
            ExprKind::UnaryOp(.., right) => {
                self.resolve_expr(right, ctx);
            }
            ExprKind::Call(callee, args) => {
                self.resolve_expr(callee, ctx);
                for arg in args {
                    self.resolve_expr(arg, ctx);
                }

                let ident = match &callee.kind {
                    ExprKind::Variable(ident) => ident,
                    _ => {
                        panic!("functions are not yet first class objects")
                    }
                };

                let id = ctx.function_scope.get(ident).unwrap();
                ctx.calls.insert(expr.id, *id);
            }
            _ => return,
        }
    }

    pub(super) fn declare_existing_var(
        &mut self,
        name: &[u8],
        id: LocalVarId,
        ctx: &mut AnalysisCtx,
        span: Span,
    ) {
        if ctx.scopes.last().unwrap().vars.contains_key(name) {
            self.error(
                format!(
                    "redeclaration of identifer '{}'",
                    std::str::from_utf8(name).unwrap()
                ),
                span,
                ctx,
            );
            return;
        }

        ctx.scopes
            .last_mut()
            .unwrap()
            .vars
            .insert(name.to_vec(), id);
    }

    fn declare_var(&mut self, name: &[u8], ctx: &mut AnalysisCtx, span: Span) -> LocalVarId {
        if ctx.scopes.last().unwrap().vars.contains_key(name) {
            self.error(
                format!(
                    "redeclaration of identifer '{}'",
                    std::str::from_utf8(name).unwrap()
                ),
                span,
                ctx,
            );
            return LocalVarId::ERROR;
        }

        let id = self.curr_var_id;
        ctx.scopes
            .last_mut()
            .unwrap()
            .vars
            .insert(name.to_vec(), id);
        self.curr_var_id += 1;
        id
    }

    fn lookup_var(&mut self, name: &[u8], ctx: &mut AnalysisCtx) -> Option<VariableId> {
        let local = ctx
            .scopes
            .iter()
            .rev()
            .find_map(|s| s.vars.get(name).copied());

        match local {
            Some(local) => {
                return Some(VariableId::Local(local));
            }
            None => {
                let id = ctx.global_scope.globals.get(name).copied();

                match id {
                    Some(id) => {
                        return Some(VariableId::Global(id));
                    }
                    None => {
                        return None;
                    }
                }
            }
        }
    }

    pub(super) fn enter_scope(&self, ctx: &mut AnalysisCtx) {
        ctx.scopes.push(Scope::new());
    }

    pub(super) fn exit_scope(&self, ctx: &mut AnalysisCtx) {
        ctx.scopes.pop();
    }
}
