//! BLUEPRINT P71 — courier surface (P52-rev).
//!
//! The realized, full-wgpu courier app for a human courier: voice-primary in
//! motion, dispatch-wired to P65, rendering K1-K8 through P38a. This crate is
//! the **render + real-time binding** layer only — it consumes the kernel-side
//! folds / P65 dispatch events / P51 track frames / P64 voice contract as
//! *wire frames* and re-owns NONE of their logic (see §1.5 of the blueprint).
//!
//! Module map:
//! - [`types`]      — shared contract types + the spec §2 constants.
//! - [`dispatch`]   — a LOCAL MIRROR of P65's `DispatchEvent`/`DispatchInput`/
//!                    `DispatchSession` (P71 consumes the wire frames; it does
//!                    NOT import bebop2). The accept-timeout is hub-owned.
//! - [`surface`]    — the `CourierSurface` state machine (R2): the only mutator
//!                    of the offer sub-state besides accept/decline emission.
//! - [`voice`]      — P64 voice binding (R3): `input_profile_for`, the parity-
//!                    pinned `offer_urgency` cue, the deterministic classifier.
//! - [`render`]     — P38a CPU-floor K-screen composition + P58 a11y mirror
//!                    (R1/R4) + the P63 SP-6 floor-parity hook.
//! - [`battery`]    — the P63 SP-5 battery gate (R5), `#[ignore]` until SP-5
//!                    lands a real `VerdictRecord`.
//!
//! TS/NODE BAN: this is Rust only. No DOM, no webview, no JS.

pub mod battery;
pub mod dispatch;
pub mod render;
pub mod surface;
pub mod types;
pub mod voice;

// Re-exports for ergonomic consumption by `tests/` and by future surfaces.
pub use battery::*;
pub use dispatch::*;
pub use render::*;
pub use surface::*;
pub use types::*;
pub use voice::*;
