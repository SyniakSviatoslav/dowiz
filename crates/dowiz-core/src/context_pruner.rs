//! context_pruner.rs — headroom-style line-importance pruning (native port).
//!
//! Reverse-engineered from `headroomlabs-ai/headroom` (Apache-2.0): score each
//! line of tool output / logs / diffs / prose by an importance signal, then drop
//! the lowest-priority lines to fit a token budget. This is the token-saving core
//! Hermes runs "always on" before injecting tool results into context.
//!
//! Zero dependencies: byte-level case-insensitive keyword matching over the
//! error/warning/security/importance/markdown signal sets; a cheap chars/4 token
//! estimator. Reuses the crate's no_std conventions.

use alloc::vec::Vec;

/// Content kind — picks which keyword set fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PruneContext {
    /// Free-form prose / markdown — structure matters.
    Text,
    /// grep/ripgrep output — error/warn keywords win.
    Search,
    /// git diff — error + security + importance keywords.
    Diff,
    /// Log output — error/warn keywords + level prefixes.
    Log,
}

/// Why a line earned its priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportanceCategory {
    Error,
    Warning,
    Importance,
    Security,
    Markdown,
}

/// One detector's verdict for one line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImportanceSignal {
    pub category: Option<ImportanceCategory>,
    /// 0.0 = drop first, 1.0 = keep at all costs.
    pub priority: f32,
    /// 0.0 = no information, 1.0 = the detector is sure.
    pub confidence: f32,
}

impl ImportanceSignal {
    pub const fn neutral() -> Self {
        Self {
            category: None,
            priority: 0.2,
            confidence: 0.0,
        }
    }
    pub const fn matched(category: ImportanceCategory, priority: f32, confidence: f32) -> Self {
        Self {
            category: Some(category),
            priority,
            confidence,
        }
    }
}

// Keyword sets (lowercased ASCII).
const SECURITY: &[&str] = &[
    "password",
    "secret",
    "token",
    "credential",
    "api key",
    "apikey",
    "private key",
];
const ERROR: &[&str] = &[
    "error",
    "panic",
    "fatal",
    "exception",
    "traceback",
    "failed",
    "failure",
    "abort",
    "cannot",
    "could not",
];
const WARNING: &[&str] = &["warn", "warning", "deprecated", "caution", "deprecat"];
const IMPORTANCE: &[&str] = &[
    "todo",
    "fixme",
    "hack",
    "important",
    "critical",
    "urgent",
    "must",
    "required",
];

/// Byte-level case-insensitive substring search (no alloc, no_std).
fn contains_ci(hay: &str, needle: &str) -> bool {
    let h = hay.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() {
        return true;
    }
    if n.len() > h.len() {
        return false;
    }
    'outer: for i in 0..=(h.len() - n.len()) {
        for j in 0..n.len() {
            if h[i + j].to_ascii_lowercase() != n[j].to_ascii_lowercase() {
                continue 'outer;
            }
        }
        return true;
    }
    false
}

/// Classify a single line's importance for the given context.
pub fn classify_line(line: &str, ctx: PruneContext) -> ImportanceSignal {
    let l = line.trim();
    if l.is_empty() {
        return ImportanceSignal::neutral();
    }
    // Security keywords fire in every context (highest stakes).
    for kw in SECURITY {
        if contains_ci(l, kw) {
            return ImportanceSignal::matched(ImportanceCategory::Security, 1.0, 0.9);
        }
    }
    match ctx {
        PruneContext::Text => {
            if l.starts_with('#') {
                return ImportanceSignal::matched(ImportanceCategory::Markdown, 0.6, 0.9);
            }
            if l.starts_with("**") || l.starts_with('>') {
                return ImportanceSignal::matched(ImportanceCategory::Markdown, 0.5, 0.8);
            }
            for kw in IMPORTANCE {
                if contains_ci(l, kw) {
                    return ImportanceSignal::matched(ImportanceCategory::Importance, 0.7, 0.8);
                }
            }
        }
        PruneContext::Search | PruneContext::Diff | PruneContext::Log => {
            // Level prefixes win first in logs (e.g. "ERROR:", "[WARN]").
            if ctx == PruneContext::Log {
                let up = l.to_ascii_uppercase();
                if up.contains("ERROR") || up.contains("FATAL") || up.contains("PANIC") {
                    return ImportanceSignal::matched(ImportanceCategory::Error, 1.0, 0.95);
                }
                if up.contains("WARN") {
                    return ImportanceSignal::matched(ImportanceCategory::Warning, 0.8, 0.9);
                }
            }
            for kw in ERROR {
                if contains_ci(l, kw) {
                    return ImportanceSignal::matched(ImportanceCategory::Error, 1.0, 0.95);
                }
            }
            for kw in WARNING {
                if contains_ci(l, kw) {
                    return ImportanceSignal::matched(ImportanceCategory::Warning, 0.8, 0.9);
                }
            }
            for kw in IMPORTANCE {
                if contains_ci(l, kw) {
                    return ImportanceSignal::matched(ImportanceCategory::Importance, 0.7, 0.8);
                }
            }
        }
    }
    ImportanceSignal::neutral()
}

/// Cheap token estimate: ~4 chars/token (matches headroom's no-tokenizer fallback).
fn est_tokens(line: &str) -> usize {
    line.len() / 4 + 1
}

/// Prune a context chunk to a token budget, keeping the highest-priority lines
/// and preserving their original order. Returns the kept lines.
pub fn prune_lines<'a>(lines: &[&'a str], ctx: PruneContext, budget_tokens: usize) -> Vec<&'a str> {
    // (index, priority, cost, line)
    let mut scored: Vec<(usize, f32, usize, &'a str)> = lines
        .iter()
        .enumerate()
        .map(|(i, l)| (i, classify_line(l, ctx).priority, est_tokens(l), *l))
        .collect();
    // Stable sort by priority descending (ties keep original order).
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

    let mut kept: Vec<usize> = Vec::new();
    let mut used = 0usize;
    for (i, _p, cost, _l) in &scored {
        if used + cost <= budget_tokens {
            used += cost;
            kept.push(*i);
        }
    }
    // Preserve original order in the output.
    kept.sort_unstable();
    kept.into_iter().map(|i| lines[i]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_lines_outrank_neutral_lines() {
        let e = classify_line("ERROR: connection refused", PruneContext::Log);
        let n = classify_line("debug trace", PruneContext::Log);
        assert_eq!(e.category, Some(ImportanceCategory::Error));
        assert!(e.priority > n.priority);
    }

    #[test]
    fn security_fires_in_all_contexts() {
        let s = classify_line("api key leaked here", PruneContext::Text);
        assert_eq!(s.category, Some(ImportanceCategory::Security));
        assert_eq!(s.priority, 1.0);
    }

    #[test]
    fn markdown_headers_count_in_text_only() {
        let s = classify_line("## Section", PruneContext::Text);
        assert_eq!(s.category, Some(ImportanceCategory::Markdown));
        // In diff context, "##" is not a header signal.
        let d = classify_line("## Section", PruneContext::Diff);
        assert_eq!(d.category, None);
    }

    #[test]
    fn prune_preserves_original_order() {
        let lines = ["a", "ERROR b", "c", "WARN d", "e"];
        let kept = prune_lines(&lines, PruneContext::Log, 4);
        assert_eq!(kept, vec!["ERROR b", "WARN d"]);
    }

    #[test]
    fn contains_ci_matches_case_insensitively() {
        assert!(contains_ci("PANIC: boom", "panic"));
        assert!(contains_ci("FooBar", "foobar"));
        assert!(!contains_ci("hello", "world"));
    }

    #[test]
    fn token_estimate_is_positive() {
        assert!(est_tokens("") >= 1);
        assert!(est_tokens("a long line of text") > 3);
    }
}
