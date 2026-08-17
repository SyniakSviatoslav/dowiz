//! Hand-rolled lexer for Bebop (spec §2). Zero dependencies.
//!
//! Produces a `Vec<Token>` from UTF-8 source. Glyph literals are `◇name◇`
//! (U+25C7 white diamond delimiters); comments are `//` and `/* */`.

use crate::token::{Kw, Span, Token, TokenKind};

pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: u32,
    col: u32,
}

#[derive(Debug)]
pub struct LexError {
    pub line: u32,
    pub col: u32,
    pub msg: String,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lex error at {}:{}: {}", self.line, self.col, self.msg)
    }
}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let mut lx = Lexer {
        src: src.as_bytes(),
        pos: 0,
        line: 1,
        col: 1,
    };
    let mut toks = Vec::new();
    loop {
        lx.skip_trivia()?;
        let span = Span::new(lx.line, lx.col);
        if lx.pos >= lx.src.len() {
            toks.push(Token::new(TokenKind::Eof, span));
            return Ok(toks);
        }
        let kind = lx.next_token()?;
        toks.push(Token::new(kind, span));
    }
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }
    fn peek2(&self) -> Option<u8> {
        self.src.get(self.pos + 1).copied()
    }
    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        if c == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }
    fn err<T>(&self, msg: impl Into<String>) -> Result<T, LexError> {
        Err(LexError {
            line: self.line,
            col: self.col,
            msg: msg.into(),
        })
    }

    fn skip_trivia(&mut self) -> Result<(), LexError> {
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n') => {
                    self.bump();
                }
                Some(b'/') if self.peek2() == Some(b'/') => {
                    while let Some(c) = self.peek() {
                        if c == b'\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                Some(b'/') if self.peek2() == Some(b'*') => {
                    self.bump();
                    self.bump();
                    let mut depth = 1;
                    while depth > 0 {
                        match (self.peek(), self.peek2()) {
                            (Some(b'*'), Some(b'/')) => {
                                self.bump();
                                self.bump();
                                depth -= 1;
                            }
                            (Some(b'/'), Some(b'*')) => {
                                self.bump();
                                self.bump();
                                depth += 1;
                            }
                            (Some(_), _) => {
                                self.bump();
                            }
                            (None, _) => return self.err("unterminated block comment"),
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn next_token(&mut self) -> Result<TokenKind, LexError> {
        let c = self.peek().unwrap();

        // glyph literal ◇name◇
        if c == 0xE2 && self.peek2() == Some(0x97) && self.src.get(self.pos + 2) == Some(&0x87) {
            // U+25C7 ◇ is E2 97 87
            self.bump();
            self.bump();
            self.bump();
            let mut name = String::new();
            loop {
                let b = self.peek();
                if b == Some(0xE2)
                    && self.peek2() == Some(0x97)
                    && self.src.get(self.pos + 2) == Some(&0x87)
                {
                    self.bump();
                    self.bump();
                    self.bump();
                    break;
                }
                match b {
                    Some(x) if x < 0x80 => {
                        name.push(x as char);
                        self.bump();
                    }
                    Some(_) => return self.err("non-ASCII in glyph name"),
                    None => return self.err("unterminated glyph literal"),
                }
            }
            return Ok(TokenKind::GlyphLit(name));
        }

        // identifiers / keywords
        if c.is_ascii_alphabetic() || c == b'_' {
            return self.lex_ident();
        }

        // numbers
        if c.is_ascii_digit() {
            return self.lex_number();
        }

        // string / char
        if c == b'"' {
            return self.lex_string();
        }
        if c == b'\'' {
            return self.lex_char();
        }

        // operators and delimiters
        self.lex_operator()
    }

    fn lex_ident(&mut self) -> Result<TokenKind, LexError> {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' {
                s.push(c as char);
                self.bump();
            } else {
                break;
            }
        }
        if let Some(kw) = Kw::from_str(&s) {
            Ok(TokenKind::Keyword(kw))
        } else {
            Ok(TokenKind::Ident(s))
        }
    }

    fn lex_number(&mut self) -> Result<TokenKind, LexError> {
        let mut s = String::new();
        // hex 0x / bin 0b
        if self.peek() == Some(b'0') && matches!(self.peek2(), Some(b'x') | Some(b'X') | Some(b'b') | Some(b'B')) {
            s.push('0');
            self.bump();
            s.push(self.bump().unwrap() as char);
            while let Some(c) = self.peek() {
                if c.is_ascii_hexdigit() || c == b'_' {
                    s.push(c as char);
                    self.bump();
                } else {
                    break;
                }
            }
            return Ok(TokenKind::IntLit(s));
        }
        // decimal / float
        let mut is_float = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == b'_' {
                s.push(c as char);
                self.bump();
            } else if c == b'.' && self.peek2().map_or(false, |d| d.is_ascii_digit()) {
                is_float = true;
                s.push('.');
                self.bump();
            } else if c == b'e' || c == b'E' {
                is_float = true;
                s.push(c as char);
                self.bump();
                if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                    s.push(self.bump().unwrap() as char);
                }
            } else {
                break;
            }
        }
        if is_float {
            Ok(TokenKind::FloatLit(s))
        } else {
            Ok(TokenKind::IntLit(s))
        }
    }

    fn lex_string(&mut self) -> Result<TokenKind, LexError> {
        self.bump(); // opening "
        let mut s = String::new();
        loop {
            match self.bump() {
                Some(b'"') => return Ok(TokenKind::StrLit(s)),
                Some(b'\\') => match self.bump() {
                    Some(esc) => {
                        s.push(match esc {
                            b'n' => '\n',
                            b't' => '\t',
                            b'r' => '\r',
                            b'0' => '\0',
                            b'"' => '"',
                            b'\\' => '\\',
                            other => return self.err(format!("bad escape \\{}", other as char)),
                        })
                    }
                    None => return self.err("unterminated string"),
                },
                Some(c) => s.push(c as char),
                None => return self.err("unterminated string literal"),
            }
        }
    }

    fn lex_char(&mut self) -> Result<TokenKind, LexError> {
        self.bump(); // opening '
        let c = self.bump().ok_or_else(|| LexError {
            line: self.line,
            col: self.col,
            msg: "unterminated char literal".into(),
        })?;
        let ch = if c == b'\\' {
            let esc = self.bump().ok_or_else(|| LexError {
                line: self.line,
                col: self.col,
                msg: "unterminated char escape".into(),
            })?;
            match esc {
                b'n' => '\n',
                b't' => '\t',
                b'r' => '\r',
                b'0' => '\0',
                b'\'' => '\'',
                b'\\' => '\\',
                other => return self.err(format!("bad escape \\{}", other as char)),
            }
        } else {
            c as char
        };
        match self.bump() {
            Some(b'\'') => Ok(TokenKind::CharLit(ch.to_string())),
            _ => self.err("unterminated char literal"),
        }
    }

    fn lex_operator(&mut self) -> Result<TokenKind, LexError> {
        let c = self.bump().unwrap();
        Ok(match c {
            b'+' => TokenKind::Plus,
            b'-' => {
                if self.peek() == Some(b'>') {
                    self.bump();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            b'*' => TokenKind::Star,
            b'/' => TokenKind::Slash,
            b'%' => TokenKind::Percent,
            b'^' => TokenKind::Caret,
            b'!' => {
                if self.peek() == Some(b'=') {
                    self.bump();
                    TokenKind::Ne
                } else {
                    TokenKind::Bang
                }
            }
            b'&' => {
                if self.peek() == Some(b'&') {
                    self.bump();
                    TokenKind::AndAnd
                } else {
                    TokenKind::Amp
                }
            }
            b'|' => {
                if self.peek() == Some(b'|') {
                    self.bump();
                    TokenKind::OrOr
                } else {
                    TokenKind::Pipe
                }
            }
            b'=' => {
                if self.peek() == Some(b'=') {
                    self.bump();
                    TokenKind::EqEq
                } else if self.peek() == Some(b'>') {
                    self.bump();
                    TokenKind::FatArrow
                } else {
                    TokenKind::Eq
                }
            }
            b'<' => {
                if self.peek() == Some(b'<') {
                    self.bump();
                    TokenKind::Shl
                } else if self.peek() == Some(b'=') {
                    self.bump();
                    TokenKind::Le
                } else {
                    TokenKind::Lt
                }
            }
            b'>' => {
                if self.peek() == Some(b'>') {
                    self.bump();
                    TokenKind::Shr
                } else if self.peek() == Some(b'=') {
                    self.bump();
                    TokenKind::Ge
                } else {
                    TokenKind::Gt
                }
            }
            b'(' => TokenKind::LParen,
            b')' => TokenKind::RParen,
            b'{' => TokenKind::LBrace,
            b'}' => TokenKind::RBrace,
            b'[' => TokenKind::LBracket,
            b']' => TokenKind::RBracket,
            b',' => TokenKind::Comma,
            b';' => TokenKind::Semicolon,
            b':' => {
                if self.peek() == Some(b':') {
                    self.bump();
                    TokenKind::ColonColon
                } else {
                    TokenKind::Colon
                }
            }
            b'.' => TokenKind::Dot,
            b'@' => TokenKind::At,
            b'#' => TokenKind::Hash,
            other => return self.err(format!("unexpected character `{}`", other as char)),
        })
    }
}
