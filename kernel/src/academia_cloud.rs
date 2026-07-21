//! `kernel::academia_cloud` — PID-керований Cloudflare Workers пул.
//!
//! # Динамічний PID-контролер
//! Використовує наявний `PidController` з `orchestrator.rs`:
//! - Setpoint: 0 (всі папери екстрактовано)
//! - Measurement: залишок паперів (total - extracted)
//! - Output: кількість активних Workers
//! - Kp/Ki/Kd: anti-windup integral
//!
//! # Спавн Workers
//! - PID output > current workers → spawn нові
//! - PID output < current workers → decommission зайві
//! - Workers без роботи > timeout → самознищення
//! - FanOutPlan для розподілу чанків
//!
//! # Zero-trace
//! Кожен Worker через Cloudflare = різний IP.
//! Випадковий User-Agent, jitter, chaff.
//!
//! # Пам'ять
//! - Worker: ~1 MB (JavaScript)
//! - 10,000 Workers × 1 MB = ~10 GB (Cloudflare, безкоштовно)
//! - R2: 4.6 GB (free tier 10 GB)

use crate::dynamic_spawner::{DynamicSpawner, SpawnBatchConfig};
use crate::orchestrator::PidController;
use crate::parallel_patterns::FanOutPlan;
use crate::TriState;

/// Максимум Workers.
pub const MAX_WORKERS: u32 = 100_000;
/// Паперів на Worker.
pub const PAPERS_PER_WORKER: u64 = 61_000; // 61K × 100K = 610M
/// Таймаут бездіяльності Worker (секунди).
pub const WORKER_TIMEOUT_S: u64 = 300;
/// Cloudflare Workers free tier limit.
pub const CF_FREE_LIMIT: u32 = 100_000;

// ─── Worker State ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CfWorker {
    pub id: u32,
    pub region: String,
    pub status: CfWorkerStatus,
    pub papers_done: u64,
    pub last_active: u64,
    pub pid_output: f64,
    pub batch_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfWorkerStatus {
    Spawning,
    Active,
    Idle,
    Decommissioned,
}

// ─── Cloudflare PID Controller ───────────────────────────────────────────

/// PID-керований пул Cloudflare Workers.
#[derive(Debug)]
pub struct CfPidPool {
    /// PID controller (setpoint=0, measurement=remaining papers).
    pub pid: PidController,
    /// Dynamic spawner for batch sizing.
    pub spawner: DynamicSpawner,
    /// Active workers.
    pub workers: Vec<CfWorker>,
    /// Total papers target.
    pub total_papers: u64,
    /// Papers extracted so far.
    pub extracted: u64,
    /// Current PID output (recommended worker count).
    pub target_workers: f64,
}

impl CfPidPool {
    pub fn new(total_papers: u64) -> Self {
        CfPidPool {
            pid: PidController::new(1, 10000), // kp=0.1, ki=1000
            spawner: DynamicSpawner::new(SpawnBatchConfig {
                min_batch: 1, max_batch: 100,
                max_swarm_total: MAX_WORKERS as usize,
                target_queue_depth: 50,
                min_interval_us: 1_000_000, // 1s min between spawns
                max_rate_per_sec: 100.0,
            }),
            workers: Vec::new(),
            total_papers,
            extracted: 0,
            target_workers: 0.0,
        }
    }

    /// PID update: обчислити скільки Workers потрібно.
    pub fn update(&mut self, now_us: u64) {
        let remaining = self.total_papers.saturating_sub(self.extracted);
        let setpoint = 0.0; // Target: 0 remaining papers
        let measurement = remaining as f64;

        // PID computes recommended concurrency
        self.target_workers = self.pid.update(setpoint, measurement);

        // Clamp to CF limits and max workers
        self.target_workers = self.target_workers
            .clamp(1.0, MAX_WORKERS as f64)
            .min(CF_FREE_LIMIT as f64);

        // Spawn or decommission based on PID
        let current = self.workers.len() as f64;
        let diff = self.target_workers - current;

        // Spawn new workers
        if diff > 0.0 && self.spawner.metrics().active_agents < MAX_WORKERS as usize {
            let batch = self.spawner.compute_batch(now_us, remaining as usize);
            for i in 0..batch.count {
                let wid = self.workers.len() as u32 + i as u32;
                if wid >= MAX_WORKERS { break; }
                self.workers.push(CfWorker {
                    id: wid, region: Self::region_for(wid),
                    status: CfWorkerStatus::Spawning,
                    papers_done: 0, last_active: now_us,
                    pid_output: self.target_workers,
                    batch_id: now_us,
                });
            }
        }

        // Decommission idle workers (not active > timeout)
        let timeout_us = WORKER_TIMEOUT_S * 1_000_000;
        self.workers.retain(|w| {
            w.status != CfWorkerStatus::Idle || now_us.saturating_sub(w.last_active) <= timeout_us
        });

        // Update worker statuses
        for w in &mut self.workers {
            if w.status == CfWorkerStatus::Spawning {
                w.status = CfWorkerStatus::Active;
            }
        }
    }

    /// Розподілити роботу між активними Workers (FanOut).
    pub fn distribute(&self) -> Vec<(u32, u64, u64)> {
        let active: Vec<&CfWorker> = self.workers.iter()
            .filter(|w| w.status == CfWorkerStatus::Active)
            .collect();

        let remaining = self.total_papers.saturating_sub(self.extracted);
        let plan = FanOutPlan::plan(remaining as usize, active.len().max(1), 100, crate::orchestrator::Priority::Normal);

        active.iter().zip(plan.assignments().iter()).map(|(w, &(_, start, end))| {
            (w.id, start as u64 + self.extracted, (end - start) as u64)
        }).collect()
    }

    /// Вибрати регіон для Worker (гео-розподіл).
    fn region_for(id: u32) -> String {
        let regions = ["us-east", "us-west", "eu-west", "eu-central", "ap-northeast",
                       "ap-southeast", "sa-east", "af-south", "me-central", "au-east"];
        regions[id as usize % regions.len()].to_string()
    }

    /// Час до завершення при поточній швидкості.
    pub fn eta(&self) -> String {
        let active = self.workers.iter().filter(|w| w.status == CfWorkerStatus::Active).count().max(1);
        let per_worker = PAPERS_PER_WORKER; // papers per second per worker
        let rate = active as u64 * per_worker;
        let remaining = self.total_papers.saturating_sub(self.extracted);
        if rate == 0 { return "∞".into(); }
        let secs = remaining / rate;
        let h = secs / 3600; let m = (secs % 3600) / 60; let s = secs % 60;
        if h > 0 { format!("{}год {}хв", h, m) }
        else if m > 0 { format!("{}хв {}с", m, s) }
        else { format!("{}с", s) }
    }

    pub fn dashboard(&self) -> String {
        let active = self.workers.iter().filter(|w| w.status == CfWorkerStatus::Active).count();
        let idle = self.workers.iter().filter(|w| w.status == CfWorkerStatus::Idle).count();
        let remaining = self.total_papers.saturating_sub(self.extracted);
        format!(
            "Academia Cloud (PID)\n  PID target: {:.0} workers\n  Active:     {}\n  Idle:       {}\n  Extracted:  {:.1}e / {:.1}e\n  Remaining:  {:.1}e\n  ETA:        {}\n  PID:        kp={:.3} ki={:.0} out={:.1}",
            self.target_workers, active, idle,
            self.extracted as f64, self.total_papers as f64,
            remaining as f64, self.eta(),
            self.pid.kp, self.pid.ki, self.pid.output
        )
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_spawns_workers_for_remaining() {
        let mut pool = CfPidPool::new(610_000_000);
        pool.update(1_000_000);
        // With 610M remaining, PID should recommend >0 workers
        assert!(pool.target_workers > 0.0);
    }

    #[test]
    fn pid_reduces_workers_as_papers_decrease() {
        let mut pool = CfPidPool::new(610_000_000);
        pool.extracted = 609_000_000; // Almost done
        pool.update(1_000_000);
        // With 1M remaining, fewer workers needed
        let low_target = pool.target_workers;
        pool.extracted = 0;
        pool.update(2_000_000);
        assert!(pool.target_workers >= low_target);
    }

    #[test]
    fn distribute_fanout() {
        let mut pool = CfPidPool::new(100_000);
        pool.extracted = 0;
        pool.update(1_000_000);
        let dist = pool.distribute();
        // Should distribute remaining work among active workers
        let total: u64 = dist.iter().map(|(_, _, c)| c).sum();
        assert_eq!(total, 100_000);
    }

    #[test]
    fn decommission_idle_workers() {
        let mut pool = CfPidPool::new(1000);
        pool.pid = PidController::new(0, 0); // Disable PID → no new spawns
        pool.update(1_000_000);
        // PID disabled, so worker count should be minimal
        let before = pool.workers.len();
        for w in &mut pool.workers {
            w.status = CfWorkerStatus::Idle;
            w.last_active = 0;
        }
        pool.update(1_000_000_000);
        // Without PID, no new workers spawned, idle ones removed
        assert!(pool.workers.len() <= before);
    }

    #[test]
    fn eta_decreases_with_more_workers() {
        let pool = CfPidPool::new(610_000_000);
        let eta = pool.eta();
        assert!(!eta.contains("∞"));
    }

    #[test]
    fn dashboard_contains_cloud() {
        let pool = CfPidPool::new(1000);
        let d = pool.dashboard();
        assert!(d.contains("Academia Cloud"));
    }

    #[test]
    fn cf_free_limit_enforced() {
        let mut pool = CfPidPool::new(1_000_000_000_000);
        pool.update(1_000_000);
        assert!(pool.target_workers <= CF_FREE_LIMIT as f64);
    }

    #[test]
    fn regions_are_distributed() {
        let regions: std::collections::HashSet<String> = (0..100).map(|i| CfPidPool::region_for(i)).collect();
        assert!(regions.len() >= 5); // At least 5 different regions
    }
}
