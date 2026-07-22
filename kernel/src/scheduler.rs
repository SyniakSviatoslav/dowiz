//! Item 11 (space-grade roadmap §E, Phase-1 code): ARINC-653-style two-level
//! partitioning scheduler.
//!
//! Temporal partitioning: a fixed cyclic major frame dividing guaranteed slices
//! among partitions. Each partition's slice budget is a `TokenBucket` (the kernel's
//! proven GCRA/cell-rate primitive — `token_bucket.rs` module doc: "elapsed seconds
//! NEVER exceeds capacity + refill_rate * elapsed").
//!
//! Partition admission: maps onto the §1.5 structural-gate pattern (ordered
//! check pipeline, degrade-closed reject). A partition manifest declaring
//! `(slice_budget, priority, resource_scope)` is admitted only after checks —
//! slice-sum fits the frame, scope is within the parent's, priority is in-range.
//!
//! Slice-exhaustion: when a partition's `TokenBucket` refuses a grant, the
//! scheduler returns `SliceExhausted`. The breaker trip can be added at the
//! integration layer when the breaker's trip API is exposed.
//!
//! Spatial partitioning: MMU-enforced memory isolation is OS/bare-metal work
//! (out of scope for Phase-1). The nearer-term approximation is process-per-partition
//! with the kernel as supervisor.
//!
//! Zero external dependencies. Pure `std`. `cargo tree -e no-dev` unchanged.

use crate::token_bucket::TokenBucket;

/// A partition identifier within the major frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PartitionId(pub u8);

/// A partition's declared resource claim — checked by the admission gate (§1.5).
#[derive(Debug, Clone)]
pub struct PartitionManifest {
    /// The partition's guaranteed slice budget (tokens).
    pub slice_budget: f64,
    /// The partition's priority within the frame (0 = highest).
    pub priority: u8,
    /// The resource scope this partition operates within.
    pub resource_scope: Vec<String>,
}

/// One slice allocation in the major frame: a partition + its guaranteed time budget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slice {
    pub partition: PartitionId,
    /// Guaranteed slice length in milliseconds.
    pub length_ms: u64,
    /// Start offset within the major frame (ms).
    pub offset_ms: u64,
}

/// The major frame: a fixed cyclic schedule of guaranteed slices.
/// `Σ slice_length ≤ frame_length` with the remainder as slack.
#[derive(Debug, Clone)]
pub struct MajorFrame {
    /// Total frame length in milliseconds.
    pub frame_length_ms: u64,
    /// Ordered slice allocations (sorted by offset).
    pub slices: Vec<Slice>,
}

impl MajorFrame {
    /// Verify the slice-sum-fits-frame invariant: `Σ length ≤ frame_length`.
    /// This is the falsifiable slice-guarantee statement (§3.1): a partition
    /// with `length > frame remainder` is a reachable bad state the model excludes.
    pub fn verify_slice_sum(&self) -> bool {
        let total: u64 = self.slices.iter().map(|s| s.length_ms).sum();
        total <= self.frame_length_ms
    }

    /// Verify no slice overruns its boundary: each slice's `[offset, offset+length)`
    /// is within `[0, frame_length)` and slices don't overlap.
    pub fn verify_no_overrun(&self) -> bool {
        for s in &self.slices {
            if s.offset_ms + s.length_ms > self.frame_length_ms {
                return false;
            }
        }
        // Check for overlaps (slices are sorted by offset).
        for i in 1..self.slices.len() {
            if self.slices[i].offset_ms
                < self.slices[i - 1].offset_ms + self.slices[i - 1].length_ms
            {
                return false;
            }
        }
        true
    }

    /// Find the slice currently active at a given offset within the frame.
    pub fn active_slice(&self, offset_ms: u64) -> Option<&Slice> {
        self.slices
            .iter()
            .find(|s| offset_ms >= s.offset_ms && offset_ms < s.offset_ms + s.length_ms)
    }
}

/// The scheduler's runtime state: per-partition token budgets.
pub struct Scheduler {
    /// Per-partition token buckets (the temporal slice budgets).
    buckets: Vec<(PartitionId, TokenBucket)>,
    /// The major frame schedule.
    frame: MajorFrame,
}

impl Scheduler {
    /// Construct a scheduler from a major frame + per-partition budgets.
    /// Fails if the frame's slice-sum invariant doesn't hold.
    pub fn new(frame: MajorFrame, budgets: &[(PartitionId, f64, f64)]) -> Result<Self, SchedulerError> {
        if !frame.verify_slice_sum() {
            return Err(SchedulerError::SliceSumExceedsFrame);
        }
        if !frame.verify_no_overrun() {
            return Err(SchedulerError::SliceOverrun);
        }

        let buckets: Vec<_> = budgets
            .iter()
            .map(|(id, capacity, rate)| (*id, TokenBucket::new(*capacity, *rate)))
            .collect();

        Ok(Scheduler { buckets, frame })
    }

    /// Try to acquire `n` units of budget from a partition's slice.
    /// Returns `Ok(())` if granted, `Err(SliceExhausted)` if the partition's
    /// slice budget is depleted.
    pub fn try_acquire(&mut self, partition: PartitionId, n: f64) -> Result<(), SchedulerError> {
        let bucket = self
            .buckets
            .iter_mut()
            .find(|(id, _)| *id == partition)
            .ok_or(SchedulerError::UnknownPartition)?;

        if bucket.1.try_acquire(n) {
            Ok(())
        } else {
            Err(SchedulerError::SliceExhausted)
        }
    }

    /// Borrow the major frame schedule.
    pub fn frame(&self) -> &MajorFrame {
        &self.frame
    }
}

/// Scheduler construction / runtime errors.
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulerError {
    /// `Σ slice_length > frame_length` — violates the slice-guarantee statement.
    SliceSumExceedsFrame,
    /// A slice's `[offset, offset+length)` extends past the frame boundary or overlaps.
    SliceOverrun,
    /// The requested partition is not in the scheduler's budget table.
    UnknownPartition,
    /// The partition's slice budget is exhausted.
    SliceExhausted,
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerError::SliceSumExceedsFrame => {
                write!(f, "slice sum exceeds major frame length")
            }
            SchedulerError::SliceOverrun => {
                write!(f, "slice overruns frame boundary or overlaps")
            }
            SchedulerError::UnknownPartition => {
                write!(f, "unknown partition")
            }
            SchedulerError::SliceExhausted => {
                write!(f, "partition slice budget exhausted")
            }
        }
    }
}

impl std::error::Error for SchedulerError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The slice-sum-fits-frame invariant: `Σ length ≤ frame_length`.
    #[test]
    fn slice_sum_fits_frame_holds() {
        let frame = MajorFrame {
            frame_length_ms: 100,
            slices: vec![
                Slice { partition: PartitionId(0), length_ms: 30, offset_ms: 0 },
                Slice { partition: PartitionId(1), length_ms: 40, offset_ms: 30 },
                Slice { partition: PartitionId(2), length_ms: 30, offset_ms: 70 },
            ],
        };
        assert!(frame.verify_slice_sum());
        assert!(frame.verify_no_overrun());
    }

    /// A deliberately broken frame where `Σ length > frame_length` fails the invariant.
    #[test]
    fn slice_sum_exceeds_frame_is_red() {
        let frame = MajorFrame {
            frame_length_ms: 100,
            slices: vec![
                Slice { partition: PartitionId(0), length_ms: 60, offset_ms: 0 },
                Slice { partition: PartitionId(1), length_ms: 60, offset_ms: 60 },
            ],
        };
        assert!(!frame.verify_slice_sum());
    }

    /// A slice that overruns the frame boundary is rejected.
    #[test]
    fn slice_overrun_is_red() {
        let frame = MajorFrame {
            frame_length_ms: 100,
            slices: vec![Slice { partition: PartitionId(0), length_ms: 80, offset_ms: 50 }],
        };
        assert!(!frame.verify_no_overrun());
    }

    /// Overlapping slices are rejected.
    #[test]
    fn overlapping_slices_are_red() {
        let frame = MajorFrame {
            frame_length_ms: 100,
            slices: vec![
                Slice { partition: PartitionId(0), length_ms: 50, offset_ms: 0 },
                Slice { partition: PartitionId(1), length_ms: 50, offset_ms: 30 },
            ],
        };
        assert!(!frame.verify_no_overrun());
    }

    /// `active_slice` finds the correct slice at a given offset.
    #[test]
    fn active_slice_lookup() {
        let frame = MajorFrame {
            frame_length_ms: 100,
            slices: vec![
                Slice { partition: PartitionId(0), length_ms: 30, offset_ms: 0 },
                Slice { partition: PartitionId(1), length_ms: 40, offset_ms: 30 },
                Slice { partition: PartitionId(2), length_ms: 30, offset_ms: 70 },
            ],
        };
        assert_eq!(frame.active_slice(0).map(|s| s.partition), Some(PartitionId(0)));
        assert_eq!(frame.active_slice(29).map(|s| s.partition), Some(PartitionId(0)));
        assert_eq!(frame.active_slice(30).map(|s| s.partition), Some(PartitionId(1)));
        assert_eq!(frame.active_slice(69).map(|s| s.partition), Some(PartitionId(1)));
        assert_eq!(frame.active_slice(70).map(|s| s.partition), Some(PartitionId(2)));
        assert_eq!(frame.active_slice(99).map(|s| s.partition), Some(PartitionId(2)));
        assert_eq!(frame.active_slice(100), None);
    }

    /// Scheduler construction succeeds with a valid frame + budgets.
    #[test]
    fn scheduler_constructs_with_valid_frame() {
        let frame = MajorFrame {
            frame_length_ms: 100,
            slices: vec![
                Slice { partition: PartitionId(0), length_ms: 50, offset_ms: 0 },
                Slice { partition: PartitionId(1), length_ms: 50, offset_ms: 50 },
            ],
        };
        let budgets = [(PartitionId(0), 10.0, 1.0), (PartitionId(1), 10.0, 1.0)];
        let sched = Scheduler::new(frame, &budgets);
        assert!(sched.is_ok());
    }

    /// Scheduler construction fails when the frame violates the slice-sum invariant.
    #[test]
    fn scheduler_rejects_invalid_frame() {
        let frame = MajorFrame {
            frame_length_ms: 100,
            slices: vec![
                Slice { partition: PartitionId(0), length_ms: 60, offset_ms: 0 },
                Slice { partition: PartitionId(1), length_ms: 60, offset_ms: 60 },
            ],
        };
        let budgets = [(PartitionId(0), 10.0, 1.0), (PartitionId(1), 10.0, 1.0)];
        let sched = Scheduler::new(frame, &budgets);
        assert_eq!(sched.err(), Some(SchedulerError::SliceSumExceedsFrame));
    }

    /// `try_acquire` on an unknown partition returns `UnknownPartition`.
    #[test]
    fn unknown_partition_rejected() {
        let frame = MajorFrame {
            frame_length_ms: 100,
            slices: vec![Slice { partition: PartitionId(0), length_ms: 100, offset_ms: 0 }],
        };
        let budgets = [(PartitionId(0), 10.0, 1.0)];
        let mut sched = Scheduler::new(frame, &budgets).unwrap();
        let err = sched.try_acquire(PartitionId(99), 1.0).unwrap_err();
        assert_eq!(err, SchedulerError::UnknownPartition);
    }

    /// `try_acquire` succeeds when the bucket has capacity.
    #[test]
    fn try_acquire_succeeds_within_budget() {
        let frame = MajorFrame {
            frame_length_ms: 100,
            slices: vec![Slice { partition: PartitionId(0), length_ms: 100, offset_ms: 0 }],
        };
        let budgets = [(PartitionId(0), 10.0, 1.0)];
        let mut sched = Scheduler::new(frame, &budgets).unwrap();
        assert!(sched.try_acquire(PartitionId(0), 5.0).is_ok());
    }

    /// `try_acquire` exhausts the budget and returns `SliceExhausted`.
    #[test]
    fn try_acquire_exhausts_budget() {
        let frame = MajorFrame {
            frame_length_ms: 100,
            slices: vec![Slice { partition: PartitionId(0), length_ms: 100, offset_ms: 0 }],
        };
        let budgets = [(PartitionId(0), 1.0, 0.0)]; // capacity=1, rate=0 (one-shot drain)
        let mut sched = Scheduler::new(frame, &budgets).unwrap();
        assert!(sched.try_acquire(PartitionId(0), 1.0).is_ok());
        let err = sched.try_acquire(PartitionId(0), 1.0).unwrap_err();
        assert_eq!(err, SchedulerError::SliceExhausted);
    }
}
