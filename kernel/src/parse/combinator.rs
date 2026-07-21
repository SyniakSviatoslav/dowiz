use crate::TriState;

#[derive(Debug, Clone, Copy)]
pub struct Input<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

impl<'a> Input<'a> {
    pub fn new(data: &'a [u8]) -> Self { Input { data, pos: 0 } }
    pub fn remaining(&self) -> &'a [u8] { &self.data[self.pos..] }
    pub fn is_empty(&self) -> bool { self.pos >= self.data.len() }
    pub fn advance(&mut self, n: usize) { self.pos = self.pos.saturating_add(n).min(self.data.len()); }
    pub fn peek(&self) -> Option<u8> { self.data.get(self.pos).copied() }
}

#[derive(Debug, Clone)]
pub struct ParseResult<T> {
    pub value: Option<T>,
    pub errors: Vec<ParseError>,
    pub consumed: usize,
}

impl<T> ParseResult<T> {
    pub fn ok(value: T, consumed: usize) -> Self { ParseResult { value: Some(value), errors: vec![], consumed } }
    pub fn fail(err: ParseError) -> Self { ParseResult { value: None, errors: vec![err], consumed: 0 } }
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub position: usize,
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    UnexpectedByte,
    InvalidUtf8,
    ExpectedDigit,
    ExpectedAlpha,
    ExpectedWhitespace,
    EndOfInput,
    Custom,
}

impl ParseError {
    pub fn new(pos: usize, kind: ErrorKind, msg: &str) -> Self { ParseError { position: pos, kind, message: msg.to_string() } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteClass {
    Whitespace, Digit, Alpha, Structural, Other, End,
}

pub static BYTE_CLASS: [ByteClass; 256] = {
    let mut t = [ByteClass::Other; 256];
    let mut i = 0;
    while i < 256 {
        t[i] = match i as u8 {
            b' ' | b'\t' | b'\n' | b'\r' => ByteClass::Whitespace,
            b'0'..=b'9' => ByteClass::Digit,
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => ByteClass::Alpha,
            b'{' | b'}' | b'[' | b']' | b':' | b',' | b'"' | b'\\' | b'/' => ByteClass::Structural,
            _ => ByteClass::Other,
        };
        i += 1;
    }
    t
};

pub fn skip_ws(input: &mut Input) {
    while let Some(b) = input.peek() {
        if !matches!(BYTE_CLASS[b as usize], ByteClass::Whitespace) { break; }
        input.advance(1);
    }
}

pub fn byte(input: &mut Input, expected: u8) -> ParseResult<u8> {
    match input.peek() {
        Some(b) if b == expected => { input.advance(1); ParseResult::ok(b, 1) }
        Some(b) => ParseResult::fail(ParseError::new(input.pos, ErrorKind::UnexpectedByte, &format!("expected '{}', got '{}'", expected as char, b as char))),
        None => ParseResult::fail(ParseError::new(input.pos, ErrorKind::EndOfInput, "unexpected end")),
    }
}

pub fn take_while<F: Fn(u8) -> bool>(input: &mut Input, pred: F) -> ParseResult<Vec<u8>> {
    let start = input.pos;
    while let Some(b) = input.peek() {
        if !pred(b) { break; }
        input.advance(1);
    }
    ParseResult::ok(input.data[start..input.pos].to_vec(), input.pos - start)
}

pub fn digit(input: &mut Input) -> ParseResult<u8> {
    match input.peek() {
        Some(b @ b'0'..=b'9') => { input.advance(1); ParseResult::ok(b - b'0', 1) }
        Some(b) => ParseResult::fail(ParseError::new(input.pos, ErrorKind::ExpectedDigit, &format!("expected digit, got '{}'", b as char))),
        None => ParseResult::fail(ParseError::new(input.pos, ErrorKind::EndOfInput, "expected digit")),
    }
}

pub fn number(input: &mut Input) -> ParseResult<u64> {
    let start = input.pos;
    let mut val: u64 = 0;
    loop {
        match digit(input).value {
            Some(d) => val = val.saturating_mul(10).saturating_add(d as u64),
            None => break,
        }
    }
    if input.pos > start { ParseResult::ok(val, input.pos - start) }
    else { ParseResult::fail(ParseError::new(input.pos, ErrorKind::ExpectedDigit, "expected number")) }
}

pub fn recover_to(input: &mut Input, sync_bytes: &[u8]) -> usize {
    let mut skipped = 0;
    while let Some(b) = input.peek() {
        if sync_bytes.contains(&b) { break; }
        input.advance(1);
        skipped += 1;
    }
    skipped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_class_table() {
        assert_eq!(BYTE_CLASS.len(), 256);
        assert_eq!(BYTE_CLASS[b' ' as usize], ByteClass::Whitespace);
        assert_eq!(BYTE_CLASS[b'0' as usize], ByteClass::Digit);
    }

    #[test]
    fn skip_whitespace() {
        let mut input = Input::new(b"   hello");
        skip_ws(&mut input);
        assert_eq!(input.peek(), Some(b'h'));
    }

    #[test]
    fn match_byte() {
        let mut input = Input::new(b"abc");
        let r = byte(&mut input, b'a');
        assert!(r.value.is_some());
        assert_eq!(input.pos, 1);
    }

    #[test]
    fn parse_digit() {
        let mut input = Input::new(b"5");
        let r = digit(&mut input);
        assert_eq!(r.value, Some(5));
    }

    #[test]
    fn parse_number() {
        let mut input = Input::new(b"12345");
        let r = number(&mut input);
        assert_eq!(r.value, Some(12345));
    }

    #[test]
    fn error_recovery() {
        let mut input = Input::new(b"xxx---yyy");
        let skipped = recover_to(&mut input, b"-");
        assert!(skipped > 0);
        assert_eq!(input.peek(), Some(b'-'));
    }

    #[test]
    fn number_via_structural_scan() {
        let mut input = Input::new(b"  \t\n42");
        skip_ws(&mut input);
        let r = number(&mut input);
        assert_eq!(r.value, Some(42));
    }
}
