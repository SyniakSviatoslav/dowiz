//! evals.rs — std shim over the no_std eval core.
//!
//! The pure benchmark-generation + scoring primitives (`EvalRow`, `EvalCheck`,
//! `EmaTracker`, `brier`/`ece`/`aurc`, `eval_loss`, the metamorphic generator,
//! and the hand-rolled JSONL serializer `EvalRow::to_jsonl`) live in
//! `dowiz_core::evals` and are re-exported here. This shim keeps only the
//! std-dependent disk write — as the free function [`eval_row_append_to`]
//! (the `append_to` method can't be re-impl'd on the foreign `EvalRow`) — and
//! the disk round-trip test.

pub use dowiz_core::evals::*;

/// Append an eval row as one JSONL line to `path`. Fail-closed: any IO error is
/// returned, never silently dropped (no amnesiac writes).
pub fn eval_row_append_to(
    row: &EvalRow,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let line = row.to_jsonl();
    crate::vfs::append(path, format!("{line}\n"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // E2 proof: append is fail-closed (writes a real, reparseable line) and
    // does not swallow errors. Uses a temp file, cleaned up.
    #[test]
    fn eval_row_append_to_persists_jsonl() {
        let dir = std::env::temp_dir();
        let path = dir.join("hermes-verify-evalrow.jsonl");
        let p = path.to_str().unwrap();
        let _ = crate::vfs::remove_file(p); // clear any stale
        let row = EvalRow {
            timestamp: EvalRow::timestamp_from_epoch(1_700_000_001),
            config_version: "1".into(),
            category: "eval".into(),
            subagent: "general".into(),
            model: "hy3".into(),
            passed: true,
            gating_failed: vec![],
            soft_failed: vec![],
            checks: vec![EvalCheck {
                name: "kalman".into(),
                passed: true,
                duration_ms: 5,
            }],
        };
        eval_row_append_to(&row, p).expect("append must succeed (fail-closed)");
        let contents = crate::vfs::read_to_string(p).expect("read back");
        assert!(contents.trim_end().ends_with('}'));
        assert!(crate::json::parse(contents.trim_end()).is_ok());
        let _ = crate::vfs::remove_file(p);
    }
}
