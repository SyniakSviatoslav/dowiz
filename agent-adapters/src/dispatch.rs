//! dispatch.rs — the `Dispatcher` pattern generalized for agents: pre-acquire the budget,
//! refuse with a typed error, and emit exactly ONE `TrackRecord` harvest row on EVERY
//! pole (success, backend error, budget refusal, depth refusal).
//!
//! F2 rate-limit is enforced at the same slot as `llm-adapters` `dispatch.rs:82-90`
//! (acquire before the call, typed `BudgetExceeded`). F10 depth is enforced per
//! invocation against the cryptographically-witnessed `invoke_depth`. The harvest row is
//! the SAME schema `gov_route` already folds (`model/task/success/value/cost/backend/
//! tokens/ms`) — one JSONL stream for LLM + agent rows.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use dowiz_kernel::ports::agent::{AgentBridge, AgentError, AgentInvocation, AgentResponse, AgentTask};
use dowiz_kernel::token_bucket::TokenBucket;

use crate::cache::AgentCache;

/// One harvested dispatch record — the SAME superset schema `gov_route` consumes
/// (`model/task/success/value/cost` + richer `backend/tokens/ms`).
#[derive(Debug, Clone, PartialEq)]
pub struct TrackRecord {
    /// Bridge identity (e.g. `"mcp:<server-id>"`).
    pub backend_id: String,
    /// The target the request addressed (quirks-profile id + mapped tool/resource name).
    pub model_id: String,
    /// Budget units of the response (0 on any failure).
    pub total_tokens: u64,
    /// Wall-clock latency in ms.
    pub ms: u64,
    /// Task label the `gov_route` fold groups by (`agent.invoke_tool` | …).
    pub task: String,
    /// Success (1) / failure (0).
    pub success: bool,
    /// Expected value (EV numerator). Default 0.0.
    pub value: f64,
    /// Cost (EV denominator) — budget units consumed/attempted.
    pub cost: f64,
}

impl TrackRecord {
    /// The exact JSONL line shape `gov_route` reads (identical to `llm-adapters`).
    pub fn to_jsonl(&self) -> String {
        format!(
            "{{\"model\":\"{}\",\"task\":\"{}\",\"success\":{},\"value\":{},\"cost\":{},\"backend\":\"{}\",\"tokens\":{},\"ms\":{}}}\n",
            self.model_id, self.task, self.success, self.value, self.cost, self.backend_id, self.total_tokens, self.ms
        )
    }
}

/// Decode one harvest line back into a `TrackRecord` (the `gov_route` fold). Returns
/// `None` on a malformed line (fail-closed; the row is skipped). Used by crit 8 to prove a
/// mixed LLM+agent stream parses with zero schema errors.
pub fn decode_track_record(line: &str) -> Option<TrackRecord> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let b = |k: &str| v.get(k).and_then(|x| x.as_bool()).unwrap_or(false);
    let f = |k: &str| v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
    let u = |k: &str| v.get(k).and_then(|x| x.as_u64()).unwrap_or(0);
    // Require the gov_route-critical keys to be present (schema check).
    for k in ["model", "task", "success", "value", "cost"] {
        v.get(k)?;
    }
    Some(TrackRecord {
        backend_id: s("backend"),
        model_id: s("model"),
        total_tokens: u("tokens"),
        ms: u("ms"),
        task: s("task"),
        success: b("success"),
        value: f("value"),
        cost: f("cost"),
    })
}

/// Where harvest rows go. `FileHarvest` appends to `track_record.jsonl` (M8-local, like
/// `llm-adapters`); tests use `VecHarvest`.
pub trait HarvestSink {
    /// Record exactly one row.
    fn record(&self, row: &TrackRecord);
}

/// Appends to `track_record.jsonl` (telemetry must never break the call path → failures
/// are silently dropped, matching `llm-adapters::append_harvest`).
pub struct FileHarvest;
impl HarvestSink for FileHarvest {
    fn record(&self, row: &TrackRecord) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("track_record.jsonl")
        {
            let _ = f.write_all(row.to_jsonl().as_bytes());
        }
    }
}

/// In-memory harvest capture for tests.
#[derive(Clone, Default)]
pub struct VecHarvest(Arc<Mutex<Vec<TrackRecord>>>);
impl VecHarvest {
    /// Empty capture.
    pub fn new() -> Self {
        VecHarvest(Arc::new(Mutex::new(Vec::new())))
    }
    /// Snapshot the captured rows.
    pub fn rows(&self) -> Vec<TrackRecord> {
        self.0.lock().unwrap().clone()
    }
}
impl HarvestSink for VecHarvest {
    fn record(&self, row: &TrackRecord) {
        self.0.lock().unwrap().push(row.clone());
    }
}

/// Typed dispatch failure — distinct from `AgentError` (the bridge's own failure).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentDispatchError {
    /// Budget exhausted (F2) — degrade-closed.
    BudgetExceeded,
    /// Delegation depth reached the granted cap (F10).
    DepthExceeded,
    /// The bridge itself failed.
    Backend(AgentError),
}

/// A bounded dispatcher over a shared `AgentBridge`, generalizing the `llm-adapters`
/// `Dispatcher`.
pub struct AgentDispatcher<B: AgentBridge, H: HarvestSink> {
    backend: Arc<B>,
    bucket: Arc<TokenBucket>,
    granted_depth: u8,
    sink: H,
    seq: AtomicU64,
    /// This agent's PRIVATE cache instance (SH-2): provisioned per-agent from
    /// `CacheProvisioner`, never shared unless the operator co-scoped it. `None` disables
    /// caching (every call reaches the bridge). Only idempotent reads are cached; tool
    /// invocations are always `NoCache`.
    cache: Option<Arc<AgentCache>>,
}

impl<B: AgentBridge, H: HarvestSink> AgentDispatcher<B, H> {
    /// Build a dispatcher from an admitted bridge + its minted budget envelope + granted
    /// F10 depth (all three come FROM the `AdmissionRecord`).
    pub fn new(backend: Arc<B>, bucket: Arc<TokenBucket>, granted_depth: u8, sink: H) -> Self {
        AgentDispatcher {
            backend,
            bucket,
            granted_depth,
            sink,
            seq: AtomicU64::new(0),
            cache: None,
        }
    }

    /// Attach this agent's PRIVATE cache instance (SH-2 per-agent partitioning). The cache
    /// is provisioned by `CacheProvisioner`; a shared instance appears here ONLY when the
    /// operator co-scoped the agents.
    pub fn with_cache(mut self, cache: Arc<AgentCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Canonical cache key for a cacheable task (`None` ⇒ not cacheable, e.g. tool invoke).
    fn cache_key(task: &AgentTask) -> Option<[u8; 32]> {
        match task {
            AgentTask::ReadResource { uri } => Some(AgentCache::key(uri.as_bytes())),
            AgentTask::RenderPrompt { name } => Some(AgentCache::key(name.as_bytes())),
            AgentTask::InvokeTool { .. } => None, // NoCache
        }
    }

    fn model_id(&self, task: &AgentTask) -> String {
        let name = match task {
            AgentTask::InvokeTool { name, .. } => name.clone(),
            AgentTask::ReadResource { uri } => uri.clone(),
            AgentTask::RenderPrompt { name } => name.clone(),
        };
        format!("{}/{}", self.backend.id(), name)
    }

    /// Dispatch one invocation. Enforces F10 depth, then F2 budget, then calls the bridge.
    /// Emits EXACTLY ONE `TrackRecord` on every pole.
    pub fn dispatch(&self, inv: AgentInvocation) -> Result<AgentResponse, AgentDispatchError> {
        self.seq.fetch_add(1, Ordering::SeqCst);
        let model_id = self.model_id(&inv.task);
        let task_label = inv.task.label().to_string();

        // F10 depth: refuse when the witnessed depth EXCEEDS the grant (a depth-0 direct
        // call is allowed even at granted_depth 0; a sub-agent link is depth ≥1).
        if inv.invoke_depth > self.granted_depth {
            self.harvest(&model_id, &task_label, false, 0, inv.cost_units.max(1), 0);
            return Err(AgentDispatchError::DepthExceeded);
        }

        // SH-2 read-through: a cacheable task served from THIS agent's private cache is
        // free (no budget) and still harvested. A separate-instance cache means a hit here
        // can only reflect THIS agent's (or its operator-co-scoped group's) prior request.
        let ck = self.cache.as_ref().and_then(|_| Self::cache_key(&inv.task));
        if let (Some(cache), Some(key)) = (self.cache.as_ref(), ck) {
            if let Some(content) = cache.get(&key) {
                let units = (content.len() as u64).max(1);
                self.harvest(&model_id, &task_label, true, units, 0, 0);
                return Ok(AgentResponse { content, units });
            }
        }

        // F2 budget: acquire BEFORE the call, typed refusal on shortfall.
        let cost = inv.cost_units.max(1);
        if !self.bucket.try_acquire(cost as f64) {
            self.harvest(&model_id, &task_label, false, 0, cost, 0);
            return Err(AgentDispatchError::BudgetExceeded);
        }

        let t0 = Instant::now();
        let result = self.backend.invoke(&inv);
        let ms = t0.elapsed().as_millis() as u64;
        match result {
            Ok(resp) => {
                // Populate this agent's private cache for cacheable tasks (idempotent put).
                if let (Some(cache), Some(key)) = (self.cache.as_ref(), ck) {
                    cache.put(key, resp.content.clone());
                }
                self.harvest(&model_id, &task_label, true, resp.units, resp.units, ms);
                Ok(resp)
            }
            Err(e) => {
                self.harvest(&model_id, &task_label, false, 0, cost, ms);
                Err(AgentDispatchError::Backend(e))
            }
        }
    }

    fn harvest(&self, model_id: &str, task: &str, success: bool, tokens: u64, cost: u64, ms: u64) {
        self.sink.record(&TrackRecord {
            backend_id: self.backend.id().to_string(),
            model_id: model_id.to_string(),
            total_tokens: tokens,
            ms,
            task: task.to_string(),
            success,
            value: 0.0,
            cost: cost as f64,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dowiz_kernel::ports::agent::{AgentCaps, AgentManifest};

    /// A minimal fake bridge: succeeds, or fails on a designated tool name. Counts invokes
    /// so a cache hit (0 extra backend calls) is falsifiable.
    struct FakeBridge {
        id: String,
        manifest: AgentManifest,
        fail_on: Option<String>,
        calls: std::sync::atomic::AtomicUsize,
    }
    impl FakeBridge {
        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }
    impl AgentBridge for FakeBridge {
        fn id(&self) -> &str {
            &self.id
        }
        fn caps(&self) -> AgentCaps {
            AgentCaps { invoke_tool: true, ..Default::default() }
        }
        fn manifest(&self) -> &AgentManifest {
            &self.manifest
        }
        fn invoke(&self, req: &AgentInvocation) -> Result<AgentResponse, AgentError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if let AgentTask::InvokeTool { name, .. } = &req.task {
                if self.fail_on.as_deref() == Some(name.as_str()) {
                    return Err(AgentError::Refused("boom".into()));
                }
            }
            Ok(AgentResponse { content: b"ok".to_vec(), units: 5 })
        }
        fn health(&self) -> Result<(), AgentError> {
            Ok(())
        }
    }

    fn manifest_stub() -> AgentManifest {
        use dowiz_kernel::ports::agent::{
            Action, BudgetRequest, CostDenomination, ExecutionModel, NodeId, QuirksProfile,
            RefSigner, Resource, Scope, SignatureVerifier, ValidationPolicy,
        };
        let v = RefSigner;
        let cls = v.classical_public(&[1u8; 32]);
        let pq = v.pq_public(&[2u8; 32]);
        AgentManifest {
            agent_node_id: NodeId::from_keys(&pq, &cls).0,
            subject_key: cls,
            subject_key_pq: pq,
            agent_caps: AgentCaps { invoke_tool: true, ..Default::default() },
            action_scopes: Scope::single(Resource::Menu, Action::Read),
            resource_needs: vec![],
            cost_denomination: CostDenomination::TokenBucketUnits,
            budget_request: BudgetRequest { capacity: 10, refill_milli_units_per_sec: 0 },
            validation_policy: ValidationPolicy::RequireBoth,
            execution_model: ExecutionModel::WasmComponent,
            config_axes: vec![],
            depth_request: 0,
            quirks_profile: QuirksProfile::McpServer,
            nonce: [0u8; 8],
            expiry: 9999,
        }
    }

    fn bridge(fail_on: Option<&str>) -> Arc<FakeBridge> {
        Arc::new(FakeBridge {
            id: "mcp:demo".into(),
            manifest: manifest_stub(),
            fail_on: fail_on.map(|s| s.to_string()),
            calls: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    fn invoke(name: &str, cost: u64, depth: u8) -> AgentInvocation {
        AgentInvocation {
            task: AgentTask::InvokeTool { name: name.into(), args: vec![] },
            cost_units: cost,
            invoke_depth: depth,
        }
    }

    // ── §4 criterion 8 — harvest is total (one row per call, mixed schema folds) ──
    #[test]
    fn crit8_every_pole_appends_exactly_one_row() {
        let sink = VecHarvest::new();
        // capacity 6, no refill: first success (5) fits; a second 5-cost call refuses.
        let bucket = Arc::new(TokenBucket::new(6 as f64, 0.0));
        let d = AgentDispatcher::new(bridge(Some("bad")), bucket, 3, sink.clone());

        // 1) success
        assert!(d.dispatch(invoke("good", 5, 0)).is_ok());
        // 2) backend error (fail_on="bad") — but budget: cost 1 fits (1 left). 6-5=1.
        assert!(matches!(
            d.dispatch(invoke("bad", 1, 0)),
            Err(AgentDispatchError::Backend(_))
        ));
        // 3) budget refusal — cost 5, only 0 left.
        assert!(matches!(
            d.dispatch(invoke("good", 5, 0)),
            Err(AgentDispatchError::BudgetExceeded)
        ));

        let rows = sink.rows();
        assert_eq!(rows.len(), 3, "exactly one row per call (success, error, refusal)");
        assert!(rows[0].success && rows[0].total_tokens == 5);
        assert!(!rows[1].success, "backend error row recorded");
        assert!(!rows[2].success && rows[2].total_tokens == 0, "budget refusal row recorded");

        // Mixed LLM+agent fold parses with zero schema errors.
        let llm_line = "{\"model\":\"ollama\",\"task\":\"chat\",\"success\":true,\"value\":0,\"cost\":42,\"backend\":\"ollama\",\"tokens\":42,\"ms\":10}";
        assert!(decode_track_record(llm_line).is_some(), "LLM-shaped row folds");
        for r in &rows {
            assert!(decode_track_record(r.to_jsonl().trim()).is_some(), "agent row folds");
        }
    }

    // ── SH-2 wiring: read-through against THIS agent's private cache instance ────
    #[test]
    fn read_resource_served_from_private_cache_on_second_call() {
        let sink = VecHarvest::new();
        let bucket = Arc::new(TokenBucket::new(1_000 as f64, 0.0));
        let b = bridge(None);
        let cache = Arc::new(AgentCache::new());
        let d = AgentDispatcher::new(b.clone(), bucket, 3, sink).with_cache(cache);
        let inv = || AgentInvocation {
            task: AgentTask::ReadResource { uri: "res://menu".into() },
            cost_units: 3,
            invoke_depth: 0,
        };
        let r1 = d.dispatch(inv()).expect("first read hits backend");
        let r2 = d.dispatch(inv()).expect("second read served from cache");
        assert_eq!(r1.content, r2.content);
        assert_eq!(b.calls(), 1, "second identical read is a cache hit (backend hit once)");
    }

    // ── §4 criterion 5 — F10 depth fires; delegate=false refused at depth 1 ──────
    #[test]
    fn crit5_depth_cap_fires() {
        let sink = VecHarvest::new();
        let bucket = Arc::new(TokenBucket::new(1_000 as f64, 0.0));
        // granted_depth 0 (delegate=false): a depth-0 DIRECT call succeeds…
        let d0 = AgentDispatcher::new(bridge(None), bucket.clone(), 0, sink.clone());
        assert!(d0.dispatch(invoke("good", 1, 0)).is_ok(), "direct call (depth 0) allowed");
        // …but a sub-agent link (depth 1) is refused.
        assert_eq!(
            d0.dispatch(invoke("good", 1, 1)),
            Err(AgentDispatchError::DepthExceeded),
            "delegate=false ⇒ refused at depth 1"
        );
        // granted_depth 3: depth 3 allowed, depth 4 refused (deeper than the cap).
        let d3 = AgentDispatcher::new(bridge(None), bucket, 3, sink);
        assert!(d3.dispatch(invoke("good", 1, 3)).is_ok(), "depth 3 within cap");
        assert_eq!(
            d3.dispatch(invoke("good", 1, 4)),
            Err(AgentDispatchError::DepthExceeded),
            "any chain deeper than 3 is refused"
        );
    }
}
