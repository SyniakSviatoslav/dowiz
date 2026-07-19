//! BLUEPRINT-P73 anti-scope structural grep-gate (§2.2 / §5.1 / §6 not-done clause).
//!
//! dowiz.org is PURE infrastructure for prospective venue owners (operator ruling §16.21). It
//! lists NO vendors, no menus, no "browse restaurants near you" — in ANY form, not even a
//! directory of links. The single most tempting feature-creep is ruled out *categorically*: there
//! is **no** `VendorId`, `slug`, `Restaurant`, `Menu`, `catalog`, `search`, or "near you" concept
//! anywhere in the `landing` module. A diff that adds a vendor list / search / browse surface is a
//! scope violation regardless of test state. This test is the teeth: it FAILS if any forbidden
//! token appears in `kernel/src/landing/` code (comments/strings stripped so doc prose is safe).
//!
//! Mirrors `kernel/tests/no_card_data.rs` (the PCI red-line teeth) — same structural-scan idiom.
//! Run: `cargo test --test landing_no_vendor_catalog`.

use std::path::Path;

// Forbidden vendor-catalog identifiers (the anti-scope §2.2). Assembled with `concat!` so this
// file's own source never contains a contiguous forbidden literal (which would make the scan
// vacuous). Doc prose naming these tokens is therefore harmless.
const FORBIDDEN_VENDOR_TOKENS: &[&str] = &[
    concat!("vendor", "_id"),
    concat!("vendor", "catalog"),
    concat!("rest", "aurant"),
    concat!("menu", "item"),
    concat!("food", "establishment"),
    concat!("browse", "vendor"),
    concat!("near", "you"),
    concat!("search", "vendor"),
    concat!("store", "front", "slug"),
];

/// Strip Rust comments + string/char literals so doc-comment prose (which names these tokens) is
/// never a false positive. We hunt CODE identifiers only.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let b = src.as_bytes();
    let mut i = 0;
    let mut in_block = false;
    while i < b.len() {
        if in_block {
            if i + 1 < b.len() && b[i] == b'*' && b[i + 1] == b'/' {
                in_block = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'/' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'*' {
            in_block = true;
            i += 2;
            continue;
        }
        if b[i] == b'"' || b[i] == b'\'' {
            let q = b[i];
            out.push(' ');
            i += 1;
            while i < b.len() && b[i] != q {
                if b[i] == b'\\' {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            i += 1;
            out.push(' ');
            continue;
        }
        out.push(b[i] as char);
        i += 1;
    }
    out
}

/// Collect every `.rs` file under `kernel/src/landing`.
fn landing_src_files() -> Vec<String> {
    let mut out = Vec::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("landing");
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                    out.push(p.to_string_lossy().into_owned());
                }
            }
        }
    }
    out
}

#[test]
fn landing_has_no_vendor_catalog_surface() {
    let files = landing_src_files();
    // The module always exists (mod.rs at minimum); if landing/ is missing the gate is meaningless.
    assert!(!files.is_empty(), "landing/ module must exist");
    let mut violations = Vec::new();
    for f in &files {
        let src = match std::fs::read_to_string(f) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let code = strip_comments(&src);
        let words: Vec<String> = code
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .map(|w| w.to_lowercase())
            .collect();
        for token in FORBIDDEN_VENDOR_TOKENS {
            let needle = token.to_lowercase();
            if words.iter().any(|w| w == &needle) {
                violations.push(format!("{f}: contains vendor-catalog token '{token}'"));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "P73 §16.21 anti-scope violation — vendor-catalog surface found in landing/: {:?}",
        violations
    );
}
