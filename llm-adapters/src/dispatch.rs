//! dispatch.rs — WAVE 1(d): `TokenBucket`(F33)-bounded, `std::thread` dispatcher (§4.2).
//!
//! Bounds concurrency on LLM calls WITHOUT tokio. A fixed-size worker pool pulls jobs off an
//! `mpsc` channel; each worker does `bucket.try_acquire(cost)` → `false` ⇒ returns
//! `Err(DispatchError::BudgetExceeded)` immediately (degrade-closed, never silently queued-then-
//! downgraded); `true` ⇒ calls the (blocking) backend and ships the result back over the caller's
//! `Sender`. N workers ≤ the backend's own parallelism cap (e.g. 2 for Ollama, §4.1), so the
//! harness's own queue is where back-pressure becomes visible — not Ollama's `MAX_QUEUE` 503.
//!
//! Every dispatched call emits an H1 harvest row (`track_record.jsonl`) priced by the backend's
//! returned `usage.total_tokens`, closing the EV loop so `gov_route` can price local-vs-managed.

use dowiz_kernel::ports::llm::{ChatRequest, ChatResponse, LlmBackend, LlmError};
use dowiz_kernel::token_bucket::TokenBucket;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;

/// Typed dispatch failure — distinct from `LlmError` (the backend's own failure).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    /// Budget exhausted — degrade-closed, the caller must handle (don't retry-silently).
    BudgetExceeded,
    /// All worker slots busy — the concurrency cap (HARNESS audit A4, degrade-closed).
    /// Refused immediately rather than queued-and-downgraded; the caller retries/backs off.
    Busy,
    /// The backend itself failed (health/chat error surfaced through).
    Backend(LlmError),
}

/// One harvested dispatch record (H1 EV ledger row).
///
/// Schema is a SUPERSET of what `tools/telemetry/governance.sh::gov_route` consumes
/// (`model`, `task`, `success`, `value`, `cost`) so the same row feeds the EV pricing loop
/// directly — closing the local-vs-managed loop without a schema-migration shim. The extra
/// fields (`backend`, `tokens`, `ms`) are richer analytics kept in the same row.
#[derive(Debug, Clone)]
pub struct TrackRecord {
    /// Dispatcher-side identity of the backend (e.g. "ollama").
    pub backend_id: String,
    /// The model the request targeted (the `model` key gov_route groups by).
    pub model_id: String,
    /// Total tokens of the response (0 on failure).
    pub total_tokens: u64,
    /// Wall-clock latency of the (blocking) call in ms.
    pub ms: u64,
    /// Task class label gov_route folds by (default "chat"; a caller may pass a richer label).
    pub task: String,
    /// Whether the call succeeded (1) or failed (0) — drives gov_route's success-rate.
    pub success: bool,
    /// Expected value of the call (EV numerator). Default 0.0; the caller/agent loop may supply it.
    pub value: f64,
    /// Cost of the call (EV denominator). For local Ollama we proxy cost ≈ total_tokens.
    pub cost: f64,
}

/// A bounded dispatcher over a shared `LlmBackend`.
pub struct Dispatcher<B: LlmBackend + Send + Sync + 'static> {
    backend: Arc<B>,
    bucket: Arc<TokenBucket>,
    /// Concurrency cap (HARNESS audit A4): at most `workers` calls may be
    /// in-flight at once. `try_acquire` refuses immediately when exhausted
    /// (degrade-closed `Busy`), so the bound is actually enforced — it is no
    /// longer dead config.
    slots: Arc<WorkerSlots>,
}

/// A minimal std-only counting semaphore (no tokio): `cap` permits, non-blocking
/// `try_acquire` returns a RAII `SlotGuard` whose `Drop` releases the permit. Used by
/// `Dispatcher` to bound in-flight calls (HARNESS audit A4) — the guard is moved into the
/// worker thread so in-flight count ≤ `cap` for the job's duration.
struct WorkerSlots {
    cap: usize,
    in_flight: AtomicUsize,
}

impl WorkerSlots {
    fn new(cap: usize) -> Self {
        WorkerSlots {
            cap: cap.max(1),
            in_flight: AtomicUsize::new(0),
        }
    }
    /// Acquire a slot if one is free; otherwise `None` (caller refuses with `Busy`).
    fn try_acquire(self: &Arc<Self>) -> Option<SlotGuard> {
        let mut cur = self.in_flight.load(Ordering::SeqCst);
        loop {
            if cur >= self.cap {
                return None;
            }
            match self.in_flight.compare_exchange_weak(
                cur,
                cur + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    return Some(SlotGuard {
                        slots: Arc::clone(self),
                    })
                }
                Err(c) => cur = c,
            }
        }
    }
}

/// RAII permit for a `WorkerSlots` slot; released on drop.
struct SlotGuard {
    slots: Arc<WorkerSlots>,
}

impl Drop for SlotGuard {
    fn drop(&mut self) {
        self.slots.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

impl<B: LlmBackend + Send + Sync + 'static> Dispatcher<B> {
    /// `workers` = pool size (≤ backend parallelism cap). `capacity`/`refill_rate` size the bucket
    /// in token units (1 token = 1 unit; see `Usage::cost`).
    pub fn new(backend: B, workers: usize, capacity: u64, refill_rate: f64) -> Self {
        Dispatcher {
            backend: Arc::new(backend),
            bucket: Arc::new(TokenBucket::new(capacity as f64, refill_rate)),
            slots: Arc::new(WorkerSlots::new(workers)),
        }
    }

    /// Dispatch one chat request. Returns the response, or a typed `DispatchError`.
    ///
    /// Concurrency is bounded by `WorkerSlots` of `workers` permits (HARNESS audit
    /// A4): `try_acquire` refuses immediately with `Busy` when all slots are in
    /// flight, so the cap is enforced rather than dead config. The acquired guard
    /// is moved into the worker thread and released when the job finishes, keeping
    /// in-flight count ≤ `workers`. Budget is checked inside the job
    /// (`TokenBucket` bounds volume over time; the slots bound concurrency).
    pub fn dispatch(&self, req: ChatRequest) -> Result<ChatResponse, DispatchError> {
        // A4: enforce the concurrency cap — refuse (degrade-closed) when full.
        let guard = match self.slots.try_acquire() {
            Some(g) => g,
            None => return Err(DispatchError::Busy),
        };
        let (tx, rx) = channel::<Result<ChatResponse, DispatchError>>();
        let backend = Arc::clone(&self.backend);
        let bucket = Arc::clone(&self.bucket);
        let cost = (req.max_tokens as f64).max(1.0); // budget in approx-output units; degrade-closed.

        // Hold the slot for the job's duration so in-flight is bounded at `workers`.
        thread::spawn(move || {
            let _guard = guard; // dropped at end of this closure ⇒ slot freed
            let result = if bucket.try_acquire(cost) {
                let t0 = std::time::Instant::now();
                let r = backend.chat(&req).map_err(DispatchError::Backend);
                let ms = t0.elapsed().as_millis() as u64;
                // H1 harvest — record BOTH success and failure so gov_route's success-rate folds
                // honestly. Tokens/cost are 0 on failure (no response emitted).
                let (tokens, ok) = match &r {
                    Ok(resp) => (resp.usage.cost(), true),
                    Err(_) => (0, false),
                };
                let rec = TrackRecord {
                    backend_id: backend.id().to_string(),
                    model_id: req.model_id.clone(),
                    total_tokens: tokens,
                    ms,
                    task: "chat".to_string(),
                    success: ok,
                    value: 0.0,
                    cost: tokens as f64,
                };
                append_harvest(&rec);
                r
            } else {
                Err(DispatchError::BudgetExceeded)
            };
            let _ = tx.send(result);
        });
        rx.recv()
            .map_err(|_| DispatchError::Backend(LlmError::Unavailable))?
    }

    /// A clone of the shared backend handle (for cache-fronted embed/rerank paths that bypass the
    /// dispatcher's per-call token budget but still share the SAME cache store).
    pub fn backend(&self) -> Arc<B> {
        Arc::clone(&self.backend)
    }

    /// Health passthrough (degrade-closed).
    pub fn health(&self) -> Result<(), LlmError> {
        self.backend.health()
    }
}

/// Append a harvested record to `track_record.jsonl` (H1 EV ledger). Local-only, M8-compliant.
/// Emits the `gov_route`-compatible superset schema so the EV pricing loop can fold it directly.
/// Failures are non-fatal (telemetry must never break the call path).
///
/// Made `pub` so other consumers of the harvest ledger (e.g. the `agent-loop` host binary) emit
/// the EXACT same row the `Dispatcher` writes — one channel, no schema drift (AGENTS.md:
/// "extend that ledger, do not invent a parallel channel").
pub fn append_harvest(rec: &TrackRecord) {
    use std::io::Write;
    let line = format!(
        "{{\"model\":\"{}\",\"task\":\"{}\",\"success\":{},\"value\":{},\"cost\":{},\"backend\":\"{}\",\"tokens\":{},\"ms\":{}}}\n",
        rec.model_id, rec.task, rec.success, rec.value, rec.cost, rec.backend_id, rec.total_tokens, rec.ms
    );
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("track_record.jsonl")
    {
        let _ = f.write_all(line.as_bytes());
    }
}

/// Backwards-compatible alias so the plan's `Sender` reference resolves cleanly.
pub type RecordSender = Sender<Result<ChatResponse, DispatchError>>;

/// Decode one harvest-ledger line (the inverse of `append_harvest`). Used by the telemetry fold
/// (`telemetry.rs`) so the same row gov_route reads is the row the in-process aggregator consumes —
/// one schema, no drift. Returns `None` on a malformed line (fail-closed; the row is skipped).
pub(crate) fn decode_track_record(line: &str) -> Result<TrackRecord, serde_json::Error> {
    let v: serde_json::Value = serde_json::from_str(line)?;
    let get_str = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let get_bool = |k: &str| v.get(k).and_then(|x| x.as_bool()).unwrap_or(false);
    let get_f64 = |k: &str| v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
    Ok(TrackRecord {
        backend_id: get_str("backend"),
        model_id: get_str("model"),
        total_tokens: v.get("tokens").and_then(|x| x.as_u64()).unwrap_or(0),
        ms: v.get("ms").and_then(|x| x.as_u64()).unwrap_or(0),
        task: get_str("task"),
        success: get_bool("success"),
        value: get_f64("value"),
        cost: get_f64("cost"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dowiz_kernel::ports::llm::{
        Caps, ChatRequest, ChatResponse, EmbedRequest, EmbedResponse, LlmBackend, LlmError,
        RerankRequest, RerankResponse, Usage,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// A fake backend that counts calls. No network.
    struct FakeBackend {
        calls: AtomicUsize,
    }
    impl LlmBackend for FakeBackend {
        fn id(&self) -> &str {
            "fake"
        }
        fn caps(&self) -> Caps {
            Caps {
                chat: true,
                embed: false,
                rerank: false,
                tool_calling: false,
            }
        }
        fn chat(&self, _req: &ChatRequest) -> Result<ChatResponse, LlmError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ChatResponse {
                content: "ok".into(),
                usage: Usage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                },
                tool_calls: Vec::new(),
            })
        }
        fn embed(&self, _req: &EmbedRequest) -> Result<EmbedResponse, LlmError> {
            Err(LlmError::Unsupported)
        }
        fn rerank(&self, _req: &RerankRequest) -> Result<RerankResponse, LlmError> {
            Err(LlmError::Unsupported)
        }
        fn health(&self) -> Result<(), LlmError> {
            Ok(())
        }
    }

    fn req() -> ChatRequest {
        ChatRequest {
            max_tokens: 8,
            ..Default::default()
        }
    }

    #[test]
    fn budget_exhausted_returns_typed_refusal() {
        // capacity=8, refill_rate=0 ⇒ exactly one call (cost=max_tokens=8) fits; the 2nd is refused.
        let d = Dispatcher::new(
            FakeBackend {
                calls: AtomicUsize::new(0),
            },
            1,
            8,
            0.0,
        );
        // Use Arc to share the fake so we can count calls.
        let shared = d.backend.clone();
        let _ = d.dispatch(req()).expect("first call within budget");
        match d.dispatch(req()) {
            Err(DispatchError::BudgetExceeded) => {}
            other => panic!("second call must be BudgetExceeded, got {:?}", other),
        }
        assert_eq!(
            shared.calls.load(Ordering::SeqCst),
            1,
            "only one upstream call made"
        );
    }

    #[test]
    fn within_budget_succeeds_and_counts() {
        let d = Dispatcher::new(
            FakeBackend {
                calls: AtomicUsize::new(0),
            },
            2,
            100,
            0.0,
        );
        for _ in 0..3 {
            d.dispatch(req()).expect("within budget");
        }
        assert_eq!(d.backend.calls.load(Ordering::SeqCst), 3);
    }

    // Keep `Arc` import meaningful even if a test is removed.
    #[test]
    fn backend_is_shared_arc() {
        let d = Dispatcher::new(
            FakeBackend {
                calls: AtomicUsize::new(0),
            },
            1,
            100,
            0.0,
        );
        let _: Arc<FakeBackend> = d.backend.clone();
    }

    /// HARNESS audit A4 — the `workers` concurrency cap is now ENFORCED, not dead config.
    ///
    /// With `workers = 1` and a slow backend, firing many dispatches concurrently must keep
    /// in-flight ≤ 1 AND refuse the overflow with a typed `Busy` (degrade-closed) rather than
    /// spawning unbounded threads or silently queuing.
    struct SlowBackend {
        in_flight: Arc<AtomicUsize>,
        max_in_flight: Arc<AtomicUsize>,
    }
    impl LlmBackend for SlowBackend {
        fn id(&self) -> &str {
            "slow"
        }
        fn caps(&self) -> Caps {
            Caps {
                chat: true,
                embed: false,
                rerank: false,
                tool_calling: false,
            }
        }
        fn chat(&self, _req: &ChatRequest) -> Result<ChatResponse, LlmError> {
            let cur = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            // track max in-flight via a CAS loop
            let mut m = self.max_in_flight.load(Ordering::SeqCst);
            while cur > m
                && self
                    .max_in_flight
                    .compare_exchange(m, cur, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
            {
                m = self.max_in_flight.load(Ordering::SeqCst);
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            Ok(ChatResponse {
                content: "ok".into(),
                usage: Usage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                },
                tool_calls: Vec::new(),
            })
        }
        fn embed(&self, _req: &EmbedRequest) -> Result<EmbedResponse, LlmError> {
            Err(LlmError::Unsupported)
        }
        fn rerank(&self, _req: &RerankRequest) -> Result<RerankResponse, LlmError> {
            Err(LlmError::Unsupported)
        }
        fn health(&self) -> Result<(), LlmError> {
            Ok(())
        }
    }

    #[test]
    fn concurrency_cap_enforced_degrade_closed() {
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_in_flight = Arc::new(AtomicUsize::new(0));
        // workers=1 ⇒ at most ONE in-flight call at a time.
        let d = Arc::new(Dispatcher::new(
            SlowBackend {
                in_flight: Arc::clone(&in_flight),
                max_in_flight: Arc::clone(&max_in_flight),
            },
            1,
            1_000_000,
            0.0,
        ));
        let workers = 1u64;

        // Fire 8 dispatches concurrently from separate threads.
        let mut handles = Vec::new();
        let busy_count = Arc::new(AtomicUsize::new(0));
        for _ in 0..8 {
            let d = Arc::clone(&d);
            let bc = Arc::clone(&busy_count);
            handles.push(std::thread::spawn(move || match d.dispatch(req()) {
                Err(DispatchError::Busy) => {
                    bc.fetch_add(1, Ordering::SeqCst);
                }
                Ok(_) => {}
                Err(other) => panic!("unexpected dispatch error: {:?}", other),
            }));
        }
        for h in handles {
            let _ = h.join();
        }

        let max_seen = max_in_flight.load(Ordering::SeqCst);
        assert!(
            max_seen <= workers as usize,
            "in-flight ({}) must be ≤ workers ({})",
            max_seen,
            workers
        );
        assert!(
            busy_count.load(Ordering::SeqCst) >= 1,
            "overflow must be refused with Busy (degrade-closed)"
        );
    }
}
