//! `kernel::dynamic_actions` — PID-controlled action batch execution across parallel cores.
//!
//! Groups pending actions into batches, distributes across N workers (cores),
//! PID adjusts batch size based on measured latency vs predicted latency.
//! Predictions cached and invalidated on new observations.
//! ASCII dashboard for live diagnostics.
//!
//! # Architecture
//! ```text
//! DynamicActionBatcher
//! +-- BatchConfig (min/max size, latency target, worker count)
//! +-- PidController (setpoint=predicted, measurement=actual)
//! +-- WorkerPool (per-core load, work-stealing, EMA tracking)
//! +-- ActionCache (cached plans, invalidated on new data)
//! +-- compute_batches() -> ExecutionPlan
//! +-- ascii_dashboard() -> live diagnostics
//! ```

use crate::orchestrator::{ActionCategory, PidController, Priority, ScheduledTask};
use crate::TriState;

pub const MIN_BATCH_SIZE: usize = 1;
pub const MAX_BATCH_SIZE: usize = 32;
pub const WORKER_LATENCY_ALPHA: f64 = 0.3;
pub const THROUGHPUT_ALPHA: f64 = 0.2;

// ─── Batch Configuration ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub min_batch: usize,
    pub max_batch: usize,
    pub worker_count: usize,
    pub target_batch_latency_us: u64,
    pub max_in_flight: usize,
    pub min_recompute_interval_us: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        BatchConfig {
            min_batch: MIN_BATCH_SIZE,
            max_batch: MAX_BATCH_SIZE,
            worker_count: 4,
            target_batch_latency_us: 50_000,
            max_in_flight: 64,
            min_recompute_interval_us: 10_000,
        }
    }
}

// ─── Worker State ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WorkerState {
    pub id: usize,
    pub avg_latency_us: f64,
    pub throughput: f64,
    pub in_flight: usize,
    pub batches_completed: u64,
    pub actions_completed: u64,
    pub total_work_us: u64,
    pub idle: TriState,
}

impl WorkerState {
    pub fn new(id: usize) -> Self {
        WorkerState { id, avg_latency_us: 0.0, throughput: 0.0, in_flight: 0, batches_completed: 0, actions_completed: 0, total_work_us: 0, idle: TriState::True }
    }

    pub fn record_batch_done(&mut self, actions: usize, latency_us: u64) {
        self.in_flight = self.in_flight.saturating_sub(actions);
        self.batches_completed += 1;
        self.actions_completed += actions as u64;
        self.total_work_us += latency_us;
        self.idle = TriState::from_bool(self.in_flight == 0);
        if latency_us > 0 {
            let per_action = latency_us as f64 / actions.max(1) as f64;
            self.avg_latency_us = WORKER_LATENCY_ALPHA * per_action + (1.0 - WORKER_LATENCY_ALPHA) * self.avg_latency_us;
            let tput = (actions as f64 / latency_us as f64) * 1_000_000.0;
            self.throughput = THROUGHPUT_ALPHA * tput + (1.0 - THROUGHPUT_ALPHA) * self.throughput;
        }
    }

    pub fn assign(&mut self, count: usize) {
        self.in_flight += count;
        self.idle = TriState::False;
    }

    pub fn load_factor(&self, max_per_worker: usize) -> f64 {
        self.in_flight as f64 / max_per_worker.max(1) as f64
    }
}

// ─── Worker Pool ─────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct WorkerPool {
    workers: Vec<WorkerState>,
    max_per_worker: usize,
}

impl WorkerPool {
    pub fn new(worker_count: usize, max_per_worker: usize) -> Self {
        WorkerPool {
            workers: (0..worker_count).map(WorkerState::new).collect(),
            max_per_worker,
        }
    }

    pub fn least_loaded(&self) -> Option<usize> {
        self.workers.iter()
            .filter(|w| w.in_flight < self.max_per_worker)
            .min_by_key(|w| (w.in_flight * 1000) as u64)
            .map(|w| w.id)
    }

    pub fn total_in_flight(&self) -> usize {
        self.workers.iter().map(|w| w.in_flight).sum()
    }

    pub fn total_capacity(&self, max_in_flight: usize) -> usize {
        max_in_flight.saturating_sub(self.total_in_flight())
    }

    pub fn steal_pair(&self) -> Option<(usize, usize)> {
        let idle = self.workers.iter().find(|w| w.idle == TriState::True)?;
        let busiest = self.workers.iter()
            .filter(|w| w.id != idle.id && w.in_flight > 1)
            .max_by_key(|w| w.in_flight)?;
        Some((idle.id, busiest.id))
    }

    pub fn avg_latency_us(&self) -> f64 {
        let active: Vec<&WorkerState> = self.workers.iter().filter(|w| w.batches_completed > 0).collect();
        if active.is_empty() { 0.0 } else { active.iter().map(|w| w.avg_latency_us).sum::<f64>() / active.len() as f64 }
    }

    pub fn avg_throughput(&self) -> f64 {
        let active: Vec<&WorkerState> = self.workers.iter().filter(|w| w.batches_completed > 0).collect();
        if active.is_empty() { 0.0 } else { active.iter().map(|w| w.throughput).sum::<f64>() / active.len() as f64 }
    }

    pub fn get(&self, id: usize) -> Option<&WorkerState> { self.workers.get(id) }
    pub fn get_mut(&mut self, id: usize) -> Option<&mut WorkerState> { self.workers.get_mut(id) }
    pub fn len(&self) -> usize { self.workers.len() }
    pub fn is_empty(&self) -> bool { self.workers.is_empty() }
    pub fn workers(&self) -> &[WorkerState] { &self.workers }
}

// ─── Action Batch ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ActionBatch {
    pub batch_id: u64,
    pub worker_id: usize,
    pub tasks: Vec<ScheduledTask>,
    pub action_count: usize,
    pub predicted_us: u64,
    pub pid_batch_size: f64,
    pub batch_hash: [u8; 32],
}

// ─── Execution Plan ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub batches: Vec<ActionBatch>,
    pub assignments: Vec<(usize, Vec<u64>)>,
    pub pid_batch_size: f64,
    pub total_actions: usize,
    pub predicted_total_us: u64,
    pub predicted_parallel_us: u64,
    pub idle_workers: usize,
    pub computed_us: u64,
    /// Whether this plan was served from cache.
    pub from_cache: TriState,
}

// ─── Prediction Cache ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ActionCache {
    pub cached_plan_hash: [u8; 32],
    pub cached_batch_size: usize,
    pub cached_total_actions: usize,
    pub cached_predicted_us: u64,
    pub cache_computed_us: u64,
    pub stale: TriState,
    pub hits: u64,
    pub misses: u64,
}

impl ActionCache {
    pub fn new() -> Self {
        ActionCache { cached_plan_hash: [0u8; 32], cached_batch_size: 0, cached_total_actions: 0, cached_predicted_us: 0, cache_computed_us: 0, stale: TriState::Unknown, hits: 0, misses: 0 }
    }

    pub fn is_valid(&self, total_actions: usize, _avg_latency_us: f64, now_us: u64, ttl_us: u64) -> bool {
        self.stale == TriState::False
            && self.cached_total_actions == total_actions
            && now_us.saturating_sub(self.cache_computed_us) < ttl_us
    }

    pub fn serve(&mut self, now_us: u64) -> ExecutionPlan {
        self.hits += 1;
        ExecutionPlan {
            batches: Vec::new(),
            assignments: Vec::new(),
            pid_batch_size: 0.0,
            total_actions: self.cached_total_actions,
            predicted_total_us: self.cached_predicted_us,
            predicted_parallel_us: 0,
            idle_workers: 0,
            computed_us: now_us,
            from_cache: TriState::True,
        }
    }

    pub fn invalidate(&mut self) { self.stale = TriState::True; self.misses += 1; }

    pub fn update(&mut self, plan: &ExecutionPlan, now_us: u64) {
        self.cached_plan_hash = plan.batches.first().map(|b| b.batch_hash).unwrap_or([0u8; 32]);
        self.cached_batch_size = plan.pid_batch_size as usize;
        self.cached_total_actions = plan.total_actions;
        self.cached_predicted_us = plan.predicted_total_us;
        self.cache_computed_us = now_us;
        self.stale = TriState::False;
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }
}

// ─── Dynamic Action Batcher ──────────────────────────────────────────────

#[derive(Debug)]
pub struct DynamicActionBatcher {
    config: BatchConfig,
    pid: PidController,
    pool: WorkerPool,
    category_latencies: std::collections::HashMap<ActionCategory, (f64, f64)>,
    alpha: f64,
    fractional_accum: f64,
    next_batch_id: u64,
    last_recompute_us: u64,
    cache: ActionCache,
    cache_ttl_us: u64,
}

impl DynamicActionBatcher {
    pub fn new(config: BatchConfig) -> Self {
        let max_per_worker = config.max_batch;
        DynamicActionBatcher {
            pid: PidController::new_min_max(config.min_batch, config.max_batch),
            pool: WorkerPool::new(config.worker_count, max_per_worker),
            config,
            category_latencies: std::collections::HashMap::new(),
            alpha: 0.3,
            fractional_accum: 0.0,
            next_batch_id: 1,
            last_recompute_us: 0,
            cache: ActionCache::new(),
            cache_ttl_us: 50_000,
        }
    }

    pub fn with_pid_tuning(config: BatchConfig, kp: f64, ki: f64, kd: f64) -> Self {
        let (min_batch, max_batch) = (config.min_batch, config.max_batch);
        let mut b = Self::new(config);
        b.pid = PidController::new(kp, ki, kd, min_batch as f64, max_batch as f64);
        b
    }

    pub fn with_cache_ttl(config: BatchConfig, ttl_us: u64) -> Self {
        let mut b = Self::new(config);
        b.cache_ttl_us = ttl_us;
        b
    }

    /// Compute execution plan (with cache).
    pub fn compute_plan(&mut self, tasks: &[ScheduledTask], now_us: u64) -> ExecutionPlan {
        // Cache check.
        if self.cache.is_valid(tasks.len(), self.pool.avg_latency_us(), now_us, self.cache_ttl_us) {
            return self.cache.serve(now_us);
        }
        self.cache.invalidate();

        // Rate limit.
        if now_us.saturating_sub(self.last_recompute_us) < self.config.min_recompute_interval_us && !tasks.is_empty() {
            return ExecutionPlan {
                batches: Vec::new(), assignments: Vec::new(),                 pid_batch_size: self.pid.output(),
                total_actions: 0, predicted_total_us: 0, predicted_parallel_us: 0,
                idle_workers: self.pool.len(), computed_us: now_us, from_cache: TriState::False,
            };
        }
        self.last_recompute_us = now_us;

        if tasks.is_empty() {
            return ExecutionPlan {
                batches: Vec::new(),
                assignments: self.pool.workers().iter().map(|w| (w.id, Vec::new())).collect(),
                pid_batch_size: self.pid.output(), total_actions: 0, predicted_total_us: 0,
                predicted_parallel_us: 0, idle_workers: self.pool.len(), computed_us: now_us, from_cache: TriState::False,
            };
        }

        // PID feedback.
        let actual_latency = self.pool.avg_latency_us();
        let target_latency = self.config.target_batch_latency_us as f64;
        let pid_output = self.pid.update(target_latency, actual_latency);

        self.fractional_accum += pid_output;
        let batch_size = self.fractional_accum as usize;
        self.fractional_accum -= batch_size as f64;
        let batch_size = batch_size.max(self.config.min_batch).min(self.config.max_batch);

        // Split + distribute.
        let mut batches = Vec::new();
        let mut assignments: Vec<(usize, Vec<u64>)> = self.pool.workers().iter().map(|w| (w.id, Vec::new())).collect();

        for chunk in tasks.chunks(batch_size) {
            let worker_id = self.pool.least_loaded().unwrap_or(0);
            let predicted_us = self.predict_batch_latency(chunk);
            let mut hash_input = Vec::with_capacity(10);
            hash_input.extend_from_slice(&self.next_batch_id.to_le_bytes());
            hash_input.push(worker_id as u8);
            hash_input.push(batch_size.to_le_bytes()[0]);
            let batch_hash = crate::event_log::sha3_256(&hash_input);

            let batch = ActionBatch {
                batch_id: self.next_batch_id, worker_id, tasks: chunk.to_vec(),
                action_count: chunk.len(), predicted_us, pid_batch_size: pid_output, batch_hash,
            };

            if let Some(w) = self.pool.get_mut(worker_id) { w.assign(chunk.len()); }
            if let Some((_, ref mut ids)) = assignments.iter_mut().find(|(id, _)| *id == worker_id) {
                ids.push(batch.batch_id);
            }
            self.next_batch_id += 1;
            batches.push(batch);
        }

        let total_actions: usize = batches.iter().map(|b| b.action_count).sum();
        let predicted_total_us: u64 = batches.iter().map(|b| b.predicted_us).sum();
        let idle_workers = self.pool.workers().iter().filter(|w| w.idle == TriState::True).count();

        let mut worker_totals: Vec<u64> = vec![0; self.pool.len()];
        for b in &batches { worker_totals[b.worker_id] += b.predicted_us; }
        let predicted_parallel_us = worker_totals.into_iter().max().unwrap_or(0);

        let plan = ExecutionPlan {
            batches, assignments, pid_batch_size: pid_output, total_actions,
            predicted_total_us,             predicted_parallel_us, idle_workers, computed_us: now_us, from_cache: TriState::False,
        };

        self.cache.update(&plan, now_us);
        plan
    }

    pub fn record_batch_done(&mut self, worker_id: usize, actions: usize, latency_us: u64) {
        if let Some(w) = self.pool.get_mut(worker_id) { w.record_batch_done(actions, latency_us); }
        self.cache.stale = TriState::True;
    }

    pub fn observe_category(&mut self, category: ActionCategory, duration_us: u64) {
        let alpha = self.alpha;
        let entry = self.category_latencies.entry(category).or_insert((duration_us as f64, 0.0));
        let old_val = entry.0;
        entry.0 = alpha * (duration_us as f64) + (1.0 - alpha) * old_val;
        let diff = (duration_us as f64) - old_val;
        entry.1 = alpha * diff * diff + (1.0 - alpha) * entry.1;
    }

    fn predict_batch_latency(&self, tasks: &[ScheduledTask]) -> u64 {
        tasks.iter().map(|t| {
            self.category_latencies.get(&t.category)
                .map(|(avg, var)| { let ci = 1.96 * var.sqrt(); (*avg + ci) as u64 })
                .unwrap_or(t.estimated_us)
        }).sum()
    }

    /// ASCII dashboard for live diagnostics.
    pub fn ascii_dashboard(&self) -> String {
        let p = &self.pool;
        let mut out = String::with_capacity(768);
        out.push_str("DynamicActionBatcher Dashboard\n");
        out.push_str(&format!("  Workers:     {} total, {} idle\n", p.len(), self.pool.workers().iter().filter(|w| w.idle == TriState::True).count()));
        out.push_str(&format!("  In-flight:   {}/{}\n", p.total_in_flight(), self.config.max_in_flight));
        out.push_str(&format!("  PID output:  {:.2} (target={}us)\n", self.pid.output(), self.config.target_batch_latency_us));
        out.push_str(&format!("  Batch size:  {:.0} (min={} max={})\n", self.pid.output(), self.config.min_batch, self.config.max_batch));
        out.push_str(&format!("  Avg latency: {:.0} us\n", p.avg_latency_us()));
        out.push_str(&format!("  Throughput:   {:.1} actions/sec\n", p.avg_throughput()));
        out.push_str(&format!("  Categories:  {}\n", self.category_latencies.len()));
        out.push_str(&format!("  Cache:       {:.0}% hit rate ({} hits / {} misses)\n",
            self.cache.hit_rate() * 100.0, self.cache.hits, self.cache.misses));
        // Per-worker breakdown.
        out.push_str("  Workers:\n");
        for w in p.workers() {
            let load = w.load_factor(self.config.max_batch);
            out.push_str(&format!("    W{}: in_flight={} completed={} latency={:.0}us tput={:.1}/s load={:.0}%{}\n",
                w.id, w.in_flight, w.actions_completed, w.avg_latency_us, w.throughput, load * 100.0,
                if w.idle == TriState::True { " IDLE" } else { "" }
            ));
        }
        out
    }

    pub fn pool(&self) -> &WorkerPool { &self.pool }
    pub fn pool_mut(&mut self) -> &mut WorkerPool { &mut self.pool }
    pub fn pid_output(&self) -> f64 { self.pid.output() }
    pub fn pid_recommended(&self) -> usize { self.pid.recommended() }
    pub fn config(&self) -> &BatchConfig { &self.config }
    pub fn cache(&self) -> &ActionCache { &self.cache }
    pub fn cache_mut(&mut self) -> &mut ActionCache { &mut self.cache }

    pub fn reset_pid(&mut self) {
        self.pid.reset();
        self.fractional_accum = 0.0;
        self.cache.invalidate();
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tasks(n: usize) -> Vec<ScheduledTask> {
        (0..n).map(|i| ScheduledTask {
            task_id: i as u64 + 1, category: ActionCategory::ToolRead, priority: Priority::Normal,
            estimated_us: 1000, created_us: 1000, budget_cost: 1.0, depends_on: Vec::new(),
        }).collect()
    }

    #[test]
    fn empty_tasks_empty_plan() {
        let mut b = DynamicActionBatcher::new(BatchConfig::default());
        let p = b.compute_plan(&[], 1000);
        assert_eq!(p.total_actions, 0);
    }

    #[test]
    fn tasks_split_into_batches() {
        let mut b = DynamicActionBatcher::new(BatchConfig {
            min_batch: 1, max_batch: 3, worker_count: 2, min_recompute_interval_us: 0, ..Default::default()
        });
        let p = b.compute_plan(&make_tasks(10), 1000);
        assert!(p.total_actions <= 10);
    }

    #[test]
    fn record_batch_done_updates_worker() {
        let mut b = DynamicActionBatcher::new(BatchConfig { worker_count: 2, ..Default::default() });
        // Assign some work first so the worker is not idle.
        if let Some(w) = b.pool_mut().get_mut(0) { w.assign(10); }
        assert!(b.pool().get(0).unwrap().idle == TriState::False);
        b.record_batch_done(0, 5, 10_000);
        let w = b.pool().get(0).unwrap();
        assert_eq!(w.actions_completed, 5);
        assert_eq!(w.in_flight, 5); // 10 assigned - 5 completed = 5 remaining
    }

    #[test]
    fn cache_hit_on_same_inputs() {
        let mut b = DynamicActionBatcher::new(BatchConfig {
            min_recompute_interval_us: 0, ..Default::default()
        });
        let _p1 = b.compute_plan(&make_tasks(8), 1000);
        let p2 = b.compute_plan(&make_tasks(8), 1050);
        assert!(p2.from_cache.is_true());
    }

    #[test]
    fn cache_invalidation_on_record() {
        let mut b = DynamicActionBatcher::new(BatchConfig { min_recompute_interval_us: 0, ..Default::default() });
        let _p1 = b.compute_plan(&make_tasks(8), 1000);
        assert!(b.cache().stale == TriState::False);
        b.record_batch_done(0, 5, 1000);
        assert!(b.cache().stale == TriState::True);
    }

    #[test]
    fn cache_hit_rate() {
        let mut b = DynamicActionBatcher::new(BatchConfig { min_recompute_interval_us: 0, ..Default::default() });
        let _p1 = b.compute_plan(&make_tasks(8), 1000);
        let _p2 = b.compute_plan(&make_tasks(8), 1050);
        let _p3 = b.compute_plan(&make_tasks(8), 1100);
        assert!(b.cache().hit_rate() > 0.0);
    }

    #[test]
    fn dashboard_contains_sections() {
        let b = DynamicActionBatcher::new(BatchConfig::default());
        let d = b.ascii_dashboard();
        assert!(d.contains("DynamicActionBatcher Dashboard"));
        assert!(d.contains("Workers:"));
        assert!(d.contains("PID output:"));
        assert!(d.contains("Cache:"));
        assert!(d.contains("Workers:\n    W"));
    }

    #[test]
    fn predict_category_latencies() {
        let mut b = DynamicActionBatcher::new(BatchConfig::default());
        for _ in 0..10 { b.observe_category(ActionCategory::Parse, 500); }
        let tasks = vec![ScheduledTask {
            task_id: 1, category: ActionCategory::Parse, priority: Priority::Normal,
            estimated_us: 1000, created_us: 1000, budget_cost: 1.0, depends_on: Vec::new(),
        }];
        let lat = b.predict_batch_latency(&tasks);
        assert!(lat > 400 && lat < 2000, "latency={}", lat);
    }

    #[test]
    fn worker_pool_least_loaded() {
        let mut pool = WorkerPool::new(3, 10);
        pool.get_mut(0).unwrap().in_flight = 5;
        pool.get_mut(1).unwrap().in_flight = 2;
        pool.get_mut(2).unwrap().in_flight = 8;
        assert_eq!(pool.least_loaded(), Some(1));
    }

    #[test]
    fn worker_pool_steal_pair() {
        let mut pool = WorkerPool::new(3, 10);
        pool.get_mut(0).unwrap().idle = TriState::True;
        pool.get_mut(1).unwrap().in_flight = 5;
        let pair = pool.steal_pair();
        assert!(pair.is_some());
        assert_eq!(pair.unwrap().0, 0);
    }

    #[test]
    fn parallel_time_less_than_serial() {
        let mut b = DynamicActionBatcher::new(BatchConfig {
            min_batch: 1, max_batch: 4, worker_count: 4, min_recompute_interval_us: 0, ..Default::default()
        });
        let p = b.compute_plan(&make_tasks(12), 1000);
        if p.total_actions > 1 {
            assert!(p.predicted_parallel_us <= p.predicted_total_us);
        }
    }

    #[test]
    fn reset_pid_clears_state() {
        let mut b = DynamicActionBatcher::new(BatchConfig::default());
        b.pid.integral = 10.0;
        b.fractional_accum = 0.7;
        b.reset_pid();
        assert_eq!(b.pid.integral, 0.0);
        assert_eq!(b.fractional_accum, 0.0);
    }

    #[test]
    fn custom_pid_tuning() {
        let b = DynamicActionBatcher::with_pid_tuning(BatchConfig::default(), 2.0, 0.5, 0.1);
        assert_eq!(b.pid.kp(), 2.0);
        assert_eq!(b.pid.ki(), 0.5);
        assert_eq!(b.pid.kd(), 0.1);
    }

    #[test]
    fn worker_load_factor() {
        let mut w = WorkerState::new(0);
        w.in_flight = 5;
        assert!((w.load_factor(10) - 0.5).abs() < 0.01);
    }

    #[test]
    fn worker_throughput_tracked() {
        let mut b = DynamicActionBatcher::new(BatchConfig { worker_count: 2, ..Default::default() });
        for _ in 0..5 { b.record_batch_done(0, 10, 10_000); }
        let w = b.pool().get(0).unwrap();
        assert!(w.throughput > 0.0);
        assert!(w.avg_latency_us > 0.0);
    }

    #[test]
    fn cache_ttl_expires() {
        let mut b = DynamicActionBatcher::new(BatchConfig { min_recompute_interval_us: 0, ..Default::default() });
        b.cache_ttl_us = 100;
        let _p1 = b.compute_plan(&make_tasks(8), 1000);
        let p2 = b.compute_plan(&make_tasks(8), 2000);
        assert!(p2.from_cache == TriState::False);
    }
}
