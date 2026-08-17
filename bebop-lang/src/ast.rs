//! Abstract syntax tree for Bebop (spec §3, Appendix A).
//!
//! v0.1 scope: enough to parse and pretty-print (round-trip) the core grammar —
//! `module`, `fn` (with params, return, contract clauses, block body), `struct`,
//! `data`, `quotient`, `contract`, `use`, `type`, and the expression language.

use crate::token::Span;

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

impl Ident {
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Ident { name: name.into(), span }
    }
}

#[derive(Debug, Clone)]
pub enum Item {
    Module(ModuleDecl),
    Fn(FnItem),
    Struct(StructItem),
    Data(DataItem),
    Quotient(QuotientItem),
    Contract(ContractItem),
    Use(UseItem),
    TypeAlias(TypeAliasItem),
    Const(ConstItem),
}

#[derive(Debug, Clone)]
pub struct ConstItem {
    pub name: Ident,
    pub ty: Option<Type>,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub name: Ident,
}

#[derive(Debug, Clone)]
pub struct FnItem {
    pub is_pure: bool,
    pub is_hardware: bool,
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    pub clauses: Vec<ContractClause>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: Ident,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub enum ContractClause {
    Requires(Expr),
    Ensures(Expr),
    Invariant(Expr),
    Reads(Vec<Ident>),
    Writes(Vec<Ident>),
    Decreases(Expr),
}

#[derive(Debug, Clone)]
pub struct StructItem {
    pub name: Ident,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: Ident,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct DataItem {
    pub name: Ident,
    pub ctors: Vec<Ctor>,
}

#[derive(Debug, Clone)]
pub struct Ctor {
    pub name: Ident,
    pub fields: Vec<Type>,
}

#[derive(Debug, Clone)]
pub struct QuotientItem {
    pub name: Ident,
    pub base: Type,
    pub equiv: Expr, // body of `a ~ b => ...`
}

#[derive(Debug, Clone)]
pub struct ContractItem {
    pub name: Ident,
    pub members: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct UseItem {
    pub path: Vec<Ident>,
}

#[derive(Debug, Clone)]
pub struct TypeAliasItem {
    pub name: Ident,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub enum Type {
    Named(String),
    /// `Vector<W, T>` — `Vector<const, type>`.
    Vector(Box<Type>, Box<Type>),
    /// `Field<P>` / `Zmod<M>`.
    Ring(String, Box<Type>),
    /// `&T` / `&mut T`.
    Ref(bool, Box<Type>),
    /// `(x : A) -> B` (dependent arrow; name optional).
    Arrow(Option<String>, Box<Type>, Box<Type>),
    /// `Type` (universe).
    Universe,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(Ident, Option<Type>, Expr),
    Expr(Expr),
    Return(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(String),
    Float(String),
    Str(String),
    Bool(bool),
    Ident(String),
    Glyph(String),
    Bin(Box<Expr>, BinOp, Box<Expr>),
    Un(UnOp, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Field(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Block>, Option<Box<Block>>),
    Match(Box<Expr>, Vec<MatchArm>),
    Block(Block),
    /// `x : T` (type annotation on an expression).
    Annotated(Box<Expr>, Box<Type>),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Ident(String),
    Wildcard,
    Int(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Rem,
    Shl, Shr, BitAnd, BitOr, BitXor,
    And, Or, Eq, Ne, Lt, Gt, Le, Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}
