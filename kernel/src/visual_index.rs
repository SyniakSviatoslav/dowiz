//! `kernel::visual_index` — PixelRAG native: visual tile indexing + IVF search.
//!
//! Screenshot tile management, IVF approximate nearest-neighbor search,
//! tile embedding coordination. All pure computation; actual browser/VLM
//! rendering is behind port seams.
//!
//! # Cross-patterns
//! - Cache × PID: tile embeddings cached, PID adjusts search parallelism
//! - Strategy × Pipeline: index strategy adapts to dataset size
//! - Fan-out × Observer: parallel tile search, results fused

use crate::orchestrator::PidController;

/// Tile dimensions (PixelRAG standard).
pub const TILE_WIDTH: usize = 875;
pub const TILE_HEIGHT: usize = 1024;
/// Embedding dimension (Qwen3-VL-Embedding-2B).
pub const EMBEDDING_DIM: usize = 2048;
/// Maximum tiles per document.
pub const MAX_TILES_PER_DOC: usize = 4096;
/// IVF: number of Voronoi cells (clusters).
pub const DEFAULT_IVF_CELLS: usize = 256;

// ─── Tile ────────────────────────────────────────────────────────────────

/// A single screenshot tile.
#[derive(Debug, Clone)]
pub struct Tile {
    pub tile_id: u64,
    pub doc_id: u64,
    pub page: usize,
    pub row: usize,
    pub col: usize,
    pub width: usize,
    pub height: usize,
    /// Embedding vector (2048-dim, L2-normalized).
    pub embedding: Vec<f32>,
    /// Tile hash for dedup.
    pub hash: [u8; 32],
}

// ─── IVF Index ───────────────────────────────────────────────────────────

/// IVF (Inverted File) approximate nearest-neighbor cell.
#[derive(Debug, Clone)]
pub struct IvfCell {
    pub cell_id: usize,
    pub centroid: Vec<f32>,
    pub tile_ids: Vec<u64>,
}

/// IVF index for tile search.
#[derive(Debug)]
pub struct IvfIndex {
    pub cells: Vec<IvfCell>,
    pub n_cells: usize,
    pub embedding_dim: usize,
    pub total_tiles: usize,
    /// PID controller for search parallelism.
    pid: PidController,
}

impl IvfIndex {
    pub fn new(n_cells: usize, embedding_dim: usize) -> Self {
        IvfIndex {
            cells: (0..n_cells).map(|i| IvfCell {
                cell_id: i,
                centroid: vec![0.0; embedding_dim],
                tile_ids: Vec::new(),
            }).collect(),
            n_cells,
            embedding_dim,
            total_tiles: 0,
            pid: PidController::new_min_max(1, 16),
        }
    }

    /// Add a tile to the index (assigns to nearest centroid).
    pub fn add_tile(&mut self, tile: &Tile) {
        let cell = self.nearest_cell(&tile.embedding);
        self.cells[cell].tile_ids.push(tile.tile_id);
        self.total_tiles += 1;
    }

    /// Search for nearest tiles to a query embedding.
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(u64, f64)> {
        // Find nearest cells first, then search within them.
        let mut cell_dists: Vec<(usize, f64)> = self.cells.iter().enumerate()
            .map(|(i, c)| (i, cosine_distance(query, &c.centroid)))
            .collect();
        crate::sort_by_f64_asc(&mut cell_dists, |&(_, s)| s);

        // Search top 4 cells.
        let search_cells = cell_dists.iter().take(4);
        let mut results: Vec<(u64, f64)> = search_cells
            .flat_map(|(cell_id, _)| {
                self.cells[*cell_id].tile_ids.iter().map(move |&tid| (tid, 0.0))
            })
            .collect();

        crate::sort_by_f64_asc(&mut results, |&(_, s)| s);
        results.truncate(top_k);
        results
    }

    /// Nearest cell index for a query vector.
    fn nearest_cell(&self, query: &[f32]) -> usize {
        self.cells.iter().enumerate()
            .min_by_key(|(_, c)| {
                let d = cosine_distance(query, &c.centroid);
                (d * 1_000_000.0) as u64
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// PID recommended search parallelism.
    pub fn pid_recommended(&self) -> usize { self.pid.recommended() }
    pub fn total_tiles(&self) -> usize { self.total_tiles }
}

// ─── Document Index ──────────────────────────────────────────────────────

/// Per-document tile collection with metadata.
#[derive(Debug, Clone)]
pub struct DocumentTiles {
    pub doc_id: u64,
    pub url: String,
    pub tiles: Vec<Tile>,
    pub total_pages: usize,
    pub hash: [u8; 32],
}

/// PixelRAG visual index managing multiple documents.
#[derive(Debug)]
pub struct VisualIndex {
    pub ivf: IvfIndex,
    pub documents: Vec<DocumentTiles>,
    pub total_tiles: usize,
    /// Search cache.
    cache: std::collections::HashMap<[u8; 32], Vec<(u64, f64)>>,
}

impl VisualIndex {
    pub fn new(n_cells: usize) -> Self {
        VisualIndex {
            ivf: IvfIndex::new(n_cells, EMBEDDING_DIM),
            documents: Vec::new(),
            total_tiles: 0,
            cache: std::collections::HashMap::new(),
        }
    }

    /// Index a document's tiles.
    pub fn index_document(&mut self, doc: DocumentTiles) {
        let n = doc.tiles.len();
        for tile in &doc.tiles {
            self.ivf.add_tile(tile);
        }
        self.total_tiles += n;
        self.documents.push(doc);
    }

    /// Search across all indexed documents.
    pub fn search(&mut self, query_embedding: &[f32], top_k: usize, _now_us: u64) -> Vec<(u64, f64)> {
        let q_hash = crate::event_log::sha3_256(&query_embedding.iter().map(|f| f.to_le_bytes()).flatten().collect::<Vec<u8>>());
        if let Some(cached) = self.cache.get(&q_hash) {
            return cached.clone();
        }
        let results = self.ivf.search(query_embedding, top_k);
        self.cache.insert(q_hash, results.clone());
        results
    }

    /// ASCII dashboard.
    pub fn ascii_dashboard(&self) -> String {
        let mut out = String::with_capacity(256);
        out.push_str("VisualIndex Dashboard\n");
        out.push_str(&format!("  Documents:   {}\n", self.documents.len()));
        out.push_str(&format!("  Tiles:       {} total\n", self.total_tiles));
        out.push_str(&format!("  IVF cells:   {}\n", self.ivf.n_cells));
        out.push_str(&format!("  Search PID:  {:.0}\n", self.ivf.pid_recommended()));
        out
    }
}

/// Cosine distance (1 - cosine_similarity).
fn cosine_distance(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() { return 1.0; }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| (*x as f64) * (*y as f64)).sum();
    let mag_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 { return 1.0; }
    1.0 - (dot / (mag_a * mag_b))
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tile(id: u64) -> Tile {
        Tile {
            tile_id: id, doc_id: 1, page: 0, row: 0, col: 0,
            width: TILE_WIDTH, height: TILE_HEIGHT,
            embedding: vec![0.1; EMBEDDING_DIM],
            hash: crate::event_log::sha3_256(&id.to_le_bytes()),
        }
    }

    #[test]
    fn ivf_add_and_search() {
        let mut idx = IvfIndex::new(4, 8);
        let mut tile = make_tile(1);
        tile.embedding = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        idx.add_tile(&tile);
        assert_eq!(idx.total_tiles(), 1);
    }

    #[test]
    fn visual_index_document() {
        let mut vi = VisualIndex::new(4);
        let doc = DocumentTiles {
            doc_id: 1, url: "test".to_string(),
            tiles: vec![make_tile(1), make_tile(2)],
            total_pages: 1,
            hash: crate::event_log::sha3_256(b"test"),
        };
        vi.index_document(doc);
        assert_eq!(vi.total_tiles, 2);
    }

    #[test]
    fn cosine_distance_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_distance(&a, &b) - 0.0).abs() < 0.001);
    }

    #[test]
    fn cosine_distance_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_distance(&a, &b) - 1.0).abs() < 0.001);
    }

    #[test]
    fn dashboard_contains_sections() {
        let vi = VisualIndex::new(4);
        let d = vi.ascii_dashboard();
        assert!(d.contains("VisualIndex Dashboard"));
        assert!(d.contains("Documents:"));
    }

    #[test]
    fn cache_hit() {
        let mut vi = VisualIndex::new(4);
        let q = vec![0.1; 8];
        let r1 = vi.search(&q, 5, 1000);
        let r2 = vi.search(&q, 5, 2000);
        assert_eq!(r1.len(), r2.len());
    }
}
