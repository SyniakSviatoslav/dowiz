//! `living_memory_store` — crash-safe, append-only persistence for the
//! living-memory graph.
//!
//! Reuses the `brain/hydra.rs` `FileEventStore` durability pattern (lazy open,
//! append + `flush` + `sync_all`, in-memory state advances only after the
//! durable write). Every `remember` appends one record line + fsync, so a
//! crash mid-write leaves a valid prefix that replays cleanly on the next
//! `open` — nothing is forgotten between sessions or after a failure.

use dowiz_core::living_memory::{self, LivingMemory, MemoryKind};
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
    /// parser (`from_lines` ignores malformed/empty lines).
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mem = match std::fs::read_to_string(&path) {
            Ok(text) => LivingMemory::from_lines(&text),
            Err(_) => LivingMemory::new(),
        };
        Ok(Self { mem, path, handle: None })
    }

    /// Add a record and durably append it (write + flush + fsync). Crash-safe:
    /// the record is on disk before this returns.
    pub fn remember(&mut self, kind: MemoryKind, key: &str, content: &str) -> usize {
        let id = self.mem.remember(kind, key, content);
        let line = living_memory::record_to_line(self.mem.recall(id).expect("just added"));
        self.append_line(&line);
        id
    }

    /// Rewrite the whole palace atomically (temp file + fsync + rename). Use
    /// after in-place mutation (facets, links) to reconcile the durable file.
    pub fn persist(&mut self) -> std::io::Result<()> {
        let tmp = self.path.with_extension("tmp");
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(self.mem.to_lines().as_bytes())?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, &self.path)?;
        self.handle = None; // reopen lazily on next append
        Ok(())
    }

    fn append_line(&mut self, line: &str) {
        if self.handle.is_none() {
            let f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .expect("open living-memory store");
            self.handle = Some(f);
        }
        let f = self.handle.as_mut().expect("handle just set");
        f.write_all(line.as_bytes()).expect("append record");
        f.write_all(b"\n").expect("newline");
        f.flush().expect("flush");
        f.sync_all().expect("fsync"); // durable before returning
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
            s.memory_mut().add_facet(id, dowiz_core::living_memory::FacetType::Decision, "retire serde");
            s.persist().unwrap();
        }
        let s2 = LivingMemoryStore::open(&tmp).unwrap();
        let r = s2.memory().recall_by_key("mig").unwrap();
        assert_eq!(r.facets.len(), 1);
        let _ = std::fs::remove_file(&tmp);
    }
}
