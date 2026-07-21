//! cache.rs — WAVE 1(b): exact-match sha3 response cache (Layer A, §3.2).
//!
//! `CachingBackend<B, S>` wraps any `LlmBackend` `B` with a content-addressed store `S`
//! (`dowiz_kernel::backup::{BlockStore, MemStore}`). Key = `sha3_256(BTreeMap-canonical request)`;
//! value = the serialized `ChatResponse`. `MemStore::put`'s idempotence IS the cache-hit semantics
//! for free. Layer B (semantic) is intentionally NOT here — it belongs to advisory call sites only,
//! and the `CachePolicy` type on `ChatRequest` enforces that a gate-critical caller cannot opt in.
//!
//! Correctness = exactness: the key includes `model_id` + every sampling param, so a hit is
//! provably identical. For `temperature > 0` / unpinned-seed calls a hit returns one valid prior
//! sample; such callers should pass `CachePolicy::NoCache` when a fresh sample is contractually required.

use dowiz_kernel::backup::{BlockStore, Hash, MemStore};
use dowiz_kernel::event_log::sha3_256;
use dowiz_kernel::ports::llm::{
    Caps, ChatRequest, ChatResponse, EmbedRequest, EmbedResponse, LlmBackend, LlmError,
    RerankRequest, RerankResponse, Usage,
};
use serde_json::json;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

/// Default upper bound on distinct cached responses (HARNESS audit A3). The unbounded
/// `MemStore` this used to wrap is now bounded; pick a sane local-first default. A hub
/// may choose a different cap via `with_capacity`.
pub const DEFAULT_CACHE_CAP: usize = 1024;

/// LRU-bounded wrapper over any `BlockStore` (HARNESS audit A3).
///
/// `put` evicts the least-recently-used entry when at `max_entries`; `touch` promotes a
/// key to most-recent on a cache hit. This makes `CachingBackend` finite — the previous
/// `Arc<Mutex<MemStore>>` retained every distinct prompt tuple for the process lifetime.
struct BoundedStore<S: BlockStore> {
    inner: S,
    /// LRU order: front = oldest, back = most-recently used.
    order: VecDeque<Hash>,
    max_entries: usize,
}

impl<S: BlockStore> BoundedStore<S> {
    fn new(inner: S, max_entries: usize) -> Self {
        BoundedStore {
            inner,
            order: VecDeque::new(),
            max_entries: max_entries.max(1),
        }
    }

    /// Promote `id` to most-recent (called on a cache hit). No-op if absent.
    fn touch(&mut self, id: &Hash) {
        if self.inner.get(id).is_some() {
            self.order.retain(|h| h != id);
            self.order.push_back(*id);
        }
    }
}

impl<S: BlockStore> BlockStore for BoundedStore<S> {
    fn put(&mut self, id: Hash, bytes: &[u8]) -> bool {
        // Evict LRU when at capacity AND this is a genuinely new id.
        if self.inner.get(&id).is_none() && self.order.len() >= self.max_entries {
            if let Some(old) = self.order.pop_front() {
                self.inner.remove(&old);
            }
        }
        let is_new = self.inner.put(id, bytes);
        if is_new {
            self.order.retain(|h| h != &id);
            self.order.push_back(id);
        }
        is_new
    }
    fn get(&self, id: &Hash) -> Option<&[u8]> {
        self.inner.get(id)
    }
    fn get_owned(&self, id: &Hash) -> Option<Vec<u8>> {
        self.inner.get_owned(id)
    }
    fn len(&self) -> usize {
        self.inner.len()
    }
    fn remove(&mut self, id: &Hash) -> Option<Vec<u8>> {
        self.order.retain(|h| h != id);
        self.inner.remove(id)
    }
}

/// A response cache over any `LlmBackend`. Generic over the store so tests can supply a
/// call-counting `BlockStore` double (done-check: second identical call makes ZERO HTTP calls).
///
/// `store` is an `Arc<Mutex<_>>` (NOT `RefCell`) so the cache is `Sync` and can be shared behind an
/// `Arc` across the `Dispatcher`'s worker threads (WAVE 1(d) bound `B: Send + Sync`). The `Arc`
/// ALSO makes `CachingBackend` `Clone` without re-cloning the store — so the `Dispatcher`'s
/// backend handle and the `Harness`'s direct backend handle share the SAME cache contents.
#[derive(Clone)]
pub struct CachingBackend<B: LlmBackend, S: BlockStore> {
    inner: B,
    store: Arc<Mutex<BoundedStore<S>>>,
}

impl<B: LlmBackend> CachingBackend<B, MemStore> {
    /// Convenience: wrap with the in-memory `MemStore` (default for single-node/local-first),
    /// bounded at [`DEFAULT_CACHE_CAP`].
    pub fn new(inner: B) -> Self {
        CachingBackend {
            inner,
            store: Arc::new(Mutex::new(BoundedStore::new(
                MemStore::new(),
                DEFAULT_CACHE_CAP,
            ))),
        }
    }
}

impl<B: LlmBackend, S: BlockStore> CachingBackend<B, S> {
    pub fn with_store(inner: B, store: S) -> Self {
        CachingBackend {
            inner,
            store: Arc::new(Mutex::new(BoundedStore::new(store, DEFAULT_CACHE_CAP))),
        }
    }

    /// Wrap with an explicit entry cap (HARNESS audit A3 — finite cache).
    pub fn with_capacity(inner: B, store: S, max_entries: usize) -> Self {
        CachingBackend {
            inner,
            store: Arc::new(Mutex::new(BoundedStore::new(store, max_entries))),
        }
    }

    /// Canonical, deterministic request key (BTreeMap → sorted JSON → sha3_256). Only cacheable
    /// requests (`Exact` policy, or `SemanticOk` which falls back to Exact on a miss) participate;
    /// `NoCache` bypasses entirely.
    fn cache_key(req: &ChatRequest) -> Option<[u8; 32]> {
        if matches!(
            req.cache_policy,
            dowiz_kernel::ports::llm::CachePolicy::NoCache
        ) {
            return None;
        }
        let mut m = BTreeMap::new();
        m.insert("model_id", req.model_id.clone());
        m.insert(
            "messages",
            req.messages
                .iter()
                .map(|msg| format!("{}:{}", msg.role, msg.content))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        m.insert("temperature", format!("{:.6}", req.temperature));
        m.insert("top_p", format!("{:.6}", req.top_p));
        m.insert("max_tokens", req.max_tokens.to_string());
        m.insert("seed", req.seed.map(|s| s.to_string()).unwrap_or_default());
        m.insert("task_class", format!("{:?}", req.task_class));
        // `options` (Ollama knobs) affect the output ⇒ part of the key.
        let opts: BTreeMap<String, String> = req.options.clone();
        m.insert("options", serde_json::to_string(&opts).unwrap_or_default());
        // HARNESS §3.2: a tool-bearing request partitions the cache from its
        // tool-less twin. Omitting `tools` here would poison the cache — a cached
        // tool-less answer served to a tool run (or vice versa). The key carries a
        // stable, comparable form: each tool's name+arg_name (the declaration, not
        // the description text, which is model-facing prose and excludes nothing
        // semantically). Sorted for determinism.
        let mut tool_keys: Vec<String> = req
            .tools
            .iter()
            .map(|t| format!("{}:{}", t.name, t.arg_name))
            .collect();
        tool_keys.sort();
        m.insert("tools", tool_keys.join(","));
        let canonical = serde_json::to_vec(&json!(m)).unwrap_or_default();
        Some(sha3_256(&canonical))
    }
}

impl<B: LlmBackend, S: BlockStore> LlmBackend for CachingBackend<B, S> {
    fn id(&self) -> &str {
        self.inner.id()
    }
    fn caps(&self) -> Caps {
        self.inner.caps()
    }
    fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, LlmError> {
        let key = Self::cache_key(req);
        if let Some(k) = key {
            // Peek (owned clone) + promote to MRU under one lock; decode outside the lock.
            let hit = {
                let mut g = self.store.lock().unwrap();
                let bytes = g.get(&k).map(|b| b.to_vec());
                if bytes.is_some() {
                    g.touch(&k);
                }
                bytes
            };
            if let Some(bytes) = hit {
                if let Some(resp) = decode_response(&bytes) {
                    return Ok(resp); // EXACT hit — zero upstream call.
                }
            }
        }
        let resp = self.inner.chat(req)?;
        if let Some(k) = key {
            // BoundedStore::put evicts LRU when at cap (HARNESS audit A3).
            let bytes = encode_response(&resp);
            self.store.lock().unwrap().put(k, &bytes);
        }
        Ok(resp)
    }
    fn embed(&self, req: &EmbedRequest) -> Result<EmbedResponse, LlmError> {
        self.inner.embed(req)
    }
    fn rerank(&self, req: &RerankRequest) -> Result<RerankResponse, LlmError> {
        self.inner.rerank(req)
    }
    fn health(&self) -> Result<(), LlmError> {
        self.inner.health()
    }
}

/// Manual (de)serialization of `ChatResponse` — kept local so the kernel's `llm.rs` value types
/// stay serde-free (zero-dep invariant). Format: three lines — content, then
/// `prompt|completion|total`, then `model_id|utterance_id`. The provenance line is mandatory
/// (L1 opacity): a cache hit must restore the real model tag, never fabricate one.
fn encode_response(resp: &ChatResponse) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(resp.content.as_bytes());
    out.push(b'\n');
    out.extend_from_slice(
        format!(
            "{}|{}|{}",
            resp.usage.prompt_tokens, resp.usage.completion_tokens, resp.usage.total_tokens
        )
        .as_bytes(),
    );
    out.push(b'\n');
    out.extend_from_slice(format!("{}|{}", resp.model_id, resp.utterance_id).as_bytes());
    out
}

/// Inverse of `encode_response`. Returns `None` on any malformed row (degrade to miss, never panic).
/// The provenance line MUST be present and well-formed — a decoded reply without a model tag is
/// refused (L1: no provenance-less output can re-enter the supervisor from cache).
fn decode_response(bytes: &[u8]) -> Option<ChatResponse> {
    let s = std::str::from_utf8(bytes).ok()?;
    let mut lines = s.splitn(3, '\n');
    let content = lines.next()?;
    let usage = lines.next()?;
    let prov = lines.next()?;
    let mut uparts = usage.split('|');
    let prompt = uparts.next()?.parse::<u32>().ok()?;
    let completion = uparts.next()?.parse::<u32>().ok()?;
    let total = uparts.next()?.parse::<u32>().ok()?;
    let mut pparts = prov.split('|');
    let model_id = pparts.next()?.to_string();
    let utterance_id = pparts.next()?.parse::<u64>().ok()?;
    if model_id.is_empty() {
        return None;
    }
    Some(ChatResponse {
        content: content.to_string(),
        usage: Usage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: total,
        },
        model_id,
        utterance_id,
        tool_calls: Vec::new(),
    })
}
/// A no-op store: every `get` misses, every `put` is dropped. Used to compile-time-disable
/// caching (M5 config: `StackBuilder::with_cache(false)`) without a separate code path.
#[derive(Clone, Default)]
pub struct NoCache;

impl BlockStore for NoCache {
    fn put(&mut self, _id: Hash, _bytes: &[u8]) -> bool {
        false // never persists — disables the cache
    }
    fn get(&self, _id: &Hash) -> Option<&[u8]> {
        None // always a miss
    }
    fn len(&self) -> usize {
        0 // holds no blocks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dowiz_kernel::backup::MemStore;
    use dowiz_kernel::ports::llm::{
        Caps, ChatRequest, ChatResponse, EmbedRequest, EmbedResponse, LlmBackend, LlmError,
        Message, RerankRequest, RerankResponse, TaskClass,
    };
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    /// A call-counting double: records how many times `chat` ran.
    struct CountingBackend {
        calls: RefCell<usize>,
    }
    impl CountingBackend {
        fn new() -> Self {
            CountingBackend {
                calls: RefCell::new(0),
            }
        }
        fn call_count(&self) -> usize {
            *self.calls.borrow()
        }
    }
    impl LlmBackend for CountingBackend {
        fn id(&self) -> &str {
            "double"
        }
        fn caps(&self) -> Caps {
            Caps {
                chat: true,
                embed: false,
                rerank: false,
                tool_calling: false,
            }
        }
        fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, LlmError> {
            *self.calls.borrow_mut() += 1;
            Ok(ChatResponse {
                content: "cached-answer".into(),
                usage: dowiz_kernel::ports::llm::Usage {
                    prompt_tokens: 3,
                    completion_tokens: 2,
                    total_tokens: 5,
                },
                model_id: req.model_id.clone(),
                utterance_id: 1,
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

    fn req(temp: f32) -> ChatRequest {
        ChatRequest {
            model_id: "m:tag".into(),
            messages: vec![dowiz_kernel::ports::llm::Message {
                role: "user".into(),
                content: "hi".into(),
            }],
            temperature: temp,
            max_tokens: 8,
            task_class: TaskClass::General,
            cache_policy: dowiz_kernel::ports::llm::CachePolicy::Exact,
            options: BTreeMap::new(),
            ..Default::default()
        }
    }

    #[test]
    fn exact_hit_makes_zero_upstream_calls() {
        let backend = CountingBackend::new();
        let cached = CachingBackend::with_store(backend, MemStore::new());
        let r1 = cached.chat(&req(0.0)).unwrap();
        assert_eq!(r1.content, "cached-answer");
        assert_eq!(cached.inner.call_count(), 1);
        // Identical temperature=0 request → served from cache, NO upstream call.
        let r2 = cached.chat(&req(0.0)).unwrap();
        assert_eq!(r2.content, "cached-answer");
        assert_eq!(
            cached.inner.call_count(),
            1,
            "second identical call must hit cache (0 HTTP)"
        );
    }

    #[test]
    fn changed_param_is_a_miss() {
        let backend = CountingBackend::new();
        let cached = CachingBackend::with_store(backend, MemStore::new());
        let _ = cached.chat(&req(0.0)).unwrap();
        assert_eq!(cached.inner.call_count(), 1);
        // Different temperature → different canonical key → miss → upstream call.
        let _ = cached.chat(&req(0.5)).unwrap();
        assert_eq!(
            cached.inner.call_count(),
            2,
            "changed sampling param must miss"
        );
    }

    #[test]
    fn no_cache_policy_bypasses() {
        let backend = CountingBackend::new();
        let cached = CachingBackend::with_store(backend, MemStore::new());
        let mut r = req(0.0);
        r.cache_policy = dowiz_kernel::ports::llm::CachePolicy::NoCache;
        let _ = cached.chat(&r).unwrap();
        let _ = cached.chat(&r).unwrap();
        assert_eq!(cached.inner.call_count(), 2, "NoCache must never hit cache");
    }

    // B-c (HARNESS §3.2): a tool-bearing request PARTITIONS the cache from its
    // tool-less twin. Two requests that are identical except for `tools=[]` vs
    // `tools=[read_order_status]` must produce two distinct cache entries and
    // therefore TWO upstream calls (never a poisoned shared hit). This is the
    // regression guard against cache-poisoning once `ChatRequest.tools` lands.
    #[test]
    fn tools_field_partitions_the_cache() {
        let backend = CountingBackend::new();
        let cached = CachingBackend::with_store(backend, MemStore::new());
        let plain = req(0.0);
        let mut tooled = req(0.0);
        tooled.tools.push(dowiz_kernel::ports::llm::ToolDecl {
            name: "read_order_status".to_string(),
            description: "Read order status.".to_string(),
            arg_name: "order_id".to_string(),
        });

        let _ = cached.chat(&plain).unwrap();
        let _ = cached.chat(&tooled).unwrap();
        assert_eq!(
            cached.inner.call_count(),
            2,
            "tool-bearing request must NOT collide with its tool-less twin in the cache"
        );

        // A repeat of each still hits its own partition (no extra upstream call).
        let _ = cached.chat(&plain).unwrap();
        let _ = cached.chat(&tooled).unwrap();
        assert_eq!(
            cached.inner.call_count(),
            2,
            "identical re-requests must each hit their own distinct cache partition"
        );
    }

    /// HARNESS audit A3 — the cache is now BOUNDED: once `max_entries` distinct keys exist,
    /// the least-recently-used is evicted, so a previously-cached (now-evicted) key misses and
    /// re-calls upstream instead of being retained for the process lifetime.
    #[test]
    fn cache_is_bounded_lru_evicts() {
        // cap = 2 distinct entries
        let backend = CountingBackend::new();
        let cached = CachingBackend::with_capacity(backend, MemStore::new(), 2);

        // Three DISTINCT temperature=0 prompts → only the 2 most-recent survive.
        let mut a = req(0.0);
        a.messages = vec![Message {
            role: "user".into(),
            content: "A".into(),
        }];
        let mut b = req(0.0);
        b.messages = vec![Message {
            role: "user".into(),
            content: "B".into(),
        }];
        let mut c = req(0.0);
        c.messages = vec![Message {
            role: "user".into(),
            content: "C".into(),
        }];

        let _ = cached.chat(&a).unwrap();
        let _ = cached.chat(&b).unwrap();
        assert_eq!(cached.inner.call_count(), 2);
        // Re-request A → still cached (it's one of the 2 most-recent). No new call.
        let _ = cached.chat(&a).unwrap();
        assert_eq!(cached.inner.call_count(), 2, "A still cached");
        // Insert C → pushes the LRU (B) out (A was just touched, so B is oldest).
        let _ = cached.chat(&c).unwrap();
        assert_eq!(cached.inner.call_count(), 3, "C is a new entry");
        // B was evicted → re-requesting B re-calls upstream.
        let _ = cached.chat(&b).unwrap();
        assert_eq!(
            cached.inner.call_count(),
            4,
            "B was LRU-evicted and must miss (bounded cache)"
        );
    }
}
