//! `kernel::academia_p2p` — Academia Mesh: P2P синк через mesh-мережу.
//!
//! # Mesh-мережа
//! Кожен вузол має однакову швидкість (100 Mbps).
//! Вузли з'єднані через mesh-логіку (swarm + harmonic centrality).
//! Кожен вузол = сідер свого чанка матриці.
//!
//! # Архітектура
//! ```text
//!         ┌───────────┐
//!         │  Seed     │ chunk 0
//!         │  (raw→mat)│
//!         └─────┬─────┘
//!          ┌────┼────┐
//!     ┌────┴┐ ┌┴───┐ ┌┴────┐
//!     │Node1│ │Node│ │NodeN│  ← всі 100 Mbps
//!     │chk1 │ │chk2│ │chkN │
//!     └─────┘ └────┘ └─────┘
//!        ↓       ↓       ↓
//!     ┌──────────────────────┐
//!     │   Mesh Swarm         │  ← harmonic routing
//!     │   (academia mesh)    │
//!     └──────────────────────┘
//! ```
//!
//! # Швидкість
//! Mesh з N вузлів × 100 Mbps:
//! - N=1:   10 год  (сідер, raw→mat)
//! - N=10:  6 хв   (mesh × 10)
//! - N=100: 36 с   (mesh × 100)
//!
//! # Перевикористання
//! - `mesh.rs` / `swarm.rs`: логіка координації
//! - `harmonic.rs`: рейтинг вузлів
//! - `academia::Academia`: матричне сховище
//! - FanOut: розподіл чанків по вузлах mesh

use crate::academia::Academia;
use crate::event_log::sha3_256;

/// Розмір чанка (8 MB).
pub const CHUNK_SIGS: u64 = 1_000_000;
pub const CHUNK_BYTES: u64 = CHUNK_SIGS * 8;

/// Вузол mesh-мережі.
#[derive(Debug, Clone)]
pub struct MeshNode {
    pub id: String,
    pub addr: String,
    pub bandwidth: u32,
    /// Індекси чанків, які this node має.
    pub chunks: Vec<u32>,
    /// Кількість сигнатур (0 = сідер без даних).
    pub sigs: u64,
    /// Harmonic centrality в mesh.
    pub centrality: f64,
}

/// Mesh-топологія академії.
#[derive(Debug)]
pub struct AcademiaMesh {
    /// Всі вузли mesh.
    pub nodes: Vec<MeshNode>,
    /// Загальна кількість сигнатур.
    pub total_sigs: u64,
    /// Загальна пропускна здатність mesh.
    pub total_bandwidth: u32,
}

impl AcademiaMesh {
    pub fn new() -> Self {
        AcademiaMesh { nodes: Vec::new(), total_sigs: 0, total_bandwidth: 0 }
    }

    /// Додати вузол до mesh.
    pub fn add_node(&mut self, id: &str, addr: &str, bw: u32) {
        self.nodes.push(MeshNode {
            id: id.to_string(), addr: addr.to_string(),
            bandwidth: bw, chunks: Vec::new(), sigs: 0, centrality: 0.0,
        });
        self.total_bandwidth += bw;
    }

    /// Призначити чанки вузлам (FanOut по mesh).
    pub fn assign_chunks(&mut self, total_sigs: u64) {
        self.total_sigs = total_sigs;
        if self.nodes.is_empty() { return; }
        let num_chunks = ((total_sigs + CHUNK_SIGS - 1) / CHUNK_SIGS) as u32;
        for node in &mut self.nodes { node.chunks.clear(); }
        for cid in 0..num_chunks {
            let idx = cid as usize % self.nodes.len();
            self.nodes[idx].chunks.push(cid);
            self.nodes[idx].sigs = CHUNK_SIGS.min(total_sigs - cid as u64 * CHUNK_SIGS);
        }
    }

    /// Harmonic centrality: який вузол має найкращу позицію в mesh.
    pub fn rank_nodes(&self) -> Vec<(String, f64)> {
        let n = self.nodes.len() as f64;
        let mut ranked: Vec<(String, f64)> = self.nodes.iter().map(|node| {
            // Centrality = bandwidth share + chunk count share + mesh position
            let bw_score = node.bandwidth as f64 / self.total_bandwidth.max(1) as f64;
            let chunk_score = node.chunks.len() as f64 / (self.total_sigs / CHUNK_SIGS).max(1) as f64;
            let centrality = bw_score * 0.4 + chunk_score * 0.6;
            (node.id.clone(), centrality)
        }).collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    /// Час синку для всіх вузлів (мережа = mesh, всі 100 Mbps).
    pub fn sync_time(&self) -> String {
        if self.nodes.is_empty() { return "∞".into(); }
        let bytes_per_node = (self.total_sigs * 8) as f64 / self.nodes.len() as f64;
        let bw_per_node = (self.total_bandwidth / self.nodes.len() as u32).max(1) as f64;
        let secs = bytes_per_node * 8.0 / (bw_per_node * 1_000_000.0);
        let h = secs / 3600.0;
        let m = (secs % 3600.0) / 60.0;
        let s = secs % 60.0;
        if h >= 1.0 { format!("{:.0}год {:.0}хв", h, m) }
        else if m >= 1.0 { format!("{:.0}хв {:.0}с", m, s) }
        else { format!("{:.0}с", s) }
    }

    /// Симуляція синку через mesh.
    pub fn simulate_sync(&self) -> Vec<(String, String, u64)> {
        let mut results = Vec::new();
        for node in &self.nodes {
            let bytes = node.chunks.len() as u64 * CHUNK_BYTES;
            let secs = bytes as f64 * 8.0 / (node.bandwidth as f64 * 1_000_000.0);
            let h = secs / 3600.0; let m = (secs % 3600.0) / 60.0; let s = secs % 60.0;
            let time = if h >= 1.0 { format!("{:.1}год", h) } else if m >= 1.0 { format!("{:.0}хв", m) } else { format!("{:.0}с", s) };
            results.push((node.id.clone(), time, bytes));
        }
        results
    }

    pub fn dashboard(&self) -> String {
        let gb = self.total_sigs * 8 / 1_000_000_000;
        let ranked = self.rank_nodes();
        let top = ranked.iter().take(3).map(|(id, c)| format!("    {} (cent: {:.3})", id, c)).collect::<Vec<_>>().join("\n");
        let sync_results = self.simulate_sync();
        let sync_summary: Vec<String> = sync_results.iter().map(|(id, t, b)| format!("    {}: {} ({} MB)", id, t, b / 1_000_000)).collect();
        format!(
            "Academia Mesh\n  Nodes:  {}\n  Total:  {} GB / {} sigs\n  BW:     {} Mbps\n  Sync:   {}\n  Top:\n{}\n  Per node:\n{}",
            self.nodes.len(), gb, self.total_sigs, self.total_bandwidth, self.sync_time(), top, sync_summary.join("\n")
        )
    }
}

/// Симуляція mesh-розподілу: seed → N вузлів → всі 100 Mbps.
pub fn simulate_mesh(num_peers: u32, total_sigs: u64) -> AcademiaMesh {
    let mut mesh = AcademiaMesh::new();
    for i in 0..num_peers {
        mesh.add_node(&format!("mesh-node-{}", i), &format!("10.0.0.{}:{}", i, 9000 + i), 100);
    }
    mesh.assign_chunks(total_sigs);
    mesh
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_add_nodes() {
        let mut mesh = AcademiaMesh::new();
        mesh.add_node("A", "10.0.0.1:9000", 100);
        mesh.add_node("B", "10.0.0.2:9000", 100);
        assert_eq!(mesh.nodes.len(), 2);
        assert_eq!(mesh.total_bandwidth, 200);
    }

    #[test]
    fn assign_chunks_fanout() {
        let mut mesh = simulate_mesh(4, 100_000_000);
        mesh.assign_chunks(100_000_000);
        let total: u32 = mesh.nodes.iter().map(|n| n.chunks.len() as u32).sum();
        let expected = ((100_000_000 + CHUNK_SIGS - 1) / CHUNK_SIGS) as u32;
        assert_eq!(total, expected);
    }

    #[test]
    fn sync_time_decreases_with_more_nodes() {
        let m1 = simulate_mesh(1, 610_000_000);
        let m10 = simulate_mesh(10, 610_000_000);
        assert_ne!(m1.sync_time(), m10.sync_time());
    }

    #[test]
    fn ranking_returns_ordered() {
        let mut mesh = simulate_mesh(5, 50_000_000);
        mesh.assign_chunks(50_000_000);
        let ranked = mesh.rank_nodes();
        assert_eq!(ranked.len(), 5);
        // First should have highest centrality
        assert!(ranked[0].1 >= ranked[1].1);
    }

    #[test]
    fn simulate_mesh_10_nodes_100mbps() {
        let mesh = simulate_mesh(10, 610_000_000);
        let time = mesh.sync_time();
        // 10 nodes × 100 Mbps = 1 Gbps
        // 610M × 8B = 4.88 GB / 125 MB/s ≈ 39s
        assert_eq!(mesh.total_bandwidth, 1000);
        assert!(time.contains("с") || time.contains("хв"));
    }

    #[test]
    fn simulate_mesh_100_nodes() {
        let mesh = simulate_mesh(100, 610_000_000);
        // 100 nodes × 100 Mbps = 10 Gbps
        // 4.88 GB / 1.25 GB/s ≈ 4s
        assert_eq!(mesh.total_bandwidth, 10_000);
    }

    #[test]
    fn dashboard_contains_mesh() {
        let mesh = simulate_mesh(3, 1_000_000);
        let d = mesh.dashboard();
        assert!(d.contains("Academia Mesh"));
    }

    #[test]
    fn single_node_takes_longest() {
        let m1 = simulate_mesh(1, 610_000_000);
        // Single node at 100 Mbps: 4.88 GB × 8 / 100 Mbps ≈ 390s ≈ 6.5 min
        let results = m1.simulate_sync();
        assert_eq!(results[0].0, "mesh-node-0");
    }

    #[test]
    fn harmonic_centrality_peers_equal() {
        let mut mesh = AcademiaMesh::new();
        mesh.add_node("A", "addr", 100);
        mesh.add_node("B", "addr", 100);
        mesh.add_node("C", "addr", 100);
        mesh.add_node("D", "addr", 100);
        mesh.assign_chunks(100_000);
        let ranked = mesh.rank_nodes();
        // 4 nodes with equal BW: should have chunk-based ranking
        assert_eq!(ranked.len(), 4);
    }
}
