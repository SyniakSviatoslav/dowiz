//! parallel_patterns.rs — Reusable parallel execution pattern library.
//!
//! # What this is
//! Pure-computation patterns for dynamic parallel execution. Each pattern
//! produces an execution PLAN (task graph with resource assignments) that
//! the orchestrator dispatches. No real threads, no I/O — the kernel is
//! the planning authority; actual execution happens behind the port seam.
//!
//! # Design principles
//! - Kernel = pure computation: patterns produce plans, not threads
//! - Dynamic: pattern selection adapts to PID controller output + system load
//! - Priority-aware: higher-priority tasks get more resources
//! - All patterns compose: pipeline contains fan-outs, fan-in reads pipeline output

use crate::event_log::sha3_256;
use crate::orchestrator::Priority;
use crate::TriState;

// ─── Pattern: Fan-Out / Fan-In ────────────────────────────────────────────

/// A fan-out/fan-in execution plan.
///
/// Split input data into chunks, process each chunk independently, then
/// merge results. The number of workers is dynamically chosen based on
/// available concurrency.
#[derive(Debug, Clone)]
pub struct FanOutPlan {
    /// Total number of input items.
    pub input_count: usize,
    /// Number of parallel workers (from PID controller).
    pub worker_count: usize,
    /// How items are distributed across workers.
    pub distribution: Distribution,
    /// Priority of the fan-out tasks.
    pub priority: Priority,
    /// Estimated per-item processing time (microseconds).
    pub per_item_us: u64,
    /// Total estimated time (including merge).
    pub total_estimated_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Distribution {
    /// Each worker gets a contiguous block.
    Contiguous,
    /// Items are interleaved (round-robin across workers).
    Interleaved,
    /// Items are assigned by weight (larger items to faster workers).
    Weighted,
}

impl FanOutPlan {
    /// Create a fan-out plan from input parameters.
    pub fn plan(
        input_count: usize,
        available_workers: usize,
        per_item_us: u64,
        priority: Priority,
    ) -> Self {
        let worker_count = available_workers.max(1).min(input_count);
        if worker_count == 0 {
            return FanOutPlan {
                input_count: 0,
                worker_count: 1,
                distribution: Distribution::Contiguous,
                priority,
                per_item_us,
                total_estimated_us: 0,
            };
        }

        let items_per_worker = (input_count + worker_count - 1) / worker_count;
        let parallel_us = items_per_worker as u64 * per_item_us;
        // Merge overhead: O(items_per_worker * log2(worker_count)).
        let merge_us = (input_count as f64 * (worker_count as f64).log2().ceil() as f64) as u64;

        FanOutPlan {
            input_count,
            worker_count,
            distribution: Distribution::Contiguous,
            priority,
            per_item_us,
            total_estimated_us: parallel_us + merge_us,
        }
    }

    /// Generate the worker assignments: Vec<(worker_id, start, end)>.
    pub fn assignments(&self) -> Vec<(usize, usize, usize)> {
        if self.worker_count == 0 || self.input_count == 0 {
            return Vec::new();
        }
        let chunk_size = (self.input_count + self.worker_count - 1) / self.worker_count;
        let mut assignments = Vec::with_capacity(self.worker_count);
        for w in 0..self.worker_count {
            let start = w * chunk_size;
            let end = (start + chunk_size).min(self.input_count);
            if start < self.input_count {
                assignments.push((w, start, end));
            }
        }
        assignments
    }
}

// ─── Pattern: Pipeline ────────────────────────────────────────────────────

/// A pipeline execution plan: stages connected by bounded buffers.
///
/// Each stage processes items and passes them to the next. Stages can
/// run concurrently if there is sufficient capacity. Buffer sizes are
/// dynamically adjusted based on PID output.
#[derive(Debug, Clone)]
pub struct PipelinePlan {
    /// Number of pipeline stages.
    pub stage_count: usize,
    /// Buffer size between stages (backpressure bound).
    pub buffer_size: usize,
    /// Per-stage estimated latency (microseconds).
    pub stage_latencies: Vec<u64>,
    /// Total estimated latency (sum of all stages + buffer overhead).
    pub total_estimated_us: u64,
    /// Items that can be in-flight simultaneously.
    pub in_flight: usize,
}

impl PipelinePlan {
    /// Create a pipeline plan from stage latencies and concurrency.
    pub fn plan(stage_latencies: Vec<u64>, available_workers: usize) -> Self {
        let stage_count = stage_latencies.len();
        if stage_count == 0 {
            return PipelinePlan {
                stage_count: 0,
                buffer_size: 1,
                stage_latencies: vec![],
                total_estimated_us: 0,
                in_flight: 0,
            };
        }

        // Buffer size: scale with concurrency. More workers = larger buffers
        // to keep all stages fed.
        let buffer_size = (available_workers * 2).max(2);
        let total_estimated_us: u64 = stage_latencies.iter().sum();
        let in_flight = available_workers.min(stage_count);

        PipelinePlan {
            stage_count,
            buffer_size,
            stage_latencies,
            total_estimated_us,
            in_flight,
        }
    }

    /// Throughput estimate: items per second given the bottleneck stage.
    pub fn throughput_per_sec(&self) -> f64 {
        if self.stage_latencies.is_empty() {
            return 0.0;
        }
        let bottleneck_us = self.stage_latencies.iter().copied().max().unwrap();
        if bottleneck_us == 0 {
            return f64::INFINITY;
        }
        1_000_000.0 / bottleneck_us as f64
    }
}

// ─── Pattern: Work-Stealing Queue ─────────────────────────────────────────

/// A work-stealing queue plan: idle workers steal from busy ones.
///
/// Each worker has a local queue. When a worker's queue is empty, it
/// steals from the busiest worker's queue. This minimizes idle time
/// when task sizes are uneven.
#[derive(Debug, Clone)]
pub struct WorkStealingPlan {
    /// Per-worker queue sizes at planning time.
    pub worker_queues: Vec<usize>,
    /// Total items to process.
    pub total_items: usize,
    /// Estimated per-item time (microseconds).
    pub per_item_us: u64,
    /// Number of workers.
    pub worker_count: usize,
    /// Maximum queue imbalance (max - min) before stealing triggers.
    pub steal_threshold: usize,
}

impl WorkStealingPlan {
    /// Create a work-stealing plan from per-worker queue sizes.
    pub fn plan(
        worker_queues: Vec<usize>,
        per_item_us: u64,
        steal_threshold: usize,
    ) -> Self {
        let total_items: usize = worker_queues.iter().sum();
        let worker_count = worker_queues.len().max(1);

        WorkStealingPlan {
            worker_queues,
            total_items,
            per_item_us,
            worker_count,
            steal_threshold,
        }
    }

    /// Whether stealing should trigger (imbalance exceeds threshold).
    pub fn should_steal(&self) -> TriState {
        if self.worker_queues.is_empty() {
            return TriState::False;
        }
        let max_q = *self.worker_queues.iter().max().unwrap();
        let min_q = *self.worker_queues.iter().min().unwrap();
        TriState::from_bool(max_q.saturating_sub(min_q) > self.steal_threshold)
    }

    /// Identify the steal pair: (thief_id, victim_id).
    ///
    /// The empty queue steals from the fullest queue.
    pub fn steal_pair(&self) -> Option<(usize, usize)> {
        if self.worker_queues.is_empty() {
            return None;
        }
        let min_idx = self
            .worker_queues
            .iter()
            .enumerate()
            .min_by_key(|(_, &q)| q)
            .map(|(i, _)| i)?;
        let max_idx = self
            .worker_queues
            .iter()
            .enumerate()
            .max_by_key(|(_, &q)| q)
            .map(|(i, _)| i)?;

        if min_idx == max_idx {
            return None; // all equal
        }
        // Only suggest stealing if there's an actual imbalance.
        let min_q = self.worker_queues[min_idx];
        let max_q = self.worker_queues[max_idx];
        if max_q <= min_q {
            return None;
        }
        Some((min_idx, max_idx))
    }

    /// Estimated total time with optimal stealing (max queue * per_item_us).
    pub fn estimated_total_us(&self) -> u64 {
        let max_q = self.worker_queues.iter().copied().max().unwrap_or(0);
        max_q as u64 * self.per_item_us
    }
}

// ─── Pattern: Dynamic Batch ───────────────────────────────────────────────

/// A dynamic batch plan: adapt batch size based on load and resources.
///
/// Under low load, process items one-by-one (low latency). Under high load,
/// batch items together (higher throughput). The PID controller output
/// determines the batch size.
#[derive(Debug, Clone)]
pub struct DynamicBatchPlan {
    /// Total items to process.
    pub total_items: usize,
    /// Current batch size (from PID).
    pub batch_size: usize,
    /// Number of batches needed.
    pub batch_count: usize,
    /// Estimated per-batch time (microseconds).
    pub per_batch_us: u64,
    /// Total estimated time.
    pub total_estimated_us: u64,
}

impl DynamicBatchPlan {
    /// Create a dynamic batch plan.
    ///
    /// `pid_concurrency` = PID controller output (higher = more aggressive batching).
    /// `per_item_us` = base time per item.
    pub fn plan(total_items: usize, pid_concurrency: usize, per_item_us: u64) -> Self {
        // Batch size scales with PID output: more concurrency = larger batches
        // to amortize per-batch overhead.
        let batch_size = (pid_concurrency * 4).max(1);
        let batch_count = (total_items + batch_size - 1) / batch_size;
        let per_batch_us = (batch_size as u64) * per_item_us;
        let total_estimated_us = batch_count as u64 * per_batch_us;

        DynamicBatchPlan {
            total_items,
            batch_size,
            batch_count,
            per_batch_us,
            total_estimated_us,
        }
    }
}

// ─── Pattern Selection Engine ─────────────────────────────────────────────

/// Select the optimal parallel pattern based on system state and task characteristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternKind {
    FanOutFanIn,
    Pipeline,
    WorkStealing,
    DynamicBatch,
}

/// Context for pattern selection.
#[derive(Debug, Clone)]
pub struct PatternContext {
    /// Number of items to process.
    pub item_count: usize,
    /// Per-item estimated time (microseconds).
    pub per_item_us: u64,
    /// Available concurrency (from PID controller).
    pub concurrency: usize,
    /// Task priority.
    pub priority: Priority,
    /// Whether items have dependencies between them.
    pub has_dependencies: TriState,
}

/// Select the optimal pattern given the context.
pub fn select_pattern(ctx: &PatternContext) -> PatternKind {
    // Dependencies => pipeline (items must be ordered).
    if ctx.has_dependencies.is_true() {
        return PatternKind::Pipeline;
    }

    // Few items => dynamic batch (overhead of parallelism not worth it).
    if ctx.item_count < ctx.concurrency * 2 {
        return PatternKind::DynamicBatch;
    }

    // Large item count with uneven work distribution => work-stealing.
    // Heuristic: if per_item_us is high (> 1ms), assume uneven work.
    if ctx.per_item_us > 1000 && ctx.concurrency >= 4 {
        return PatternKind::WorkStealing;
    }

    // Default: fan-out/fan-in.
    PatternKind::FanOutFanIn
}

/// Compute a content hash of a parallel plan for audit trail.
pub fn plan_hash(kind: PatternKind, item_count: usize, concurrency: usize) -> [u8; 32] {
    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(&(kind as u32).to_le_bytes());
    buf.extend_from_slice(&(item_count as u64).to_le_bytes());
    buf.extend_from_slice(&(concurrency as u64).to_le_bytes());
    sha3_256(&buf)
}

// ─── Cross-Patterns ──────────────────────────────────────────────────────

/// Cross-pattern: Fan-Out into parallel Pipelines.
///
/// Each worker runs its own pipeline stage chain. Used when work has both
/// parallelism (independent chunks) AND sequential dependencies within each chunk.
/// E.g. parsing multiple files where each file needs tokenize -> transform -> serialize.
#[derive(Debug, Clone)]
pub struct FanOutPipeline {
    /// Per-worker pipeline stages (latency in microseconds).
    pub stages: Vec<u64>,
    /// Number of parallel workers.
    pub worker_count: usize,
    /// Total input items split across workers.
    pub input_count: usize,
    /// Priority.
    pub priority: Priority,
    /// Estimated total time (max across workers).
    pub total_estimated_us: u64,
    /// Per-worker throughput.
    pub per_worker_throughput: f64,
}

impl FanOutPipeline {
    pub fn plan(stages: Vec<u64>, worker_count: usize, input_count: usize, priority: Priority) -> Self {
        if stages.is_empty() || worker_count == 0 || input_count == 0 {
            return FanOutPipeline {
                stages,
                worker_count,
                input_count,
                priority,
                total_estimated_us: 0,
                per_worker_throughput: 0.0,
            };
        }
        // Each worker processes input_count / worker_count items through the pipeline.
        let items_per_worker = (input_count + worker_count - 1) / worker_count;
        let stage_total: u64 = stages.iter().sum();
        let bottleneck = stages.iter().copied().max().unwrap_or(0);
        let total_estimated_us = items_per_worker as u64 * stage_total;
        let per_worker_throughput = if bottleneck > 0 {
            1_000_000.0 / bottleneck as f64
        } else {
            0.0
        };

        FanOutPipeline {
            stages,
            worker_count,
            input_count,
            priority,
            total_estimated_us,
            per_worker_throughput,
        }
    }

    /// Worker assignments: (worker_id, start_item, end_item).
    pub fn assignments(&self) -> Vec<(usize, usize, usize)> {
        if self.worker_count == 0 || self.input_count == 0 || self.stages.is_empty() {
            return Vec::new();
        }
        let per = (self.input_count + self.worker_count - 1) / self.worker_count;
        (0..self.worker_count)
            .map(|w| {
                let start = w * per;
                let end = ((w + 1) * per).min(self.input_count);
                (w, start, end)
            })
            .filter(|(_, s, e)| s < e)
            .collect()
    }
}

/// Cross-pattern: Pipeline with Work-Stealing between stages.
///
/// Multiple pipeline stages; if one stage is a bottleneck, idle workers from
/// faster stages can steal from the bottleneck's queue.
#[derive(Debug, Clone)]
pub struct PipelineWithStealing {
    /// Per-stage latency (microseconds).
    pub stage_latencies: Vec<u64>,
    /// Workers per stage.
    pub workers_per_stage: usize,
    /// Queue depth per stage.
    pub queue_depths: Vec<usize>,
    /// Steal threshold (imbalance before stealing triggers).
    pub steal_threshold: usize,
    /// Estimated throughput (items/sec).
    pub throughput: f64,
    /// Whether stealing should happen.
    pub should_steal: TriState,
    /// Steal pair: (thief_stage, victim_stage).
    pub steal_pair: Option<(usize, usize)>,
}

impl PipelineWithStealing {
    pub fn plan(
        stage_latencies: Vec<u64>,
        workers_per_stage: usize,
        queue_depths: Vec<usize>,
        steal_threshold: usize,
    ) -> Self {
        let should_steal = if queue_depths.len() >= 2 {
            let max_q = *queue_depths.iter().max().unwrap_or(&0);
            let min_q = *queue_depths.iter().min().unwrap_or(&0);
            TriState::from_bool(max_q.saturating_sub(min_q) > steal_threshold)
        } else {
            TriState::False
        };

        let steal_pair = if should_steal.is_true() {
            let min_idx = queue_depths.iter().enumerate()
                .min_by_key(|(_, &q)| q).map(|(i, _)| i);
            let max_idx = queue_depths.iter().enumerate()
                .max_by_key(|(_, &q)| q).map(|(i, _)| i);
            match (min_idx, max_idx) {
                (Some(min), Some(max)) if min != max && queue_depths[max] > queue_depths[min] => {
                    Some((min, max))
                }
                _ => None,
            }
        } else {
            None
        };

        // Throughput = 1 / bottleneck_stage_latency.
        let bottleneck = stage_latencies.iter().copied().max().unwrap_or(1);
        let throughput = if bottleneck > 0 { 1_000_000.0 / bottleneck as f64 } else { 0.0 };

        PipelineWithStealing {
            stage_latencies,
            workers_per_stage,
            queue_depths,
            steal_threshold,
            throughput,
            should_steal,
            steal_pair,
        }
    }
}

/// Cross-pattern: Batched Fan-Out with dynamic per-worker batch sizing.
///
/// Fan-out where each worker gets a different batch size based on its measured
/// throughput. Faster workers get more items. PID controls total batch size.
#[derive(Debug, Clone)]
pub struct BatchedFanOut {
    /// Worker batch sizes (indexed by worker_id).
    pub worker_batches: Vec<usize>,
    /// Per-worker estimated latency (microseconds).
    pub worker_latency_us: Vec<u64>,
    /// Total items distributed.
    pub total_items: usize,
    /// PID output used for total sizing.
    pub pid_output: f64,
    /// Priority.
    pub priority: Priority,
    /// Estimated total time (max across workers).
    pub total_estimated_us: u64,
}

impl BatchedFanOut {
    pub fn plan(
        total_items: usize,
        worker_latencies_us: &[u64],
        pid_output: f64,
        priority: Priority,
    ) -> Self {
        let worker_count = worker_latencies_us.len().max(1);
        if total_items == 0 {
            return BatchedFanOut {
                worker_batches: vec![0; worker_count],
                worker_latency_us: worker_latencies_us.to_vec(),
                total_items: 0,
                pid_output,
                priority,
                total_estimated_us: 0,
            };
        }

        // Inverse-latency weighted distribution: faster workers get more items.
        let total_inv_latency: f64 = worker_latencies_us
            .iter()
            .map(|&lat| if lat > 0 { 1.0 / lat as f64 } else { 1.0 })
            .sum();

        let mut worker_batches = Vec::with_capacity(worker_count);
        let mut assigned = 0usize;
        for (i, &lat) in worker_latencies_us.iter().enumerate() {
            let weight = if lat > 0 { 1.0 / lat as f64 } else { 1.0 } / total_inv_latency;
            let batch = if i == worker_count - 1 {
                total_items.saturating_sub(assigned)
            } else {
                ((total_items as f64 * weight).round() as usize)
                    .min(total_items.saturating_sub(assigned))
            };
            assigned += batch;
            worker_batches.push(batch);
        }

        // Estimated total = max per-worker time.
        let total_estimated_us = worker_batches
            .iter()
            .zip(worker_latencies_us.iter())
            .map(|(&batch, &lat)| batch as u64 * lat)
            .max()
            .unwrap_or(0);

        BatchedFanOut {
            worker_batches,
            worker_latency_us: worker_latencies_us.to_vec(),
            total_items,
            pid_output,
            priority,
            total_estimated_us,
        }
    }

    /// Fairness ratio: min_batch / max_batch (1.0 = perfectly fair).
    pub fn fairness(&self) -> f64 {
        let max_b = self.worker_batches.iter().copied().max().unwrap_or(1);
        let min_b = self.worker_batches.iter().copied().min().unwrap_or(0);
        if max_b == 0 { 1.0 } else { min_b as f64 / max_b as f64 }
    }
}

/// Cross-pattern: Adaptive — switches between patterns at runtime based on load.
///
/// Monitors throughput and latency; if throughput drops below threshold,
/// switches from one pattern to another. The switch is smooth: completes
/// current batch in old pattern, starts next batch in new pattern.
#[derive(Debug, Clone)]
pub struct AdaptivePattern {
    /// Current active pattern.
    pub current: PatternKind,
    /// Pattern to switch to if degrading.
    pub fallback: PatternKind,
    /// Throughput threshold (actions/sec) below which switch triggers.
    pub switch_threshold: f64,
    /// Current measured throughput.
    pub current_throughput: f64,
    /// Whether a switch is recommended.
    pub should_switch: TriState,
    /// Number of switches since creation.
    pub switch_count: u64,
}

impl AdaptivePattern {
    pub fn new(initial: PatternKind, fallback: PatternKind, switch_threshold: f64) -> Self {
        AdaptivePattern {
            current: initial,
            fallback,
            switch_threshold,
            current_throughput: 0.0,
            should_switch: TriState::False,
            switch_count: 0,
        }
    }

    /// Update with latest throughput measurement. Returns whether a switch happened.
    pub fn update(&mut self, throughput: f64) -> TriState {
        self.current_throughput = throughput;
        self.should_switch = TriState::from_bool(throughput < self.switch_threshold && throughput > 0.0);
        if self.should_switch.is_true() {
            core::mem::swap(&mut self.current, &mut self.fallback);
            self.switch_count += 1;
            self.should_switch = TriState::False;
            TriState::True
        } else {
            TriState::False
        }
    }

    /// ASCII dashboard for this adaptive pattern.
    pub fn ascii_dashboard(&self) -> String {
        format!(
            "AdaptivePattern: {:?} (fallback={:?}) throughput={:.1}/s threshold={:.1}/s switches={}",
            self.current, self.fallback, self.current_throughput, self.switch_threshold, self.switch_count
        )
    }
}

// ─── Cross-Pattern Kind ──────────────────────────────────────────────────

/// All pattern kinds including cross-patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternKindExtended {
    FanOutFanIn,
    Pipeline,
    WorkStealing,
    DynamicBatch,
    // Cross-patterns:
    FanOutPipeline,
    PipelineWithStealing,
    BatchedFanOut,
    Adaptive,
}

/// Extended pattern context (includes per-worker latency data for cross-patterns).
#[derive(Debug, Clone)]
pub struct ExtendedPatternContext {
    pub base: PatternContext,
    /// Per-worker latency observations (for BatchedFanOut / PipelineWithStealing).
    pub worker_latencies_us: Vec<u64>,
    /// Per-stage queue depths (for PipelineWithStealing).
    pub stage_queue_depths: Vec<usize>,
    /// Per-stage latencies (for FanOutPipeline / PipelineWithStealing).
    pub stage_latencies_us: Vec<u64>,
    /// Current throughput (for AdaptivePattern).
    pub current_throughput: f64,
    /// Switch threshold for adaptive pattern.
    pub adaptive_threshold: f64,
}

/// Select the optimal extended pattern given the context.
pub fn select_pattern_extended(ctx: &ExtendedPatternContext) -> PatternKindExtended {
    // Dependencies => pipeline family.
    if ctx.base.has_dependencies.is_true() {
        if ctx.worker_latencies_us.len() >= 2 && ctx.stage_queue_depths.len() >= 2 {
            return PatternKindExtended::PipelineWithStealing;
        }
        return PatternKindExtended::Pipeline;
    }

    // Few items => dynamic batch.
    if ctx.base.item_count < ctx.base.concurrency * 2 {
        return PatternKindExtended::DynamicBatch;
    }

    // Has stage latencies and worker latencies => fan-out-pipeline.
    if !ctx.stage_latencies_us.is_empty() && ctx.worker_latencies_us.len() >= 2 {
        return PatternKindExtended::FanOutPipeline;
    }

    // Has worker latencies with uneven throughput => batched fan-out.
    if ctx.worker_latencies_us.len() >= 2 {
        let max_lat = ctx.worker_latencies_us.iter().copied().max().unwrap_or(1);
        let min_lat = ctx.worker_latencies_us.iter().copied().min().unwrap_or(1);
        if max_lat > 0 && min_lat > 0 && (max_lat as f64 / min_lat as f64) > 2.0 {
            return PatternKindExtended::BatchedFanOut;
        }
    }

    // High variance + adaptive threshold set => adaptive.
    if ctx.adaptive_threshold > 0.0 && ctx.current_throughput > 0.0 {
        return PatternKindExtended::Adaptive;
    }

    // Large uneven work => work-stealing.
    if ctx.base.per_item_us > 1000 && ctx.base.concurrency >= 4 {
        return PatternKindExtended::WorkStealing;
    }

    PatternKindExtended::FanOutFanIn
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fan_out_assignments_contiguous() {
        let plan = FanOutPlan::plan(10, 3, 100, Priority::Normal);
        let assignments = plan.assignments();
        assert_eq!(assignments.len(), 3);
        // 10 items / 3 workers => 4+4+2.
        assert_eq!(assignments[0], (0, 0, 4));
        assert_eq!(assignments[1], (1, 4, 8));
        assert_eq!(assignments[2], (2, 8, 10));
    }

    #[test]
    fn fan_out_worker_count_capped() {
        let plan = FanOutPlan::plan(3, 8, 100, Priority::Normal);
        assert_eq!(plan.worker_count, 3); // capped at input_count
    }

    #[test]
    fn fan_out_empty_input() {
        let plan = FanOutPlan::plan(0, 4, 100, Priority::Normal);
        assert!(plan.assignments().is_empty());
        assert_eq!(plan.total_estimated_us, 0);
    }

    #[test]
    fn pipeline_throughput_limited_by_bottleneck() {
        let plan = PipelinePlan::plan(vec![100, 500, 200], 4);
        // Bottleneck is 500us => throughput = 2000 items/sec.
        assert!((plan.throughput_per_sec() - 2000.0).abs() < 1.0);
        assert_eq!(plan.stage_count, 3);
        assert!(plan.in_flight > 0);
    }

    #[test]
    fn pipeline_empty() {
        let plan = PipelinePlan::plan(vec![], 4);
        assert_eq!(plan.stage_count, 0);
        assert_eq!(plan.throughput_per_sec(), 0.0);
    }

    #[test]
    fn work_stealing_triggers_on_imbalance() {
        let plan = WorkStealingPlan::plan(vec![10, 0, 0], 100, 3);
        assert!(plan.should_steal().is_true());
    }

    #[test]
    fn work_stealing_no_trigger_on_balanced() {
        let plan = WorkStealingPlan::plan(vec![5, 5, 5], 100, 3);
        assert!(plan.should_steal().is_false());
    }

    #[test]
    fn work_stealing_pair_picks_empty_and_full() {
        let plan = WorkStealingPlan::plan(vec![0, 10, 3], 100, 1);
        let (thief, victim) = plan.steal_pair().unwrap();
        assert_eq!(thief, 0); // queue 0 is empty
        assert_eq!(victim, 1); // queue 1 is fullest
    }

    #[test]
    fn work_stealing_equal_queues_no_pair() {
        let plan = WorkStealingPlan::plan(vec![5, 5], 100, 1);
        assert!(plan.steal_pair().is_none());
    }

    #[test]
    fn dynamic_batch_scales_with_pid() {
        let small = DynamicBatchPlan::plan(100, 1, 100);
        let large = DynamicBatchPlan::plan(100, 8, 100);
        assert!(large.batch_size > small.batch_size);
        assert!(large.batch_count <= small.batch_count);
    }

    #[test]
    fn dynamic_batch_total_items_conserved() {
        let plan = DynamicBatchPlan::plan(100, 4, 100);
        assert_eq!(plan.batch_count, (100 + plan.batch_size - 1) / plan.batch_size);
    }

    #[test]
    fn select_pattern_depends_on_pipeline() {
        let ctx = PatternContext {
            item_count: 100,
            per_item_us: 100,
            concurrency: 4,
            priority: Priority::Normal,
            has_dependencies: TriState::True,
        };
        assert_eq!(select_pattern(&ctx), PatternKind::Pipeline);
    }

    #[test]
    fn select_pattern_few_items_batch() {
        let ctx = PatternContext {
            item_count: 3,
            per_item_us: 100,
            concurrency: 4,
            priority: Priority::Normal,
            has_dependencies: TriState::False,
        };
        assert_eq!(select_pattern(&ctx), PatternKind::DynamicBatch);
    }

    #[test]
    fn select_pattern_large_uneven_work_stealing() {
        let ctx = PatternContext {
            item_count: 1000,
            per_item_us: 5000, // 5ms per item = uneven
            concurrency: 8,
            priority: Priority::Normal,
            has_dependencies: TriState::False,
        };
        assert_eq!(select_pattern(&ctx), PatternKind::WorkStealing);
    }

    #[test]
    fn select_pattern_default_fan_out() {
        let ctx = PatternContext {
            item_count: 100,
            per_item_us: 50,
            concurrency: 4,
            priority: Priority::Normal,
            has_dependencies: TriState::False,
        };
        assert_eq!(select_pattern(&ctx), PatternKind::FanOutFanIn);
    }

    #[test]
    fn plan_hash_deterministic() {
        let a = plan_hash(PatternKind::FanOutFanIn, 100, 4);
        let b = plan_hash(PatternKind::FanOutFanIn, 100, 4);
        assert_eq!(a, b);
    }

    #[test]
    fn plan_hash_distinct() {
        let a = plan_hash(PatternKind::FanOutFanIn, 100, 4);
        let b = plan_hash(PatternKind::Pipeline, 100, 4);
        assert_ne!(a, b);
    }

    // ── Cross-pattern tests ────────────────────────────────────────────

    #[test]
    fn fan_out_pipeline_assignments() {
        let plan = FanOutPipeline::plan(vec![100, 200], 3, 12, Priority::Normal);
        let a = plan.assignments();
        assert_eq!(a.len(), 3);
        // 12 items / 3 workers = 4 each.
        assert_eq!(a[0], (0, 0, 4));
        assert_eq!(a[1], (1, 4, 8));
        assert_eq!(a[2], (2, 8, 12));
    }

    #[test]
    fn fan_out_pipeline_empty() {
        let plan = FanOutPipeline::plan(vec![], 3, 12, Priority::Normal);
        assert_eq!(plan.total_estimated_us, 0);
        assert!(plan.assignments().is_empty());
    }

    #[test]
    fn fan_out_pipeline_throughput() {
        let plan = FanOutPipeline::plan(vec![100, 200], 2, 10, Priority::Normal);
        // Bottleneck stage = 200us => throughput = 5000/s per worker.
        assert!((plan.per_worker_throughput - 5000.0).abs() < 1.0);
    }

    #[test]
    fn pipeline_stealing_triggers_on_imbalance() {
        let plan = PipelineWithStealing::plan(vec![100, 500, 200], 2, vec![0, 10, 2], 3);
        assert!(plan.should_steal.is_true());
        assert!(plan.steal_pair.is_some());
    }

    #[test]
    fn pipeline_stealing_no_trigger_on_balanced() {
        let plan = PipelineWithStealing::plan(vec![100, 200], 2, vec![5, 5], 3);
        assert!(plan.should_steal.is_false());
    }

    #[test]
    fn pipeline_stealing_throughput() {
        let plan = PipelineWithStealing::plan(vec![100, 500, 200], 2, vec![0, 5, 0], 3);
        // Bottleneck = 500us => throughput = 2000/s.
        assert!((plan.throughput - 2000.0).abs() < 1.0);
    }

    #[test]
    fn batched_fan_out_inverse_latency_weighted() {
        // Worker 0 is fast (100us), worker 1 is slow (500us).
        let plan = BatchedFanOut::plan(100, &[100, 500], 4.0, Priority::Normal);
        // Worker 0 should get more items (inverse latency weighting).
        assert!(plan.worker_batches[0] > plan.worker_batches[1]);
    }

    #[test]
    fn batched_fan_out_fairness() {
        // Equal latency => equal batches.
        let plan = BatchedFanOut::plan(100, &[200, 200], 4.0, Priority::Normal);
        assert!((plan.fairness() - 1.0).abs() < 0.01);
    }

    #[test]
    fn batched_fan_out_empty() {
        let plan = BatchedFanOut::plan(0, &[100, 200], 4.0, Priority::Normal);
        assert_eq!(plan.total_items, 0);
        assert!(plan.worker_batches.iter().all(|&b| b == 0));
    }

    #[test]
    fn batched_fan_out_total_conserves() {
        let plan = BatchedFanOut::plan(100, &[100, 200, 300], 4.0, Priority::Normal);
        let total: usize = plan.worker_batches.iter().sum();
        assert_eq!(total, 100);
    }

    #[test]
    fn adaptive_pattern_switches_on_low_throughput() {
        let mut ap = AdaptivePattern::new(PatternKind::FanOutFanIn, PatternKind::WorkStealing, 100.0);
        assert!(ap.update(200.0).is_false()); // above threshold
        assert_eq!(ap.current, PatternKind::FanOutFanIn);
        assert!(ap.update(50.0).is_true()); // below threshold => switch
        assert_eq!(ap.current, PatternKind::WorkStealing);
        assert_eq!(ap.switch_count, 1);
    }

    #[test]
    fn adaptive_pattern_ascii_dashboard() {
        let ap = AdaptivePattern::new(PatternKind::FanOutFanIn, PatternKind::WorkStealing, 100.0);
        let d = ap.ascii_dashboard();
        assert!(d.contains("AdaptivePattern"));
        assert!(d.contains("FanOutFanIn"));
    }

    #[test]
    fn select_pattern_extended_dependencies() {
        let ctx = ExtendedPatternContext {
            base: PatternContext {
                item_count: 100, per_item_us: 100, concurrency: 4,
                priority: Priority::Normal, has_dependencies: TriState::True,
            },
            worker_latencies_us: vec![100, 200],
            stage_queue_depths: vec![5, 10],
            stage_latencies_us: vec![],
            current_throughput: 0.0,
            adaptive_threshold: 0.0,
        };
        assert_eq!(select_pattern_extended(&ctx), PatternKindExtended::PipelineWithStealing);
    }

    #[test]
    fn select_pattern_extended_fan_out_pipeline() {
        let ctx = ExtendedPatternContext {
            base: PatternContext {
                item_count: 100, per_item_us: 100, concurrency: 4,
                priority: Priority::Normal, has_dependencies: TriState::False,
            },
            worker_latencies_us: vec![100, 200],
            stage_queue_depths: vec![],
            stage_latencies_us: vec![100, 200],
            current_throughput: 0.0,
            adaptive_threshold: 0.0,
        };
        assert_eq!(select_pattern_extended(&ctx), PatternKindExtended::FanOutPipeline);
    }

    #[test]
    fn select_pattern_extended_batched_fan_out() {
        let ctx = ExtendedPatternContext {
            base: PatternContext {
                item_count: 100, per_item_us: 100, concurrency: 4,
                priority: Priority::Normal, has_dependencies: TriState::False,
            },
            worker_latencies_us: vec![100, 500], // 5x difference
            stage_queue_depths: vec![],
            stage_latencies_us: vec![],
            current_throughput: 0.0,
            adaptive_threshold: 0.0,
        };
        assert_eq!(select_pattern_extended(&ctx), PatternKindExtended::BatchedFanOut);
    }

    #[test]
    fn select_pattern_extended_adaptive() {
        let ctx = ExtendedPatternContext {
            base: PatternContext {
                item_count: 100, per_item_us: 50, concurrency: 4,
                priority: Priority::Normal, has_dependencies: TriState::False,
            },
            worker_latencies_us: vec![],
            stage_queue_depths: vec![],
            stage_latencies_us: vec![],
            current_throughput: 50.0,
            adaptive_threshold: 100.0,
        };
        assert_eq!(select_pattern_extended(&ctx), PatternKindExtended::Adaptive);
    }
}
