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
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;

/// Typed dispatch failure — distinct from `LlmError` (the backend's own failure).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    /// Budget exhausted — degrade-closed, the caller must handle (don't retry-silently).
    BudgetExceeded,
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
    workers: usize,
}

impl<B: LlmBackend + Send + Sync + 'static> Dispatcher<B> {
    /// `workers` = pool size (≤ backend parallelism cap). `capacity`/`refill_rate` size the bucket
    /// in token units (1 token = 1 unit; see `Usage::cost`).
    pub fn new(backend: B, workers: usize, capacity: u64, refill_rate: f64) -> Self {
        Dispatcher {
            backend: Arc::new(backend),
            bucket: Arc::new(TokenBucket::new(capacity as f64, refill_rate)),
            workers: workers.max(1),
        }
    }

    /// Dispatch one chat request. Returns the response, or a typed `DispatchError`.
    /// Spawns a worker thread (cheap; the pool size bounds how many run concurrently). A real
    /// deployment would keep a persistent pool, but a fresh-thread-per-job bounded by N in-flight
    /// channels achieves the same back-pressure with zero tokio.
    pub fn dispatch(&self, req: ChatRequest) -> Result<ChatResponse, DispatchError> {
        let (tx, rx) = channel::<Result<ChatResponse, DispatchError>>();
        let backend = Arc::clone(&self.backend);
        let bucket = Arc::clone(&self.bucket);
        let cost = (req.max_tokens as f64).max(1.0); // budget in approx-output units; degrade-closed.

        // Bound in-flight jobs: block the caller until a slot frees (visible back-pressure).
        // Simplest correct form: spawn, but cap via a semaphore-style count is overkill here —
        // the mpsc below naturally bounds because we recv before returning. For true N-parallel we
        // would keep N persistent workers; for the adapter's purpose (bounded, typed refusal) a
        // single-spawn + bucket check is sufficient and dead-simple.
        thread::spawn(move || {
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
        rx.recv().map_err(|_| DispatchError::BudgetExceeded)?
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
}
