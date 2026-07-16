//! Knowledge-spine validator + MAP.md generator (W3-3 / P1).
//!
//! A hand-rolled, dependency-free frontmatter parser for living-knowledge
//! markdown docs. No yaml crate: we only need the *flat* key→value subset that
//! our spine documents emit (`title`, `id`, `tags`, ...). Deterministic: same
//! bytes ⇒ same parsed map + same MAP.md output (blueprint §3).
//!
//! This organ lives in the retrieval layer (sibling to `bm25`/`recall`) and is
//! the P1 seed of the knowledge-spine retrieval organ — a MAP.md index over the
//! corpus frontmatter that the BM25/trigram layers can later fuse with. Pure
//! `std`, no new deps.

use std::collections::HashMap;

/// A parsed frontmatter block: flat key → value (both trimmed).
pub type Frontmatter = HashMap<String, String>;

/// Structured validation failure. Lists every required key that was absent so
/// the caller can surface a single, complete error instead of one round-trip
/// per missing key (fail-closed: a missing key is a hard error, never a warning).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpineError {
    /// Required keys that were missing from the frontmatter.
    pub missing_keys: Vec<String>,
}

impl SpineError {
    /// Single-sentence human message naming every missing key.
    pub fn message(&self) -> String {
        format!("missing required frontmatter keys: {}", self.missing_keys.join(", "))
    }
}

/// Parse a (possibly frontmatter-bearing) markdown document into its flat
/// `key → value` frontmatter map plus the remaining body text.
///
/// Frontmatter is the **leading** block delimited by an opening `---` line, then
/// `key: value` lines, then a closing `---` line. Anything after the closing
/// delimiter is the body. A document with no leading `---` block yields an
/// empty frontmatter and the **entire** document as body. An opening `---`
/// without a matching closing `---` is treated as *not* frontmatter (whole doc
/// is body). Never panics.
pub fn parse_frontmatter(doc: &str) -> (Frontmatter, String) {
    let mut fm = Frontmatter::new();
    let mut body = doc.to_string();

    // Use `split('\n')` (NOT `lines()`) so a trailing newline is preserved as a
    // final empty element and reconstructable with `join('\n')` byte-exactly.
    let lines: Vec<&str> = doc.split('\n').collect();
    // Need at least: opening `---`, >=0 kv lines, closing `---`.
    if lines.is_empty() || lines[0].trim() != "---" {
        return (fm, body);
    }

    // Find the *first* closing delimiter after the opening line.
    let mut close = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            close = Some(i);
            break;
        }
    }
    let close = match close {
        Some(c) => c,
        None => return (fm, body), // unterminated: treat whole doc as body
    };

    // Parse `key: value` lines between opening (idx 0) and closing (idx `close`).
    for line in &lines[1..close] {
        let line = line.trim();
        if line.is_empty() {
            continue; // tolerate blank lines inside the block
        }
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim().to_string();
            let v = v.trim().to_string();
            if !k.is_empty() {
                fm.insert(k, v);
            }
        }
        // Lines without a ':' are ignored (lenient — we only need the subset).
    }

    // Body = everything after the closing delimiter, reconstructed line-wise.
    body = lines[close + 1..].join("\n");
    (fm, body)
}

/// Validate that `fm` contains every key in `required`.
/// Returns `Err(SpineError)` listing *all* missing keys (not just the first),
/// so a single pass reports the complete set of defects.
pub fn validate_frontmatter(fm: &Frontmatter, required: &[&str]) -> Result<(), SpineError> {
    let missing: Vec<String> = required
        .iter()
        .filter(|k| !fm.contains_key(**k))
        .map(|k| k.to_string())
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(SpineError { missing_keys: missing })
    }
}

/// A single spine entry: where it lives (`path`) and its parsed frontmatter.
#[derive(Debug, Clone)]
pub struct SpineEntry {
    pub path: String,
    pub fm: Frontmatter,
}

/// Escape a markdown table cell: a literal `|` would break column alignment, so
/// backslash-escape it. Everything else passes through unchanged.
fn escape_cell(s: &str) -> String {
    s.replace('|', "\\|")
}

/// Generate a deterministic `MAP.md`-style index over `entries`.
///
/// Sorted by `id` (ascending byte order) so the output is reproducible
/// regardless of input ordering. Each row is `id → title → path`. Entries
/// without an `id` sort last (stable, never panic). Returns a markdown table.
pub fn generate_map(entries: &[SpineEntry]) -> String {
    let mut rows: Vec<(&SpineEntry, String)> = entries
        .iter()
        .map(|e| {
            let id = e.fm.get("id").cloned().unwrap_or_default();
            (e, id)
        })
        .collect();
    // Sort by id ascending; a missing id (empty) sorts after any real id.
    rows.sort_by(|a, b| {
        if a.1.is_empty() && b.1.is_empty() {
            a.0.path.cmp(&b.0.path)
        } else if a.1.is_empty() {
            std::cmp::Ordering::Greater
        } else if b.1.is_empty() {
            std::cmp::Ordering::Less
        } else {
            a.1.cmp(&b.1)
        }
    });

    let mut out = String::from("# MAP\n\n");
    out.push_str("| id | title | path |\n");
    out.push_str("| --- | --- | --- |\n");
    for (e, id) in &rows {
        let title = e.fm.get("title").map(|s| s.as_str()).unwrap_or("");
        // Cells are space-padded (`| a |`) to match standard Markdown tables.
        out.push('|');
        out.push(' ');
        out.push_str(&escape_cell(&id));
        out.push(' ');
        out.push('|');
        out.push(' ');
        out.push_str(&escape_cell(title));
        out.push(' ');
        out.push('|');
        out.push(' ');
        out.push_str(&escape_cell(&e.path));
        out.push(' ');
        out.push_str("|\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spine_parse_frontmatter() {
        // RED→GREEN: a doc with a leading frontmatter block must have its keys
        // parsed into the map and the body cleanly separated after the closing
        // `---`.
        let doc = "---\ntitle: Hello World\nid: doc-1\ntags: a, b, c\n---\n# Body\nSome text here.\n";
        let (fm, body) = parse_frontmatter(doc);
        assert_eq!(
            fm.get("title").map(|s| s.as_str()),
            Some("Hello World"),
            "title parsed"
        );
        assert_eq!(
            fm.get("id").map(|s| s.as_str()),
            Some("doc-1"),
            "id parsed"
        );
        assert_eq!(
            fm.get("tags").map(|s| s.as_str()),
            Some("a, b, c"),
            "tags parsed"
        );
        // Body is exactly what follows the closing delimiter.
        assert_eq!(body, "# Body\nSome text here.\n");
        // No stray delimiter leaks into the body.
        assert!(!body.contains("---"));
    }

    #[test]
    fn spine_missing_required_key_detected() {
        // RED→GREEN: a doc lacking `id` must fail validation and the error must
        // name exactly the missing keys (not a generic failure).
        let doc = "---\ntitle: No Id\n---\nbody text\n";
        let (fm, _body) = parse_frontmatter(doc);
        let required = ["title", "id", "tags"];
        let err = validate_frontmatter(&fm, &required).expect_err("must reject missing keys");
        assert!(
            err.missing_keys.contains(&"id".to_string()),
            "error must name missing `id`"
        );
        assert!(
            err.missing_keys.contains(&"tags".to_string()),
            "error must name missing `tags`"
        );
        assert!(
            !err.missing_keys.contains(&"title".to_string()),
            "present key must NOT be reported"
        );
    }

    #[test]
    fn spine_map_generation_sorted() {
        // RED→GREEN: MAP output must be deterministically sorted by id and
        // contain every entry, regardless of input order.
        let mut e_a = Frontmatter::new();
        e_a.insert("id".into(), "b".into());
        e_a.insert("title".into(), "Beta".into());
        let mut e_b = Frontmatter::new();
        e_b.insert("id".into(), "a".into());
        e_b.insert("title".into(), "Alpha".into());
        let mut e_c = Frontmatter::new();
        e_c.insert("id".into(), "c".into());
        e_c.insert("title".into(), "Gamma".into());

        let entries = vec![
            SpineEntry { path: "docs/b.md".into(), fm: e_a },
            SpineEntry { path: "docs/a.md".into(), fm: e_b },
            SpineEntry { path: "docs/c.md".into(), fm: e_c },
        ];

        let map = generate_map(&entries);
        // Positions of each id row must appear in ascending id order.
        let pos_a = map.find("| a |").expect("row a present");
        let pos_b = map.find("| b |").expect("row b present");
        let pos_c = map.find("| c |").expect("row c present");
        assert!(
            pos_a < pos_b && pos_b < pos_c,
            "MAP rows must sort by id: a<b<c (got {pos_a},{pos_b},{pos_c})"
        );
        // Every entry's path must appear (title + path columns present).
        for p in ["docs/a.md", "docs/b.md", "docs/c.md"] {
            assert!(map.contains(p), "every entry path present: {p}");
        }
        // Titles likewise surfaced.
        for t in ["Alpha", "Beta", "Gamma"] {
            assert!(map.contains(t), "every entry title present: {t}");
        }
    }

    #[test]
    fn spine_no_frontmatter_handled() {
        // RED→GREEN: a doc with no leading `---` block yields empty frontmatter
        // and the whole doc as body; an unterminated opening `---` must not
        // panic either.
        let doc = "# Just a heading\n\nNo frontmatter here.\n";
        let (fm, body) = parse_frontmatter(doc);
        assert!(fm.is_empty(), "no frontmatter parsed");
        assert_eq!(body, doc, "body == whole doc when no frontmatter");

        // Unterminated opening delimiter: lenient, no panic, treated as body.
        let unterminated = "---\ntitle: orphan\n";
        let (fm2, body2) = parse_frontmatter(unterminated);
        assert!(fm2.is_empty(), "unterminated block yields no frontmatter");
        assert_eq!(body2, unterminated);

        // Empty input is fine too.
        let (fm3, body3) = parse_frontmatter("");
        assert!(fm3.is_empty());
        assert_eq!(body3, "");
    }
}
