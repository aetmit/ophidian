use macros::{Constructor, Hir};

use crate::analysis::ids::{FunctionId, GlobalVarId, HirId, LocalVarId, VariableId};
use crate::analysis::types::Type;

// marker trait for hir
pub trait Hir {}

#[derive(Debug, PartialEq, Clone, Constructor, Hir)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, PartialEq, Clone, Hir)]
pub enum Item {
    Function(Function),
    GlobalVarDecl(GlobalVarDecl),
}

#[derive(Debug, PartialEq, Clone, Hir)]
pub struct GlobalVarDecl {
    pub id: HirId,
    pub var_id: GlobalVarId,
    pub type_annotation: Type,
    pub init: Option<Expr>,
}

#[derive(Debug, PartialEq, Clone, Constructor, Hir)]
pub struct Function {
    pub id: HirId,
    pub fn_id: FunctionId,
    pub return_type: Type,
    pub params: Vec<Param>,
    pub body: Block,
}

#[derive(Debug, PartialEq, Clone, Constructor, Hir)]
pub struct Param {
    pub id: LocalVarId,
    pub ty: Type,
}

#[derive(Debug, PartialEq, Clone, Hir, Constructor)]
pub struct Block {
    pub body: Vec<Stmt>,
}

#[derive(Debug, PartialEq, Clone, Hir, Constructor)]
pub struct Print {
    pub expr: Expr,
}

#[derive(Debug, PartialEq, Clone, Hir, Constructor)]
pub struct ExprStmt {
    pub expr: Expr,
}

#[derive(Debug, PartialEq, Clone, Hir, Constructor)]
pub struct VarDecl {
    pub id: LocalVarId,
    pub ty: Type,
    // use of a variable without an init expression is undefined behaviour
    pub init: Option<Expr>,
}

#[derive(Debug, PartialEq, Clone, Hir, Constructor)]
pub struct If {
    pub condition: Expr,
    pub body: Box<Stmt>,
    pub else_clause: Option<Box<Stmt>>,
}

#[derive(Debug, PartialEq, Clone, Hir, Constructor)]
pub struct While {
    pub condition: Expr,
    pub body: Box<Stmt>,
}

#[derive(Debug, PartialEq, Clone, Hir, Constructor)]
pub struct For {
    pub init: Option<ForInit>,
    pub condition: Option<Expr>,
    pub increment: Option<Expr>,
    pub body: Box<Stmt>,
}

#[derive(Debug, PartialEq, Clone, Hir)]
pub enum ForInitKind {
    Expr(Expr),
    Decl(VarDecl),
}

#[derive(Debug, PartialEq, Clone, Hir, Constructor)]
pub struct ForInit {
    kind: ForInitKind,
    id: HirId,
}

#[derive(Debug, PartialEq, Clone, Hir, Constructor)]
pub struct Return {
    pub value: Option<Expr>,
}

#[derive(Debug, PartialEq, Clone, Hir, Constructor)]
pub struct Stmt {
    kind: StmtKind,
    id: HirId,
}

#[derive(Debug, PartialEq, Clone, Hir)]
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

#[derive(Debug, PartialEq, Clone, Hir)]
pub enum LiteralKind {
    Int(i32),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, PartialEq, Clone, Constructor, Hir)]
pub struct Literal {
    pub kind: LiteralKind,
}

#[derive(Debug, PartialEq, Clone, Constructor, Hir)]
pub struct BinaryOp {
    pub kind: BinaryOpKind,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
}

#[derive(Debug, PartialEq, Clone, Hir)]
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

#[derive(Debug, PartialEq, Clone, Constructor, Hir)]
pub struct UnaryOp {
    pub kind: UnaryOpKind,
    pub operand: Box<Expr>,
}

#[derive(Debug, PartialEq, Clone, Hir)]
pub enum UnaryOpKind {
    Negate,

    PreIncrement,
    PostIncrement,

    PreDecrement,
    PostDecrement,
}

#[derive(Debug, PartialEq, Clone, Constructor, Hir)]
pub struct Variable {
    id: VariableId,
}

#[derive(Debug, PartialEq, Clone, Constructor, Hir)]
pub struct VarAssign {
    pub target: Box<Expr>,
    pub value: Box<Expr>,
}

#[derive(Debug, PartialEq, Clone, Constructor, Hir)]
pub struct Call {
    pub callee: Box<Expr>,
    pub function: FunctionId,
    pub args: Vec<Expr>,
}

#[derive(Debug, PartialEq, Clone, Hir)]
pub enum ConversionKind {
    IntToDouble,
}

#[derive(Debug, PartialEq, Clone, Constructor, Hir)]
pub struct Conversion {
    kind: ConversionKind,
    operand: Box<Expr>,
}

#[derive(Debug, PartialEq, Clone, Hir)]
pub enum ExprKind {
    Literal(Literal),
    BinaryOp(BinaryOp),
    UnaryOp(UnaryOp),
    Variable(Variable),
    VarAssign(VarAssign),
    Call(Call),
    Conversion(Conversion),
}

#[derive(Debug, PartialEq, Clone, Constructor, Hir)]
pub struct Expr {
    pub kind: ExprKind,
    pub id: HirId,
    pub ty: Type,
}
