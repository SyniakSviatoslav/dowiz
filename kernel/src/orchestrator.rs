//! orchestrator.rs — Kernel-native tool/skill/agent orchestration.
//!
//! # What this is
//! A centralized orchestrator that manages tool invocation, skill selection,
//! agent dispatch, and parallel execution. ALL metrics, health signals, and
//! load predictions flow through this module — no grep, no shell scripts,
//! no external processes. Everything is kernel-native and cryptographically
//! verifiable.
//!
//! # Design principles
//! - Zero grep: all search via kernel-native BM25/trigram/PPR
//! - Zero scripts: all orchestration via Rust structs and functions
//! - Detailed metrics: every action emits structured telemetry
//! - Health signaling: real-time system health assessment
//! - Load prediction: predict system load from action history
//! - Parallel dispatch: fan-out based on available resources
//! - Cryptographic verification: SHA3-256 on all state transitions

use std::fmt;

use crate::event_log::sha3_256;
use crate::token_bucket::TokenBucket;
use crate::workflow_gate::{GatePhase, GateError, WorkflowGate};
use crate::TriState;

/// Maximum concurrent tasks (parallel dispatch limit).
const MAX_CONCURRENT: usize = 8;

/// Action categories for metrics and load tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionCategory {
    /// Read-only tool invocation.
    ToolRead,
    /// Write tool invocation.
    ToolWrite,
    /// Skill execution (composite action).
    Skill,
    /// Agent dispatch (LLM call).
    AgentDispatch,
    /// Swarm fan-out.
    SwarmFanOut,
    /// Verification step.
    Verification,
    /// Browser-based parse operation (anti-detect, zero-trace, PQ-signed).
    Parse,
}

impl ActionCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            ActionCategory::ToolRead => "tool_read",
            ActionCategory::ToolWrite => "tool_write",
            ActionCategory::Skill => "skill",
            ActionCategory::AgentDispatch => "agent_dispatch",
            ActionCategory::SwarmFanOut => "swarm_fan_out",
            ActionCategory::Verification => "verification",
            ActionCategory::Parse => "parse",
        }
    }
}

// ─── PID Controller ───────────────────────────────────────────────────────

/// PID (Proportional-Integral-Derivative) controller for dynamic concurrency.
///
/// Re-exported from `crate::pid::PidController` for backward compatibility.
/// The generalized version lives in `pid.rs` and supports f64, f32, and
/// vectorized batch variants.
pub use crate::pid::PidController as PidController;

// ─── Priority Scheduler ───────────────────────────────────────────────────

/// Task priority (higher = execute first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    /// Background maintenance (lowest).
    Background = 0,
    /// Normal tool invocation.
    Normal = 1,
    /// Interactive / user-facing (higher).
    Interactive = 2,
    /// Parse operations (need anti-detect + PQ signing).
    Parse = 3,
    /// Verification / integrity checks (highest — must pass before commit).
    Critical = 4,
}

impl Priority {
    pub fn as_str(self) -> &'static str {
        match self {
            Priority::Background => "background",
            Priority::Normal => "normal",
            Priority::Interactive => "interactive",
            Priority::Parse => "parse",
            Priority::Critical => "critical",
        }
    }
}

/// A task ready for dispatch, with priority and estimated execution time.
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// Unique task identifier (PID-like: monotonic per-session).
    pub task_id: u64,
    /// Action category.
    pub category: ActionCategory,
    /// Priority level.
    pub priority: Priority,
    /// Estimated execution time in microseconds (from prediction engine).
    pub estimated_us: u64,
    /// When this task was created (unix microseconds).
    pub created_us: u64,
    /// Budget cost (tokens or compute units).
    pub budget_cost: f64,
    /// Dependencies: task_ids that must complete before this one can start.
    pub depends_on: Vec<u64>,
}

// ─── Predictive ETA Engine ────────────────────────────────────────────────

/// Rolling-window prediction for action execution time and load.
///
/// Uses exponential moving averages (EMA) over the recent history to predict:
/// - How long a given action category will take
/// - What the system load will be in the next window
/// - When each queued task will complete (priority-aware scheduling)
#[derive(Debug)]
pub struct PredictiveEngine {
    /// EMA estimates per action category: (avg_us, avg_variance).
    pub category_stats: std::collections::HashMap<ActionCategory, EmaEstimate>,
    /// Global load EMA (actions per second).
    pub load_ema: f64,
    /// Global latency EMA (microseconds).
    pub latency_ema: f64,
    /// Alpha for EMA smoothing (0.0 = never update, 1.0 = only latest).
    pub alpha: f64,
    /// Number of observations.
    pub observations: u64,
}

/// Exponential moving average estimate for a single metric.
#[derive(Debug, Clone)]
pub struct EmaEstimate {
    /// Current EMA value (e.g. average latency in us).
    pub value: f64,
    /// Current variance estimate (for confidence intervals).
    pub variance: f64,
    /// Number of observations for this category.
    pub count: u64,
}

impl PredictiveEngine {
    /// Create a new predictive engine.
    pub fn new() -> Self {
        PredictiveEngine {
            category_stats: std::collections::HashMap::new(),
            load_ema: 0.0,
            latency_ema: 0.0,
            alpha: 0.3, // responsive but not too twitchy
            observations: 0,
        }
    }

    /// Record an observation and update the model.
    pub fn observe(&mut self, category: ActionCategory, duration_us: u64) {
        let alpha = self.alpha;
        self.observations += 1;

        let entry = self
            .category_stats
            .entry(category)
            .or_insert_with(|| EmaEstimate {
                value: duration_us as f64,
                variance: 0.0,
                count: 0,
            });

        let old_val = entry.value;
        entry.value = alpha * (duration_us as f64) + (1.0 - alpha) * old_val;
        let diff = (duration_us as f64) - old_val;
        entry.variance = alpha * diff * diff + (1.0 - alpha) * entry.variance;
        entry.count += 1;

        // Global stats.
        self.latency_ema = alpha * (duration_us as f64) + (1.0 - alpha) * self.latency_ema;
    }

    /// Predict execution time for a given category.
    ///
    /// Returns (estimated_us, confidence_interval_95_us).
    /// If no data for this category, returns a conservative default.
    pub fn predict_eta(&self, category: ActionCategory) -> (u64, u64) {
        match self.category_stats.get(&category) {
            Some(stats) if stats.count >= 3 => {
                let ci_95 = (1.96 * stats.variance.sqrt() * 2.0) as u64; // 95% CI half-width
                (stats.value as u64, ci_95)
            }
            _ => {
                // Conservative default: 100ms, wide CI.
                (100_000, 50_000)
            }
        }
    }

    /// Predict the total ETA for a queue of tasks (priority-sorted).
    ///
    /// Assumes `available_concurrent` tasks can run in parallel.
    /// Returns Vec<(task_id, predicted_start_us, predicted_end_us)>.
    pub fn predict_schedule(
        &self,
        tasks: &[ScheduledTask],
        available_concurrent: usize,
    ) -> Vec<(u64, u64, u64)> {
        if tasks.is_empty() || available_concurrent == 0 {
            return Vec::new();
        }

        // Sort by priority (descending), then by estimated duration (ascending).
        let mut sorted: Vec<&ScheduledTask> = tasks.iter().collect();
        sorted.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(a.estimated_us.cmp(&b.estimated_us))
        });

        // Simulate parallel scheduling.
        let mut slots: Vec<u64> = vec![0; available_concurrent]; // when each slot is free
        let mut results = Vec::with_capacity(tasks.len());

        for task in &sorted {
            // Find the earliest free slot.
            let slot_idx = slots
                .iter()
                .enumerate()
                .min_by_key(|(_, &t)| t)
                .map(|(i, _)| i)
                .unwrap_or(0);

            let start_us = slots[slot_idx].max(task.created_us);
            let end_us = start_us + task.estimated_us;
            slots[slot_idx] = end_us;

            results.push((task.task_id, start_us, end_us));
        }

        // Sort by predicted start time.
        results.sort_by_key(|&(_, start, _)| start);
        results
    }
}

impl Default for PredictiveEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// A single action record — the atomic unit of orchestration telemetry.
#[derive(Debug, Clone)]
pub struct ActionRecord {
    /// Timestamp (monotonic microseconds since orchestrator creation).
    pub timestamp_us: u64,
    /// What kind of action.
    pub category: ActionCategory,
    /// Human-readable action name.
    pub name: String,
    /// Duration in microseconds (0 if still in progress).
    pub duration_us: u64,
    /// Success flag (TriState::Unknown if not yet known).
    pub success: TriState,
    /// Error message (empty on success).
    pub error: String,
    /// Budget consumed (tokens or compute units).
    pub budget_consumed: f64,
    /// SHA3-256 of the action's canonical bytes (for audit trail).
    pub hash: [u8; 32],
}

/// Compute SHA3-256 of an action record's canonical bytes.
fn action_hash(cat: ActionCategory, name: &str, ts: u64) -> [u8; 32] {
    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(&(ts).to_le_bytes());
    buf.extend_from_slice(&(cat as u32).to_le_bytes());
    buf.extend_from_slice(name.as_bytes());
    sha3_256(&buf)
}

/// System health assessment — aggregated from all subsystem signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemHealth {
    /// All subsystems nominal.
    Healthy,
    /// One or more subsystems degraded but operational.
    Degraded,
    /// Critical failure in one or more subsystems.
    Critical,
    /// No data yet (orchestrator just created).
    Unknown,
}

impl fmt::Display for SystemHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemHealth::Healthy => write!(f, "HEALTHY"),
            SystemHealth::Degraded => write!(f, "DEGRADED"),
            SystemHealth::Critical => write!(f, "CRITICAL"),
            SystemHealth::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Load prediction — estimated future load based on recent action history.
#[derive(Debug, Clone)]
pub struct LoadPrediction {
    /// Predicted actions per second (next window).
    pub actions_per_sec: f64,
    /// Predicted budget consumption rate (tokens/sec).
    pub budget_rate: f64,
    /// Predicted concurrent task count.
    pub concurrent_tasks: usize,
    /// Confidence in the prediction (0.0..1.0).
    pub confidence: f64,
    /// Recommended max concurrent based on prediction.
    pub recommended_concurrent: usize,
}

/// Subsystem health signal — one per monitored component.
#[derive(Debug, Clone)]
pub struct HealthSignal {
    /// Subsystem name.
    pub subsystem: String,
    /// Current health status.
    pub status: SystemHealth,
    /// Last error message (empty if healthy).
    pub last_error: String,
    /// Success rate (0.0..1.0) over the recent window.
    pub success_rate: f64,
    /// Average latency in microseconds.
    pub avg_latency_us: u64,
    /// SHA3-256 of this signal's state for verification.
    pub hash: [u8; 32],
}

/// The Orchestrator — central hub for all kernel actions.
///
/// Manages:
/// - Tool invocation routing and metrics
/// - Skill selection and execution tracking
/// - Agent dispatch with workflow gate enforcement
/// - PID-controlled dynamic concurrency
/// - Priority-based task scheduling
/// - Predictive ETA and load forecasting
/// - All state transitions are SHA3-256 verifiable
pub struct Orchestrator {
    /// Action history (ring buffer of recent records).
    history: Vec<ActionRecord>,
    /// Maximum history size.
    max_history: usize,
    /// Budget allocator for the orchestrator.
    budget: TokenBucket,
    /// Workflow gate for the current task.
    gate: WorkflowGate,
    /// Monotonic clock (microseconds since creation).
    #[allow(dead_code)]
    start_us: u64,
    /// Subsystem health signals.
    health_signals: Vec<HealthSignal>,
    /// Current concurrent task count.
    active_tasks: usize,
    /// Running hash chain over all actions (prev_hash || this_hash).
    chain_hash: [u8; 32],
    /// PID controller for dynamic concurrency management.
    pid: PidController,
    /// Priority-based task queue (pending tasks).
    task_queue: Vec<ScheduledTask>,
    /// Monotonic task ID counter (PID-like: unique per task).
    next_task_id: u64,
    /// Predictive engine for ETA and load forecasting.
    prediction: PredictiveEngine,
}

impl Orchestrator {
    /// Create a new orchestrator with a budget and workflow gate.
    pub fn new(budget: TokenBucket) -> Self {
        Orchestrator {
            history: Vec::with_capacity(64),
            max_history: 256,
            budget,
            gate: WorkflowGate::new(),
            start_us: monotonic_us(),
            health_signals: Vec::new(),
            active_tasks: 0,
            chain_hash: [0u8; 32],
            pid: PidController::new_min_max(1, MAX_CONCURRENT),
            task_queue: Vec::new(),
            next_task_id: 1,
            prediction: PredictiveEngine::new(),
        }
    }

    /// Record an action and emit structured telemetry.
    pub fn record_action(
        &mut self,
        category: ActionCategory,
        name: &str,
        duration_us: u64,
        success: bool,
        error: &str,
        budget_consumed: f64,
    ) -> ActionRecord {
        let ts = monotonic_us();
        let hash = action_hash(category, name, ts);

        // Update hash chain.
        let mut chain_input = Vec::with_capacity(64);
        chain_input.extend_from_slice(&self.chain_hash);
        chain_input.extend_from_slice(&hash);
        self.chain_hash = sha3_256(&chain_input);

        let record = ActionRecord {
            timestamp_us: ts,
            category,
            name: name.to_string(),
            duration_us,
            success: TriState::from_bool(success),
            error: error.to_string(),
            budget_consumed,
            hash,
        };

        // Ring buffer: evict oldest if full.
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(record.clone());

        // Feed predictive engine.
        self.prediction.observe(category, duration_us);

        record
    }

    /// Get the current workflow gate state.
    pub fn gate(&self) -> &WorkflowGate {
        &self.gate
    }

    /// Advance the workflow gate.
    pub fn advance_gate(&mut self, phase: GatePhase) -> Result<(), GateError> {
        self.gate.advance(phase)
    }

    /// Can the current task commit?
    pub fn can_commit(&self) -> TriState {
        TriState::from_bool(self.gate.can_commit())
    }

    /// Start a new task (increment active count, check budget).
    pub fn start_task(&mut self) -> Result<(), String> {
        if self.active_tasks >= MAX_CONCURRENT {
            return Err(format!(
                "max concurrent tasks ({}) reached",
                MAX_CONCURRENT
            ));
        }
        if !self.budget.try_acquire(1.0) {
            return Err("budget exhausted".to_string());
        }
        self.active_tasks += 1;
        Ok(())
    }

    /// Finish a task (decrement active count).
    pub fn finish_task(&mut self) {
        if self.active_tasks > 0 {
            self.active_tasks -= 1;
        }
    }

    /// Get the number of active tasks.
    pub fn active_tasks(&self) -> usize {
        self.active_tasks
    }

    /// Can we dispatch more tasks in parallel?
    pub fn can_parallel(&self) -> TriState {
        TriState::from_bool(self.active_tasks < MAX_CONCURRENT)
    }

    /// How many more tasks can we run in parallel.
    pub fn parallel_capacity(&self) -> usize {
        MAX_CONCURRENT.saturating_sub(self.active_tasks)
    }

    /// Compute system health from recent action history.
    pub fn health(&self) -> SystemHealth {
        if self.history.is_empty() {
            return SystemHealth::Unknown;
        }

        let window = self.history.len().min(20);
        let recent = &self.history[self.history.len() - window..];

        let failures = recent.iter().filter(|r| r.success.is_false()).count();
        let failure_rate = failures as f64 / window as f64;

        if failure_rate > 0.5 {
            SystemHealth::Critical
        } else if failure_rate > 0.1 {
            SystemHealth::Degraded
        } else {
            SystemHealth::Healthy
        }
    }

    /// Predict system load based on recent action patterns.
    ///
    /// Uses a simple moving-average model:
    /// - actions_per_sec = recent_count / recent_duration
    /// - budget_rate = recent_budget / recent_duration
    /// - concurrent = current active_tasks
    /// - confidence = min(1.0, recent_count / 10)  (need 10+ samples for confidence)
    pub fn predict_load(&self) -> LoadPrediction {
        if self.history.is_empty() {
            return LoadPrediction {
                actions_per_sec: 0.0,
                budget_rate: 0.0,
                concurrent_tasks: self.active_tasks,
                confidence: 0.0,
                recommended_concurrent: MAX_CONCURRENT,
            };
        }

        let window = self.history.len().min(20);
        let recent = &self.history[self.history.len() - window..];

        // Time span of the window (microseconds).
        let first_ts = recent[0].timestamp_us;
        let last_ts = recent.last().unwrap().timestamp_us;
        let span_us = if last_ts > first_ts {
            (last_ts - first_ts) as f64
        } else {
            1.0 // Avoid division by zero for single-record windows
        };

        let actions_per_sec = (window as f64 / span_us) * 1_000_000.0;
        let total_budget: f64 = recent.iter().map(|r| r.budget_consumed).sum();
        let budget_rate = (total_budget / span_us) * 1_000_000.0;

        // Confidence grows with sample count.
        let confidence = (window as f64 / 10.0).min(1.0);

        // Recommended concurrent: scale down if failure rate is high.
        let failures = recent.iter().filter(|r| r.success.is_false()).count();
        let failure_rate = failures as f64 / window as f64;
        let recommended = if failure_rate > 0.3 {
            1 // Conservative: one task at a time
        } else if failure_rate > 0.1 {
            2
        } else {
            MAX_CONCURRENT
        };

        LoadPrediction {
            actions_per_sec,
            budget_rate,
            concurrent_tasks: self.active_tasks,
            confidence,
            recommended_concurrent: recommended,
        }
    }

    /// Get action history (read-only).
    pub fn history(&self) -> &[ActionRecord] {
        &self.history
    }

    /// The chain hash over all actions (for audit verification).
    pub fn chain_hash(&self) -> [u8; 32] {
        self.chain_hash
    }

    /// Register a subsystem health signal.
    pub fn register_health(&mut self, signal: HealthSignal) {
        // Replace existing signal for the same subsystem.
        if let Some(existing) = self.health_signals.iter_mut().find(|s| s.subsystem == signal.subsystem) {
            *existing = signal;
        } else {
            self.health_signals.push(signal);
        }
    }

    /// Get all health signals.
    pub fn health_signals(&self) -> &[HealthSignal] {
        &self.health_signals
    }

    /// Aggregate health across all registered subsystems.
    pub fn aggregate_health(&self) -> SystemHealth {
        if self.health_signals.is_empty() {
            return self.health();
        }

        let has_critical = self.health_signals.iter().any(|s| s.status == SystemHealth::Critical);
        let has_degraded = self.health_signals.iter().any(|s| s.status == SystemHealth::Degraded);

        if has_critical {
            SystemHealth::Critical
        } else if has_degraded {
            SystemHealth::Degraded
        } else {
            SystemHealth::Healthy
        }
    }

    /// ASCII health dashboard for diagnostics.
    ///
    /// ```text
    /// Orchestrator Health Dashboard
    ///   Status:     HEALTHY
    ///   Active:     3/8 tasks
    ///   History:    42 actions
    ///   Chain:      [a1b2c3d4...]
    ///   Gate:       [x] research [x] synthesis [ ] critique-1 ...
    ///   Load:       12.3 actions/sec, confidence: 0.8
    ///   Subsystems:
    ///     tool_executor  HEALTHY  (98.5% success, 123us avg)
    ///     agent_loop     DEGRADED (85.0% success, 456us avg)
    /// ```
    pub fn ascii_dashboard(&self) -> String {
        let health = self.aggregate_health();
        let load = self.predict_load();
        let gate_status = self.gate().ascii_status();

        let mut out = String::with_capacity(512);
        out.push_str("Orchestrator Health Dashboard\n");
        out.push_str(&format!("  Status:     {}\n", health));
        out.push_str(&format!(
            "  Active:     {}/{} tasks\n",
            self.active_tasks, MAX_CONCURRENT
        ));
        out.push_str(&format!("  History:    {} actions\n", self.history.len()));
        out.push_str(&format!(
            "  Chain:      [{:02x?}...]\n",
            &self.chain_hash[..4]
        ));
        out.push_str(&format!("  Gate:\n{}", gate_status));
        out.push_str(&format!(
            "  Load:       {:.1} actions/sec, confidence: {:.1}\n",
            load.actions_per_sec, load.confidence
        ));

        if !self.health_signals.is_empty() {
            out.push_str("  Subsystems:\n");
            for sig in &self.health_signals {
                out.push_str(&format!(
                    "    {:<20} {}  ({:.1}% success, {}us avg)\n",
                    sig.subsystem,
                    sig.status,
                    sig.success_rate * 100.0,
                    sig.avg_latency_us
                ));
            }
        }

        out
    }

    // ─── PID-Controlled Dynamic Concurrency ─────────────────────────────────

    /// Update the PID controller with the latest performance measurement.
    ///
    /// `target_latency_us` = setpoint (ideal action latency).
    /// `actual_latency_us` = measured latency.
    /// The PID adjusts the recommended concurrency to minimize latency.
    pub fn pid_update(&mut self, target_latency_us: f64, actual_latency_us: f64) -> usize {
        self.pid.update(target_latency_us, actual_latency_us);
        self.pid.recommended()
    }

    /// PID-recommended concurrency limit.
    pub fn pid_recommended_concurrency(&self) -> usize {
        self.pid.recommended()
    }

    /// Effective concurrency: minimum of static MAX_CONCURRENT and PID output.
    pub fn effective_concurrency(&self) -> usize {
        MAX_CONCURRENT.min(self.pid.recommended())
    }

    // ─── Priority Scheduling ───────────────────────────────────────────────

    /// Enqueue a task for priority-based scheduling.
    ///
    /// Returns the assigned task ID (monotonic, PID-like).
    pub fn enqueue_task(
        &mut self,
        category: ActionCategory,
        priority: Priority,
        budget_cost: f64,
        depends_on: Vec<u64>,
    ) -> u64 {
        let task_id = self.next_task_id;
        self.next_task_id += 1;

        let (estimated_us, _ci) = self.prediction.predict_eta(category);

        let task = ScheduledTask {
            task_id,
            category,
            priority,
            estimated_us,
            created_us: monotonic_us(),
            budget_cost,
            depends_on,
        };

        self.task_queue.push(task);
        task_id
    }

    /// Dequeue the highest-priority ready task (no unmet dependencies).
    ///
    /// Returns the task and its predicted ETA. The caller must call
    /// `finish_task` when done.
    pub fn dequeue_ready(&mut self) -> Option<(ScheduledTask, u64)> {
        // Find tasks whose dependencies are all satisfied.
        let completed: std::collections::HashSet<u64> = self
            .history
            .iter()
            .filter(|r| r.success.is_true())
            .map(|_r| {
                // We don't store task_id in ActionRecord, so we approximate:
                // a completed task's deps are considered satisfied.
                0u64 // placeholder — real wiring stores task_id in ActionRecord
            })
            .collect();

        // Filter to tasks with no pending deps.
        let ready_idx = self
            .task_queue
            .iter()
            .enumerate()
            .filter(|(_, t)| t.depends_on.iter().all(|dep| completed.contains(dep)))
            .min_by_key(|(_, t)| {
                // Sort by priority desc, then estimated duration asc.
                std::cmp::Reverse((t.priority, std::cmp::Reverse(t.estimated_us)))
            })
            .map(|(i, _)| i);

        if let Some(idx) = ready_idx {
            let task = self.task_queue.remove(idx);
            let (eta, _ci) = self.prediction.predict_eta(task.category);
            Some((task, eta))
        } else {
            None
        }
    }

    /// Number of tasks waiting in the queue.
    pub fn queue_depth(&self) -> usize {
        self.task_queue.len()
    }

    /// Priority-sorted snapshot of the queue (for diagnostics).
    pub fn queue_snapshot(&self) -> Vec<&ScheduledTask> {
        let mut snap: Vec<&ScheduledTask> = self.task_queue.iter().collect();
        snap.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(a.estimated_us.cmp(&b.estimated_us))
        });
        snap
    }

    // ─── Predictive Engine ─────────────────────────────────────────────────

    /// Record an action's duration for predictive modeling.
    pub fn observe_action(&mut self, category: ActionCategory, duration_us: u64) {
        self.prediction.observe(category, duration_us);
    }

    /// Predict ETA for a given action category.
    pub fn predict_eta(&self, category: ActionCategory) -> (u64, u64) {
        self.prediction.predict_eta(category)
    }

    /// Predict the full schedule for all queued tasks.
    pub fn predict_schedule(&self) -> Vec<(u64, u64, u64)> {
        self.prediction
            .predict_schedule(&self.task_queue, self.effective_concurrency())
    }

    /// ASCII dashboard with PID and scheduling info.
    pub fn ascii_dashboard_full(&self) -> String {
        let health = self.aggregate_health();
        let load = self.predict_load();
        let gate_status = self.gate().ascii_status();

        let mut out = String::with_capacity(768);
        out.push_str("Orchestrator Health Dashboard\n");
        out.push_str(&format!("  Status:     {}\n", health));
        out.push_str(&format!(
            "  Active:     {}/{} tasks (PID recommends {})\n",
            self.active_tasks,
            MAX_CONCURRENT,
            self.pid.recommended()
        ));
        out.push_str(&format!("  Queue:      {} tasks pending\n", self.task_queue.len()));
        out.push_str(&format!("  History:    {} actions\n", self.history.len()));
        out.push_str(&format!(
            "  Chain:      [{:02x?}...]\n",
            &self.chain_hash[..4]
        ));
        out.push_str(&format!("  Gate:\n{}", gate_status));
        out.push_str(&format!(
            "  Load:       {:.1} actions/sec, confidence: {:.1}\n",
            load.actions_per_sec, load.confidence
        ));

        if !self.task_queue.is_empty() {
            out.push_str("  Task Queue (priority-sorted):\n");
            for task in self.queue_snapshot().iter().take(5) {
                out.push_str(&format!(
                    "    [{}] {} est.{}us ${:.1}\n",
                    task.priority.as_str(),
                    task.category.as_str(),
                    task.estimated_us,
                    task.budget_cost
                ));
            }
            if self.task_queue.len() > 5 {
                out.push_str(&format!("    ... +{} more\n", self.task_queue.len() - 5));
            }
        }

        if !self.health_signals.is_empty() {
            out.push_str("  Subsystems:\n");
            for sig in &self.health_signals {
                out.push_str(&format!(
                    "    {:<20} {}  ({:.1}% success, {}us avg)\n",
                    sig.subsystem,
                    sig.status,
                    sig.success_rate * 100.0,
                    sig.avg_latency_us
                ));
            }
        }

        out
    }
}

/// Get monotonic timestamp in microseconds (platform-specific).
fn monotonic_us() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}

/// Build a health signal for a subsystem from recent action records.
pub fn build_health_signal(subsystem: &str, records: &[ActionRecord]) -> HealthSignal {
    if records.is_empty() {
        return HealthSignal {
            subsystem: subsystem.to_string(),
            status: SystemHealth::Unknown,
            last_error: String::new(),
            success_rate: 0.0,
            avg_latency_us: 0,
            hash: sha3_256(subsystem.as_bytes()),
        };
    }

    let total = records.len();
    let successes = records.iter().filter(|r| r.success.is_true()).count();
    let success_rate = successes as f64 / total as f64;
    let total_latency: u64 = records.iter().map(|r| r.duration_us).sum();
    let avg_latency = total_latency / total as u64;

    let last_error = records
        .iter()
        .rev()
        .find(|r| r.success.is_false())
        .map(|r| r.error.clone())
        .unwrap_or_default();

    let status = if success_rate < 0.5 {
        SystemHealth::Critical
    } else if success_rate < 0.9 {
        SystemHealth::Degraded
    } else {
        SystemHealth::Healthy
    };

    // Hash the signal state.
    let mut buf = Vec::with_capacity(128);
    buf.extend_from_slice(subsystem.as_bytes());
    buf.extend_from_slice(&(success_rate.to_bits()).to_le_bytes());
    buf.extend_from_slice(&avg_latency.to_le_bytes());
    let hash = sha3_256(&buf);

    HealthSignal {
        subsystem: subsystem.to_string(),
        status,
        last_error,
        success_rate,
        avg_latency_us: avg_latency,
        hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_budget() -> TokenBucket {
        TokenBucket::new(100.0, 1.0)
    }

    #[test]
    fn new_orchestrator_starts_healthy_unknown() {
        let orch = Orchestrator::new(test_budget());
        assert_eq!(orch.health(), SystemHealth::Unknown);
        assert_eq!(orch.active_tasks(), 0);
        assert!(orch.can_parallel().is_true());
    }

    #[test]
    fn record_action_emits_telemetry() {
        let mut orch = Orchestrator::new(test_budget());
        let record = orch.record_action(
            ActionCategory::ToolRead,
            "read_order",
            150,
            true,
            "",
            1.0,
        );
        assert_eq!(record.category, ActionCategory::ToolRead);
        assert_eq!(record.name, "read_order");
        assert_eq!(record.duration_us, 150);
        assert!(record.success.is_true());
        assert!(record.error.is_empty());
        assert_ne!(record.hash, [0u8; 32]);
    }

    #[test]
    fn health_improves_with_successes() {
        let mut orch = Orchestrator::new(test_budget());
        for _ in 0..10 {
            orch.record_action(ActionCategory::ToolRead, "test", 100, true, "", 0.5);
        }
        assert_eq!(orch.health(), SystemHealth::Healthy);
    }

    #[test]
    fn health_degrades_with_failures() {
        let mut orch = Orchestrator::new(test_budget());
        for _ in 0..10 {
            orch.record_action(ActionCategory::ToolRead, "test", 100, false, "error", 0.5);
        }
        assert_eq!(orch.health(), SystemHealth::Critical);
    }

    #[test]
    fn load_prediction_with_data() {
        let mut orch = Orchestrator::new(test_budget());
        for _ in 0..15 {
            orch.record_action(ActionCategory::ToolRead, "test", 1000, true, "", 1.0);
        }
        let pred = orch.predict_load();
        assert!(pred.actions_per_sec > 0.0);
        assert!(pred.confidence > 0.0);
        assert!(pred.recommended_concurrent > 0);
    }

    #[test]
    fn load_prediction_empty_history() {
        let orch = Orchestrator::new(test_budget());
        let pred = orch.predict_load();
        assert_eq!(pred.actions_per_sec, 0.0);
        assert_eq!(pred.confidence, 0.0);
    }

    #[test]
    fn start_finish_task_tracks_count() {
        let mut orch = Orchestrator::new(test_budget());
        orch.start_task().unwrap();
        assert_eq!(orch.active_tasks(), 1);
        orch.start_task().unwrap();
        assert_eq!(orch.active_tasks(), 2);
        orch.finish_task();
        assert_eq!(orch.active_tasks(), 1);
        orch.finish_task();
        assert_eq!(orch.active_tasks(), 0);
    }

    #[test]
    fn max_concurrent_enforced() {
        let mut orch = Orchestrator::new(test_budget());
        for _ in 0..MAX_CONCURRENT {
            orch.start_task().unwrap();
        }
        assert!(orch.start_task().is_err());
        assert_eq!(orch.parallel_capacity(), 0);
    }

    #[test]
    fn workflow_gate_integration() {
        let mut orch = Orchestrator::new(test_budget());
        assert!(orch.can_commit().is_false());
        orch.advance_gate(GatePhase::Research).unwrap();
        orch.advance_gate(GatePhase::Synthesis).unwrap();
        assert_eq!(orch.gate().completed_count(), 2);
    }

    #[test]
    fn chain_hash_changes_with_actions() {
        let mut orch = Orchestrator::new(test_budget());
        let hash0 = orch.chain_hash();
        orch.record_action(ActionCategory::ToolRead, "a", 10, true, "", 0.0);
        let hash1 = orch.chain_hash();
        assert_ne!(hash0, hash1);
        orch.record_action(ActionCategory::ToolWrite, "b", 20, true, "", 0.0);
        let hash2 = orch.chain_hash();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn ascii_dashboard_contains_sections() {
        let mut orch = Orchestrator::new(test_budget());
        orch.record_action(ActionCategory::ToolRead, "test", 100, true, "", 1.0);
        let dash = orch.ascii_dashboard();
        assert!(dash.contains("Orchestrator Health Dashboard"));
        assert!(dash.contains("Status:"));
        assert!(dash.contains("Active:"));
        assert!(dash.contains("Gate:"));
    }

    #[test]
    fn health_signal_registration() {
        let mut orch = Orchestrator::new(test_budget());
        let signal = HealthSignal {
            subsystem: "tool_executor".to_string(),
            status: SystemHealth::Healthy,
            last_error: String::new(),
            success_rate: 0.95,
            avg_latency_us: 120,
            hash: sha3_256(b"tool_executor"),
        };
        orch.register_health(signal);
        assert_eq!(orch.health_signals().len(), 1);

        // Replace with degraded signal.
        let degraded = HealthSignal {
            subsystem: "tool_executor".to_string(),
            status: SystemHealth::Degraded,
            last_error: "timeout".to_string(),
            success_rate: 0.7,
            avg_latency_us: 500,
            hash: sha3_256(b"tool_executor_degraded"),
        };
        orch.register_health(degraded);
        assert_eq!(orch.health_signals().len(), 1);
        assert_eq!(orch.health_signals()[0].status, SystemHealth::Degraded);
    }

    #[test]
    fn aggregate_health_reflects_subsystems() {
        let mut orch = Orchestrator::new(test_budget());
        orch.register_health(HealthSignal {
            subsystem: "a".to_string(),
            status: SystemHealth::Healthy,
            last_error: String::new(),
            success_rate: 0.99,
            avg_latency_us: 50,
            hash: sha3_256(b"a"),
        });
        assert_eq!(orch.aggregate_health(), SystemHealth::Healthy);

        orch.register_health(HealthSignal {
            subsystem: "b".to_string(),
            status: SystemHealth::Critical,
            last_error: "crash".to_string(),
            success_rate: 0.1,
            avg_latency_us: 1000,
            hash: sha3_256(b"b"),
        });
        assert_eq!(orch.aggregate_health(), SystemHealth::Critical);
    }

    #[test]
    fn build_health_signal_from_records() {
        let records: Vec<ActionRecord> = (0..10)
            .map(|i| ActionRecord {
                timestamp_us: 1000 + i * 100,
                category: ActionCategory::ToolRead,
                name: "test".to_string(),
                duration_us: 100 + i as u64 * 10,
                success: TriState::from_bool(i != 5), // one failure
                error: if i == 5 { "timeout".to_string() } else { String::new() },
                budget_consumed: 1.0,
                hash: action_hash(ActionCategory::ToolRead, "test", 1000 + i * 100),
            })
            .collect();
        let signal = build_health_signal("tool_executor", &records);
        assert_eq!(signal.status, SystemHealth::Healthy); // 90% success
        assert_eq!(signal.success_rate, 0.9);
        assert!(signal.avg_latency_us > 0);
    }

    #[test]
    fn parse_category_exists() {
        assert_eq!(ActionCategory::Parse.as_str(), "parse");
    }

    // ── PID Controller tests ───────────────────────────────────────────────

    #[test]
    fn pid_controller_initial_output_is_max() {
        let pid = PidController::new_min_max(1, 8);
        assert_eq!(pid.recommended(), 8);
    }

    #[test]
    fn pid_controller_reduces_concurrency_on_high_latency() {
        let mut pid = PidController::new_min_max(1, 8);
        // Target: 100us. Actual: 500us (too slow).
        for _ in 0..20 {
            pid.update(100.0, 500.0);
        }
        // PID should reduce concurrency below max.
        assert!(pid.recommended() < 8);
    }

    #[test]
    fn pid_controller_increases_concurrency_on_low_latency() {
        let mut pid = PidController::new_min_max(1, 8);
        // Start at min.
        pid.output = 2.0;
        // Target: 100us. Actual: 10us (very fast — can afford more).
        for _ in 0..20 {
            pid.update(100.0, 10.0);
        }
        assert!(pid.recommended() >= 2);
    }

    #[test]
    fn pid_controller_respects_bounds() {
        let mut pid = PidController::new_min_max(2, 6);
        for _ in 0..100 {
            pid.update(100.0, 0.0); // should push up
        }
        assert!(pid.recommended() <= 6);
        let mut pid2 = PidController::new_min_max(2, 6);
        for _ in 0..100 {
            pid2.update(100.0, 10000.0); // should push down
        }
        assert!(pid2.recommended() >= 2);
    }

    // ── Priority Scheduler tests ───────────────────────────────────────────

    #[test]
    fn priority_ordering() {
        assert!(Priority::Critical > Priority::Parse);
        assert!(Priority::Parse > Priority::Interactive);
        assert!(Priority::Interactive > Priority::Normal);
        assert!(Priority::Normal > Priority::Background);
    }

    #[test]
    fn enqueue_and_dequeue() {
        let mut orch = Orchestrator::new(test_budget());
        let id1 = orch.enqueue_task(ActionCategory::ToolRead, Priority::Normal, 1.0, vec![]);
        let id2 = orch.enqueue_task(ActionCategory::Parse, Priority::Parse, 2.0, vec![]);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(orch.queue_depth(), 2);

        // Dequeue should return Parse first (higher priority).
        let (task, _eta) = orch.dequeue_ready().unwrap();
        assert_eq!(task.category, ActionCategory::Parse);
        assert_eq!(task.priority, Priority::Parse);
        assert_eq!(orch.queue_depth(), 1);

        // Second dequeue returns the remaining task.
        let (task2, _) = orch.dequeue_ready().unwrap();
        assert_eq!(task2.category, ActionCategory::ToolRead);
        assert_eq!(orch.queue_depth(), 0);
    }

    #[test]
    fn queue_snapshot_is_priority_sorted() {
        let mut orch = Orchestrator::new(test_budget());
        orch.enqueue_task(ActionCategory::ToolRead, Priority::Background, 1.0, vec![]);
        orch.enqueue_task(ActionCategory::Parse, Priority::Critical, 1.0, vec![]);
        orch.enqueue_task(ActionCategory::Skill, Priority::Normal, 1.0, vec![]);

        let snap = orch.queue_snapshot();
        assert_eq!(snap[0].priority, Priority::Critical);
        assert_eq!(snap[1].priority, Priority::Normal);
        assert_eq!(snap[2].priority, Priority::Background);
    }

    // ── Predictive Engine tests ────────────────────────────────────────────

    #[test]
    fn predictive_engine_observe_and_predict() {
        let mut engine = PredictiveEngine::new();
        // No data yet — conservative default.
        let (eta, ci) = engine.predict_eta(ActionCategory::ToolRead);
        assert_eq!(eta, 100_000);
        assert_eq!(ci, 50_000);

        // Record observations.
        for _ in 0..10 {
            engine.observe(ActionCategory::ToolRead, 200);
        }
        let (eta, ci) = engine.predict_eta(ActionCategory::ToolRead);
        assert!(eta < 100_000, "should predict faster than default: {}", eta);
        assert!(ci < 50_000, "confidence interval should narrow: {}", ci);
    }

    #[test]
    fn predictive_engine_category_distinction() {
        let mut engine = PredictiveEngine::new();
        for _ in 0..10 {
            engine.observe(ActionCategory::ToolRead, 100);
            engine.observe(ActionCategory::Parse, 500);
        }
        let (read_eta, _) = engine.predict_eta(ActionCategory::ToolRead);
        let (parse_eta, _) = engine.predict_eta(ActionCategory::Parse);
        assert!(read_eta < parse_eta, "reads should be predicted faster");
    }

    #[test]
    fn predictive_schedule_priority_sorted() {
        let engine = PredictiveEngine::new();
        let tasks = vec![
            ScheduledTask {
                task_id: 1,
                category: ActionCategory::ToolRead,
                priority: Priority::Background,
                estimated_us: 100,
                created_us: 0,
                budget_cost: 1.0,
                depends_on: vec![],
            },
            ScheduledTask {
                task_id: 2,
                category: ActionCategory::Parse,
                priority: Priority::Critical,
                estimated_us: 100,
                created_us: 0,
                budget_cost: 1.0,
                depends_on: vec![],
            },
        ];
        let schedule = engine.predict_schedule(&tasks, 2);
        assert_eq!(schedule.len(), 2);
        // Critical task should start first.
        assert_eq!(schedule[0].0, 2);
    }

    // ── Full orchestrator integration tests ─────────────────────────────────

    #[test]
    fn orchestrator_pid_and_prediction_integration() {
        let mut orch = Orchestrator::new(test_budget());

        // Simulate some actions.
        for _ in 0..5 {
            orch.record_action(ActionCategory::Parse, "fetch", 500, true, "", 2.0);
        }

        // PID update based on latency.
        let recommended = orch.pid_update(100.0, 500.0);
        assert!(recommended > 0);

        // Prediction should have data.
        let (eta, _) = orch.predict_eta(ActionCategory::Parse);
        assert!(eta < 100_000, "should predict parse time from observations");

        // Full dashboard should not panic.
        let dash = orch.ascii_dashboard_full();
        assert!(dash.contains("PID recommends"));
        assert!(dash.contains("Queue:"));
    }

    #[test]
    fn orchestrator_full_dashboard_contains_all_sections() {
        let mut orch = Orchestrator::new(test_budget());
        orch.record_action(ActionCategory::Parse, "test", 100, true, "", 1.0);
        orch.enqueue_task(ActionCategory::Parse, Priority::Parse, 1.0, vec![]);
        let dash = orch.ascii_dashboard_full();
        assert!(dash.contains("Orchestrator Health Dashboard"));
        assert!(dash.contains("PID recommends"));
        assert!(dash.contains("Queue:"));
        assert!(dash.contains("Task Queue"));
    }

    // ── edge-case tests ────────────────────────────────────────────────────

    #[test]
    fn dequeue_ready_on_empty_queue_returns_none() {
        let mut orch = Orchestrator::new(test_budget());
        assert_eq!(orch.queue_depth(), 0);
        assert!(orch.dequeue_ready().is_none());
    }

    #[test]
    fn priority_ties_ordered_by_lower_estimated_us() {
        let mut orch = Orchestrator::new(test_budget());
        orch.enqueue_task(ActionCategory::ToolRead, Priority::Normal, 1.0, vec![]);
        // Manually set a second Normal-priority task with a lower estimated_us so it
        // should sort first (ascending estimated_us within same priority).
        let task_id = orch.enqueue_task(
            ActionCategory::ToolWrite,
            Priority::Normal,
            1.0,
            vec![],
        );
        // Override the predicted eta on the second task to be lower.
        if let Some(t) = orch.task_queue.iter_mut().find(|t| t.task_id == task_id) {
            t.estimated_us = 10;
        }
        let snap = orch.queue_snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].priority, Priority::Normal);
        assert_eq!(snap[1].priority, Priority::Normal);
        // The one with lower estimated_us should come first in the tie.
        assert!(snap[0].estimated_us <= snap[1].estimated_us);
    }

    #[test]
    fn start_task_blocks_on_budget_exhaustion() {
        let empty_budget = TokenBucket::new(0.0, 0.0);
        let mut orch = Orchestrator::new(empty_budget);
        assert!(orch.start_task().is_err());
        assert_eq!(orch.active_tasks(), 0);
    }

    #[test]
    fn predict_schedule_empty_tasks_returns_empty() {
        let engine = PredictiveEngine::new();
        let schedule = engine.predict_schedule(&[], 4);
        assert!(schedule.is_empty());
    }

    #[test]
    fn predict_schedule_zero_concurrent_returns_empty() {
        let engine = PredictiveEngine::new();
        let tasks = vec![ScheduledTask {
            task_id: 1,
            category: ActionCategory::ToolRead,
            priority: Priority::Normal,
            estimated_us: 100,
            created_us: 0,
            budget_cost: 1.0,
            depends_on: vec![],
        }];
        let schedule = engine.predict_schedule(&tasks, 0);
        assert!(schedule.is_empty());
    }

    #[test]
    fn schedule_respects_priority_ordering_all_ids_present() {
        let engine = PredictiveEngine::new();
        let tasks = vec![
            ScheduledTask {
                task_id: 1,
                category: ActionCategory::ToolRead,
                priority: Priority::Background,
                estimated_us: 100,
                created_us: 0,
                budget_cost: 1.0,
                depends_on: vec![],
            },
            ScheduledTask {
                task_id: 2,
                category: ActionCategory::Parse,
                priority: Priority::Critical,
                estimated_us: 200,
                created_us: 0,
                budget_cost: 1.0,
                depends_on: vec![],
            },
            ScheduledTask {
                task_id: 3,
                category: ActionCategory::Skill,
                priority: Priority::Normal,
                estimated_us: 150,
                created_us: 0,
                budget_cost: 1.0,
                depends_on: vec![],
            },
        ];
        let schedule = engine.predict_schedule(&tasks, 1);
        assert_eq!(schedule.len(), 3);
        // With 1 concurrent slot, Critical (task_id=2) must start first.
        assert_eq!(schedule[0].0, 2);
        let ids: Vec<u64> = schedule.iter().map(|s| s.0).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(ids.contains(&3));
    }

    #[test]
    fn predict_eta_no_history_returns_conservative_default() {
        let engine = PredictiveEngine::new();
        for cat in &[
            ActionCategory::ToolRead,
            ActionCategory::SwarmFanOut,
            ActionCategory::Verification,
            ActionCategory::AgentDispatch,
        ] {
            let (eta, ci) = engine.predict_eta(*cat);
            assert_eq!(eta, 100_000, "no-history default for {:?}", cat);
            assert_eq!(ci, 50_000, "no-history CI for {:?}", cat);
        }
    }
}
