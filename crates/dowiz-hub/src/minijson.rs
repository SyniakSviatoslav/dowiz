//! A deliberately tiny JSON reader and writer for hub-internal records.
//!
//! WHY NOT serde. These records are written and read by this crate alone, and
//! one of them — the token payload — is SIGNED. A signed payload must have a
//! byte-for-byte stable encoding, and a derive's output is stable only until
//! the derive changes. Writing the fields in a fixed order by hand makes the
//! signed bytes a property of this file rather than of a dependency's version.
//!
//! WHY IT IS SAFE TO BE THIS SMALL. The reader is not a general JSON parser and
//! is never pointed at untrusted documents. It reads records produced by the
//! writer below, whose shape is flat and known: string and integer fields, no
//! nesting, no arrays. A general parser here would be a larger attack surface
//! for no gain. Anything arriving from the network is parsed by `serde_json` at
//! the HTTP boundary instead.

/// Escape a value going into a JSON string.
///
/// Load-bearing, not cosmetic: an unescaped quote in a value would close the
/// string early and let the value inject SIBLING FIELDS into the record. In a
/// token payload that happens before signing, so the MAC would be valid over
/// the forged claims.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

pub fn unesc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('u') => {
                let hex: String = it.by_ref().take(4).collect();
                if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    out.push(ch);
                }
            }
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// The RAW (still-escaped) text of a string field, or `None` if absent.
///
/// Returns the raw slice so a caller that needs the escaped form — a verifier
/// comparing signed bytes — can have it, and one that needs the value calls
/// [`unesc`].
pub fn raw_str<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\":\"");
    let start = json.find(&pat)? + pat.len();
    let rest = &json[start..];
    let b = rest.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'"' => return Some(&rest[..i]),
            _ => i += 1,
        }
    }
    None
}

/// The value of a string field, unescaped.
pub fn str_field(json: &str, key: &str) -> Option<String> {
    raw_str(json, key).map(unesc)
}

/// The value of an integer field.
pub fn int_field(json: &str, key: &str) -> Option<i64> {
    let pat = format!("\"{key}\":");
    // Skip a string field that happens to share the prefix, e.g. looking for
    // "i" must not match "id".
    let mut from = 0usize;
    loop {
        let at = json[from..].find(&pat)? + from;
        let rest = &json[at + pat.len()..];
        if rest.starts_with('"') {
            from = at + pat.len();
            continue;
        }
        let end = rest
            .find(|c: char| !c.is_ascii_digit() && c != '-')
            .unwrap_or(rest.len());
        return rest[..end].parse().ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_round_trips() {
        for s in ["", "plain", "with \"quotes\"", "back\\slash", "line\nbreak", "tab\there"] {
            assert_eq!(unesc(&esc(s)), s, "round trip {s:?}");
        }
    }

    /// The injection case, stated as a test rather than a comment: a value
    /// containing a quote and a field must stay ONE value.
    #[test]
    fn a_quote_in_a_value_cannot_add_a_field() {
        let evil = r#"x","role":"owner"#;
        let doc = format!(r#"{{"name":"{}","role":"courier"}}"#, esc(evil));
        assert_eq!(str_field(&doc, "name").as_deref(), Some(evil));
        assert_eq!(str_field(&doc, "role").as_deref(), Some("courier"), "the real role must win");
    }

    #[test]
    fn fields_are_read_by_exact_key() {
        let doc = r#"{"id":"p1","i":42,"issued":7,"name":"Ana"}"#;
        assert_eq!(str_field(doc, "id").as_deref(), Some("p1"));
        assert_eq!(str_field(doc, "name").as_deref(), Some("Ana"));
        assert_eq!(int_field(doc, "i"), Some(42));
        assert_eq!(int_field(doc, "issued"), Some(7));
        assert_eq!(str_field(doc, "missing"), None);
        assert_eq!(int_field(doc, "missing"), None);
    }

    /// An integer lookup must skip a STRING field with the same key prefix
    /// rather than trying to parse its opening quote as a digit.
    #[test]
    fn an_int_lookup_skips_a_same_named_string() {
        let doc = r#"{"e":"not a number","ee":5,"e":9}"#;
        assert_eq!(int_field(doc, "e"), Some(9), "must skip the string and find the number");
    }

    #[test]
    fn negative_numbers_read_back() {
        assert_eq!(int_field(r#"{"n":-1700}"#, "n"), Some(-1700));
    }
}
