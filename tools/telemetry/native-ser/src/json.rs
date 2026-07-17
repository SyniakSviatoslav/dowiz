//! Minimal hand-rolled JSON — enough for the EDGE adapter contract
//! (`json_edge` / `from_json`). No serde, no external crates.
//!
//! Supports the subset the telemetry edges actually emit/consume:
//! objects, arrays, numbers (f64), strings (with the common escapes),
//! `true`/`false`/`null`. This mirrors ser.py's stdlib-`json` use exactly:
//! the gauges dict (`{"disk_pct":81.0,"load1":0.12,"mem_pct":9.0,"ts":...}`)
//! round-trips, which is the only thing the EDGE adapter needs.

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        self.write(&mut s);
        s
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => {
                if n.fract() == 0.0 && n.is_finite() && n.abs() < 1e15 {
                    out.push_str(&format!("{}", *n as i64));
                } else {
                    out.push_str(&format!("{}", n));
                }
            }
            Json::Str(s) => {
                out.push('"');
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        _ => out.push(c),
                    }
                }
                out.push('"');
            }
            Json::Arr(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out);
                }
                out.push(']');
            }
            Json::Obj(o) => {
                out.push('{');
                for (i, (k, v)) in o.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    Json::Str(k.clone()).write(out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// Parse a JSON string. Returns `Err` with a position on the first failure.
pub fn parse(s: &str) -> Result<Json, String> {
    let bytes: Vec<char> = s.chars().collect();
    let mut p = Parser { b: &bytes, i: 0 };
    p.skip_ws();
    let v = p.parse_value()?;
    p.skip_ws();
    if p.i != p.b.len() {
        return Err(format!("trailing chars at {}", p.i));
    }
    Ok(v)
}

struct Parser<'a> {
    b: &'a [char],
    i: usize,
}

impl<'a> Parser<'a> {
    fn skip_ws(&mut self) {
        while self.i < self.b.len() {
            match self.b[self.i] {
                ' ' | '\t' | '\n' | '\r' => self.i += 1,
                _ => break,
            }
        }
    }

    fn parse_value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        if self.i >= self.b.len() {
            return Err("unexpected eof".into());
        }
        match self.b[self.i] {
            '{' => self.parse_obj(),
            '[' => self.parse_arr(),
            '"' => Ok(Json::Str(self.parse_str()?)),
            't' | 'f' => self.parse_bool(),
            'n' => self.parse_null(),
            _ => self.parse_num(),
        }
    }

    fn parse_obj(&mut self) -> Result<Json, String> {
        self.i += 1; // {
        let mut out = Vec::new();
        self.skip_ws();
        if self.peek() == Some('}') {
            self.i += 1;
            return Ok(Json::Obj(out));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some('"') {
                return Err(format!("expected key at {}", self.i));
            }
            let k = self.parse_str()?;
            self.skip_ws();
            if self.peek() != Some(':') {
                return Err(format!("expected : at {}", self.i));
            }
            self.i += 1;
            let v = self.parse_value()?;
            out.push((k, v));
            self.skip_ws();
            match self.peek() {
                Some(',') => {
                    self.i += 1;
                }
                Some('}') => {
                    self.i += 1;
                    break;
                }
                _ => return Err(format!("expected , or }} at {}", self.i)),
            }
        }
        Ok(Json::Obj(out))
    }

    fn parse_arr(&mut self) -> Result<Json, String> {
        self.i += 1; // [
        let mut out = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.i += 1;
            return Ok(Json::Arr(out));
        }
        loop {
            let v = self.parse_value()?;
            out.push(v);
            self.skip_ws();
            match self.peek() {
                Some(',') => {
                    self.i += 1;
                }
                Some(']') => {
                    self.i += 1;
                    break;
                }
                _ => return Err(format!("expected , or ] at {}", self.i)),
            }
        }
        Ok(Json::Arr(out))
    }

    fn parse_str(&mut self) -> Result<String, String> {
        self.i += 1; // opening "
        let mut s = String::new();
        while self.i < self.b.len() {
            let c = self.b[self.i];
            self.i += 1;
            match c {
                '"' => return Ok(s),
                '\\' => {
                    if self.i >= self.b.len() {
                        return Err("bad escape".into());
                    }
                    let e = self.b[self.i];
                    self.i += 1;
                    match e {
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        '/' => s.push('/'),
                        'n' => s.push('\n'),
                        'r' => s.push('\r'),
                        't' => s.push('\t'),
                        _ => return Err(format!("unsupported escape \\{}", e)),
                    }
                }
                _ => s.push(c),
            }
        }
        Err("unterminated string".into())
    }

    fn parse_bool(&mut self) -> Result<Json, String> {
        if self.b[self.i..].starts_with(&['t', 'r', 'u', 'e']) {
            self.i += 4;
            Ok(Json::Bool(true))
        } else if self.b[self.i..].starts_with(&['f', 'a', 'l', 's', 'e']) {
            self.i += 5;
            Ok(Json::Bool(false))
        } else {
            Err(format!("bad literal at {}", self.i))
        }
    }

    fn parse_null(&mut self) -> Result<Json, String> {
        if self.b[self.i..].starts_with(&['n', 'u', 'l', 'l']) {
            self.i += 4;
            Ok(Json::Null)
        } else {
            Err(format!("bad literal at {}", self.i))
        }
    }

    fn parse_num(&mut self) -> Result<Json, String> {
        let start = self.i;
        while self.i < self.b.len() {
            match self.b[self.i] {
                '0'..='9' | '.' | '-' | '+' | 'e' | 'E' => self.i += 1,
                _ => break,
            }
        }
        let s: String = self.b[start..self.i].iter().collect();
        s.parse::<f64>()
            .map(Json::Num)
            .map_err(|_| format!("bad number {:?}", s))
    }

    fn peek(&self) -> Option<char> {
        self.b.get(self.i).copied()
    }
}
