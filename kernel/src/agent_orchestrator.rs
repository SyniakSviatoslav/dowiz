use crate::delta::{DeltaComparison, DeltaTracker};
use crate::pid::PidController;
use crate::telemetry_harvest::HarvestLedger;
use crate::trinary::Tri;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AgentClass {
    Light,
    Heavy,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskFingerprint {
    pub task_id: String,
    pub file_hashes: Vec<(String, [u8; 32])>,
    pub deps_hash: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    pub fingerprint: TaskFingerprint,
    pub result: Tri,
    pub duration_ms: u64,
    pub agent_class: AgentClass,
    pub timestamp: u64,
}

pub struct TaskOracle {
    history: VecDeque<(String, AgentClass, AgentClass, bool)>,
    accuracy_ema: HashMap<String, f64>,
    class_rules: HashMap<String, AgentClass>,
    classifier_pid: PidController,
    telemetry: HarvestLedger,
}

impl TaskOracle {
    pub fn new() -> Self {
        let mut class_rules = HashMap::new();
        for kw in &[
            "read", "analyze", "search", "grep", "query", "fetch", "lookup",
            "review", "audit", "document", "explain", "summarize", "translate",
            "enrich", "detect", "classify", "intent", "pattern",
        ] {
            class_rules.insert(kw.to_string(), AgentClass::Light);
        }
        for kw in &[
            "compile", "build", "test", "run", "execute", "bench", "fuzz",
            "mutants", "coverage", "link", "assemble", "optimize", "generate",
            "migrate", "transform", "refactor", "rewrite", "restructure",
        ] {
            class_rules.insert(kw.to_string(), AgentClass::Heavy);
        }

        let mut classifier_pid = PidController::new(0.5, 0.1, 0.05, 0.0, 1.0);
        classifier_pid.output = 0.5;

        Self {
            history: VecDeque::with_capacity(1000),
            accuracy_ema: HashMap::new(),
            class_rules,
            classifier_pid,
            telemetry: HarvestLedger::new(500),
        }
    }

    pub fn predict(&self, task_desc: &str) -> (AgentClass, f64) {
        let lower = task_desc.to_lowercase();
        let mut light_score = 0.0_f64;
        let mut heavy_score = 0.0_f64;

        for (kw, class) in &self.class_rules {
            if lower.contains(kw.as_str()) {
                let accuracy = self.accuracy_ema.get(kw).copied().unwrap_or(0.8);
                match class {
                    AgentClass::Light => light_score += accuracy,
                    AgentClass::Heavy => heavy_score += accuracy,
                }
            }
        }

        let total = light_score + heavy_score;
        if total < 0.1 {
            return (AgentClass::Light, 0.5);
        }

        let confidence = light_score / total;
        let class = if confidence >= self.classifier_pid.output() {
            AgentClass::Light
        } else {
            AgentClass::Heavy
        };
        (class, confidence)
    }

    pub fn record_outcome(&mut self, task_desc: &str, predicted: AgentClass, actual: AgentClass) {
        let lower = task_desc.to_lowercase();
        let correct = predicted == actual;

        for (kw, class) in &self.class_rules {
            if lower.contains(kw.as_str()) && *class == predicted {
                let entry = self.accuracy_ema.entry(kw.clone()).or_insert(0.8);
                *entry = 0.9 * *entry + 0.1 * if correct { 1.0 } else { 0.0 };
            }
        }

        let error_input = if correct { 0.5 } else { 1.0 };
        self.classifier_pid.update(0.5, error_input);

        self.history.push_back((task_desc.to_string(), predicted, actual, correct));
        if self.history.len() > 1000 {
            self.history.pop_front();
        }

        self.telemetry.record(
            "task_oracle",
            task_desc,
            correct,
            if correct { 1.0 } else { 0.0 },
            0.0,
        );
    }

    pub fn accuracy(&self) -> f64 {
        if self.history.is_empty() {
            return 1.0;
        }
        let correct = self.history.iter().filter(|(_, _, _, c)| *c).count();
        correct as f64 / self.history.len() as f64
    }
}

pub struct SnapshotCache {
    snapshots: HashMap<String, TaskSnapshot>,
    max_snapshots: usize,
    pub hits: usize,
    pub misses: usize,
}

impl SnapshotCache {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: HashMap::new(),
            max_snapshots,
            hits: 0,
            misses: 0,
        }
    }

    pub fn check(&self, fingerprint: &TaskFingerprint) -> Option<&TaskSnapshot> {
        for snap in self.snapshots.values() {
            if snap.fingerprint == *fingerprint {
                return Some(snap);
            }
        }
        None
    }

    pub fn store(&mut self, snapshot: TaskSnapshot) {
        if self.snapshots.len() >= self.max_snapshots {
            if let Some(oldest) = self
                .snapshots
                .values()
                .min_by_key(|s| s.timestamp)
                .map(|s| s.fingerprint.task_id.clone())
            {
                self.snapshots.remove(&oldest);
            }
        }
        self.snapshots.insert(snapshot.fingerprint.task_id.clone(), snapshot);
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

pub struct WaveOrchestrator {
    pending: VecDeque<(String, String)>,
    oracle: TaskOracle,
    snapshot_cache: SnapshotCache,
    core_load: Vec<usize>,
    current_wave: usize,
    max_light_per_core: usize,
    max_heavy_per_core: usize,
    delta_tracker: DeltaTracker,
    #[allow(dead_code)]
    chronos: crate::chronos::Chronos,
    total_dispatched: usize,
    total_skipped: usize,
    #[allow(dead_code)]
    n_cores: usize,
}

impl WaveOrchestrator {
    pub fn new() -> Self {
        let n_cores = crate::cpuid::detect().cores.max(1);
        Self {
            pending: VecDeque::new(),
            oracle: TaskOracle::new(),
            snapshot_cache: SnapshotCache::new(1000),
            core_load: vec![0; n_cores],
            current_wave: 0,
            max_light_per_core: 8,
            max_heavy_per_core: 2,
            delta_tracker: DeltaTracker::new(1.0, 10.0),
            chronos: crate::chronos::Chronos::new(100),
            total_dispatched: 0,
            total_skipped: 0,
            n_cores,
        }
    }

    pub fn enqueue(&mut self, task_id: &str, description: &str) {
        self.pending.push_back((task_id.to_string(), description.to_string()));
    }

    pub fn fingerprint(&self, task_id: &str) -> TaskFingerprint {
        TaskFingerprint {
            task_id: task_id.to_string(),
            file_hashes: Vec::new(),
            deps_hash: [0u8; 32],
        }
    }

    pub fn dispatch_wave(&mut self) -> (usize, usize) {
        self.current_wave += 1;
        let mut dispatched = 0;
        let mut skipped = 0;

        let n = self.pending.len();
        if n == 0 {
            return (0, 0);
        }

        let mut light_tasks: Vec<(String, String)> = Vec::new();
        let mut heavy_tasks: Vec<(String, String)> = Vec::new();

        let drained: Vec<_> = self.pending.drain(..).collect();

        for (tid, desc) in drained {
            let fp = self.fingerprint(&tid);

            if self.snapshot_cache.check(&fp).is_some() {
                self.snapshot_cache.hits += 1;
                skipped += 1;
                self.total_skipped += 1;
                continue;
            }
            self.snapshot_cache.misses += 1;

            let (class, _conf) = self.oracle.predict(&desc);
            match class {
                AgentClass::Light => light_tasks.push((tid, desc)),
                AgentClass::Heavy => heavy_tasks.push((tid, desc)),
            }
        }

        for (tid, desc) in light_tasks {
            if self.can_dispatch(AgentClass::Light) {
                self.dispatch_one(&tid, &desc, AgentClass::Light);
                dispatched += 1;
            } else {
                self.pending.push_back((tid, desc));
            }
        }

        for (tid, desc) in heavy_tasks {
            if self.can_dispatch(AgentClass::Heavy) {
                self.dispatch_one(&tid, &desc, AgentClass::Heavy);
                dispatched += 1;
            } else {
                self.pending.push_back((tid, desc));
            }
        }

        let prev_balance = self.total_dispatched as f64 - self.total_skipped as f64;
        self.total_dispatched += dispatched;
        let curr_balance = self.total_dispatched as f64 - self.total_skipped as f64;
        self.delta_tracker.observe_transition(
            &[prev_balance],
            crate::now_ms(),
            &[curr_balance],
            crate::now_ms(),
        );

        (dispatched, skipped)
    }

    fn can_dispatch(&self, class: AgentClass) -> bool {
        let max_per_core = match class {
            AgentClass::Light => self.max_light_per_core,
            AgentClass::Heavy => self.max_heavy_per_core,
        };
        self.core_load.iter().any(|&load| load < max_per_core)
    }

    fn dispatch_one(&mut self, _task_id: &str, _description: &str, class: AgentClass) {
        let max_per_core = match class {
            AgentClass::Light => self.max_light_per_core,
            AgentClass::Heavy => self.max_heavy_per_core,
        };

        if let Some(core) = self.core_load.iter().position(|&l| l < max_per_core) {
            self.core_load[core] += 1;
            crate::core_pinning::pin_to_core(core);
        }
    }

    pub fn agent_complete(&mut self, core: usize) {
        if core < self.core_load.len() {
            self.core_load[core] = self.core_load[core].saturating_sub(1);
        }
    }

    pub fn stats(&self) -> WaveStats {
        let delta = if self.delta_tracker.len() >= 1 {
            let last = &self.delta_tracker.history[self.delta_tracker.len() - 1];
            if last.magnitude <= 0.1 {
                DeltaComparison::Stable
            } else {
                let sum: f64 = last.components.iter().sum();
                if sum > 0.1 {
                    DeltaComparison::Growing
                } else if sum < -0.1 {
                    DeltaComparison::Shrinking
                } else {
                    DeltaComparison::Oscillating
                }
            }
        } else {
            DeltaComparison::Stable
        };

        WaveStats {
            total_dispatched: self.total_dispatched,
            total_skipped: self.total_skipped,
            pending: self.pending.len(),
            current_wave: self.current_wave,
            oracle_accuracy: self.oracle.accuracy(),
            cache_hit_rate: self.snapshot_cache.hit_rate(),
            core_load: self.core_load.clone(),
            delta,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WaveStats {
    pub total_dispatched: usize,
    pub total_skipped: usize,
    pub pending: usize,
    pub current_wave: usize,
    pub oracle_accuracy: f64,
    pub cache_hit_rate: f64,
    pub core_load: Vec<usize>,
    pub delta: DeltaComparison,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oracle_predicts_light_for_read_tasks() {
        let oracle = TaskOracle::new();
        let (class, conf) = oracle.predict("read and analyze the codebase");
        assert_eq!(class, AgentClass::Light);
        assert!(conf > 0.5);
    }

    #[test]
    fn oracle_predicts_heavy_for_build_tasks() {
        let oracle = TaskOracle::new();
        let (class, conf) = oracle.predict("compile and build the kernel");
        assert_eq!(class, AgentClass::Heavy);
        assert!(conf < 0.5);
    }

    #[test]
    fn oracle_learns_from_outcomes() {
        let mut oracle = TaskOracle::new();
        let (class, _) = oracle.predict("run tests");
        assert_eq!(class, AgentClass::Heavy);

        oracle.record_outcome("run tests", AgentClass::Heavy, AgentClass::Light);
        for _ in 0..9 {
            oracle.record_outcome("run test suite", AgentClass::Heavy, AgentClass::Light);
        }

        let accuracy = oracle.accuracy();
        assert!(accuracy < 1.0, "Should have learned from mistakes");
    }

    #[test]
    fn snapshot_cache_hit_and_miss() {
        let mut cache = SnapshotCache::new(10);
        let fp = TaskFingerprint {
            task_id: "test1".into(),
            file_hashes: vec![],
            deps_hash: [1u8; 32],
        };
        assert!(cache.check(&fp).is_none());
        cache.store(TaskSnapshot {
            fingerprint: fp.clone(),
            result: Tri::True,
            duration_ms: 100,
            agent_class: AgentClass::Light,
            timestamp: crate::now_ms(),
        });
        assert!(cache.check(&fp).is_some());
    }

    #[test]
    fn wave_orchestrator_dispatches_in_waves() {
        let mut orch = WaveOrchestrator::new();
        orch.enqueue("t1", "read codebase overview");
        orch.enqueue("t2", "compile kernel module");
        orch.enqueue("t3", "analyze performance");
        orch.enqueue("t4", "build release binary");

        let (d1, _) = orch.dispatch_wave();
        assert!(d1 > 0, "First wave should dispatch some tasks");

        let stats = orch.stats();
        assert!(stats.current_wave > 0);
        assert_eq!(stats.pending + stats.total_dispatched + stats.total_skipped, 4);
    }

    #[test]
    fn oracle_default_accuracy_is_high() {
        let oracle = TaskOracle::new();
        assert!(oracle.accuracy() > 0.9, "Initial accuracy should be high");
    }

    #[test]
    fn wave_orchestrator_handles_empty_queue() {
        let mut orch = WaveOrchestrator::new();
        let (d, s) = orch.dispatch_wave();
        assert_eq!((d, s), (0, 0));
    }
}
