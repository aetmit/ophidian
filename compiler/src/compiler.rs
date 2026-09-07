use cli::options::Options;
use frontend::analysis::SemanticAnalyzer;
use frontend::analysis::types::Type;
use frontend::diagnostics::Diagnostic;
use frontend::lex::Lexer;
use frontend::parse::Parser;
use frontend::analysis::hir::{BinaryOpKind, Expr, ExprKind, ForInit, ForInitKind, Function, GlobalVarDecl, Item, LiteralKind, Program, Stmt, StmtKind, UnaryOpKind};
use frontend::analysis::ids::LocalVarId;
use runtime::chunk::Chunk;
use runtime::disassembler::Disassembler;
use runtime::opcodes::OpCode;
use runtime::value::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct LocalSlot(usize);

impl From<LocalSlot> for u32 {
    fn from(value: LocalSlot) -> Self {
        value
            .0
            .try_into()
            .expect("local slot index exceeds u32::MAX")
    }
}

#[derive(Debug, Clone)]
pub struct LoopContext {
    continue_jumps: Vec<usize>,
    break_jumps: Vec<usize>,
}

pub struct Compiler {
    locals: HashMap<LocalVarId, LocalSlot>,
    loop_stack: Vec<LoopContext>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            locals: HashMap::new(),
            loop_stack: Vec::new(),
        }
    }

    #[must_use]
    pub fn compile(&mut self, source: &[u8], options: Options) -> Result<Chunk, Vec<Diagnostic>> {
        let mut diagnostics = Vec::<Diagnostic>::new();
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer, &mut diagnostics, source);

        let program = parser.parse();

        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }

        let mut analyzer = SemanticAnalyzer::new(&mut diagnostics);
        let hir = analyzer.analyze(program);

        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }

        let hir = hir.unwrap();

        let mut chunk = Chunk::new();

        self.compile_program(&hir, &mut chunk);        

        if options.dump_bytecode {
            let disassembler = Disassembler::new(&chunk);
            disassembler.disassemble();
        }

        Ok(chunk)
    }

    fn compile_program(&mut self, program: &Program, chunk: &mut Chunk) {
        for item in &program.items {
            match item {
                Item::Function(function) => {
                    self.compile_fn(function, &mut chunk);
                }
                Item::GlobalVarDecl(decl) => {
                    self.compile_gloval_var_decl(decl, &mut chunk);
                }
            }
        }

        chunk.write(OpCode::LoadConst as u8);
        let idx = chunk.write_constant(Value::new_int(0));
        chunk.write_u24(idx as u32);

        chunk.write(OpCode::Halt as u8);
    }

    fn compile_fn(&mut self, function: &Function, chunk: &mut Chunk) {

    }

    fn compile_gloval_var_decl(&mut self, decl: &GlobalVarDecl, chunk: &mut Chunk) {

    }

    fn compile_stmt(&mut self, stmt: &Stmt, chunk: &mut Chunk) {
        match &stmt.kind {
            StmtKind::ExprStmt(expr) => {
                self.compile_expr(&expr.expr, chunk);

                chunk.write(OpCode::Pop as u8);
            }
            StmtKind::Print(print) => {
                self.compile_expr(&print.expr, chunk);

                match metadata.converted_types.get(&expr.id).unwrap() {
                    Type::Int => {
                        chunk.write(OpCode::I32Print as u8);
                    }
                    Type::Bool => {
                        chunk.write(OpCode::BPrint as u8);
                    }
                    Type::Double => {
                        chunk.write(OpCode::F64Print as u8);
                    }
                    Type::Void => {
                        todo!()
                    }
                    Type::Error => {
                        unreachable!()
                    }
                }
            }
            StmtKind::VarDecl(vardecl )=> {
                match &vardecl.init {
                    Some(init) => {
                        self.compile_expr(init, chunk);

                        let localid = vardecl.id;

                        self.locals.insert(localid, LocalSlot(localid.0));

                        match vardecl.ty {
                            Type::Int => {
                                chunk.write(OpCode::I32StoreLocal as u8);
                                chunk.write_u24(
                                    localid.0.try_into().expect("varid exceeds u32::MAX"),
                                );
                            }
                            Type::Bool => {
                                chunk.write(OpCode::BStoreLocal as u8);
                                chunk
                                    .write_u24(localid.0.try_into().expect("varid exceeds u32::MAX"));
                            }
                            Type::Double => {
                                chunk.write(OpCode::F64StoreLocal as u8);
                                chunk
                                    .write_u24(localid.0.try_into().expect("varid exceeds u32::MAX"));
                            }
                            Type::Void => {
                                todo!()
                            }
                            Type::Error => {
                                unreachable!()
                            }
                        }
                    }
                    None => {
                        // use of a variable before its given a value is UB
                        let varid = vardecl.id;

                        self.locals.insert(varid, LocalSlot(varid.0));

                        chunk.write(OpCode::LoadConst as u8);
                        let idx = chunk.write_constant(Value::UNINITIALIZED);
                        chunk.write_u24(idx as u32);

                        match vardecl.ty {
                            Type::Int => {
                                chunk.write(OpCode::I32StoreLocal as u8);
                                chunk
                                    .write_u24(varid.0.try_into().expect("varid exceeds u32::MAX"));
                            }
                            Type::Bool => {
                                chunk.write(OpCode::BStoreLocal as u8);
                                chunk
                                    .write_u24(varid.0.try_into().expect("varid exceeds u32::MAX"));
                            }
                            Type::Double => {
                                todo!()
                            }
                            Type::Void => {
                                todo!()
                            }
                            Type::Error => {
                                unreachable!()
                            }
                        }
                    }
                }
            }
            StmtKind::Block(block) => {
                for stmt in &block.body {
                    self.compile_stmt(stmt, chunk);
                }
            }
            StmtKind::If(if_stmt) => {
                self.compile_expr(&if_stmt.condition, chunk);

                let pos = chunk.write_jump(OpCode::JmpFalse);

                self.compile_stmt(&if_stmt.body, chunk);

                if let Some(else_body) = if_stmt.else_clause {
                    let end_jump = chunk.write_jump(OpCode::Jmp);

                    chunk.patch_jump(pos);

                    self.compile_stmt(&else_body, chunk);

                    chunk.patch_jump(end_jump);
                } else {
                    chunk.patch_jump(pos);
                }
            }
            StmtKind::While(while_loop) => {
                let loop_start = chunk.bytecode.len();

                self.compile_expr(&while_loop.condition, chunk);

                let exit_jump = chunk.write_jump(OpCode::JmpFalse);

                self.loop_stack.push(LoopContext {
                    continue_jumps: Vec::new(),
                    break_jumps: Vec::new(),
                });

                self.compile_stmt(&while_loop.body, chunk);

                chunk.write_jump_back(OpCode::Jmp, loop_start);

                chunk.patch_jump(exit_jump);

                let loop_context = self.loop_stack.pop().unwrap();

                for jump in loop_context.continue_jumps {
                    chunk.patch_jump_to(jump, loop_start);
                }

                for jump in loop_context.break_jumps {
                    chunk.patch_jump(jump);
                }
            }
            StmtKind::For(for_loop) => {
                if let Some(init) = for_loop.init {
                    match init.kind {
                        ForInitKind::Decl(decl) => {
                            self.compile_stmt(&decl.into_stmt(init.id), chunk);
                        }
                        ForInitKind::Expr(expr) => {
                            self.compile_expr(&expr, chunk);
                        }
                    }
                }

                let loop_start = chunk.bytecode.len();
                let jump_out = if let Some(cond) = for_loop.condition {
                    self.compile_expr(&cond, chunk);

                    Some(chunk.write_jump(OpCode::JmpFalse))
                } else {
                    None
                };

                self.loop_stack.push(LoopContext {
                    continue_jumps: Vec::new(),
                    break_jumps: Vec::new(),
                });

                self.compile_stmt(&for_loop.body, chunk);

                let continue_jump_pos = chunk.bytecode.len();

                if let Some(incre) = &for_loop.increment {
                    self.compile_expr(incre, chunk);
                }

                chunk.write_jump_back(OpCode::Jmp, loop_start);

                if let Some(jump_out) = jump_out {
                    chunk.patch_jump(jump_out);
                }

                let loop_context = self.loop_stack.pop().unwrap();

                for jump in loop_context.break_jumps {
                    chunk.patch_jump(jump);
                }

                for jump in loop_context.continue_jumps {
                    chunk.patch_jump_to(jump, continue_jump_pos);
                }
            }
            StmtKind::Break => {
                let jump = chunk.write_jump(OpCode::Jmp);
                self.loop_stack.last_mut().unwrap().break_jumps.push(jump);
            }
            StmtKind::Continue => {
                let jump = chunk.write_jump(OpCode::Jmp);
                self.loop_stack
                    .last_mut()
                    .unwrap()
                    .continue_jumps
                    .push(jump);
            }
            StmtKind::Return(expr) => {
                todo!()
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr, chunk: &mut Chunk) {
        match &expr.kind {
            ExprKind::Literal(litkind) => {
                match litkind.kind {
                    LiteralKind::Int(i) => {
                        // we convert to i32 here because Int means i32
                        let value = Value::new_int(i as i32);
                        chunk.write(OpCode::LoadConst as u8);
                        let idx = chunk.write_constant(value);
                        chunk.write_u24(idx as u32);
                    }
                    LiteralKind::Bool(b) => {
                        let value = Value::new_bool(b);
                        chunk.write(OpCode::LoadConst as u8);
                        let idx = chunk.write_constant(value);
                        chunk.write_u24(idx as u32);
                    }
                    LiteralKind::Float(f) => {
                        let value = Value::new_double(f);
                        chunk.write(OpCode::LoadConst as u8);
                        let idx = chunk.write_constant(value);
                        chunk.write_u24(idx as u32);
                    }
                }
            }
            ExprKind::BinaryOp(binop) => {
                if !matches!(binop.kind, BinaryOpKind::And | BinaryOpKind::Or) {
                    self.compile_expr(&binop.left, chunk,);
                    self.compile_expr(&binop.right, chunk);
                }

                match binop.kind {
                    BinaryOpKind::Add => match metadata.converted_types.get(&expr.id).unwrap() {
                        Type::Int => {
                            chunk.write(OpCode::I32Add as u8);
                        }
                        Type::Double => {
                            chunk.write(OpCode::F64Add as u8);
                        }
                        Type::Void => {
                            todo!()
                        }
                        Type::Error | Type::Bool => {
                            unreachable!()
                        }
                    },
                    BinaryOpKind::Sub => match metadata.converted_types.get(&expr.id).unwrap() {
                        Type::Int => {
                            chunk.write(OpCode::I32Sub as u8);
                        }
                        Type::Double => {
                            chunk.write(OpCode::F64Sub as u8);
                        }
                        Type::Void => {
                            todo!()
                        }
                        Type::Error | Type::Bool => {
                            unreachable!()
                        }
                    },
                    BinaryOpKind::Mul => match metadata.converted_types.get(&expr.id).unwrap() {
                        Type::Int => {
                            chunk.write(OpCode::I32Mul as u8);
                        }
                        Type::Double => {
                            chunk.write(OpCode::F64Mul as u8);
                        }
                        Type::Void => {
                            todo!()
                        }
                        Type::Error | Type::Bool => {
                            unreachable!()
                        }
                    },
                    BinaryOpKind::Div => match metadata.converted_types.get(&expr.id).unwrap() {
                        Type::Int => {
                            chunk.write(OpCode::I32Div as u8);
                        }
                        Type::Double => {
                            chunk.write(OpCode::F64Div as u8);
                        }
                        Type::Void => {
                            todo!()
                        }
                        Type::Error | Type::Bool => {
                            unreachable!()
                        }
                    },
                    BinaryOpKind::BangEq => match metadata.converted_types.get(&left.id).unwrap() {
                        Type::Int => {
                            chunk.write(OpCode::I32NEqual as u8);
                        }
                        Type::Bool => {
                            chunk.write(OpCode::BNEqual as u8);
                        }
                        Type::Double => {
                            chunk.write(OpCode::F64NEqual as u8);
                        }
                        Type::Void => {
                            todo!()
                        }
                        Type::Error => unreachable!(),
                    },
                    BinaryOpKind::EqEq => match metadata.converted_types.get(&left.id).unwrap() {
                        Type::Int => {
                            chunk.write(OpCode::I32Equal as u8);
                        }
                        Type::Bool => {
                            chunk.write(OpCode::BEqual as u8);
                        }
                        Type::Double => {
                            chunk.write(OpCode::F64Equal as u8);
                        }
                        Type::Void => {
                            todo!()
                        }
                        Type::Error => unreachable!(),
                    },
                    BinaryOpKind::GreaterEq => match metadata.converted_types.get(&left.id).unwrap() {
                        Type::Int => {
                            chunk.write(OpCode::I32GreaterEq as u8);
                        }
                        Type::Double => {
                            chunk.write(OpCode::F64GreaterEq as u8);
                        }
                        Type::Void => {
                            todo!()
                        }
                        Type::Bool | Type::Error => unreachable!(),
                    },
                    BinaryOpKind::GreaterThan => match metadata.converted_types.get(&left.id).unwrap()
                    {
                        Type::Int => {
                            chunk.write(OpCode::I32Greater as u8);
                        }
                        Type::Double => {
                            chunk.write(OpCode::F64Greater as u8);
                        }
                        Type::Void => {
                            todo!()
                        }
                        Type::Bool | Type::Error => unreachable!(),
                    },
                    BinaryOpKind::LessEq => match metadata.converted_types.get(&left.id).unwrap() {
                        Type::Int => {
                            chunk.write(OpCode::I32LessEq as u8);
                        }
                        Type::Double => {
                            chunk.write(OpCode::F64LessEq as u8);
                        }
                        Type::Void => {
                            todo!()
                        }
                        Type::Bool | Type::Error => unreachable!(),
                    },
                    BinaryOpKind::LessThan => match metadata.converted_types.get(&left.id).unwrap() {
                        Type::Int => {
                            chunk.write(OpCode::I32Less as u8);
                        }
                        Type::Double => {
                            chunk.write(OpCode::F64Less as u8);
                        }
                        Type::Void => {
                            todo!()
                        }
                        Type::Bool | Type::Error => unreachable!(),
                    },
                    BinaryOpKind::And => {
                        self.compile_expr(&binop.left, chunk);
                        chunk.write(OpCode::Dup as u8);
                        let pos = chunk.write_jump(OpCode::JmpFalse);
                        chunk.write(OpCode::Pop as u8);
                        self.compile_expr(&binop.right, chunk);
                        chunk.patch_jump(pos);
                    }
                    BinaryOpKind::Or => {
                        self.compile_expr(&binop.left, chunk);

                        chunk.write(OpCode::Dup as u8);

                        let pos = chunk.write_jump(OpCode::JmpTrue);

                        chunk.write(OpCode::Pop as u8);

                        self.compile_expr(&binop.right, chunk);

                        chunk.patch_jump(pos);
                    }
                };
            }
            ExprKind::UnaryOp(unary) => {
                self.compile_expr(&unary.operand, chunk);

                match unary.kind {
                    UnaryOpKind::Negate => match metadata.converted_types.get(&right.id).unwrap() {
                        Type::Int => {
                            chunk.write(OpCode::I32Negate as u8);
                        }
                        Type::Double => {
                            chunk.write(OpCode::F64Negate as u8);
                        }
                        Type::Void => {
                            todo!()
                        }
                        Type::Error | Type::Bool => {
                            unreachable!()
                        }
                    },
                    UnaryOpKind::PostDecrement => {
                        match metadata.converted_types.get(&right.id).unwrap() {
                            Type::Int => {
                                chunk.write(OpCode::Dup as u8);
                                chunk.write(OpCode::LoadConst as u8);
                                let idx = chunk.write_constant(Value::new_int(1));

                                chunk.write_u24(idx as u32);

                                chunk.write(OpCode::I32Sub as u8);

                                chunk.write(OpCode::I32StoreLocal as u8);
                                let varid = metadata.variables.get(&right.id).unwrap();
                                let slot = self.locals.get(varid).unwrap();
                                chunk.write_u24(slot.0 as u32);
                            }
                            Type::Double => {
                                chunk.write(OpCode::Dup as u8);
                                chunk.write(OpCode::LoadConst as u8);
                                let idx = chunk.write_constant(Value::new_double(1.0));

                                chunk.write_u24(idx as u32);

                                chunk.write(OpCode::F64Sub as u8);

                                chunk.write(OpCode::F64StoreLocal as u8);
                                let varid = metadata.variables.get(&right.id).unwrap();
                                let slot = self.locals.get(varid).unwrap();
                                chunk.write_u24(slot.0 as u32);
                            }
                            Type::Void => {
                                todo!()
                            }
                            Type::Bool | Type::Error => {
                                unreachable!()
                            }
                        }
                    }
                    UnaryOpKind::PostIncrement => {
                        match metadata.converted_types.get(&right.id).unwrap() {
                            Type::Int => {
                                chunk.write(OpCode::Dup as u8);
                                chunk.write(OpCode::LoadConst as u8);
                                let idx = chunk.write_constant(Value::new_int(1));
                                chunk.write_u24(idx as u32);

                                chunk.write(OpCode::I32Add as u8);

                                chunk.write(OpCode::I32StoreLocal as u8);
                                let varid = metadata.variables.get(&right.id).unwrap();
                                let slot = self.locals.get(varid).unwrap();
                                chunk.write_u24(slot.0 as u32);
                            }
                            Type::Double => {
                                chunk.write(OpCode::Dup as u8);
                                chunk.write(OpCode::LoadConst as u8);
                                let idx = chunk.write_constant(Value::new_double(1.0));
                                chunk.write_u24(idx as u32);

                                chunk.write(OpCode::F64Add as u8);

                                chunk.write(OpCode::F64StoreLocal as u8);
                                let varid = metadata.variables.get(&right.id).unwrap();
                                let slot = self.locals.get(varid).unwrap();
                                chunk.write_u24(slot.0 as u32);
                            }
                            Type::Void => {
                                todo!()
                            }
                            Type::Bool | Type::Error => {
                                unreachable!()
                            }
                        }
                    }
                    UnaryOpKind::PreDecrement => {
                        match metadata.converted_types.get(&right.id).unwrap() {
                            Type::Int => {
                                chunk.write(OpCode::LoadConst as u8);
                                let idx = chunk.write_constant(Value::new_int(1));
                                chunk.write_u24(idx as u32);

                                chunk.write(OpCode::I32Sub as u8);
                                chunk.write(OpCode::Dup as u8);
                                chunk.write(OpCode::I32StoreLocal as u8);
                                let varid = metadata.variables.get(&right.id).unwrap();
                                let slot = self.locals.get(varid).unwrap();
                                chunk.write_u24(slot.0 as u32);
                            }
                            Type::Double => {
                                chunk.write(OpCode::LoadConst as u8);
                                let idx = chunk.write_constant(Value::new_double(1.0));
                                chunk.write_u24(idx as u32);

                                chunk.write(OpCode::F64Sub as u8);
                                chunk.write(OpCode::Dup as u8);
                                chunk.write(OpCode::F64StoreLocal as u8);
                                let varid = metadata.variables.get(&right.id).unwrap();
                                let slot = self.locals.get(varid).unwrap();
                                chunk.write_u24(slot.0 as u32);
                            }
                            Type::Void => {
                                todo!()
                            }
                            Type::Bool | Type::Error => {
                                unreachable!()
                            }
                        }
                    }
                    UnaryOpKind::PreIncrement => {
                        match metadata.converted_types.get(&right.id).unwrap() {
                            Type::Int => {
                                chunk.write(OpCode::LoadConst as u8);
                                let idx = chunk.write_constant(Value::new_int(1));
                                chunk.write_u24(idx as u32);
                                chunk.write(OpCode::I32Add as u8);
                                chunk.write(OpCode::Dup as u8);
                                chunk.write(OpCode::I32StoreLocal as u8);
                                let varid = metadata.variables.get(&right.id).unwrap();
                                let slot = self.locals.get(varid).unwrap();
                                chunk.write_u24(slot.0 as u32);
                            }
                            Type::Double => {
                                chunk.write(OpCode::LoadConst as u8);
                                let idx = chunk.write_constant(Value::new_double(1.0));
                                chunk.write_u24(idx as u32);
                                chunk.write(OpCode::F64Add as u8);
                                chunk.write(OpCode::Dup as u8);
                                chunk.write(OpCode::F64StoreLocal as u8);
                                let varid = metadata.variables.get(&right.id).unwrap();
                                let slot = self.locals.get(varid).unwrap();
                                chunk.write_u24(slot.0 as u32);
                            }
                            Type::Void => {
                                todo!()
                            }
                            Type::Bool | Type::Error => {
                                unreachable!()
                            }
                        }
                    }
                };
            }
            ExprKind::VarAssign(target, value) => {
                self.compile_expr(value, chunk, metadata);

                let varid = match target.kind {
                    ExprKind::Variable(..) => metadata.variables.get(&target.id).unwrap(),
                    _ => unreachable!("non lvalue?"),
                };

                chunk.write(OpCode::Dup as u8);

                match metadata.var_types.get(varid).unwrap() {
                    Type::Int => {
                        chunk.write(OpCode::I32StoreLocal as u8);
                        chunk.write_u24((*self.locals.get(varid).unwrap()).into());
                    }
                    Type::Bool => {
                        chunk.write(OpCode::BStoreLocal as u8);
                        chunk.write_u24((*self.locals.get(varid).unwrap()).into());
                    }
                    Type::Double => {
                        chunk.write(OpCode::F64StoreLocal as u8);
                        chunk.write_u24((*self.locals.get(varid).unwrap()).into());
                    }
                    Type::Void => {
                        todo!()
                    }
                    Type::Error => unreachable!(),
                }
            }
            ExprKind::Variable(_name) => {
                let varid = *metadata.variables.get(&expr.id).unwrap();
                match metadata.var_types.get(&varid).unwrap() {
                    Type::Int => {
                        chunk.write(OpCode::I32LoadLocal as u8);
                    }
                    Type::Bool => {
                        chunk.write(OpCode::BLoadLocal as u8);
                    }
                    Type::Double => {
                        chunk.write(OpCode::F64LoadLocal as u8);
                    }
                    Type::Void => {
                        todo!()
                    }
                    Type::Error => {
                        unreachable!()
                    }
                }
                chunk.write_u24(
                    (*self
                        .locals
                        .get(metadata.variables.get(&expr.id).unwrap())
                        .unwrap())
                    .try_into()
                    .expect("overflow"),
                );
            }
            ExprKind::Call(call) => {
                todo!()
            }
        }

        if let Some(conversion) = metadata.conversions.get(&expr.id) {
            match conversion {
                Conversion::IntToDouble => {
                    chunk.write(OpCode::I32ToF64 as u8);
                }
            }
        }
    }
}
