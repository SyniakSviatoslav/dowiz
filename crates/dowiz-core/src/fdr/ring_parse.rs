//! `fdr/ring_parse.rs` — the pure JSONL line-parsing helpers used by the FDR
//! ring's recovery path (blueprint §4.4).
//!
//! The durable on-disk segment ring itself (`FdrRing`, `recover`, the post-mortem
//! writer) is std file I/O and stays in the kernel (`dowiz_kernel::fdr::ring`).
//! These two helpers are pure string parsing — no std, no file I/O, alloc-only —
//! so they live here in the `no_std` core and are re-exported back through the
//! kernel shim to keep the `fdr::ring::extract_u64` / `extract_str` names stable.
//!
//! Scope: parse side only. Compiles on every target incl. `wasm32`.

use alloc::string::{String, ToString};

/// Minimal field extraction (kernel is serde-free). Finds `"<key>":<number>`.
pub fn extract_u64(line: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{key}\":");
    let i = line.find(&pat)? + pat.len();
    let rest = &line[i..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Finds `"<key>":"<string>"` (values here are simple `[a-z_-]` names — no escaping).
pub fn extract_str(line: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":\"");
    let i = line.find(&pat)? + pat.len();
    let rest = &line[i..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_u64_parses_number_field() {
        let line = r#"{"seq":123,"kind":"event"}"#;
        assert_eq!(extract_u64(line, "seq"), Some(123));
        assert_eq!(extract_u64(line, "nope"), None);
    }

    #[test]
    fn extract_str_parses_string_field() {
        let line = r#"{"seq":1,"kind":"event","name":"clean_shutdown"}"#;
        assert_eq!(extract_str(line, "kind"), Some("event".to_string()));
        assert_eq!(
            extract_str(line, "name"),
            Some("clean_shutdown".to_string())
        );
        assert_eq!(extract_str(line, "missing"), None);
    }
}
