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

/// The value of a boolean field. `None` when the key is absent OR holds
/// something that is not a bare `true`/`false`, so a caller decides what a
/// missing switch means rather than inheriting `false` by accident.
pub fn bool_field(json: &str, key: &str) -> Option<bool> {
    let pat = format!("\"{key}\":");
    let mut from = 0usize;
    loop {
        let at = json[from..].find(&pat)? + from;
        let rest = &json[at + pat.len()..];
        if rest.starts_with("true") {
            return Some(true);
        }
        if rest.starts_with("false") {
            return Some(false);
        }
        from = at + pat.len();
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

/// The raw text of each object in a JSON array field, in order.
///
/// WRITTEN HERE RATHER THAN PULLING IN A PARSER because this crate has two
/// dependencies and the gate that keeps it that way may only shrink. It does
/// one job: hand back the `{...}` slices of `"key":[{...},{...}]` so a caller
/// can read fields out of each with the functions above.
///
/// STRING-AWARE, which is the whole difficulty. A brace inside a dish name —
/// and a restaurant menu is exactly where one turns up — would end an object
/// early, and a `\"` inside that name would end the string early. Both are
/// tracked; a malformed array yields what it could read rather than panicking,
/// because a menu that half-parses must not take the hub down with it.
pub fn objects_in(json: &str, key: &str) -> Vec<String> {
    let pat = format!("\"{key}\":[");
    let Some(start) = json.find(&pat) else { return Vec::new() };
    let b = json.as_bytes();
    let mut i = start + pat.len();
    let (mut out, mut depth, mut begin, mut in_str) = (Vec::new(), 0usize, 0usize, false);
    while i < b.len() {
        let c = b[i];
        if in_str {
            match c {
                b'\\' => i += 1,
                b'"' => in_str = false,
                _ => {}
            }
        } else {
            match c {
                b'"' => in_str = true,
                b'{' => {
                    if depth == 0 {
                        begin = i;
                    }
                    depth += 1;
                }
                b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        out.push(json[begin..=i].to_string());
                    }
                }
                // The array's own close, at depth zero, ends the field.
                b']' if depth == 0 => break,
                _ => {}
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod array_tests {
    use super::*;

    #[test]
    fn objects_come_back_whole_and_in_order() {
        let j = r#"{"items":[{"id":"a","qty":1},{"id":"b","qty":2}],"total":300}"#;
        let got = objects_in(j, "items");
        assert_eq!(got.len(), 2);
        assert_eq!(str_field(&got[0], "id").as_deref(), Some("a"));
        assert_eq!(int_field(&got[1], "qty"), Some(2));
    }

    /// The one that matters on a restaurant menu: punctuation inside a name.
    #[test]
    fn a_brace_inside_a_name_does_not_end_the_object() {
        let j = r#"{"items":[{"name":"Set {50/50}","id":"x"},{"id":"y"}]}"#;
        let got = objects_in(j, "items");
        assert_eq!(got.len(), 2, "got {got:?}");
        assert_eq!(str_field(&got[0], "id").as_deref(), Some("x"));
        assert_eq!(str_field(&got[1], "id").as_deref(), Some("y"));
    }

    #[test]
    fn an_escaped_quote_does_not_end_the_string() {
        let j = r#"{"items":[{"name":"Chef\"s pick","id":"x"}]}"#;
        let got = objects_in(j, "items");
        assert_eq!(got.len(), 1);
        assert_eq!(str_field(&got[0], "id").as_deref(), Some("x"));
    }

    #[test]
    fn an_absent_or_empty_array_is_no_objects_not_a_panic() {
        assert!(objects_in(r#"{"a":1}"#, "items").is_empty());
        assert!(objects_in(r#"{"items":[]}"#, "items").is_empty());
        assert!(objects_in(r#"{"items":[{"id":"a""#, "items").is_empty(), "unterminated yields nothing");
    }
}
