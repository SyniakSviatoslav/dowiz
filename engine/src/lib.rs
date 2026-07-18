//! dowiz field-ui engine core — FE-01/02/03.
//!
//! Pure Rust, zero-dependency, offline-clean. Authoritative compute is
//! CPU-side; GPU/wasm is a display surface (the "bridge" models the linear
//! memory the GPU reads from). Every module carries a falsifiable
//! RED→GREEN gate as a `#[test]`.
//!
//! Invariants (from BLUEPRINTS-FIELD-UI.md Appendix B, never violated):
//! - Boundary kind = update frequency: transactional → JSON, per-frame →
//!   zero-copy view. NEVER JSON in the frame loop.
//! - Fixed dt = DT_STABLE; the integrator never sees a divergent dt.
//! - Determinism: scalar == SIMD bit-identical (no fuzz here, but the store
//!   is plain `f32` arrays so it is).

mod bridge;
// field_frame is exposed publicly: the browser renders the kernel-computed
// field via `field_frame::compose` (zero TS). No compute lives in the wasm crate.
pub mod field_frame;
// BLUEPRINT-E1 energy-gate + sign-pin at the kernel↔engine seam. TEST-ONLY:
// it changes no runtime contract (test-side reference oracle, never on the
// FieldFrame::step hot path), so it compiles only under `cfg(test)`.
#[cfg(test)]
mod field_energy;
mod loop_;
mod money_guard;
mod motion;
pub mod scene;
mod sdf;
pub mod text_input;
pub mod text_scope;
// P58 — a11y-mirror-everywhere (M1 model + M6 invariant). Offline-clean; the
// `serde` feature (optional, cached) lets the wasm boundary (de)serialize the
// tree. `mirror()` / `diff()` / `tree_hash()` are the shared contract every
// surface blueprint consumes.
pub mod semantics;
// P58 — M4 native AccessKit adapter. Feature-gated (`a11y_native`): the real
// `accesskit*` crates are NOT in the cargo cache (AK-unlock pending), so the
// default build compiles ZERO AccessKit code. Offline gates (role-totality,
// EditState bounds, composing-refused) live here and are tested now.
#[cfg(feature = "a11y_native")]
pub mod a11y_native;
mod widget_store;
mod zerocopy;

pub use bridge::{FrameProfiler, VertexBridge};
pub use loop_::{FixedTimestep, DT_STABLE, MAX_FRAME, MAX_SUBSTEPS};
pub use money_guard::{interpolate, FieldValue, Money, TweenGuard};
pub use motion::{heat_kernel_delay, Spring};
pub use scene::{Scene, SdfShape, BACKGROUND};
pub use sdf::{
    op_intersection, op_smooth_union, op_subtraction, op_union, sdf_box, sdf_circle,
    sdf_line_segment, sdf_rounded_box, SdfField,
};
pub use text_input::{
    ByteCursor, ClipboardPort, EditCmd, EditEvent, EditReject, FieldPos, KeyMods, Rect, Selection,
    ShapedGlyph, TextField, WidgetId, CARET_BAR_HALF_W, CARET_BLINK_HZ, FIELD_MAX_BYTES,
    WORD_BOUNDARY,
};
pub use text_scope::{in_wave0_scope, str_in_scope, WAVE0_SCRIPTS};
pub use widget_store::{ParticlePool, ParticlePoolRing, WidgetStore};
pub use zerocopy::{view_as_f32, write_into_linear, GpuSink, ParticleBuffer};
