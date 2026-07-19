//! telemetry/pprof.rs — P83 Layer 2 feature-gated fallback (NO external crate).
//!
//! SYNTHESIS PERFORMANCE AUDIT 2026-07-18 §3.3-C4: "`pprof` only as feature-gated
//! fallback" when the system-wide `perf record` is unavailable (no perms / not Linux).
//!
//! This is a SAFE NO-OP: it writes a single `alert.jsonl` marker row naming the fallback
//! and returns. It NEVER shells out, NEVER hangs, NEVER needs root, and pulls NO external
//! crate into the dependency graph (pure `std`). It is compiled ONLY when the `pprof`
//! feature is enabled, so the shipping binary carries zero pprof symbols. The real capture
//! path is `breach.rs` → `perf record`; this is the degrade-closed backup.

use std::path::PathBuf;

use super::obs::{ALERT_JSONL, JsonlWriter};

/// Emit the pprof-fallback marker row into `alert.jsonl`. `dir = None` disables the file
/// write but still returns the marker string (callers may log it).
pub fn emit_fallback_marker(dir: Option<PathBuf>, reason: &str) -> String {
    let row = format!(
        "{{\"alert\":\"load_breach\",\"fallback\":\"pprof\",\"detail\":{:?}}}\n",
        reason
    );
    JsonlWriter::new(dir).append(ALERT_JSONL, &row);
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_pprof_fallback_marker_is_safe_noop() {
        // No panic, returns a valid-shape row, even with no dir.
        let row = emit_fallback_marker(None, "perf unavailable in sandbox");
        assert!(row.contains("\"fallback\":\"pprof\""));
        assert!(row.ends_with('\n'));
    }
}
