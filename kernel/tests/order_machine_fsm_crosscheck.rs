//! Cross-module cross-check relocated out of `spectral` when that module was
//! extracted to no_std `dowiz-core` (which cannot see the kernel's
//! `order_machine`). The check lives in `spectral`'s old test module as
//! `green_crosscheck_live_fsm_is_acyclic`: the live lifecycle FSM is a DAG, so
//! its spectral radius ρ ≈ 0 — verified against the INDEPENDENT
//! `order_machine::spectral_radius()` power-iteration (not against spectral's
//! own Faddeev path), so the two implementations cross-validate each other.

#[test]
fn live_fsm_is_acyclic_spectral_radius_is_zero() {
    assert!(dowiz_kernel::order_machine::spectral_radius() < 1e-9);
}
