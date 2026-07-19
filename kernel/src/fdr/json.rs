//! `fdr/json.rs` — the kernel's single JSON *write* authority (blueprint §2).
//!
//! ABSORBS the two escaping primitives that coexisted in the tree before this module
//! (blueprint §2 / synthesis §10-P2 — a live "two different escapers" inconsistency):
//!   1. `markov_attractor::esc()` — escaped `" \ \n \r \t`, everything else verbatim.
//!   2. `span_metrics::obs::LogBucket::to_jsonl` — escaped the span name via Rust's
//!      `{:?}` Debug formatting (a *second*, different escaper: `{:?}` also emits
//!      `\u{..}` for other control chars).
//! Both now route through `escape_into` here; each old call site is pinned
//! byte-identical by a golden test (this file's tests + `markov_attractor`'s golden).
//!
//! Scope: this is the *serialize* side only. Parse-side JSON (`json_api.rs`, the serde
//! carriers) is item 31's scope and is untouched.
//!
//! Pure `std` (String pushes only) — compiles on every target incl. `wasm32`.

/// Escape `s` into `out` using the historical `esc()` semantics: only `" \ \n \r \t`
/// are escaped; every other char (including other control chars) passes through
/// verbatim. This is the ONE escaping authority — byte-compatible with the deleted
/// `esc()` for every string the markov CLI can emit (golden-pinned in
/// `bin/markov_attractor.rs` + here), and byte-compatible with the old `{:?}`
/// span-name escaping for the 8 real span names (all `[a-z_]`, escaping never fires).
pub fn escape_into(out: &mut String, s: &str) {
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
}

/// Escape to a fresh `String` — the drop-in for the deleted `markov_attractor::esc()`.
/// Byte-identical to it by construction (same match arms, same initial capacity).
pub fn escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    escape_into(&mut o, s);
    o
}

/// Escape `s` as a quoted JSON string (`"..."`) into `out`. Used by [`JsonWriter`] and
/// by any call site that needs the surrounding quotes (the old `to_jsonl` got these
/// quotes "for free" from `{:?}` — this reproduces them exactly).
pub fn quote_into(out: &mut String, s: &str) {
    out.push('"');
    escape_into(out, s);
    out.push('"');
}

/// A minimal, allocation-cheap JSON *object* builder. Field methods CANNOT emit an
/// unescaped string byte (illegal-state-unrepresentable applied to serialization,
/// synthesis §1.5): every string routes through [`escape_into`]. Field order is fixed
/// by call order (no map iteration ⇒ deterministic output, mirroring
/// `typed_metrics`'s determinism contract). This is the writer the FDR event schema
/// and (post-cutover) `metric.jsonl`/`alert.jsonl`/the markov CLI all emit through.
pub struct JsonWriter {
    buf: String,
    started: bool,
}

impl JsonWriter {
    /// Begin a JSON object (`{`).
    pub fn obj() -> Self {
        let mut buf = String::new();
        buf.push('{');
        JsonWriter { buf, started: false }
    }

    fn key(&mut self, k: &str) {
        if self.started {
            self.buf.push(',');
        }
        self.started = true;
        quote_into(&mut self.buf, k);
        self.buf.push(':');
    }

    /// `"k":"<escaped v>"`.
    pub fn field_str(mut self, k: &str, v: &str) -> Self {
        self.key(k);
        quote_into(&mut self.buf, v);
        self
    }

    /// `"k":<v>` (unsigned integer).
    pub fn field_u64(mut self, k: &str, v: u64) -> Self {
        self.key(k);
        self.buf.push_str(&v.to_string());
        self
    }

    /// `"k":<v>` (u128 — for ns timestamps).
    pub fn field_u128(mut self, k: &str, v: u128) -> Self {
        self.key(k);
        self.buf.push_str(&v.to_string());
        self
    }

    /// `"k":<raw>` — the caller guarantees `raw` is already valid JSON (a nested object
    /// built by another `JsonWriter`, or a `Reading`'s `{"unavailable":...}` form). The
    /// ONLY method that does not escape, by contract; never pass user text here.
    pub fn field_raw(mut self, k: &str, raw: &str) -> Self {
        self.key(k);
        self.buf.push_str(raw);
        self
    }

    /// Close the object (`}`) and return the finished JSON string.
    pub fn finish(mut self) -> String {
        self.buf.push('}');
        self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pre-absorption `esc()` body, verbatim, as the golden oracle. `escape()` must
    /// equal it byte-for-byte for every input (proves the absorption is lossless).
    fn old_esc(s: &str) -> String {
        let mut o = String::with_capacity(s.len() + 2);
        for c in s.chars() {
            match c {
                '"' => o.push_str("\\\""),
                '\\' => o.push_str("\\\\"),
                '\n' => o.push_str("\\n"),
                '\r' => o.push_str("\\r"),
                '\t' => o.push_str("\\t"),
                _ => o.push(c),
            }
        }
        o
    }

    #[test]
    fn golden_escape_matches_old_esc_byte_for_byte() {
        // The markov CLI's selftest corpus tokens + adversarial escape strings.
        let corpus = [
            "edit",
            "run_ok",
            "run_fail",
            "HEALTHY",
            "analyzer error",
            "loop: edit→run_fail (k=3)",
            "a\"b",              // quote
            "a\\b",              // backslash
            "line1\nline2",      // newline
            "carriage\rreturn",  // CR
            "tab\there",         // tab
            "mix \"\\\n\r\t end",
            "unicode: café → ☕", // non-ascii passes through verbatim (both escapers agree)
            "",
        ];
        for s in corpus {
            assert_eq!(escape(s), old_esc(s), "escape() diverged from old esc() for {s:?}");
        }
    }

    #[test]
    fn golden_jsonwriter_deterministic_field_order() {
        let s = JsonWriter::obj()
            .field_str("metric", "span_latency_us")
            .field_str("span", "place_order")
            .field_u64("count", 2)
            .finish();
        assert_eq!(
            s,
            r#"{"metric":"span_latency_us","span":"place_order","count":2}"#
        );
    }

    #[test]
    fn golden_jsonwriter_escapes_strings() {
        let s = JsonWriter::obj().field_str("k", "a\"b\\c").finish();
        assert_eq!(s, r#"{"k":"a\"b\\c"}"#);
    }
}
