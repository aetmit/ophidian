use cli::options::Options;
use frontend::analysis::SemanticAnalyzer;
use frontend::analysis::analyzer::AnalysisResult;
use frontend::analysis::resolution::VarId;
use frontend::analysis::types::{Conversion, Type};
use frontend::diagnostics::Diagnostic;
use frontend::lex::Lexer;
use frontend::parse::Parser;
use frontend::parse::ast::{
    BinOpKind, Expr, ExprKind, ForInit, LitKind, Stmt, StmtKind, UnaryOpKind,
};
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
    locals: HashMap<VarId, LocalSlot>,
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
        let metadata = analyzer.analyze(program);

        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }

        let metadata = metadata.unwrap();

        let mut chunk = Chunk::new();

        for stmt in &program.decls {
            self.compile_stmt(stmt, &mut chunk, &metadata);
        }

        chunk.write(OpCode::LoadConst as u8);
        let idx = chunk.write_constant(Value::new_int(0));
        chunk.write_u24(idx as u32);

        chunk.write(OpCode::Halt as u8);

        if options.dump_bytecode {
            let disassembler = Disassembler::new(&chunk);
            disassembler.disassemble();
        }

        Ok(chunk)
    }

    fn compile_stmt(&mut self, stmt: &Stmt, chunk: &mut Chunk, metadata: &AnalysisResult) {
        match &stmt.kind {
            StmtKind::ExprStmt(expr) => {
                self.compile_expr(expr, chunk, metadata);

                chunk.write(OpCode::Pop as u8);
            }
            StmtKind::Print(expr) => {
                self.compile_expr(expr, chunk, metadata);

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
            StmtKind::VarDecl(_name, _type_annotation, initialiser) => {
                match initialiser {
                    Some(init) => {
                        self.compile_expr(init, chunk, metadata);

                        let varid = *metadata.variables.get(&stmt.id).unwrap();

                        self.locals.insert(varid, LocalSlot(varid.0));

                        match metadata.var_types.get(&varid).unwrap() {
                            Type::Int => {
                                chunk.write(OpCode::I32StoreLocal as u8);
                                chunk.write_u24(
                                    varid.0.try_into().expect("hopefully this doesnt happen"),
                                );
                            }
                            Type::Bool => {
                                chunk.write(OpCode::BStoreLocal as u8);
                                chunk
                                    .write_u24(varid.0.try_into().expect("varid exceeds u32::MAX"));
                            }
                            Type::Double => {
                                chunk.write(OpCode::F64StoreLocal as u8);
                                chunk
                                    .write_u24(varid.0.try_into().expect("varid exceeds u32::MAX"));
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
                        let varid = *metadata.variables.get(&stmt.id).unwrap();

                        self.locals.insert(varid, LocalSlot(varid.0));

                        chunk.write(OpCode::LoadConst as u8);
                        let idx = chunk.write_constant(Value::UNINITIALIZED);
                        chunk.write_u24(idx as u32);

                        match metadata.var_types.get(&varid).unwrap() {
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
            StmtKind::Block(body) => {
                for stmt in body {
                    self.compile_stmt(stmt, chunk, metadata);
                }
            }
            StmtKind::If(cond, body, else_body) => {
                self.compile_expr(cond, chunk, metadata);

                let pos = chunk.write_jump(OpCode::JmpFalse);

                self.compile_stmt(body, chunk, metadata);

                if let Some(else_body) = else_body {
                    let end_jump = chunk.write_jump(OpCode::Jmp);

                    chunk.patch_jump(pos);

                    self.compile_stmt(else_body, chunk, metadata);

                    chunk.patch_jump(end_jump);
                } else {
                    chunk.patch_jump(pos);
                }
            }
            StmtKind::While(cond, body) => {
                let loop_start = chunk.bytecode.len();

                self.compile_expr(cond, chunk, metadata);

                let exit_jump = chunk.write_jump(OpCode::JmpFalse);

                self.loop_stack.push(LoopContext {
                    continue_jumps: Vec::new(),
                    break_jumps: Vec::new(),
                });

                self.compile_stmt(body, chunk, metadata);

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
            StmtKind::For(init, cond, incre, body) => {
                if let Some(init) = init {
                    match &**init {
                        ForInit::Decl(decl) => {
                            self.compile_stmt(decl, chunk, metadata);
                        }
                        ForInit::Expr(expr) => {
                            self.compile_expr(expr, chunk, metadata);
                        }
                    }
                }

                let loop_start = chunk.bytecode.len();
                let jump_out = if let Some(cond) = cond {
                    self.compile_expr(cond, chunk, metadata);

                    Some(chunk.write_jump(OpCode::JmpFalse))
                } else {
                    None
                };

                self.loop_stack.push(LoopContext {
                    continue_jumps: Vec::new(),
                    break_jumps: Vec::new(),
                });

                self.compile_stmt(body, chunk, metadata);

                let continue_jump_pos = chunk.bytecode.len();

                if let Some(incre) = incre {
                    self.compile_expr(incre, chunk, metadata);
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
            StmtKind::Error => {
                unreachable!()
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr, chunk: &mut Chunk, metadata: &AnalysisResult) {
        match &expr.kind {
            ExprKind::Literal(litkind) => {
                match litkind {
                    LitKind::Int(i) => {
                        // we convert to i32 here because Int means i32
                        let value = Value::new_int(*i as i32);
                        chunk.write(OpCode::LoadConst as u8);
                        let idx = chunk.write_constant(value);
                        chunk.write_u24(idx as u32);
                    }
                    LitKind::Bool(b) => {
                        let value = Value::new_bool(*b);
                        chunk.write(OpCode::LoadConst as u8);
                        let idx = chunk.write_constant(value);
                        chunk.write_u24(idx as u32);
                    }
                    LitKind::Float(f) => {
                        let value = Value::new_double(*f);
                        chunk.write(OpCode::LoadConst as u8);
                        let idx = chunk.write_constant(value);
                        chunk.write_u24(idx as u32);
                    }
                }
            }
            ExprKind::BinaryOp(op, left, right) => {
                if !matches!(op.node, BinOpKind::And | BinOpKind::Or) {
                    self.compile_expr(&left, chunk, metadata);
                    self.compile_expr(&right, chunk, metadata);
                }

                match op.node {
                    // only type int exists rn so we dont needa
                    // check for different types
                    BinOpKind::Add => match metadata.converted_types.get(&expr.id).unwrap() {
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
                    BinOpKind::Sub => match metadata.converted_types.get(&expr.id).unwrap() {
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
                    BinOpKind::Mul => match metadata.converted_types.get(&expr.id).unwrap() {
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
                    BinOpKind::Div => match metadata.converted_types.get(&expr.id).unwrap() {
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
                    BinOpKind::BangEq => match metadata.converted_types.get(&left.id).unwrap() {
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
                    BinOpKind::EqEq => match metadata.converted_types.get(&left.id).unwrap() {
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
                    BinOpKind::GreaterEq => match metadata.converted_types.get(&left.id).unwrap() {
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
                    BinOpKind::GreaterThan => match metadata.converted_types.get(&left.id).unwrap()
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
                    BinOpKind::LessEq => match metadata.converted_types.get(&left.id).unwrap() {
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
                    BinOpKind::LessThan => match metadata.converted_types.get(&left.id).unwrap() {
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
                    BinOpKind::And => {
                        self.compile_expr(left, chunk, metadata);
                        chunk.write(OpCode::Dup as u8);
                        let pos = chunk.write_jump(OpCode::JmpFalse);
                        chunk.write(OpCode::Pop as u8);
                        self.compile_expr(right, chunk, metadata);
                        chunk.patch_jump(pos);
                    }
                    BinOpKind::Or => {
                        self.compile_expr(left, chunk, metadata);

                        chunk.write(OpCode::Dup as u8);

                        let pos = chunk.write_jump(OpCode::JmpTrue);

                        chunk.write(OpCode::Pop as u8);

                        self.compile_expr(right, chunk, metadata);

                        chunk.patch_jump(pos);
                    }
                };
            }
            ExprKind::UnaryOp(op, right) => {
                self.compile_expr(&right, chunk, metadata);

                match op.node {
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
            ExprKind::Error => {
                unreachable!()
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
            ExprKind::Call(callee, args) => {
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
