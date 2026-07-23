//! Channel analytics — closes the open attribution measurement loop.
//!
//! The oracle (`apps/api/src/routes/orders.ts`) captures order attribution via
//! `order_events` but has **zero readers on prod** (roadmap finding: "attribution
//! captured but ZERO readers on prod; measurement loop open"). `ChannelLedger` is
//! the first deterministic reader: it ingests `(order_id, channel, status, at_ms)`
//! events and answers the two questions that were previously unanswerable:
//!
//!   * `orders_by_channel()` — how many orders came from each acquisition channel?
//!   * `funnel(channel)`      — where in the lifecycle do orders in a channel stall?
//!
//! A second reducer, `reduce_anomalies`, folds a raw `(order_id, status)` event
//! stream through [`order_machine::fold_transitions`] to detect illegal state
//! sequences that would otherwise corrupt the funnel counts.
//!
//! Pure std only (HashMap). WASM/headless safe. No float, no I/O. No courier
//! scoring — this module only measures channel attribution, it does not rank
//! couriers.

use std::collections::HashMap;

use crate::order_machine::{fold_transitions, OrderStatus};

/// One ingested event. `at_ms` is monotonic wall-clock ms (kept for funnel
/// latency math later; not used in v1 counting).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelEvent {
    pub order_id: &'static str,
    pub channel: &'static str,
    pub status: OrderStatus,
    pub at_ms: i64,
}

/// Deterministic, idempotent ledger of channel-attributed orders.
///
/// `ingest` rejects a duplicate `order_id` (the same order cannot be attributed
/// to two channels) — the second sighting is ignored, not overwritten. Counts are
/// exact because each `order_id` maps to exactly one `(channel, latest_status)`.
#[derive(Debug, Default)]
pub struct ChannelLedger {
    /// order_id -> (channel, current status)
    orders: HashMap<String, (String, OrderStatus)>,
    /// channel -> count of distinct orders
    by_channel: HashMap<String, u64>,
    /// (channel, status) -> count of orders currently in that status
    funnel_counts: HashMap<(String, OrderStatus), u64>,
}

impl ChannelLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingest one event. Returns `false` if the `order_id` was already seen
    /// (duplicate rejected/ignored), `true` if it was newly recorded.
    ///
    /// The status updates the order's latest known status (so a funnel reflects
    /// the most recent event for that order), but the channel is fixed at first
    /// sighting — re-attributing an order to a different channel is ignored.
    pub fn ingest(&mut self, ev: ChannelEvent) -> bool {
        if let Some(slot) = self.orders.get_mut(ev.order_id) {
            // Duplicate order_id: ignore. Channel is locked to first sighting.
            // Update funnel to the new status (remove old, add new).
            let old_status = slot.1;
            let channel = slot.0.clone();
            self.funnel_counts
                .entry((channel.clone(), old_status))
                .and_modify(|c| *c = c.saturating_sub(1));
            slot.1 = ev.status;
            *self.funnel_counts.entry((channel, ev.status)).or_insert(0) += 1;
            return false;
        }
        // New order.
        let mut channel = String::new();
        channel.push_str(ev.channel);
        *self.by_channel.entry(channel.clone()).or_insert(0) += 1;
        *self
            .funnel_counts
            .entry((channel.clone(), ev.status))
            .or_insert(0) += 1;
        self.orders.insert(
            {
                let mut id = String::new();
                id.push_str(ev.order_id);
                id
            },
            (channel, ev.status),
        );
        true
    }

    /// Distinct order count per channel, descending by count then channel name.
    pub fn orders_by_channel(&self) -> Vec<(String, u64)> {
        let mut out: Vec<(String, u64)> = self
            .by_channel
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        out
    }

    /// Current status distribution for a single channel, in `OrderStatus` enum
    /// declaration order (canonical funnel stage ordering). Stages with zero
    /// orders are included with count 0 so the funnel always has a fixed shape
    /// (a missing stage reads as 0, never as "absent").
    pub fn funnel(&self, channel: &str) -> Vec<(OrderStatus, u64)> {
        use OrderStatus::*;
        let stage_order = [
            Pending, Confirmed, Preparing, Ready, InDelivery, Delivered, Rejected, Cancelled,
            Scheduled, PickedUp,
        ];
        stage_order
            .iter()
            .map(|s| {
                (
                    *s,
                    *self
                        .funnel_counts
                        .get(&(channel.to_string(), *s))
                        .unwrap_or(&0),
                )
            })
            .collect()
    }
}

/// Cohort retention tracking by acquisition week.
///
/// Each cohort is keyed by an opaque string (e.g. `"2026-W30"`).
/// `record_acquisition` bumps the cohort size and marks the user active at week 0.
/// `record_activity` marks a user active at a given number of weeks after acquisition.
/// `retention_rate` returns `week_N_active / cohort_size`, or `None` if the cohort is absent.
#[derive(Debug, Default)]
pub struct CohortRetention {
    /// cohort_key (e.g. "2026-W30") → [week0_active, week1_retained, week2_retained, ...]
    pub cohorts: HashMap<String, Vec<u64>>,
    /// Total users acquired per cohort
    pub cohort_size: HashMap<String, u64>,
}

impl CohortRetention {
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment the cohort's acquisition count and record the user as active at week 0.
    pub fn record_acquisition(&mut self, cohort_key: &str) {
        let key = cohort_key.to_string();
        *self.cohort_size.entry(key.clone()).or_insert(0) += 1;
        let weeks = self.cohorts.entry(key).or_default();
        if weeks.is_empty() {
            weeks.push(1);
        } else {
            weeks[0] = weeks[0].saturating_add(1);
        }
    }

    /// Mark a user from `cohort_key` active `weeks_since_acquisition` weeks after acquisition.
    /// The `weeks` vector is grown to fit if needed, filling intermediate weeks with 0.
    pub fn record_activity(&mut self, cohort_key: &str, weeks_since_acquisition: usize) {
        let weeks = self
            .cohorts
            .entry(cohort_key.to_string())
            .or_default();
        if weeks_since_acquisition >= weeks.len() {
            weeks.resize(weeks_since_acquisition + 1, 0);
        }
        weeks[weeks_since_acquisition] = weeks[weeks_since_acquisition].saturating_add(1);
    }

    /// Retention rate at `week` for `cohort_key`: `active_at_week / cohort_size`, or `None` if unknown.
    pub fn retention_rate(&self, cohort_key: &str, week: usize) -> Option<f64> {
        let size = *self.cohort_size.get(cohort_key)?;
        if size == 0 {
            return None;
        }
        let weeks = self.cohorts.get(cohort_key)?;
        let active = weeks.get(week).copied().unwrap_or(0);
        Some(active as f64 / size as f64)
    }

    /// Compact text summary of every known cohort.
    pub fn dashboard(&self) -> String {
        let mut keys: Vec<&String> = self.cohorts.keys().collect();
        keys.sort();
        let mut out = String::new();
        for key in keys {
            let size = self.cohort_size.get(key).copied().unwrap_or(0);
            let weeks = self.cohorts.get(key).map(|w| w.as_slice()).unwrap_or(&[]);
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!("cohort={key} size={size} weeks={weeks:?}\n"),
            );
        }
        out
    }
}

/// Reduce a raw `(order_id, status)` event stream into anomalies.
///
/// Groups events per `order_id`, orders them by `at_ms`, then folds the status
/// sequence through [`order_machine::fold_transitions`] starting from
/// `OrderStatus::Pending`. Every order whose sequence contains an illegal
/// transition counts as exactly one anomaly. Returns the anomaly count.
///
/// This reuses the kernel's canonical `decide/fold` Law rather than re-implementing
/// a transition check — the measurement loop must agree with the source of truth.
pub fn reduce_anomalies(events: &[(String, OrderStatus, i64)]) -> u64 {
    use std::collections::BTreeMap;

    // order_id -> events sorted by at_ms (BTreeMap gives ascending key order).
    let mut by_order: HashMap<&str, BTreeMap<i64, OrderStatus>> = HashMap::new();
    for (id, status, at) in events {
        by_order
            .entry(id.as_str())
            .or_default()
            .insert(*at, *status);
    }

    let mut anomalies = 0u64;
    for (_id, seq) in by_order {
        let statuses: Vec<OrderStatus> = seq.values().copied().collect();
        if statuses.is_empty() {
            continue;
        }
        // The first observed status is this order's *start* state; the remaining
        // observations are the transitions to fold. (Pending → Pending as a step
        // would spuriously read as a SameStatus anomaly — it is the seed, not a step.)
        let start = statuses[0];
        let steps = &statuses[1..];
        if fold_transitions(start, steps).is_err() {
            anomalies += 1;
        }
    }
    anomalies
}

#[cfg(test)]
mod tests {
    use super::*;
    use OrderStatus::*;

    // ── RED: a duplicate order_id must be rejected/ignored ──
    #[test]
    fn red_duplicate_order_id_rejected() {
        let mut ledger = ChannelLedger::new();
        assert!(ledger.ingest(ChannelEvent {
            order_id: "o1",
            channel: "tiktok",
            status: Pending,
            at_ms: 1,
        }));
        // Same id, different channel — must be ignored (no recount, no re-attribute).
        assert!(!ledger.ingest(ChannelEvent {
            order_id: "o1",
            channel: "instagram",
            status: Confirmed,
            at_ms: 2,
        }));
        let ch = ledger.orders_by_channel();
        assert_eq!(ch.len(), 1, "only one channel should be recorded");
        assert_eq!(ch[0], ("tiktok".to_string(), 1));
        // funnel should reflect the updated status (Confirmed), not double count.
        let f = ledger.funnel("tiktok");
        let confirmed = f.iter().find(|(s, _)| *s == Confirmed).unwrap().1;
        assert_eq!(confirmed, 1);
        let pending = f.iter().find(|(s, _)| *s == Pending).unwrap().1;
        assert_eq!(pending, 0);
    }

    // ── GREEN: ingest a mixed sample, assert counts ──
    #[test]
    fn green_mixed_sample_counts() {
        let mut ledger = ChannelLedger::new();
        let sample = [
            ChannelEvent {
                order_id: "a1",
                channel: "tiktok",
                status: Pending,
                at_ms: 1,
            },
            ChannelEvent {
                order_id: "a2",
                channel: "tiktok",
                status: Confirmed,
                at_ms: 2,
            },
            ChannelEvent {
                order_id: "a3",
                channel: "tiktok",
                status: Delivered,
                at_ms: 3,
            },
            ChannelEvent {
                order_id: "b1",
                channel: "instagram",
                status: Pending,
                at_ms: 4,
            },
            ChannelEvent {
                order_id: "b2",
                channel: "instagram",
                status: Rejected,
                at_ms: 5,
            },
            ChannelEvent {
                order_id: "c1",
                channel: "organic",
                status: Delivered,
                at_ms: 6,
            },
        ];
        for ev in sample {
            assert!(ledger.ingest(ev));
        }

        let by = ledger.orders_by_channel();
        let map: HashMap<&str, u64> = by.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        assert_eq!(map["tiktok"], 3);
        assert_eq!(map["instagram"], 2);
        assert_eq!(map["organic"], 1);
        assert_eq!(by.len(), 3);

        // tiktok funnel: 1 pending, 1 confirmed, 1 delivered.
        let tiktok = ledger.funnel("tiktok");
        let tm: HashMap<OrderStatus, u64> = tiktok.iter().map(|(s, c)| (*s, *c)).collect();
        assert_eq!(tm[&Pending], 1);
        assert_eq!(tm[&Confirmed], 1);
        assert_eq!(tm[&Delivered], 1);
        assert_eq!(tm[&Rejected], 0);

        // instagram funnel: 1 pending, 1 rejected.
        let ig = ledger.funnel("instagram");
        let igmap: HashMap<OrderStatus, u64> = ig.iter().map(|(s, c)| (*s, *c)).collect();
        assert_eq!(igmap[&Pending], 1);
        assert_eq!(igmap[&Rejected], 1);

        // unknown channel funnel is all-zero (fixed shape, 10 stages).
        let unknown = ledger.funnel("does-not-exist");
        assert_eq!(unknown.len(), 10);
        assert!(unknown.iter().all(|(_, c)| *c == 0));
    }

    // ── GREEN: anomaly reducer reuses fold_transitions ──
    #[test]
    fn green_anomaly_detects_illegal_sequence() {
        // o1: clean full happy path (Pending -> ... -> Delivered).
        // o2: illegal jump Pending -> Delivered (anomaly).
        // o3: out-of-order at_ms but legal sequence Pending -> Confirmed (not an anomaly).
        let events = vec![
            ("o1".to_string(), Pending, 1),
            ("o1".to_string(), Confirmed, 2),
            ("o1".to_string(), Preparing, 3),
            ("o1".to_string(), Ready, 4),
            ("o1".to_string(), InDelivery, 5),
            ("o1".to_string(), Delivered, 6),
            ("o2".to_string(), Pending, 4),
            ("o2".to_string(), Delivered, 5),
            ("o3".to_string(), Confirmed, 10),
            ("o3".to_string(), Pending, 9), // earlier at_ms; sorted → Pending -> Confirmed, legal
        ];
        let anomalies = reduce_anomalies(&events);
        assert_eq!(anomalies, 1, "exactly one illegal sequence (o2)");
    }

    #[test]
    fn green_anomaly_empty_stream() {
        assert_eq!(reduce_anomalies(&[]), 0);
    }

    #[test]
    fn green_anomaly_single_order_clean() {
        let events = vec![
            ("x".to_string(), Pending, 1),
            ("x".to_string(), Confirmed, 2),
            ("x".to_string(), Preparing, 3),
            ("x".to_string(), Ready, 4),
            ("x".to_string(), InDelivery, 5),
            ("x".to_string(), Delivered, 6),
        ];
        assert_eq!(reduce_anomalies(&events), 0);
    }

    // ── CohortRetention tests ──

    #[test]
    fn cohort_basic_retention() {
        let mut cr = CohortRetention::new();

        // 2026-W30: 10 users acquired
        for _ in 0..10 {
            cr.record_acquisition("2026-W30");
        }

        // Week 0 retention = 10/10 = 1.0
        assert_eq!(cr.retention_rate("2026-W30", 0), Some(1.0));

        // Week 1: 7 users came back
        for _ in 0..7 {
            cr.record_activity("2026-W30", 1);
        }
        assert_eq!(cr.retention_rate("2026-W30", 1), Some(0.7));

        // Week 2: 5 users came back
        for _ in 0..5 {
            cr.record_activity("2026-W30", 2);
        }
        assert_eq!(cr.retention_rate("2026-W30", 2), Some(0.5));

        // Unknown cohort
        assert_eq!(cr.retention_rate("2025-W01", 0), None);

        // Unknown week (beyond recorded range)
        assert_eq!(cr.retention_rate("2026-W30", 10), Some(0.0));
    }

    #[test]
    fn cohort_dashboard_includes_both_cohorts() {
        let mut cr = CohortRetention::new();
        cr.record_acquisition("2026-W30");
        cr.record_acquisition("2026-W31");
        cr.record_activity("2026-W30", 1);
        let d = cr.dashboard();
        assert!(d.contains("cohort=2026-W30"));
        assert!(d.contains("cohort=2026-W31"));
        assert!(d.contains("size=1"));
    }
}
