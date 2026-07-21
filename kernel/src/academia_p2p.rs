//! `kernel::academia_p2p` — P2P distribution: peers → parallel chunks → merge.
//!
//! # Аналогія з рекурсивним пошуком
//! Пошук: split matrix → workers → merge → recursion.
//! Завантаження: split snapshot → peers → merge → recursion.
//! Одна логіка: FanOut + Merge, тільки для даних а не запитів.
//!
//! # Архітектура
//! ```text
//! [Seed Peer] → chunks bloom → [Peer A: chunk 0-99]
//!                              → [Peer B: chunk 100-199]
//!                              → [Peer C: chunk 200-299]
//!                              → ...
//! [Requestor] → bloom exchange → request chunks → merge → snapshot
//! ```
//!
//! # Швидкість
//! | Peers | Per peer | Total BW | 460 GB |
//! |-------|----------|----------|--------|
//! | 1     | 100 Mbps | 100 Mbps | 10 год |
//! | 10    | 100 Mbps | 1 Gbps   | 1 год  |
//! | 50    | 100 Mbps | 5 Gbps   | 12 хв  |
//! | 100   | 100 Mbps | 10 Gbps  | 6 хв   |
//!
//! # P2P протокол (мінімальний)
//! 1. Connect: peer → peer (TCP/QUIC)
//! 2. Exchange: bloom filter (hashes I have)
//! 3. Diff: which chunks each peer needs
//! 4. Transfer: parallel chunk download
//! 5. Merge: reassemble chunks → matrix snapshot

use crate::event_log::sha3_256;

/// Розмір chunk (1M signatures = 8 MB).
pub const CHUNK_SIGS: usize = 1_000_000;
pub const CHUNK_BYTES: usize = CHUNK_SIGS * 8;

/// P2P peer descriptor.
#[derive(Debug, Clone)]
pub struct P2pPeer {
    pub id: String,
    pub addr: String,
    pub latency_ms: u32,
    pub bandwidth_mbps: u32,
    /// Which chunks this peer has (bloom filter response).
    pub chunks_have: Vec<bool>,
    /// Total signatures this peer has.
    pub total_sigs: u64,
}

/// Chunk descriptor for parallel download.
#[derive(Debug, Clone)]
pub struct ChunkSpec {
    pub chunk_id: u32,
    pub start_sig: u64,
    pub count: u32,
    pub byte_offset: u64,
    pub byte_len: u32,
}

/// FanOut download plan: which chunks from which peers.
#[derive(Debug, Clone)]
pub struct DownloadPlan {
    pub chunks: Vec<ChunkSpec>,
    pub peer_assignments: Vec<(String, Vec<u32>)>, // peer_id → chunk_ids
    pub total_bytes: u64,
    pub estimated_bandwidth_mbps: u32,
    pub estimated_seconds: u64,
}

impl DownloadPlan {
    /// Plan parallel download across available peers.
    pub fn plan(total_sigs: u64, peers: &[P2pPeer], total_bandwidth_mbps: u32) -> Self {
        let num_chunks = ((total_sigs + CHUNK_SIGS as u64 - 1) / CHUNK_SIGS as u64) as u32;
        let chunks: Vec<ChunkSpec> = (0..num_chunks).map(|cid| {
            let start = cid as u64 * CHUNK_SIGS as u64;
            let count = (CHUNK_SIGS as u64).min(total_sigs - start) as u32;
            ChunkSpec {
                chunk_id: cid, start_sig: start, count,
                byte_offset: start * 8, byte_len: count * 8,
            }
        }).collect();

        // FanOut: assign chunks to peers round-robin.
        let mut assignments: Vec<(String, Vec<u32>)> = peers.iter().map(|p| (p.id.clone(), Vec::new())).collect();
        for (i, chunk) in chunks.iter().enumerate() {
            let peer_idx = i % peers.len().max(1);
            if peer_idx < assignments.len() {
                assignments[peer_idx].1.push(chunk.chunk_id);
            }
        }

        let total_bytes = total_sigs * 8;
        let bw = total_bandwidth_mbps.max(1);
        let est_secs = (total_bytes * 8) as u64 / (bw as u64 * 1_000_000).max(1);

        DownloadPlan { chunks, peer_assignments: assignments, total_bytes, estimated_bandwidth_mbps: bw, estimated_seconds: est_secs }
    }

    /// Estimated time with N peers.
    pub fn estimate_time(total_gb: f64, num_peers: u32, per_peer_mbps: u32) -> String {
        let total_bw = num_peers as f64 * per_peer_mbps as f64; // Mbps
        let total_bits = total_gb * 8.0 * 1024.0; // Gb → Mb
        let secs = total_bits / total_bw * 60.0; // convert... let me be simpler
        let secs_simple = total_gb * 1000.0 / (total_bw / 8.0); // GB / (MB/s)
        let h = secs_simple / 3600.0;
        let m = (secs_simple % 3600.0) / 60.0;
        format!("{:.0}год {:.0}хв", h, m)
    }
}

/// P2P sync manager.
pub struct P2pSync {
    pub peers: Vec<P2pPeer>,
    pub plan: Option<DownloadPlan>,
}

impl P2pSync {
    pub fn new() -> Self {
        P2pSync { peers: Vec::new(), plan: None }
    }

    pub fn add_peer(&mut self, id: &str, addr: &str, bw: u32, sigs: u64) {
        self.peers.push(P2pPeer {
            id: id.to_string(), addr: addr.to_string(),
            latency_ms: 0, bandwidth_mbps: bw,
            chunks_have: vec![], total_sigs: sigs,
        });
    }

    /// Create download plan from current peers.
    pub fn create_plan(&mut self, total_sigs: u64) -> DownloadPlan {
        let total_bw: u32 = self.peers.iter().map(|p| p.bandwidth_mbps).sum();
        let plan = DownloadPlan::plan(total_sigs, &self.peers, total_bw);
        self.plan = Some(plan.clone());
        plan
    }

    /// Merge downloaded chunks into final snapshot.
    pub fn merge_chunks(chunks: &[(u32, Vec<u8>)], total_sigs: u64) -> Vec<u8> {
        let mut snapshot = vec![0u8; (total_sigs * 8 + 4) as usize];
        // Header
        let n = total_sigs as u32;
        snapshot[..4].copy_from_slice(&n.to_le_bytes());
        // Copy each chunk into place
        for (chunk_id, data) in chunks {
            let offset = 4 + *chunk_id as usize * CHUNK_BYTES;
            let end = offset + data.len().min(CHUNK_BYTES);
            let dest = &mut snapshot[offset..end];
            let src = &data[..dest.len().min(data.len())];
            dest.copy_from_slice(src);
        }
        snapshot
    }

    pub fn dashboard(&self) -> String {
        let total_bw: u32 = self.peers.iter().map(|p| p.bandwidth_mbps).sum();
        let total_sigs: u64 = self.peers.iter().map(|p| p.total_sigs).sum();
        let gb = (total_sigs * 8) as f64 / 1_000_000_000.0;
        let time = DownloadPlan::estimate_time(gb, self.peers.len() as u32, total_bw / self.peers.len().max(1) as u32);
        format!(
            "Academia P2P\n  Peers:  {}\n  Total:  {:.1} GB / {:.1e} sigs\n  BW:     {} Mbps\n  ETA:    {}",
            self.peers.len(), gb, total_sigs as f64, total_bw, time
        )
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_plan_creates_chunks() {
        let plan = DownloadPlan::plan(5_000_000, &[], 100);
        assert!(plan.chunks.len() >= 5); // 5M sigs / 1M per chunk = 5 chunks
    }

    #[test]
    fn peer_assignment_round_robin() {
        let peers = vec![
            P2pPeer { id: "A".into(), addr: "".into(), latency_ms: 0, bandwidth_mbps: 100, chunks_have: vec![], total_sigs: 0 },
            P2pPeer { id: "B".into(), addr: "".into(), latency_ms: 0, bandwidth_mbps: 100, chunks_have: vec![], total_sigs: 0 },
        ];
        let plan = DownloadPlan::plan(10_000_000, &peers, 200);
        assert_eq!(plan.peer_assignments.len(), 2);
        // Each peer should have chunks assigned
        assert!(!plan.peer_assignments[0].1.is_empty());
        assert!(!plan.peer_assignments[1].1.is_empty());
    }

    #[test]
    fn merge_chunks_reconstructs() {
        let total = 100_000u64;
        let chunk_data = vec![
            (0u32, vec![1u8; 800_000]),  // 100K sigs × 8 bytes
        ];
        let merged = P2pSync::merge_chunks(&chunk_data, total);
        assert_eq!(merged.len(), (total * 8 + 4) as usize);
    }

    #[test]
    fn estimate_time_scales() {
        let t1 = DownloadPlan::estimate_time(460.0, 1, 100);
        let t10 = DownloadPlan::estimate_time(460.0, 10, 100);
        assert_ne!(t1, t10); // Different times
    }

    #[test]
    fn p2p_sync_dashboard() {
        let mut sync = P2pSync::new();
        sync.add_peer("peer1", "10.0.0.1:9000", 100, 1_000_000);
        sync.add_peer("peer2", "10.0.0.2:9000", 200, 2_000_000);
        let d = sync.dashboard();
        assert!(d.contains("P2P"));
    }

    #[test]
    fn plan_estimates_bandwidth() {
        let peers = vec![
            P2pPeer { id: "A".into(), addr: "".into(), latency_ms: 0, bandwidth_mbps: 500, chunks_have: vec![], total_sigs: 0 },
        ];
        // 100 GB at 500 Mbps ≈ 1600 seconds ≈ 27 min
        let plan = DownloadPlan::plan(610_000_000, &peers, 500);
        assert!(plan.estimated_seconds > 0);
    }
}
