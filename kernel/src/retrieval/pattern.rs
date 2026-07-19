//! `pattern` — kernel-owned restricted wildcard matcher (roadmap item 5).
//!
//! The last external crate in the kernel's default no-dev tree was `regex`,
//! linked by exactly one production seam (`TrigramIndex::query_regex`) that had
//! **zero production callers**. Its full generality (alternation, classes,
//! bounded repetition, Unicode, DFA guarantees) was 100% unused; the only
//! pattern ever compiled anywhere in the repo was `note-.*-recall`. This module
//! replaces that seam for the *actually-used* pattern subset and lets `regex` be
//! removed outright (`cargo tree -e no-dev` → 0 external crates).
//!
//! # Why this exists (dependency-replacement ruling — procedure §2, steps 1–5)
//!
//! Recorded here per `PROCEDURE-DEPENDENCY-REPLACEMENT-STANDING-2026-07-19.md`
//! step 9(i); full walk in `docs/design/BLUEPRINT-ITEM-05-regex-retirement-2026-07-19.md`.
//!
//! 1. **Trigger:** the zero-dependency push (synthesis §0.1; roadmap §B item 5).
//!    Items 4+29 shrank the allowlist `{regex, tracing, tracing-subscriber}` →
//!    `{regex}`; item 5 is the named final shrink `{regex}` → `{}`.
//! 2. **Sweep:** one manifest line (`Cargo.toml` `regex = "1"`), one import, one
//!    production seam (`query_regex`) with **zero production callers**, one
//!    misnamed hand-rolled sibling (`candidate_count_regex`, never touched the
//!    crate), one self-testing test — the crate was its own oracle.
//! 3. **Edge, verified in-house:** what `regex 1.13.1` gave THIS kernel — a
//!    linear-time DFA against pathological patterns and the full RE language,
//!    exercised by nobody. Cost, measured: 5 crates = 100% of the remaining
//!    external no-dev tree. The subset replacement has no pathological cases by
//!    construction (see *Algorithm*).
//! 4. **In-kernel alternative compile-checked BEFORE the flip:** this module
//!    lands complete with a parity suite against the live `regex` crate while it
//!    is still present (commit 1/3); the seam flips only after parity is green
//!    (commit 2/3); the crate is removed last (commit 3/3).
//! 5. **Terminal state (a) — removed outright.** Not opt-in (a feature flag
//!    would preserve a dead API for zero callers); a pattern matcher is not a
//!    syscall/wire/ABI boundary.
//!
//! # Pattern language (a CLOSED contract — degrade-closed)
//!
//! - **literal byte** — matches that exact byte;
//! - **`.`** — matches any single byte;
//! - **`.*`** — matches any gap (the only quantifier, only as this two-byte
//!   token: an unbounded run of any bytes, including empty).
//!
//! Any other metacharacter (`* + ? ( ) [ ] { } | ^ $ \`, and a bare `*` not
//! forming `.*`) is a **loud, typed rejection** at compile time
//! (`PatternError::UnsupportedMeta { byte, pos }`) — parse-then-match, never a
//! silent wrong answer (`arena.rs` degrade-closed discipline).
//!
//! # Semantics
//!
//! Unanchored **contains-match**, bit-identical to `regex::Regex::is_match`
//! restricted to this subset **over the module's domain** (ASCII note names, no
//! newlines — `fixtures.rs`). ONE deliberate divergence from `regex`'s default:
//! this matcher's `.` matches any single byte *including* `\n`, whereas
//! `regex`'s default `.` excludes `\n`. The two coincide exactly on newline-free
//! input, which is the entire real domain (note names never contain newlines);
//! the parity suite therefore proves equality over newline-free ASCII corpora,
//! and the property is asserted (not assumed) by the differential tests below.
//!
//! # Algorithm (no backtracking exists ⇒ no pathological blowup)
//!
//! The pattern splits into fixed-length *segments* at `.*` boundaries (each
//! segment is a run of `Lit`/`Any` atoms; leading/trailing/adjacent `.*` yield
//! empty boundary segments, which are match-neutral for a contains-match). A
//! match is a greedy left-to-right leftmost placement of each segment at-or-after
//! the previous segment's end. Correctness rests on the classic glob lemma: with
//! no nested quantifiers and every gap unbounded, placing each segment as early
//! as possible never precludes a later match (exchange argument), so greedy
//! leftmost is exact and **no backtracking is possible**. Worst case is
//! O(|doc|·|pattern|) per candidate doc — trivial after trigram narrowing.
//!
//! # Reopening trigger (step 10)
//!
//! A **real production caller** (not a test, not "might be handy") needing
//! pattern features beyond {literal, `.`, `.*`} — alternation, classes, bounded
//! repetition, Unicode. Resolution then chooses extend-the-subset vs re-adopt
//! `regex` as opt-in state (b), through the same procedure. Nothing else reopens
//! it.

use std::fmt;

/// A pattern used a metacharacter outside the supported subset
/// {literal byte, `.`, `.*`}. Degrade-closed: the matcher NEVER guesses a
/// meaning for an unsupported token — it rejects at compile time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternError {
    /// An unsupported metacharacter `byte` appeared at byte offset `pos`.
    UnsupportedMeta { byte: u8, pos: usize },
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatternError::UnsupportedMeta { byte, pos } => write!(
                f,
                "unsupported pattern metacharacter {:?} (0x{:02x}) at byte {}; \
                 supported subset is {{literal, '.', '.*'}}",
                *byte as char, byte, pos
            ),
        }
    }
}

impl std::error::Error for PatternError {}

/// A single fixed-width match atom within a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Atom {
    /// A literal byte that must match exactly.
    Lit(u8),
    /// `.` — matches any single byte (see module doc on the newline caveat).
    Any,
}

/// A compiled restricted pattern: fixed-length segments separated by `.*` gaps.
/// `is_match` is an UNANCHORED contains-match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    /// Fixed-length segments in order; adjacent segments are joined by an
    /// unbounded `.*` gap. Leading/trailing/adjacent `.*` produce empty boundary
    /// segments, which impose no constraint for a contains-match.
    segments: Vec<Vec<Atom>>,
}

impl Pattern {
    /// Parse `pattern` into segments, rejecting any unsupported metacharacter.
    /// Byte-oriented, to match the byte-level trigram index (ASCII note names by
    /// module contract).
    pub fn compile(pattern: &str) -> Result<Pattern, PatternError> {
        let b = pattern.as_bytes();
        let mut segments: Vec<Vec<Atom>> = Vec::new();
        let mut cur: Vec<Atom> = Vec::new();
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                // `.` immediately followed by `*` is the gap token `.*`;
                // otherwise a single any-byte atom.
                b'.' => {
                    if i + 1 < b.len() && b[i + 1] == b'*' {
                        segments.push(std::mem::take(&mut cur));
                        i += 2;
                    } else {
                        cur.push(Atom::Any);
                        i += 1;
                    }
                }
                // Every other metacharacter — including a bare `*` (only `.*` is
                // allowed) — is a hard, typed rejection.
                b'*' | b'+' | b'?' | b'(' | b')' | b'[' | b']' | b'{' | b'}' | b'|' | b'^'
                | b'$' | b'\\' => {
                    return Err(PatternError::UnsupportedMeta { byte: b[i], pos: i });
                }
                c => {
                    cur.push(Atom::Lit(c));
                    i += 1;
                }
            }
        }
        segments.push(cur);
        Ok(Pattern { segments })
    }

    /// UNANCHORED contains-match: does `doc` contain a substring matched by this
    /// pattern? Greedy leftmost placement of each segment after the previous
    /// segment's end. No backtracking exists (the only quantifier is the
    /// unbounded `.*` gap and segments are fixed-length).
    pub fn is_match(&self, doc: &str) -> bool {
        let hay = doc.as_bytes();
        // Leftmost feasible start for the next segment. The unanchored head lets
        // the first segment start anywhere; the unanchored tail lets the last
        // segment end anywhere; each `.*` gap allows any run (including empty)
        // between adjacent segments.
        let mut from = 0usize;
        for seg in &self.segments {
            if seg.is_empty() {
                // Empty boundary segment (leading/trailing/adjacent `.*`) — no
                // constraint; `from` is unchanged.
                continue;
            }
            match find_segment(hay, seg, from) {
                Some(end) => from = end,
                None => return false,
            }
        }
        true
    }
}

/// Leftmost occurrence of the fixed-length `seg` in `hay[from..]`; returns the
/// index one past the match end, or `None`. `Lit` atoms match exactly, `Any`
/// atoms match any byte. `seg` is non-empty (callers skip empty segments).
fn find_segment(hay: &[u8], seg: &[Atom], from: usize) -> Option<usize> {
    let n = seg.len();
    // `None` when the segment is longer than what remains anywhere in `hay`.
    let last_start = hay.len().checked_sub(n)?;
    let mut start = from;
    while start <= last_start {
        let hit = seg
            .iter()
            .zip(&hay[start..start + n])
            .all(|(atom, &c)| match atom {
                Atom::Lit(l) => c == *l,
                Atom::Any => true,
            });
        if hit {
            return Some(start + n);
        }
        start += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{Pattern, PatternError};
    use crate::retrieval::fixtures::{synthetic_corpus, FIXTURE};
    use crate::retrieval::index::TrigramIndex;

    /// Pattern battery covering the used subset shapes: empty, gap-only, double
    /// gap, literal (with/without trigrams), single/double any-byte, holes,
    /// leading/trailing/multiple gaps, absent literal, synthetic-corpus hitters,
    /// and the one historical pattern.
    const BATTERY: &[&str] = &[
        "",
        ".*",
        ".*.*",
        "note",
        "md", // <3 bytes ⇒ no-trigram fallback (all-docs scan) path
        ".",
        "..",
        "n.te",
        "note-.*-recall", // the only pattern ever compiled in the repo
        ".*recall",
        "note.*",
        "note-.*-.*-recall",
        "n.*l",
        "zzz", // absent from every fixture doc ⇒ matches nothing
        "boilerplate",
        "prefix-.*-suffix",
        ".uffix",
        "a.*z",
        "-",
        "MEMORY",
    ];

    // ── Independent reference matcher (permanent — survives regex removal) ─────
    //
    // A recursive-descent matcher with backtracking on `.*`, over the SAME
    // subset. Deliberately a DIFFERENT algorithm from the production greedy
    // segment matcher, so agreement is a real cross-check (it also empirically
    // validates the "greedy leftmost is exact" claim against a backtracking
    // oracle). Its exponential worst case is irrelevant on tiny test inputs.

    enum Tok {
        Lit(u8),
        Any,
        Star,
    }

    fn naive_tokenize(pattern: &str) -> Result<Vec<Tok>, PatternError> {
        let b = pattern.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'.' => {
                    if i + 1 < b.len() && b[i + 1] == b'*' {
                        out.push(Tok::Star);
                        i += 2;
                    } else {
                        out.push(Tok::Any);
                        i += 1;
                    }
                }
                b'*' | b'+' | b'?' | b'(' | b')' | b'[' | b']' | b'{' | b'}' | b'|' | b'^'
                | b'$' | b'\\' => {
                    return Err(PatternError::UnsupportedMeta { byte: b[i], pos: i });
                }
                c => {
                    out.push(Tok::Lit(c));
                    i += 1;
                }
            }
        }
        Ok(out)
    }

    fn naive_ref_is_match(pattern: &str, doc: &str) -> Result<bool, PatternError> {
        let toks = naive_tokenize(pattern)?;
        let hay = doc.as_bytes();
        // Unanchored head: try to anchor-match starting at each position.
        for start in 0..=hay.len() {
            if naive_go(&toks, 0, hay, start) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn naive_go(toks: &[Tok], ti: usize, hay: &[u8], hi: usize) -> bool {
        // Unanchored tail: consuming all tokens is a match regardless of what
        // remains in `hay`.
        if ti == toks.len() {
            return true;
        }
        match toks[ti] {
            Tok::Lit(c) => hi < hay.len() && hay[hi] == c && naive_go(toks, ti + 1, hay, hi + 1),
            Tok::Any => hi < hay.len() && naive_go(toks, ti + 1, hay, hi + 1),
            Tok::Star => {
                // `.*` matches any run: try consuming 0..=remaining bytes.
                for k in hi..=hay.len() {
                    if naive_go(toks, ti + 1, hay, k) {
                        return true;
                    }
                }
                false
            }
        }
    }

    // ── Rejection: every unsupported metacharacter → typed Err with position ──

    #[test]
    fn rejects_unsupported_metacharacters_with_position() {
        let cases: &[(&str, u8, usize)] = &[
            ("a+b", b'+', 1),
            ("a?b", b'?', 1),
            ("(ab)", b'(', 0),
            ("a|b", b'|', 1),
            ("[ab]", b'[', 0),
            ("a{2}", b'{', 1),
            ("^ab", b'^', 0),
            ("ab$", b'$', 2),
            ("a\\.b", b'\\', 1),
            ("a*", b'*', 1), // bare `*` (only `.*` is allowed)
            ("*", b'*', 0),
            ("]", b']', 0),
            ("}", b'}', 0),
            (")", b')', 0),
        ];
        for (pat, byte, pos) in cases {
            match Pattern::compile(pat) {
                Err(PatternError::UnsupportedMeta { byte: b, pos: p }) => {
                    assert_eq!(b, *byte, "pattern {:?}: wrong byte", pat);
                    assert_eq!(p, *pos, "pattern {:?}: wrong pos", pat);
                }
                other => panic!("pattern {:?} must be rejected, got {:?}", pat, other),
            }
            // The reference tokenizer rejects the same inputs identically.
            assert!(naive_tokenize(pat).is_err(), "reference must reject {:?}", pat);
        }
        // The seam surfaces the same typed error.
        let idx = TrigramIndex::new(FIXTURE);
        assert!(matches!(
            idx.query_pattern("a+b"),
            Err(PatternError::UnsupportedMeta { .. })
        ));
    }

    #[test]
    fn supported_subset_always_compiles() {
        for p in BATTERY {
            assert!(Pattern::compile(p).is_ok(), "battery pattern {:?} must compile", p);
        }
    }

    // ── Permanent differential: hand-rolled vs the independent naive reference ─

    #[test]
    fn hand_rolled_agrees_with_naive_reference_on_battery_and_fixture() {
        let mut pairs = 0u64;
        for p in BATTERY {
            let compiled = Pattern::compile(p).expect("battery compiles");
            for d in FIXTURE {
                let hr = compiled.is_match(d);
                let nv = naive_ref_is_match(p, d).expect("battery compiles");
                assert_eq!(hr, nv, "pattern={:?} doc={:?}", p, d);
                pairs += 1;
            }
        }
        assert_eq!(pairs, BATTERY.len() as u64 * FIXTURE.len() as u64);
    }

    #[test]
    fn hand_rolled_agrees_with_naive_reference_deterministic_sweep() {
        // Deterministic pseudo-random sweep (seeded xorshift64*, no dev-dep
        // needed) of subset patterns × ASCII docs — permanent, regex-free.
        let mut state: u64 = 0x1234_5678_9abc_def0;
        let mut rng = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let alpha: &[u8] = b"ab-.";
        let mut checked = 0u64;
        for _ in 0..4000 {
            // Build a subset pattern from {literal a/b/-, '.', '.*'}.
            let plen = (rng() % 7) as usize;
            let mut pat = String::new();
            for _ in 0..plen {
                match rng() % 5 {
                    0 => pat.push('a'),
                    1 => pat.push('b'),
                    2 => pat.push('-'),
                    3 => pat.push('.'),
                    _ => pat.push_str(".*"),
                }
            }
            // Guard: a trailing lone '.' that becomes '.<end>' is fine; a bare
            // '*' can never be produced by this generator (only ".*" adds '*').
            let compiled = match Pattern::compile(&pat) {
                Ok(c) => c,
                Err(_) => continue, // generator never emits an unsupported meta
            };
            let dlen = (rng() % 12) as usize;
            let mut doc = String::new();
            for _ in 0..dlen {
                doc.push(alpha[(rng() % 3) as usize] as char); // a/b/- only in docs
            }
            let hr = compiled.is_match(&doc);
            let nv = naive_ref_is_match(&pat, &doc).expect("subset compiles");
            assert_eq!(hr, nv, "pattern={:?} doc={:?}", pat, doc);
            checked += 1;
        }
        assert!(checked > 3000, "sweep must exercise most iterations (got {checked})");
    }

    // ── Frozen golden: the incumbent's verdict on the historical pattern ──────

    #[test]
    fn golden_seam_verdict_frozen_from_regex_incumbent() {
        // `note-.*-recall` matched exactly doc 7 ("note-heat-kernel-recall.md")
        // under the retired `regex` crate. Frozen here as the permanent golden
        // (items-4+29 "byte-compatible" analogue): the hand-rolled seam must
        // reproduce the incumbent's verdict forever.
        let idx = TrigramIndex::new(FIXTURE);
        assert_eq!(idx.query_pattern("note-.*-recall").unwrap(), vec![7]);
        // 0 false positives at the seam over the whole fixture.
        let compiled = Pattern::compile("note-.*-recall").unwrap();
        let oracle: Vec<u32> = FIXTURE
            .iter()
            .enumerate()
            .filter(|(_, d)| compiled.is_match(d))
            .map(|(i, _)| i as u32)
            .collect();
        assert_eq!(idx.query_pattern("note-.*-recall").unwrap(), oracle);
    }

    #[test]
    fn seam_query_pattern_equals_linear_matcher_oracle_over_synthetic() {
        // query_pattern (trigram-narrowed + verify) must equal a brute-force
        // linear scan with the SAME matcher over a 2000-doc corpus, for every
        // battery pattern (proves trigram narrowing never drops a true match).
        let corpus = synthetic_corpus(2000);
        let docs: Vec<&str> = corpus.iter().map(|s| s.as_str()).collect();
        let idx = TrigramIndex::new(&docs);
        for p in BATTERY {
            let compiled = Pattern::compile(p).unwrap();
            let oracle: Vec<u32> = docs
                .iter()
                .enumerate()
                .filter(|(_, d)| compiled.is_match(d))
                .map(|(i, _)| i as u32)
                .collect();
            assert_eq!(idx.query_pattern(p).unwrap(), oracle, "pattern {:?}", p);
        }
    }

    // ── Post-removal differential: hand-rolled vs the independent naive
    //    reference (proptest). Before commit 3/3 an identical battery ran against
    //    the live `regex` crate (bit-identical over the newline-free ASCII
    //    domain); that oracle retired WITH the crate, its verdicts preserved by
    //    the frozen golden above. Two independent implementations must agree
    //    forever.

    proptest::proptest! {
        /// Property-based differential vs the independent naive reference — the
        /// permanent post-removal cross-check (kept when the regex block is cut).
        #[test]
        fn prop_parity_vs_naive_reference(
            pat in subset_pattern_strategy(),
            doc in "[a-z0-9-]{0,40}",
        ) {
            let compiled = Pattern::compile(&pat).unwrap();
            let nv = naive_ref_is_match(&pat, &doc).unwrap();
            proptest::prop_assert_eq!(compiled.is_match(&doc), nv);
        }
    }

    /// Strategy for a valid subset pattern: 0..=6 tokens, each a single literal
    /// char from `[a-z0-9-]`, `.`, or `.*`. Never emits a bare `*` or any
    /// unsupported metacharacter, so `Pattern::compile` always succeeds and only
    /// `.`/`.*` are interpreted specially.
    fn subset_pattern_strategy() -> impl proptest::strategy::Strategy<Value = String> {
        use proptest::prelude::*;
        prop::collection::vec(
            prop_oneof![
                "[a-z0-9-]".prop_map(|s| s),
                Just(".".to_string()),
                Just(".*".to_string()),
            ],
            0..=6,
        )
        .prop_map(|toks| toks.concat())
    }
}
