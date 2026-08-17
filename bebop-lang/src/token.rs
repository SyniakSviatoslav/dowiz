//! Token definitions for the Bebop lexer (spec §2).

/// A source span (line, column), 1-based. Columns are byte columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: u32,
    pub col: u32,
}

impl Span {
    pub fn new(line: u32, col: u32) -> Self {
        Span { line, col }
    }
}

/// Reserved keywords (spec §2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kw {
    Fn, Data, Struct, Enum, Trait, Impl, Module, Use, Type, Let, Mut,
    Quotient, Contract,
    Match, If, Else, Loop, While, For, Return, Break, Continue,
    Pure, Ghost, Comptime, Const, Inline, Extern,
    Requires, Ensures, Invariant, Reads, Writes, Decreases, Where,
    SelfKw, SelfType, Pub, Priv, True, False, Nil, Unsafe, Hardware, As,
}

impl Kw {
    pub fn from_str(s: &str) -> Option<Kw> {
        Some(match s {
            "fn" => Kw::Fn,
            "data" => Kw::Data,
            "struct" => Kw::Struct,
            "enum" => Kw::Enum,
            "trait" => Kw::Trait,
            "impl" => Kw::Impl,
            "module" => Kw::Module,
            "use" => Kw::Use,
            "type" => Kw::Type,
            "let" => Kw::Let,
            "mut" => Kw::Mut,
            "quotient" => Kw::Quotient,
            "contract" => Kw::Contract,
            "match" => Kw::Match,
            "if" => Kw::If,
            "else" => Kw::Else,
            "loop" => Kw::Loop,
            "while" => Kw::While,
            "for" => Kw::For,
            "return" => Kw::Return,
            "break" => Kw::Break,
            "continue" => Kw::Continue,
            "pure" => Kw::Pure,
            "ghost" => Kw::Ghost,
            "comptime" => Kw::Comptime,
            "const" => Kw::Const,
            "inline" => Kw::Inline,
            "extern" => Kw::Extern,
            "requires" => Kw::Requires,
            "ensures" => Kw::Ensures,
            "invariant" => Kw::Invariant,
            "reads" => Kw::Reads,
            "writes" => Kw::Writes,
            "decreases" => Kw::Decreases,
            "where" => Kw::Where,
            "self" => Kw::SelfKw,
            "Self" => Kw::SelfType,
            "pub" => Kw::Pub,
            "priv" => Kw::Priv,
            "true" => Kw::True,
            "false" => Kw::False,
            "nil" => Kw::Nil,
            "unsafe" => Kw::Unsafe,
            "hardware" => Kw::Hardware,
            "as" => Kw::As,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Kw::Fn => "fn",
            Kw::Data => "data",
            Kw::Struct => "struct",
            Kw::Enum => "enum",
            Kw::Trait => "trait",
            Kw::Impl => "impl",
            Kw::Module => "module",
            Kw::Use => "use",
            Kw::Type => "type",
            Kw::Let => "let",
            Kw::Mut => "mut",
            Kw::Quotient => "quotient",
            Kw::Contract => "contract",
            Kw::Match => "match",
            Kw::If => "if",
            Kw::Else => "else",
            Kw::Loop => "loop",
            Kw::While => "while",
            Kw::For => "for",
            Kw::Return => "return",
            Kw::Break => "break",
            Kw::Continue => "continue",
            Kw::Pure => "pure",
            Kw::Ghost => "ghost",
            Kw::Comptime => "comptime",
            Kw::Const => "const",
            Kw::Inline => "inline",
            Kw::Extern => "extern",
            Kw::Requires => "requires",
            Kw::Ensures => "ensures",
            Kw::Invariant => "invariant",
            Kw::Reads => "reads",
            Kw::Writes => "writes",
            Kw::Decreases => "decreases",
            Kw::Where => "where",
            Kw::SelfKw => "self",
            Kw::SelfType => "Self",
            Kw::Pub => "pub",
            Kw::Priv => "priv",
            Kw::True => "true",
            Kw::False => "false",
            Kw::Nil => "nil",
            Kw::Unsafe => "unsafe",
            Kw::Hardware => "hardware",
            Kw::As => "as",
        }
    }
}

/// Token kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    IntLit(String),
    FloatLit(String),
    StrLit(String),
    CharLit(String),
    GlyphLit(String), // ◇name◇
    Ident(String),
    Keyword(Kw),

    // operators
    Plus, Minus, Star, Slash, Percent,
    Shl, Shr, Amp, Pipe, Caret, Bang,
    AndAnd, OrOr,
    Eq, EqEq, Ne, Lt, Gt, Le, Ge,
    Arrow,      // ->
    FatArrow,   // =>
    ColonColon, // ::

    // delimiters
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    LAngle, RAngle, Comma, Semicolon, Colon, Dot, At, Hash,

    Eof,
}

impl TokenKind {
    /// Human-readable name for error messages / token dumps.
    pub fn describe(&self) -> String {
        match self {
            TokenKind::IntLit(s) => format!("int literal `{s}`"),
            TokenKind::FloatLit(s) => format!("float literal `{s}`"),
            TokenKind::StrLit(_) => "string literal".into(),
            TokenKind::CharLit(_) => "char literal".into(),
            TokenKind::GlyphLit(s) => format!("glyph literal `◇{s}◇`"),
            TokenKind::Ident(s) => format!("identifier `{s}`"),
            TokenKind::Keyword(k) => format!("keyword `{}`", k.as_str()),
            TokenKind::Plus => "`+`".into(),
            TokenKind::Minus => "`-`".into(),
            TokenKind::Star => "`*`".into(),
            TokenKind::Slash => "`/`".into(),
            TokenKind::Percent => "`%`".into(),
            TokenKind::Shl => "`<<`".into(),
            TokenKind::Shr => "`>>`".into(),
            TokenKind::Amp => "`&`".into(),
            TokenKind::Pipe => "`|`".into(),
            TokenKind::Caret => "`^`".into(),
            TokenKind::Bang => "`!`".into(),
            TokenKind::AndAnd => "`&&`".into(),
            TokenKind::OrOr => "`||`".into(),
            TokenKind::Eq => "`=`".into(),
            TokenKind::EqEq => "`==`".into(),
            TokenKind::Ne => "`!=`".into(),
            TokenKind::Lt => "`<`".into(),
            TokenKind::Gt => "`>`".into(),
            TokenKind::Le => "`<=`".into(),
            TokenKind::Ge => "`>=`".into(),
            TokenKind::Arrow => "`->`".into(),
            TokenKind::FatArrow => "`=>`".into(),
            TokenKind::ColonColon => "`::`".into(),
            TokenKind::LParen => "`(`".into(),
            TokenKind::RParen => "`)`".into(),
            TokenKind::LBrace => "`{`".into(),
            TokenKind::RBrace => "`}`".into(),
            TokenKind::LBracket => "`[`".into(),
            TokenKind::RBracket => "`]`".into(),
            TokenKind::LAngle => "`<`".into(),
            TokenKind::RAngle => "`>`".into(),
            TokenKind::Comma => "`,`".into(),
            TokenKind::Semicolon => "`;`".into(),
            TokenKind::Colon => "`:`".into(),
            TokenKind::Dot => "`.`".into(),
            TokenKind::At => "`@`".into(),
            TokenKind::Hash => "`#`".into(),
            TokenKind::Eof => "end of file".into(),
        }
    }
}

/// A token with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Token { kind, span }
    }
}
