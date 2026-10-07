//! dowiz-canvas -- the zero-DOM scene engine (roadmap Wave CV, row CV0 core) and its first
//! surface, the room/kitchen board (row CV1).
//!
//! THE SHAPE (docs/research/2026-10-06-canvas-rust-webgl-ui.md §5.1). A page is `<body><canvas>`
//! and one module script. Each frame Rust builds two things from its own state:
//!   * a SCENE (`scene.rs`): every tappable thing as a node with an id, a tour anchor (the
//!     `data-tour` word the learn lessons already use) and an integer rectangle -- the hit-test,
//!     the tap-size check and the tour all read it, so nothing needs a DOM element to be found;
//!   * a COMMAND BUFFER (`cmd.rs`): rects and text runs the loader replays on Canvas2D, with an
//!     FNV hash over its CONTENT (never a pointer) so two frames can be compared byte for byte --
//!     that is what the context-loss gate compares after a restore.
//! Text is shaped and rasterised by the host's Canvas2D (system fonts, no font download); Rust asks
//! for each run's width ONCE and keeps it (`text.rs`). Layout is integer CSS pixels (`layout.rs`);
//! the loader multiplies by devicePixelRatio exactly once, on the backing store.
//!
//! STRINGS are `lang.rs`: an exhaustive `match` per word with all four languages as positional
//! arguments, so a missing language is a compile error, not a blank label.
//!
//! NO ALLOCATOR. Pools are fixed arrays sized for a venue's busiest pass; what does not fit is
//! COUNTED (`Board::dropped`) and drawn as a line saying so, never silently cut.
#![cfg_attr(target_arch = "wasm32", no_std)]
#![allow(clippy::new_without_default)]

pub mod board;
pub mod cmd;
pub mod field;
pub mod geom;
pub mod lang;
pub mod layout;
pub mod scene;
pub mod text;
pub mod ui;

#[cfg(target_arch = "wasm32")]
mod ffi;

/// Decimal digits of a non-negative integer into `buf`, returned as a str (no `core::fmt`, which
/// costs kilobytes of wasm for one number).
pub fn itoa(mut n: u64, buf: &mut [u8; 20]) -> &str {
    let mut i = buf.len();
    if n == 0 {
        i -= 1;
        buf[i] = b'0';
    }
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    // ASCII digits only.
    core::str::from_utf8(&buf[i..]).unwrap_or("")
}

#[cfg(test)]
mod tests;
