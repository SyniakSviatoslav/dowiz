//! `readability` — native, pure-`std` readable-text extraction from raw HTML.
//!
//! The kernel-side half of the P40 `ToolResource::WebFetch` tool (see
//! `ports::tool`). Reimplements the core mechanism of Mozilla's Readability.js
//! algorithm natively — a deterministic heuristic scoring pass, not a machine-
//! learning model — in the same spirit as `retrieval::pattern` (native regex
//! substitute) and `json` (native serde substitute): pure computation, zero I/O,
//! zero external crates, differential-tested against known-shape fixtures rather
//! than a live browser.
//!
//! # Scope, stated honestly (see the native-agentic-browsing research this
//! module implements the recommended half of)
//! This is a **static HTML text extractor**, not a browser. It does not execute
//! JavaScript, does not build a CSS-aware layout, and cannot see content a page
//! only renders client-side. That is a deliberate, permanent scope boundary —
//! full interactive/JS-driven browsing is categorically different work (the
//! research's honest comparison: building it natively is "build a browser
//! engine," the Servo-scale undertaking) and stays an external tool (e.g.
//! `agent-browser`) behind its own port, never reimplemented here. This module
//! only ever consumes bytes already fetched by the caller (`agent-facade`'s
//! `WebFetchTool`, over `ureq`) — no network code lives in this crate.
//!
//! # Algorithm (deliberately simplified from Readability.js, not a faithful port)
//! 1. Tokenize: a single forward scan splits the byte stream into open/close/
//!    self-closing tags, attribute `class`/`id` values, and text runs, decoding
//!    the handful of HTML entities real pages actually use. Malformed markup is
//!    tolerated best-effort (never panics — see [`extract`]'s fail-open contract)
//!    rather than rejected; this is not a spec-compliant HTML5 parser.
//! 2. Boilerplate exclusion: text inside `script`/`style`/`noscript`/`nav`/
//!    `header`/`footer`/`aside`/`form`/`button`/`select`/`svg` never reaches any
//!    candidate block — the cheap, robust half of Readability's "strip unlikely
//!    nodes" step.
//! 3. Candidate scoring: every open block-level container (`div`/`article`/
//!    `section`/`main`/`p`/`td`/`li`/`blockquote`/`body`) accumulates the text of
//!    everything nested inside it (so an outer `<article>` naturally scores the
//!    combined text of its inner `<p>`s — the propagate-to-parent effect,
//!    achieved by concurrent accumulation rather than a second tree-walk pass).
//!    Each candidate's score = length bonus (capped) + comma-count bonus (capped)
//!    + class/id keyword bonus/penalty − link-density penalty.
//! 4. The highest-scoring candidate's accumulated text, whitespace-normalized, is
//!    the extracted result.

/// A single candidate content block being scored.
struct Candidate {
    /// Byte offset this block's open tag started at — used only as a stable,
    /// deterministic tie-breaker (earliest wins), never exposed.
    start: usize,
    text: String,
    link_text_len: usize,
    class_id: String,
}

/// Tags whose CONTENTS are candidate-worthy container text (accumulate their
/// descendants' text as one scoring unit).
fn is_candidate_tag(tag: &str) -> bool {
    matches!(
        tag,
        "div" | "article" | "section" | "main" | "p" | "td" | "li" | "blockquote" | "body"
    )
}

/// Tags whose contents are boilerplate and must never reach any candidate's
/// accumulated text (Readability's "strip unlikely nodes", the robust half).
fn is_excluded_tag(tag: &str) -> bool {
    matches!(
        tag,
        "script"
            | "style"
            | "noscript"
            | "nav"
            | "header"
            | "footer"
            | "aside"
            | "form"
            | "button"
            | "select"
            | "svg"
            | "template"
    )
}

const POSITIVE_KEYWORDS: &[&str] = &[
    "article", "content", "main", "post", "body", "entry", "text", "story",
];
const NEGATIVE_KEYWORDS: &[&str] = &[
    "nav", "footer", "sidebar", "comment", "ad", "share", "social", "related", "widget", "menu",
    "header", "banner", "promo", "popup", "cookie",
];

/// Bounded length bonus: 1 point per 100 chars, capped at 30 (a ~3000-char block
/// already reads as "clearly the content" — no benefit to unbounded growth).
const LENGTH_BONUS_CAP: i64 = 30;
/// Bounded comma bonus, same rationale (commas correlate with prose, not markup).
const COMMA_BONUS_CAP: i64 = 20;

fn keyword_score(class_id: &str) -> i64 {
    let lower = class_id.to_ascii_lowercase();
    let mut score = 0i64;
    for kw in POSITIVE_KEYWORDS {
        if lower.contains(kw) {
            score += 25;
        }
    }
    for kw in NEGATIVE_KEYWORDS {
        if lower.contains(kw) {
            score -= 25;
        }
    }
    score
}

fn score_candidate(c: &Candidate) -> i64 {
    let len = c.text.chars().count() as i64;
    let length_bonus = (len / 100).min(LENGTH_BONUS_CAP);
    let comma_bonus = (c.text.matches(',').count() as i64).min(COMMA_BONUS_CAP);
    let link_density = if len == 0 {
        0.0
    } else {
        c.link_text_len as f64 / len as f64
    };
    // Link-heavy blocks (nav-like, even if not caught by tag exclusion) are
    // penalized proportionally — a block that's 80% link text loses most of
    // its length bonus.
    let link_penalty = ((length_bonus as f64) * link_density * 0.8) as i64;
    length_bonus + comma_bonus + keyword_score(&c.class_id) - link_penalty
}

/// Decode the small, fixed set of HTML entities real page content actually uses.
/// Unknown/unhandled entities pass through verbatim (fail-open — never a panic,
/// never a dropped byte).
fn decode_entities(raw: &str) -> String {
    if !raw.as_bytes().contains(&b'&') {
        return raw.to_string();
    }
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        if ch != '&' {
            out.push(ch);
            continue;
        }
        let rest = &raw[i..];
        let (replacement, skip) = if let Some(r) = rest.strip_prefix("&amp;") {
            ('&', rest.len() - r.len())
        } else if let Some(r) = rest.strip_prefix("&lt;") {
            ('<', rest.len() - r.len())
        } else if let Some(r) = rest.strip_prefix("&gt;") {
            ('>', rest.len() - r.len())
        } else if let Some(r) = rest.strip_prefix("&quot;") {
            ('"', rest.len() - r.len())
        } else if let Some(r) = rest.strip_prefix("&#39;") {
            ('\'', rest.len() - r.len())
        } else if let Some(r) = rest.strip_prefix("&apos;") {
            ('\'', rest.len() - r.len())
        } else if let Some(r) = rest.strip_prefix("&nbsp;") {
            (' ', rest.len() - r.len())
        } else if let Some(r) = rest.strip_prefix("&mdash;") {
            ('\u{2014}', rest.len() - r.len())
        } else if let Some(r) = rest.strip_prefix("&ndash;") {
            ('\u{2013}', rest.len() - r.len())
        } else if let Some(r) = rest.strip_prefix("&#8217;") {
            ('\u{2019}', rest.len() - r.len())
        } else {
            out.push('&');
            continue;
        };
        out.push(replacement);
        for _ in 1..skip {
            chars.next();
        }
    }
    out
}

/// Extract a tag's `class`/`id` attribute values (lowercased, space-joined) and
/// whether it is a self-closing / void tag, from the raw attribute substring
/// between the tag name and the closing `>`.
fn parse_attrs(attrs_raw: &str) -> (String, bool) {
    let self_closing = attrs_raw.trim_end().ends_with('/');
    let mut class_id = String::new();
    // Deliberately simple key="value" / key='value' scanner — sufficient for
    // class/id extraction on real-world markup; not a full attribute grammar.
    let bytes = attrs_raw.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        // Find next attribute name start.
        while i < bytes.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        let name_start = i;
        while i < bytes.len() && bytes[i] != b'=' && !(bytes[i] as char).is_whitespace() {
            i += 1;
        }
        let name = &attrs_raw[name_start..i.min(attrs_raw.len())];
        while i < bytes.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && (bytes[i] as char).is_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let quote = bytes[i];
                i += 1;
                let val_start = i;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                let val = &attrs_raw[val_start..i.min(attrs_raw.len())];
                if name.eq_ignore_ascii_case("class") || name.eq_ignore_ascii_case("id") {
                    if !class_id.is_empty() {
                        class_id.push(' ');
                    }
                    class_id.push_str(val);
                }
                i += 1; // skip closing quote
            }
        } else {
            i += 1;
        }
    }
    (class_id, self_closing)
}

/// Extract the readable text from raw HTML bytes. Fail-open: malformed HTML
/// never panics — worst case, the whole document is treated as one candidate
/// and its raw text (post-entity-decode, boilerplate-excluded) is returned.
/// Empty/whitespace-only input returns an empty string, never an error — this
/// is a best-effort extraction primitive, not a validating parser.
pub fn extract(html: &str) -> String {
    let bytes = html.as_bytes();
    let mut i = 0usize;
    let mut candidates: Vec<Candidate> = Vec::new();
    // Stack of currently-open candidate blocks (index into `candidates`).
    let mut open_stack: Vec<usize> = Vec::new();
    // Depth of currently-open excluded (boilerplate) tags — while > 0, no text
    // reaches any candidate.
    let mut excluded_depth: usize = 0;
    // Depth of currently-open <a> tags — while > 0, text also counts as link text.
    let mut anchor_depth: usize = 0;
    // Stack of excluded tag names, so a same-named nested open doesn't
    // prematurely close the exclusion on the inner tag's close.
    let mut excluded_tags: Vec<String> = Vec::new();

    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Skip comments <!-- ... -->.
            if html[i..].starts_with("<!--") {
                if let Some(end) = html[i..].find("-->") {
                    i += end + 3;
                } else {
                    break; // unterminated comment — stop, fail-open on what we have
                }
                continue;
            }
            // Skip doctype / processing instructions.
            if html[i..].starts_with("<!") || html[i..].starts_with("<?") {
                if let Some(end) = html[i..].find('>') {
                    i += end + 1;
                } else {
                    break;
                }
                continue;
            }
            let is_close = i + 1 < bytes.len() && bytes[i + 1] == b'/';
            let tag_start = if is_close { i + 2 } else { i + 1 };
            let mut j = tag_start;
            while j < bytes.len() && (bytes[j] as char).is_ascii_alphanumeric() {
                j += 1;
            }
            if j == tag_start {
                // Not a recognizable tag (e.g. a bare '<' in text) — treat as text char.
                if excluded_depth == 0 {
                    push_text(&mut candidates, &open_stack, "<", anchor_depth > 0);
                }
                i += 1;
                continue;
            }
            let tag_name = html[tag_start..j].to_ascii_lowercase();
            let Some(close_gt) = html[j..].find('>') else {
                break; // unterminated tag — fail-open, stop here
            };
            let attrs_raw = &html[j..j + close_gt];
            let (class_id, attr_self_closing) = parse_attrs(attrs_raw);
            let self_closing = attr_self_closing || attrs_raw.trim_end().ends_with('/');
            let advance = j + close_gt + 1;

            if is_close {
                if is_excluded_tag(&tag_name) {
                    if excluded_tags.last().map(|s| s.as_str()) == Some(tag_name.as_str()) {
                        excluded_tags.pop();
                        excluded_depth = excluded_depth.saturating_sub(1);
                    }
                } else if tag_name == "a" {
                    anchor_depth = anchor_depth.saturating_sub(1);
                } else if is_candidate_tag(&tag_name) {
                    // The candidate was already accumulating text in-place (it's
                    // in `candidates`, and `push_text` writes to it via
                    // `open_stack` while open) — closing it just means it stops
                    // receiving further text. Popping the WRONG index (a
                    // mismatched close tag under malformed HTML) is tolerated:
                    // pop whatever's on top, fail-open, never panic.
                    open_stack.pop();
                }
            } else {
                if is_excluded_tag(&tag_name) && !self_closing {
                    excluded_tags.push(tag_name.clone());
                    excluded_depth += 1;
                } else if tag_name == "a" && !self_closing {
                    anchor_depth += 1;
                } else if is_candidate_tag(&tag_name) && !self_closing && excluded_depth == 0 {
                    open_stack.push(open_slots_push(&mut candidates, i, &class_id));
                } else if tag_name == "br" || tag_name == "p" {
                    // <br> (and a bare self-closed <p/>) contributes a soft
                    // paragraph break so extracted text doesn't run words
                    // together across block boundaries.
                    if excluded_depth == 0 {
                        push_text(&mut candidates, &open_stack, "\n", false);
                    }
                }
            }
            i = advance;
            continue;
        }
        // Text run up to the next '<'.
        let text_end = html[i..].find('<').map(|p| i + p).unwrap_or(bytes.len());
        if excluded_depth == 0 && text_end > i {
            let raw = &html[i..text_end];
            let decoded = decode_entities(raw);
            push_text(&mut candidates, &open_stack, &decoded, anchor_depth > 0);
        }
        i = text_end;
    }
    // Every candidate in `candidates` already holds its complete accumulated
    // text regardless of whether its tag was ever properly closed (malformed/
    // truncated HTML included) — accumulation happened synchronously via
    // `push_text` for as long as the block's index stayed on `open_stack`.
    // Nothing further to finalize; just score them all.
    let mut best: Option<&Candidate> = None;
    let mut best_score = i64::MIN;
    for c in &candidates {
        let s = score_candidate(c);
        if s > best_score || (s == best_score && best.map(|b| c.start < b.start).unwrap_or(true)) {
            best_score = s;
            best = Some(c);
        }
    }
    match best {
        Some(c) => normalize_whitespace(&c.text),
        None => normalize_whitespace(html),
    }
}

// --- internal helpers kept below `extract` for readability of the main scan ---

fn open_slots_push(candidates: &mut Vec<Candidate>, start: usize, class_id: &str) -> usize {
    candidates.push(Candidate {
        start,
        text: String::new(),
        link_text_len: 0,
        class_id: class_id.to_string(),
    });
    candidates.len() - 1
}

fn push_text(candidates: &mut [Candidate], open_stack: &[usize], text: &str, is_link: bool) {
    for &idx in open_stack {
        candidates[idx].text.push_str(text);
        if is_link {
            candidates[idx].link_text_len += text.chars().count();
        }
    }
}

fn normalize_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_was_space = true; // trim leading whitespace
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                out.push(' ');
            }
            last_was_space = true;
        } else {
            out.push(ch);
            last_was_space = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_article_over_nav_and_footer() {
        let html = r#"
            <html><body>
              <nav class="site-nav"><a href="/">Home</a><a href="/about">About</a></nav>
              <header class="banner">Site Name</header>
              <article class="post-content">
                <p>This is the real article text, long enough to score well because it
                has multiple sentences, some commas, and no link-heavy boilerplate around
                it, unlike the navigation and footer sections nearby.</p>
                <p>A second paragraph continues the real content, adding more length and
                more commas, so the scoring clearly favors this block over the chrome.</p>
              </article>
              <footer class="site-footer">
                <a href="/privacy">Privacy</a> <a href="/terms">Terms</a> <a href="/x">X</a>
              </footer>
            </body></html>
        "#;
        let extracted = extract(html);
        assert!(extracted.contains("real article text"));
        assert!(extracted.contains("second paragraph"));
        assert!(!extracted.contains("Privacy"));
        assert!(!extracted.contains("Home"));
    }

    #[test]
    fn strips_script_and_style_content_entirely() {
        let html = r#"<div class="content"><script>var x = "never extracted";</script>
            <style>.foo { color: red; }</style>
            <p>Actual paragraph text that should survive, with enough length and, commas,
            to be picked as the winning candidate over any empty or script-only block.</p>
            </div>"#;
        let extracted = extract(html);
        assert!(!extracted.contains("never extracted"));
        assert!(!extracted.contains("color: red"));
        assert!(extracted.contains("Actual paragraph text"));
    }

    #[test]
    fn decodes_common_entities() {
        let html = "<p>Tom &amp; Jerry said &quot;hello&quot; &mdash; it&#39;s fine, right, \
                     with enough padding text and, commas, to win the scoring pass here.</p>";
        let extracted = extract(html);
        assert!(extracted.contains("Tom & Jerry"));
        assert!(extracted.contains("\"hello\""));
        assert!(extracted.contains("it's fine"));
    }

    #[test]
    fn link_heavy_block_loses_to_prose_block() {
        let html = r#"
            <div class="link-list">
              <a href="/1">Link one</a> <a href="/2">Link two</a> <a href="/3">Link three</a>
              <a href="/4">Link four</a> <a href="/5">Link five</a> <a href="/6">Link six</a>
            </div>
            <div class="content">
              <p>Real prose here, with enough characters and, commas, and sentences to
              clearly outscore a block that is almost entirely link text, per the link
              density penalty this extractor applies during scoring.</p>
            </div>
        "#;
        let extracted = extract(html);
        assert!(extracted.contains("Real prose here"));
        assert!(!extracted.contains("Link one"));
    }

    #[test]
    fn empty_input_returns_empty_string() {
        assert_eq!(extract(""), "");
        assert_eq!(extract("   \n\t  "), "");
    }

    #[test]
    fn malformed_html_never_panics() {
        // Unterminated tags, stray '<', mismatched close tags — all fail-open.
        let inputs = [
            "<div><p>unterminated",
            "<<<not a real tag>>>",
            "<div class=\"x\">text</span></div>",
            "<p>no closing tag at all",
            "plain text with no tags whatsoever, just words, and, commas.",
            "<a href=\"http://x\">",
        ];
        for html in inputs {
            let _ = extract(html); // must not panic
        }
    }

    #[test]
    fn whitespace_is_normalized() {
        let html = "<p>line one\n\n   line   two\t\tline three, with commas, for scoring.</p>";
        let extracted = extract(html);
        assert!(!extracted.contains("  ")); // no double spaces
        assert!(extracted.contains("line one line two line three"));
    }

    #[test]
    fn cover_extract_empty() {
        let _ = super::extract("");
    }

    #[test]
    fn cover_extract_short() {
        let _ = super::extract("a");
    }

    #[test]
    fn cover_extract_text() {
        let _ = super::extract("The quick brown fox jumps over the lazy dog.");
    }

    #[test]
    fn cover_extract_html() {
        let r = super::extract("<html><body><p>Hello world.</p></body></html>"); assert!(!r.is_empty());
    }

    #[test]
    fn cover_extract_long() {
        let r = super::extract("A very long text with multiple sentences. Second sentence here. Third one. Fourth sentence goes here. Fifth one for good measure."); assert!(!r.is_empty());
    }

    #[test]
    fn cover_extract_only_html() {
        let r = super::extract("<div><p>text</p></div>"); assert!(!r.is_empty());
    }

    #[test]
    fn cover_extract_entities() {
        let r = super::extract("&amp; &lt; &gt; &quot; text"); assert!(!r.is_empty());
    }

    #[test]
    fn cover_extract_body_only() {
        let r = super::extract("<body>Main content here.</body>"); assert!(!r.is_empty());
    }

    // ── Edge case: empty input returns empty (already covered, but test the normalize path) ──
    #[test]
    fn extract_all_whitespace_html() {
        assert_eq!(extract("\n\t  \r\n"), "");
        assert_eq!(extract("<div>   </div>"), "");
    }

    // ── Edge case: single character ──
    #[test]
    fn extract_single_character() {
        let r = extract("X");
        assert_eq!(r, "X");
    }

    // ── Edge case: single character in candidate tag ──
    #[test]
    fn extract_single_char_in_tag() {
        let r = extract("<p>X</p>");
        assert_eq!(r, "X");
    }

    // ── Edge case: very long block (stress length bonus cap) ──
    #[test]
    fn extract_very_long_content() {
        let body = "word ".repeat(5000);
        let html = format!("<article>{}</article>", body);
        let r = extract(&html);
        assert!(r.len() > 1000);
        assert!(r.contains("word"));
    }

    // ── Edge case: non-ASCII Unicode text ──
    #[test]
    fn extract_non_ascii_preserved() {
        let html = "<p>Съешь ещё этих мягких булок, да выпей чаю! And, more, commas, here.</p>";
        let r = extract(html);
        assert!(r.contains("Съешь"));
        assert!(r.contains("чаю"));
    }

    // ── Edge case: code block vs prose — code loses to longer prose ──
    #[test]
    fn code_block_loses_to_prose() {
        let html = r#"
            <div class="code"><p>fn main() { let x = vec![1,2,3]; }</p></div>
            <div class="content">
              <p>This article discusses the architecture of the system in detail,
              with many sentences, and, of course, enough commas and length to
              dominate the scoring over a short code snippet nearby.</p>
            </div>
        "#;
        let r = extract(html);
        assert!(r.contains("architecture"));
        assert!(!r.contains("fn main"));
    }

    // ── Edge case: keyword_score positive/negative boundaries ──
    #[test]
    fn keyword_score_positive_keywords() {
        assert!(keyword_score("article-body") > 0);
        assert!(keyword_score("main-content") > 0);
    }

    #[test]
    fn keyword_score_negative_keywords() {
        assert!(keyword_score("nav-sidebar") < 0);
        assert!(keyword_score("footer-ad") < 0);
    }

    // ── Edge case: unknown entity passthrough ──
    #[test]
    fn decode_entities_unknown_passthrough() {
        let r = decode_entities("plain &foobar; text");
        assert_eq!(r, "plain &foobar; text");
    }
    #[test]
    fn decode_entities_no_ampersand() {
        assert_eq!(decode_entities("no entities here"), "no entities here");
    }
}
