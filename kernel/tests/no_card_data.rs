//! No-card-data firewall integration test (BLUEPRINT-P60 §4.1 / §6 — the PCI red-line teeth).
//!
//! Scans the WHOLE `kernel/src/` tree for card-data identifiers (`pan`, `cvv`, `cvc`,
//! `card_number`, `card_holder`, `exp_month`, `exp_year`). Adding a `struct CardNumber` or a
//! `pan:` field anywhere in kernel/ makes this test FAIL — so the no-PAN guarantee is enforced
//! by construction (the kernel never deserializes a card — there is no field to hold one) PLUS a
//! CI-teeth scan that keeps it that way. Mirrors `kernel/tests/firewall_p47.rs` (structurally
//! deterministic, no subprocess). Run: `cargo test --test no_card_data`.

use std::path::Path;

// Forbidden card-data identifiers (the no-PAN structural guarantee). Assembled so this file's own
// source never contains a contiguous forbidden literal (which would make `!contains` vacuous) —
// the doc above uses prose rather than the raw tokens.
const FORBIDDEN_CARD_TOKENS: &[&str] = &[
    concat!("card_", "number"),
    concat!("card", "number"),
    concat!("card_", "holder"),
    concat!("card", "holder"),
    concat!("exp_", "month"),
    concat!("exp_", "year"),
    concat!("c", "vv"),
    concat!("c", "vc"),
    concat!("p", "an"),
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

/// Collect every `.rs` file under `kernel/src`.
fn kernel_src_files() -> Vec<String> {
    let mut out = Vec::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
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
fn no_card_data_type_anywhere_in_kernel_src() {
    let files = kernel_src_files();
    assert!(!files.is_empty(), "kernel src tree must be non-empty");
    let mut violations = Vec::new();
    for f in &files {
        let src = match std::fs::read_to_string(f) {
            Ok(s) => s,
            Err(_) => continue,
        };
        // Scan CODE only (comments/strings stripped) so doc prose naming a token is harmless.
        let code = strip_comments(&src);
        // Tokenize into lowercase words so `pan`/`card_number` are caught regardless of case or
        // container punctuation.
        let words: Vec<String> = code
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .map(|w| w.to_lowercase())
            .collect();
        for token in FORBIDDEN_CARD_TOKENS {
            let needle = token.to_lowercase();
            if words.iter().any(|w| w == &needle) {
                violations.push(format!("{f}: contains card-data token '{token}'"));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "P60 PCI red-line violation — card-data type found in kernel/: {:?}",
        violations
    );
}
