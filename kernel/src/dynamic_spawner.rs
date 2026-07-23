//! `kernel::dynamic_spawner` — PID-controlled agent spawn batching with caching.
//!
//! Decides HOW MANY agents to spawn per tick based on load feedback.
//! PID adjusts batch size: under load → fewer spawns, idle → more agents.
//! Predictions are cached and invalidated on new observations.
//! ASCII dashboard for live diagnostics.
//!
//! # Architecture
//! ```text
//! DynamicSpawner
//! +-- SpawnBatchConfig (min/max batch, rate limits)
//! +-- PidController (setpoint=target_queue, measurement=actual_queue)
//! +-- SpawnMetrics (EMA latency, success rate, queue depth)
//! +-- SpawnCache (cached predictions, invalidated on new data)
//! +-- compute_batch() -> SpawnBatch (count, reason, hash)
//! +-- ascii_dashboard() -> live diagnostics
//! ```

use crate::orchestrator::PidController;
use crate::TriState;

pub const MIN_SPAWN_BATCH: usize = 1;
pub const MAX_SPAWN_BATCH: usize = 8;
pub const MAX_SWARM_TOTAL: usize = 32;
pub const SPAWN_LATENCY_ALPHA: f64 = 0.3;
pub const SPAWN_SUCCESS_ALPHA: f64 = 0.2;

// ─── Spawn Batch Configuration ────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SpawnBatchConfig {
    pub min_batch: usize,
    pub max_batch: usize,
    pub max_swarm_total: usize,
    pub target_queue_depth: usize,
    pub min_interval_us: u64,
    pub max_rate_per_sec: f64,
}

impl Default for SpawnBatchConfig {
    fn default() -> Self {
        SpawnBatchConfig {
            min_batch: MIN_SPAWN_BATCH,
            max_batch: MAX_SPAWN_BATCH,
            max_swarm_total: MAX_SWARM_TOTAL,
            target_queue_depth: 4,
            min_interval_us: 100_000,
            max_rate_per_sec: 20.0,
        }
    }
}

// ─── Spawn Metrics (EMA) ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SpawnMetrics {
    pub avg_latency_us: f64,
    pub success_rate: f64,
    pub queue_depth: usize,
    pub active_agents: usize,
    pub total_spawned: u64,
    pub total_failures: u64,
    pub last_spawn_us: u64,
}

impl SpawnMetrics {
    pub fn new() -> Self {
        SpawnMetrics {
            avg_latency_us: 0.0,
            success_rate: 1.0,
            queue_depth: 0,
            active_agents: 0,
            total_spawned: 0,
            total_failures: 0,
            last_spawn_us: 0,
        }
    }

    pub fn record_batch(&mut self, spawned: usize, failed: usize, latency_us: u64, now_us: u64) {
        let total = spawned + failed;
        if total > 0 {
            let batch_success = spawned as f64 / total as f64;
            self.success_rate =
                SPAWN_SUCCESS_ALPHA * batch_success + (1.0 - SPAWN_SUCCESS_ALPHA) * self.success_rate;
        }
        if spawned > 0 && latency_us > 0 {
            let per_agent = latency_us as f64 / spawned as f64;
            self.avg_latency_us =
                SPAWN_LATENCY_ALPHA * per_agent + (1.0 - SPAWN_LATENCY_ALPHA) * self.avg_latency_us;
        }
        self.total_spawned += spawned as u64;
        self.total_failures += failed as u64;
        self.active_agents += spawned;
        self.active_agents = self.active_agents.saturating_sub(failed);
        self.last_spawn_us = now_us;
    }

    pub fn record_agent_died(&mut self) {
        self.active_agents = self.active_agents.saturating_sub(1);
    }

    pub fn spawn_capacity(&self, max_total: usize) -> usize {
        max_total.saturating_sub(self.active_agents)
    }
}

// ─── Prediction Cache ────────────────────────────────────────────────────

/// Cached spawn prediction — avoids recomputing when inputs haven't changed.
#[derive(Debug, Clone)]
pub struct SpawnCache {
    /// Cached batch count.
    pub cached_count: usize,
    /// Cached reason.
    pub cached_reason: SpawnReason,
    /// Queue depth this cache was computed for.
    pub cache_queue_depth: usize,
    /// Active agents when this cache was computed.
    pub cache_active_agents: usize,
    /// Timestamp of cache computation.
    pub cache_computed_us: u64,
    /// Whether cache is stale (new observation invalidated it).
    pub stale: TriState,
    /// Total cache hits.
    pub hits: u64,
    /// Total cache misses.
    pub misses: u64,
}

impl SpawnCache {
    pub fn new() -> Self {
        SpawnCache {
            cached_count: 0,
            cached_reason: SpawnReason::ColdStart,
            cache_queue_depth: 0,
            cache_active_agents: 0,
            cache_computed_us: 0,
            stale: TriState::Unknown,
            hits: 0,
            misses: 0,
        }
    }

    /// Check if cache is valid for the given inputs.
    pub fn is_valid(&self, queue_depth: usize, active_agents: usize, now_us: u64, ttl_us: u64) -> bool {
        self.stale == TriState::False
            && self.cache_queue_depth == queue_depth
            && self.cache_active_agents == active_agents
            && now_us.saturating_sub(self.cache_computed_us) < ttl_us
    }

    /// Serve from cache (increment hit counter).
    pub fn serve(&mut self) -> (usize, SpawnReason) {
        self.hits += 1;
        (self.cached_count, self.cached_reason)
    }

    /// Invalidate and record miss.
    pub fn invalidate(&mut self) {
        self.stale = TriState::True;
        self.misses += 1;
    }

    /// Update cache with new computation.
    pub fn update(&mut self, count: usize, reason: SpawnReason, queue_depth: usize, active_agents: usize, now_us: u64) {
        self.cached_count = count;
        self.cached_reason = reason;
        self.cache_queue_depth = queue_depth;
        self.cache_active_agents = active_agents;
        self.cache_computed_us = now_us;
        self.stale = TriState::False;
    }

    /// Cache hit rate.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }
}

// ─── Spawn Batch ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SpawnBatch {
    pub count: usize,
    pub skills: Vec<String>,
    pub pid_output: f64,
    pub pid_setpoint: f64,
    pub reason: SpawnReason,
    pub computed_us: u64,
    pub batch_hash: [u8; 32],
    /// Whether this batch was served from cache.
    pub from_cache: TriState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnReason {
    PidIncrease,
    PidHold,
    PidDecrease,
    ColdStart,
    AtCapacity,
    RateLimited,
    Cached,
}

// ─── Dynamic Spawner ─────────────────────────────────────────────────────

#[derive(Debug)]
pub struct DynamicSpawner {
    config: SpawnBatchConfig,
    pid: PidController,
    metrics: SpawnMetrics,
    cache: SpawnCache,
    fractional_accum: f64,
    /// Cache TTL (microseconds). How long a cached prediction is valid.
    cache_ttl_us: u64,
}

impl DynamicSpawner {
    pub fn new(config: SpawnBatchConfig) -> Self {
        DynamicSpawner {
            pid: PidController::new_min_max(0, config.max_batch),
            config,
            metrics: SpawnMetrics::new(),
            cache: SpawnCache::new(),
            fractional_accum: 0.0,
            cache_ttl_us: 50_000, // 50ms default TTL
        }
    }

    pub fn with_pid_tuning(config: SpawnBatchConfig, kp: f64, ki: f64, kd: f64) -> Self {
        let (min_batch, max_batch) = (config.min_batch, config.max_batch);
        let mut spawner = Self::new(config);
        spawner.pid = PidController::new(kp, ki, kd, min_batch as f64, max_batch as f64);
        spawner
    }

    pub fn with_cache_ttl(config: SpawnBatchConfig, cache_ttl_us: u64) -> Self {
        let mut spawner = Self::new(config);
        spawner.cache_ttl_us = cache_ttl_us;
        spawner
    }

    /// Compute the next spawn batch (with cache).
    pub fn compute_batch(&mut self, now_us: u64, pending_tasks: usize) -> SpawnBatch {
        self.metrics.queue_depth = pending_tasks;

        // Cache check.
        if self.cache.is_valid(pending_tasks, self.metrics.active_agents, now_us, self.cache_ttl_us) {
            let (count, _reason) = self.cache.serve();
            return self.make_batch_cached(count, SpawnReason::Cached, now_us);
        }
        self.cache.invalidate();

        // Cold start.
        if self.metrics.active_agents == 0 && pending_tasks > 0 {
            let count = self.config.min_batch.max(1);
            self.cache.update(count, SpawnReason::ColdStart, pending_tasks, self.metrics.active_agents, now_us);
            return self.make_batch(count, SpawnReason::ColdStart, now_us);
        }

        // Rate limit.
        if now_us.saturating_sub(self.metrics.last_spawn_us) < self.config.min_interval_us {
            self.cache.update(0, SpawnReason::RateLimited, pending_tasks, self.metrics.active_agents, now_us);
            return self.make_batch(0, SpawnReason::RateLimited, now_us);
        }

        // Capacity.
        let capacity = self.metrics.spawn_capacity(self.config.max_swarm_total);
        if capacity == 0 {
            self.cache.update(0, SpawnReason::AtCapacity, pending_tasks, self.metrics.active_agents, now_us);
            return self.make_batch(0, SpawnReason::AtCapacity, now_us);
        }

        // No demand.
        if pending_tasks == 0 && self.metrics.active_agents > 0 {
            self.cache.update(0, SpawnReason::PidHold, pending_tasks, self.metrics.active_agents, now_us);
            return self.make_batch(0, SpawnReason::PidHold, now_us);
        }

        // PID feedback.
        let target = self.config.target_queue_depth as f64;
        let actual = pending_tasks as f64;
        let pid_output = self.pid.update(target, actual);

        let reason = if pid_output > self.pid.output() + 0.1 {
            SpawnReason::PidIncrease
        } else if pid_output < self.pid.output - 0.1 {
            SpawnReason::PidDecrease
        } else {
            SpawnReason::PidHold
        };

        self.fractional_accum += pid_output;
        let raw_count = self.fractional_accum as usize;
        self.fractional_accum -= raw_count as f64;

        let count = raw_count
            .max(self.config.min_batch)
            .min(self.config.max_batch)
            .min(capacity);

        self.cache.update(count, reason, pending_tasks, self.metrics.active_agents, now_us);
        self.make_batch(count, reason, now_us)
    }

    pub fn record_outcome(&mut self, spawned: usize, failed: usize, latency_us: u64, now_us: u64) {
        self.metrics.record_batch(spawned, failed, latency_us, now_us);
        // Invalidate cache on new observation (inputs changed).
        self.cache.stale = TriState::True;
    }

    pub fn record_agent_died(&mut self) {
        self.metrics.record_agent_died();
        self.cache.stale = TriState::True;
    }

    /// ASCII dashboard for live diagnostics.
    pub fn ascii_dashboard(&self) -> String {
        let m = &self.metrics;
        let mut out = String::with_capacity(512);
        out.push_str("DynamicSpawner Dashboard\n");
        out.push_str(&format!("  Agents:      {}/{}\n", m.active_agents, self.config.max_swarm_total));
        out.push_str(&format!("  Queue:       {} pending\n", m.queue_depth));
        out.push_str(&format!("  PID output:  {:.2} (setpoint={})\n", self.pid.output(), self.config.target_queue_depth));
        out.push_str(&format!("  Batch size:  {:.0} (min={} max={})\n", self.pid.output(), self.config.min_batch, self.config.max_batch));
        out.push_str(&format!("  Spawned:     {} total, {} failures\n", m.total_spawned, m.total_failures));
        out.push_str(&format!("  Latency EMA: {:.0} us/agent\n", m.avg_latency_us));
        out.push_str(&format!("  Success:     {:.1}%\n", m.success_rate * 100.0));
        out.push_str(&format!("  Cache:       {:.0}% hit rate ({} hits / {} misses)\n",
            self.cache.hit_rate() * 100.0, self.cache.hits, self.cache.misses));
        out
    }

    pub fn metrics(&self) -> &SpawnMetrics { &self.metrics }
    pub fn pid_output(&self) -> f64 { self.pid.output() }
    pub fn pid_recommended(&self) -> usize { self.pid.recommended() }
    pub fn config(&self) -> &SpawnBatchConfig { &self.config }
    pub fn cache(&self) -> &SpawnCache { &self.cache }
    pub fn cache_mut(&mut self) -> &mut SpawnCache { &mut self.cache }

    pub fn reset_pid(&mut self) {
        self.pid.reset();
        self.fractional_accum = 0.0;
        self.cache.invalidate();
    }

    fn make_batch(&self, count: usize, reason: SpawnReason, now_us: u64) -> SpawnBatch {
        let hash_input = [count as u8, reason as u8, (self.pid.output() as u64).to_le_bytes()[0]];
        let batch_hash = crate::event_log::sha3_256(&hash_input);
        SpawnBatch {
            count,
            skills: Vec::new(),
            pid_output: self.pid.output(),
            pid_setpoint: self.config.target_queue_depth as f64,
            reason,
            computed_us: now_us,
            batch_hash,
            from_cache: TriState::False,
        }
    }

    fn make_batch_cached(&self, count: usize, reason: SpawnReason, now_us: u64) -> SpawnBatch {
        let hash_input = [count as u8, reason as u8, (self.pid.output() as u64).to_le_bytes()[0]];
        let batch_hash = crate::event_log::sha3_256(&hash_input);
        SpawnBatch {
            count,
            skills: Vec::new(),
            pid_output: self.pid.output(),
            pid_setpoint: self.config.target_queue_depth as f64,
            reason,
            computed_us: now_us,
            batch_hash,
            from_cache: TriState::True,
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_start_spawns_minimum() {
        let mut s = DynamicSpawner::new(SpawnBatchConfig::default());
        let b = s.compute_batch(1000, 5);
        assert!(b.count >= 1);
        assert_eq!(b.reason, SpawnReason::ColdStart);
    }

    #[test]
    fn no_demand_zero_batch() {
        let mut s = DynamicSpawner::new(SpawnBatchConfig::default());
        s.metrics.active_agents = 4;
        let b = s.compute_batch(1000, 0);
        assert_eq!(b.count, 0);
    }

    #[test]
    fn at_capacity_zero_batch() {
        let mut s = DynamicSpawner::new(SpawnBatchConfig { max_swarm_total: 2, min_interval_us: 0, ..Default::default() });
        s.metrics.active_agents = 2;
        let b = s.compute_batch(100_000, 10);
        assert_eq!(b.count, 0);
        assert_eq!(b.reason, SpawnReason::AtCapacity);
    }

    #[test]
    fn rate_limit_zero_batch() {
        let mut s = DynamicSpawner::new(SpawnBatchConfig { min_interval_us: 1000, ..Default::default() });
        s.metrics.active_agents = 4;
        s.metrics.last_spawn_us = 500;
        let b = s.compute_batch(1000, 5);
        assert_eq!(b.count, 0);
        assert_eq!(b.reason, SpawnReason::RateLimited);
    }

    #[test]
    fn cache_hit_on_same_inputs() {
        let mut s = DynamicSpawner::new(SpawnBatchConfig {
            min_interval_us: 0,
            ..Default::default()
        });
        s.metrics.active_agents = 4;
        let _b1 = s.compute_batch(1000, 5);
        let b2 = s.compute_batch(1050, 5); // same queue_depth, same active, within TTL
        assert_eq!(b2.reason, SpawnReason::Cached);
        assert!(b2.from_cache.is_true());
    }

    #[test]
    fn cache_invalidation_on_outcome() {
        let mut s = DynamicSpawner::new(SpawnBatchConfig { min_interval_us: 0, ..Default::default() });
        s.metrics.active_agents = 4;
        let _b1 = s.compute_batch(1000, 5);
        assert!(s.cache().stale == TriState::False);
        s.record_outcome(2, 0, 100, 1100);
        assert!(s.cache().stale == TriState::True);
    }

    #[test]
    fn cache_hit_rate() {
        let mut s = DynamicSpawner::new(SpawnBatchConfig { min_interval_us: 0, ..Default::default() });
        s.metrics.active_agents = 4;
        let _b1 = s.compute_batch(1000, 5);
        let _b2 = s.compute_batch(1050, 5); // cache hit
        let _b3 = s.compute_batch(1100, 5); // cache hit
        assert!(s.cache().hit_rate() > 0.0);
    }

    #[test]
    fn dashboard_contains_sections() {
        let s = DynamicSpawner::new(SpawnBatchConfig::default());
        let d = s.ascii_dashboard();
        assert!(d.contains("DynamicSpawner Dashboard"));
        assert!(d.contains("Agents:"));
        assert!(d.contains("PID output:"));
        assert!(d.contains("Cache:"));
    }

    #[test]
    fn metrics_track_success_rate() {
        let mut m = SpawnMetrics::new();
        m.record_batch(8, 2, 1000, 1000);
        // EMA: 0.2 * 0.8 + 0.8 * 1.0 = 0.96
        assert!((m.success_rate - 0.96).abs() < 0.01);
        m.record_batch(10, 0, 500, 2000);
        // EMA: 0.2 * 1.0 + 0.8 * 0.96 = 0.968
        assert!((m.success_rate - 0.968).abs() < 0.01);
    }

    #[test]
    fn spawn_capacity_respects_limit() {
        let m = SpawnMetrics { active_agents: 28, ..SpawnMetrics::new() };
        assert_eq!(m.spawn_capacity(32), 4);
        assert_eq!(m.spawn_capacity(28), 0);
    }

    #[test]
    fn record_agent_died_decrements() {
        let mut m = SpawnMetrics::new();
        m.active_agents = 5;
        m.record_agent_died();
        assert_eq!(m.active_agents, 4);
    }

    #[test]
    fn batch_hash_deterministic() {
        let s = DynamicSpawner::new(SpawnBatchConfig::default());
        let b1 = s.make_batch(3, SpawnReason::PidIncrease, 1000);
        let b2 = s.make_batch(3, SpawnReason::PidIncrease, 1000);
        assert_eq!(b1.batch_hash, b2.batch_hash);
    }

    #[test]
    fn different_reason_different_hash() {
        let s = DynamicSpawner::new(SpawnBatchConfig::default());
        let b1 = s.make_batch(3, SpawnReason::PidIncrease, 1000);
        let b2 = s.make_batch(3, SpawnReason::PidHold, 1000);
        assert_ne!(b1.batch_hash, b2.batch_hash);
    }

    #[test]
    fn reset_pid_clears_state() {
        let mut s = DynamicSpawner::new(SpawnBatchConfig::default());
        s.pid.integral = 10.0;
        s.fractional_accum = 0.7;
        s.reset_pid();
        assert_eq!(s.pid.integral, 0.0);
        assert_eq!(s.fractional_accum, 0.0);
    }

    #[test]
    fn custom_pid_tuning_applied() {
        let s = DynamicSpawner::with_pid_tuning(SpawnBatchConfig::default(), 2.0, 0.5, 0.1);
        assert_eq!(s.pid.kp(), 2.0);
        assert_eq!(s.pid.ki(), 0.5);
        assert_eq!(s.pid.kd(), 0.1);
    }

    #[test]
    fn batch_count_clamped() {
        let mut s = DynamicSpawner::new(SpawnBatchConfig {
            min_batch: 2, max_batch: 4, max_swarm_total: 100, min_interval_us: 0, ..Default::default()
        });
        s.metrics.active_agents = 1;
        let b = s.compute_batch(1000, 100);
        assert!(b.count <= 4);
        assert!(b.count >= 2);
    }

    #[test]
    fn cache_ttl_expires() {
        let mut s = DynamicSpawner::new(SpawnBatchConfig { min_interval_us: 0, ..Default::default() });
        s.cache_ttl_us = 100;
        s.metrics.active_agents = 4;
        let _b1 = s.compute_batch(1000, 5);
        // After TTL expires, should recompute.
        let b2 = s.compute_batch(2000, 5);
        assert_ne!(b2.reason, SpawnReason::Cached);
    }

    #[test]
    fn cache_invalidation_on_agent_died() {
        let mut s = DynamicSpawner::new(SpawnBatchConfig { min_interval_us: 0, ..Default::default() });
        s.metrics.active_agents = 4;
        let _b1 = s.compute_batch(1000, 5);
        assert!(s.cache().stale == TriState::False);
        s.record_agent_died();
        assert!(s.cache().stale == TriState::True);
    }
}
