//! Language-neutral intermediate representation for algosketch.
//!
//! The IR is intentionally smaller than a full AST: we keep control flow
//! and data flow, drop language-specific syntax noise (modifiers,
//! decorators, namespace details), and preserve unparsed fragments as
//! `Raw` so renderers can still produce useful output.

use crate::SourceLang;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub source_language: SourceLang,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Function(Function),
    Class(Class),
    Import(Import),
    GlobalVar(VarDecl),
    Raw(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeHint>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub name: String,
    pub parents: Vec<String>,
    pub fields: Vec<VarDecl>,
    pub methods: Vec<Function>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub type_hint: Option<TypeHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeHint(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarDecl {
    pub name: String,
    pub type_hint: Option<TypeHint>,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Block(pub Vec<Stmt>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Assign {
        target: Expr,
        value: Expr,
    },
    VarDecl(VarDecl),
    If {
        cond: Expr,
        then_block: Block,
        else_block: Option<Block>,
    },
    While {
        cond: Expr,
        body: Block,
    },
    For {
        kind: ForKind,
        body: Block,
    },
    Return(Option<Expr>),
    Break,
    Continue,
    ExprStmt(Expr),
    Raw(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForKind {
    CStyle {
        init: Box<Stmt>,
        cond: Expr,
        step: Expr,
    },
    ForEach {
        var: String,
        iter: Expr,
    },
    Range {
        var: String,
        start: Expr,
        end: Expr,
        step: Option<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Literal(Literal),
    Ident(String),
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Index {
        obj: Box<Expr>,
        index: Box<Expr>,
    },
    Field {
        obj: Box<Expr>,
        name: String,
    },
    Tuple(Vec<Expr>),
    Raw(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Int(i64),
    Float(String),
    Str(String),
    Bool(bool),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
}
