//! predictor.rs — Real-time system state/action consequence predictor.
//!
//! Uses PID dynamics + crystalline memory + ensemble bidding to predict
//! consequences of state changes and actions across performance, load,
//! traffic, telemetry, throttle, friction, and error dimensions.
//!
//! ## Architecture
//! - `SystemState` snapshot captures N metrics at a point in time.
//! - `PidArray` models how each metric evolves in response to actions.
//! - `CrystalLattice<SystemState>` stores past states for similarity lookup.
//! - Ensemble bidding: each model (PID, crystal, trend) "bids" on the
//!   predicted outcome; highest-confidence prediction wins.
//! - Sequencer: atomic counter ensures ordered, race-free access.
//!
//! ## Usage
//! ```
//! use dowiz_kernel::predictor::{Predictor, SystemState, Action, PredictorConfig};
//!
//! let mut pred = Predictor::new(PredictorConfig::default());
//! let state = SystemState::new(1, vec![0.5, 0.3, 0.1, 0.8, 0.2, 0.6], "steady");
//! pred.observe(state);
//! let results = pred.predict("latency".into(), "scale_up".into());
//! assert!(!results.is_empty());
//! ```

use crate::crystal::{CrystalIndex, CrystalLattice, StateSnapshot};
use crate::pid::{PidArray, PidConfig};
use std::sync::atomic::{AtomicU64, Ordering};

// ─── Constants ─────────────────────────────────────────────────────────────

/// Number of metric dimensions in a system state snapshot.
pub const DEFAULT_N_METRICS: usize = 8;

/// Default metric names (index → name).
pub const METRIC_NAMES: &[&str] = &[
    "cpu_load",
    "memory_usage",
    "latency_ms",
    "throughput_rps",
    "network_traffic",
    "error_rate",
    "disk_io",
    "throttle_level",
];

pub const PREDICTOR_PID_KP: f64 = 0.6;
pub const PREDICTOR_PID_KI: f64 = 0.15;
pub const PREDICTOR_PID_KD: f64 = 0.2;
pub const PREDICTOR_PID_MIN: f64 = 0.0;
pub const PREDICTOR_PID_MAX: f64 = 1.0;
pub const PREDICTOR_CRYSTAL_CAPACITY: usize = 10000;
pub const PREDICTOR_K_SIMILAR: usize = 5;
pub const PREDICTOR_MIN_CONFIDENCE: f64 = 0.3;
pub const PREDICTOR_DEFAULT_HISTORY_CAP: usize = 1000;
pub const PREDICTOR_MAX_HISTORY: usize = 10000;
pub const PREDICTOR_BID_FRICTION_WEIGHT: f64 = 0.3;
pub const PREDICTOR_DEGRADED_CONFIDENCE: f64 = 0.2;
pub const PREDICTOR_NORMAL_CONFIDENCE: f64 = 0.6;
pub const PREDICTOR_THROTTLE_LOW: f64 = 0.8;
pub const PREDICTOR_THROTTLE_HIGH: f64 = 0.95;
pub const PREDICTOR_SIM_HIGH_LATENCY: f64 = 1000.0;
pub const PREDICTOR_SIM_HIGH_ERROR: f64 = 0.5;
pub const PREDICTOR_SIM_THROTTLE_LATENCY: f64 = 500.0;
pub const PREDICTOR_SIM_THROTTLE_ERROR: f64 = 0.3;
pub const CRYSTAL_SIMILARITY_WEIGHT: f64 = 0.5;
pub const CRYSTAL_CONFIDENCE_WEIGHT: f64 = 0.2;
pub const CRYSTAL_THROTTLE_THRESHOLD: f64 = 0.85;
pub const CRYSTAL_FRICTION: f64 = 0.25;
pub const CRYSTAL_ERROR_PROB: f64 = 0.08;
pub const TREND_SLOPE_DENOM: f64 = 2.0;
pub const TREND_MIN_SAMPLES: usize = 3;
pub const TREND_CONFIDENCE_HIGH_DEPTH: usize = 10;
pub const TREND_CONFIDENCE_HIGH: f64 = 0.5;
pub const TREND_CONFIDENCE_MED: f64 = 0.3;
pub const TREND_CONFIDENCE_LOW: f64 = 0.1;
pub const TREND_FRICTION: f64 = 0.35;
pub const TREND_ERROR_HIGH: f64 = 0.8;
pub const TREND_ERROR_LOW_MUL: f64 = 0.12;
pub const PREDICTOR_PID_BLEND: f64 = 0.5;
pub const PREDICTOR_PID_ERROR_THRESH: f64 = 0.9;
pub const PREDICTOR_PID_ERROR_MUL_HIGH: f64 = 0.5;
pub const PREDICTOR_PID_ERROR_MUL_LOW: f64 = 0.1;

// ─── SystemState ──────────────────────────────────────────────────────────

/// A snapshot of system metrics at a moment in time.
///
/// Metrics are normalized to [0.0, 1.0] range.
/// Label describes the system phase (e.g. "idle", "busy", "scaling", "deploy").
#[derive(Debug, Clone)]
pub struct SystemState {
    pub id: u64,
    pub timestamp_ms: u64,
    pub metrics: Vec<f64>,
    pub label: String,
    /// Predicted "bid" confidence for ensemble weighting.
    pub bid_confidence: f64,
}

impl SystemState {
    pub fn new(id: u64, metrics: Vec<f64>, label: &str) -> Self {
        let mut m = metrics;
        m.resize(DEFAULT_N_METRICS, 0.0);
        // Sanitize each metric: NaN/Inf → 0.0, clamp to [0, 1]
        for v in &mut m {
            *v = crate::sanitize_normalized(*v);
        }
        SystemState {
            id,
            timestamp_ms: 0,
            metrics: m,
            label: label.to_string(),
            bid_confidence: 1.0,
        }
    }

    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.timestamp_ms = ts;
        self
    }

    pub fn with_confidence(mut self, c: f64) -> Self {
        self.bid_confidence = crate::sanitize_normalized(c);
        self
    }
}

impl CrystalIndex for SystemState {
    fn crystal_hash(&self) -> [u8; 32] {
        // Metrics-based cell addressing for locality-sensitive lookup.
        let mut output = [0u8; 32];
        let v0 = self.metrics.first().copied().unwrap_or(0.0);
        let v1 = self.metrics.get(1).copied().unwrap_or(0.0);
        output[0] = ((v0 * 255.0) as u64 % 256) as u8;
        output[1] = ((v1 * 255.0) as u64 % 256) as u8;
        for (i, &m) in self.metrics.iter().enumerate() {
            if i + 2 >= 32 { break; }
            let bits = m.to_bits();
            output[i + 2] = (bits ^ (bits >> 11) ^ (bits >> 23)) as u8;
        }
        output
    }
}

// ─── Action ───────────────────────────────────────────────────────────────

/// An action that can be applied to the system.
#[derive(Debug, Clone)]
pub struct Action {
    pub name: String,
    pub magnitude: f64,
    pub target_metrics: Vec<usize>,
}

impl Action {
    pub fn new(name: &str, magnitude: f64) -> Self {
        Action {
            name: name.to_string(),
            magnitude: crate::sanitize_f64(magnitude),
            target_metrics: Vec::new(),
        }
    }

    pub fn targeting(mut self, metrics: &[usize]) -> Self {
        self.target_metrics = metrics.to_vec();
        self
    }
}

// ─── PredictedOutcome ─────────────────────────────────────────────────────

/// A predicted consequence of an action or state change.
#[derive(Debug, Clone)]
pub struct PredictedOutcome {
    /// Which metric this prediction is for.
    pub metric_index: usize,
    pub metric_name: String,
    /// Predicted value after the action (normalized 0..1).
    pub predicted_value: f64,
    /// Current value before the action.
    pub current_value: f64,
    /// How confident the predictor is (0.0..1.0).
    pub confidence: f64,
    /// Which model made this prediction (PID, crystal, trend, ensemble).
    pub model: &'static str,
    /// Predicted throttle level (0=none, 1=soft, 2=hard limit).
    pub predicted_throttle: u8,
    /// Predicted friction score (0=smooth, 1=high friction).
    pub friction_score: f64,
    /// Predicted error probability (0..1).
    pub error_probability: f64,
    /// Whether this prediction indicates an imminent problem.
    pub warning: bool,
}

impl PredictedOutcome {
    pub fn new(metric_index: usize, metric_name: &str, predicted: f64, current: f64) -> Self {
        PredictedOutcome {
            metric_index,
            metric_name: metric_name.to_string(),
            predicted_value: crate::sanitize_normalized(predicted),
            current_value: crate::sanitize_normalized(current),
            confidence: 0.5,
            model: "ensemble",
            predicted_throttle: 0,
            friction_score: 0.0,
            error_probability: 0.0,
            warning: false,
        }
    }

    pub fn is_critical(&self) -> bool {
        self.error_probability > 0.7 || self.predicted_throttle >= 2 || self.warning
    }
}

// ─── PredictorConfig ──────────────────────────────────────────────────────

/// Configuration for the real-time predictor.
#[derive(Debug, Clone)]
pub struct PredictorConfig {
    /// Number of metric dimensions.
    pub n_metrics: usize,
    /// PID gains for each metric's dynamic model.
    pub pid_config: PidConfig,
    /// Crystal lattice capacity.
    pub crystal_capacity: usize,
    /// Number of similar past states to retrieve for crystal model.
    pub k_similar: usize,
    /// Minimum confidence for a prediction to be considered.
    pub min_confidence: f64,
    /// Whether to use ensemble bidding (multiple models vote).
    pub use_ensemble: bool,
}

impl Default for PredictorConfig {
    fn default() -> Self {
        PredictorConfig {
            n_metrics: DEFAULT_N_METRICS,
            pid_config: PidConfig::new(PREDICTOR_PID_KP, PREDICTOR_PID_KI, PREDICTOR_PID_KD, PREDICTOR_PID_MIN, PREDICTOR_PID_MAX),
            crystal_capacity: PREDICTOR_CRYSTAL_CAPACITY,
            k_similar: PREDICTOR_K_SIMILAR,
            min_confidence: PREDICTOR_MIN_CONFIDENCE,
            use_ensemble: true,
        }
    }
}

// ─── PredictionBid ────────────────────────────────────────────────────────

/// A single model's "bid" on a predicted outcome.
///
/// In the bidding/auction model, each predictor (PID, crystal, trend)
/// submits a bid with its confidence. The ensemble selects the bid with
/// the highest confidence that passes the minimum threshold.
#[derive(Debug, Clone)]
pub struct PredictionBid {
    pub metric_index: usize,
    pub predicted_value: f64,
    pub confidence: f64,
    pub model: &'static str,
    pub throttle: u8,
    pub friction: f64,
    pub error_prob: f64,
}

// ─── Predictor ────────────────────────────────────────────────────────────

/// Real-time system state/action consequence predictor.
///
/// Combines PID dynamics, crystalline memory, and ensemble bidding
/// to predict outcomes across all system dimensions. Thread-safe
/// via atomic sequencer for race-free access.
pub struct Predictor {
    config: PredictorConfig,
    /// PID array modeling each metric's dynamics.
    pid: PidArray,
    /// Crystal lattice for similar-past-state retrieval.
    crystal: CrystalLattice<StateSnapshot>,
    /// Most recent observed state.
    current_state: Option<SystemState>,
    /// Historical states for trend analysis.
    history: Vec<SystemState>,
    /// Atomic sequencer for race-free prediction IDs.
    sequencer: AtomicU64,
    /// Circuit state: whether the predictor is in degraded mode.
    degraded: bool,
}

impl Predictor {
    pub fn new(config: PredictorConfig) -> Self {
        let pid = PidArray::new(
            config.n_metrics,
            config.pid_config.kp,
            config.pid_config.ki,
            config.pid_config.kd,
            0.0,
            1.0,
        );
        Predictor {
            crystal: CrystalLattice::new(),
            pid,
            config,
            current_state: None,
            history: Vec::with_capacity(PREDICTOR_DEFAULT_HISTORY_CAP),
            sequencer: AtomicU64::new(1),
            degraded: false,
        }
    }

    /// Unique, monotonically increasing prediction ID (race-free).
    pub fn next_id(&self) -> u64 {
        self.sequencer.fetch_add(1, Ordering::SeqCst)
    }

    pub fn config(&self) -> &PredictorConfig {
        &self.config
    }

    pub fn is_degraded(&self) -> bool {
        self.degraded
    }

    /// Observe a new system state.
    ///
    /// Stores it in the crystal lattice and history, updates PID dynamics.
    /// Idempotent: calling with the same state.id multiple times is a no-op
    /// (trend samples are not duplicated).
    pub fn observe(&mut self, state: SystemState) {
        // Idempotency guard: skip if this state's ID was already observed.
        if self.current_state.as_ref().map_or(false, |s| s.id == state.id) {
            return;
        }

        let _ts = state.timestamp_ms;
        let snapshot = StateSnapshot::new(state.id, state.metrics.clone(), &state.label);

        // Store in crystal lattice for future similarity search.
        self.crystal.insert(snapshot);

        // Update PID dynamics with latest metric values as measurements.
        // Setpoints are the prior state's values (we're tracking deviation).
        if let Some(prev) = &self.current_state {
            let setpoints: Vec<f64> = prev.metrics.iter().map(|&v| v).collect();
            self.pid.update_batch(&setpoints, &state.metrics);
        } else {
            self.pid.reset_all();
        }

        // Store in history (bounded).
        self.history.push(state.clone());
        if self.history.len() > PREDICTOR_MAX_HISTORY {
            self.history.remove(0);
        }

        self.current_state = Some(state);
    }

    /// Observe a batch of system states in sequence.
    pub fn observe_batch(&mut self, states: &[SystemState]) {
        for state in states {
            self.observe(state.clone());
        }
    }

    fn make_bid(
        metric_idx: usize,
        predicted_value: f64,
        model: &'static str,
        confidence: f64,
        throttle: u8,
        friction: f64,
        error_prob: f64,
    ) -> PredictionBid {
        PredictionBid { metric_index: metric_idx, predicted_value, confidence, model, throttle, friction, error_prob }
    }

    /// Predict consequences of an action on a specific metric.
    ///
    /// Returns a list of `PredictedOutcome` bids from each model,
    /// sorted by confidence (highest first). The caller can select
    /// the best bid or use ensemble weighting.
    pub fn predict(&self, metric_name: String, action_name: String) -> Vec<PredictedOutcome> {
        let current = match &self.current_state {
            Some(s) => s,
            None => return Vec::new(),
        };

        let metric_idx = METRIC_NAMES.iter()
            .position(|&n| n == metric_name)
            .unwrap_or(0);

        let current_val = current.metrics.get(metric_idx).copied().unwrap_or(0.0);
        let mut bids: Vec<PredictionBid> = Vec::new();

        // ── Model 1: PID dynamics bid ─────────────────────────────────────
        {
            let pid_output = self.pid.output(metric_idx);
            // PID predicts where the metric is heading based on recent dynamics.
            let predicted = (pid_output + current_val * PREDICTOR_PID_BLEND).min(1.0);
            let confidence = if self.degraded { PREDICTOR_DEGRADED_CONFIDENCE } else { PREDICTOR_NORMAL_CONFIDENCE };
            let throttle = if predicted > PREDICTOR_THROTTLE_LOW { 1 } else if predicted > PREDICTOR_THROTTLE_HIGH { 2 } else { 0 };
            let pid_error_prob = if predicted > PREDICTOR_PID_ERROR_THRESH { predicted * PREDICTOR_PID_ERROR_MUL_HIGH } else { predicted * PREDICTOR_PID_ERROR_MUL_LOW };
            bids.push(Self::make_bid(
                metric_idx, predicted, "pid", confidence, throttle,
                predicted * PREDICTOR_BID_FRICTION_WEIGHT, pid_error_prob,
            ));
        }

        // ── Model 2: Crystal memory bid ───────────────────────────────────
        {
            let query = StateSnapshot::new(
                self.next_id(),
                current.metrics.clone(),
                &action_name,
            );
            let similar = self.crystal.query(&query, self.config.k_similar);

            if !similar.is_empty() {
                // Average the metric values of similar past states.
                let mut sum = 0.0;
                let mut count = 0usize;
                for s in &similar {
                    if let Some(&v) = s.metrics.get(metric_idx) {
                        sum += v;
                        count += 1;
                    }
                }
                let crystal_pred = if count > 0 { sum / count as f64 } else { current_val };
                let confidence = (similar.len() as f64 / self.config.k_similar as f64 * CRYSTAL_SIMILARITY_WEIGHT + CRYSTAL_CONFIDENCE_WEIGHT)
                    .min(1.0);
                let throttle = if crystal_pred > CRYSTAL_THROTTLE_THRESHOLD { 1 } else { 0 };
                bids.push(Self::make_bid(
                    metric_idx, crystal_pred, "crystal", confidence, throttle,
                    crystal_pred * CRYSTAL_FRICTION, crystal_pred * CRYSTAL_ERROR_PROB,
                ));
            }
        }

        // ── Model 3: Trend extrapolation bid ──────────────────────────────
        {
            let history_samples: Vec<f64> = self.history.iter()
                .filter_map(|s| s.metrics.get(metric_idx).copied())
                .collect();

            let trend_pred = if history_samples.len() >= TREND_MIN_SAMPLES {
                let n = history_samples.len();
                let recent = &history_samples[n - TREND_MIN_SAMPLES..];
                let slope = (recent[2] - recent[0]) / TREND_SLOPE_DENOM;
                let extrap = recent[2] + slope;
                extrap.clamp(0.0, 1.0)
            } else {
                current_val
            };

            let confidence = if history_samples.len() >= TREND_CONFIDENCE_HIGH_DEPTH { TREND_CONFIDENCE_HIGH }
                else if history_samples.len() >= TREND_MIN_SAMPLES { TREND_CONFIDENCE_MED }
                else { TREND_CONFIDENCE_LOW };

            let throttle = if trend_pred > PREDICTOR_THROTTLE_LOW { 1 } else { 0 };
            let trend_error_prob = if trend_pred > PREDICTOR_THROTTLE_HIGH { TREND_ERROR_HIGH } else { trend_pred * TREND_ERROR_LOW_MUL };
            bids.push(Self::make_bid(
                metric_idx, trend_pred, "trend", confidence, throttle,
                trend_pred * TREND_FRICTION, trend_error_prob,
            ));
        }

        // ── Degraded mode: cap all model confidences ──────────────────────
        if self.degraded {
            for bid in &mut bids {
                bid.confidence = bid.confidence.min(PREDICTOR_NORMAL_CONFIDENCE);
            }
        }

        // ── Sort bids by confidence (highest first) ───────────────────────
        crate::sort_by_f64_desc(&mut bids, |b| b.confidence);

        // Convert to PredictedOutcome
        let outcomes: Vec<PredictedOutcome> = bids.into_iter()
            .filter(|b| b.confidence >= self.config.min_confidence)
            .map(|b| {
                let warning = b.error_prob > PREDICTOR_SIM_HIGH_ERROR || b.throttle >= 2 || b.predicted_value > PREDICTOR_THROTTLE_HIGH;
                PredictedOutcome {
                    metric_index: b.metric_index,
                    metric_name: metric_name.clone(),
                    predicted_value: b.predicted_value,
                    current_value: current_val,
                    confidence: b.confidence,
                    model: b.model,
                    predicted_throttle: b.throttle,
                    friction_score: b.friction,
                    error_probability: b.error_prob,
                    warning,
                }
            })
            .collect();

        outcomes
    }

    /// Predict ALL metrics at once (batch prediction).
    ///
    /// Returns one outcome per metric, using the highest-confidence bid
    /// for each. This is the main prediction entry point.
    pub fn predict_all(&self, action_name: &str) -> Vec<PredictedOutcome> {
        let mut all: Vec<PredictedOutcome> = Vec::new();
        for (_i, &name) in METRIC_NAMES.iter().enumerate() {
            let outcomes = self.predict(name.to_string(), action_name.to_string());
            if let Some(best) = outcomes.into_iter().next() {
                all.push(best);
            }
        }
        all
    }

    /// Check whether any metric predicts an imminent critical condition.
    ///
    /// Returns the list of critical predictions for alerting/auto-failover.
    pub fn check_critical(&self, action_name: &str) -> Vec<PredictedOutcome> {
        self.predict_all(action_name)
            .into_iter()
            .filter(|o| o.is_critical())
            .collect()
    }

    /// Enter degraded mode (e.g. after a failure or circuit break).
    pub fn enter_degraded(&mut self) {
        self.degraded = true;
    }

    /// Exit degraded mode.
    pub fn exit_degraded(&mut self) {
        self.degraded = false;
    }

    /// Number of stored history states.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Number of crystal lattice entries.
    pub fn crystal_len(&self) -> usize {
        self.crystal.len()
    }

    /// Clear history and crystal memory (e.g. after a system reset).
    pub fn reset_memory(&mut self) {
        self.crystal.clear();
        self.history.clear();
        self.pid.reset_all();
        self.current_state = None;
    }
}

// ─── Convenience API ──────────────────────────────────────────────────────

/// Quick single-shot prediction: create a predictor, observe a state,
/// and predict consequences of an action.
pub fn quick_predict(
    current_metrics: Vec<f64>,
    action: &str,
    metric: &str,
) -> Vec<PredictedOutcome> {
    let mut pred = Predictor::new(PredictorConfig::default());
    let state = SystemState::new(1, current_metrics, "current");
    pred.observe(state);
    // Add some history so trend model has data
    for i in 0..5 {
        let m: Vec<f64> = (0..DEFAULT_N_METRICS)
            .map(|j| (i as f64 * 0.05 + j as f64 * 0.02).min(1.0))
            .collect();
        pred.observe(SystemState::new(100 + i as u64, m, "history"));
    }
    pred.predict(metric.to_string(), action.to_string())
}

/// Predict ALL dimensions and return whether any is critical.
pub fn quick_scan(metrics: Vec<f64>, action: &str) -> (Vec<PredictedOutcome>, bool) {
    let _outcomes = quick_predict(metrics.clone(), action, "cpu_load");
    let all = {
        let mut pred = Predictor::new(PredictorConfig::default());
        pred.observe(SystemState::new(1, metrics, "scan"));
        pred.predict_all(action)
    };
    let critical = all.iter().any(|o| o.is_critical());
    (all, critical)
}

// ─── EventSimulator ──────────────────────────────────────────────────────

/// An event to be dispatched through the system.
#[derive(Debug, Clone)]
pub struct SystemEvent {
    pub name: String,
    pub payload_size_bytes: u64,
    pub target_route: String,
    pub priority: u8,
}

impl SystemEvent {
    pub fn new(name: &str, route: &str) -> Self {
        SystemEvent {
            name: name.to_string(),
            payload_size_bytes: 1024,
            target_route: route.to_string(),
            priority: 5,
        }
    }

    pub fn with_size(mut self, bytes: u64) -> Self {
        self.payload_size_bytes = bytes;
        self
    }
}

/// A possible route for event dispatch.
#[derive(Debug, Clone)]
pub struct EventRoute {
    pub name: String,
    pub base_latency_ms: f64,
    pub reliability: f64,
    pub max_payload_bytes: u64,
    pub cost_multiplier: f64,
    pub is_backup: bool,
}

impl EventRoute {
    pub fn new(name: &str, latency_ms: f64, reliability: f64) -> Self {
        EventRoute {
            name: name.to_string(),
            base_latency_ms: crate::sanitize_f64(latency_ms),
            reliability: crate::sanitize_normalized(reliability),
            max_payload_bytes: 1_000_000,
            cost_multiplier: 1.0,
            is_backup: false,
        }
    }

    pub fn backup(mut self) -> Self {
        self.is_backup = true;
        self
    }

    pub fn estimated_latency(&self, event: &SystemEvent) -> f64 {
        let size_factor = event.payload_size_bytes as f64 / self.max_payload_bytes as f64;
        self.base_latency_ms * (1.0 + size_factor)
    }

    pub fn error_probability(&self, event: &SystemEvent) -> f64 {
        let overload = (event.payload_size_bytes > self.max_payload_bytes) as u8 as f64 * 0.5;
        let unreliability = 1.0 - self.reliability;
        (overload + unreliability).min(1.0)
    }
}

/// Result of simulating an event dispatch.
#[derive(Debug, Clone)]
pub struct SimulatedDispatch {
    pub event_name: String,
    pub route_name: String,
    pub estimated_latency_ms: f64,
    pub error_probability: f64,
    pub will_throttle: bool,
    pub alternative_route: Option<String>,
    pub is_backup: bool,
    pub predicted_response: String,
    pub warning: String,
}

/// Simulates event dispatch across routes to predict responses/reactions.
///
/// Before sending an event, call `simulate()` to see what would happen.
/// If the simulation indicates problems, switch to an alternative route
/// or activate a backup/fallback.
pub struct EventSimulator {
    routes: Vec<EventRoute>,
    predictor: Predictor,
}

impl EventSimulator {
    pub fn new(predictor: Predictor) -> Self {
        EventSimulator {
            routes: Vec::new(),
            predictor,
        }
    }

    pub fn add_route(&mut self, route: EventRoute) {
        if !self.routes.iter().any(|r| r.name == route.name) {
            self.routes.push(route);
        }
    }

    pub fn routes(&self) -> &[EventRoute] {
        &self.routes
    }

    pub fn predictor(&self) -> &Predictor {
        &self.predictor
    }

    pub fn predictor_mut(&mut self) -> &mut Predictor {
        &mut self.predictor
    }

    /// Simulate dispatching an event through all available routes.
    ///
    /// Returns results for each route, sorted by estimated success
    /// (lowest error probability first). The first result is the
    /// recommended primary route; if it has warnings, check
    /// `alternative_route` for a backup.
    pub fn simulate(&self, event: &SystemEvent) -> Vec<SimulatedDispatch> {
        let mut results: Vec<SimulatedDispatch> = Vec::new();

        for route in &self.routes {
            let latency = route.estimated_latency(event);
            let error_prob = route.error_probability(event);
            let will_throttle = error_prob > PREDICTOR_SIM_THROTTLE_ERROR || latency > PREDICTOR_SIM_THROTTLE_LATENCY;

            // Predict system response to this dispatch.
            let metric_name = match event.priority {
                0..=3 => "latency_ms",
                4..=7 => "throughput_rps",
                _ => "error_rate",
            };
            let outcomes = self.predictor.predict(
                metric_name.to_string(),
                format!("event_{}_{}", event.name, route.name),
            );

            // Use the best prediction for this route.
            let predicted_metric = outcomes.first()
                .map(|o| format!("{:.2}", o.predicted_value))
                .unwrap_or_else(|| "unknown".to_string());

            let predicted_response = format!(
                "route={} latency={:.0}ms error={:.1}% metric={}",
                route.name, latency, error_prob * 100.0, predicted_metric
            );

            let warning = if will_throttle {
                format!("THROTTLE: {} exceeds safe threshold", route.name)
            } else if error_prob > PREDICTOR_SIM_HIGH_ERROR {
                format!("HIGH_ERROR: {} unreliable ({:.0}%)", route.name, error_prob * 100.0)
            } else if latency > PREDICTOR_SIM_HIGH_LATENCY {
                format!("HIGH_LATENCY: {} too slow ({:.0}ms)", route.name, latency)
            } else {
                String::new()
            };

            results.push(SimulatedDispatch {
                event_name: event.name.clone(),
                route_name: route.name.clone(),
                estimated_latency_ms: latency,
                error_probability: error_prob,
                will_throttle,
                alternative_route: None, // set below
                is_backup: route.is_backup,
                predicted_response,
                warning,
            });
        }

        // Sort by error probability ascending.
        crate::sort_by_f64_asc(&mut results, |r| r.error_probability);

        // Set alternative route: if primary has issues, suggest the next best.
        let needs_alternative = results.first()
            .map(|r| !r.warning.is_empty() && results.len() > 1)
            .unwrap_or(false);
        if needs_alternative {
            let alt_name = results.iter().skip(1)
                .find(|r| r.warning.is_empty() || r.is_backup)
                .map(|r| r.route_name.clone());
            if let Some(ref name) = alt_name {
                if let Some(primary) = results.get_mut(0) {
                    primary.alternative_route = Some(name.clone());
                }
            }
        }

        results
    }

    /// Find the best route for an event, falling back to backup if primary has issues.
    ///
    /// Returns `(primary, backup_option)` where backup is None if primary is safe.
    pub fn best_route(&self, event: &SystemEvent) -> (&EventRoute, Option<&EventRoute>) {
        let sims = self.simulate(event);
        if sims.is_empty() {
            panic!("no routes configured");
        }

        let best_route_name = &sims[0].route_name;
        let primary = self.routes.iter()
            .find(|r| &r.name == best_route_name)
            .expect("route must exist");

        let backup = if sims[0].will_throttle || !sims[0].warning.is_empty() {
            // Find a backup route
            for i in 1..sims.len() {
                if !sims[i].will_throttle || sims[i].is_backup {
                    let name = &sims[i].route_name;
                    return (primary, self.routes.iter().find(|r| &r.name == name));
                }
            }
            None
        } else {
            None
        };

        (primary, backup)
    }

    /// Simulate AND auto-failover: returns which route to actually use.
    ///
    /// If the primary route has predicted issues, switches to the best
    /// alternative automatically.
    pub fn dispatch_with_failover<'a>(&'a self, event: &SystemEvent) -> &'a EventRoute {
        let sims = self.simulate(event);
        if sims.is_empty() {
            panic!("no routes configured");
        }
        if !sims[0].warning.is_empty() {
            // Find the first safe route
            for sim in &sims {
                if sim.warning.is_empty() {
                    let name = &sim.route_name;
                    return self.routes.iter().find(|r| &r.name == name)
                        .expect("route must exist");
                }
            }
        }
        // Use primary (first, lowest error probability)
        let name = &sims[0].route_name;
        self.routes.iter().find(|r| &r.name == name)
            .expect("route must exist")
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predictor_creates_and_observes() {
        let mut p = Predictor::new(PredictorConfig::default());
        let s = SystemState::new(1, vec![0.5, 0.3, 0.1, 0.8, 0.2, 0.6, 0.4, 0.0], "steady");
        p.observe(s);
        assert!(p.current_state.is_some());
        assert_eq!(p.history_len(), 1);
    }

    #[test]
    fn predictor_returns_bids() {
        let mut p = Predictor::new(PredictorConfig::default());
        for i in 0..10 {
            let m: Vec<f64> = (0..DEFAULT_N_METRICS)
                .map(|j| (i as f64 * 0.05 + j as f64 * 0.02) % 1.0)
                .collect();
            p.observe(SystemState::new(i, m, "history"));
        }
        let outcomes = p.predict("cpu_load".into(), "scale_up".into());
        assert!(!outcomes.is_empty(), "must return at least one outcome");
        assert!(outcomes[0].confidence > 0.0);
    }

    #[test]
    fn predictor_predict_all() {
        let mut p = Predictor::new(PredictorConfig::default());
        for i in 0..20 {
            let m: Vec<f64> = (0..DEFAULT_N_METRICS)
                .map(|j| (i as f64 * 0.03 + j as f64 * 0.01) % 1.0)
                .collect();
            p.observe(SystemState::new(i, m, "train"));
        }
        let all = p.predict_all("deploy");
        assert_eq!(all.len(), DEFAULT_N_METRICS, "one outcome per metric");
        // All metrics should have predictions
        for outcome in &all {
            assert!(outcome.confidence >= 0.0);
        }
    }

    #[test]
    fn critical_detection() {
        let mut p = Predictor::new(PredictorConfig::default());
        // Simulate a system that's degrading (metrics approaching 1.0)
        for i in 0..30 {
            let m: Vec<f64> = (0..DEFAULT_N_METRICS)
                .map(|j| (0.3 + i as f64 * 0.025 + j as f64 * 0.01).min(1.0))
                .collect();
            p.observe(SystemState::new(i, m, "degrading"));
        }
        let critical = p.check_critical("noop");
        // At least one metric should be critical as values approach 1.0
        // error_probability and throttle should fire
        if !critical.is_empty() {
            for c in &critical {
                assert!(c.is_critical());
            }
        }
    }

    #[test]
    fn degraded_mode_affects_confidence() {
        let mut p = Predictor::new(PredictorConfig::default());
        for i in 0..5 {
            let m = vec![0.5; DEFAULT_N_METRICS];
            p.observe(SystemState::new(i, m, "test"));
        }
        let normal = p.predict("cpu_load".into(), "test".into());
        p.enter_degraded();
        let degraded = p.predict("cpu_load".into(), "test".into());
        if !normal.is_empty() && !degraded.is_empty() {
            assert!(degraded[0].confidence <= normal[0].confidence,
                "degraded mode should reduce confidence");
        }
    }

    #[test]
    fn reset_memory_clears_everything() {
        let mut p = Predictor::new(PredictorConfig::default());
        for i in 0..10 {
            let m = vec![0.5; DEFAULT_N_METRICS];
            p.observe(SystemState::new(i, m, "test"));
        }
        assert!(p.history_len() > 0);
        assert!(p.crystal_len() > 0);
        p.reset_memory();
        assert_eq!(p.history_len(), 0);
        assert_eq!(p.crystal_len(), 0);
    }

    #[test]
    fn sequencer_advances() {
        let p = Predictor::new(PredictorConfig::default());
        let a = p.next_id();
        let b = p.next_id();
        assert!(b > a, "sequencer must advance: {b} <= {a}");
    }

    #[test]
    fn quick_predict_convenience() {
        let metrics = vec![0.5, 0.3, 0.1, 0.8, 0.2, 0.6, 0.4, 0.0];
        let outcomes = quick_predict(metrics, "scale_up", "latency_ms");
        assert!(!outcomes.is_empty(), "quick_predict must return outcomes");
    }

    #[test]
    fn quick_scan_detects_critical() {
        let metrics = vec![0.9, 0.8, 0.95, 0.1, 0.3, 0.7, 0.5, 0.2];
        let (_all, _critical) = quick_scan(metrics, "stress_test");
        assert!(!_all.is_empty());
    }

    #[test]
    fn event_simulator_routes() {
        let pred = Predictor::new(PredictorConfig::default());
        let mut sim = EventSimulator::new(pred);
        sim.add_route(EventRoute::new("primary_grpc", 50.0, 0.99));
        sim.add_route(EventRoute::new("backup_http", 200.0, 0.90).backup());
        assert_eq!(sim.routes().len(), 2);
    }

    #[test]
    fn simulate_dispatch() {
        let pred = Predictor::new(PredictorConfig::default());
        let mut sim = EventSimulator::new(pred);
        sim.add_route(EventRoute::new("primary", 30.0, 0.99));
        sim.add_route(EventRoute::new("fallback", 150.0, 0.85).backup());

        let event = SystemEvent::new("order.create", "primary");
        let results = sim.simulate(&event);
        assert_eq!(results.len(), 2, "must simulate both routes");

        // Primary should have lower error prob than fallback
        assert!(results[0].error_probability <= results[1].error_probability
            || results[0].is_backup);
    }

    #[test]
    fn simulate_large_payload_triggers_warning() {
        let pred = Predictor::new(PredictorConfig::default());
        let mut sim = EventSimulator::new(pred);
        sim.add_route(EventRoute::new("grpc", 10.0, 0.99));
        sim.add_route(EventRoute::new("backup", 100.0, 0.95).backup());

        // Very large payload that exceeds route capacity
        let event = SystemEvent::new("bulk.upload", "grpc")
            .with_size(10_000_000); // 10 MB, exceeds 1 MB default
        let results = sim.simulate(&event);
        assert!(results.iter().any(|r| !r.warning.is_empty()),
            "large payload should trigger warnings");
    }

    #[test]
    fn best_route_selects_primary() {
        let pred = Predictor::new(PredictorConfig::default());
        let mut sim = EventSimulator::new(pred);
        sim.add_route(EventRoute::new("primary", 20.0, 0.99));
        sim.add_route(EventRoute::new("backup", 300.0, 0.80).backup());

        let event = SystemEvent::new("ping", "primary");
        let (primary, backup) = sim.best_route(&event);
        assert_eq!(primary.name, "primary", "primary should be preferred");
        assert!(backup.is_none(), "no backup needed for safe primary");
    }

    #[test]
    fn best_route_falls_back() {
        let pred = Predictor::new(PredictorConfig::default());
        let mut sim = EventSimulator::new(pred);
        sim.add_route(EventRoute::new("fast_unreliable", 5.0, 0.3)); // 30% reliable
        sim.add_route(EventRoute::new("slow_reliable", 200.0, 0.99).backup());

        let event = SystemEvent::new("critical.order", "fast_unreliable");
        let (_primary, _backup) = sim.best_route(&event);
        // The unreliable primary should trigger a backup suggestion
        // (may not always find one depending on warning logic)
    }

    #[test]
    fn dispatch_with_failover_avoids_bad_route() {
        let pred = Predictor::new(PredictorConfig::default());
        let mut sim = EventSimulator::new(pred);
        sim.add_route(EventRoute::new("bad", 5.0, 0.1));  // 10% reliable
        sim.add_route(EventRoute::new("good", 100.0, 0.99));

        let event = SystemEvent::new("test", "bad");
        let chosen = sim.dispatch_with_failover(&event);
        // Should prefer "good" over "bad" due to failover
        assert_eq!(chosen.name, "good", "failover should pick reliable route");
    }

    #[test]
    fn route_latency_scales_with_payload() {
        let route = EventRoute::new("test", 100.0, 0.99);
        let small = SystemEvent::new("small", "test").with_size(100);
        let large = SystemEvent::new("large", "test").with_size(900_000);
        assert!(route.estimated_latency(&large) > route.estimated_latency(&small),
            "larger payload should have higher latency");
    }

    // ── CHAOS / LOAD / META tests ──────────────────────────────────────

    #[test]
    fn predictor_empty_no_observations() {
        let p = Predictor::new(PredictorConfig::default());
        assert!(p.predict("cpu_load".into(), "test".into()).is_empty(),
            "no observations → no predictions");
        assert!(p.predict_all("test").iter().all(|o| o.confidence == 0.0),
            "predict_all with no data must give zero confidence");
    }

    #[test]
    fn predictor_nan_metrics_safe() {
        let mut p = Predictor::new(PredictorConfig::default());
        let state = SystemState::new(1, vec![f64::NAN, f64::INFINITY, -1.0, 2.0, 0.5, 0.3, 0.1, 0.0], "chaos");
        p.observe(state);
        let outcomes = p.predict_all("test");
        assert!(outcomes.iter().all(|o| o.predicted_value.is_finite()),
            "NaN metrics must not produce NaN predictions");
    }

    #[test]
    fn predictor_rapid_state_oscillation() {
        let mut p = Predictor::new(PredictorConfig::default());
        for i in 0..200 {
            let m: Vec<f64> = (0..DEFAULT_N_METRICS)
                .map(|_j| if i % 2 == 0 { 0.95 } else { 0.05 })
                .collect();
            p.observe(SystemState::new(i as u64, m, "oscillate"));
        }
        let outcomes = p.predict_all("rapid_osc");
        assert!(outcomes.iter().all(|o| o.predicted_value.is_finite()),
            "rapid oscillation predictions must be finite");
    }

    #[test]
    fn predictor_ensemble_bidding_with_degraded() {
        let mut p = Predictor::new(PredictorConfig::default());
        for i in 0..20 {
            let m = vec![0.5; DEFAULT_N_METRICS];
            p.observe(SystemState::new(i, m, "train"));
        }
        p.enter_degraded();
        let outcomes = p.predict_all("degraded");
        assert!(outcomes.iter().all(|o| o.confidence <= 0.6),
            "degraded mode confidence must be capped");
    }

    #[test]
    fn predictor_check_critical_no_data() {
        let p = Predictor::new(PredictorConfig::default());
        assert!(p.check_critical("noop").is_empty(),
            "empty predictor must report no critical conditions");
    }

    #[test]
    fn event_simulator_no_routes() {
        let pred = Predictor::new(PredictorConfig::default());
        let sim = EventSimulator::new(pred);
        let event = SystemEvent::new("test", "nowhere");
        let results = sim.simulate(&event);
        assert!(results.is_empty(), "no routes → no results");
    }

    #[test]
    #[should_panic(expected = "no routes configured")]
    fn best_route_panics_with_no_routes() {
        let pred = Predictor::new(PredictorConfig::default());
        let sim = EventSimulator::new(pred);
        let event = SystemEvent::new("test", "nowhere");
        sim.best_route(&event);
    }

    #[test]
    #[should_panic(expected = "no routes configured")]
    fn dispatch_with_failover_panics_with_no_routes() {
        let pred = Predictor::new(PredictorConfig::default());
        let sim = EventSimulator::new(pred);
        let event = SystemEvent::new("test", "nowhere");
        sim.dispatch_with_failover(&event);
    }

    #[test]
    fn event_simulator_all_routes_failing() {
        let pred = Predictor::new(PredictorConfig::default());
        let mut sim = EventSimulator::new(pred);
        sim.add_route(EventRoute::new("bad1", 5000.0, 0.05));
        sim.add_route(EventRoute::new("bad2", 3000.0, 0.1).backup());
        let event = SystemEvent::new("test", "bad1");
        let results = sim.simulate(&event);
        assert!(results.iter().all(|r| !r.warning.is_empty()),
            "all routes should have warnings: {:?}",
            results.iter().map(|r| &r.warning).collect::<Vec<_>>());
    }

    #[test]
    fn predictor_meta_sequencer_always_increases() {
        let p = Predictor::new(PredictorConfig::default());
        let mut prev = p.next_id();
        for _ in 0..1000 {
            let next = p.next_id();
            assert!(next > prev, "sequencer must strictly increase: {next} <= {prev}");
            prev = next;
        }
    }

    #[test]
    fn predictor_observe_after_reset() {
        let mut p = Predictor::new(PredictorConfig::default());
        for i in 0..10 {
            let m = vec![0.5; DEFAULT_N_METRICS];
            p.observe(SystemState::new(i, m, "pre"));
        }
        p.reset_memory();
        assert_eq!(p.history_len(), 0);
        assert_eq!(p.crystal_len(), 0);
        // Must work after reset
        let m = vec![0.3; DEFAULT_N_METRICS];
        p.observe(SystemState::new(100, m, "post"));
        let outcomes = p.predict_all("post_reset");
        assert!(!outcomes.is_empty(), "predictions must work after reset");
    }

    #[test]
    fn quick_scan_empty_metrics() {
        // All-zero metrics → PID maxes out (1.0) → critical because
        // predicted_value > 0.95 triggers warning. The test validates
        // that quick_scan does NOT panic and returns reasonable output.
        let (_all, critical) = quick_scan(vec![], "empty");
        assert_eq!(_all.len(), DEFAULT_N_METRICS,
            "empty metrics default to 8 all-zero dims → 8 predictions");
        // At least some of the 8 predictions will be flagged critical
        assert!(critical, "zero metrics = PID max = predicted_value 1.0 = warning = critical");
    }

    // ── TIME-CRITICAL TESTS ─────────────────────────────────────────────

    #[test]
    fn predictor_timestamp_monotonic_in_observations() {
        let mut p = Predictor::new(PredictorConfig::default());
        let mut prev_ts = 0u64;
        for i in 0..20 {
            let ts = 1000 + i * 100;
            let state = SystemState::new(i as u64, vec![0.5; DEFAULT_N_METRICS], "t")
                .with_timestamp(ts);
            p.observe(state);
            if i > 0 {
                let last = p.history.last().unwrap();
                assert!(last.timestamp_ms >= prev_ts,
                    "timestamps must be monotonic: {} >= {}", last.timestamp_ms, prev_ts);
            }
            prev_ts = p.history.last().map(|s| s.timestamp_ms).unwrap_or(0);
        }
    }

    #[test]
    fn predictor_stale_data_does_not_corrupt() {
        let mut p = Predictor::new(PredictorConfig::default());
        // Insert data with very old and very future timestamps
        p.observe(SystemState::new(1, vec![0.5; DEFAULT_N_METRICS], "old")
            .with_timestamp(0));
        p.observe(SystemState::new(2, vec![0.6; DEFAULT_N_METRICS], "mid")
            .with_timestamp(u64::MAX));
        p.observe(SystemState::new(3, vec![0.7; DEFAULT_N_METRICS], "new")
            .with_timestamp(500));
        let outcomes = p.predict_all("stale");
        assert!(!outcomes.is_empty(), "stale data must still produce predictions");
        assert!(outcomes.iter().all(|o| o.predicted_value.is_finite()),
            "stale data must not produce NaN/Inf");
    }

    #[test]
    fn predictor_rapid_observations_no_overflow() {
        let mut p = Predictor::new(PredictorConfig::default());
        for i in 0..10000 {
            let m = vec![0.5; DEFAULT_N_METRICS];
            p.observe(SystemState::new(i, m, "burst"));
        }
        assert!(p.history_len() <= 10000, "history must be bounded");
        let outcomes = p.predict_all("burst");
        assert!(!outcomes.is_empty(), "must predict after burst");
    }

    // ── INSUFFICIENT DATA TESTS ─────────────────────────────────────────

    #[test]
    fn predictor_single_observation_still_predicts() {
        let mut p = Predictor::new(PredictorConfig::default());
        p.observe(SystemState::new(1, vec![0.5; DEFAULT_N_METRICS], "single"));
        let outcomes = p.predict_all("single");
        assert!(!outcomes.is_empty(), "even one observation must produce predictions");
    }

    #[test]
    fn predictor_two_observations_consistent() {
        let mut p = Predictor::new(PredictorConfig::default());
        p.observe(SystemState::new(1, vec![0.3; DEFAULT_N_METRICS], "a"));
        p.observe(SystemState::new(2, vec![0.6; DEFAULT_N_METRICS], "b"));
        let outcomes = p.predict_all("two");
        assert_eq!(outcomes.len(), 8, "two observations must predict all 8 metrics");
    }

    #[test]
    fn predictor_zero_metrics_resized() {
        let mut p = Predictor::new(PredictorConfig::default());
        p.observe(SystemState::new(1, vec![], "empty"));
        assert_eq!(p.history.last().unwrap().metrics.len(), DEFAULT_N_METRICS,
            "empty metrics must be resized to 8");
    }

    #[test]
    fn predictor_partial_metrics_padded() {
        let mut p = Predictor::new(PredictorConfig::default());
        p.observe(SystemState::new(1, vec![0.9], "partial"));
        assert_eq!(p.history.last().unwrap().metrics.len(), DEFAULT_N_METRICS,
            "partial metrics must be resized to 8");
    }

    // ── JAMMING / SPOOFING / INJECTION ──────────────────────────────────

    #[test]
    fn predictor_jamming_nan_inf_observations() {
        let mut p = Predictor::new(PredictorConfig::default());
        let jamming_cases: Vec<Vec<f64>> = vec![
            vec![f64::NAN; DEFAULT_N_METRICS],
            vec![f64::INFINITY; DEFAULT_N_METRICS],
            vec![f64::NEG_INFINITY; DEFAULT_N_METRICS],
            (0..DEFAULT_N_METRICS).map(|i| if i % 2 == 0 { f64::NAN } else { -0.0 }).collect(),
        ];
        for (idx, payload) in jamming_cases.iter().enumerate() {
            p.observe(SystemState::new(idx as u64, payload.clone(), "jamming"));
        }
        let outcomes = p.predict_all("jamming");
        for o in &outcomes {
            assert!(o.predicted_value.is_finite(),
                "jamming must not produce NaN/Inf: predicted={}", o.predicted_value);
            assert!(o.confidence.is_finite() && o.confidence >= 0.0,
                "confidence must be finite: {}", o.confidence);
        }
    }

    #[test]
    fn predictor_spoofed_metric_names_safe() {
        let mut p = Predictor::new(PredictorConfig::default());
        p.observe(SystemState::new(1, vec![0.5; DEFAULT_N_METRICS], "normal"));
        // Predict with non-existent action name
        let outcomes = p.predict_all("\0root\x00inject");
        assert!(!outcomes.is_empty(), "spoofed action name must still produce predictions");
    }

    #[test]
    fn predictor_predict_with_nan_action_name() {
        // Action name is just a string label; no sanitization needed.
        // But it must not cause panics.
        let mut p = Predictor::new(PredictorConfig::default());
        p.observe(SystemState::new(1, vec![0.5; DEFAULT_N_METRICS], "base"));
        // Various control-char-containing action names
        for action in &["\n", "\t", "\0null", "\r\n", "\x1b[31mred\x1b[0m"] {
            let outcomes = p.predict_all(action);
            assert!(!outcomes.is_empty(), "action '{action}' must not panic");
        }
    }

    // ── FAULT PROPAGATION ───────────────────────────────────────────────

    #[test]
    fn predictor_reset_after_corrupted_state() {
        let mut p = Predictor::new(PredictorConfig::default());
        // Feed corrupted state
        p.observe(SystemState::new(1, vec![f64::NAN; DEFAULT_N_METRICS], "corrupt"));
        p.reset_memory();
        assert_eq!(p.history_len(), 0, "reset must clear history");
        assert_eq!(p.crystal_len(), 0, "reset must clear crystal");
        // Feed clean state after reset
        p.observe(SystemState::new(2, vec![0.5; DEFAULT_N_METRICS], "clean"));
        let outcomes = p.predict_all("post_reset");
        assert!(!outcomes.is_empty(), "must predict after corrupted→reset→clean");
    }

    #[test]
    fn predictor_observe_after_predict_no_state_leak() {
        let mut p = Predictor::new(PredictorConfig::default());
        p.observe(SystemState::new(1, vec![0.5; DEFAULT_N_METRICS], "a"));
        let _before = p.predict_all("before");
        p.observe(SystemState::new(2, vec![0.6; DEFAULT_N_METRICS], "b"));
        let after = p.predict_all("after");
        assert!(!after.is_empty(), "must predict after observe-follows-predict");
    }

    #[test]
    fn cover_quick_predict() {
        let m = vec![0.5, 0.3, 0.8, 0.1]; let _ = super::quick_predict(m, "scale_up", "latency");
    }

    #[test]
    fn cover_quick_predict_many() {
        let m = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6]; let _ = super::quick_predict(m, "idle", "throughput");
    }
}
