//! `retrieval/bm25.rs` — std shim over the pure no_std core.
//!
//! The Okapi BM25 ranker ([`Bm25`]/[`Document`]/[`Bm25Params`] + scoring/encode/decode) lives
//! in `dowiz_core::retrieval::bm25` and is re-exported here. This shim adds ONLY the std
//! on-disk persistence (P95 Option A) as free functions: [`bm25_save`]/[`bm25_load`].

pub use dowiz_core::retrieval::bm25::*;

/// Persist a BM25 index to a std-only on-disk file (P95 Option A).
pub fn bm25_save(bm: &Bm25, path: &std::path::Path) -> Result<(), String> {
    crate::vfs::write(path, bm.encode())
        .map_err(|e| format!("bm25_save {}: {e}", path.display()))
}

/// Load a persisted BM25 index from a std-only on-disk file (P95 Option A).
pub fn bm25_load(path: &std::path::Path) -> Result<Bm25, String> {
    let buf =
        crate::vfs::read(path).map_err(|e| format!("bm25_load {}: {e}", path.display()))?;
    Bm25::decode(&buf).ok_or_else(|| format!("bm25_load {}: corrupt index", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_load_roundtrips_byte_identical() {
        let docs = vec![
            Document::from_text("the quick brown fox jumps over the lazy dog"),
            Document::from_text("never gonna give you up never gonna let you down"),
            Document::from_text("to be or not to be that is the question"),
        ];
        let bm = Bm25::new(docs);
        let path =
            std::env::temp_dir().join(format!("bm25_persist_test_{}.bin", std::process::id()));
        bm25_save(&bm, &path).expect("save");
        let loaded = bm25_load(&path).expect("load");
        crate::vfs::remove_file(&path).ok();
        assert_eq!(loaded.encode(), bm.encode(), "loaded index must be byte-identical");
    }
}
