//! `kernel::json` — hand-rolled, always-compiled, pure-`std` JSON parse + serialize primitive.
//!
//! Roadmap item 31 §4 (enactment half). This is the PARSE-side home for the serde carriers
//! being cut over (Phase A: `agent-facade`, `skillspector-rs`). It is deliberately SEPARATE from
//! `kernel::fdr::json` (which is *serialize*-only, fixed-schema, golden-pinned — see its module
//! doc): bolting a parser onto that writer would break the narrowness that is its whole value.
//!
//! Scope (honest, per §4.2): bounded config / API / JSON-RPC shapes with an explicit max depth
//! and max input length. This is NOT a hardened parser for the attacker-facing HTTP boundary —
//! `tools/native-spa-server` deliberately KEEPS `serde_json` there (a decade of fuzz hardening),
//! and cutting it over would be a security regression until this hand-roll has carried equivalent
//! differential-fuzz load (§4.4 Phase-B trigger). Every malformed input returns `Err` (never a
//! panic, never a wrong-but-silent value); deep nesting is bounded, not stack-exhausting.
//!
//! `serde_json` is retained ONLY as a **dev-dependency differential oracle** (see
//! `kernel/tests/json_oracle.rs`); it sits outside the `-e no-dev` zero-dep proof surface, so the
//! kernel's empty allowlist stays empty.

use std::fmt::Write as _;

/// Maximum nesting depth the parser will descend before refusing (degrade-closed vs stack
/// exhaustion on adversarial deep input). Comfortably above any real config/API/RPC shape.
pub const MAX_DEPTH: usize = 128;

/// Maximum input length the parser will accept, in bytes. Above any real carrier payload; a
/// bound, not a promise of unbounded-input safety.
pub const MAX_LEN: usize = 8 * 1024 * 1024;

/// A parsed JSON value. Mirrors the `serde_json::Value` shape and the accessor subset the
/// carriers actually use (`get`, `as_str`, `as_array`, `as_i64`, `as_f64`, `as_bool`). Numbers
/// are split into `Int`/`Float` (like `serde_json`'s integer/float distinction) so JSON-RPC ids
/// and integer counts round-trip without a spurious decimal point.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Array(Vec<Value>),
    /// Insertion-ordered members. Duplicate keys are preserved as parsed; `get` returns the
    /// LAST occurrence, matching `serde_json`'s last-wins semantics.
    Object(Vec<(String, Value)>),
}

/// A parse failure. Degrade-closed: the caller gets `Err`, never a panic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub msg: &'static str,
    /// Byte offset into the input where the failure was detected.
    pub pos: usize,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "json parse error at byte {}: {}", self.pos, self.msg)
    }
}
impl std::error::Error for Error {}

impl Value {
    /// Object member lookup (last-wins on duplicate keys). `None` for non-objects/missing keys.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(m) => m.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Serialize to compact JSON (no insignificant whitespace). Valid RFC 8259 output:
    /// strings are minimally escaped, non-ASCII passes through as UTF-8, `Int` prints with no
    /// decimal point, `Float` uses Rust's shortest round-tripping `Display`.
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        self.write_to(&mut s);
        s
    }

    fn write_to(&self, out: &mut String) {
        match self {
            Value::Null => out.push_str("null"),
            Value::Bool(true) => out.push_str("true"),
            Value::Bool(false) => out.push_str("false"),
            Value::Int(i) => {
                let _ = write!(out, "{}", i);
            }
            Value::Float(f) => {
                if !f.is_finite() {
                    // JSON has no NaN/Inf — degrade-closed to null rather than emit invalid JSON.
                    out.push_str("null");
                } else {
                    // Rust's `{}` prints large-magnitude floats in full NON-scientific integer form
                    // (e.g. 2.3e136 → a 137-digit integer). A JSON reader re-parses that integer with
                    // different rounding — no round-trip. When the shortest form is a bare integer
                    // AND the magnitude is past 2^53 (where f64 can no longer represent every integer
                    // exactly), emit scientific `{:e}` instead — it always round-trips and matches
                    // serde_json's ryu style. Normal-range numbers keep the readable `{}` form.
                    let s = format!("{}", f);
                    if !s.contains(['.', 'e', 'E']) && f.abs() >= 9_007_199_254_740_992.0 {
                        let _ = write!(out, "{:e}", f);
                    } else {
                        out.push_str(&s);
                    }
                }
            }
            Value::Str(s) => write_json_string(s, out),
            Value::Array(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write_to(out);
                }
                out.push(']');
            }
            Value::Object(m) => {
                out.push('{');
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_json_string(k, out);
                    out.push(':');
                    v.write_to(out);
                }
                out.push('}');
            }
        }
    }
}

// ---- Value construction helpers (for carriers that BUILD output, e.g. skillspector-rs) --------
impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(s.to_owned())
    }
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Str(s)
    }
}
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}
impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}
impl From<usize> for Value {
    fn from(i: usize) -> Self {
        Value::Int(i as i64)
    }
}
impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(o: Option<T>) -> Self {
        o.map(Into::into).unwrap_or(Value::Null)
    }
}

fn write_json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Parse a JSON document. RFC 8259 with bounded depth + input length; degrade-closed.
pub fn parse(input: &str) -> Result<Value, Error> {
    if input.len() > MAX_LEN {
        return Err(Error { msg: "input exceeds MAX_LEN", pos: 0 });
    }
    let mut p = Parser { b: input.as_bytes(), i: 0 };
    p.ws();
    let v = p.value(0)?;
    p.ws();
    if p.i != p.b.len() {
        return Err(Error { msg: "trailing characters after value", pos: p.i });
    }
    Ok(v)
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    #[inline]
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn ws(&mut self) {
        while let Some(c) = self.peek() {
            // RFC 8259 insignificant whitespace: space, tab, LF, CR.
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, Error> {
        if depth > MAX_DEPTH {
            return Err(Error { msg: "max nesting depth exceeded", pos: self.i });
        }
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b't') => self.lit(b"true", Value::Bool(true)),
            Some(b'f') => self.lit(b"false", Value::Bool(false)),
            Some(b'n') => self.lit(b"null", Value::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.number(),
            Some(_) => Err(Error { msg: "unexpected character", pos: self.i }),
            None => Err(Error { msg: "unexpected end of input", pos: self.i }),
        }
    }

    fn lit(&mut self, kw: &[u8], v: Value) -> Result<Value, Error> {
        if self.b[self.i..].starts_with(kw) {
            self.i += kw.len();
            Ok(v)
        } else {
            Err(Error { msg: "invalid literal", pos: self.i })
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, Error> {
        self.i += 1; // consume '{'
        let mut members = Vec::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.ws();
            if self.peek() != Some(b'"') {
                return Err(Error { msg: "expected object key string", pos: self.i });
            }
            let key = self.string()?;
            self.ws();
            if self.peek() != Some(b':') {
                return Err(Error { msg: "expected ':' after object key", pos: self.i });
            }
            self.i += 1;
            self.ws();
            let val = self.value(depth + 1)?;
            members.push((key, val));
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                }
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Value::Object(members));
                }
                _ => return Err(Error { msg: "expected ',' or '}' in object", pos: self.i }),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, Error> {
        self.i += 1; // consume '['
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.ws();
            let val = self.value(depth + 1)?;
            items.push(val);
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                }
                Some(b']') => {
                    self.i += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(Error { msg: "expected ',' or ']' in array", pos: self.i }),
            }
        }
    }

    fn string(&mut self) -> Result<String, Error> {
        self.i += 1; // consume opening '"'
        let mut s = String::new();
        loop {
            let c = match self.peek() {
                Some(c) => c,
                None => return Err(Error { msg: "unterminated string", pos: self.i }),
            };
            match c {
                b'"' => {
                    self.i += 1;
                    return Ok(s);
                }
                b'\\' => {
                    self.i += 1;
                    let e = self.peek().ok_or(Error { msg: "unterminated escape", pos: self.i })?;
                    match e {
                        b'"' => s.push('"'),
                        b'\\' => s.push('\\'),
                        b'/' => s.push('/'),
                        b'b' => s.push('\u{08}'),
                        b'f' => s.push('\u{0c}'),
                        b'n' => s.push('\n'),
                        b'r' => s.push('\r'),
                        b't' => s.push('\t'),
                        b'u' => {
                            self.i += 1;
                            let cp = self.hex4()?;
                            if (0xD800..=0xDBFF).contains(&cp) {
                                // high surrogate — must be followed by \uDC00..=\uDFFF
                                if self.peek() != Some(b'\\') {
                                    return Err(Error { msg: "lone high surrogate", pos: self.i });
                                }
                                self.i += 1;
                                if self.peek() != Some(b'u') {
                                    return Err(Error { msg: "lone high surrogate", pos: self.i });
                                }
                                self.i += 1;
                                let lo = self.hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&lo) {
                                    return Err(Error { msg: "invalid low surrogate", pos: self.i });
                                }
                                let c = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                                match char::from_u32(c) {
                                    Some(ch) => s.push(ch),
                                    None => return Err(Error { msg: "invalid surrogate pair", pos: self.i }),
                                }
                                continue; // hex4 already advanced past the low surrogate digits
                            } else if (0xDC00..=0xDFFF).contains(&cp) {
                                return Err(Error { msg: "lone low surrogate", pos: self.i });
                            } else {
                                match char::from_u32(cp) {
                                    Some(ch) => s.push(ch),
                                    None => return Err(Error { msg: "invalid unicode escape", pos: self.i }),
                                }
                            }
                            continue; // hex4 advanced past the 4 digits
                        }
                        _ => return Err(Error { msg: "invalid escape character", pos: self.i }),
                    }
                    self.i += 1;
                }
                // Raw control characters are not allowed unescaped in a JSON string.
                0x00..=0x1F => return Err(Error { msg: "control character in string", pos: self.i }),
                _ => {
                    // Copy one UTF-8 scalar (input is a valid &str, so byte boundaries are valid).
                    let start = self.i;
                    self.i += 1;
                    while let Some(&b) = self.b.get(self.i) {
                        if b & 0xC0 == 0x80 {
                            self.i += 1;
                        } else {
                            break;
                        }
                    }
                    s.push_str(std::str::from_utf8(&self.b[start..self.i]).unwrap());
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, Error> {
        if self.i + 4 > self.b.len() {
            return Err(Error { msg: "truncated \\u escape", pos: self.i });
        }
        let mut v = 0u32;
        for _ in 0..4 {
            let d = self.b[self.i];
            let n = match d {
                b'0'..=b'9' => (d - b'0') as u32,
                b'a'..=b'f' => (d - b'a' + 10) as u32,
                b'A'..=b'F' => (d - b'A' + 10) as u32,
                _ => return Err(Error { msg: "invalid hex digit in \\u escape", pos: self.i }),
            };
            v = v * 16 + n;
            self.i += 1;
        }
        Ok(v)
    }

    fn number(&mut self) -> Result<Value, Error> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        // integer part: 0 alone, or [1-9][0-9]*  (leading zeros are invalid JSON)
        match self.peek() {
            Some(b'0') => {
                self.i += 1;
            }
            Some(c) if c.is_ascii_digit() => {
                while matches!(self.peek(), Some(d) if d.is_ascii_digit()) {
                    self.i += 1;
                }
            }
            _ => return Err(Error { msg: "invalid number", pos: self.i }),
        }
        let mut is_float = false;
        // fraction
        if self.peek() == Some(b'.') {
            is_float = true;
            self.i += 1;
            if !matches!(self.peek(), Some(d) if d.is_ascii_digit()) {
                return Err(Error { msg: "digit expected after decimal point", pos: self.i });
            }
            while matches!(self.peek(), Some(d) if d.is_ascii_digit()) {
                self.i += 1;
            }
        }
        // exponent
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            is_float = true;
            self.i += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.i += 1;
            }
            if !matches!(self.peek(), Some(d) if d.is_ascii_digit()) {
                return Err(Error { msg: "digit expected in exponent", pos: self.i });
            }
            while matches!(self.peek(), Some(d) if d.is_ascii_digit()) {
                self.i += 1;
            }
        }
        let text = std::str::from_utf8(&self.b[start..self.i]).unwrap();
        if !is_float {
            // Negative zero: `i64` has no -0, but the incumbent (serde_json) preserves the sign as
            // the float -0.0. Match it so the differential oracle is exact (the ONLY integer
            // literal that carries a distinguishable sign-of-zero).
            if text == "-0" {
                return Ok(Value::Float(-0.0));
            }
            if let Ok(i) = text.parse::<i64>() {
                return Ok(Value::Int(i));
            }
            // Integer too large for i64 — fall through to f64 (matches serde_json's behaviour of
            // widening an out-of-i64/u64-range integer literal to a float).
        }
        match text.parse::<f64>() {
            Ok(f) => Ok(Value::Float(f)),
            Err(_) => Err(Error { msg: "number out of range", pos: start }),
        }
    }
}
