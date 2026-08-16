//! External capability ports — the seams where the no_std core meets the outside
//! world through trait abstractions (zero network / HTTP / JSON / serde here).
//!
//! - [`agent`]: the `AgentBridge` port (B1) — hybrid-signed, fail-closed
//!   agent-admission seam (capability certs, scopes, sentinel integrity, manifest,
//!   admission gate).

pub mod agent;
pub mod payment_provider;
pub mod llm;
pub mod customer;
pub mod hub_intake;
pub mod llm_fallback;
pub mod mcp;
pub mod notification;
pub mod payment;
pub mod payment_capability;
pub mod tool;
