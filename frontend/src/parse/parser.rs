use crate::analysis::types::Type;
use crate::lex::token::{TokenKind, TokenStream};
use crate::parse::Parser;
use crate::parse::ast::{
    BinOpKind, Block, Expr, ExprKind, ExprStmt, For, ForInit, ForInitKind, Function, GlobalVarDecl, If, Item, LitKind, Param, Print, Program, Stmt, StmtKind, UnaryOpKind, VarDecl, While,
};
use crate::span::{Span, Spanned};

impl<'src, 'diag, T: TokenStream> Parser<'src, 'diag, T> {
    pub fn parse(&mut self) -> Program {
        let mut program = Program::new();

        while self.peek().kind != TokenKind::Eof {
            if self.peek().kind == TokenKind::Fn {
                let function = match self.parse_fn() {
                    Ok(function) => function,
                    Err(_) => {
                        continue;
                    }
                };

                program.items.push(Item::Function(function));
            } else {
                match self.peek().kind {
                    TokenKind::Let => {
                        let decl = match self.parse_global_var_decl() {
                            Ok(decl) => decl,
                            Err(_) => {
                                continue;
                            }
                        };
                        program.items.push(Item::GlobalVarDecl(decl));
                    }
                    _ => {
                        self.error("statement not allowed at file scope", self.peek().span);
                        continue;
                    }
                }
            }
        }

        program
    }

    pub fn parse_statement(&mut self) -> Stmt {
        match self.peek().kind {
            TokenKind::Print => self.parse_print(),
            TokenKind::IntegerLiteral | TokenKind::OpenParen | TokenKind::Identifier => {
                self.parse_exprstmt()
            }
            TokenKind::Let => self.parse_var_decl(),
            TokenKind::OpenBrace => self.parse_block(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::Break => self.parse_break(),
            TokenKind::Continue => self.parse_continue(),
            TokenKind::For => self.parse_for(),
            _ => {
                let span = self.peek().span;
                self.error("unexpected token", span);
                self.sync();
                return Stmt::new(self.next_node_id(), StmtKind::Error, span);
            }
        }
    }

    fn parse_global_var_decl(&mut self) -> Result<GlobalVarDecl, ()> {
        let start_span = self.advance().span;

        if self.peek().kind != TokenKind::Identifier {
            self.error("expected identifier", self.peek().span);
            return Err(());
        }

        let identifier = Span::retrieve_slice(self.source, &self.peek().span).to_owned();

        self.advance();

        if self.peek().kind != TokenKind::Colon {
            self.error("expected type annotation", self.peek().span);
            return Err(());
        }

        self.advance();

        let var_type = {
            let ty: Type = self.peek().kind.into();
            if ty == Type::Error {
                let end_span = self.advance().span;
                self.error("expected type name", end_span);
                return Err(());
            }
            self.advance();
            ty
        };

        match self.peek().kind {
            TokenKind::Equal => {
                self.advance();
                let init = self.parse_expression();
                if self.peek().kind != TokenKind::Semicolon {
                    self.error("expected ';'", self.peek().span);
                    return Err(());
                }

                let end_span = self.advance().span;

                return Ok(GlobalVarDecl::new(
                    self.next_node_id(),
                    start_span.join(end_span),
                    identifier,
                    var_type,
                    Some(init),
                ));
            }
            TokenKind::Semicolon => {
                let end_span = self.advance().span;

                return Ok(GlobalVarDecl::new(
                    self.next_node_id(),
                    start_span.join(end_span),
                    identifier,
                    var_type,
                    None,
                ));
            }
            _ => {
                self.error("expected ';'", self.peek().span);
                return Err(());
            }
        }
    }

    fn parse_fn(&mut self) -> Result<Function, ()> {
        let start_span = self.advance().span;

        if self.peek().kind != TokenKind::Identifier {
            self.error("expected identifer", self.peek().span);
            self.sync_fn();
            return Err(());
        }

        let ident = Span::retrieve_slice(self.source, &self.peek().span).to_vec();
        self.advance();

        if self.peek().kind != TokenKind::OpenParen {
            self.error("expected '('", self.peek().span);
            self.sync_fn();
            return Err(());
        }

        self.advance();

        let params = match self.parse_params() {
            Ok(params) => params,
            Err((msg, span)) => {
                self.error(msg, span);
                self.sync_fn();
                return Err(());
            }
        };

        if self.peek().kind != TokenKind::CloseParen {
            self.error("expected ')'", self.peek().span);
            self.sync_fn();
            return Err(());
        }

        self.advance();

        let ret_type = if self.peek().kind == TokenKind::Arrow {
            self.advance();
            let ty: Type = self.peek().kind.into();
            if ty == Type::Error {
                self.error("expected return type", self.peek().span);
                self.sync_fn();
                return Err(());
            }
            self.advance();
            Some(ty)
        } else {
            None
        };

        if self.peek().kind != TokenKind::OpenBrace {
            self.error("expected block", self.peek().span);
            self.sync_fn();
            return Err(());
        }

        // guarenteed to return block
        let body = self.parse_statement();
        let end_span = body.span;

        return Ok(Function::new(
            self.next_node_id(),
            start_span.join(end_span),
            ident,
            params,
            ret_type,
            Spanned::new(body.try_into().expect("unreachable"), end_span),
        ));
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, (String, Span)> {
        let mut params = Vec::new();

        if self.peek().kind == TokenKind::CloseParen {
            return Ok(params);
        }

        params.push(self.parse_param()?);
        while self.peek().kind == TokenKind::Comma {
            self.advance();
            params.push(self.parse_param()?);
        }

        Ok(params)
    }

    fn parse_param(&mut self) -> Result<Param, (String, Span)> {
        if self.peek().kind != TokenKind::Identifier {
            return Err(("expected identifier".to_string(), self.peek().span));
        }

        let start_span = self.peek().span;

        let ident = Span::retrieve_slice(self.source, &self.peek().span).to_vec();
        self.advance();

        if self.peek().kind != TokenKind::Colon {
            return Err(("expected ':'".to_string(), self.peek().span));
        }

        self.advance();

        let (ty, end_span) = {
            let ty = self.peek().kind.into();
            if ty == Type::Error {
                return Err(("expected type annotation".to_string(), self.peek().span));
            }
            let end_span = self.advance().span;
            (ty, end_span)
        };

        return Ok(Param::new(
            self.next_node_id(),
            ident,
            ty,
            start_span.join(end_span),
        ));
    }

    fn parse_for(&mut self) -> Stmt {
        let start_span = self.advance().span;

        if self.peek().kind != TokenKind::OpenParen {
            self.error("expected '('", self.peek().span);
            return Stmt::new(self.next_node_id(), StmtKind::Error, self.peek().span);
        }

        self.advance();

        let init = if self.peek().kind == TokenKind::Let {
            // parse_statement guarenteed to parse a var decl because of TokenKind::Let
            let stmt = self.parse_statement();
            let span = stmt.span;
            Some(Box::new(ForInit::new(ForInitKind::Decl(Spanned::new(
                stmt.try_into().expect("unreacheable"),
                span,
            )), self.next_node_id())))
        } else if self.peek().kind == TokenKind::Semicolon {
            self.advance();
            None
        } else {
            let val = Some(Box::new(ForInit::new(ForInitKind::Expr(self.parse_expression()), self.next_node_id())));
            if self.peek().kind != TokenKind::Semicolon {
                self.error("expected ';' after loop initializer", self.peek().span);
                return Stmt::new(self.next_node_id(), StmtKind::Error, self.peek().span);
            }
            self.advance();
            val
        };

        let cond = if self.peek().kind == TokenKind::Semicolon {
            None
        } else {
            Some(self.parse_expression())
        };

        if self.peek().kind != TokenKind::Semicolon {
            self.error("expected ';' after for loop condition", self.peek().span);
            return Stmt::new(self.next_node_id(), StmtKind::Error, self.peek().span);
        }

        self.advance();

        let incre = if self.peek().kind == TokenKind::CloseParen {
            None
        } else {
            Some(self.parse_expression())
        };

        if self.peek().kind != TokenKind::CloseParen {
            self.error("expected ')'", self.peek().span);
            return Stmt::new(self.next_node_id(), StmtKind::Error, self.peek().span);
        }

        self.advance();

        let body = self.parse_statement();

        let end_span = body.span;

        return Stmt::new(
            self.next_node_id(),
            StmtKind::For(For::new(init, cond, incre, Box::new(body))),
            start_span.join(end_span),
        );
    }

    fn parse_break(&mut self) -> Stmt {
        let span = self.advance().span;

        if self.peek().kind != TokenKind::Semicolon {
            self.error("expected ';' after break statement", self.peek().span);
            return Stmt::new(self.next_node_id(), StmtKind::Error, self.peek().span);
        }

        let end_span = self.advance().span;

        return Stmt::new(self.next_node_id(), StmtKind::Break, span.join(end_span));
    }

    fn parse_continue(&mut self) -> Stmt {
        let span = self.advance().span;

        if self.peek().kind != TokenKind::Semicolon {
            self.error("expected ';' after continue statement", self.peek().span);
            return Stmt::new(self.next_node_id(), StmtKind::Error, self.peek().span);
        }

        let end_span = self.advance().span;

        return Stmt::new(self.next_node_id(), StmtKind::Continue, span.join(end_span));
    }

    fn parse_while(&mut self) -> Stmt {
        let start_span = self.advance().span;

        if self.peek().kind != TokenKind::OpenParen {
            self.error("expected '('", self.peek().span);
            return Stmt::new(self.next_node_id(), StmtKind::Error, self.peek().span);
        }

        self.advance();
        let cond = self.parse_expression();

        if self.peek().kind != TokenKind::CloseParen {
            self.error("expected ')'", self.peek().span);
            return Stmt::new(self.next_node_id(), StmtKind::Error, self.peek().span);
        }

        self.advance();

        let body = self.parse_statement();
        let end_span = body.span;

        return Stmt::new(
            self.next_node_id(),
            StmtKind::While(While::new(Box::new(cond), Box::new(body))),
            start_span.join(end_span),
        );
    }

    fn parse_if(&mut self) -> Stmt {
        // advance past the if and get its span
        let start_span = self.advance().span;

        if self.peek().kind != TokenKind::OpenParen {
            let span = self.peek().span;
            self.error("expected '('", span);
            self.sync();
            return Stmt::new(self.next_node_id(), StmtKind::Error, span);
        }

        self.advance();
        let cond = self.parse_expression();

        if self.peek().kind != TokenKind::CloseParen {
            let span = self.peek().span;
            self.error("expected ')'", span);
            self.sync();
            return Stmt::new(self.next_node_id(), StmtKind::Error, span);
        }
        self.advance();

        let body = self.parse_statement();

        if self.peek().kind == TokenKind::Else {
            self.advance();
            if self.peek().kind == TokenKind::If {
                let if_stmt = self.parse_if();
                let end_span = if_stmt.span;
                return Stmt::new(
                    self.next_node_id(),
                    StmtKind::If(If::new(
                        Box::new(cond),
                        Box::new(body),
                        Some(Box::new(if_stmt)),
                    )),
                    start_span.join(end_span),
                );
            }
            let else_body = self.parse_statement();
            let end_span = else_body.span;
            return Stmt::new(
                self.next_node_id(),
                StmtKind::If(If::new(
                    Box::new(cond),
                    Box::new(body),
                    Some(Box::new(else_body)),
                )),
                start_span.join(end_span),
            );
        } else {
            let end_span = body.span;
            return Stmt::new(
                self.next_node_id(),
                StmtKind::If(If::new(Box::new(cond), Box::new(body), None)),
                start_span.join(end_span),
            );
        }
    }

    fn parse_block(&mut self) -> Stmt {
        let start_span = self.advance().span;

        let mut stmts = Vec::<Stmt>::new();

        while self.peek().kind != TokenKind::Eof {
            if self.peek().kind == TokenKind::CloseBrace {
                break;
            }
            stmts.push(self.parse_statement());
        }

        let end_token = self.advance();

        if end_token.kind != TokenKind::CloseBrace {
            self.error("unterminated block", end_token.span);
            return Stmt::new(self.next_node_id(), StmtKind::Error, end_token.span);
        }

        Stmt::new(
            self.next_node_id(),
            StmtKind::Block(Block::new(stmts)),
            start_span.join(end_token.span),
        )
    }

    fn parse_var_decl(&mut self) -> Stmt {
        let start_span = self.advance().span;

        if self.peek().kind != TokenKind::Identifier {
            self.error("expected identifer", start_span);
            return Stmt::new(self.next_node_id(), StmtKind::Error, start_span);
        }

        // peek is now guarenteed to be an identifier
        let identifier = Span::retrieve_slice(self.source, &self.peek().span).to_vec();

        self.advance();

        match self.peek().kind {
            TokenKind::Colon => {
                self.advance();

                let var_type = {
                    let ty: Type = self.peek().kind.into();
                    if ty == Type::Error {
                        let end_span = self.advance().span;
                        self.error("expected a type annotation", end_span);
                        return Stmt::new(self.next_node_id(), StmtKind::Error, end_span);
                    }
                    self.advance();
                    ty
                };

                match self.peek().kind {
                    TokenKind::Equal => {
                        self.advance();
                        let init = self.parse_expression();

                        if self.peek().kind != TokenKind::Semicolon {
                            let end_span = self.advance().span;
                            self.error("expected ';' after variable declaration", end_span);
                            return Stmt::new(self.next_node_id(), StmtKind::Error, end_span);
                        }

                        let end_span = self.advance().span;

                        return Stmt::new(
                            self.next_node_id(),
                            StmtKind::VarDecl(VarDecl::new(identifier, Some(var_type), Some(init))),
                            start_span.join(end_span),
                        );
                    }
                    TokenKind::Semicolon => {
                        let end_span = self.advance().span;

                        return Stmt::new(
                            self.next_node_id(),
                            StmtKind::VarDecl(VarDecl::new(identifier, Some(var_type), None)),
                            start_span.join(end_span),
                        );
                    }
                    _ => {
                        let end_span = self.peek().span;

                        self.error("expected '=' or ';'", end_span);

                        return Stmt::new(
                            self.next_node_id(),
                            StmtKind::Error,
                            start_span.join(end_span),
                        );
                    }
                }
            }
            TokenKind::Equal => {
                self.advance();

                let init = self.parse_expression();

                if self.peek().kind != TokenKind::Semicolon {
                    let end_span = self.advance().span;
                    self.error("expected ';' after expression", start_span.join(end_span));
                    return Stmt::new(
                        self.next_node_id(),
                        StmtKind::Error,
                        start_span.join(end_span),
                    );
                }

                let end_span = self.advance().span;

                return Stmt::new(
                    self.next_node_id(),
                    StmtKind::VarDecl(VarDecl::new(identifier, None, Some(init))),
                    start_span.join(end_span),
                );
            }
            _ => {
                self.error("expected '=' or ';'", start_span.join(self.peek().span));
                return Stmt::new(
                    self.next_node_id(),
                    StmtKind::Error,
                    start_span.join(self.peek().span),
                );
            }
        }
    }

    fn parse_exprstmt(&mut self) -> Stmt {
        let expr = self.parse_expression();

        if self.peek().kind != TokenKind::Semicolon {
            let span = self.peek().span;
            self.error("expected ';' after statement", span);
            self.sync();
            return Stmt::new(self.next_node_id(), StmtKind::Error, span);
        }

        let span = expr.span.join(self.advance().span);
        Stmt::new(
            self.next_node_id(),
            StmtKind::ExprStmt(ExprStmt::new(Box::new(expr))),
            span,
        )
    }

    fn parse_print(&mut self) -> Stmt {
        let span = self.advance().span;

        if self.peek().kind != TokenKind::OpenParen {
            let span = self.advance().span;
            self.error("expected '(' after keyword 'print'", span);
            self.sync();
            return Stmt::new(self.next_node_id(), StmtKind::Error, span);
        }

        self.advance();

        let expr = self.parse_expression();

        if self.peek().kind != TokenKind::CloseParen {
            let span = self.advance().span;
            self.error("expected ')' after expression", span);
            self.sync();
            return Stmt::new(self.next_node_id(), StmtKind::Error, span);
        }

        self.advance();

        if self.peek().kind != TokenKind::Semicolon {
            let span = self.advance().span;
            self.error("expected ';' after statement", span);
            self.sync();
            return Stmt::new(self.next_node_id(), StmtKind::Error, span);
        }

        Stmt::new(
            self.next_node_id(),
            StmtKind::Print(Print::new(Box::new(expr))),
            self.advance().span.join(span),
        )
    }

    fn parse_expression(&mut self) -> Expr {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Expr {
        let left = self.parse_or();

        if self.peek().kind == TokenKind::Equal {
            self.advance();

            let right = self.parse_assignment();

            let span = left.span.join(right.span);

            return Expr::new(
                self.next_node_id(),
                ExprKind::VarAssign(Box::new(left), Box::new(right)),
                span,
            );
        }

        left
    }

    fn parse_or(&mut self) -> Expr {
        let mut left = self.parse_and();
        let start_span = left.span;
        while self.peek().kind == TokenKind::Or {
            let op_span = self.advance().span;
            let right = self.parse_and();
            let right_span = right.span;
            left = Expr::new(
                self.next_node_id(),
                ExprKind::BinaryOp(
                    Spanned::new(BinOpKind::Or, op_span),
                    Box::new(left),
                    Box::new(right),
                ),
                start_span.join(right_span),
            )
        }

        left
    }

    fn parse_and(&mut self) -> Expr {
        let mut left = self.parse_equality();
        let start_span = left.span;
        while self.peek().kind == TokenKind::And {
            let op_span = self.advance().span;
            let right = self.parse_equality();
            let right_span = right.span;
            left = Expr::new(
                self.next_node_id(),
                ExprKind::BinaryOp(
                    Spanned::new(BinOpKind::And, op_span),
                    Box::new(left),
                    Box::new(right),
                ),
                start_span.join(right_span),
            )
        }

        left
    }

    fn parse_equality(&mut self) -> Expr {
        let mut left = self.parse_comparison();
        let start_span = left.span;
        while matches!(
            self.peek().kind,
            TokenKind::BangEqual | TokenKind::EqualEqual
        ) {
            let op = self.peek();
            self.advance();

            let right = self.parse_comparison();

            let right_span = right.span;

            left = Expr::new(
                self.next_node_id(),
                ExprKind::BinaryOp(
                    Spanned::new(op.into(), op.span),
                    Box::new(left),
                    Box::new(right),
                ),
                start_span.join(right_span),
            );
        }

        left
    }

    fn parse_comparison(&mut self) -> Expr {
        let mut left = self.parse_term();
        let start_span = left.span;

        while matches!(
            self.peek().kind,
            TokenKind::LessThan | TokenKind::GreaterThan | TokenKind::GreaterEq | TokenKind::LessEq
        ) {
            let op = self.peek();
            self.advance();
            let right = self.parse_term();
            let right_span = right.span;

            left = Expr::new(
                self.next_node_id(),
                ExprKind::BinaryOp(
                    Spanned::new(op.into(), op.span),
                    Box::new(left),
                    Box::new(right),
                ),
                start_span.join(right_span),
            );
        }

        left
    }

    fn parse_term(&mut self) -> Expr {
        let mut left = self.parse_factor();
        let start_span = left.span;
        while self.peek().kind == TokenKind::Plus || self.peek().kind == TokenKind::Minus {
            let op = self.advance();

            let right = self.parse_factor();

            // we create a copy of the span for the rhs of the expr because
            // we cannot access the span field after we move it into a box
            let right_span = right.span;

            if op.kind == TokenKind::Plus {
                left = Expr::new(
                    self.next_node_id(),
                    ExprKind::BinaryOp(
                        Spanned::new(BinOpKind::Add, op.span),
                        Box::new(left),
                        Box::new(right),
                    ),
                    start_span.join(right_span),
                );
            } else {
                left = Expr::new(
                    self.next_node_id(),
                    ExprKind::BinaryOp(
                        Spanned::new(BinOpKind::Sub, op.span),
                        Box::new(left),
                        Box::new(right),
                    ),
                    start_span.join(right_span),
                )
            }
        }
        left
    }

    fn parse_factor(&mut self) -> Expr {
        let mut left = self.parse_unary();
        let start_span = left.span;
        while self.peek().kind == TokenKind::Star || self.peek().kind == TokenKind::Slash {
            let op = self.advance();

            let right = self.parse_unary();

            // we create a copy of the span for right because we cannot access
            // it after we move it into a Box
            let right_span = right.span;

            if op.kind == TokenKind::Star {
                left = Expr::new(
                    self.next_node_id(),
                    ExprKind::BinaryOp(
                        Spanned::new(BinOpKind::Mul, op.span),
                        Box::new(left),
                        Box::new(right),
                    ),
                    start_span.join(right_span),
                )
            } else {
                left = Expr::new(
                    self.next_node_id(),
                    ExprKind::BinaryOp(
                        Spanned::new(BinOpKind::Div, op.span),
                        Box::new(left),
                        Box::new(right),
                    ),
                    start_span.join(right_span),
                )
            }
        }
        left
    }

    fn parse_unary(&mut self) -> Expr {
        if self.peek().kind == TokenKind::Minus {
            let op_span = self.advance().span;

            let right = self.parse_unary();

            // we save the span of the expr because we move right into a Box
            // before we can pass the span into the function
            let right_span = right.span;

            return Expr::new(
                self.next_node_id(),
                ExprKind::UnaryOp(Spanned::new(UnaryOpKind::Negate, op_span), Box::new(right)),
                right_span.join(op_span),
            );
        } else if self.peek().kind == TokenKind::PlusPlus {
            let op_span = self.advance().span;

            let operand = self.parse_unary();
            let operand_span = operand.span;

            return Expr::new(
                self.next_node_id(),
                ExprKind::UnaryOp(
                    Spanned::new(UnaryOpKind::PreIncrement, op_span),
                    Box::new(operand),
                ),
                op_span.join(operand_span),
            );
        } else if self.peek().kind == TokenKind::MinusMinus {
            let op_span = self.advance().span;

            let operand = self.parse_unary();

            let operand_span = operand.span;

            return Expr::new(
                self.next_node_id(),
                ExprKind::UnaryOp(
                    Spanned::new(UnaryOpKind::PreDecrement, op_span),
                    Box::new(operand),
                ),
                op_span.join(operand_span),
            );
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();

        loop {
            match self.peek().kind {
                TokenKind::OpenParen => {
                    let open_span = self.advance().span;
                    let mut args = Vec::new();

                    if self.peek().kind != TokenKind::CloseParen {
                        loop {
                            args.push(self.parse_expression());

                            if self.peek().kind == TokenKind::CloseParen {
                                break;
                            }

                            if self.peek().kind != TokenKind::Comma {
                                let span = self.peek().span;
                                self.error("expected ',' or ')' after argument", span);
                                self.sync();
                                return Expr::new(self.next_node_id(), ExprKind::Error, span);
                            }

                            self.advance();

                            if self.peek().kind == TokenKind::CloseParen {
                                let span = self.peek().span;
                                self.error("trailing comma in function arguments", span);
                                self.sync();
                                return Expr::new(self.next_node_id(), ExprKind::Error, span);
                            }
                        }
                    }

                    if self.peek().kind != TokenKind::CloseParen {
                        let span = self.peek().span;
                        self.error("expected ')' after arguments to function", span);
                        self.sync();
                        return Expr::new(self.next_node_id(), ExprKind::Error, span);
                    }

                    let close_span = self.advance().span;

                    let expr_span = expr.span;

                    expr = Expr::new(
                        self.next_node_id(),
                        ExprKind::Call(Box::new(expr), args),
                        expr_span.join(open_span).join(close_span),
                    );
                }
                TokenKind::PlusPlus | TokenKind::MinusMinus => {
                    let op_kind = self.peek().kind;
                    let op_span = self.advance().span;

                    let expr_span = expr.span;

                    expr = match op_kind {
                        TokenKind::PlusPlus => Expr::new(
                            self.next_node_id(),
                            ExprKind::UnaryOp(
                                Spanned::new(UnaryOpKind::PostIncrement, op_span),
                                Box::new(expr),
                            ),
                            op_span.join(expr_span),
                        ),
                        TokenKind::MinusMinus => Expr::new(
                            self.next_node_id(),
                            ExprKind::UnaryOp(
                                Spanned::new(UnaryOpKind::PostDecrement, op_span),
                                Box::new(expr),
                            ),
                            op_span.join(expr_span),
                        ),
                        _ => unreachable!(),
                    };
                }
                _ => {
                    break;
                }
            }
        }

        expr
    }

    fn parse_primary(&mut self) -> Expr {
        if self.peek().kind == TokenKind::IntegerLiteral {
            self.advance();

            let lexeme = Span::retrieve_slice(self.source, &self.prev.span);

            let value = std::str::from_utf8(lexeme)
                // we expect the user to uphold the invariant of the source string
                // being valid ascii and therefore valid utf8 so if the user fails
                // to uphold that invariant, the compiler will panic and crash
                .expect("source code is assumed to be valid ascii and therefore valid utf8")
                .parse::<u128>()
                // TODO: handle integer parsing errors more elegantly
                .expect("failed to parse integer");

            return Expr::new(
                self.next_node_id(),
                ExprKind::Literal(LitKind::Int(value)),
                self.prev.span,
            );
        } else if self.peek().kind == TokenKind::FloatLiteral {
            self.advance();
            let lexeme = Span::retrieve_slice(self.source, &self.prev.span);

            let value = std::str::from_utf8(lexeme)
                .expect("source code is assumed to be valid ascii and therefore valid utf8")
                .parse::<f64>()
                .expect("failed to pass float");

            return Expr::new(
                self.next_node_id(),
                ExprKind::Literal(LitKind::Float(value)),
                self.prev.span,
            );
        } else if self.peek().kind == TokenKind::OpenParen {
            // advnace past the open parentheses
            self.advance();

            // span of the `(`
            let open_span = self.prev.span;

            let mut expr = self.parse_expression();

            if self.peek().kind != TokenKind::CloseParen {
                let span = self.advance().span;
                self.error("expected ')' after expression", span);
                return Expr::new(self.next_node_id(), ExprKind::Error, span);
            }
            self.advance();

            // span of the `)`
            let close_span = self.prev.span;

            expr.span = expr.span.join(open_span).join(close_span);

            return expr;
        } else if self.peek().kind == TokenKind::Identifier {
            let name = Span::retrieve_slice(self.source, &self.peek().span).to_vec();
            let span = self.advance().span;
            return Expr::new(self.next_node_id(), ExprKind::Variable(name), span);
        } else if matches!(self.peek().kind, TokenKind::True | TokenKind::False) {
            self.advance();
            return Expr::new(
                self.next_node_id(),
                ExprKind::Literal(LitKind::Bool(self.prev.kind.into())),
                self.prev.span,
            );
        } else {
            let span = self.peek().span;
            self.error(
                format!(
                    "unexpected token: '{:#?}'",
                    Span::retrieve_slice(self.source, &span)
                ),
                span,
            );
            return Expr::new(self.next_node_id(), ExprKind::Error, span);
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::diagnostics::Diagnostic;
    use crate::lex::Lexer;
    use crate::lex::token::Token;
    use crate::parse::ast::NodeId;

    fn new_diag_vec() -> Vec<Diagnostic> {
        Vec::new()
    }

    fn new_parser<'a: 'diag, 'src: 'a, 'diag>(
        source: &'src [u8],
        diag: &'a mut Vec<Diagnostic>,
    ) -> Parser<'src, 'diag, Lexer<'a>> {
        let lexer = Lexer::new(source);
        Parser::new(lexer, diag, source)
    }

    #[test]
    // test node id generation
    fn test_nodeid_generation() {
        let mut diag = new_diag_vec();
        let mut parser = new_parser(b"(1 + 2) * 3", &mut diag);
        parser.next_node_id();
        assert_eq!(NodeId(1), parser.next_node_id());
    }

    #[test]
    // test that peek returns the correct token
    fn test_peek() {
        let mut diag = new_diag_vec();
        let parser = new_parser(b"(", &mut diag);

        assert_eq!(
            parser.peek(),
            Token::new(TokenKind::OpenParen, Span::new(0, 1), 0, 0)
        );
    }

    #[test]
    fn test_peek_at_eof() {
        let mut diag = new_diag_vec();
        let parser = new_parser(b"", &mut diag);

        assert_eq!(
            parser.peek(),
            Token::new(TokenKind::Eof, Span::new(0, 0), 0, 0)
        );
    }

    // test that advnace returns the token that it just advancde
    // past
    #[test]
    fn test_advance_returns_correct_token() {
        let mut diag = new_diag_vec();
        let mut parser = new_parser(b"(1)", &mut diag);

        assert_eq!(
            parser.advance(),
            Token::new(TokenKind::OpenParen, Span::new(0, 1), 0, 0)
        );
    }

    #[test]
    fn test_advance_at_eof() {
        let mut diag = new_diag_vec();
        let mut parser = new_parser(b"", &mut diag);

        assert_eq!(
            parser.advance(),
            Token::new(TokenKind::Eof, Span::new(0, 0), 0, 0)
        )
    }
}
