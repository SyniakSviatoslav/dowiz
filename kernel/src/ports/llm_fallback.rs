//! `kernel::ports::llm_fallback` — Multi-provider LLM fallback chain (pure data).
//!
//! Defines the fallback routing topology for LLM API calls. The kernel owns
//! the *configuration* and *routing logic* — the actual HTTP/JSON lives in
//! the `llm-adapters` crate, which calls `resolve_chain` to decide which
//! provider to try and in what order.
//!
//! # Free / Open Providers Supported
//! 1. **Ollama** — local, fully free, OpenAI-compatible, no API key
//! 2. **Groq** — free tier (30 RPM, 6K TPM), ultra-fast LPU hardware
//! 3. **HuggingFace Inference** — $0.10/mo free credits, 15+ provider gateway
//! 4. **DeepInfra** — lowest pay-as-you-go, small free credits on signup
//! 5. **Fireworks AI** — $1 free credits, dual OpenAI/Anthropic compat
//! 6. **llama.cpp** — local, zero deps, maximum efficiency
//! 7. **LocalAI** — local, full OpenAI drop-in
//! 8. **vLLM** — self-hosted, production throughput
//! 9. **TGI** — HuggingFace Text Generation Inference
//!
//! # Fallback Strategy
//! ```text
//! Priority 1: Ollama (local, 0 latency, no network)
//! Priority 2: Groq (fastest cloud, generous free tier)
//! Priority 3: HuggingFace (broadest model selection, free credits)
//! Priority 4: DeepInfra (lowest cost, free startup credits)
//! Priority 5: Fireworks ($1 free, dual compat)
//! ```
//!
//! Each provider has a `max_retries`, `timeout_ms`, and `cost_weight` so
//! the `LlmBackend` adapter can make informed fallback decisions.

use crate::TriState;

pub const FALLBACK_DEFAULT_TIMEOUT_MS: u64 = 30_000;
pub const FALLBACK_DEFAULT_MAX_RETRIES: u32 = 2;
pub const FALLBACK_CONSECUTIVE_FAIL_LIMIT: u32 = 3;
pub const FALLBACK_AVAILABILITY_FAIL_LIMIT: u32 = 5;
pub const FALLBACK_EWMA_ALPHA: f64 = 0.9;

// ─── Provider Kind ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    /// Local Ollama (default: http://localhost:11434)
    Ollama,
    /// Groq cloud (api.groq.com, free tier)
    Groq,
    /// HuggingFace Inference API (router.huggingface.co)
    HuggingFace,
    /// DeepInfra (api.deepinfra.com, free startup credits)
    DeepInfra,
    /// Fireworks AI (api.fireworks.ai, $1 free)
    Fireworks,
    /// Local llama.cpp server
    LlamaCpp,
    /// LocalAI (full OpenAI drop-in)
    LocalAi,
    /// Self-hosted vLLM
    Vllm,
    /// HuggingFace TGI
    Tgi,
    /// OpenAI-compatible (generic, user-specified)
    OpenAiCompat,
}

impl ProviderKind {
    /// Default base URL for this provider.
    pub fn default_base_url(&self) -> &'static str {
        match self {
            ProviderKind::Ollama => "http://localhost:11434/v1",
            ProviderKind::Groq => "https://api.groq.com/openai/v1",
            ProviderKind::HuggingFace => "https://router.huggingface.co/v1",
            ProviderKind::DeepInfra => "https://api.deepinfra.com/v1/openai",
            ProviderKind::Fireworks => "https://api.fireworks.ai/inference/v1",
            ProviderKind::LlamaCpp => "http://localhost:8080/v1",
            ProviderKind::LocalAi => "http://localhost:8080/v1",
            ProviderKind::Vllm => "http://localhost:8000/v1",
            ProviderKind::Tgi => "http://localhost:3000/v1",
            ProviderKind::OpenAiCompat => "",
        }
    }

    /// Whether this is a local provider (no network egress).
    pub fn is_local(&self) -> bool {
        matches!(self, ProviderKind::Ollama | ProviderKind::LlamaCpp | ProviderKind::LocalAi)
    }

    /// Whether this provider requires an API key.
    pub fn needs_api_key(&self) -> TriState {
        match self {
            ProviderKind::Ollama | ProviderKind::LlamaCpp | ProviderKind::LocalAi
                | ProviderKind::Vllm | ProviderKind::Tgi => TriState::False,
            ProviderKind::Groq | ProviderKind::DeepInfra | ProviderKind::Fireworks
                | ProviderKind::HuggingFace => TriState::True,
            ProviderKind::OpenAiCompat => TriState::Unknown,
        }
    }

    /// Typical free-tier rate limit (RPM). 0 = unknown/unlimited.
    pub fn free_rpm(&self) -> u32 {
        match self {
            ProviderKind::Groq => 30,
            ProviderKind::HuggingFace => 60,
            ProviderKind::DeepInfra => 60,
            ProviderKind::Fireworks => 30,
            _ => 0,
        }
    }

    /// Cost ranking (lower = cheaper). Local = 0, free cloud ≈ 1, paid = 2+.
    pub fn cost_rank(&self) -> u8 {
        match self {
            ProviderKind::Ollama | ProviderKind::LlamaCpp | ProviderKind::LocalAi => 0,
            ProviderKind::Groq | ProviderKind::HuggingFace
                | ProviderKind::DeepInfra | ProviderKind::Fireworks => 1,
            ProviderKind::Vllm | ProviderKind::Tgi | ProviderKind::OpenAiCompat => 2,
        }
    }
}

// ─── Provider Instance ────────────────────────────────────────────────────

/// A single provider instance in a fallback chain.
#[derive(Debug, Clone)]
pub struct ProviderInstance {
    pub kind: ProviderKind,
    pub base_url: String,
    pub api_key: Option<String>,
    /// Custom model name override (e.g. "llama-3.1-8b-instant" for Groq).
    pub model_override: Option<String>,
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub priority: u32,
    /// Whether this provider is available (discovered by health check).
    pub available: TriState,
    /// Running average latency (ms), 0 = unknown.
    pub avg_latency_ms: f64,
    /// Consecutive failures (triggers temporary deprioritization).
    pub consecutive_failures: u32,
}

impl ProviderInstance {
    pub fn new(kind: ProviderKind) -> Self {
        ProviderInstance {
            kind,
            base_url: kind.default_base_url().to_string(),
            api_key: None,
            model_override: None,
            timeout_ms: FALLBACK_DEFAULT_TIMEOUT_MS,
            max_retries: FALLBACK_DEFAULT_MAX_RETRIES,
            priority: kind.cost_rank() as u32,
            available: TriState::Unknown,
            avg_latency_ms: 0.0,
            consecutive_failures: 0,
        }
    }

    /// Mark as failed, increase failure count.
    pub fn record_failure(&mut self) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        if self.consecutive_failures >= FALLBACK_CONSECUTIVE_FAIL_LIMIT {
            self.available = TriState::False;
        }
    }

    /// Mark as successful, reset failure count.
    pub fn record_success(&mut self, latency_ms: f64) {
        self.consecutive_failures = 0;
        self.available = TriState::True;
        self.avg_latency_ms = if self.avg_latency_ms == 0.0 {
            latency_ms
        } else {
            self.avg_latency_ms * FALLBACK_EWMA_ALPHA + latency_ms * (1.0 - FALLBACK_EWMA_ALPHA)
        };
    }

    pub fn is_available(&self) -> TriState {
        if self.consecutive_failures >= FALLBACK_AVAILABILITY_FAIL_LIMIT { TriState::False }
        else { self.available }
    }
}

// ─── Fallback Chain ───────────────────────────────────────────────────────

/// Ordered list of providers to try, with configurable strategy.
#[derive(Debug, Clone)]
pub struct FallbackChain {
    pub providers: Vec<ProviderInstance>,
    /// Strategy for selecting the next provider.
    pub strategy: FallbackStrategy,
}

/// How to select the next provider in the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackStrategy {
    /// Fixed priority order (defined by vector order — cheapest first).
    PriorityOrder,
    /// Try the fastest (lowest avg latency) available provider first.
    FastestFirst,
    /// Try the cheapest (lowest cost_rank) available provider first.
    CheapestFirst,
    /// Round-robin across all available providers.
    RoundRobin,
}

impl FallbackChain {
    /// Build the default fallback chain (local → free cloud → paid).
    pub fn default_chain() -> Self {
        FallbackChain {
            providers: vec![
                ProviderInstance::new(ProviderKind::Ollama),
                ProviderInstance::new(ProviderKind::LlamaCpp),
                ProviderInstance::new(ProviderKind::LocalAi),
                ProviderInstance::new(ProviderKind::Groq),
                ProviderInstance::new(ProviderKind::HuggingFace),
                ProviderInstance::new(ProviderKind::DeepInfra),
                ProviderInstance::new(ProviderKind::Fireworks),
            ],
            strategy: FallbackStrategy::PriorityOrder,
        }
    }

    /// Resolve the best available provider given current conditions.
    /// Returns None if no provider is available.
    pub fn resolve(&self) -> Option<&ProviderInstance> {
        let mut candidates: Vec<&ProviderInstance> = self.providers.iter()
            .filter(|p| p.is_available() != TriState::False)
            .collect();

        if candidates.is_empty() { return None; }

        match self.strategy {
            FallbackStrategy::PriorityOrder => {
                candidates.sort_by_key(|p| p.priority);
            }
            FallbackStrategy::FastestFirst => {
                candidates.sort_by(|a, b| {
                    a.avg_latency_ms.partial_cmp(&b.avg_latency_ms).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            FallbackStrategy::CheapestFirst => {
                candidates.sort_by_key(|p| p.kind.cost_rank());
            }
            FallbackStrategy::RoundRobin => {
                // Keep existing order, round-robin is handled externally.
            }
        }

        candidates.into_iter().next()
    }

    /// All local providers.
    pub fn local(&self) -> Vec<&ProviderInstance> {
        self.providers.iter().filter(|p| p.kind.is_local()).collect()
    }

    /// All cloud providers (those needing network).
    pub fn cloud(&self) -> Vec<&ProviderInstance> {
        self.providers.iter().filter(|p| !p.kind.is_local()).collect()
    }
}

// ─── Fallback Adapter (seam) ──────────────────────────────────────────────

/// The fallback adapter mirrors `LlmBackend` from `ports::llm` but adds
/// multi-provider fallback routing. The `llm-adapters` crate builds a
/// concrete adapter that:
/// 1. On each request, calls `resolve()` to pick the best provider
/// 2. Tries the provider with its `timeout_ms` and `max_retries`
/// 3. On failure, records the failure, calls `resolve()` again for fallback
/// 4. On success, records latency and returns the response
///
/// This struct is the kernel-side authority on the fallback topology.
/// No HTTP, no JSON, no serde — pure routing data.
#[derive(Debug, Clone)]
pub struct FallbackAdapter {
    pub chain: FallbackChain,
    /// Round-robin counter (used when strategy == RoundRobin).
    rr_counter: usize,
}

impl FallbackAdapter {
    pub fn new() -> Self {
        FallbackAdapter {
            chain: FallbackChain::default_chain(),
            rr_counter: 0,
        }
    }

    pub fn with_chain(chain: FallbackChain) -> Self {
        FallbackAdapter { chain, rr_counter: 0 }
    }

    /// Resolve the best provider for the next request.
    pub fn resolve(&mut self) -> Option<&ProviderInstance> {
        if self.chain.strategy == FallbackStrategy::RoundRobin {
            let available: Vec<usize> = self.chain.providers.iter().enumerate()
                .filter(|(_, p)| p.is_available() != TriState::False)
                .map(|(i, _)| i)
                .collect();
            if available.is_empty() { return None; }
            let idx = available[self.rr_counter % available.len()];
            self.rr_counter += 1;
            Some(&self.chain.providers[idx])
        } else {
            self.chain.resolve()
        }
    }

    /// Record a failure for a provider kind — deprioritizes it.
    pub fn record_failure(&mut self, kind: ProviderKind) {
        for p in &mut self.chain.providers {
            if p.kind == kind { p.record_failure(); break; }
        }
    }

    /// Record a success for a provider kind — updates latency, resets failures.
    pub fn record_success(&mut self, kind: ProviderKind, latency_ms: f64) {
        for p in &mut self.chain.providers {
            if p.kind == kind { p.record_success(latency_ms); break; }
        }
    }

    pub fn dashboard(&self) -> String {
        let mut out = String::with_capacity(256);
        out.push_str("LLM Fallback Chain\n");
        for p in &self.chain.providers {
            let kind = format!("{:15}", format!("{:?}", p.kind));
            let avail = match p.is_available() {
                TriState::True => "✅", TriState::False => "❌", TriState::Unknown => "❓",
            };
            let lat = if p.avg_latency_ms > 0.0 { format!("{:.0}ms", p.avg_latency_ms) } else { "?".to_string() };
            out.push_str(&format!("  {} {} fails={} lat={} prio={}\n",
                avail, kind, p.consecutive_failures, lat, p.priority));
        }
        out
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_chain_has_all_providers() {
        let chain = FallbackChain::default_chain();
        assert_eq!(chain.providers.len(), 7);
    }

    #[test]
    fn resolve_returns_some() {
        let chain = FallbackChain::default_chain();
        assert!(chain.resolve().is_some());
    }

    #[test]
    fn local_filter() {
        let chain = FallbackChain::default_chain();
        assert_eq!(chain.local().len(), 3); // Ollama + LlamaCpp + LocalAi
    }

    #[test]
    fn cloud_filter() {
        let chain = FallbackChain::default_chain();
        assert_eq!(chain.cloud().len(), 4); // Groq + HF + DeepInfra + Fireworks
    }

    #[test]
    fn record_failure_deprioritizes() {
        let mut chain = FallbackChain::default_chain();
        for _ in 0..3 {
            for p in &mut chain.providers {
                if p.kind == ProviderKind::Groq { p.record_failure(); }
            }
        }
        let resolved = chain.resolve().unwrap();
        assert_ne!(resolved.kind, ProviderKind::Groq);
    }

    #[test]
    fn cost_rank_local_lowest() {
        assert_eq!(ProviderKind::Ollama.cost_rank(), 0);
        assert_eq!(ProviderKind::Groq.cost_rank(), 1);
        assert_eq!(ProviderKind::Vllm.cost_rank(), 2);
    }

    #[test]
    fn groq_needs_api_key() {
        assert_eq!(ProviderKind::Groq.needs_api_key(), TriState::True);
        assert_eq!(ProviderKind::Ollama.needs_api_key(), TriState::False);
    }

    #[test]
    fn dashboard_contains_providers() {
        let fa = FallbackAdapter::new();
        let d = fa.dashboard();
        assert!(d.contains("Ollama"));
        assert!(d.contains("Groq"));
    }

    #[test]
    fn rr_increments_counter() {
        let mut fa = FallbackAdapter::new();
        fa.chain.strategy = FallbackStrategy::RoundRobin;
        let _r1 = fa.resolve();
        let _r2 = fa.resolve();
        assert_eq!(fa.rr_counter, 2);
    }

    #[test]
    fn provider_instance_defaults() {
        let p = ProviderInstance::new(ProviderKind::Ollama);
        assert_eq!(p.timeout_ms, 30_000);
        assert_eq!(p.available, TriState::Unknown);
    }

    #[test]
    fn all_providers_have_base_urls() {
        for kind in &[
            ProviderKind::Ollama, ProviderKind::Groq, ProviderKind::HuggingFace,
            ProviderKind::DeepInfra, ProviderKind::Fireworks, ProviderKind::LlamaCpp,
            ProviderKind::LocalAi, ProviderKind::Vllm, ProviderKind::Tgi,
        ] {
            assert!(!kind.default_base_url().is_empty(), "missing URL for {:?}", kind);
        }
    }
}
