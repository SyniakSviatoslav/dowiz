//! Item 40 — Per-layer golden-CRC32 self-check (hard-fail to safe state).
//!
//! ## What it does
//! Build-time, a set of PINNED test vectors (a subset of item 34's domain `D`) is fed
//! through the item-37 oracle (the truth source for goldens). Each layer's **output bytes**
//! are CRC32'd with [`crate::fdr::crc32`], and the per-(vector, layer) checksums are
//! **committed as constants** (`GOLDEN_L1`, `GOLDEN_HIDDEN`, `GOLDEN_LOGITS` below). Runtime,
//! the SAME pinned vectors are re-run through the oracle and each layer's output CRC32 is
//! recomputed and compared to the golden. On mismatch the self-check **hard-fails to a safe
//! state** — it returns `Err(ChecksumFault{layer})` and NEVER returns the computed output; it
//! also emits a typed `Kind::Alarm` FDR entry recording the faulting layer.
//!
//! ## Seam split (pure core vs kernel shim)
//! This module is the **pure no_std half**: the integer/checksum math, the golden constants,
//! the check driver ([`self_check_core`]), and the production entry points
//! ([`self_check_all`], [`self_check_all_with_weights`],
//! [`self_check_all_with_activation_fault`]). The std-dependent half — the durable FDR
//! `RingHandle` write that records the fault to a file-backed ring — lives in `dowiz-kernel`'s
//! `inference::golden` shim (`fault` + `self_check_all_into_ring`), which re-exports this
//! module (`pub use dowiz_core::inference::golden::*`) and drives [`self_check_core`] with its
//! ring-writing `fault` hook.
//!
//! ## Hard constraints honoured
//! - **P2 — one CRC32.** Reuses the single, always-compiled [`crate::fdr::crc32`]
//!   (IEEE-802.3 reflected, hand-rolled, compile-time table — `fdr/mod.rs`). This module
//!   introduces NO CRC32 of its own.
//! - **Silent when healthy.** A healthy run returns `Ok(())` and emits NO FDR entry; the
//!   alarm path is only reached on a detected mismatch.
//! - **Hard-fail, never return output.** The mismatch arms route through the fault hook,
//!   which returns the `Err` (and, on the kernel's ring path, writes the typed FDR entry).
//!   The caller MUST trap to a safe state and MUST NOT use the computed output.
//!
//! ## HONEST LIMIT (architect, failure-first — acceptance B5.7)
//! The golden self-check catches **persistent** faults: corrupted `.rodata` weights, a stuck
//! compute unit, a bit-rotted matmul — anything that changes the *pinned-vector* layer
//! outputs. A **transient single-inference bit-flip on an arbitrary real input has no
//! precomputed golden and is NOT caught** by this mechanism; that class needs redundant
//! computation (dual-run compare / TMR), which is **named out-of-scope** here. This limit is
//! stated so the mechanism is not over-claimed as full runtime integrity.

use alloc::vec::Vec;
use crate::fdr::crc32;
use crate::inference::oracle::{oracle_matmul_i8, oracle_relu_i32, oracle_requantize};
use crate::inference::spec::{B1, B2, SCALE1, SCALE2, W1, W2};
use crate::inference::workspace::{C, H, N};

/// A layer whose output is golden-CRC-checked.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layer {
    /// Layer-1 pre-requantize accumulator (`Σ input·W1 + B1`), the raw matmul output.
    L1PreRequant,
    /// The hidden activation after requantize + ReLU (the input to layer 2).
    Hidden,
    /// The final post-requantize logits (the `y` of `f(x)=y`).
    Logits,
}

impl Layer {
    pub fn as_str(self) -> &'static str {
        match self {
            Layer::L1PreRequant => "L1PreRequant",
            Layer::Hidden => "Hidden",
            Layer::Logits => "Logits",
        }
    }
}

/// The hard-fail evidence: a checksum mismatch was detected at `layer`.
///
/// Returned (never implicitly swallowed) so the caller can trap to a safe state and MUST
/// NOT use the computed output. Mirrors the interim item-9 composition's `Tripped` leg.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ChecksumFault {
    pub layer: Layer,
}

/// The frozen weight set the golden self-check is built against (item 34's weights).
/// Held by reference so the planted-fault tests can swap in a corrupted copy without
/// touching the committed `spec` constants (the source of truth stays intact).
pub struct Weights<'a> {
    pub w1: &'a [i8; N * H],
    pub b1: &'a [i8; H],
    pub scale1: u32,
    pub w2: &'a [i8; H * C],
    pub b2: &'a [i8; C],
    pub scale2: u32,
}

impl Weights<'static> {
    /// The committed, frozen toy-pilot weights (item 34).
    pub fn spec() -> Self {
        Weights {
            w1: &W1,
            b1: &B1,
            scale1: SCALE1,
            w2: &W2,
            b2: &B2,
            scale2: SCALE2,
        }
    }
}

/// The pinned test vectors — a subset of item 34's domain `D = [Q_MIN, Q_MAX]^N`.
/// These are frozen, public, synthetic inputs; the goldens below are their oracle layer
/// CRC32s. Distinct shapes (zero, ramp, all-max, all-min) exercise every layer.
pub const PINNED_VECTORS: [[i8; N]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 3, 4, 5, 6, 7, 8],
    [127, 127, 127, 127, 127, 127, 127, 127],
    [-127, -127, -127, -127, -127, -127, -127, -127],
];

// ── Golden CRC32 per layer, per pinned vector ───────────────────────────────────────
// Computed at build time via `fdr::crc32` over the item-37 oracle's layer outputs, then
// committed here as constants (proof B4.1). These are the REAL discovered values (see
// `golden_crcs_are_recomputed_from_oracle`, which proves they equal the oracle's output).
pub const GOLDEN_L1: [u32; 4] = [420107693, 2311962874, 251483362, 3178083991];
pub const GOLDEN_HIDDEN: [u32; 4] = [1696784233, 2828387249, 1886562370, 1942587440];
pub const GOLDEN_LOGITS: [u32; 4] = [558161692, 4291752815, 1562026689, 2268824821];

/// Reinterpret an `i8` slice as bytes (i8 has identical layout to u8 — trivially valid).
#[inline]
fn i8_to_bytes(a: &[i8]) -> &[u8] {
    // SAFETY: i8 and u8 share layout; no validity invariants are broken by the view.
    unsafe { core::slice::from_raw_parts(a.as_ptr() as *const u8, a.len()) }
}

/// Reinterpret an `i32` slice as bytes (CRC32 over the raw little-endian accumulator).
#[inline]
fn i32_to_bytes(a: &[i32]) -> &[u8] {
    // SAFETY: any bit pattern is a valid u8 sequence; alignment is satisfied (i32 ≥ u8).
    unsafe { core::slice::from_raw_parts(a.as_ptr() as *const u8, a.len() * 4) }
}

/// The per-layer outputs of one forward pass, captured for CRC32.
struct LayerOut {
    l1: Vec<i32>,    // layer-1 pre-requantize accumulator
    hidden: Vec<i8>, // hidden activation (requant + relu)
    logits: Vec<i8>, // final post-requant logits
}

/// Run the oracle forward pass over `input`, capturing each layer's output.
///
/// `hidden_override` (item-40 activation-injection path) replaces the layer-1 hidden with a
/// caller-supplied (possibly corrupted) activation before layer 2 — used by the planted
/// activation-fault test to corrupt a *mid-pipeline* activation rather than a weight.
/// `wk` supplies the weights (the committed `spec` set, or a corrupted copy).
fn compute_layers(input: &[i8; N], hidden_override: Option<&[i8; H]>, wk: &Weights) -> LayerOut {
    // Layer 1: hidden[h] = requant(relu(Σ_n input[n]*w1[h][n] + b1[h]))
    //          (pre-requant accumulator = l1[h] + b1[h]).
    let l1 = oracle_matmul_i8(input, wk.w1, 1, N, H).expect("layer1 fits i32");
    let mut hidden = vec![0i8; H];
    for h in 0..H {
        let acc = l1[h] + wk.b1[h] as i32;
        let rq = oracle_requantize(acc, wk.scale1);
        hidden[h] = oracle_relu_i32(rq as i32) as i8;
    }
    // Layer 2: logits[c] = requant(Σ_h hidden[h]*w2[c][h] + b2[c]).
    let hidden_ref: &[i8] = match hidden_override {
        Some(hv) => hv,
        None => &hidden,
    };
    let l2 = oracle_matmul_i8(hidden_ref, wk.w2, 1, H, C).expect("layer2 fits i32");
    let mut logits = vec![0i8; C];
    for c in 0..C {
        let acc = l2[c] + wk.b2[c] as i32;
        logits[c] = oracle_requantize(acc, wk.scale2);
    }
    LayerOut { l1, hidden, logits }
}

/// Recompute the goldens from the oracle over the frozen weights + pinned vectors.
/// Used by `golden_crcs_are_recomputed_from_oracle` to prove the committed constants ARE
/// the item-37 oracle's outputs (a differential, not a magic number; proof B5.1/B5.5).
fn compute_goldens() -> ([u32; 4], [u32; 4], [u32; 4]) {
    let wk = Weights::spec();
    let mut l1 = [0u32; 4];
    let mut hidden = [0u32; 4];
    let mut logits = [0u32; 4];
    for (i, v) in PINNED_VECTORS.iter().enumerate() {
        let out = compute_layers(v, None, &wk);
        l1[i] = crc32(i32_to_bytes(&out.l1));
        hidden[i] = crc32(i8_to_bytes(&out.hidden));
        logits[i] = crc32(i8_to_bytes(&out.logits));
    }
    (l1, hidden, logits)
}

/// The hard-fail trap (pure half): emit the typed FDR `Kind::Alarm` entry recording the
/// faulting layer, then return `Err(ChecksumFault{layer})`. The alarm routes through the
/// no_std `fdr::emit_alarm` hook (a no-op unless an FDR sink is installed, so a healthy run
/// — which never reaches this fn — is silent). The kernel's std shim adds the durable
/// `RingHandle` write on top for its test/standalone `self_check_all_into_ring` path.
fn fault(layer: Layer) -> Result<(), ChecksumFault> {
    crate::fdr::emit_alarm("checksum_fault", &format!("layer={}", layer.as_str()));
    Err(ChecksumFault { layer })
}

/// Compare one vector's layer CRC32s against the committed goldens; hard-fail on the first
/// mismatch (never returns the computed output). `hidden_override` enables activation
/// injection. The fault is delegated to `on_fault` (which returns the `Err`), so the pure
/// production path and the kernel's ring path share ONE check implementation.
fn check_vector<F>(
    idx: usize,
    input: &[i8; N],
    hidden_override: Option<&[i8; H]>,
    wk: &Weights,
    on_fault: &mut F,
) -> Result<(), ChecksumFault>
where
    F: FnMut(Layer) -> Result<(), ChecksumFault>,
{
    let out = compute_layers(input, hidden_override, wk);
    // First-mismatch ordering: L1 → Hidden → Logits. Any mismatch is a fault.
    if crc32(i32_to_bytes(&out.l1)) != GOLDEN_L1[idx] {
        return on_fault(Layer::L1PreRequant);
    }
    if crc32(i8_to_bytes(&out.hidden)) != GOLDEN_HIDDEN[idx] {
        return on_fault(Layer::Hidden);
    }
    if crc32(i8_to_bytes(&out.logits)) != GOLDEN_LOGITS[idx] {
        return on_fault(Layer::Logits);
    }
    Ok(())
}

/// Core driver: run the golden self-check over all pinned vectors, delegating each detected
/// mismatch to `on_fault` (which returns the `Err(ChecksumFault)`). The pure production
/// paths pass [`fault`] (alarm via the no_std hook); the kernel's std shim passes its
/// ring-writing `fault` for `self_check_all_into_ring`.
pub fn self_check_core<F>(
    wk: &Weights,
    activation_override: Option<&[i8; H]>,
    mut on_fault: F,
) -> Result<(), ChecksumFault>
where
    F: FnMut(Layer) -> Result<(), ChecksumFault>,
{
    for (idx, v) in PINNED_VECTORS.iter().enumerate() {
        let ov = if idx == 0 { activation_override } else { None };
        check_vector(idx, v, ov, wk, &mut on_fault)?;
    }
    Ok(())
}

/// Run the golden self-check over ALL pinned vectors with the *current* (possibly
/// corrupted) frozen weights. Returns the first fault found; `Ok(())` means silent + healthy.
/// Alarm (on fault) goes to the process-global FDR sink — the interim typed trap + FDR entry.
pub fn self_check_all() -> Result<(), ChecksumFault> {
    let wk = Weights::spec();
    self_check_core(&wk, None, fault)
}

/// As [`self_check_all`] but overrides the weight set (planted single-bit WEIGHT-fault path).
pub fn self_check_all_with_weights(wk: &Weights) -> Result<(), ChecksumFault> {
    self_check_core(wk, None, fault)
}

/// As [`self_check_all`] but corrupts the layer-1→layer-2 **activation** of pinned vector 0
/// with `hidden_override` (planted single-bit activation-fault path). All other vectors use
/// the oracle-computed hidden.
pub fn self_check_all_with_activation_fault(
    hidden_override: &[i8; H],
) -> Result<(), ChecksumFault> {
    let wk = Weights::spec();
    self_check_core(&wk, Some(hidden_override), fault)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §B5.1 / B4.1 — the committed goldens ARE the item-37 oracle's outputs: recomputing
    /// them from the oracle must equal the frozen constants. This ties the goldens to the
    /// schoolbook (a differential, not a magic number) and trips RED if the constants are
    /// emptied (proof B5.5).
    #[test]
    fn golden_crcs_are_recomputed_from_oracle() {
        let (l1, hidden, logits) = compute_goldens();
        assert_eq!(
            l1, GOLDEN_L1,
            "GOLDEN_L1 must equal the recomputed oracle CRC"
        );
        assert_eq!(
            hidden, GOLDEN_HIDDEN,
            "GOLDEN_HIDDEN must equal the recomputed oracle CRC"
        );
        assert_eq!(
            logits, GOLDEN_LOGITS,
            "GOLDEN_LOGITS must equal the recomputed oracle CRC"
        );
    }
}
