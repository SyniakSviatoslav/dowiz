//! P57 — Canvas text input & editing (Latin + Cyrillic), per
//! `BLUEPRINT-P57-canvas-text-input.md`.
//!
//! Module layout:
//! - `field.rs` — **Lane A (offline-clean, always built):** the closed
//!   `EditCmd` alphabet, the `TextField` editor over a plain `String` buffer
//!   (UTF-8 byte-offset caret/selection, dependency-free grapheme/word motion),
//!   the `ClipboardPort`/`KeyMods` traits + `key_to_cmd` table, and the
//!   caret-blink `Spring` (blink without waking the field integrator, §5.1).
//! - `cosmic_text.rs` — **Lane B (O18a, `feature = "text"`):** the cosmic-text
//!   shaping seam. Gated OFF by default (same network grant as `gpu`, P38 §2);
//!   the default build links NO cosmic-text (P57 §2.3 / DoD-10).
//!
//! `text_scope.rs` (sibling, the v2 boundary classifier) is `/text_scope.rs`.

pub use field::key_to_cmd;
pub use field::ClipboardPort;
pub use field::{
    ByteCursor, EditCmd, EditEvent, EditReject, FieldPos, KeyMods, Rect, Selection, ShapedGlyph,
    TextField, WidgetId, CARET_BAR_HALF_W, CARET_BLINK_HZ, FIELD_MAX_BYTES, WORD_BOUNDARY,
};

#[cfg(feature = "text")]
pub use cosmic_text::{new_cosmic_editor, shape, COSMIC_SHAPING};

mod field;

#[cfg(feature = "text")]
mod cosmic_text;
