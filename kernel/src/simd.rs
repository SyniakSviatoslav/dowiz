//! simd.rs — P11 §6 `f64x4` struct-of-arrays (SoA) SIMD batch lane + the Layer-E
//! N-courier Kalman SoA consumer (BLUEPRINT-P-E §13).
//!
//! **Design rule that guarantees bit-identity (BLUEPRINT-P11 §6 / P-E §2.1):** vectorise
//! *ACROSS* the batch of independent rows/filters, never *WITHIN* a single row's reduction. A
//! 4-wide register holds 4 independent rows' (or 4 independent 1-D courier axis-filters') values
//! at the same column index; each lane replays the *exact* scalar op sequence for its own row —
//! row-max, `exp(x−max)`, the fixed left-to-right sum, the divide; or, for the Kalman lane, the
//! exact `predict`/`update` arithmetic of [`crate::kalman::AxisKalman`] — so per-lane arithmetic
//! order is unchanged and the lane is bit-identical to the scalar single-row / single-courier path.
//!
//! Runtime detection mirrors `householder.rs` (`is_x86_feature_detected!("avx2")` → AVX2 lane;
//! scalar fallback otherwise — the `softmax_batch_lane` / `kalman_batch_step` shape). No new
//! dependency. The bit-identity holds in BOTH paths: the scalar fallback trivially matches, and
//! the AVX2 lane matches because per-lane op order is unchanged (AVX2 packed-f64 arithmetic is
//! IEEE-identical to scalar SSE2 f64 per element).
//!
//! **Layer-E consumers:**
//!   * `softmax_batch_lane` — the batch-of-rows softmax reduction (P11 §6).
//!   * `kalman_batch_step` — the N-courier **Kalman SoA consumer** (BLUEPRINT-P-E §13.2). It
//!     batches N independent couriers' existing per-courier 1-D Kalman `predict`/`update`
//!     (`crate::kalman::CourierKalman`, the B1 "Brain+Body" organ) across the same 4-wide AVX2
//!     lane, bit-identical to stepping each courier once in scalar sequence.
//!
//! **Authority is untouched (BLUEPRINT-P-E §13.3 anti-scope):** the batched fn only applies the
//! EXISTING `predict`/`update` semantics; it never exposes raw `x`/`P` mutation bypassing
//! `predict`/`update`, never changes cadence/locking/which task authors courier state, and is
//! only ever invoked from inside the caller's ownership boundary (the caller already holds
//! `&mut CourierKalman` for couriers whose fold events are already co-occurring). No courier
//! identity/reputation change — NO-COURIER-SCORING red line respected: no new scoring/ranking/
//! gating use of the trust estimate is added (anti-scope §13.3.4).

use crate::kalman::{AxisState, CourierKalman};

#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicU64, Ordering};

// ── Native telemetry probe (mandatory-telemetry doctrine) ───────────────────
// Node-local counters so a dispatch silently falling back to scalar on a host
// that should have AVX2 shows up automatically, not at review time.
#[cfg(not(target_arch = "wasm32"))]
static KALMAN_DISPATCH_SIMD: AtomicU64 = AtomicU64::new(0);
#[cfg(not(target_arch = "wasm32"))]
static KALMAN_DISPATCH_SCALAR: AtomicU64 = AtomicU64::new(0);
#[cfg(not(target_arch = "wasm32"))]
static KALMAN_STEPS: AtomicU64 = AtomicU64::new(0);

/// Snapshot of the Kalman-SoA lane telemetry (dispatch pole + total stepped couriers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KalmanLaneTelemetry {
    /// Number of `kalman_batch_step` calls that took the AVX2 lane path.
    pub dispatch_simd: u64,
    /// Number of `kalman_batch_step` calls that took the scalar path
    /// (AVX2 absent, non-x86_64, or wasm).
    pub dispatch_scalar: u64,
    /// Total couriers stepped across all calls.
    pub steps: u64,
}

/// Read the current Kalman-SoA lane telemetry. On wasm this returns zeros
/// (the probe is native-only so the wasm cdylib stays lean).
pub fn kalman_lane_telemetry() -> KalmanLaneTelemetry {
    #[cfg(not(target_arch = "wasm32"))]
    {
        KalmanLaneTelemetry {
            dispatch_simd: KALMAN_DISPATCH_SIMD.load(Ordering::Relaxed),
            dispatch_scalar: KALMAN_DISPATCH_SCALAR.load(Ordering::Relaxed),
            steps: KALMAN_STEPS.load(Ordering::Relaxed),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        KalmanLaneTelemetry {
            dispatch_simd: 0,
            dispatch_scalar: 0,
            steps: 0,
        }
    }
}

// =========================================================================
// §6 — `f64x4` softmax SoA batch lane (the proven substrate pattern).
// =========================================================================

/// Scalar reference softmax — mirror of `attention.rs::softmax` *exactly* (same
/// op order, same `exps.iter().sum()` left-to-right reduction). Used by the
/// bit-identity falsifier tests and as the scalar fallback path.
pub fn softmax_scalar(xs: &[f64]) -> Vec<f64> {
    if xs.is_empty() {
        return Vec::new();
    }
    let mut m = xs[0];
    for &x in &xs[1..] {
        if x > m {
            m = x;
        }
    }
    let exps: Vec<f64> = xs.iter().map(|&x| (x - m).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.iter().map(|&e| e / sum).collect()
}

/// Process up to 4 independent softmax rows per SIMD step (struct-of-arrays).
///
/// Bit-identical to calling [`softmax_scalar`] once per row. Only compiled on
/// x86_64 with the AVX2 target feature (caller must guarantee the CPU has AVX2
/// via `is_x86_feature_detected!`). `rows` has length 1..=4.
///
/// SAFETY/CORRECTNESS notes (no `unsafe` preconditions on the *caller* beyond
/// the AVX2 feature gate — all intrinsic use is in-bounds):
///   * `max` is computed first in a separate pass with `-inf` padding for
///     short/inactive lanes, so padding can never contaminate the row max.
///   * The exponential/sum pass zero-pads short/inactive lanes *before* the
///     per-lane add, so a padded lane contributes exactly 0.0 to its own sum
///     and is never written to output.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn softmax_lane4(rows: &[&[f64]]) -> Vec<Vec<f64>> {
    use core::arch::x86_64::*;
    let k = rows.len(); // 1..=4
    debug_assert!(k <= 4 && k >= 1);

    // Active lanes = non-empty rows. Inactive (empty) rows produce an empty
    // output and are masked out of every SIMD reduction (kept NaN-free).
    let mut active = [false; 4];
    let mut lens = [0usize; 4];
    let mut max_len = 0usize;
    for (lane, r) in rows.iter().enumerate() {
        active[lane] = !r.is_empty();
        lens[lane] = r.len();
        if r.len() > max_len {
            max_len = r.len();
        }
    }

    // ── Pass 1: per-lane row-max (exact, order-free). ──────────────────────
    // Init to -inf; max with -inf-padded short lanes leaves the real max intact.
    let mut max_arr = [f64::NEG_INFINITY; 4];
    let mut max_vec = _mm256_loadu_pd(max_arr.as_ptr());
    for j in 0..max_len {
        let mut vals = [f64::NEG_INFINITY; 4];
        for lane in 0..k {
            if active[lane] && j < lens[lane] {
                vals[lane] = rows[lane][j];
            }
        }
        let val_vec = _mm256_loadu_pd(vals.as_ptr());
        max_vec = _mm256_max_pd(max_vec, val_vec);
    }
    _mm256_storeu_pd(max_arr.as_mut_ptr(), max_vec);
    // Reload as a *constant* vector reused across the exp/sum pass.
    let max_vec = _mm256_loadu_pd(max_arr.as_ptr());

    // ── Pass 2: exp(x-max) + left-to-right per-lane sum, then divide. ───────
    let mut outs: Vec<Vec<f64>> = rows.iter().map(|r| vec![0.0f64; r.len()]).collect();
    // sum_vec holds 4 independent running accumulators (one per lane).
    let mut sum_vec = _mm256_setzero_pd();

    for j in 0..max_len {
        // Gather this column's 4 values (0.0 pad for short/inactive lanes).
        let mut vals = [0.0f64; 4];
        for lane in 0..k {
            if active[lane] && j < lens[lane] {
                vals[lane] = rows[lane][j];
            }
        }
        let val_vec = _mm256_loadu_pd(vals.as_ptr());
        // diff = val - max ; per-element subtraction → identical to scalar.
        let diff = _mm256_sub_pd(val_vec, max_vec);
        let mut diff_arr = [0.0f64; 4];
        _mm256_storeu_pd(diff_arr.as_mut_ptr(), diff);
        // exp is a per-element unary op (order-free); extract + call f64::exp.
        let mut exp_arr = [0.0f64; 4];
        for lane in 0..4 {
            exp_arr[lane] = diff_arr[lane].exp();
        }
        // Zero out short/inactive lanes so they add exactly 0.0 to their own
        // sum and are never written to output (keeps NaN-free + bit-clean).
        for lane in 0..k {
            if !(active[lane] && j < lens[lane]) {
                exp_arr[lane] = 0.0;
            }
        }
        let exp_vec = _mm256_loadu_pd(exp_arr.as_ptr());
        // Per-lane add, in COLUMN ORDER = scalar left-to-right sum. Identical.
        sum_vec = _mm256_add_pd(sum_vec, exp_vec);
        // Store the (real) exps into each active row's output buffer.
        for lane in 0..k {
            if active[lane] && j < lens[lane] {
                outs[lane][j] = exp_arr[lane];
            }
        }
    }

    // ── Divide each exp by its row's (per-lane) sum — same as scalar. ────────
    let mut sum_arr = [0.0f64; 4];
    _mm256_storeu_pd(sum_arr.as_mut_ptr(), sum_vec);
    for lane in 0..k {
        if !active[lane] {
            continue; // empty row → already empty output
        }
        let s = sum_arr[lane];
        for j in 0..lens[lane] {
            outs[lane][j] /= s;
        }
    }
    outs
}

/// Batch softmax over many independent rows, 4 rows per SIMD step.
///
/// Bit-identical to applying [`softmax_scalar`] to each row. Rows whose count is
/// not a multiple of 4 are handled by a scalar tail (also bit-identical). When
/// AVX2 is unavailable (or on non-x86_64) the entire batch falls back to the
/// scalar path.
pub fn softmax_batch_lane(rows: &[&[f64]]) -> Vec<Vec<f64>> {
    let mut out = Vec::with_capacity(rows.len());
    let mut i = 0;

    // AVX2 fast path: consume rows in chunks of 4 via the SoA lane.
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            while i + 4 <= rows.len() {
                // SAFETY: CPU verified to have AVX2 via is_x86_feature_detected.
                let res = unsafe { softmax_lane4(&rows[i..i + 4]) };
                out.extend(res);
                i += 4;
            }
        }
    }

    // Scalar tail — covers both the <4 remainder AND every row when AVX2 is
    // not detected (the whole batch runs through here). Bit-identical either way.
    for r in &rows[i..] {
        out.push(softmax_scalar(r));
    }
    out
}

// =========================================================================
// §13 — N-courier Kalman SoA consumer (WAVE D / BLUEPRINT-P-E §13.2).
//
// Batches N independent couriers' existing per-courier 1-D Kalman step
// (`crate::kalman::CourierKalman`, the B1 organ) into one SIMD-lane pass.
// Mirrors the AVX2-detect + scalar-fallback shape of `softmax_batch_lane`, and
// the *exact* op order of `AxisKalman::predict`/`update` so the batched result
// is `f64::to_bits()`-identical to stepping each courier once in scalar
// sequence (parity test `kalman_batch_bit_identical`).
// =========================================================================

/// Advance 4 independent 1-D courier axis-filters (one lane each) by one
/// predict→(optional update) step, entirely inside one AVX2 lane.
///
/// `states` holds the 4 lanes' canonical `(pos, vel, P)`; on return it holds the
/// advanced state. `dts`/`q_pos`/`q_vel`/`rs` are per-lane (each courier may
/// have its own noise config and dt). `zs[lane]` is `Some(z)` → run the update
/// with measurement `z` (variance `rs[lane]`); `None` → hold prior (update is
/// skipped, only the predict inflation applies), exactly matching
/// `AxisKalman::update`'s contract.
///
/// **Bit-identity:** the predict step is an order-free packed-f64 add (identical
/// to scalar `a+b`), and the update step replays `AxisKalman::update`'s EXACT
/// op sequence per lane. No algebra is combined across lanes — each lane's
/// output feeds only its own state. So the result equals stepping each of the
/// 4 lanes once in scalar sequence, `to_bits()`-identically.
///
/// SAFETY/CORRECTNESS: caller must have guaranteed the CPU has AVX2 via
/// `is_x86_feature_detected!`. All intrinsic use is in-bounds.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn kalman_lane4_axis(
    states: &mut [AxisState; 4],
    dts: &[f64; 4],
    q_pos: &[f64; 4],
    q_vel: &[f64; 4],
    rs: &[f64; 4],
    zs: &[Option<f64>; 4],
) {
    use core::arch::x86_64::*;

    // ── gather the 6 canonical fields into SoA [f64;4] vectors ──────────────
    let mut pos = [0.0f64; 4];
    let mut vel = [0.0f64; 4];
    let mut p_pp = [0.0f64; 4];
    let mut p_pv = [0.0f64; 4];
    let mut p_vp = [0.0f64; 4];
    let mut p_vv = [0.0f64; 4];
    // `None` ⇒ hold-prior: mask the lane out of the update (set z=0, active=0)
    // so the masked update produces EXACTLY the predicted state (bit-identical
    // to the scalar `None` path, where update is skipped).
    let mut z = [0.0f64; 4];
    let mut act = [0.0f64; 4];
    for l in 0..4 {
        pos[l] = states[l].pos;
        vel[l] = states[l].vel;
        p_pp[l] = states[l].p_pp;
        p_pv[l] = states[l].p_pv;
        p_vp[l] = states[l].p_vp;
        p_vv[l] = states[l].p_vv;
        match zs[l] {
            Some(zv) => {
                z[l] = zv;
                act[l] = 1.0;
            }
            None => {
                z[l] = 0.0;
                act[l] = 0.0;
            }
        }
    }
    let act_v = _mm256_loadu_pd(act.as_ptr());

    // ── PREDICT (the Body): x ← Fx ; P ← FPFᵀ + Q (AVX2 packed-f64, IEEE == scalar) ──
    let dt_v = _mm256_loadu_pd(dts.as_ptr());
    let qp_v = _mm256_loadu_pd(q_pos.as_ptr());
    let qv_v = _mm256_loadu_pd(q_vel.as_ptr());

    let pos_v = _mm256_loadu_pd(pos.as_ptr());
    let vel_v = _mm256_loadu_pd(vel.as_ptr());
    let ppp_v = _mm256_loadu_pd(p_pp.as_ptr());
    let ppv_v = _mm256_loadu_pd(p_pv.as_ptr());
    let pvp_v = _mm256_loadu_pd(p_vp.as_ptr());
    let pvv_v = _mm256_loadu_pd(p_vv.as_ptr());

    // pos' = pos + vel*dt   (left-associative, matches scalar `self.pos += self.vel*dt`)
    let pos_n = _mm256_add_pd(pos_v, _mm256_mul_pd(vel_v, dt_v));
    // p_pp' = p_pp + dt*(p_pv + p_vp) + dt*dt*p_vv + q_pos
    //         — grouped EXACTLY as scalar `self.p_pp + dt*(..) + dt*dt*self.p_vv + q_pos`
    //           (((p_pp + dt*(p_pv+p_vp)) + dt*dt*p_vv) + q_pos), so the packed-f64 sum
    //           reassociates identically to the scalar left-to-right sum.
    let inner = _mm256_mul_pd(dt_v, _mm256_add_pd(ppv_v, pvp_v)); // dt*(p_pv + p_vp)
    let t1 = _mm256_add_pd(ppp_v, inner); // p_pp + dt*(p_pv+p_vp)
    let dt2 = _mm256_mul_pd(dt_v, dt_v); // dt*dt
    let t2 = _mm256_mul_pd(dt2, pvv_v); // dt*dt*p_vv
    let t3 = _mm256_add_pd(t1, t2); // (p_pp + dt*(..)) + dt*dt*p_vv
    let ppp_n = _mm256_add_pd(t3, qp_v); // + q_pos
    // p_pv' = p_pv + dt*p_vv   (uses pre-predict p_vv)
    let ppv_n = _mm256_add_pd(ppv_v, _mm256_mul_pd(dt_v, pvv_v));
    // p_vp' = p_vp + dt*p_vv
    let pvp_n = _mm256_add_pd(pvp_v, _mm256_mul_pd(dt_v, pvv_v));
    // p_vv' = p_vv + q_vel
    let pvv_n = _mm256_add_pd(pvv_v, qv_v);

    // ── UPDATE (the Brain): packed-f64 across the 4 lanes; `None` masked. ──
    // Replays `AxisKalman::update` EXACTLY — every op is the same f64 op the
    // scalar runs (S = p_pp' + r ; K_p = p_pp'/S ; K_v = p_vp'/S ; y = z − pos'
    // ; x ← x + K·y ; P ← (I−KH)P with I−KH = [[1−K_p,0],[−K_v,1]]). Because
    // the division and products are per-lane packed-f64 (IEEE == scalar per
    // element) and `act[l]==0` forces K·y ≡ 0 for that lane, the `None` lane
    // reduces to the predicted state bit-identically to the scalar hold-prior
    // path. No algebra crosses a lane; no cross-courier contamination possible.
    let one_v = _mm256_set1_pd(1.0);
    let z_v = _mm256_loadu_pd(z.as_ptr());
    let r_v = _mm256_loadu_pd(rs.as_ptr());

    let s_v = _mm256_add_pd(ppp_n, r_v); // S = p_pp' + r
    let kp_v = _mm256_div_pd(ppp_n, s_v); // K_p = p_pp' / S
    let kv_v = _mm256_div_pd(pvp_n, s_v); // K_v = p_vp' / S
    let y_v = _mm256_sub_pd(z_v, pos_n); // y = z − pos'
    // Mask the gains/innovation to 0 for `None` (hold-prior) lanes.
    let kp_m = _mm256_mul_pd(kp_v, act_v);
    let kv_m = _mm256_mul_pd(kv_v, act_v);
    let y_m = _mm256_mul_pd(y_v, act_v);
    // x ← x + K·y ; vel ← vel + K_v·y
    let pos_x = _mm256_add_pd(pos_n, _mm256_mul_pd(kp_m, y_m));
    let vel_x = _mm256_add_pd(vel_v, _mm256_mul_pd(kv_m, y_m));
    // P ← (I − KH)P, with I − KH = [[1−K_p, 0],[−K_v, 1]] (uses PRE-update predicted P)
    let pp_x = _mm256_mul_pd(_mm256_sub_pd(one_v, kp_m), ppp_n);
    let pv_x = _mm256_mul_pd(_mm256_sub_pd(one_v, kp_m), ppv_n);
    let vp_x = _mm256_sub_pd(pvp_n, _mm256_mul_pd(kv_m, ppp_n));
    let vv_x = _mm256_sub_pd(pvv_n, _mm256_mul_pd(kv_m, ppv_n));

    // Scatter the advanced state back into each lane (no cross-lane store).
    let mut opos = [0.0f64; 4];
    let mut ovel = [0.0f64; 4];
    let mut op_pp = [0.0f64; 4];
    let mut op_pv = [0.0f64; 4];
    let mut op_vp = [0.0f64; 4];
    let mut op_vv = [0.0f64; 4];
    _mm256_storeu_pd(opos.as_mut_ptr(), pos_x);
    _mm256_storeu_pd(ovel.as_mut_ptr(), vel_x);
    _mm256_storeu_pd(op_pp.as_mut_ptr(), pp_x);
    _mm256_storeu_pd(op_pv.as_mut_ptr(), pv_x);
    _mm256_storeu_pd(op_vp.as_mut_ptr(), vp_x);
    _mm256_storeu_pd(op_vv.as_mut_ptr(), vv_x);
    for l in 0..4 {
        states[l] = AxisState {
            pos: opos[l],
            vel: ovel[l],
            p_pp: op_pp[l],
            p_pv: op_pv[l],
            p_vp: op_vp[l],
            p_vv: op_vv[l],
        };
    }
}

/// Step N couriers' existing per-courier Kalman `predict`/`update` in one SoA pass.
///
/// `couriers[i]` is advanced EXACTLY as a scalar sequence of
/// `courier.lat.predict(dt, q, qv); if let Some((la,_))=obs { courier.lat.update(la, r) };`
/// `courier.lng.predict(...); if let Some((_,lo))=obs { courier.lng.update(lo, r) }` would —
/// i.e., each courier's own `CourierKalman` arithmetic, never combined with another courier's.
/// `observations[i]` is the `Option<(lat_obs, lng_obs)>` for courier `i` (`Some` → predict +
/// update; `None` → predict-only, fail-closed hold-prior, exactly like a missing observation).
///
/// Bit-identical to stepping each courier once in scalar sequence
/// (`f64::to_bits()`-exact; parity test `kalman_batch_bit_identical` proves it, over N that are
/// not multiples of the lane width). Couriers whose count is not a multiple of 4 run through the
/// scalar tail; the whole batch runs through the scalar reference when AVX2 is absent. Neither
/// diverges from the scalar verdict.
///
/// **No authority change (anti-scope §13.3):** this fn only applies the EXISTING
/// `predict`/`update` semantics; it never exposes raw state mutation bypassing them, never
/// changes which task authors courier state, never introduces a new cadence (it batches only
/// couriers whose events are already co-occurring at the caller). NO-COURIER-SCORING respected.
pub fn kalman_batch_step(
    couriers: &mut [CourierKalman],
    dt: f64,
    observations: &[Option<(f64, f64)>],
) {
    assert_eq!(
        couriers.len(),
        observations.len(),
        "kalman_batch_step: mismatched couriers/observations counts"
    );
    let n = couriers.len();
    let mut i = 0;

    // AVX2 fast path. Gated on x86_64 (std is always present in this crate; the
    // wasm/arm builds compile this out entirely via target_arch). A runtime
    // `is_x86_feature_detected!` check keeps the scalar reference as the
    // total-function fallback on any host without AVX2.
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            KALMAN_DISPATCH_SIMD.fetch_add(1, Ordering::Relaxed);
            while i + 4 <= n {
                // Gather the 4 active lanes' (lat, lng) states + per-courier config.
                let mut lat = [
                    couriers[i].lat.state(),
                    couriers[i + 1].lat.state(),
                    couriers[i + 2].lat.state(),
                    couriers[i + 3].lat.state(),
                ];
                let mut lng = [
                    couriers[i].lng.state(),
                    couriers[i + 1].lng.state(),
                    couriers[i + 2].lng.state(),
                    couriers[i + 3].lng.state(),
                ];
                let q_pos = [
                    couriers[i].q_pos(),
                    couriers[i + 1].q_pos(),
                    couriers[i + 2].q_pos(),
                    couriers[i + 3].q_pos(),
                ];
                let q_vel = [
                    couriers[i].q_vel(),
                    couriers[i + 1].q_vel(),
                    couriers[i + 2].q_vel(),
                    couriers[i + 3].q_vel(),
                ];
                let rs = [
                    couriers[i].r(),
                    couriers[i + 1].r(),
                    couriers[i + 2].r(),
                    couriers[i + 3].r(),
                ];
                let dts = [dt; 4];
                let z_lat = [
                    observations[i].map(|(la, _)| la),
                    observations[i + 1].map(|(la, _)| la),
                    observations[i + 2].map(|(la, _)| la),
                    observations[i + 3].map(|(la, _)| la),
                ];
                let z_lng = [
                    observations[i].map(|(_, lo)| lo),
                    observations[i + 1].map(|(_, lo)| lo),
                    observations[i + 2].map(|(_, lo)| lo),
                    observations[i + 3].map(|(_, lo)| lo),
                ];
                // SAFETY: AVX2 verified via is_x86_feature_detected. Each lane is
                // one courier's axis-filter; per-courier op order == scalar.
                unsafe {
                    kalman_lane4_axis(&mut lat, &dts, &q_pos, &q_vel, &rs, &z_lat);
                    kalman_lane4_axis(&mut lng, &dts, &q_pos, &q_vel, &rs, &z_lng);
                }
                // Scatter the advanced state back into each courier (no cross-lane store).
                for l in 0..4 {
                    couriers[i + l].lat.set_state(lat[l]);
                    couriers[i + l].lng.set_state(lng[l]);
                }
                i += 4;
            }
        } else {
            KALMAN_DISPATCH_SCALAR.fetch_add(1, Ordering::Relaxed);
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        // wasm/arm/etc: no AVX2 lane compiled; whole batch is scalar.
        KALMAN_DISPATCH_SCALAR.fetch_add(1, Ordering::Relaxed);
    }

    // Scalar tail — the <4 remainder AND every courier when AVX2 is absent
    // (the whole batch runs through the bit-identical reference path).
    for j in i..n {
        let (q_pos, q_vel, r) = (couriers[j].q_pos(), couriers[j].q_vel(), couriers[j].r());
        couriers[j].lat.predict(dt, q_pos, q_vel);
        if let Some((la, _)) = observations[j] {
            couriers[j].lat.update(la, r);
        }
        couriers[j].lng.predict(dt, q_pos, q_vel);
        if let Some((_, lo)) = observations[j] {
            couriers[j].lng.update(lo, r);
        }
    }
    KALMAN_STEPS.fetch_add(n as u64, Ordering::Relaxed);
}

/// Guaranteed-scalar variant of [`kalman_batch_step`] (no AVX2 cfg gate). Provided so the parity
/// suite can prove the lane and the scalar path agree, and as the documented always-available
/// fallback. Bit-identical to [`kalman_batch_step`] on a host without AVX2.
pub fn kalman_batch_step_scalar(
    couriers: &mut [CourierKalman],
    dt: f64,
    observations: &[Option<(f64, f64)>],
) {
    assert_eq!(
        couriers.len(),
        observations.len(),
        "kalman_batch_step_scalar: mismatched couriers/observations counts"
    );
    KALMAN_DISPATCH_SCALAR.fetch_add(1, Ordering::Relaxed);
    for j in 0..couriers.len() {
        let (q_pos, q_vel, r) = (couriers[j].q_pos(), couriers[j].q_vel(), couriers[j].r());
        couriers[j].lat.predict(dt, q_pos, q_vel);
        if let Some((la, _)) = observations[j] {
            couriers[j].lat.update(la, r);
        }
        couriers[j].lng.predict(dt, q_pos, q_vel);
        if let Some((_, lo)) = observations[j] {
            couriers[j].lng.update(lo, r);
        }
    }
    KALMAN_STEPS.fetch_add(couriers.len() as u64, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kalman::CourierKalman;

    // ── deterministic PRNG (no std RNG, fixed seed → reproducible battery) ──
    fn lcg(state: &mut u64) -> u64 {
        // Numerical Recipes LCG; deterministic, no external dep.
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *state
    }
    fn f64_from(state: &mut u64, lo: f64, hi: f64) -> f64 {
        let u = lcg(state) >> 11; // 53-bit mantissa
        let unit = (u as f64) / (1u64 << 53) as f64;
        lo + unit * (hi - lo)
    }

    /// Scalar reference: step each courier exactly as the batch lane does, in
    /// plain sequential order. The parity test asserts `to_bits()`-identity.
    fn scalar_reference(
        couriers: &mut [CourierKalman],
        dt: f64,
        observations: &[Option<(f64, f64)>],
    ) {
        for (j, c) in couriers.iter_mut().enumerate() {
            let (q_pos, q_vel, r) = (c.q_pos(), c.q_vel(), c.r());
            c.lat.predict(dt, q_pos, q_vel);
            if let Some((la, _)) = observations[j] {
                c.lat.update(la, r);
            }
            c.lng.predict(dt, q_pos, q_vel);
            if let Some((_, lo)) = observations[j] {
                c.lng.update(lo, r);
            }
        }
    }

    /// Build `n` couriers with deterministic-but-diverse configs and a parallel
    /// `observations` vector, driven by a fixed-seed LCG.
    fn make_battery(n: usize, seed: u64) -> (Vec<CourierKalman>, Vec<Option<(f64, f64)>>, f64) {
        let mut st = seed;
        let dt = 1.0;
        let mut couriers = Vec::with_capacity(n);
        let mut obs = Vec::with_capacity(n);
        for _ in 0..n {
            let lat0 = f64_from(&mut st, -80.0, 80.0);
            let lng0 = f64_from(&mut st, -180.0, 180.0);
            let p0 = f64_from(&mut st, 10.0, 1e3);
            let q_pos = f64_from(&mut st, 1e-5, 1e-1);
            let q_vel = f64_from(&mut st, 1e-5, 1e-1);
            let r = f64_from(&mut st, 0.5, 20.0);
            let o = CourierKalman::new(lat0, lng0, p0, q_pos, q_vel, r);
            couriers.push(o);
            // ~1/3 of couriers get a None (hold-prior) observation to hit the
            // fail-closed path.
            let z = if lcg(&mut st) % 3 == 0 {
                None
            } else {
                Some((
                    f64_from(&mut st, lat0 - 5.0, lat0 + 5.0),
                    f64_from(&mut st, lng0 - 5.0, lng0 + 5.0),
                ))
            };
            obs.push(z);
        }
        (couriers, obs, dt)
    }

    fn bits_eq(a: &CourierKalman, b: &CourierKalman) -> bool {
        a.lat.state() == b.lat.state() && a.lng.state() == b.lng.state()
    }

    // ── T(parity): batched SoA == scalar per-courier law, to_bits() exact ──
    #[test]
    fn kalman_batch_bit_identical() {
        // Across several N (incl. non-multiples of 4) and seeds.
        for &n in &[0usize, 1, 2, 3, 4, 5, 7, 8, 11, 32, 64, 100] {
            for &seed in &[1u64, 2, 7, 42] {
                let (batch_in, obs, dt) = make_battery(n, seed);
                let (scalar_in, _, _) = (make_battery(n, seed).0, make_battery(n, seed).1, dt);

                let mut batch = batch_in;
                kalman_batch_step(&mut batch, dt, &obs);

                let mut scalar = scalar_in;
                scalar_reference(&mut scalar, dt, &obs);

                assert_eq!(
                    batch.len(),
                    scalar.len(),
                    "length mismatch n={n} seed={seed}"
                );
                for i in 0..batch.len() {
                    assert!(
                        bits_eq(&batch[i], &scalar[i]),
                        "lane diverged from scalar at n={n} seed={seed} i={i}: \
                         batch.lat={:?} scalar.lat={:?}",
                        batch[i].lat.state(),
                        scalar[i].lat.state()
                    );
                }
            }
        }
    }

    // ── T(non-multiple-of-4): the remainder path is still bit-identical ──
    #[test]
    fn kalman_batch_handles_non_multiple_of_four() {
        for &n in &[1usize, 2, 3, 5, 6, 9, 13, 101] {
            let (batch_in, obs, dt) = make_battery(n, 12345);
            let scalar_in = make_battery(n, 12345).0;
            let mut batch = batch_in;
            kalman_batch_step(&mut batch, dt, &obs);
            let mut scalar = scalar_in;
            scalar_reference(&mut scalar, dt, &obs);
            for i in 0..batch.len() {
                assert!(bits_eq(&batch[i], &scalar[i]), "diverged n={n} i={i}");
            }
        }
    }

    // ── T(None hold-prior): None observation ⇒ predict-only, no update ──
    #[test]
    fn kalman_batch_none_holds_prior() {
        let (mut batch, obs, dt) = make_battery(4, 99);
        // Force all to None to isolate the hold-prior path.
        let obs_none: Vec<Option<(f64, f64)>> = vec![None; 4];
        let scalar_in = make_battery(4, 99).0;
        let mut scalar = scalar_in;
        scalar_reference(&mut scalar, dt, &obs_none);
        kalman_batch_step(&mut batch, dt, &obs_none);
        for i in 0..4 {
            assert!(
                bits_eq(&batch[i], &scalar[i]),
                "hold-prior diverged at i={i}"
            );
        }
        // And confirm a Some path also matches (sanity, reuses battery obs).
        let mut batch2 = make_battery(4, 99).0;
        let mut scalar2 = make_battery(4, 99).0;
        kalman_batch_step(&mut batch2, dt, &obs);
        scalar_reference(&mut scalar2, dt, &obs);
        for i in 0..4 {
            assert!(bits_eq(&batch2[i], &scalar2[i]), "Some-path diverged i={i}");
        }
    }

    // ── T(scalar-path parity): the guaranteed-scalar fn agrees with the lane ──
    // Proves the suite does NOT silently depend on the AVX2 lane (mirrors the
    // P-E §3.2 "GREEN with AVX2 cfg'd off" requirement at the unit level).
    #[test]
    fn kalman_batch_scalar_path_matches_lane() {
        let (lane_in, obs, dt) = make_battery(37, 2026);
        let scalar_in = make_battery(37, 2026).0;
        let mut lane = lane_in;
        kalman_batch_step(&mut lane, dt, &obs); // AVX2 path on this host
        let mut scalar = scalar_in;
        kalman_batch_step_scalar(&mut scalar, dt, &obs); // always-scalar path
        for i in 0..lane.len() {
            assert!(
                bits_eq(&lane[i], &scalar[i]),
                "scalar-path diverged from lane at i={i}"
            );
        }
    }

    // ── softmax substrate parity (the proven pattern the kalman lane mirrors) ──
    #[test]
    fn simd_softmax_bit_identical_to_scalar() {
        // Fixed battery of rows of varying length (incl. non-multiples of 4).
        let rows: Vec<Vec<f64>> = vec![
            vec![1.0, 2.0, 3.0],
            vec![5.0, 1.0, 9.0, 2.0, 0.5],
            vec![-1.0, -2.0, -3.0, -4.0],
            vec![0.0, 1e6, 2e6],               // large-magnitude stability
            vec![2.0],
            vec![3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0],
        ];
        let refs: Vec<&[f64]> = rows.iter().map(|r| r.as_slice()).collect();
        let batched = softmax_batch_lane(&refs);
        assert_eq!(batched.len(), rows.len());
        for (i, r) in rows.iter().enumerate() {
            let expected = softmax_scalar(r);
            assert_eq!(
                batched[i].len(),
                expected.len(),
                "len mismatch row {i}"
            );
            for j in 0..expected.len() {
                assert_eq!(
                    batched[i][j].to_bits(),
                    expected[j].to_bits(),
                    "softmax lane diverged from scalar at row {i} col {j}"
                );
            }
        }
    }

    #[test]
    fn simd_softmax_handles_non_multiple_of_four() {
        // 7 rows → 4 (lane) + 3 (scalar tail) should still match scalar.
        let rows: Vec<Vec<f64>> = (0..7u64)
            .map(|k| vec![k as f64, (k + 1) as f64, (k * 2) as f64 + 0.5, (k + 3) as f64])
            .collect();
        let refs: Vec<&[f64]> = rows.iter().map(|r| r.as_slice()).collect();
        let batched = softmax_batch_lane(&refs);
        for (i, r) in rows.iter().enumerate() {
            let expected = softmax_scalar(r);
            for j in 0..expected.len() {
                assert_eq!(
                    batched[i][j].to_bits(),
                    expected[j].to_bits(),
                    "non-mult-of-4 diverged row {i} col {j}"
                );
            }
        }
    }
}
