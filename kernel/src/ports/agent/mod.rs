//! ports/agent — the `AgentBridge` port (B1): a mandatorily hybrid-signed, fail-closed,
//! enumerable-only agent-admission seam. The full module tree (capability certs, scopes,
//! sentinel integrity, manifest, command filter, admission gate) now lives in
//! `dowiz_core::ports::agent`; this is a thin re-export so `crate::ports::agent::…`
//! resolves unchanged.

pub use dowiz_core::ports::agent::*;
