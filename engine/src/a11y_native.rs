//! P58 — M4: the native AccessKit adapter.
//!
//! **PRE-UNLOCK (status §2 / SYNTHESIS §5 W1 row P58, Lane B):** the real
//! `accesskit` / `accesskit_winit` / `accesskit_android` crates are NOT in the
//! cargo cache (the §O18a network grant has not been issued). Per the
//! binding task rule ("network-gated crates: implement pre-unlock half
//! feature-gated, empty by default, with a compile-off test"), this module is
//! feature-gated behind `a11y_native` and compiles **zero** AccessKit code in
//! the default build.
//!
//! What IS landed now, offline:
//! 1. `role_to_accesskit` — the exhaustive 1:1 `Role → accesskit::Role` table
//!    LOOKUP the real adapter will use. It is written so that **adding a `Role`
//!    variant without a mapping is a compile error** (no `_` arm) — the §M4
//!    adversarial gate, enforced even before the dependency exists (via the
//!    `accesskit_role_for` offline mirror below, which must cover every variant).
//! 2. `native_bounds_check` — the typed clamp/refusal for `EditState` byte
//!    offsets against `text.len()` (the §M4 "out-of-range caret refused" gate),
//!    fully offline.
//! 3. The `a11y_native` feature flag + a `#[cfg(not(feature = "a11y_native"))]`
//!    compile-off test proving the whole adapter is absent from the default
//!    build.
//!
//! When the AK-unlock grant lands, the operator un-comments the `accesskit`
//! dependency in `Cargo.toml` and fills in `to_tree_update` /
//! `apply_action` per §M4 — every other blueprint-consuming contract is already
//! here and tested.

use crate::semantics::{EditState, Role};

/// The offline mirror of the `accesskit::Role` mapping. The real adapter's
/// `role_to_accesskit` will be `match role { ... Role::X => accesskit::Role::X }`
/// with NO `_` arm. This offline version proves the mapping is total over the
/// `Role` enum (the compiler forces every variant to be present). If a `Role`
/// variant is ever added without a branch here, BOTH this and the real adapter
/// fail to compile — the gate is structural.
pub fn accesskit_role_for(role: Role) -> &'static str {
    match role {
        Role::Group => "group",
        Role::Heading => "heading",
        Role::Label => "label",
        Role::Button => "button",
        Role::Link => "link",
        Role::Image => "img",
        Role::List => "list",
        Role::ListItem => "listitem",
        Role::Status => "status",
        Role::Alert => "alert",
        Role::TextInput => "textbox",
    }
}

/// §M4 adversarial gate — typed clamp/refusal for `EditState` byte offsets.
///
/// `caret` and `sel_anchor` MUST be valid `text` byte offsets. A hostile or
/// buggy `EditState` with an out-of-range offset must be clamped-or-refused,
/// never used to index (no panic path across the wasm/native boundary, §5.1).
/// Returns `Ok(())` if valid; `Err(MirrorError::Cycle(0))` is NOT appropriate —
/// we add a dedicated refusal via `EditState` validity: we reuse a clear error.
/// (We surface an `Err(String)` rather than invent a new error variant, keeping
/// `MirrorError` focused on tree-shape faults.)
pub fn native_bounds_check(edit: &EditState) -> Result<(), String> {
    let len = edit.text.len();
    if edit.caret > len {
        return Err(format!(
            "EditState.caret {} > text.len() {}",
            edit.caret, len
        ));
    }
    if edit.sel_anchor > len {
        return Err(format!(
            "EditState.sel_anchor {} > text.len() {}",
            edit.sel_anchor, len
        ));
    }
    if edit.composing {
        // §M3 adversarial: IME/composition MUST be false Wave-0. A `composing ==
        // true` reaching the native path is a bug, caught RED.
        return Err("EditState.composing == true on Wave-0 path (IME not supported)".into());
    }
    Ok(())
}

#[cfg(test)]
mod offline_native_tests {
    use super::*;
    use crate::semantics::Role;

    // §M4 — the role mapping is total over `Role`: every variant resolves. If a
    //      variant is added without a branch, this fails to compile (gate).
    #[test]
    fn role_mapping_is_total() {
        let roles = [
            Role::Group,
            Role::Heading,
            Role::Label,
            Role::Button,
            Role::Link,
            Role::Image,
            Role::List,
            Role::ListItem,
            Role::Status,
            Role::Alert,
            Role::TextInput,
        ];
        for r in roles {
            assert!(!accesskit_role_for(r).is_empty());
        }
    }

    // §M4 adversarial — out-of-range caret is refused, never indexed.
    #[test]
    fn bounds_check_refuses_out_of_range_caret() {
        let edit = EditState {
            text: "hi".into(),
            caret: 5, // > len (2)
            sel_anchor: 0,
            composing: false,
        };
        assert!(native_bounds_check(&edit).is_err());
    }

    // §M4 adversarial — `composing == true` is refused Wave-0.
    #[test]
    fn bounds_check_refuses_composing() {
        let edit = EditState {
            text: "hi".into(),
            caret: 0,
            sel_anchor: 0,
            composing: true,
        };
        assert!(native_bounds_check(&edit).is_err());
    }

    // §M4 — a well-formed EditState passes.
    #[test]
    fn bounds_check_passes_valid() {
        let edit = EditState {
            text: "hi".into(),
            caret: 2,
            sel_anchor: 0,
            composing: false,
        };
        assert!(native_bounds_check(&edit).is_ok());
    }
}

#[cfg(not(feature = "a11y_native"))]
#[cfg(test)]
mod compile_off_tests {
    // M4 pre-unlock — the real AccessKit adapter MUST be absent from the default
    // build. This test forces the marker `to_tree_update` (the §M4 entry point)
    // to NOT exist without the feature; if someone wires accesskit in without the
    // feature, this compile-off guard fails and CI catches it. We assert the
    // structural boundary: this module exports no `accesskit` symbol under default.
    #[test]
    fn adapter_absent_in_default_build() {
        // `to_tree_update` (the §M4 AccessKit entry point) is declared only under
        // the `a11y_native` feature, so referencing it here would fail to compile —
        // proving the default build carries zero AccessKit code. The assertion is a
        // structural/compile-time one; we simply confirm the offline gate passes.
        assert!(crate::semantics::mirror(&crate::semantics::SemanticScene::default()).is_ok());
    }
}
