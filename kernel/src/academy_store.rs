//! `academy_store.rs` — std shim over the pure no_std core.
//!
//! The journal types ([`AcademyEntry`]/[`AcademyStore`]) and the deterministic
//! [`journal_to_jsonl`] serializer live in `dowiz_core::academy_store` and are re-exported
//! here. This shim adds ONLY the std file append ([`journal_write`]) — crash-tolerant
//! O_APPEND via the [`crate::vfs`] seam, same discipline as the FDR ring.

pub use dowiz_core::academy_store::*;

use std::path::PathBuf;

/// Persist academy entries to a JSONL file (one record per line, appended via the
/// [`crate::vfs`] seam — crash-tolerant O_APPEND, same as the FDR ring).
pub fn journal_write(path: PathBuf, entries: &[AcademyEntry]) {
    let buf = journal_to_jsonl(entries);
    let _ = crate::vfs::append(&path, &buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_write_appends_jsonl_and_roundtrips() {
        let dir = std::env::temp_dir().join(format!("academy_store_{}", std::process::id()));
        let _ = crate::vfs::create_dir_all(&dir);
        let path = dir.join("journal.jsonl");
        let entries = vec![AcademyEntry {
            ts_ns: 1,
            cycle: 0,
            actor: [0xAB; 32],
            title: "Title \"quoted\"\nline".into(),
            source: "src".into(),
            tags: vec!["a".into()],
        }];
        journal_write(path.clone(), &entries);
        let text = crate::vfs::read_to_string(&path).expect("read back journal");
        assert!(text.contains("\"actor\":\""), "actor hex must be present");
        assert!(text.contains("Title \\\"quoted\\\""), "title JSON-escaped");
        let _ = crate::vfs::remove_dir_all(&dir);
    }
}
