//! `living_memory_store` — crash-safe, append-only persistence for the
//! living-memory graph, in Markdown (`.md`) format.
//!
//! The durable store is a human-readable Markdown document: one `## heading`
//! + `Summary:` line per record, plus a fenced `record` block carrying the
//! exact round-trip line (pre-computed hypervector code included). The fence is
//! the only machine-parsed part, so the headings are free to be hand-edited.
//!
//! Durability reuses the `brain/hydra.rs` `FileEventStore` pattern (lazy open,
//! append + `flush` + `sync_all`, in-memory state advances only after the
//! durable write). Every `remember` appends one Markdown block + fsync, so a
//! crash mid-write leaves a valid prefix that replays cleanly on the next
//! `open` — nothing is forgotten between sessions or after a failure.
//!
//! A binary `.idx` sidecar mirrors the hypervector codes: on `open`, if it is
//! at least as new as the `.md`, the codes load straight from it (one read,
//! zero re-hash — the cold-start killer). If it is missing/stale/corrupt, the
//! `.md` hex codes are used instead (already re-hash-free), so the fallback is
//! correct but touches more bytes.

use dowiz_core::living_memory::{LivingMemory, MemoryKind};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct LivingMemoryStore {
    mem: LivingMemory,
    path: PathBuf,
    handle: Option<std::fs::File>,
}

impl LivingMemoryStore {
    /// Open (or create) the store at `path`, replaying any existing records.
    /// A missing file is tolerated; a torn trailing line is skipped by the
    /// parser (`from_md` ignores malformed/empty fence blocks). Legacy
    /// line-based stores (pre-`.md`) are read via `from_lines` as a fallback.
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut store = Self {
            mem: LivingMemory::new(),
            path,
            handle: None,
        };
        // Fast path: load the full palace from the binary `.idx` sidecar (one
        // read, zero re-hash, zero Markdown parse). Falls back to the Markdown
        // source of truth when the sidecar is missing/stale/corrupt.
        if !store.load_from_sidecar_if_fresh() {
            store.mem = match std::fs::read_to_string(&store.path) {
                Ok(text) => {
                    let mut m = LivingMemory::from_md(&text);
                    if m.record_count() == 0 && !text.trim().is_empty() {
                        // No fenced records — a legacy tab-separated store.
                        m = LivingMemory::from_lines(&text);
                    }
                    m
                }
                Err(_) => LivingMemory::new(),
            };
        }
        Ok(store)
    }

    /// Add a record and durably append it (write + flush + fsync). Crash-safe:
    /// the record is on disk before this returns.
    pub fn remember(&mut self, kind: MemoryKind, key: &str, content: &str) -> usize {
        let id = self.mem.remember(kind, key, content);
        let block = self.mem.record_to_md_block(id).expect("record just added");
        self.append_block(&block);
        id
    }

    /// Rewrite the whole palace atomically (temp file + fsync + rename), then
    /// mirror the hypervector codes to the `.idx` sidecar. Use after in-place
    /// mutation (facets, links) to reconcile the durable files.
    pub fn persist(&mut self) -> std::io::Result<()> {
        let tmp = self.path.with_extension("md.tmp");
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(self.mem.to_md().as_bytes())?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, &self.path)?;
        self.write_sidecar()?;
        self.handle = None; // reopen lazily on next append
        Ok(())
    }

    fn append_block(&mut self, block: &str) {
        if self.handle.is_none() {
            let f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .expect("open living-memory store");
            self.handle = Some(f);
        }
        let f = self.handle.as_mut().expect("handle just set");
        f.write_all(block.as_bytes()).expect("append record");
        f.flush().expect("flush");
        f.sync_all().expect("fsync"); // durable before returning
    }

    fn sidecar_path(&self) -> PathBuf {
        self.path.with_extension("idx")
    }

    /// Mirror the full palace to the `.idx` sidecar (atomic temp+rename) — the
    /// binary fast-load path that skips the Markdown parse entirely.
    fn write_sidecar(&self) -> std::io::Result<()> {
        let tmp = self.sidecar_path().with_extension("idx.tmp");
        std::fs::write(&tmp, self.mem.to_binary_full())?;
        std::fs::rename(&tmp, self.sidecar_path())?;
        Ok(())
    }

    /// Load the full palace from the `.idx` sidecar, but only if it is at least
    /// as new as the `.md` (i.e. written by the latest `persist`). Returns `true`
    /// on a successful load. A stale/corrupt sidecar is ignored — the caller
    /// falls back to the Markdown source of truth.
    fn load_from_sidecar_if_fresh(&mut self) -> bool {
        let idx_path = self.sidecar_path();
        let fresh = match (
            std::fs::metadata(&self.path),
            std::fs::metadata(&idx_path),
        ) {
            (Ok(md), Ok(idx)) => match (idx.modified(), md.modified()) {
                (Ok(ia), Ok(ma)) => ia >= ma,
                _ => false,
            },
            _ => false,
        };
        if !fresh {
            return false;
        }
        match std::fs::read(&idx_path) {
            Ok(bytes) => match LivingMemory::from_binary_full(&bytes) {
                Some(m) => {
                    self.mem = m;
                    true
                }
                None => false,
            },
            Err(_) => false,
        }
    }

    pub fn memory(&self) -> &LivingMemory {
        &self.mem
    }
    pub fn memory_mut(&mut self) -> &mut LivingMemory {
        &mut self.mem
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_persists_across_reopen() {
        let tmp = std::env::temp_dir().join(format!("lm_store_{}", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        {
            let mut s = LivingMemoryStore::open(&tmp).unwrap();
            s.remember(MemoryKind::Semantic, "auth", "auth uses pq::dsa");
            s.remember(MemoryKind::Episodic, "session", "wave-59 migrated");
        }
        // Reopen → replay from disk.
        let s2 = LivingMemoryStore::open(&tmp).unwrap();
        assert_eq!(s2.memory().by_kind(MemoryKind::Semantic).len(), 1);
        assert_eq!(s2.memory().by_kind(MemoryKind::Episodic).len(), 1);
        assert_eq!(
            s2.memory().recall_by_key("auth").unwrap().content,
            "auth uses pq::dsa"
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn persist_reconciles_full_snapshot() {
        let tmp = std::env::temp_dir().join(format!("lm_snap_{}", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        {
            let mut s = LivingMemoryStore::open(&tmp).unwrap();
            let id = s.remember(MemoryKind::Semantic, "mig", "wave-59");
            s.memory_mut().add_facet(
                id,
                dowiz_core::living_memory::FacetType::Decision,
                "retire serde",
            );
            s.persist().unwrap();
        }
        let s2 = LivingMemoryStore::open(&tmp).unwrap();
        let r = s2.memory().recall_by_key("mig").unwrap();
        assert_eq!(r.facets.len(), 1);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn md_store_is_human_readable() {
        let tmp = std::env::temp_dir().join(format!("lm_md_{}", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        {
            let mut s = LivingMemoryStore::open(&tmp).unwrap();
            s.remember(MemoryKind::Semantic, "quantum", "superposition and oracles");
            s.persist().unwrap();
        }
        let text = std::fs::read_to_string(&tmp).unwrap();
        assert!(text.contains("# dowiz living memory"), "md header present");
        assert!(text.contains("## 0 · quantum · Semantic"), "human heading");
        assert!(text.contains("```record"), "fenced machine line");
        // And the sidecar exists.
        assert!(std::fs::metadata(tmp.with_extension("idx")).is_ok());
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(tmp.with_extension("idx"));
    }
}
