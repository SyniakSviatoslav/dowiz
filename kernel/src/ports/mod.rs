//! ports/mod.rs — external capability ports (the seams where the kernel meets the outside world
//! without importing it). Each submodule is a `pub mod` with a one-line doc, matching the
//! `retrieval/mod.rs` / `isolation/mod.rs` convention in this crate.

/// `LlmBackend` port — pluggable local/managed LLM backend trait + value types (zero HTTP/serde).
pub mod llm;

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
