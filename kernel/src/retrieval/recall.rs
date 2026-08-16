//! retrieval/recall.rs — std shim over the no_std recall core.
//!
//! The pure ranker (`PrimaryRecall`, `build_fusion`, `fusion_rank`, the fixture
//! corpus, `file_stem`, and the `encode_index`/`from_parts`/`trigram_docs`/
//! `stems` accessors) lives in `dowiz_core::retrieval::recall` and is re-exported
//! here. This shim adds ONLY the std-dependent pieces:
//!   - the disk persistence free functions (`from_dir`, `save`, `load`, plus
//!     `save_to`/`load_from`/`stem_list`) — they route through the kernel's
//!     std-backed `crate::vfs` (core's `crate::vfs` is the no_std seam that
//!     returns `NotFound`, so the I/O must live here, exactly like
//!     `retrieval/ppr`/`diffusion` keep their disk round-trips in `std::fs`);
//!   - the lazy `OnceLock`-backed global `PRIMARY` instance + `primary()` accessor
//!     + the `recall_at_k` free function (the self-improvement loop's entry point);
//!   - the std-only disk round-trip + kill-9 tests.
//!
//! NOTE: the `LivingKnowledge` adapter impl for `PrimaryRecall` now lives in the core
//! (`dowiz_core::retrieval::recall`), since both the trait and the type are core items — an
//! impl here would be an orphan-rule violation.

pub use dowiz_core::retrieval::recall::*;

use alloc::string::String;
use alloc::vec::Vec;
use dowiz_core::retrieval::bm25::{Bm25, Document};
use dowiz_core::retrieval::index::TrigramIndex;

/// Lazy-initialized PRIMARY recall instance — the shared kernel recall source.
static PRIMARY: std::sync::OnceLock<PrimaryRecall> = std::sync::OnceLock::new();

fn primary() -> &'static PrimaryRecall {
    PRIMARY.get_or_init(PrimaryRecall::new)
}

/// PRIMARY recall entry point used by the self-improvement loop (W18.2).
///
/// Thin, deterministic, std-only wrapper over [`PrimaryRecall::recall_at_k`]
/// (kernel-owned BM25+trigram fusion). The (wasm-gated) `living_knowledge`
/// adapter delegates its lexical recall here. No JS, no network.
pub fn recall_at_k(query: &str, k: usize) -> Vec<(String, f64)> {
    primary().recall_at_k(query, k)
}

/// Ingest a real memory corpus from `dir` — every `*.md` file becomes one
/// document, keyed by its file stem (e.g. `MEMORY` from `MEMORY.md`).
/// Directory walk via the kernel's std-backed `crate::vfs`; no recursion into
/// subdirs (the living-memory corpus is flat). Fail-closed: a directory that
/// yields zero `*.md` files errors rather than returning an empty ranker.
pub fn from_dir(dir: &str) -> Result<PrimaryRecall, String> {
    let entries = crate::vfs::read_dir(dir)
        .map_err(|e| format!("from_dir: read_dir {dir}: {e}"))?;
    let mut paths: Vec<String> = Vec::new();
    for e in entries {
        if e.extension() == Some("md") {
            paths.push(e.path);
        }
    }
    if paths.is_empty() {
        return Err(format!("from_dir: no *.md files in {dir}"));
    }
    // Stable order ⇒ deterministic index regardless of read_dir iteration order.
    paths.sort();
    let texts: Vec<String> = paths
        .iter()
        .map(|p| crate::vfs::read_to_string(p).unwrap_or_default())
        .collect();
    let docs: Vec<Document> = texts.iter().map(|s| Document::from_text(s)).collect();
    let strs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    let bm = Bm25::new(docs);
    let idx = TrigramIndex::new(&strs);
    let ids: Vec<String> = paths
        .iter()
        .map(|p| file_stem(p).unwrap_or("?").to_string())
        .collect();
    Ok(PrimaryRecall::from_parts(bm, idx, ids))
}

/// The deterministic corpus stem list (sorted, matching `from_dir`'s sort).
fn stem_list(dir: &str) -> Result<Vec<String>, String> {
    let mut paths: Vec<String> = Vec::new();
    for e in crate::vfs::read_dir(dir)
        .map_err(|e| format!("stem_list: read_dir {dir}: {e}"))?
    {
        if e.extension() == Some("md") {
            paths.push(e.path);
        }
    }
    paths.sort();
    Ok(paths
        .iter()
        .map(|p| file_stem(p).unwrap_or("?").to_string())
        .collect())
}

/// Persist the built index to `path` (through the kernel's std-backed vfs).
pub fn save_to(recall: &PrimaryRecall, path: &str) -> Result<(), String> {
    let blob = recall.encode_index();
    // Prefix the trigram docs so load can rebuild the TrigramIndex
    // deterministically, and store the STEM LIST (the dirty fingerprint) so
    // `load` can detect a changed corpus without re-reading every file.
    let mut trig = Vec::new();
    for d in recall.trigram_docs() {
        trig.extend_from_slice(&(d.len() as u64).to_le_bytes());
        trig.extend_from_slice(d.as_bytes());
    }
    let mut stems = Vec::new();
    for s in recall.stems() {
        stems.extend_from_slice(&(s.len() as u64).to_le_bytes());
        stems.extend_from_slice(s.as_bytes());
    }
    let mut out = (trig.len() as u64).to_le_bytes().to_vec();
    out.extend_from_slice(&trig);
    out.extend_from_slice(&(stems.len() as u64).to_le_bytes());
    out.extend_from_slice(&stems);
    out.extend_from_slice(&blob);
    crate::vfs::write(path, &out).map_err(|e| format!("save_to {path}: {e}"))
}

/// Load a persisted index from `path`. Rebuilds both `Bm25` and
/// `TrigramIndex` byte-deterministically. Returns the index plus the stored
/// stem list (the dirty fingerprint) so `load` can compare.
pub fn load_from(path: &str) -> Result<(PrimaryRecall, Vec<String>), String> {
    let buf = crate::vfs::read(path).map_err(|e| format!("load_from {path}: {e}"))?;
    let trig_len =
        u64::from_le_bytes(<[u8; 8]>::try_from(&buf[0..8]).map_err(|_| "corrupt header")?)
            as usize;
    let mut p = 8;
    let mut trig_docs = Vec::new();
    let end_trig = p + trig_len;
    if end_trig > buf.len() {
        return Err("load_from: corrupt trigram section".into());
    }
    while p < end_trig {
        let l = u64::from_le_bytes(<[u8; 8]>::try_from(&buf[p..p + 8]).map_err(|_| "corrupt")?)
            as usize;
        p += 8;
        let s = &buf[p..p + l];
        trig_docs.push(String::from_utf8(s.to_vec()).map_err(|_| "corrupt utf8")?);
        p += l;
    }
    let stems_len = u64::from_le_bytes(
        <[u8; 8]>::try_from(&buf[p..p + 8]).map_err(|_| "corrupt stems header")?,
    ) as usize;
    p += 8;
    let end_stems = p + stems_len;
    if end_stems > buf.len() {
        return Err("load_from: corrupt stems section".into());
    }
    let mut stems = Vec::new();
    while p < end_stems {
        let l = u64::from_le_bytes(<[u8; 8]>::try_from(&buf[p..p + 8]).map_err(|_| "corrupt")?)
            as usize;
        p += 8;
        let s = &buf[p..p + l];
        stems.push(String::from_utf8(s.to_vec()).map_err(|_| "corrupt utf8")?);
        p += l;
    }
    let bm = Bm25::decode(&buf[p..])
        .ok_or_else(|| "load_from: corrupt bm25".to_string())?;
    let idx = TrigramIndex::new(&trig_docs.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    // ids are the persisted stems (the dirty fingerprint), NOT the doc bodies.
    Ok((PrimaryRecall::from_parts(bm, idx, stems.clone()), stems))
}

/// Save to the default cache file next to `dir` (`<dir>/.primary_recall.idx`).
/// Overwrites any previous cache.
pub fn save(recall: &PrimaryRecall, dir: &str) -> Result<(), String> {
    let path = format!("{dir}/.primary_recall.idx");
    save_to(recall, &path)
}

/// Load from the default cache file next to `dir`; returns the persisted
/// index only if its stored stem list matches the live directory (else
/// `Ok(None)` — caller falls back to `from_dir`).
pub fn load(dir: &str) -> Result<Option<PrimaryRecall>, String> {
    let path = format!("{dir}/.primary_recall.idx");
    if crate::vfs::metadata(&path).is_err() {
        return Ok(None);
    }
    let (cached, stems) = load_from(&path)?;
    let live_stems = stem_list(dir)?;
    if live_stems == stems {
        Ok(Some(cached))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hermetic-audit Cause-and-Effect Finding B: the ranking must survive a real
    /// disk serialization boundary and match an independently fresh recompute.
    #[test]
    fn fusion_ranking_survives_serialize_reread_boundary() {
        let (bm, idx) = build_fusion();
        let computed = fusion_rank(&bm, &idx, "how is the order total calculated");
        let serialized = computed
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let path = std::env::temp_dir().join(format!(
            "fusion_rank_reread_test_{}.txt",
            std::process::id()
        ));
        crate::vfs::write(&path, &serialized).expect("write serialized ranking");
        let reread = crate::vfs::read_to_string(&path).expect("re-read serialized ranking");
        crate::vfs::remove_file(&path).ok();

        assert_eq!(reread, serialized, "byte content did not survive a disk round-trip");

        let reparsed: Vec<usize> = if reread.is_empty() {
            Vec::new()
        } else {
            reread
                .split(',')
                .map(|s| s.parse::<usize>().expect("reparse usize"))
                .collect()
        };
        let fresh = fusion_rank(&bm, &idx, "how is the order total calculated");
        assert_eq!(
            reparsed, fresh,
            "value re-read from disk does not match an independently fresh computation"
        );
    }

    /// P95 acceptance #1 — the kill-9 / restart primary proof, exercised in-process:
    /// build a `PrimaryRecall` over a real on-disk `.md` corpus, `save` it, then
    /// `load` it back and confirm the cached index is byte-identical to a fresh
    /// `from_dir` rebuild AND re-ranks identically.
    #[test]
    fn primary_recall_survives_kill9_restart() {
        let dir =
            std::env::temp_dir().join(format!("primary_recall_kill9_{}", std::process::id()));
        crate::vfs::create_dir_all(&dir).expect("mk corpus dir");
        let dir_s = dir.to_str().unwrap();
        // Write a small deterministic corpus of .md docs (sorted stems).
        let docs = [
            ("a_order_total.md", "how is the order total calculated by the engine"),
            ("b_refund.md", "request a refund for a cancelled order"),
            ("c_shipping.md", "shipping delay and delivery estimate for my package"),
            ("d_loyalty.md", "loyalty points balance and how to redeem rewards"),
            ("e_invoice.md", "download the invoice pdf for last months purchase"),
        ];
        for (name, body) in docs {
            crate::vfs::write(dir.join(name), body).expect("write corpus doc");
        }
        // Build + save (simulates the running process persisting its index).
        let built = from_dir(dir_s).expect("from_dir");
        save(&built, dir_s).expect("save index cache");
        // Simulate restart: load the persisted cache.
        let loaded = load(dir_s)
            .expect("load cached index")
            .expect("cache must be fresh (stem list unchanged)");
        // Byte-identical index: deterministic codec ⇒ same encode().
        assert_eq!(
            built.encode_index(),
            loaded.encode_index(),
            "persisted index must be byte-identical to a fresh build"
        );
        // Re-rank identically after the 'restart'.
        let q = "how is the order total calculated";
        assert_eq!(
            built.recall_at_k(q, 5),
            loaded.recall_at_k(q, 5),
            "ranking must survive the kill-9/restart boundary"
        );
        // Dirty check: corrupt the corpus (add a doc) ⇒ load returns None (caller
        // must rebuild), proving the fingerprint actually detects change.
        crate::vfs::write(dir.join("f_promo.md"), "promo code discount applied at checkout")
            .expect("add doc");
        let stale = load(dir_s).expect("load after corpus change");
        assert!(
            stale.is_none(),
            "load must detect a changed corpus and refuse a stale cache"
        );
        // Cleanup.
        let _ = crate::vfs::remove_dir_all(&dir);
    }
}
