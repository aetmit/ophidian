use crate::analysis::types::Type;
use crate::lex::token::{Token, TokenKind};
use crate::span::{Span, Spanned};
use macros::Constructor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub usize);

impl NodeId {
    pub const ERROR: Self = Self(usize::MAX);

    pub fn increment(&mut self) {
        *self += 1
    }
}

impl std::ops::AddAssign<usize> for NodeId {
    fn add_assign(&mut self, rhs: usize) {
        if self.0 + rhs > usize::MAX {
            self.0 = usize::MAX;
        } else {
            self.0 += rhs;
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum LitKind {
    Int(u128),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,

    EqEq,
    BangEq,
    LessThan,
    GreaterThan,
    LessEq,
    GreaterEq,

    And,
    Or,
}

impl std::fmt::Display for BinOpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self {
            BinOpKind::Add => "+",
            BinOpKind::Sub => "-",
            BinOpKind::Mul => "*",
            BinOpKind::Div => "/",

            BinOpKind::EqEq => "==",
            BinOpKind::BangEq => "!=",
            BinOpKind::LessThan => "<",
            BinOpKind::GreaterThan => ">",
            BinOpKind::LessEq => "<=",
            BinOpKind::GreaterEq => ">=",

            BinOpKind::And => "&&",
            BinOpKind::Or => "||",
        };

        write!(f, "{}", op)
    }
}

impl From<Token> for BinOpKind {
    fn from(value: Token) -> Self {
        match value.kind {
            TokenKind::Plus => Self::Add,
            TokenKind::Minus => Self::Sub,
            TokenKind::Star => Self::Mul,
            TokenKind::Slash => Self::Div,
            TokenKind::EqualEqual => Self::EqEq,
            TokenKind::BangEqual => Self::BangEq,
            TokenKind::LessThan => Self::LessThan,
            TokenKind::GreaterThan => Self::GreaterThan,
            TokenKind::LessEq => Self::LessEq,
            TokenKind::GreaterEq => Self::GreaterEq,
            TokenKind::And => Self::And,
            TokenKind::Or => Self::Or,
            _ => panic!("not a binary operation"),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum UnaryOpKind {
    Negate,
    // ++i
    PreIncrement,
    // i++
    PostIncrement,
    // --i
    PreDecrement,
    // i--
    PostDecrement,
}

impl std::fmt::Display for UnaryOpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self {
            Self::Negate => "!",
            Self::PostDecrement => "--",
            Self::PostIncrement => "++",
            Self::PreDecrement => "--",
            Self::PreIncrement => "++",
        };

        write!(f, "{}", op)
    }
}

pub type BinOp = Spanned<BinOpKind>;

pub type UnaryOp = Spanned<UnaryOpKind>;

// all the different exprs in the language
#[derive(Debug, PartialEq, Clone)]
pub enum ExprKind {
    // a literal
    // e.g. '1'
    Literal(LitKind),

    // a binary operation like `1 + 2`
    BinaryOp(BinOp, Box<Expr>, Box<Expr>),

    // a unary operation like  `-1`
    UnaryOp(UnaryOp, Box<Expr>),

    // usage of a variable in an expression
    // e.g. print(x);
    //            ^
    // Vec<u8> being the name of the identifier
    Variable(Vec<u8>),

    // a variable assignment
    // e.g. x = 5;
    // the expression evaluates to the value that was assigned
    // the first box being the target and the second one being the value
    // that was assigned
    VarAssign(Box<Expr>, Box<Expr>),

    // a function call
    // Box<Expr> being the thing being called
    // Vec<Expr> being the arguments
    Call(Box<Expr>, Vec<Expr>),

    // this error node represents a recoverable error. this
    // node exists so that when the parser encounters an error
    // e.g. an unexpected token, it can create an error node, return it
    // from the parsing function and continue parsing. However, before
    // semantic analysis begins, if the parser has accumulated any error
    // nodes, then complilation wont continue because theres simply no point
    // in analysing and compiling a half broken ast. we will simply print the
    // parser diagnostics and exit
    Error,
}

// an expression
#[derive(Debug, PartialEq, Clone)]
pub struct Expr {
    pub id: NodeId,
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub const fn new(id: NodeId, kind: ExprKind, span: Span) -> Self {
        Self { id, kind, span }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ForInit {
    Expr(Expr),
    Decl(Spanned<VarDecl>),
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct Print {
    pub expr: Box<Expr>,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct ExprStmt {
    pub expr: Box<Expr>,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct VarDecl {
    pub name: Vec<u8>,
    pub type_annotation: Option<Type>,
    pub init: Option<Expr>,
}

impl TryFrom<Stmt> for VarDecl {
    type Error = ();

    fn try_from(value: Stmt) -> Result<Self, Self::Error> {
        match value.kind {
            StmtKind::VarDecl(decl) => Ok(decl),
            _ => Err(()),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct If {
    pub condition: Box<Expr>,
    pub body: Box<Stmt>,
    pub else_body: Option<Box<Stmt>>,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct While {
    pub condition: Box<Expr>,
    pub body: Box<Stmt>,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct For {
    pub init: Option<Box<ForInit>>,
    pub condition: Option<Expr>,
    pub increment: Option<Expr>,
    pub body: Box<Stmt>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Return {
    pub expr: Option<Expr>,
}

#[derive(Debug, PartialEq, Clone, Constructor)]
pub struct Block {
    pub body: Vec<Stmt>,
}

impl TryFrom<Stmt> for Block {
    type Error = ();

    fn try_from(value: Stmt) -> Result<Self, Self::Error> {
        match value.kind {
            StmtKind::Block(b) => Ok(b),
            _ => Err(()),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum StmtKind {
    // we have a built in print statement that is still called like a
    // function i.e. `print(expression);` requiring the parentheses
    Print(Print),

    // expression statement
    ExprStmt(ExprStmt),

    // a variable declaration
    //
    // Vec<u8> is the name of the identifier being declared
    // Option<Type> is optional type annotation
    // Option<Expr> is an optional init expression to initialise the variable
    VarDecl(VarDecl),

    // a block (scope)
    // opened with a '{' and closed with '}'
    Block(Block),

    // an if statement
    // Box<Expr> represents the condition for the if body to execute
    // Box<Stmt> represents the if body
    // i.e.
    // if (cond) {
    //              <--
    // }
    // Option<Box<Stmt>> represents an optional else body
    // for else if statements simply store another if statement in the else clause
    // so it is recursive
    If(If),

    // a while loop
    // Box<Expr> represents the condition for the while loop
    // Box<Stmt> is the body of the loop
    While(While),

    // a break statement
    Break,

    // a continue statement
    Continue,

    // a for loop
    // Option<Box<Stmt>> is the intializer statement
    // Option<Expr> is the condition for the loop to continue
    // Option<Expr> is the increment expression that happens after each interation
    // Box<Stmt> is the body of the loop
    // e.g.
    //       |          |       |
    //       |          |       |
    //       v          v       v
    //   initializer   cond     increment
    // for (let i = 0; i < 10; i++) {
    //                            <-- body
    // }
    For(For),

    // a return statement
    // Option<Expr> represnts an optional return value
    Return(Return),

    // an error node to represent a recoverable error this node exists
    // so that the parser can recover from a parsing function. Before the
    // semantic analysis phase begins, if the parser has any error nodes
    // then the driver will simply print the diagnostics and exit because
    // there is no point in analysing an ast that is half formed.
    Error,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Stmt {
    pub id: NodeId,
    pub kind: StmtKind,
    pub span: Span,
}

impl Stmt {
    pub const fn new(id: NodeId, kind: StmtKind, span: Span) -> Self {
        Self { id, kind, span }
    }
}

#[derive(Debug, PartialEq, Constructor)]
pub struct Function {
    pub id: NodeId,
    pub span: Span,
    pub name: Vec<u8>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Spanned<Block>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Param {
    pub span: Span,
    pub id: NodeId,
    pub name: Vec<u8>,
    pub ty: Type,
}

impl Param {
    pub fn new(id: NodeId, name: Vec<u8>, ty: Type, span: Span) -> Self {
        Self { span, id, name, ty }
    }
}

#[derive(Debug, PartialEq, Constructor)]
pub struct GlobalVarDecl {
    pub id: NodeId,
    pub span: Span,
    pub name: Vec<u8>,
    pub type_annotation: Type,
    pub init: Option<Expr>,
}

#[derive(Debug, PartialEq)]
pub enum Item {
    GlobalVarDecl(GlobalVarDecl),
    Function(Function),
}

#[derive(Debug, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}

impl Program {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
}
