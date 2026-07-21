//! ports/mod.rs — external capability ports (the seams where the kernel meets the outside world
//! without importing it). Each submodule is a `pub mod` with a one-line doc, matching the
//! `retrieval/mod.rs` / `isolation/mod.rs` convention in this crate.

/// `LlmBackend` port — pluggable local/managed LLM backend trait + value types (zero HTTP/serde).
pub mod llm;
/// Multi-provider LLM fallback chain — configures routing across 9 free/open providers
/// (Ollama, Groq, HuggingFace, DeepInfra, Fireworks, etc.). Pure data, no HTTP/serde.
pub mod llm_fallback;

/// `AgentBridge` port (B1) — hybrid-signed, fail-closed agent-admission seam.
pub mod agent;

/// `PaymentPort` port (P47 Wave-0) — cash-on-delivery settlement rail seam. Compile firewall:
/// kernel has NO payment-adapter dependency; the concrete adapter (if any future Wave needs
/// one) lives outside the kernel, mirroring `LlmBackend` / `AgentBridge`.
pub mod payment;

/// `PaymentProvider` port (BLUEPRINT-P60, W1/P60) — provider-agnostic online-fiat rail seam
/// (R2 §5.2 shape) + the idempotency contract (X6) + N-leg vendor-as-MoR atomicity (§0.2-1) +
/// the type-level no-card-data firewall (PCI red-line, structural). Compile firewall: kernel has
/// NO payment-adapter dependency; the concrete Stripe adapter lives OUT-OF-KERNEL in the
/// `payment-adapters` crate. No card-data type exists in core (no PAN / cvv / card_*).
pub mod payment_provider;

/// `PaymentCapability` (P47 operator ruling) — pure CAPABILITY DECLARATION for the rail set
/// { Fiat, Crypto, Stripe, Google/Apple Pay, OtherLater }. No client, no credentials, no
/// network: a feature flag only. `validate()` rejects `OtherLater` (`NotYetSupported`). The
/// red-line (no real provider/secret refs) is proven by construction via the module test that
/// greps this file's own source.
pub mod payment_capability;

/// `ToolPort` firewall (P40/P42) — the closed tool authority (writes
/// UNREPRESENTABLE) + the Skills-pattern discovery layer (P42).
pub mod tool;

/// MCP port + capability-scoped tool boundary (P42) — verify_chain-gated,
/// fail-closed, Skill-discovering. Typed; the stdio/JSON framing lives downstream.
pub mod mcp;

/// P49 — per-order customer identity (capability grant, privacy-minimal, no
/// device/personal data). Reuses the proto-cap signing *convention* (domain-
/// separated SHA3 commitment) over existing kernel math (geo/kalman/rng).
pub mod customer;

/// BLUEPRINT-P61 — notification fabric: `Notifier` fan-out over `PushPort`/`SmsPort`/`EmailPort`,
/// hub-local `ChannelRegistry`, the proven X10 coverage matrix, and dead-token eviction. Compile
/// firewall: ZERO network / HTTP / serde / tokio — the concrete adapters live in `notify-adapters`.
pub mod notification;

/// BLUEPRINT-P70 (W2) — Owner Surface: node-local management & configuration lanes for the
/// hub owner. All panes are deterministic FOLDS of signed, content-addressed events (the
/// `hub_no_shadow_store` invariant) — there is no admin database, no dowiz aggregator, no
/// analytics dashboard (§1.4-1, deferred to v2). Every mutating owner action is an
/// owner-cap-cert-signed intent; confirm/cancel reuses the P48 facade (agent-invocable
/// confirm/cancel is UNREPRESENTABLE — `no-agent-order-authority` grep gate). Reuse-first:
/// consumes P62 catalog, P59 cap-certs, P48 orders, P58 a11y mirror. Zero network/HTTP/serde.
pub mod owner_surface;

/// P48-INTAKE Phase 1 — channel-agnostic inbound-message vocabulary (zero-I/O firewall).
/// Provider payload types die at the adapter boundary and never reach here. Mirrors the
/// `engine/src/intcept.rs` normalization precedent: many input shapes, one downstream
/// vocabulary. The intake service is the only component holding order-placement authority;
/// adapters structurally cannot call `place_order`. Guard: NOT `kernel/src/intake.rs`
/// (unrelated constraint compiler — naming collision only).
pub mod hub_intake;

/// AgentBrowserPort — the kernel<->browser seam for anti-detect parse operations.
/// Trait defines WHAT to fetch; adapters (outside kernel) execute the actual browser automation.
pub mod agent_browser;
