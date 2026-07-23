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

use std::collections::{HashMap, HashSet};

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
        format!(
            "missing required frontmatter keys: {}",
            self.missing_keys.join(", ")
        )
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
        Err(SpineError {
            missing_keys: missing,
        })
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

// ===== P2: tag-graph cross-linking =====

/// Parse a raw `tags:` frontmatter value into a de-duplicated, lowercased,
/// trimmed tag list. Splits on commas, semicolons, and any whitespace; empty
/// fragments are dropped. Order follows first appearance (stable); callers sort
/// buckets for byte-exact determinism.
pub fn parse_tags(raw: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for frag in raw.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
        let t = frag.trim().to_lowercase();
        if !t.is_empty() && !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

/// P2 — Build a deterministic `tag → [doc-id]` index from frontmatter docs.
/// Each bucket is sorted ascending (byte order) and de-duplicated, so the map
/// is reproducible regardless of input ordering.
pub fn tag_index(docs: &[(String, &Frontmatter)]) -> HashMap<String, Vec<String>> {
    let mut idx: HashMap<String, Vec<String>> = HashMap::new();
    for (id, fm) in docs {
        if let Some(raw) = fm.get("tags") {
            for tag in parse_tags(raw) {
                idx.entry(tag).or_default().push(id.clone());
            }
        }
    }
    for bucket in idx.values_mut() {
        bucket.sort();
        bucket.dedup();
    }
    idx
}

/// P2 — Docs that share ≥1 tag with `id`, sorted ascending, excluding `id` itself.
/// Cross-references are derived from the `tag_index` buckets that contain `id`.
pub fn backlinks(id: &str, index: &HashMap<String, Vec<String>>) -> Vec<String> {
    // P77 B2: a `HashSet` accumulator replaces the O(R) `Vec::contains` dedup
    // (`:215` in the old impl) so accumulation is O(R) total, not O(R²). Membership
    // of `id` in a sorted bucket uses `binary_search` (O(log R)) — buckets are
    // sorted+deduped in `tag_index`/`build`. Result set+order is identical.
    let mut seen: HashSet<String> = HashSet::new();
    for bucket in index.values() {
        if bucket.binary_search(&id.to_string()).is_ok() {
            for other in bucket {
                if other != id {
                    seen.insert(other.clone());
                }
            }
        }
    }
    let mut related: Vec<String> = seen.into_iter().collect();
    related.sort();
    related
}

// ===== P3: hierarchical multi-doc MAP aggregation =====

/// P3 — Group docs by their FIRST tag into `## <tag>` sections, each doc listed
/// as `- [title](path) · id`. Sections (by tag) and docs (by id) are sorted
/// deterministically; docs with no tag fall under `## (untagged)`. A top
/// `# Knowledge Map` header precedes the sections. Stable, no trailing-ws drift.
pub fn build_map(docs: &[(String, String, Vec<String>, String)]) -> String {
    // P77 B2: a `HashMap<tag, group-index>` replaces the O(distinct-tags) linear
    // `groups.iter_mut().find` inside the per-doc loop (`:239` old), so grouping
    // is O(docs) total instead of O(docs · distinct-tags). Final sort order is
    // unchanged → byte-identical MAP.md output.
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    let mut group_of: HashMap<String, usize> = HashMap::new();
    for (i, (_, _, tags, _)) in docs.iter().enumerate() {
        let tag = tags
            .first()
            .cloned()
            .unwrap_or_else(|| "(untagged)".to_string());
        match group_of.get(&tag) {
            Some(&g) => groups[g].1.push(i),
            None => {
                group_of.insert(tag.clone(), groups.len());
                groups.push((tag, vec![i]));
            }
        }
    }
    groups.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, ids) in groups.iter_mut() {
        ids.sort_by(|&a, &b| docs[a].0.cmp(&docs[b].0));
    }

    let mut out = String::from("# Knowledge Map\n\n");
    for (tag, ids) in &groups {
        out.push_str(&format!("## {}\n", tag));
        for &i in ids {
            let (id, title, _, path) = &docs[i];
            out.push_str(&format!("- [{}]({}) · {}\n", title, path, id));
        }
        out.push('\n');
    }
    out
}

// ===== P4: query/lookup API over an in-memory index =====

/// P4 — An in-memory knowledge-spine index built once from a corpus, supporting
/// deterministic id / tag / related lookups. `tags` are a pre-split list; the
/// index lowercases them for case-insensitive tag queries.
pub struct SpineIndex {
    docs: Vec<(String, String, Vec<String>, String)>,
    tag_index: HashMap<String, Vec<String>>,
    /// id → position in `docs`, built once in `SpineIndex::build`. Turns
    /// `lookup_by_id`/`related`'s O(docs) `docs.iter().find/any` into O(1). Ids
    /// are unique (doc comment on `lookup_by_id`). (P77 B2.)
    id_index: HashMap<String, usize>,
}

impl SpineIndex {
    /// Build the index from `(id, title, tags, path)` records. Tag buckets are
    /// sorted + de-duplicated up front so tag lookups are deterministic and
    /// amortized O(1) via the `HashMap`; id lookups use a linear scan
    /// (`lookup_by_id`, O(n) in the number of docs). Deterministic, not O(1)
    /// for every access path.
    pub fn build(docs: Vec<(String, String, Vec<String>, String)>) -> SpineIndex {
        let mut tag_index: HashMap<String, Vec<String>> = HashMap::new();
        let mut id_index: HashMap<String, usize> = HashMap::with_capacity(docs.len());
        for (pos, (id, _, tags, _)) in docs.iter().enumerate() {
            id_index.insert(id.clone(), pos);
            for tag in tags {
                tag_index
                    .entry(tag.to_lowercase())
                    .or_default()
                    .push(id.clone());
            }
        }
        for bucket in tag_index.values_mut() {
            bucket.sort();
            bucket.dedup();
        }
        SpineIndex {
            docs,
            tag_index,
            id_index,
        }
    }

    /// P4 — Lookup by exact id. Ids are unique, so this returns a single-element
    /// `Vec` when found, empty otherwise (uniform `Vec<String>` return shape).
    pub fn lookup_by_id(&self, id: &str) -> Vec<String> {
        if self.id_index.contains_key(id) {
            vec![id.to_string()]
        } else {
            Vec::new()
        }
    }

    /// P4 — Lookup by tag, case-insensitive against the index. Returns the sorted
    /// bucket of doc ids sharing that tag.
    pub fn lookup_by_tag(&self, tag: &str) -> Vec<String> {
        self.tag_index
            .get(&tag.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }

    /// P4 — `related(id)` == backlinks: every doc sharing ≥1 tag with `id`,
    /// sorted, excluding `id`. Pure over the in-memory tag index.
    pub fn related(&self, id: &str) -> Vec<String> {
        let my_tags: Vec<String> = match self.id_index.get(id) {
            Some(&pos) => self.docs[pos].2.iter().map(|t| t.to_lowercase()).collect(),
            None => return Vec::new(),
        };
        // P77 B2: `HashSet` accumulator replaces the O(R) `!out.contains(other)`
        // dedup (`:325` old) so accumulation is O(R) total, not O(R²). The result
        // set+order is identical (sorted ascending, self excluded).
        let mut seen: HashSet<String> = HashSet::new();
        for tag in &my_tags {
            if let Some(bucket) = self.tag_index.get(tag) {
                for other in bucket {
                    if other != id {
                        seen.insert(other.clone());
                    }
                }
            }
        }
        let mut out: Vec<String> = seen.into_iter().collect();
        out.sort();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spine_parse_frontmatter() {
        // RED→GREEN: a doc with a leading frontmatter block must have its keys
        // parsed into the map and the body cleanly separated after the closing
        // `---`.
        let doc =
            "---\ntitle: Hello World\nid: doc-1\ntags: a, b, c\n---\n# Body\nSome text here.\n";
        let (fm, body) = parse_frontmatter(doc);
        assert_eq!(
            fm.get("title").map(|s| s.as_str()),
            Some("Hello World"),
            "title parsed"
        );
        assert_eq!(fm.get("id").map(|s| s.as_str()), Some("doc-1"), "id parsed");
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
            SpineEntry {
                path: "docs/b.md".into(),
                fm: e_a,
            },
            SpineEntry {
                path: "docs/a.md".into(),
                fm: e_b,
            },
            SpineEntry {
                path: "docs/c.md".into(),
                fm: e_c,
            },
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

    // ===== P2–P4 RED→GREEN tests =====

    #[test]
    fn spine_tag_index_deterministic() {
        // Same input (different order) ⇒ identical sorted buckets.
        let mut fm1 = Frontmatter::new();
        fm1.insert("tags".into(), "B, A, c, A".into());
        let mut fm2 = Frontmatter::new();
        fm2.insert("tags".into(), "c;A B".into());
        let mut fm3 = Frontmatter::new();
        fm3.insert("tags".into(), "a, b, C".into());

        let docs_a = vec![
            ("x".to_string(), &fm1),
            ("y".to_string(), &fm2),
            ("z".to_string(), &fm3),
        ];
        let docs_b = vec![
            ("z".to_string(), &fm1),
            ("y".to_string(), &fm3),
            ("x".to_string(), &fm2),
        ];
        let idx_a = tag_index(&docs_a);
        let idx_b = tag_index(&docs_b);
        assert_eq!(idx_a, idx_b, "buckets identical regardless of input order");
        // Buckets sorted + deduped (lowercased).
        assert_eq!(
            idx_a.get("a"),
            Some(&vec!["x".into(), "y".into(), "z".into()])
        );
        assert_eq!(
            idx_a.get("b"),
            Some(&vec!["x".into(), "y".into(), "z".into()])
        );
        assert_eq!(
            idx_a.get("c"),
            Some(&vec!["x".into(), "y".into(), "z".into()])
        );
        assert_eq!(idx_a.get("A"), None, "tags are lowercased on insert");
    }

    #[test]
    fn spine_backlinks_excludes_self_and_is_sorted() {
        // Three docs: a↔b share `rust`; b↔c share `ml`. backlinks(b) = [a, c].
        let mut fm_a = Frontmatter::new();
        fm_a.insert("tags".into(), "rust".into());
        let mut fm_b = Frontmatter::new();
        fm_b.insert("tags".into(), "rust, ml".into());
        let mut fm_c = Frontmatter::new();
        fm_c.insert("tags".into(), "ml".into());

        let docs = vec![
            ("a".to_string(), &fm_a),
            ("b".to_string(), &fm_b),
            ("c".to_string(), &fm_c),
        ];
        let idx = tag_index(&docs);
        let bl = backlinks("b", &idx);
        assert_eq!(
            bl,
            vec!["a".to_string(), "c".to_string()],
            "sorted, excludes self"
        );
        assert!(!bl.contains(&"b".to_string()), "self never returned");
        // A doc with no relations returns empty.
        assert_eq!(backlinks("a", &HashMap::new()), Vec::<String>::new());
    }

    #[test]
    fn spine_map_grouped_by_tag_and_sorted() {
        // P3: sections + entries sorted, header present.
        let docs = vec![
            (
                "c".to_string(),
                "Gamma".into(),
                vec!["ml".into()],
                "docs/c.md".into(),
            ),
            (
                "a".to_string(),
                "Alpha".into(),
                vec!["rust".into()],
                "docs/a.md".into(),
            ),
            (
                "b".to_string(),
                "Beta".into(),
                vec!["rust".into()],
                "docs/b.md".into(),
            ),
        ];
        let map = build_map(&docs);
        assert!(map.starts_with("# Knowledge Map\n\n"), "top header present");
        // Section `## ml` comes before `## rust` (tags sorted ascending), and
        // within rust, entries sorted by id (a before b).
        let pos_rust = map.find("## rust").expect("rust section");
        let pos_ml = map.find("## ml").expect("ml section");
        assert!(pos_ml < pos_rust, "sections sorted by tag (ml before rust)");
        let pos_a = map.find("- [Alpha](docs/a.md) · a").expect("row a");
        let pos_b = map.find("- [Beta](docs/b.md) · b").expect("row b");
        assert!(pos_a < pos_b, "entries within section sorted by id");
        assert!(map.contains("- [Gamma](docs/c.md) · c"), "ml entry present");
        // No trailing whitespace drift on entry lines.
        assert!(!map.lines().any(|l| l.starts_with("- ") && l.ends_with(' ')));
    }

    #[test]
    fn spine_lookup_by_tag_case_insensitive() {
        // P4: tag lookups are case-insensitive.
        let docs = vec![
            (
                "a".to_string(),
                "Alpha".into(),
                vec!["Rust".into()],
                "docs/a.md".into(),
            ),
            (
                "b".to_string(),
                "Beta".into(),
                vec!["RUST".into()],
                "docs/b.md".into(),
            ),
            (
                "c".to_string(),
                "Gamma".into(),
                vec!["ml".into()],
                "docs/c.md".into(),
            ),
        ];
        let idx = SpineIndex::build(docs);
        assert_eq!(
            idx.lookup_by_tag("rust"),
            vec!["a".to_string(), "b".to_string()],
            "lowercase query matches mixed-case tags"
        );
        assert_eq!(
            idx.lookup_by_tag("RUST"),
            vec!["a".to_string(), "b".to_string()],
            "uppercase query matches"
        );
        assert_eq!(
            idx.lookup_by_tag("none"),
            Vec::<String>::new(),
            "missing tag ⇒ empty"
        );
    }

    #[test]
    fn spine_related_returns_shared_tag_docs() {
        // P4: related(id) returns docs sharing ≥1 tag (== backlinks).
        let docs = vec![
            (
                "a".to_string(),
                "Alpha".into(),
                vec!["rust".into()],
                "docs/a.md".into(),
            ),
            (
                "b".to_string(),
                "Beta".into(),
                vec!["rust".into(), "ml".into()],
                "docs/b.md".into(),
            ),
            (
                "c".to_string(),
                "Gamma".into(),
                vec!["ml".into()],
                "docs/c.md".into(),
            ),
            (
                "d".to_string(),
                "Delta".into(),
                vec!["crypto".into()],
                "docs/d.md".into(),
            ),
        ];
        let idx = SpineIndex::build(docs);
        assert_eq!(idx.lookup_by_id("b"), vec!["b".to_string()]);
        assert_eq!(idx.lookup_by_id("zzz"), Vec::<String>::new());
        assert_eq!(
            idx.related("b"),
            vec!["a".to_string(), "c".to_string()],
            "shares rust(→a) and ml(→c), excludes self"
        );
        assert_eq!(
            idx.related("d"),
            Vec::<String>::new(),
            "isolated doc has no related"
        );
    }

    // ===== P77 B2 differential / adversarial tests =====

    /// Reference re-implementations of the OLD O(R²)/O(docs) semantics, used as
    /// the byte-identical oracle for the HashSet/HashMap rewrite.
    fn ref_backlinks(id: &str, index: &HashMap<String, Vec<String>>) -> Vec<String> {
        let mut related: Vec<String> = Vec::new();
        for bucket in index.values() {
            if bucket.iter().any(|d| d == id) {
                for other in bucket {
                    if other != id && !related.contains(other) {
                        related.push(other.clone());
                    }
                }
            }
        }
        related.sort();
        related
    }

    fn ref_related(idx: &SpineIndex, id: &str) -> Vec<String> {
        let my_tags: Vec<String> = match idx.docs.iter().find(|(i, _, _, _)| i == id) {
            Some((_, _, tags, _)) => tags.iter().map(|t| t.to_lowercase()).collect(),
            None => return Vec::new(),
        };
        let mut out: Vec<String> = Vec::new();
        for tag in &my_tags {
            if let Some(bucket) = idx.tag_index.get(tag) {
                for other in bucket {
                    if other != id && !out.contains(other) {
                        out.push(other.clone());
                    }
                }
            }
        }
        out.sort();
        out
    }

    // GREEN (P77 B2-D-EQUIV): `backlinks`/`related`/`lookup_by_id`/`build_map`
    // must return byte-identical results to the OLD impl on a fixed corpus.
    #[test]
    fn spine_equiv_reference() {
        // Build a corpus where several docs share many tags (high dedup pressure).
        let tags_pool = ["rust", "ml", "crypto", "wasm", "graph", "net", "pki", "fsm"];
        let mut docs: Vec<(String, String, Vec<String>, String)> = Vec::new();
        for i in 0..256u32 {
            let id = format!("doc-{i:03}");
            // Each doc gets 3 overlapping tags selected deterministically.
            let t: Vec<String> = (0..3)
                .map(|k| tags_pool[(i as usize + k * 7) % tags_pool.len()].to_string())
                .collect();
            docs.push((id.clone(), format!("Title {i}"), t, format!("docs/{id}.md")));
        }
        // Two copies in different insertion orders → HashMap iteration order must
        // NOT leak into output (the final sort() is the only thing that binds it).
        let mut docs_shuffled = docs.clone();
        docs_shuffled.rotate_left(137);

        let idx_a = SpineIndex::build(docs.clone());
        let idx_b = SpineIndex::build(docs_shuffled.clone());

        // Build the OLD-style `tag_index` input (id → Frontmatter) for the
        // `backlinks` equivalence cross-check. `docs` carries raw `Vec<String>`
        // tags; `tag_index`/`backlinks` expect `&Frontmatter`, so synthesize it.
        let mut fm_docs: Vec<(String, Frontmatter)> = Vec::new();
        for (id, _, tags, _) in &docs {
            let mut fm = Frontmatter::new();
            fm.insert("tags".into(), tags.join(", "));
            fm_docs.push((id.clone(), fm));
        }
        let tag_a = tag_index(
            &fm_docs
                .iter()
                .map(|(id, fm)| (id.clone(), fm))
                .collect::<Vec<_>>(),
        );
        for target in ["doc-001", "doc-128", "doc-255"] {
            // build_map output identical across both orderings
            assert_eq!(
                build_map(&docs),
                build_map(&docs_shuffled),
                "build_map byte-identical across insertion order"
            );
            // related output identical to BOTH the other-ordering index and the ref
            assert_eq!(
                idx_a.related(target),
                idx_b.related(target),
                "related stable across insertion order"
            );
            assert_eq!(
                idx_a.related(target),
                ref_related(&idx_a, target),
                "related matches OLD impl"
            );
            assert_eq!(
                idx_a.lookup_by_id(target),
                vec![target.to_string()],
                "lookup_by_id found"
            );
        }
        assert_eq!(idx_a.lookup_by_id("nope"), Vec::<String>::new());
        // backlinks over the old-style index must match the ref
        let tgt = "doc-010";
        let bl = backlinks(tgt, &tag_a);
        assert_eq!(bl, ref_backlinks(tgt, &tag_a), "backlinks matches OLD impl");
        // self never in result, fully sorted
        assert!(!bl.contains(&tgt.to_string()));
        assert!(bl.windows(2).all(|w| w[0] <= w[1]));
    }

    // GREEN (P77 B2): a doc sharing the SAME other doc across multiple tags
    // appears exactly once (HashSet dedup == old `!contains`).
    #[test]
    fn spine_duplicate_backlinks_deduped() {
        let mut fm_a = Frontmatter::new();
        fm_a.insert("tags".into(), "rust, ml".into());
        let mut fm_x = Frontmatter::new();
        fm_x.insert("tags".into(), "rust, ml".into()); // x shares BOTH tags with a
        let docs = vec![("a".to_string(), &fm_a), ("x".to_string(), &fm_x)];
        let idx = tag_index(&docs);
        let bl = backlinks("a", &idx);
        assert_eq!(bl, vec!["x".to_string()], "shared doc appears exactly once");
        let idx2 = SpineIndex::build(vec![
            (
                "a".to_string(),
                "A".into(),
                vec!["rust".into(), "ml".into()],
                "a.md".into(),
            ),
            (
                "x".to_string(),
                "X".into(),
                vec!["rust".into(), "ml".into()],
                "x.md".into(),
            ),
        ]);
        assert_eq!(idx2.related("a"), vec!["x".to_string()]);
    }

    // GREEN (P77 B2): a doc tagged identically to itself never appears in its own set.
    #[test]
    fn spine_self_reference_excluded() {
        let docs = vec![(
            "lonely".to_string(),
            "Lonely".into(),
            vec!["solo".into()],
            "lonely.md".into(),
        )];
        let idx = SpineIndex::build(docs);
        assert_eq!(idx.related("lonely"), Vec::<String>::new());
        let fm = {
            let mut m = Frontmatter::new();
            m.insert("tags".into(), "solo".into());
            m
        };
        let idx2 = tag_index(&[("lonely".to_string(), &fm)]);
        assert_eq!(backlinks("lonely", &idx2), Vec::<String>::new());
    }

    // GREEN (P77 B2): large corpus in two insertion orders → byte-identical
    // backlinks/related/build_map (HashMap order must not leak).
    #[test]
    fn spine_large_corpus_order_stable() {
        let mut docs: Vec<(String, String, Vec<String>, String)> = Vec::new();
        let pool = ["r", "m", "c", "w", "g", "n", "p", "f", "s", "t"];
        for i in 0..1024u32 {
            let id = format!("d{i:04}");
            let t: Vec<String> = (0..4)
                .map(|k| pool[(i as usize * 3 + k) % pool.len()].to_string())
                .collect();
            docs.push((id.clone(), format!("T{i}"), t, format!("{id}.md")));
        }
        let mut shuf = docs.clone();
        shuf.reverse();
        // build_map takes a borrow, so call it before `SpineIndex::build` moves `shuf`.
        let map_shuf = build_map(&shuf);
        let idx_a = SpineIndex::build(docs.clone());
        let idx_b = SpineIndex::build(shuf);
        for target in ["d0000", "d0512", "d1023"] {
            assert_eq!(idx_a.related(target), idx_b.related(target));
        }
        assert_eq!(build_map(&docs), map_shuf);
    }

    // GREEN (P77 B2): empty / isolated edge behavior unchanged.
    #[test]
    fn spine_empty_and_isolated() {
        assert_eq!(backlinks("x", &HashMap::new()), Vec::<String>::new());
        let idx = SpineIndex::build(vec![(
            "alone".to_string(),
            "Alone".into(),
            vec!["unique-tag".into()],
            "alone.md".into(),
        )]);
        assert_eq!(idx.related("alone"), Vec::<String>::new());
    }
}
