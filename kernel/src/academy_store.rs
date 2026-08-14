//! Academy internal store for Hydra.
//!
//! Each Hydra organism owns a local append-only academy journal and a
//! searchable view over shared Academia entries. Every append is logged
//! with full metadata; every read is deterministic.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AcademyEntry {
    pub ts_ns: u64,
    pub cycle: u64,
    pub actor: [u8; 32],
    pub title: String,
    pub source: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Default)]
pub struct AcademyStore {
    pub entries: Vec<AcademyEntry>,
    pub search_index: BTreeMap<String, Vec<usize>>,
}

impl AcademyStore {
    /// Append a new record to the journal.
    pub fn append(
        &mut self,
        cycle: u64,
        actor: [u8; 32],
        title: impl Into<String>,
        source: impl Into<String>,
        tags: Vec<String>,
    ) -> usize {
        let entry = AcademyEntry {
            ts_ns: 0,
            cycle,
            actor,
            title: title.into(),
            source: source.into(),
            tags,
        };
        let idx = self.entries.len();
        self.entries.push(entry);
        self.reindex(idx);
        idx
    }

    fn reindex(&mut self, idx: usize) {
        let entry = &self.entries[idx];
        let norm = entry.title.to_lowercase();
        self.search_index.entry(norm).or_default().push(idx);
    }

    /// Search local journal by title substring. Returns matched indices.
    pub fn search(&self, query: &str) -> Vec<usize> {
        let q = query.to_lowercase();
        self.search_index
            .iter()
            .filter_map(|(key, idxs)| {
                if key.contains(&q) {
                    Some(idxs.clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }

    /// Get an entry by index.
    pub fn get(&self, idx: usize) -> Option<&AcademyEntry> {
        self.entries.get(idx)
    }

    /// Total entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Persist academy entries to a JSONL file (one record per line, appended via
/// the [`crate::vfs`] seam — crash-tolerant O_APPEND, same as the FDR ring).
pub fn journal_write(path: PathBuf, entries: &[AcademyEntry]) {
    let mut buf = String::new();
    for entry in entries {
        buf.push_str(&format!(
            "{{\"ts_ns\":{},\"cycle\":{},\"actor\":\"{}\",\"title\":\"{}\",\"source\":\"{}\",\"tags\":{:?}}}\n",
            entry.ts_ns,
            entry.cycle,
            crate::hex_util::encode(&entry.actor),
            json_escape(&entry.title),
            json_escape(&entry.source),
            entry.tags
        ));
    }
    let _ = crate::vfs::append(&path, &buf);
}

/// Helper to escape JSON strings manually. Order is load-bearing: the backslash
/// is escaped FIRST so the subsequent `"` / control-char escapes are not
/// themselves re-escaped. Escapes `"` as well — a JSON string without the quote
/// escape is invalid and would corrupt the journal line.
fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_then_search_is_case_insensitive_substring() {
        let mut store = AcademyStore::default();
        store.append(0, [0u8; 32], "Quantum Lattice Theory", "arxiv", vec![]);
        store.append(1, [1u8; 32], "Crystal Diffraction", "arxiv", vec![]);
        assert_eq!(store.len(), 2);
        // Lowercased index → case-insensitive substring match.
        assert_eq!(store.search("lattice"), vec![0]);
        assert_eq!(store.search("CRYSTAL"), vec![1]);
        assert_eq!(store.search("diff"), vec![1]);
        assert!(store.search("missing").is_empty());
        assert_eq!(store.get(0).unwrap().title, "Quantum Lattice Theory");
    }

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
