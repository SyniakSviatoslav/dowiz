#![allow(unused)]
//! `dowiz_core::chronos` — time navigation: snapshots, state history, time-travel (pure core).
//!
//! Every system state change is recorded as a timestamped snapshot. The chronos
//! engine allows navigation between snapshots: rewind to any point in time, see
//! the full system state at that moment, diff between any two timestamps.
//!
//! Time is the universal axis — all modules write their state here, and any
//! agent can query "what was the state at t = X?"
//!
//! ZERO deps. Uses the core SHA3-256 for snapshot integrity. `Chronos::snapshot_at`
//! takes `now_ms` explicitly so this module stays `no_std`; the kernel shim wraps
//! it as `snapshot_now` which stamps `crate::now_ms()`.

use crate::event_log::sha3_256;
use crate::trig::Xyz;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// A single system state snapshot at a specific timestamp.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub id: u64,
    pub timestamp_ms: u64,
    /// Named state dimensions (e.g. "order_count", "cpu_load", "confidence").
    pub values: BTreeMap<String, f64>,
    /// XYZ encoding of the composite state.
    pub xyz: Xyz,
    /// SHA3-256 integrity hash of (timestamp + sorted values).
    pub integrity: [u8; 32],
}

impl Snapshot {
    pub fn new(timestamp_ms: u64, values: BTreeMap<String, f64>) -> Self {
        let id = timestamp_ms; // monotonic (ms granularity)
        let xyz = Snapshot::compute_xyz(&values);
        let integrity = Snapshot::compute_hash(timestamp_ms, &values);
        Snapshot { id, timestamp_ms, values, xyz, integrity }
    }

    fn compute_xyz(values: &BTreeMap<String, f64>) -> Xyz {
        let mut keys: Vec<&String> = values.keys().collect();
        keys.sort();
        let mut x = 0.0f64; let mut y = 0.0f64; let mut z = 0.0f64;
        for (i, k) in keys.iter().enumerate() {
            let v = values.get(*k).copied().unwrap_or(0.0).clamp(-1.0, 1.0);
            match i % 3 {
                0 => x = v,
                1 => y = v,
                _ => z = (z + v) / 2.0,
            }
        }
        Xyz::new(x, y, z)
    }

    fn compute_hash(ts: u64, values: &BTreeMap<String, f64>) -> [u8; 32] {
        let mut data = ts.to_le_bytes().to_vec();
        let mut keys: Vec<&String> = values.keys().collect();
        keys.sort();
        for k in keys {
            data.extend_from_slice(k.as_bytes());
            data.extend_from_slice(&values[k].to_le_bytes());
        }
        sha3_256(&data)
    }

    /// Verify integrity.
    pub fn verify(&self) -> bool {
        Snapshot::compute_hash(self.timestamp_ms, &self.values) == self.integrity
    }
}

/// The chronos engine — a time-indexed history of system snapshots.
#[derive(Debug, Clone)]
pub struct Chronos {
    pub snapshots: Vec<Snapshot>,
    /// Max snapshots retained (sliding window).
    pub capacity: usize,
    /// Index: dimension name → last known values for interpolation.
    pub dimensions: BTreeMap<String, Vec<f64>>,
}

impl Chronos {
    pub fn new(capacity: usize) -> Self {
        Chronos { snapshots: Vec::with_capacity(capacity), capacity: capacity.max(1), dimensions: BTreeMap::new() }
    }

    /// Record a snapshot of system state at the explicit `now_ms` timestamp.
    pub fn snapshot_at(&mut self, values: BTreeMap<String, f64>, now_ms: u64) -> &Snapshot {
        let snap = Snapshot::new(now_ms, values.clone());
        self.snapshots.push(snap);
        if self.snapshots.len() > self.capacity {
            self.snapshots.remove(0);
        }
        for (k, v) in values {
            self.dimensions.entry(k).or_default().push(v);
        }
        self.snapshots.last().unwrap()
    }

    /// Find the snapshot closest to a given timestamp (linear scan — ok for small N).
    pub fn at(&self, timestamp_ms: u64) -> Option<&Snapshot> {
        self.snapshots.iter()
            .min_by_key(|s| (s.timestamp_ms as i64 - timestamp_ms as i64).unsigned_abs())
    }

    /// Get all snapshots in a time window.
    pub fn window(&self, from_ms: u64, to_ms: u64) -> Vec<&Snapshot> {
        self.snapshots.iter()
            .filter(|s| s.timestamp_ms >= from_ms && s.timestamp_ms <= to_ms)
            .collect()
    }

    /// Delta between two timestamps: XYZ distance + per-dimension deltas.
    pub fn delta(&self, t1: u64, t2: u64) -> Option<(f64, BTreeMap<String, f64>)> {
        let s1 = self.at(t1)?;
        let s2 = self.at(t2)?;
        let xyz_delta = s1.xyz.distance(&s2.xyz);
        let mut dim_deltas = BTreeMap::new();
        for k in s1.values.keys() {
            let v1 = s1.values.get(k).copied().unwrap_or(0.0);
            let v2 = s2.values.get(k).copied().unwrap_or(0.0);
            dim_deltas.insert(k.clone(), v2 - v1);
        }
        Some((xyz_delta, dim_deltas))
    }

    /// Latest snapshot.
    pub fn latest(&self) -> Option<&Snapshot> { self.snapshots.last() }

    /// Total recorded snapshots.
    pub fn len(&self) -> usize { self.snapshots.len() }

    /// Time range: (earliest, latest).
    pub fn time_range(&self) -> Option<(u64, u64)> {
        let first = self.snapshots.first()?;
        let last = self.snapshots.last()?;
        Some((first.timestamp_ms, last.timestamp_ms))
    }

    /// Interpolate state between two nearest snapshots.
    pub fn interpolate(&self, timestamp_ms: u64) -> Option<BTreeMap<String, f64>> {
        if self.snapshots.is_empty() { return None; }
        // Find nearest before and after
        let before = self.snapshots.iter()
            .filter(|s| s.timestamp_ms <= timestamp_ms)
            .last();
        let after = self.snapshots.iter()
            .filter(|s| s.timestamp_ms >= timestamp_ms)
            .next();
        match (before, after) {
            (Some(b), Some(a)) if b.timestamp_ms != a.timestamp_ms => {
                let w = (timestamp_ms - b.timestamp_ms) as f64
                    / (a.timestamp_ms - b.timestamp_ms) as f64;
                let mut result = BTreeMap::new();
                for k in b.values.keys() {
                    let v1 = b.values.get(k).copied().unwrap_or(0.0);
                    let v2 = a.values.get(k).copied().unwrap_or(0.0);
                    result.insert(k.clone(), v1 + (v2 - v1) * w);
                }
                Some(result)
            }
            (Some(b), _) => Some(b.values.clone()),
            (None, Some(a)) => Some(a.values.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_values(x: f64, y: f64) -> BTreeMap<String, f64> {
        let mut m = BTreeMap::new();
        m.insert("x".into(), x);
        m.insert("y".into(), y);
        m
    }

    #[test]
    fn snapshot_integrity_verifies() {
        let s = Snapshot::new(1000, make_values(0.5, -0.3));
        assert!(s.verify());
    }

    #[test]
    fn chronos_snapshot_and_retrieve() {
        let mut c = Chronos::new(100);
        c.snapshot_at(make_values(0.1, 0.2), 1000);
        c.snapshot_at(make_values(0.5, 0.8), 1010);
        c.snapshot_at(make_values(0.9, 0.3), 1020);
        assert_eq!(c.len(), 3);
        // Latest should carry the most recent injected timestamp.
        let latest = c.latest().unwrap();
        assert!(latest.timestamp_ms >= 1020);
    }

    #[test]
    fn chronos_delta_between_timestamps() {
        let mut c = Chronos::new(100);
        let ts0 = 1000u64;
        let snap0 = Snapshot::new(ts0, make_values(0.0, 0.0));
        let snap2 = Snapshot::new(ts0 + 500, make_values(1.0, 1.0));
        c.snapshots.push(snap0);
        c.snapshots.push(Snapshot::new(ts0 + 200, make_values(0.5, 0.5)));
        c.snapshots.push(snap2);
        let (xyz_d, dims) = c.delta(ts0, ts0 + 500).unwrap();
        assert!(xyz_d >= 0.0, "xyz_delta={xyz_d}");
        assert!((dims["x"] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn chronos_interpolate() {
        let mut c = Chronos::new(100);
        c.snapshot_at(make_values(0.0, 0.0), 1000);
        c.snapshot_at(make_values(1.0, 2.0), 1500);
        c.snapshot_at(make_values(2.0, 4.0), 2000);
        let interp = c.interpolate(1750);
        assert!(interp.is_some());
        let v = interp.unwrap();
        assert!(v["x"] >= 0.0);
        assert!(v["y"] >= 0.0);
    }

    #[test]
    fn chronos_window_queries() {
        let mut c = Chronos::new(100);
        let s1 = c.snapshot_at(make_values(0.0, 0.0), 1000).timestamp_ms;
        let s2 = c.snapshot_at(make_values(0.5, 0.5), 1500).timestamp_ms;
        let window = c.window(s1, s2);
        assert!(window.len() >= 2);
    }

    #[test]
    fn snapshot_tamper_detection() {
        let mut s = Snapshot::new(1000, make_values(0.5, -0.3));
        assert!(s.verify());
        s.values.insert("x".into(), 999.0); // tamper
        assert!(!s.verify());
    }
}
