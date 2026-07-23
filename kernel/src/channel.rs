use crate::predictor::SystemState;
use std::collections::VecDeque;

pub const DEFAULT_MAX_KEPT: usize = 3;
pub const STUCK_SENSOR_EPSILON: f64 = 1e-12;
pub const DEFAULT_BATCH_LIMIT: usize = 64;

// ─── LastHealthyState ────────────────────────────────────────────────────

/// Stores the last verified healthy system state for emergency recovery.
/// On serious failure (crash, critical degradation), this state is published
/// so downstream modules can fall back to known-good data instead of
/// operating on corrupted or missing state.
pub struct LastHealthyState {
    snapshot: Option<SystemState>,
    /// How many consecutive healthy observations before we update
    confirmation_threshold: u32,
    healthy_run: u32,
    max_kept: usize,
}

impl LastHealthyState {
    pub fn new(confirmation_threshold: u32) -> Self {
        LastHealthyState {
            snapshot: None,
            confirmation_threshold,
            healthy_run: 0,
            max_kept: DEFAULT_MAX_KEPT,
        }
    }

    /// Accept a system state. If it passes health checks and has a sufficient
    /// run of consecutive healthy reports, it becomes the canonical "last healthy."
    pub fn observe(&mut self, state: &SystemState, is_healthy: bool) {
        if is_healthy {
            // Additional checks beyond sanitization:
            // 1. All metrics finite and in [0,1] (should be true after sanitize)
            // 2. Not all identical (stuck sensor detection)
            // 3. Not all zero (sanitized NaN/Inf → 0.0)
            if !state.metrics.iter().all(|m| m.is_finite() && (0.0..=1.0).contains(m)) {
                self.healthy_run = 0;
                return;
            }
            if state.metrics.len() < 2 {
                self.healthy_run = 0;
                return;
            }
            // Reject if all metrics are identical (stuck sensor / sanitized NaN)
            if state.metrics.iter().all(|&m| (m - state.metrics[0]).abs() < STUCK_SENSOR_EPSILON) {
                self.healthy_run = 0;
                return;
            }
            self.healthy_run = self.healthy_run.saturating_add(1);
            if self.healthy_run >= self.confirmation_threshold {
                self.snapshot = Some(state.clone());
            }
        } else {
            self.healthy_run = 0;
        }
    }

    /// Retrieve the last confirmed healthy snapshot.
    pub fn last_healthy(&self) -> Option<&SystemState> {
        self.snapshot.as_ref()
    }

    /// Force-set a snapshot (e.g. from deserialized recovery data).
    pub fn set_snapshot(&mut self, state: SystemState) {
        self.snapshot = Some(state);
        self.healthy_run = self.confirmation_threshold;
    }

    /// Reset to empty state (after recovery completes).
    pub fn clear(&mut self) {
        self.snapshot = None;
        self.healthy_run = 0;
    }

    pub fn is_armed(&self) -> bool {
        self.snapshot.is_some()
    }
}

// ─── Wormhole ────────────────────────────────────────────────────────────

/// A direct fast-transmission lane that bypasses gossip batching/queuing.
/// Best-effort: no ACK, no retry, no ordering guarantees.
/// Use for time-critical alerts (circuit-breaker trips, critical degradation,
/// hard deadlines) where a few microseconds of latency matters more than
/// delivery guarantee.
///
/// A wormhole sender pushes directly into the receiver's buffer,
/// skipping all intermediate queues.
pub struct Wormhole {
    name: String,
    priority: u8,
    direct_buffer: VecDeque<WormholePacket>,
    max_packets: usize,
    dropped: u64,
    sent: u64,
}

/// A message sent through a wormhole.
#[derive(Debug, Clone)]
pub struct WormholePacket {
    pub tag: &'static str,
    pub payload: Vec<u8>,
    pub timestamp_ms: u64,
    pub urgency: u8,
}

impl Wormhole {
    pub fn new(name: &str, priority: u8, max_packets: usize) -> Self {
        Wormhole {
            name: name.to_string(),
            priority,
            direct_buffer: VecDeque::with_capacity(max_packets),
            max_packets,
            dropped: 0,
            sent: 0,
        }
    }

    /// Send a packet through this wormhole (direct, no queue).
    /// If the buffer is full, the oldest packet is dropped.
    pub fn send(&mut self, tag: &'static str, payload: Vec<u8>, timestamp_ms: u64, urgency: u8) {
        let ts = crate::sanitize_f64(timestamp_ms as f64) as u64;
        self.direct_buffer.push_back(WormholePacket {
            tag,
            payload,
            timestamp_ms: ts,
            urgency,
        });
        self.sent += 1;
        if self.direct_buffer.len() > self.max_packets {
            self.direct_buffer.pop_front();
            self.dropped += 1;
        }
    }

    /// Receive all pending packets (drains the buffer).
    pub fn receive(&mut self) -> Vec<WormholePacket> {
        self.direct_buffer.drain(..).collect()
    }

    /// Peek at the oldest packet without draining.
    pub fn peek(&self) -> Option<&WormholePacket> {
        self.direct_buffer.front()
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn priority(&self) -> u8 { self.priority }
    pub fn sent_count(&self) -> u64 { self.sent }
    pub fn dropped_count(&self) -> u64 { self.dropped }
    pub fn pending(&self) -> usize { self.direct_buffer.len() }

    /// Reset counters.
    pub fn reset_stats(&mut self) {
        self.sent = 0;
        self.dropped = 0;
    }
}

// ─── Tunnel ──────────────────────────────────────────────────────────────

/// A dedicated direct-connection channel between two modules.
/// Supports backpressure, flow control, and high-throughput streaming.
/// Unlike Wormhole (single packet, best-effort), Tunnel provides reliable
/// ordered delivery with configurable buffer sizing.
///
/// Tunnels are typically established between modules that exchange
/// high-volume data: telemetry collector → predictor, predictor →
/// resilience, gossip → offline storage.
pub struct Tunnel {
    name: String,
    inbound: VecDeque<TunnelFrame>,
    outbound: VecDeque<TunnelFrame>,
    capacity: usize,
    /// Drops oldest frame when buffer is full
    overflow_drop: bool,
    overflow_dropped: u64,
    frames_sent: u64,
    frames_received: u64,
    /// Flow-control: max frames per batch drain
    batch_limit: usize,
}

/// A frame sent through a tunnel.
#[derive(Debug, Clone)]
pub struct TunnelFrame {
    pub stream_id: u64,
    pub seq: u64,
    pub data: Vec<u8>,
    pub timestamp_ms: u64,
    pub checksum: u64,
}

impl TunnelFrame {
    pub fn new(stream_id: u64, seq: u64, data: Vec<u8>, timestamp_ms: u64) -> Self {
        let checksum = crate::checksum_fold(&data);
        TunnelFrame { stream_id, seq, data, timestamp_ms, checksum }
    }

    pub fn verify(&self) -> bool {
        crate::checksum_fold(&self.data) == self.checksum
    }
}

impl Tunnel {
    pub fn new(name: &str, capacity: usize) -> Self {
        Tunnel {
            name: name.to_string(),
            inbound: VecDeque::with_capacity(capacity),
            outbound: VecDeque::with_capacity(capacity),
            capacity,
            overflow_drop: true,
            overflow_dropped: 0,
            frames_sent: 0,
            frames_received: 0,
            batch_limit: DEFAULT_BATCH_LIMIT,
        }
    }

    fn push_with_overflow<T>(buffer: &mut VecDeque<T>, item: T, capacity: usize, overflow_drop: bool, dropped: &mut u64) {
        if buffer.len() >= capacity {
            if overflow_drop {
                buffer.pop_front();
                *dropped += 1;
            } else {
                return;
            }
        }
        buffer.push_back(item);
    }

    pub fn write(&mut self, frame: TunnelFrame) {
        self.frames_sent += 1;
        Self::push_with_overflow(&mut self.inbound, frame, self.capacity, self.overflow_drop, &mut self.overflow_dropped);
    }

    /// Read all pending frames from the inbound buffer (drains up to batch_limit).
    pub fn read(&mut self) -> Vec<TunnelFrame> {
        let count = self.batch_limit.min(self.inbound.len());
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            if let Some(frame) = self.inbound.pop_front() {
                result.push(frame);
            }
        }
        self.frames_received += result.len() as u64;
        result
    }

    /// Read all frames without limit.
    pub fn read_all(&mut self) -> Vec<TunnelFrame> {
        let result: Vec<TunnelFrame> = self.inbound.drain(..).collect();
        self.frames_received += result.len() as u64;
        result
    }

    /// Send a frame to the tunnel's outbound (for bidirectional tunnels).
    pub fn write_outbound(&mut self, frame: TunnelFrame) {
        self.frames_sent += 1;
        Self::push_with_overflow(&mut self.outbound, frame, self.capacity, self.overflow_drop, &mut self.overflow_dropped);
    }

    /// Read from outbound.
    pub fn read_outbound(&mut self) -> Vec<TunnelFrame> {
        let result: Vec<TunnelFrame> = self.outbound.drain(..).collect();
        self.frames_received += result.len() as u64;
        result
    }

    pub fn pending_inbound(&self) -> usize { self.inbound.len() }
    pub fn pending_outbound(&self) -> usize { self.outbound.len() }
    pub fn name(&self) -> &str { &self.name }
    pub fn set_overflow_drop(&mut self, drop: bool) { self.overflow_drop = drop; }
    pub fn set_batch_limit(&mut self, limit: usize) { self.batch_limit = limit; }
    pub fn stats_sent(&self) -> u64 { self.frames_sent }
    pub fn stats_received(&self) -> u64 { self.frames_received }
    pub fn stats_dropped(&self) -> u64 { self.overflow_dropped }
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── LastHealthyState tests ──────────────────────────────────────────

    #[test]
    fn last_healthy_empty_initially() {
        let lh = LastHealthyState::new(3);
        assert!(!lh.is_armed());
        assert!(lh.last_healthy().is_none());
    }

    #[test]
    fn last_healthy_confirms_after_threshold() {
        let mut lh = LastHealthyState::new(3);
        let s = SystemState::new(1, vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8], "test");
        lh.observe(&s, true);
        assert!(!lh.is_armed(), "must not arm before threshold (1/3)");
        lh.observe(&s, true);
        assert!(!lh.is_armed(), "must not arm before threshold (2/3)");
        lh.observe(&s, true);
        assert!(lh.is_armed(), "must arm after 3 healthy observations");
    }

    #[test]
    fn last_healthy_rejected_on_unhealthy() {
        let mut lh = LastHealthyState::new(2);
        let s = SystemState::new(1, vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8], "test");
        lh.observe(&s, false);
        assert!(!lh.is_armed());
        lh.observe(&s, true);
        assert!(!lh.is_armed(), "run resets on unhealthy");
    }

    #[test]
    fn last_healthy_corrupted_metrics_rejected() {
        let mut lh = LastHealthyState::new(1);
        // All-identical metrics are treated as stuck sensor / sanitized NaN
        let s = SystemState::new(1, vec![0.0; 8], "corrupt");
        lh.observe(&s, true);
        assert!(!lh.is_armed(), "all-identical metrics (stuck sensor) must be rejected");
    }

    #[test]
    fn last_healthy_clear_resets() {
        let mut lh = LastHealthyState::new(1);
        let s = SystemState::new(1, vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8], "test");
        lh.observe(&s, true);
        assert!(lh.is_armed());
        lh.clear();
        assert!(!lh.is_armed());
        assert!(lh.last_healthy().is_none());
    }

    #[test]
    fn last_healthy_set_snapshot_bypasses_threshold() {
        let mut lh = LastHealthyState::new(5);
        let s = SystemState::new(1, vec![0.5; 8], "recovered");
        lh.set_snapshot(s.clone());
        assert!(lh.is_armed());
        assert_eq!(lh.last_healthy().unwrap().label, "recovered");
    }

    // ── Wormhole tests ─────────────────────────────────────────────────

    #[test]
    fn wormhole_send_receive() {
        let mut wh = Wormhole::new("critical", 1, 100);
        wh.send("panic", b"out of memory".to_vec(), 1000, 255);
        let pkts = wh.receive();
        assert_eq!(pkts.len(), 1);
        assert_eq!(pkts[0].tag, "panic");
        assert_eq!(pkts[0].payload, b"out of memory");
    }

    #[test]
    fn wormhole_drops_oldest_on_overflow() {
        let mut wh = Wormhole::new("test", 0, 3);
        for i in 0..5 {
            wh.send("data", vec![i as u8], i, 0);
        }
        assert_eq!(wh.sent_count(), 5);
        assert_eq!(wh.dropped_count(), 2);
        let pkts = wh.receive();
        assert_eq!(pkts.len(), 3);
        assert_eq!(pkts[0].payload[0], 2, "oldest (0,1) dropped, newest=2,3,4");
    }

    #[test]
    fn wormhole_empty_receive() {
        let mut wh = Wormhole::new("empty", 0, 10);
        assert!(wh.receive().is_empty());
    }

    #[test]
    fn wormhole_peek_no_drain() {
        let mut wh = Wormhole::new("peek", 0, 10);
        wh.send("alert", b"test".to_vec(), 1, 0);
        assert!(wh.peek().is_some());
        assert_eq!(wh.pending(), 1, "peek must not drain");
        let pkts = wh.receive();
        assert_eq!(pkts.len(), 1, "receive after peek must still work");
    }

    #[test]
    fn wormhole_reset_stats() {
        let mut wh = Wormhole::new("reset", 0, 10);
        for _ in 0..10 { wh.send("x", vec![0], 1, 0); }
        assert_eq!(wh.sent_count(), 10);
        wh.reset_stats();
        assert_eq!(wh.sent_count(), 0);
    }

    #[test]
    fn wormhole_timestamp_never_goes_backwards() {
        let mut wh = Wormhole::new("ts", 0, 100);
        let mut prev = 0u64;
        for i in 0..50 {
            wh.send("tick", vec![i as u8], i * 100, 0);
        }
        let pkts = wh.receive();
        for p in &pkts {
            assert!(p.timestamp_ms >= prev, "wormhole timestamps must be monotonic");
            prev = p.timestamp_ms;
        }
    }

    // ── Tunnel tests ───────────────────────────────────────────────────

    #[test]
    fn tunnel_write_read() {
        let mut t = Tunnel::new("telemetry", 100);
        let frame = TunnelFrame::new(1, 0, b"cpu:0.5".to_vec(), 1000);
        t.write(frame);
        let frames = t.read();
        assert_eq!(frames.len(), 1);
        assert!(frames[0].verify());
    }

    #[test]
    fn tunnel_verify_corrupted_frame() {
        let mut frame = TunnelFrame::new(1, 0, b"data".to_vec(), 1000);
        assert!(frame.verify());
        frame.data[0] ^= 0xFF; // corrupt
        assert!(!frame.verify(), "corrupted frame must fail verification");
    }

    #[test]
    fn tunnel_batch_limit() {
        let mut t = Tunnel::new("batch", 100);
        t.set_batch_limit(10);
        for i in 0..50 {
            t.write(TunnelFrame::new(1, i, vec![i as u8], i));
        }
        let batch1 = t.read();
        assert_eq!(batch1.len(), 10, "read must respect batch limit");
        let batch2 = t.read();
        assert_eq!(batch2.len(), 10);
    }

    #[test]
    fn tunnel_overflow_drop() {
        let mut t = Tunnel::new("overflow", 5);
        for i in 0..20 {
            t.write(TunnelFrame::new(1, i, vec![i as u8], i));
        }
        assert_eq!(t.stats_dropped(), 15);
        let frames = t.read_all();
        assert_eq!(frames.len(), 5, "only 5 fit in capacity");
        // Verify oldest were dropped
        let first_seq = frames[0].seq;
        assert!(first_seq >= 15, "oldest frames dropped, first kept seq={first_seq}");
    }

    #[test]
    fn tunnel_no_drop_when_overflow_disabled() {
        let mut t = Tunnel::new("nodrop", 3);
        t.set_overflow_drop(false);
        for i in 0..10 {
            t.write(TunnelFrame::new(1, i, vec![i as u8], i));
        }
        assert_eq!(t.stats_dropped(), 0);
        // Only first 3 went through (no drop mode = reject new)
        let frames = t.read_all();
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].seq, 0, "first frames kept when overflow_drop=false");
    }

    #[test]
    fn tunnel_bidirectional() {
        let mut t = Tunnel::new("bidir", 100);
        let f1 = TunnelFrame::new(1, 0, b"req".to_vec(), 1000);
        t.write(f1);
        let f2 = TunnelFrame::new(2, 0, b"resp".to_vec(), 1001);
        t.write_outbound(f2);
        assert_eq!(t.pending_inbound(), 1);
        assert_eq!(t.pending_outbound(), 1);
        let reqs = t.read();
        let resps = t.read_outbound();
        assert_eq!(reqs[0].data, b"req");
        assert_eq!(resps[0].data, b"resp");
    }

    #[test]
    fn tunnel_empty_read() {
        let mut t = Tunnel::new("empty", 10);
        assert!(t.read().is_empty());
        assert!(t.read_all().is_empty());
    }

    #[test]
    fn tunnel_stats_counting() {
        let mut t = Tunnel::new("stats", 100);
        for i in 0..10 {
            t.write(TunnelFrame::new(1, i, vec![i as u8], i));
        }
        assert_eq!(t.stats_sent(), 10);
        let _ = t.read_all();
        assert_eq!(t.stats_received(), 10);
    }

    #[test]
    fn tunnel_frame_checksum_stable() {
        let a = TunnelFrame::new(1, 0, b"hello".to_vec(), 100);
        let b = TunnelFrame::new(1, 0, b"hello".to_vec(), 100);
        assert_eq!(a.checksum, b.checksum, "identical payloads must produce same checksum");
        let c = TunnelFrame::new(1, 0, b"world".to_vec(), 100);
        assert_ne!(a.checksum, c.checksum, "different payloads must produce different checksum");
    }

    #[test]
    fn tunnel_stream_monotonic_seq() {
        let mut t = Tunnel::new("mono", 100);
        let mut prev_seq = u64::MAX;
        for i in 0..50 {
            t.write(TunnelFrame::new(1, i, vec![i as u8], i));
        }
        let frames = t.read_all();
        for f in &frames {
            if prev_seq != u64::MAX {
                assert!(f.seq > prev_seq, "sequence numbers must increase: {fseq} <= {prev}", fseq=f.seq, prev=prev_seq);
            }
            prev_seq = f.seq;
        }
    }

    #[test]
    fn last_healthy_jamming_nan_safe() {
        let mut lh = LastHealthyState::new(1);
        // NaN/Inf are sanitized to 0.0 by SystemState::new, producing all-zeros,
        // which is detected as stuck sensor and rejected.
        let s1 = SystemState::new(1, vec![0.0; 8], "jam1");
        lh.observe(&s1, true);
        assert!(!lh.is_armed(), "all-zero state (sanitized NaN) must be rejected");
        // Also test that non-identical but all from an Inf source is rejected
        let s2 = SystemState::new(2, vec![1.0; 8], "jam2");
        lh.observe(&s2, true);
        assert!(!lh.is_armed(), "all-1.0 state (sanitized Inf) must be rejected");
    }

    #[test]
    fn wormhole_rapid_fire_no_panic() {
        let mut wh = Wormhole::new("rapid", 0, 1000);
        for i in 0..10_000 {
            wh.send("data", vec![(i % 256) as u8], i, 0);
        }
        assert_eq!(wh.sent_count(), 10_000);
        assert_eq!(wh.dropped_count(), 9_000);
        let pkts = wh.receive();
        assert_eq!(pkts.len(), 1000);
    }

    #[test]
    fn tunnel_rapid_fire_integrity() {
        let mut t = Tunnel::new("rapid", 500);
        for i in 0..5_000 {
            t.write(TunnelFrame::new(1, i, vec![(i % 256) as u8], i));
        }
        let frames = t.read_all();
        assert!(frames.iter().all(|f| f.verify()), "all tunnel frames must pass integrity check");
    }
}
