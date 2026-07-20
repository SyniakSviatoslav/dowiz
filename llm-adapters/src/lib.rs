//! llm-adapters — concrete `LlmBackend` adapters for dowiz.
//!
//! One ureq-based `OpenAiCompatTransport` + a per-adapter `Quirks` struct, surfacing three
//! adapters (`OllamaAdapter` live now, `VllmAdapter` GPU-gated, `ManagedApiAdapter` default).
//! Synchronous HTTP only (no tokio) — per the 2026-07-15 operator mandate ("rustls with ring
//! everywhere possible... NO tokio"). Concurrency is bounded by the harness `Dispatcher` (§4.2),
//! which reuses Ollama's own request-level parallelism rather than re-implementing batching.

pub mod cache;
pub mod compose;
pub mod dispatch;
pub mod ollama;
pub mod quirks;
pub mod telemetry;
pub mod transport;

pub use cache::{CachingBackend, NoCache};
pub use compose::{Harness, StackBuilder, DEFAULT_OLLAMA_BASE};
pub use dispatch::{append_harvest, DispatchError, Dispatcher, TrackRecord};
pub use ollama::{OllamaAdapter, OllamaQuirks};
pub use quirks::Quirks;
pub use transport::OpenAiCompatTransport;

/// Re-export the kernel's cache-policy type so callers pick it from one crate.
pub use dowiz_kernel::ports::llm::CachePolicy;
