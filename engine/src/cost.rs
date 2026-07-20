//! Item 60 (gaps G3 + G11) — the item-58 `(work, cost)` cost pair, engine side.
//!
//! Per BLUEPRINT-ITEM-60 §3 step 2 / §5 criterion 5, frame and voice records
//! carry the item-58 `(FramesRendered, Δframe_us)` pair. Item 58 itself is the
//! authority for `WorkloadKind` and the `Work { kind, delta_count }` shape; it is
//! **not present in this tree** (prerequisite not landed here), so this module
//! carries the engine's local, item-58-shaped mirror so the emission side is
//! wired and testable. When item 58 lands, `WorkloadKind`/`FramesRendered` become
//! the single authority and this mirror is collapsed into it (one owner, no
//! schema drift).
//!
//! **No ratio field** (item-58 law): a record is a raw-f64 `(work Δcount, cost
//! Δmicros)` pair. Ratios (e.g. frames/sec, asr_feeds/sec, µs/frame) are computed
//! consumer-side from the raw pair — never stored.
//!
//! Feature-gated (`telemetry`): empty on the default build so the offline-clean
//! byte set is unchanged; only the heavy pair emission turns on.

/// The item-58-shaped workload kind the engine emits. Mirrors `WorkloadKind` from
/// item 58 (which owns the canonical enum). `FramesRendered` = frame-loop work
/// unit (G3); `AsrFeeds` = voice wake-gated ASR feed work unit (G11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadKind {
    FramesRendered,
    AsrFeeds,
}

impl WorkloadKind {
    /// Greppable serialized name (mirrors `Kind::as_str` / `Absence::as_str`).
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkloadKind::FramesRendered => "frames_rendered",
            WorkloadKind::AsrFeeds => "asr_feeds",
        }
    }
}

/// The item-58-shaped numerator: a workload kind + its raw δcount (u64, no rate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Work {
    pub kind: WorkloadKind,
    pub delta_count: u64,
}

/// A single item-58 cost-pair record. Raw counts + raw microseconds. NO ratio.
/// `(work, cost)` is the pair; `cost_us` is the Δframe_us / Δasr_latency_us.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostPair {
    pub work: Work,
    pub cost_us: u64,
}

impl CostPair {
    /// Build a frame-loop pair: `Δframe_us` is the measured frame cost.
    pub fn frame(delta_frames: u64, delta_frame_us: u64) -> Self {
        CostPair {
            work: Work {
                kind: WorkloadKind::FramesRendered,
                delta_count: delta_frames,
            },
            cost_us: delta_frame_us,
        }
    }

    /// Build a voice pair: `Δasr_latency_us` is the wake-gated ASR feed latency.
    pub fn asr(delta_feeds: u64, delta_asr_latency_us: u64) -> Self {
        CostPair {
            work: Work {
                kind: WorkloadKind::AsrFeeds,
                delta_count: delta_feeds,
            },
            cost_us: delta_asr_latency_us,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The pair carries raw work + raw cost; NO ratio field exists.
    #[test]
    fn pair_is_raw_no_ratio() {
        let p = CostPair::frame(1, 12_345);
        assert_eq!(p.work.kind, WorkloadKind::FramesRendered);
        assert_eq!(p.work.delta_count, 1);
        assert_eq!(p.cost_us, 12_345);
        assert_eq!(p.work.kind.as_str(), "frames_rendered");

        let a = CostPair::asr(1, 107_000);
        assert_eq!(a.work.kind, WorkloadKind::AsrFeeds);
        assert_eq!(a.work.kind.as_str(), "asr_feeds");
    }
}
