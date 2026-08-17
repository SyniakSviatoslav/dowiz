//! Bebop front-end — Phase 1 reference scaffold.
//!
//! NOTE: superseded by the operator's "native, not Rust" directive — the real
//! compiler is a native C bootstrap that self-hosts in Bebop (direct
//! aarch64/x86_64 codegen, no LLVM). This crate is kept as a token/grammar/glyph
//! *reference* for that port.

pub mod ast;
pub mod glyph;
pub mod lexer;
pub mod token;

pub use lexer::lex;

#[cfg(test)]
mod tests {
    use crate::glyph;
    use crate::token::{Kw, TokenKind};

    #[test]
    fn lexer_tokenizes_core_keywords() {
        let src = "module ntt\n\npure fn ntt(a: i64) -> i64 { a + 1 }\n";
        let toks = crate::lex(src).expect("lex");
        let has = |k: Kw| toks.iter().any(|t| matches!(&t.kind, TokenKind::Keyword(x) if *x == k));
        assert!(has(Kw::Module));
        assert!(has(Kw::Pure));
        assert!(has(Kw::Fn));
        assert!(!has(Kw::Const), "no const keyword in this source");
    }

    #[test]
    fn glyph_delta_outline_round_trips_losslessly() {
        let g = glyph::glyph_for("quantum");
        let w = g.grid[0].len();
        let h = g.grid.len();
        let back = glyph::outline_to_grid(&g.outline, w, h);
        assert_eq!(g.grid, back, "δ-outline round-trip must be lossless");
    }

    #[test]
    fn glyph_renders_nonempty() {
        assert!(!glyph::render_braille("quantum").is_empty());
        assert!(!glyph::to_halfblock(&glyph::glyph_for("fn").grid).is_empty());
    }
}
