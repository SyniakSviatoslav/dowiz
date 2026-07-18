//! P57 M1 — Latin + Cyrillic script scope classifier (Lane A, pure, offline-clean).
//!
//! RED→GREEN GATE (per blueprint §4.1): the Wave-0 script scope is
//! **Latin + Cyrillic ONLY**. This module is the *mechanism* that makes
//! "non-Latin scripts deferred to v2" a **tested refusal**, not a prose
//! promise. It classifies **codepoints**, never shaped runs — grapheme
//! clustering is cosmic-text's authority (P57 §1.2 ad-fontes line), this
//! module owns only the v2 scope gate.
//!
//! The boundary is drawn at the **Unicode block**, not the glyph shape: a
//! Cyrillic homoglyph (`а` U+0430) is IN scope, a Greek look-alike
//! (`α` U+03B1) is OUT. Dead-key / precomposed Latin (é, ï, ї) lands
//! inside Latin-1/Extended-A/B. Combining marks (U+0300..U+036F) are IN
//! so composed Latin sequences cluster correctly; control / zero-width
//! codepoints (U+200B, U+0000) are OUT and must never reach the buffer.

/// Wave-0 script scope label — the only scripts P57 edits.
pub const WAVE0_SCRIPTS: &str = "Latin+Cyrillic";

/// True iff `c` is inside Wave-0 editing scope (Latin + Cyrillic + the
/// always-allowed structural set: ASCII space, common punctuation, digits).
///
/// Everything else — CJK, Arabic, Thai, Indic, Greek, emoji, control,
/// zero-width — is **OUT** (v2, §2 anti-scope). The classifier is a hard
/// choke-point: every insert path (Insert / Paste / Ime::Commit / keydown)
/// funnels through [`crate::text_input::in_wave0_scope`] which calls this.
pub fn in_wave0_scope(c: char) -> bool {
    let u = c as u32;
    // ── always-allowed structural set ────────────────────────────────
    // ASCII control range: only TAB + LF are acceptable whitespace; NUL and
    // the rest are rejected (U+0000 is explicitly OutOfScope per §4.1).
    if u < 0x20 {
        return u == 0x09 || u == 0x0A; // \t, \n
    }
    if u == 0x7F {
        return false; // DEL
    }
    // ── Latin ───────────────────────────────────────────────────────
    // Basic Latin
    if u <= 0x007F {
        return true;
    }
    // Latin-1 Supplement (includes precomposed Latin: é è ñ ï … and ¡ ¢ etc.)
    if (0x0080..=0x00FF).contains(&u) {
        return true;
    }
    // Latin Extended-A
    if (0x0100..=0x017F).contains(&u) {
        return true;
    }
    // Latin Extended-B
    if (0x0180..=0x024F).contains(&u) {
        return true;
    }
    // Combining Diacritical Marks — Latin-composable (precomposed preferred,
    // but the marks themselves are in-scope so a base+mark clusters as one).
    if (0x0300..=0x036F).contains(&u) {
        return true;
    }
    // General Punctuation (em/en dash, curly quotes, ellipsis, …) — script-
    // agnostic "common punctuation" the spec lists as in-scope (`—`, `…`).
    // Excludes the zero-width controls (ZWSP/ZWJ/ZWNJ) which are rejected
    // as OutOfScope (§4.1: never inserted).
    if (0x2000..=0x206F).contains(&u) {
        return u != 0x200B && u != 0x200C && u != 0x200D;
    }
    // ── Cyrillic ────────────────────────────────────────────────────
    // Cyrillic
    if (0x0400..=0x04FF).contains(&u) {
        return true;
    }
    // Cyrillic Supplement
    if (0x0500..=0x052F).contains(&u) {
        return true;
    }
    // ── everything else is v2 (CJK, Arabic, Thai, Indic, Greek, emoji,
    //     zero-width, etc.) ─────────────────────────────────────────────
    false
}

/// Convenience: a whole string is in scope iff **every** codepoint is.
/// Used by `Paste` / `set_value` to apply the gate per-codepoint.
pub fn str_in_scope(s: &str) -> bool {
    s.chars().all(in_wave0_scope)
}

#[cfg(test)]
mod tests {
    use super::*;

    // RED→GREEN (§4.1): Latin + Cyrillic + structural all accepted.
    #[test]
    fn scope_accepts_latin_cyrillic() {
        for c in ['a', 'Z', '0', ' ', '—', 'é', 'ï'] {
            assert!(in_wave0_scope(c), "{c:?} must be in Wave-0 scope");
        }
        // Cyrillic first-class, not a fallback.
        for c in ['ё', 'Я', 'ї', 'ґ'] {
            assert!(
                in_wave0_scope(c),
                "{c:?} (U+{:04X}) must be in scope",
                c as u32
            );
        }
    }

    // RED→GREEN (§4.1): v2 scripts explicitly rejected.
    #[test]
    fn scope_rejects_v2_scripts() {
        for c in ['中', 'あ', 'ا', 'ก', '😀'] {
            assert!(
                !in_wave0_scope(c),
                "{c:?} (U+{:04X}) must be OUT of scope",
                c as u32
            );
        }
    }

    // ADVERSARIAL (§4.1 homoglyph trap): the boundary is the BLOCK, not the
    // glyph. Cyrillic `а` (U+0430) is IN; Latin `a` (U+0061) is IN; Greek
    // `α` (U+03B1) is OUT. Asserting the Greek letter is refused pins the
    // line at Unicode blocks, defeating any shape-based bypass.
    #[test]
    fn homoglyph_trap_block_not_glyph() {
        let cyr_a = 'а'; // U+0430 Cyrillic small a
        let lat_a = 'a'; // U+0061 Latin small a
        let grk_a = 'α'; // U+03B1 Greek small alpha
        assert_ne!(cyr_a as u32, lat_a as u32);
        assert!(in_wave0_scope(cyr_a), "Cyrillic homoglyph IN");
        assert!(in_wave0_scope(lat_a), "Latin a IN");
        assert!(
            !in_wave0_scope(grk_a),
            "Greek alpha OUT (block boundary, not glyph)"
        );
    }

    // ADVERSARIAL (§4.1): combining marks in-scope; control + zero-width OUT.
    #[test]
    fn combining_in_control_and_zw_out() {
        let combining_acute = '\u{0301}'; // U+0301 — Latin-composable
        assert!(in_wave0_scope(combining_acute), "combining mark IN");
        let zwsp = '\u{200B}'; // zero-width space
        assert!(!in_wave0_scope(zwsp), "zero-width space OUT");
        let nul = '\u{0000}';
        assert!(!in_wave0_scope(nul), "NUL OUT (never inserted)");
        let del = '\u{007F}';
        assert!(!in_wave0_scope(del), "DEL OUT");
    }

    // ADVERSARIAL: a mixed string is in-scope only if every codepoint is.
    #[test]
    fn mixed_string_scope() {
        assert!(str_in_scope("hello привіт 123"), "all-Latin+Cyrillic IN");
        assert!(!str_in_scope("cafe中文"), "mixed with CJK OUT");
        assert!(
            !str_in_scope("abc\u{200B}def"),
            "zero-width makes whole string OUT"
        );
    }
}
