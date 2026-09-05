use macros::Constructor;

use crate::analysis::ids::{FunctionId, GlobalVarId, HirId, LocalVarId, VariableId};
use crate::analysis::types::Type;

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Item {
    Function(Function),
    GlobalVarDecl(GlobalVarDecl),
}

#[derive(Debug, PartialEq, Clone)]
pub struct GlobalVarDecl {
    pub id: HirId,
    pub var_id: GlobalVarId,
    pub type_annotation: Type,
    pub init: Option<Expr>,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct Function {
    pub id: HirId,
    pub fn_id: FunctionId,
    pub return_type: Type,
    pub params: Vec<Param>,
    pub body: Block,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct Param {
    pub id: LocalVarId,
    pub ty: Type,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Block {
    pub body: Vec<Stmt>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Print {
    pub expr: Expr,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExprStmt {
    pub expr: Expr,
}

#[derive(Debug, PartialEq, Clone)]
pub struct VarDecl {
    pub id: LocalVarId,
    pub ty: Type,
    // use of a variable without an init expression is undefined behaviour
    pub init: Option<Expr>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct If {
    pub condition: Expr,
    pub body: Box<Stmt>,
    pub else_clause: Option<Box<Stmt>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct While {
    pub condition: Expr,
    pub body: Box<Stmt>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct For {
    pub init: Option<ForInit>,
    pub condition: Option<Expr>,
    pub increment: Option<Expr>,
    pub body: Box<Stmt>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ForInit {
    Expr(Expr),
    Decl(VarDecl),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Return {
    pub value: Option<Expr>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Stmt {
    kind: StmtKind,
    id: HirId,
}

#[derive(Debug, PartialEq, Clone)]
pub enum StmtKind {
    Block(Block),
    Print(Print),
    ExprStmt(ExprStmt),
    VarDecl(VarDecl),
    If(If),
    While(While),
    Break,
    Continue,
    For(For),
    Return(Return),
}

#[derive(Debug, PartialEq, Clone)]
pub enum LiteralKind {
    Int(i32),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct Literal {
    pub kind: LiteralKind,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct BinaryOp {
    pub kind: BinaryOpKind,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BinaryOpKind {
    Add,
    Sub,
    Mul,
    Div,

    EqEq,
    BangEq,

    LessThan,
    LessEq,
    GreaterThan,
    GreaterEq,

    And,
    Or,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct UnaryOp {
    pub kind: UnaryOpKind,
    pub operand: Box<Expr>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum UnaryOpKind {
    Negate,

    PreIncrement,
    PostIncrement,

    PreDecrement,
    PostDecrement,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct Variable {
    id: VariableId,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct VarAssign {
    pub target: Box<Expr>,
    pub value: Box<Expr>,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct Call {
    pub callee: Box<Expr>,
    pub args: Vec<Expr>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExprKind {
    Literal(Literal),
    BinaryOp(BinaryOp),
    UnaryOp(UnaryOp),
    Variable(Variable),
    VarAssign(VarAssign),
    Call(Call),
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct Expr {
    pub kind: ExprKind,
    pub id: HirId,
    pub ty: Type,
}
