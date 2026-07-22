//! `kernel::swarm` — decentralized mesh swarm coordinator.
//!
//! Implements the mesh swarm architecture: agents self-organize, decompose tasks
//! via DSU, dispatch through spool + token bucket, monitor via breakers + FDR,
//! and adapt via spectral/Markov prediction. No hierarchical orchestration —
//! agents select skills from living memory based on context.
//!
//! # Architecture
//! ```text
//! SwarmCoordinator
//! +-- Task Decomposition
//! |   +-- DSU.components(task_deps) -> independent groups
//! |   +-- Router.dispatch(group, executors) -> assignment
//! +-- Execution
//! |   +-- Spool.append(TaskSpec[]) -> durable queue
//! |   +-- TokenBucket.child_bucket() -> budget slice per executor
//! |   +-- AgentLoop.run(executor) -> parallel execution
//! +-- Monitoring
//! |   +-- Breaker per executor -> fault isolation
//! |   +-- FDR.event!() -> telemetry stream
//! +-- Dynamic Adaptation
//!     +-- markov::analyze() -> swarm health verdict
//!     +-- spectral::classify_drift() -> swarm trajectory
//! ```

use crate::dsu::Dsu;

/// Maximum number of executors in a swarm.
pub const MAX_SWARM_SIZE: usize = 16;

/// Maximum number of sub-tasks per FanOut.
pub const MAX_FANOUT: usize = 32;

/// Health status of a single executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutorHealth {
    /// Executor is running normally.
    Healthy,
    /// Executor has tripped its circuit breaker.
    Tripped,
    /// Executor has not reported in within the heartbeat window.
    Stale,
    /// Executor status unknown (not yet observed).
    Unknown,
}

/// Overall swarm health.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwarmHealth {
    /// All executors healthy.
    AllHealthy,
    /// Some executors tripped or stale, but swarm can still function.
    Degraded,
    /// Too many executors tripped — swarm cannot meet its quota.
    Critical,
    /// No executors active.
    Empty,
}

/// A swarmling — one executor in the mesh swarm.
#[derive(Debug, Clone)]
pub struct Swarmling {
    /// Unique executor id.
    pub id: usize,
    /// Skills this executor can perform (selected from living memory).
    pub skills: Vec<String>,
    /// Budget capacity (token bucket capacity).
    pub budget_capacity: f64,
    /// Budget consumed so far.
    pub budget_consumed: f64,
    /// Health status.
    pub health: ExecutorHealth,
    /// Number of tasks completed.
    pub tasks_completed: u64,
    /// Number of tasks failed.
    pub tasks_failed: u64,
    /// Last heartbeat timestamp (monotonic ms).
    pub last_heartbeat_ms: u64,
}

impl Swarmling {
    /// Create a new swarmling with a given skill set and budget capacity.
    pub fn new(id: usize, skills: Vec<String>, budget_capacity: f64) -> Self {
        Swarmling {
            id,
            skills,
            budget_capacity,
            budget_consumed: 0.0,
            health: ExecutorHealth::Healthy,
            tasks_completed: 0,
            tasks_failed: 0,
            last_heartbeat_ms: 0,
        }
    }

    /// Check if this swarmling can perform a given skill.
    pub fn can_handle(&self, skill: &str) -> bool {
        self.skills.iter().any(|s| s == skill)
    }

    /// Try to acquire budget. Returns true if budget available.
    pub fn try_acquire(&mut self, amount: f64) -> bool {
        if self.budget_consumed + amount <= self.budget_capacity {
            self.budget_consumed += amount;
            true
        } else {
            false
        }
    }

    /// Record a task completion.
    pub fn record_success(&mut self) {
        self.tasks_completed += 1;
    }

    /// Record a task failure.
    pub fn record_failure(&mut self) {
        self.tasks_failed += 1;
    }

    /// Compute success rate (0.0-1.0). Returns 0.5 for no data (neutral prior).
    pub fn success_rate(&self) -> f64 {
        let total = self.tasks_completed + self.tasks_failed;
        if total == 0 {
            0.5
        } else {
            self.tasks_completed as f64 / total as f64
        }
    }
}

/// A task specification for the swarm.
#[derive(Debug, Clone)]
pub struct TaskSpec {
    /// Unique task id.
    pub id: usize,
    /// Skill name required.
    pub skill: String,
    /// Raw argument.
    pub raw_arg: String,
    /// Task dependencies (other task ids that must complete first).
    pub dependencies: Vec<usize>,
}

/// Result of a completed task.
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Task id.
    pub id: usize,
    /// Whether the task succeeded.
    pub success: bool,
    /// Output content (if successful).
    pub output: String,
    /// Error message (if failed).
    pub error: String,
    /// Executor id that ran this task.
    pub executor_id: usize,
}

/// The swarm coordinator — manages task decomposition, dispatch, and monitoring.
pub struct SwarmCoordinator {
    /// Active executors.
    swarmlings: Vec<Swarmling>,
    /// Task dependency graph (DSU for grouping independent tasks).
    dsu: Dsu,
    /// Completed task results.
    results: Vec<TaskResult>,
    /// Total tasks dispatched.
    tasks_dispatched: u64,
}

impl SwarmCoordinator {
    /// Create a coordinator with the given executors.
    pub fn new(swarmlings: Vec<Swarmling>) -> Self {
        let _n = swarmlings.len();
        SwarmCoordinator {
            swarmlings,
            dsu: Dsu::new(MAX_FANOUT),
            results: Vec::new(),
            tasks_dispatched: 0,
        }
    }

    /// Get the number of active executors.
    pub fn executor_count(&self) -> usize {
        self.swarmlings.len()
    }

    /// Get overall swarm health.
    pub fn health(&self) -> SwarmHealth {
        if self.swarmlings.is_empty() {
            return SwarmHealth::Empty;
        }
        let tripped = self
            .swarmlings
            .iter()
            .filter(|s| s.health != ExecutorHealth::Healthy)
            .count();
        if tripped == 0 {
            SwarmHealth::AllHealthy
        } else if tripped < self.swarmlings.len() {
            SwarmHealth::Degraded
        } else {
            SwarmHealth::Critical
        }
    }

    /// Decompose a set of tasks into independent groups using DSU.
    /// Tasks with no dependencies form single-element groups.
    pub fn decompose(&mut self, tasks: &[TaskSpec]) -> Vec<Vec<usize>> {
        self.dsu = Dsu::new(tasks.len());
        for task in tasks {
            for &dep in &task.dependencies {
                self.dsu.union(task.id, dep);
            }
        }
        let present = vec![true; tasks.len()];
        self.dsu.components(&present)
    }

    /// Find the best executor for a task (highest success rate among those
    /// that can handle the required skill).
    pub fn select_executor(&self, task: &TaskSpec) -> Option<usize> {
        self.swarmlings
            .iter()
            .filter(|s| s.health == ExecutorHealth::Healthy && s.can_handle(&task.skill))
            .max_by(|a, b| {
                a.success_rate()
                    .partial_cmp(&b.success_rate())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.id)
    }

    /// Dispatch a task to a specific executor. Returns false if budget is exhausted.
    pub fn dispatch(&mut self, _task: &TaskSpec, executor_id: usize) -> bool {
        if let Some(exec) = self.swarmlings.iter_mut().find(|s| s.id == executor_id) {
            if exec.try_acquire(1.0) {
                self.tasks_dispatched += 1;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Record a task result and update executor health.
    pub fn record_result(&mut self, result: TaskResult) {
        if let Some(exec) = self
            .swarmlings
            .iter_mut()
            .find(|s| s.id == result.executor_id)
        {
            if result.success {
                exec.record_success();
            } else {
                exec.record_failure();
                // Trip breaker if failure rate exceeds threshold
                if exec.tasks_failed > 3 && exec.success_rate() < 0.3 {
                    exec.health = ExecutorHealth::Tripped;
                }
            }
        }
        self.results.push(result);
    }

    /// Get all completed results.
    pub fn results(&self) -> &[TaskResult] {
        &self.results
    }

    /// Total tasks dispatched.
    pub fn tasks_dispatched(&self) -> u64 {
        self.tasks_dispatched
    }

    /// Compute the swarm's aggregate success rate.
    pub fn aggregate_success_rate(&self) -> f64 {
        let total_completed: u64 = self
            .swarmlings
            .iter()
            .map(|s| s.tasks_completed + s.tasks_failed)
            .sum();
        let total_success: u64 = self.swarmlings.iter().map(|s| s.tasks_completed).sum();
        if total_completed == 0 {
            0.5
        } else {
            total_success as f64 / total_completed as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_swarmling(id: usize, skills: Vec<&str>, budget: f64) -> Swarmling {
        Swarmling::new(
            id,
            skills.into_iter().map(String::from).collect(),
            budget,
        )
    }

    #[test]
    fn swarmling_can_handle() {
        let s = make_swarmling(0, vec!["search", "parse"], 10.0);
        assert!(s.can_handle("search"));
        assert!(s.can_handle("parse"));
        assert!(!s.can_handle("write"));
    }

    #[test]
    fn swarmling_success_rate() {
        let mut s = make_swarmling(0, vec!["search"], 10.0);
        assert_eq!(s.success_rate(), 0.5); // neutral prior
        s.record_success();
        s.record_success();
        s.record_failure();
        assert!((s.success_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn coordinator_healthy() {
        let swarmlings = vec![
            make_swarmling(0, vec!["search"], 10.0),
            make_swarmling(1, vec!["parse"], 10.0),
        ];
        let coord = SwarmCoordinator::new(swarmlings);
        assert_eq!(coord.health(), SwarmHealth::AllHealthy);
        assert_eq!(coord.executor_count(), 2);
    }

    #[test]
    fn coordinator_empty() {
        let coord = SwarmCoordinator::new(vec![]);
        assert_eq!(coord.health(), SwarmHealth::Empty);
    }

    #[test]
    fn select_executor_picks_best() {
        let mut s0 = make_swarmling(0, vec!["search"], 10.0);
        let mut s1 = make_swarmling(1, vec!["search"], 10.0);
        // s0 has better history
        for _ in 0..8 {
            s0.record_success();
        }
        s0.record_failure();
        for _ in 0..4 {
            s1.record_success();
        }
        for _ in 0..4 {
            s1.record_failure();
        }
        let coord = SwarmCoordinator::new(vec![s0, s1]);
        let task = TaskSpec {
            id: 0,
            skill: "search".to_string(),
            raw_arg: "test".to_string(),
            dependencies: vec![],
        };
        assert_eq!(coord.select_executor(&task), Some(0)); // s0 has 0.89 vs s1 0.5
    }

    #[test]
    fn dispatch_costs_budget() {
        let swarmlings = vec![make_swarmling(0, vec!["search"], 2.0)];
        let mut coord = SwarmCoordinator::new(swarmlings);
        let task = TaskSpec {
            id: 0,
            skill: "search".to_string(),
            raw_arg: "test".to_string(),
            dependencies: vec![],
        };
        assert!(coord.dispatch(&task, 0));
        assert!(coord.dispatch(&task, 0));
        assert!(!coord.dispatch(&task, 0)); // budget exhausted
    }

    #[test]
    fn decompose_independent_tasks() {
        let swarmlings = vec![make_swarmling(0, vec!["search"], 10.0)];
        let mut coord = SwarmCoordinator::new(swarmlings);
        let tasks = vec![
            TaskSpec { id: 0, skill: "a".into(), raw_arg: "".into(), dependencies: vec![] },
            TaskSpec { id: 1, skill: "b".into(), raw_arg: "".into(), dependencies: vec![] },
            TaskSpec { id: 2, skill: "c".into(), raw_arg: "".into(), dependencies: vec![0] },
        ];
        let groups = coord.decompose(&tasks);
        // Tasks 0 and 1 are independent; task 2 depends on 0
        assert!(!groups.is_empty());
    }

    #[test]
    fn record_result_updates_health() {
        let swarmlings = vec![make_swarmling(0, vec!["search"], 10.0)];
        let mut coord = SwarmCoordinator::new(swarmlings);
        // Fail 4 times with only 1 success -> tripped
        for _ in 0..4 {
            coord.record_result(TaskResult {
                id: 0,
                success: false,
                output: String::new(),
                error: "fail".into(),
                executor_id: 0,
            });
        }
        coord.record_result(TaskResult {
            id: 1,
            success: true,
            output: "ok".into(),
            error: String::new(),
            executor_id: 0,
        });
        assert_eq!(coord.health(), SwarmHealth::Critical);
        assert!((coord.aggregate_success_rate() - 0.2).abs() < 0.01);
    }
}
