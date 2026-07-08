//! Bebop — a deterministic, OS-agnostic agent core built on the `dowiz-core` invariant kernel.
//!
//! This is the "device": a hard-deterministic core (no clock/RNG/IO in any decision path) that any
//! host OS can drive. The agent operating system is baked in as native behavior — red-line denial,
//! scope-block, and falsifiable (RED+GREEN) guardrail certification — not as prompts the model can
//! ignore.
//!
//! Modules:
//!   - [`core`]  — the `kernel`-backed decision/fold/replay engine + deterministic command hash.
//!   - [`guard`] — the OS-native denial layer + Verified-by-Math gate certification.
//!   - [`brand`] — Warm Cosmo-Noir narration, teal signal from the Cowboy Bebop spaceship.

pub mod brand;
pub mod core;
pub mod guard;

pub use brand::{SHIP_TEAL, SHIP_TEAL_DEEP, VOID, HULL, BONE, AMBER, BLOOD, SHIP, TAGLINE, say, Tone, Line};
pub use core::{Core, Step, command_hash};
pub use guard::{guard_path, GuardKind, self_test, certify, Gate, RED_LINE_GLOBS, DEFAULT_SCOPE_GLOBS};
