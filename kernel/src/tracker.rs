//! Tracker — comprehensive error tracking, deterministic logging, telemetry,
//! reverse replay, and inverse simulation.
//!
//! Provides:
//! - `TrackedError` — typed error with context chain for root‑cause analysis
//! - `EventLog` — append‑only log with forward/reverse iteration and replay
//! - `TelemetryCollector` — unified atomic counters across all modules
//! - `ReverseReplay` — reconstruct past states from event history
//! - `InverseSimulator` — given an observed outcome, infer probable input ranges

use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_MAX_EVENTS_INITIAL: usize = 1024;
pub const INVERSE_DEFAULT_CANDIDATES: usize = 20;
pub const INVERSE_DEFAULT_STEP: f64 = 0.1;
pub const INVERSE_HILL_CLIMB_ITERS: usize = 50;
pub const INVERSE_TOP_N: usize = 10;
pub const INVERSE_RNG_SEED: u64 = 42;

// ─── TrackedError ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TrackedError {
    pub module: &'static str,
    pub kind: &'static str,
    pub message: String,
    pub timestamp_ms: u64,
    pub seq: u64,
    pub chain: Vec<TrackedError>,
}

impl TrackedError {
    pub fn new(module: &'static str, kind: &'static str, message: impl Into<String>) -> Self {
        TrackedError {
            module,
            kind,
            message: message.into(),
            timestamp_ms: crate::now_ms(),
            seq: next_error_seq(),
            chain: Vec::new(),
        }
    }

    pub fn with_cause(mut self, cause: TrackedError) -> Self {
        self.chain.push(cause);
        self
    }

    pub fn root_cause(&self) -> &TrackedError {
        self.chain.last().unwrap_or(self)
    }

    pub fn is_critical(&self) -> bool {
        matches!(self.kind, "panic" | "corruption" | "security" | "data_loss" | "resource_exhaustion")
    }
}

fn next_error_seq() -> u64 {
    static ERROR_SEQ: AtomicU64 = AtomicU64::new(1);
    ERROR_SEQ.fetch_add(1, Ordering::Relaxed)
}

// ─── EventLog ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LoggedEvent {
    pub seq: u64,
    pub timestamp_ms: u64,
    pub module: &'static str,
    pub event_type: &'static str,
    pub payload: Vec<u8>,
    pub checksum: u64,
}

impl LoggedEvent {
    pub fn new(module: &'static str, event_type: &'static str, payload: Vec<u8>) -> Self {
        static EVENT_SEQ: AtomicU64 = AtomicU64::new(1);
        let ts = crate::now_ms();
        let checksum = crate::checksum_fold(&payload);
        LoggedEvent {
            seq: EVENT_SEQ.fetch_add(1, Ordering::Relaxed),
            timestamp_ms: ts,
            module,
            event_type,
            payload,
            checksum,
        }
    }

    pub fn verify(&self) -> bool {
        crate::checksum_fold(&self.payload) == self.checksum
    }
}

/// Append-only event log with forward/reverse iteration.
pub struct EventLog {
    events: Vec<LoggedEvent>,
    max_events: usize,
    pruning_count: u64,
}

impl EventLog {
    pub fn new(max_events: usize) -> Self {
        EventLog {
            events: Vec::with_capacity(max_events.min(DEFAULT_MAX_EVENTS_INITIAL)),
            max_events,
            pruning_count: 0,
        }
    }

    pub fn record(&mut self, event: LoggedEvent) {
        if self.events.len() >= self.max_events {
            self.events.remove(0);
            self.pruning_count += 1;
        }
        self.events.push(event);
    }

    pub fn len(&self) -> usize { self.events.len() }
    pub fn is_empty(&self) -> bool { self.events.is_empty() }
    pub fn total_recorded(&self) -> u64 { self.events.len() as u64 + self.pruning_count }
    pub fn pruned(&self) -> u64 { self.pruning_count }

    /// Forward iterator: oldest → newest.
    pub fn iter(&self) -> impl Iterator<Item = &LoggedEvent> + DoubleEndedIterator {
        self.events.iter()
    }

    /// Reverse iterator: newest → oldest.
    pub fn rev_iter(&self) -> impl Iterator<Item = &LoggedEvent> {
        self.events.iter().rev()
    }

    /// Replay all events from a given sequence number forward.
    pub fn replay_from(&self, from_seq: u64) -> Vec<&LoggedEvent> {
        self.events.iter().filter(|e| e.seq >= from_seq).collect()
    }

    /// Replay in reverse from a given sequence number (for unwinding).
    pub fn replay_reverse_from(&self, from_seq: u64) -> Vec<&LoggedEvent> {
        self.events.iter().rev().filter(|e| e.seq <= from_seq).collect()
    }

    /// Find events by module.
    pub fn by_module(&self, module: &str) -> Vec<&LoggedEvent> {
        self.events.iter().filter(|e| e.module == module).collect()
    }

    /// Find events by type.
    pub fn by_type(&self, event_type: &str) -> Vec<&LoggedEvent> {
        self.events.iter().filter(|e| e.event_type == event_type).collect()
    }

    /// Verify integrity of all stored events.
    pub fn verify_all(&self) -> bool {
        self.events.iter().all(|e| e.verify())
    }
}

// ─── TelemetryCollector ───────────────────────────────────────────────────

/// Unified zero-dep telemetry collector using atomic counters.
/// In production: lightweight atomic increments.
/// Under `telemetry` feature: also records to a stamp collector.
pub struct TelemetryCollector {
    counters: Vec<TelemetryCounter>,
}

struct TelemetryCounter {
    module: &'static str,
    name: &'static str,
    value: AtomicU64,
}

impl std::fmt::Debug for TelemetryCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelemetryCollector")
            .field("count", &self.counters.len())
            .finish()
    }
}

impl TelemetryCollector {
    pub fn new() -> Self {
        TelemetryCollector { counters: Vec::new() }
    }

    pub fn register(&mut self, module: &'static str, name: &'static str) -> usize {
        let idx = self.counters.len();
        self.counters.push(TelemetryCounter {
            module,
            name,
            value: AtomicU64::new(0),
        });
        idx
    }

    pub fn increment(&self, idx: usize, delta: u64) {
        if let Some(c) = self.counters.get(idx) {
            c.value.fetch_add(delta, Ordering::Relaxed);
        }
    }

    pub fn set(&self, idx: usize, value: u64) {
        if let Some(c) = self.counters.get(idx) {
            c.value.store(value, Ordering::Relaxed);
        }
    }

    pub fn get(&self, idx: usize) -> u64 {
        self.counters.get(idx).map(|c| c.value.load(Ordering::Relaxed)).unwrap_or(0)
    }

    pub fn snapshot(&self) -> Vec<(&'static str, &'static str, u64)> {
        self.counters.iter().map(|c| {
            (c.module, c.name, c.value.load(Ordering::Relaxed))
        }).collect()
    }

    pub fn reset(&self, idx: usize) {
        if let Some(c) = self.counters.get(idx) {
            c.value.store(0, Ordering::Relaxed);
        }
    }

    pub fn count(&self) -> usize { self.counters.len() }
}

impl Default for TelemetryCollector {
    fn default() -> Self { Self::new() }
}

// ─── ReverseReplay ────────────────────────────────────────────────────────

/// Reconstruct past metric sequences from an event log by replaying in reverse.
/// Each step "un-applies" a state change to recover the prior state.
pub struct ReverseReplay {
    history: Vec<Vec<f64>>,
    timestamps: Vec<u64>,
    max_states: usize,
}

impl ReverseReplay {
    pub fn new(max_states: usize) -> Self {
        ReverseReplay {
            history: Vec::with_capacity(max_states.min(DEFAULT_MAX_EVENTS_INITIAL)),
            timestamps: Vec::with_capacity(max_states.min(DEFAULT_MAX_EVENTS_INITIAL)),
            max_states,
        }
    }

    /// Record a state snapshot forward (for reverse replay later).
    pub fn record(&mut self, metrics: &[f64], timestamp_ms: u64) {
        if self.history.len() >= self.max_states {
            self.history.remove(0);
            self.timestamps.remove(0);
        }
        self.history.push(metrics.to_vec());
        self.timestamps.push(timestamp_ms);
    }

    /// Replay forward from oldest to newest.
    pub fn replay_forward(&self) -> Vec<(&[f64], u64)> {
        self.history.iter().zip(&self.timestamps)
            .map(|(m, t)| (m.as_slice(), *t))
            .collect()
    }

    /// Replay backward from newest to oldest (time-travel).
    pub fn replay_reverse(&self) -> Vec<(&[f64], u64)> {
        self.history.iter().rev().zip(self.timestamps.iter().rev())
            .map(|(m, t)| (m.as_slice(), *t))
            .collect()
    }

    /// Estimate state at a specific timestamp by interpolating between
    /// the two nearest recorded states.
    pub fn estimate_at(&self, target_ms: u64) -> Option<Vec<f64>> {
        let n = self.timestamps.len();
        if n == 0 { return None; }
        if target_ms <= self.timestamps[0] {
            return Some(self.history[0].clone());
        }
        if target_ms >= self.timestamps[n - 1] {
            return Some(self.history[n - 1].clone());
        }
        // Binary search for the right insertion point
        let idx = match self.timestamps.binary_search(&target_ms) {
            Ok(i) => return Some(self.history[i].clone()),
            Err(i) => i,
        };
        if idx == 0 {
            return Some(self.history[0].clone());
        }
        let t0 = self.timestamps[idx - 1];
        let t1 = self.timestamps[idx];
        let frac = (target_ms - t0) as f64 / (t1 - t0) as f64;
        let m0 = &self.history[idx - 1];
        let m1 = &self.history[idx];
        let result: Vec<f64> = m0.iter().zip(m1).map(|(a, b)| a + (b - a) * frac).collect();
        Some(result)
    }

    pub fn len(&self) -> usize { self.history.len() }
    pub fn is_empty(&self) -> bool { self.history.is_empty() }
    pub fn clear(&mut self) { self.history.clear(); self.timestamps.clear(); }
}

// ─── InverseSimulator ────────────────────────────────────────────────────

/// Given an observed (output) state, infer the range of input states that
/// could have produced it. Simple inverse model using gradient-free search.
pub struct InverseSimulator {
    /// Number of candidate input vectors to try per inverse query.
    pub n_candidates: usize,
    /// Perturbation step for hill-climbing.
    pub step_size: f64,
    /// Forward model function pointer
    forward_model: Option<fn(&[f64]) -> Vec<f64>>,
}

impl InverseSimulator {
    pub fn new(n_candidates: usize) -> Self {
        InverseSimulator {
            n_candidates,
            step_size: INVERSE_DEFAULT_STEP,
            forward_model: None,
        }
    }

    pub fn with_forward_model(mut self, model: fn(&[f64]) -> Vec<f64>) -> Self {
        self.forward_model = Some(model);
        self
    }

    /// Infer likely input ranges from observed output.
    /// Uses random perturbation + hill-climbing to minimize
    /// ||forward(input) - observed_output||.
    pub fn infer_input(&self, observed: &[f64], dims: usize) -> InverseResult {
        let mut candidates: Vec<Vec<f64>> = Vec::with_capacity(self.n_candidates);
        let mut costs: Vec<f64> = Vec::with_capacity(self.n_candidates);

        for _ in 0..self.n_candidates {
            let input: Vec<f64> = (0..dims).map(|_| fast_rng_f64()).collect();
            let cost = self.evaluate(&input, observed);
            candidates.push(input);
            costs.push(cost);
        }

        // Hill-climb best candidate
        let mut best_idx = 0;
        for i in 1..self.n_candidates {
            if costs[i] < costs[best_idx] { best_idx = i; }
        }

        let mut best = candidates[best_idx].clone();
        let mut best_cost = costs[best_idx];

        for _ in 0..INVERSE_HILL_CLIMB_ITERS {
            let mut candidate = best.clone();
            for v in &mut candidate {
                *v = (*v + fast_rng_f64() * self.step_size - self.step_size * 0.5).clamp(0.0, 1.0);
            }
            let cost = self.evaluate(&candidate, observed);
            if cost < best_cost {
                best = candidate;
                best_cost = cost;
            }
        }

        // Estimate bounds from top candidates
        let mut sorted: Vec<usize> = (0..self.n_candidates).collect();
        crate::sort_by_f64_asc(&mut sorted, |&i| costs[i]);

        let top_n = INVERSE_TOP_N.min(self.n_candidates);
        let mut lower_bound = vec![f64::INFINITY; dims];
        let mut upper_bound = vec![f64::NEG_INFINITY; dims];

        for &idx in &sorted[..top_n] {
            for d in 0..dims {
                let v = candidates[idx][d];
                if v < lower_bound[d] { lower_bound[d] = v; }
                if v > upper_bound[d] { upper_bound[d] = v; }
            }
        }

        InverseResult {
            best_input: best,
            best_cost,
            lower_bound,
            upper_bound,
            iterations: INVERSE_HILL_CLIMB_ITERS as u32,
        }
    }

    fn evaluate(&self, input: &[f64], observed: &[f64]) -> f64 {
        match self.forward_model {
            Some(model) => {
                let output = model(input);
                if output.len() != observed.len() {
                    return f64::MAX;
                }
                output.iter().zip(observed).map(|(a, b)| (a - b).powi(2)).sum()
            }
            None => {
                // Without a forward model, assume identity (input ≈ output)
                input.iter().zip(observed).map(|(a, b)| (a - b).powi(2)).sum()
            }
        }
    }
}

/// Result of an inverse simulation query.
pub struct InverseResult {
    pub best_input: Vec<f64>,
    pub best_cost: f64,
    pub lower_bound: Vec<f64>,
    pub upper_bound: Vec<f64>,
    pub iterations: u32,
}

/// Simple deterministic RNG for inverse search (no std::rand dependency).
fn fast_rng_f64() -> f64 {
    use std::sync::atomic::Ordering;
    const XORSHIFT_MUL: u64 = 0x2545_F491_4F6C_DD1D;
    const SCALE: f64 = 5.421010862427522e-20;
    static SEED: AtomicU64 = AtomicU64::new(INVERSE_RNG_SEED);
    let mut x = SEED.fetch_add(1, Ordering::Relaxed);
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    (x.wrapping_mul(XORSHIFT_MUL)) as f64 * SCALE
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── TrackedError ────────────────────────────────────────────────────

    #[test]
    fn tracked_error_basic() {
        let err = TrackedError::new("pid", "overflow", "integral exceeded max");
        assert_eq!(err.module, "pid");
        assert_eq!(err.kind, "overflow");
        assert!(err.seq > 0);
    }

    #[test]
    fn tracked_error_chain() {
        let cause = TrackedError::new("sensor", "jamming", "NaN received");
        let err = TrackedError::new("resilience", "failover", "circuit opened")
            .with_cause(cause);
        assert_eq!(err.chain.len(), 1);
        assert_eq!(err.root_cause().kind, "jamming");
    }

    #[test]
    fn tracked_error_critical_classification() {
        let e1 = TrackedError::new("test", "panic", "oops");
        assert!(e1.is_critical());
        let e2 = TrackedError::new("test", "timeout", "slow");
        assert!(!e2.is_critical());
    }

    #[test]
    fn tracked_error_unique_sequence() {
        let a = TrackedError::new("a", "x", "first");
        let b = TrackedError::new("b", "y", "second");
        assert_ne!(a.seq, b.seq, "errors must have unique seq numbers");
    }

    // ── EventLog ────────────────────────────────────────────────────────

    #[test]
    fn event_log_basic_record_and_iter() {
        let mut log = EventLog::new(100);
        log.record(LoggedEvent::new("test", "info", b"hello".to_vec()));
        log.record(LoggedEvent::new("test", "warn", b"world".to_vec()));
        assert_eq!(log.len(), 2);
        let events: Vec<_> = log.iter().collect();
        assert_eq!(events[0].event_type, "info");
        assert_eq!(events[1].event_type, "warn");
    }

    #[test]
    fn event_log_reverse_iter() {
        let mut log = EventLog::new(100);
        for i in 0..5 {
            log.record(LoggedEvent::new("t", "ev", vec![i]));
        }
        let rev: Vec<_> = log.rev_iter().collect();
        assert_eq!(rev[0].payload[0], 4, "reverse: newest first");
        assert_eq!(rev[4].payload[0], 0, "reverse: oldest last");
    }

    #[test]
    fn event_log_pruning() {
        let mut log = EventLog::new(3);
        for i in 0..10 {
            log.record(LoggedEvent::new("t", "ev", vec![i]));
        }
        assert_eq!(log.len(), 3);
        assert_eq!(log.pruned(), 7);
        let events: Vec<_> = log.iter().collect();
        assert_eq!(events[0].payload[0], 7, "oldest kept = seq 7");
    }

    #[test]
    fn event_log_integrity() {
        let mut log = EventLog::new(100);
        log.record(LoggedEvent::new("m1", "data", b"valid".to_vec()));
        log.record(LoggedEvent::new("m2", "data", b"also valid".to_vec()));
        assert!(log.verify_all());
        // Corrupt a stored event
        if let Some(e) = log.events.get_mut(0) {
            e.checksum ^= 1;
        }
        assert!(!log.verify_all(), "must detect corruption");
    }

    #[test]
    fn event_log_filter_by_module() {
        let mut log = EventLog::new(100);
        log.record(LoggedEvent::new("pid", "update", vec![1]));
        log.record(LoggedEvent::new("gossip", "pub", vec![2]));
        log.record(LoggedEvent::new("pid", "tune", vec![3]));
        let pid_events = log.by_module("pid");
        assert_eq!(pid_events.len(), 2);
    }

    #[test]
    fn event_log_replay_from_seq() {
        let mut log = EventLog::new(100);
        for i in 0..20 {
            log.record(LoggedEvent::new("t", "ev", vec![i]));
        }
        let events: Vec<_> = log.iter().collect();
        let from_seq = events[10].seq;
        let replay = log.replay_from(from_seq);
        assert_eq!(replay.len(), 10, "replay from seq {from_seq}: {n} events", n=10);
    }

    #[test]
    fn event_log_reverse_replay_to_unwind() {
        let mut log = EventLog::new(100);
        for i in 0..20 {
            log.record(LoggedEvent::new("t", "state", vec![i]));
        }
        let events: Vec<_> = log.iter().collect();
        let target_seq = events[15].seq;
        let unwind = log.replay_reverse_from(target_seq);
        // Newest-first from target_seq down
        assert!(!unwind.is_empty(), "reverse replay must return events");
        assert!(unwind[0].seq <= target_seq, "first reversed event seq={} ≤ target={}",
            unwind[0].seq, target_seq);
    }

    // ── TelemetryCollector ──────────────────────────────────────────────

    #[test]
    fn telemetry_basic_increment() {
        let mut tc = TelemetryCollector::new();
        let idx = tc.register("oracle", "gps_fix");
        assert_eq!(tc.get(idx), 0);
        tc.increment(idx, 1);
        assert_eq!(tc.get(idx), 1);
        tc.increment(idx, 5);
        assert_eq!(tc.get(idx), 6);
    }

    #[test]
    fn telemetry_snapshot_contains_all() {
        let mut tc = TelemetryCollector::new();
        let i1 = tc.register("cpu", "load");
        let i2 = tc.register("mem", "usage");
        tc.increment(i1, 10);
        tc.increment(i2, 20);
        let snap = tc.snapshot();
        assert_eq!(snap.len(), 2);
        assert!(snap.iter().any(|(m, n, v)| *m == "cpu" && *n == "load" && *v == 10));
        assert!(snap.iter().any(|(m, n, v)| *m == "mem" && *n == "usage" && *v == 20));
    }

    #[test]
    fn telemetry_reset_clears() {
        let mut tc = TelemetryCollector::new();
        let idx = tc.register("test", "counter");
        tc.increment(idx, 100);
        assert_eq!(tc.get(idx), 100);
        tc.reset(idx);
        assert_eq!(tc.get(idx), 0);
    }

    #[test]
    fn telemetry_set_absolute() {
        let mut tc = TelemetryCollector::new();
        let idx = tc.register("test", "value");
        tc.set(idx, 42);
        assert_eq!(tc.get(idx), 42);
    }

    #[test]
    fn telemetry_out_of_range_get() {
        let tc = TelemetryCollector::new();
        assert_eq!(tc.get(999), 0, "out-of-range index must return 0");
    }

    // ── ReverseReplay ───────────────────────────────────────────────────

    #[test]
    fn reverse_replay_empty() {
        let rr = ReverseReplay::new(100);
        assert!(rr.is_empty());
        assert!(rr.replay_forward().is_empty());
        assert!(rr.replay_reverse().is_empty());
    }

    #[test]
    fn reverse_replay_forward_backward() {
        let mut rr = ReverseReplay::new(100);
        let m = |v| vec![v; 8];
        rr.record(&m(0.0), 0);
        rr.record(&m(0.5), 100);
        rr.record(&m(1.0), 200);

        let fwd = rr.replay_forward();
        assert_eq!(fwd.len(), 3);
        assert_eq!(fwd[0].0[0], 0.0);
        assert_eq!(fwd[2].0[0], 1.0);

        let rev = rr.replay_reverse();
        assert_eq!(rev.len(), 3);
        assert_eq!(rev[0].0[0], 1.0, "reverse: newest first");
        assert_eq!(rev[2].0[0], 0.0, "reverse: oldest last");
    }

    #[test]
    fn reverse_replay_estimate_interpolation() {
        let mut rr = ReverseReplay::new(100);
        rr.record(&vec![0.0; 8], 0);
        rr.record(&vec![1.0; 8], 100);

        let est = rr.estimate_at(50).unwrap();
        assert!((est[0] - 0.5).abs() < 0.01, "interpolated at t=50 should be ~0.5: {e}", e=est[0]);
    }

    #[test]
    fn reverse_replay_estimate_out_of_bounds() {
        let mut rr = ReverseReplay::new(100);
        rr.record(&vec![0.5; 8], 100);
        // Before first timestamp
        let early = rr.estimate_at(0).unwrap();
        assert_eq!(early[0], 0.5, "before-first returns first state");
        // After last timestamp
        let late = rr.estimate_at(999).unwrap();
        assert_eq!(late[0], 0.5, "after-last returns last state");
    }

    #[test]
    fn reverse_replay_clear() {
        let mut rr = ReverseReplay::new(100);
        rr.record(&vec![0.5; 8], 0);
        rr.clear();
        assert!(rr.is_empty());
    }

    #[test]
    fn reverse_replay_pruning() {
        let mut rr = ReverseReplay::new(5);
        for i in 0..20 {
            rr.record(&vec![i as f64; 8], i);
        }
        assert_eq!(rr.len(), 5);
        let fwd = rr.replay_forward();
        assert_eq!(fwd[0].0[0], 15.0, "oldest kept = 15 after pruning 0..14");
    }

    // ── InverseSimulator ────────────────────────────────────────────────

    #[test]
    fn inverse_simulator_identity_model() {
        let sim = InverseSimulator::new(20);
        let observed = vec![0.3, 0.5, 0.7];
        let result = sim.infer_input(&observed, 3);
        assert_eq!(result.best_input.len(), 3);
        assert!(result.best_cost >= 0.0);
        assert!(result.lower_bound.len() == 3);
        assert!(result.upper_bound.len() == 3);
        assert_eq!(result.iterations, 50);
    }

    #[test]
    fn inverse_simulator_with_forward_model() {
        fn square_model(input: &[f64]) -> Vec<f64> {
            input.iter().map(|&x| x * x).collect()
        }
        let sim = InverseSimulator::new(30)
            .with_forward_model(square_model);
        let observed = vec![0.25, 0.81]; // sqrt = 0.5, 0.9
        let result = sim.infer_input(&observed, 2);
        // Best input should be near sqrt(observed)
        assert!(result.best_cost < 1.0, "inverse must converge: cost={}", result.best_cost);
    }

    #[test]
    fn inverse_simulator_bounds_valid() {
        let sim = InverseSimulator::new(10);
        let result = sim.infer_input(&[0.5, 0.5], 2);
        for d in 0..2 {
            assert!(result.lower_bound[d] <= result.upper_bound[d],
                "lower <= upper for dim {d}: {l} <= {u}",
                l=result.lower_bound[d], u=result.upper_bound[d]);
        }
    }

    #[test]
    fn inverse_simulator_dimension_mismatch() {
        fn mismatched_model(_input: &[f64]) -> Vec<f64> {
            vec![0.0] // returns 1D for 2D input
        }
        let sim = InverseSimulator::new(5)
            .with_forward_model(mismatched_model);
        let result = sim.infer_input(&[0.5, 0.5], 2);
        assert_eq!(result.best_input.len(), 2, "input dim must match");
    }

    #[test]
    fn inverse_simulator_zero_cost_ideal() {
        fn identity(input: &[f64]) -> Vec<f64> { input.to_vec() }
        let sim = InverseSimulator::new(50)
            .with_forward_model(identity);
        let observed = vec![0.42, 0.58];
        let result = sim.infer_input(&observed, 2);
        // With identity model, best match should be close to observed
        assert!(result.best_cost < 0.5, "identity model must find near-exact match: cost={}", result.best_cost);
    }

    // ── Cross-module integration ────────────────────────────────────────

    #[test]
    fn tracker_event_log_integration_with_errors() {
        let mut log = EventLog::new(100);
        let err = TrackedError::new("predictor", "NaN_input", "metric 2 is NaN");
        log.record(LoggedEvent::new("predictor", "error", err.message.as_bytes().to_vec()));
        assert_eq!(log.len(), 1);
        assert!(log.verify_all());
        // Verify we can round-trip the error message
        let events: Vec<_> = log.iter().collect();
        let msg = String::from_utf8_lossy(&events[0].payload);
        assert_eq!(msg, "metric 2 is NaN");
    }

    #[test]
    fn tracker_telemetry_reverse_replay_integration() {
        let mut rr = ReverseReplay::new(100);
        let mut tc = TelemetryCollector::new();
        let obs_idx = tc.register("observer", "observations");

        for i in 0..20 {
            let metrics = vec![i as f64 * 0.05; 8];
            rr.record(&metrics, i * 100);
            tc.increment(obs_idx, 1);
        }

        assert_eq!(tc.get(obs_idx), 20);
        let rev = rr.replay_reverse();
        assert_eq!(rev.len(), 20);

        // Reverse replay: newest first at t=1900
        let (first_state, first_ts) = rev[0];
        assert_eq!(first_ts, 1900);
        assert!((first_state[0] - 0.95).abs() < 0.01);
    }

    #[test]
    fn tracker_inverse_to_estimate_prior() {
        // Use inverse simulation + reverse replay to "look backward":
        // Given current state, what might the prior state have been?
        let mut rr = ReverseReplay::new(100);
        // Record some history
        rr.record(&vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8], 0);
        rr.record(&vec![0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9], 100);
        rr.record(&vec![0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0], 200);

        // Current state: what might the prior (t=100) state have been?
        let _current = &[0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
        let est_prior = rr.estimate_at(100).unwrap();
        // Estimated prior should be close to the recorded state at t=100
        assert!((est_prior[0] - 0.2).abs() < 0.01,
            "estimated prior t=100 should be ~0.2: {}", est_prior[0]);
    }

    #[test]
    fn tracker_full_cycle_error_log_replay() {
        // Simulate: record error → log it → replay in reverse → trace root cause
        let mut event_log = EventLog::new(100);

        let root = TrackedError::new("sensor", "jamming", "spike detected");
        let err = TrackedError::new("resilience", "failover", "circuit breaker opened")
            .with_cause(root);

        event_log.record(LoggedEvent::new(
            err.module, "circuit_breaker",
            err.message.as_bytes().to_vec(),
        ));

        // Reverse replay: find the circuit breaker event
        let rev: Vec<_> = event_log.rev_iter().collect();
        assert_eq!(rev[0].event_type, "circuit_breaker");

        // Trace root cause from the error chain
        assert_eq!(err.root_cause().kind, "jamming");
    }
}
