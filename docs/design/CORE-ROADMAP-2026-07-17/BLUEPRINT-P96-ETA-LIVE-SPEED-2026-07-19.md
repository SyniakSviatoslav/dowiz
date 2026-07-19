# BLUEPRINT P96 — ETA from live courier speed (wire the estimator the codebase already claims to use) (2026-07-19)

> **Standalone KERNEL blueprint (dowiz `kernel`).** One small, isolated, independently
> buildable unit against the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md` §2.
> Research source: `docs/research/OPUS-HIGHERABSTRACTION-PRODUCT-SCAN-2026-07-19.md` §3 — the
> single real find of a four-surface product-layer scan whose other three verdicts came back
> already-covered / honest-negative. Format precedent:
> `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md` (structure adopted; **ceremony
> deliberately not** — this is a ~20-line pure-kernel change, not a crypto protocol; see VERDICT).
> Grounding tree: `/root/dowiz` at HEAD, every cite re-read live this pass.
>
> **One sentence:** the ETA the customer sees is computed from a **static planned pace**
> (`total_m / baseline_s`, or a hardcoded `5.0 m/s`), while the courier's **live smoothed
> ground speed** `v_mps` is already computed on every GPS ping and then thrown away — route that
> live speed into the ETA speed term, deterministically, with a fallback that guarantees the ETA
> can never become *worse or wilder* than today's baseline.

---

## VERDICT (stated up front, per session discipline)

**GO — small, low-risk, high-value, non-red-line.** This is the opposite of the mesh/crypto
blueprints in this folder: no new dependency, no new primitive, no money/auth/RLS/migration
surface, no cross-node protocol, no adversarial-review gate required. It is a pure-`kernel`
change of ~4 named constants + 1 pure function (~15 lines) in `geo.rs`, plus threading one
`Option<(f64, u32)>` through three existing `ports/customer.rs` constructors, plus a small EMA
step in the ping-folding caller. It composes only functions that **already exist and are already
tested** (`geo::ema_next`, `geo::eta_seconds`).

Two things make it safe enough to ship on ordinary RED→GREEN discipline rather than the heavy
review the P92 fast-path needs:

1. **It respects the standing TimesFM rejection — it does not reopen it.** The rejection
   (`GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-2026-07-16.md:41-42`, `:401-403`) rests on the
   premise that *"`geo::eta_seconds` + `kalman.rs` are already the optimal linear estimator"* for
   per-order ETA. That premise is **currently false about the wiring** (§1): `eta_seconds` uses a
   static baseline and the adaptive estimator is not connected to it. P96 makes the premise **true**
   — it is the deterministic, no-ML, zero-weight-file completion of exactly the estimator the
   rejection assumed was already in place. It is the *anti-TimesFM*.

2. **It degrades closed to the current behaviour, provably.** The new path is used **only** when
   the live signal is warm (≥ `ETA_MIN_PINGS`) and inside a sane speed band. In every other case —
   new courier/order, sparse pings, courier stopped at a light, GPS glitch — it delegates
   byte-for-byte to the existing `eta_seconds`. The bounded-degradation invariant (§6, D2) is a
   machine-checkable property, not a prose promise: **the adaptive ETA never returns `∞` and never
   diverges further from truth than the static baseline in any edge case.**

The one honestly-stated limit: this improves ETA *accuracy* only when the courier's real pace
differs from the route plan's planned pace. On a route whose plan was already accurate it is a
no-op (correctly — it falls back). That is the intended scope, not a shortfall.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> Every claim below was read from source **this pass** (`/root/dowiz`, HEAD), not inherited from
> the research sketch. The research doc's §3 citations were checked against the files and hold.

### 0.1 The ETA path today — speed is a static planned pace

`geo::eta_seconds(remaining_m, total_m, baseline_s)` (`kernel/src/geo.rs:153-166`):

```rust
pub fn eta_seconds(remaining_m: f64, total_m: f64, baseline_s: f64) -> f64 {
    if remaining_m <= 0.0 { return 0.0; }
    let speed = if baseline_s > 0.0 && total_m > 0.0 {
        total_m / baseline_s          // ← STATIC planned pace (route length / planned duration)
    } else {
        5.0                           // ← hardcoded urban fallback (m/s)   (geo.rs:160)
    };
    if speed <= 0.0 { return f64::INFINITY; }   // (geo.rs:162-164) — guard, unreachable today
    (remaining_m / speed).max(0.0)
}
```

- **The speed term is never the courier's real speed.** It is `total_m / baseline_s`, a pace fixed
  at plan time, or the constant `5.0 m/s` when no baseline is supplied (`geo.rs:157-161`).
- `baseline_s` is **passed in from outside** the kernel — it is a parameter of every ETA caller
  (`ports/customer.rs:198` `from_positions`, `:224` `from_kalman`, `:316` `tracking_view`), not
  derived from courier motion.
- The consumer: `TrackingView::from_positions` (`ports/customer.rs:193-215`) calls
  `geo::eta_seconds(prog.remaining_m, total_m, eta_baseline_s)` at `:203` and stores it in
  `TrackingView.eta_seconds` (`:180`), the field the customer/courier app renders.

**How stale a static pace can be (the motivating numbers, from the real code):**

| Situation | Static-baseline speed | Real average speed | ETA error on 1 km remaining |
|---|---|---|---|
| Plan assumes free-flow, courier in traffic | `5.0 m/s` (fallback) | `2.0 m/s` | says **200 s**, truth **500 s** — 5 min under-promise |
| Plan padded, courier on a clear arterial | `5.0 m/s` | `9.0 m/s` | says **200 s**, truth **111 s** — 1.5 min over-promise |
| No `baseline_s` at all (fallback path) | `5.0 m/s` fixed regardless of city/road/vehicle | anything | error grows with the plan/reality gap, unbounded by design |

These are not worst-case fabrications — `5.0 m/s` (18 km/h) is a single hardcoded constant applied
to every order with no baseline, and even a supplied `baseline_s` is a *plan-time* pace that a
live courier deviates from continuously (lights, traffic, wrong turns, vehicle type). The live
signal that would correct all of this already exists on the wire (§0.2).

### 0.2 The live signal exists and is discarded for ETA

| Element | Cite | State |
|---|---|---|
| courier ground speed per ping | `apps/courier/src/types.rs:79` — `pub v_mps: f32,  // ground speed m/s` | **computed every ping**, carried in `TrackFrame` |
| `TrackFrame` note | `apps/courier/src/types.rs:71-73` | "P71 renders it; **it computes no ETA/routing**" — `v_mps` is displayed, never fed to ETA |
| scalar EMA smoother | `kernel/src/geo.rs:39` — `ema_next(prev, sample, alpha) = prev + alpha*(sample-prev)` | **exists, tested** (`geo.rs:355-361`, `518-560` parity fixtures); documented 1-D steady-state Kalman |
| full n-D Kalman filter | `kernel/src/kalman.rs:149` `KalmanFilter`; velocity readable from `x` (2-D constant-velocity test `kalman.rs:438-466`, `kf.x[1] ≈ velocity`) | exists, tested; but the instance stepped on courier pings today is a **trust filter** (`domain.rs:296-323`, research §3), **not** a kinematic velocity filter |
| what the ETA path surfaces from the Kalman | `ports/customer.rs:186` field `kalman_surprise`; `:191-192` doc; `:213`, `:233` pull **only** `kalman.last_surprise()` | **only the dimensionless surprise scalar** reaches the view — never a speed or velocity |

**Consequence:** on the ETA path, `geo.rs`, `ema_next`, and the whole Kalman apparatus are
present, but the one quantity ETA actually needs — an adaptive speed — is computed
(`v_mps`) and dropped. The prior doc's *"Kalman/EMA are already the optimal estimator"* is true
about the **primitives** and false about the **wiring**. P96 closes exactly that wiring gap.

### 0.3 The standing TimesFM rejection this respects (not reopens)

`GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-2026-07-16.md`:

- `:41-42` — *"**TimesFM — NO for per-order ETA** (geo::eta_seconds + kalman.rs are already the
  optimal linear estimator for that problem)"*.
- `:401-403` — *"**TimesFM for per-order ETA — rejected.** Kalman/EMA are already the optimal
  estimator for the actual problem; a 200M-parameter transformer is worse on every axis that
  matters on the hot path."*

P96 is **not** a forecasting model, carries **no** weights, adds **no** dependency, is
**bit-deterministic**, and runs in nanoseconds on the hot path. It is the deterministic linear
estimator the rejection names — finally connected. **This blueprint does not disturb that ruling;
it fulfils its premise.**

---

## 1. The finding, precisely

The ETA speed term is a **static** `total_m / baseline_s` (`geo.rs:158`) or a hardcoded `5.0 m/s`
(`geo.rs:160`). The courier's **live smoothed ground speed** — derivable from `TrackFrame.v_mps`
(`courier/types.rs:79`) via the already-tested `geo::ema_next` (`geo.rs:39`) — is never routed
into it. The fix is to feed the live smoothed speed into the speed term **when it is trustworthy**,
and fall back to the existing static path otherwise. Pure kernel, RED→GREEN, zero new deps.

---

## 2. Scope — what P96 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P96 OWNS

1. A new pure kernel function `geo::eta_seconds_adaptive(remaining_m, total_m, baseline_s,
   observed)` that uses a live smoothed speed when warm-and-in-band, else delegates to the existing
   `eta_seconds` unchanged.
2. Four named constants in `geo.rs` (`ETA_MIN_SPEED_MPS`, `ETA_MAX_SPEED_MPS`, `ETA_MIN_PINGS`,
   `ETA_SPEED_ALPHA`) — no magic numbers.
3. Threading an `observed_speed_mps: Option<(f64, u32)>` (smoothed speed, accepted-ping count)
   through the three `ports/customer.rs` constructors that today take `eta_baseline_s`.
4. The tiny EMA step (`v̂ = ema_next(v̂, v_mps, ETA_SPEED_ALPHA)`; `pings += 1`) in the caller that
   folds the accepted (in-order) ping stream, and passes `Some((v̂, pings))` alongside the baseline.
5. The bounded-degradation invariant as executable tests (§6).

### 2.2 P96 does NOT own (anti-scope)

- **No ML, no forecasting model, no training, no weight file, no new dependency** — the standing
  TimesFM rejection is honoured, not reopened (§0.3).
- **No change to `eta_seconds` itself.** The existing static function stays byte-for-byte as the
  fallback; adaptive is a *new* function that delegates to it. Its existing test `eta_basic`
  (`geo.rs:366-380`) must stay green untouched (D-NOREG).
- **No new stateful estimator in the pure math layer.** `geo.rs` stays a library of pure functions;
  the EMA *state* (`v̂`, `pings`) lives in the caller that already owns the ping stream, exactly as
  `eta_baseline_s` and `kalman_surprise` are caller-owned today (`ports/customer.rs`).
- **No graduation to the full CV Kalman in this unit.** Reading a true velocity component from a
  constant-velocity `KalmanFilter` (`kalman.rs`, `x[vel]`) is a *real* optional later step (§7), but
  it requires instantiating a kinematic filter per order and feeding it lat/lng — larger, deferred.
  P96 ships the minimal EMA-on-`v_mps` path first (ponytail: the best code is the code not written).
- **No routing / VRP / zone / courier-scoring surface** — untouched; those are separate (and, for
  courier scoring, a red-line — research §2).
- **No UI/render change.** `TrackingView.eta_seconds` keeps its type and meaning; only its *value*
  gets more accurate. The courier app's `"ETA {}s"` render (`apps/courier/src/render.rs`) is unchanged.

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs (in tree, already tested):** `geo::eta_seconds` (`geo.rs:153`), `geo::ema_next`
(`geo.rs:39`), `TrackFrame.v_mps` (`courier/types.rs:79`), the three `TrackingView` /
`TrackingAuthority` constructors (`ports/customer.rs:193`, `:219`, `:308`).
**Consumers:** `TrackingView.eta_seconds` (`ports/customer.rs:180`) and whatever renders it. Same
type, same semantics, better value. **No new deps, no new crates, no wire/schema change.**

### 2.4 Honest reconciliation with the static baseline (standard §2 item 6)

The static baseline is **not** wrong — it is the correct behaviour when no live signal exists (a
just-placed order, a courier who hasn't moved). P96 keeps it as the load-bearing fallback and only
*overrides* it when a trustworthy live speed is available. The default is the baseline; adaptive is
an opt-in, auto-fallback overlay. Anything failing the trust gate falls through to the exact
current code.

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

All additions live in the existing `kernel/src/geo.rs` (extend, don't create a module — item 19).
Constants are named engineering decisions, never magic (§5 justifies the values).

```rust
// kernel/src/geo.rs  (ADD near eta_seconds)

/// Minimum trustworthy *average* speed (m/s). Below this, a smoothed observation is treated as
/// "courier stopped / GPS noise" and the ETA falls back to the planned baseline rather than
/// exploding toward `f64::INFINITY`. 0.5 m/s ≈ 1.8 km/h — slower than a walk ⇒ not "in transit".
pub const ETA_MIN_SPEED_MPS: f64 = 0.5;

/// Maximum plausible courier *average* speed (m/s) ≈ 108 km/h. Above this the observation is a GPS
/// glitch, not a real pace, and the ETA falls back to the baseline.
pub const ETA_MAX_SPEED_MPS: f64 = 30.0;

/// Accepted in-order pings required before the smoothed speed is trusted over the baseline.
/// Cold-start guard: a new courier/order rides the planned pace until the EMA has warmed.
pub const ETA_MIN_PINGS: u32 = 3;

/// EMA smoothing factor for the observed ground-speed stream (same shape as `ema_next`'s alpha).
/// 0.3 ⇒ ~10-ping memory: rejects single-ping GPS spikes, still tracks a real traffic change.
pub const ETA_SPEED_ALPHA: f64 = 0.3;
```

The caller-side ping state is just two scalars (no new type needed), maintained where the ping
stream is already folded:

```rust
// caller (ping-folding surface) — NOT in geo.rs:
//   v_hat: f64   // smoothed speed, seeded 0.0
//   pings: u32   // accepted in-order pings, seeded 0
// per accepted (in-order) ping with sample `v_mps`:
//   v_hat = geo::ema_next(v_hat, v_mps as f64, geo::ETA_SPEED_ALPHA);
//   pings = pings.saturating_add(1);
// then pass Some((v_hat, pings)) as `observed` into the ETA constructor.
```

---

## 4. Build items — spec → RED test → code (standard §2 items 2, 3, 5)

### 4.1 M1 — `eta_seconds_adaptive`: trust-gated live speed, byte-identical fallback

- **Spec:** a new pure function that uses the live smoothed speed **iff** warm and in-band, else
  delegates to `eta_seconds` unchanged.

```rust
/// Adaptive ETA. Uses the courier's smoothed observed ground speed when it is trustworthy;
/// otherwise falls back to the EXACT existing static-baseline `eta_seconds` behaviour.
/// `observed = Some((smoothed_v_mps, accepted_ping_count))`; `None` = no live signal yet.
/// INVARIANT: when the live signal is absent/cold/out-of-band this returns *exactly*
/// `eta_seconds(remaining_m, total_m, baseline_s)` — never worse, never `∞`.
pub fn eta_seconds_adaptive(
    remaining_m: f64,
    total_m: f64,
    baseline_s: f64,
    observed: Option<(f64, u32)>,
) -> f64 {
    if remaining_m <= 0.0 {
        return 0.0;
    }
    if let Some((v_hat, pings)) = observed {
        if pings >= ETA_MIN_PINGS
            && v_hat.is_finite()
            && v_hat >= ETA_MIN_SPEED_MPS
            && v_hat <= ETA_MAX_SPEED_MPS
        {
            return (remaining_m / v_hat).max(0.0); // v_hat bounded ⇒ ETA bounded, never ∞
        }
    }
    eta_seconds(remaining_m, total_m, baseline_s) // fallback: byte-for-byte current behaviour
}
```

- **RED `eta_adaptive_falls_back_when_cold`:** `observed = None` and `observed = Some((8.0, 2))`
  (pings < `ETA_MIN_PINGS`) each return **exactly** `eta_seconds(remaining, total, baseline)`. RED
  against a naive direct-swap implementation (which would use the speed regardless), GREEN after.
- **RED `eta_adaptive_uses_live_speed_when_warm`:** `observed = Some((10.0, 5))`, remaining `1000` ⇒
  `100.0 s` (uses live speed, not the baseline pace). Proves the wiring actually fires.
- **RED `eta_adaptive_stopped_courier_never_infinite`:** `observed = Some((0.0, 9))` (courier
  parked at a light, smoothed speed below the floor) ⇒ falls back to baseline, **not** `f64::INFINITY`.
  This is the critical "never wilder than baseline" guard — RED against the direct-swap
  (`remaining/0.0 = ∞`), GREEN after.
- **RED `eta_adaptive_gps_glitch_clamped_out`:** `observed = Some((500.0, 9))` (impossible pace) ⇒
  falls back to baseline, not an absurd `2 s` ETA.

### 4.2 M2 — thread `observed_speed_mps` through the customer port

- **Spec:** add `observed_speed_mps: Option<(f64, u32)>` to `TrackingView::from_positions`
  (`ports/customer.rs:193`), `from_kalman` (`:219`), and `TrackingAuthority::tracking_view`
  (`:308`), placed next to the existing `eta_baseline_s` param. `from_positions` calls
  `geo::eta_seconds_adaptive(prog.remaining_m, total_m, eta_baseline_s, observed_speed_mps)` at
  `:203` instead of `eta_seconds`. `None` reproduces today's output exactly.
- **RED `tracking_view_none_matches_legacy`:** `from_positions(..., None)` yields the same
  `eta_seconds` value as before P96 for a fixture route (regression pin). GREEN by construction of
  the fallback.
- **RED `tracking_view_warm_speed_beats_baseline_gap`:** with a fixture where planned pace ≠ live
  pace and `observed = Some((v̂, ≥3))`, the returned `eta_seconds` reflects the live pace.
- **NOTE (no math in the caller):** the EMA step lives in the ping-folding surface, not in
  `customer.rs`; `customer.rs` only forwards the already-smoothed `Option`. Keeps the port a pure
  view-builder (matches how `kalman_surprise` is forwarded, `:213`).

### 4.3 M3 — the EMA step at the ping-folding site (caller)

- **Spec:** where accepted in-order pings are folded (the tracking surface that already rejects
  out-of-order pings via `geo::is_out_of_order`, `geo.rs:171`), maintain `(v_hat, pings)`: on each
  accepted ping, `v_hat = geo::ema_next(v_hat, v_mps as f64, geo::ETA_SPEED_ALPHA); pings += 1;`
  Pass `Some((v_hat, pings))` into the ETA constructor. Reset `(0.0, 0)` on a new order / route
  version change (`TrackFrame.route_version`, `courier/types.rs:82`) so a re-route cold-starts
  cleanly.
- **RED `speed_ema_rejects_single_spike`:** feed `[8, 8, 8, 40, 8]` m/s through `ema_next` at
  `alpha=0.3`; assert the post-spike `v_hat` stays within a tight band of 8 (the spike is absorbed,
  not tracked). Proves GPS-noise robustness comes from the smoother, before the band-gate even fires.
- **RED `speed_ema_resets_on_route_version_change`:** a `route_version` bump zeroes `(v_hat, pings)`
  so the next ETA cold-starts on the baseline (no stale speed leaking across a re-route).

---

## 5. Why the values (engineering decisions the blueprint sets — standard §2 item 10)

- `ETA_MIN_SPEED_MPS = 0.5` — below a walking pace ⇒ the courier is stopped, not "making progress";
  using such a speed would inflate ETA toward `∞`. The floor converts "stopped" into "fall back to
  the plan", which is the honest estimate while parked.
- `ETA_MAX_SPEED_MPS = 30.0` (~108 km/h) — a courier *average* over a city leg above this is a GPS
  artefact; clamp-to-fallback rather than promise an impossibly early arrival.
- `ETA_MIN_PINGS = 3` — one or two pings is not a pace; three accepted in-order pings through an
  `alpha=0.3` EMA is the minimum before the estimate is more signal than transient. Cold-start uses
  the plan, exactly as today.
- `ETA_SPEED_ALPHA = 0.3` — ~10-sample memory: long enough to reject a single spurious `v_mps`
  spike, short enough to follow a genuine regime change (open road → gridlock) within a few pings.
  Reuses `ema_next`'s existing, tested semantics; no new smoother.

All four are single-line constants with a one-place change surface. None is safety-critical: every
value only decides *when* to prefer the live signal over the plan; the fallback is always the
current, already-shipped behaviour.

---

## 6. Invariants & adversarial self-check (standard §2 items 3, 6)

The unsafe states are made **unrepresentable or provably-bounded**, not merely avoided:

- **I1 — never worse than baseline (bounded degradation).** For *any* inputs, if the observation is
  `None`, cold (`pings < ETA_MIN_PINGS`), non-finite, or out of `[ETA_MIN_SPEED_MPS,
  ETA_MAX_SPEED_MPS]`, `eta_seconds_adaptive == eta_seconds`. Adaptive strictly *adds* a
  better estimate in the confident case and is *identical* otherwise. **Property test**
  `prop_adaptive_never_worse` (§6.1).
- **I2 — never `∞`, never negative.** When the live path fires, `v_hat ∈ [0.5, 30]` ⇒ `ETA ∈
  [remaining/30, remaining/0.5]`, finite and positive; `.max(0.0)` and the `remaining_m <= 0.0`
  early-return cover the boundaries. The `speed <= 0.0 → f64::INFINITY` arm of `eta_seconds`
  (`geo.rs:162`) is unreachable on the adaptive path by construction (the floor gate precedes it).
- **I3 — fully deterministic / bit-reproducible.** Only `ema_next` (a documented deterministic
  filter) and integer/`f64` comparisons; `no_std`-friendly; no clock, no RNG, no float
  nondeterminism beyond IEEE-754 already covered by `geo.rs`'s parity fixtures. Consistent with the
  TimesFM rejection's determinism requirement (§0.3).
- **I4 — no stale speed across orders/routes.** `(v_hat, pings)` resets on new order / `route_version`
  bump (M3), so one order's pace can never bleed into another's ETA.
- **Adversarial — GPS spike:** absorbed twice — first by the EMA (`speed_ema_rejects_single_spike`),
  then by the band-gate (`eta_adaptive_gps_glitch_clamped_out`). Defense in depth.
- **Adversarial — courier parked:** `v_hat → 0` ⇒ floor gate ⇒ fallback, not `∞`
  (`eta_adaptive_stopped_courier_never_infinite`). This is the single most important guard and has
  a dedicated RED test.

### 6.1 The one property test that matters

```
prop_adaptive_never_worse:
  ∀ remaining_m ≥ 0, total_m ≥ 0, baseline_s ≥ 0, observed:
    let a = eta_seconds_adaptive(remaining_m, total_m, baseline_s, observed);
    a.is_finite() || eta_seconds(remaining_m, total_m, baseline_s).is_finite() == a.is_finite();
    // and: when `observed` is None/cold/out-of-band ⇒ a == eta_seconds(remaining_m,total_m,baseline_s)
    // and: a >= 0.0 always
```

---

## 7. Optional later graduation (NOT this unit)

The research doc (§3) notes a fuller version: instantiate a **constant-velocity `KalmanFilter`**
(`kalman.rs`, `F=[[1,dt],[0,1]]`, `H=[[1,0]]`) per order, feed it position, and read the *velocity*
component `x[vel]` — the estimator the codebase's own comment credits (`kalman.rs:3-6`, "generalises
`ema_next`"). That is a real, principled upgrade (it fuses position observations, not just raw
`v_mps`), but it is **larger** — a new per-order filter, tuning `Q`/`R`, resets, and its own tests —
and is **deferred**. P96 ships the minimal EMA-on-`v_mps` path first; the CV-Kalman graduation can
replace the smoother behind the same `observed_speed_mps` seam later with zero API change. Named
here so it is not lost, explicitly out of scope now (item 19: upgrade only if the minimal version
proves insufficient on real tracks).

---

## 8. DoD — falsifiable, RED→GREEN, machine-checkable (standard §2 item 2)

| # | Done when… | Falsifier (RED test / check) |
|---|---|---|
| D1 | `eta_seconds_adaptive` uses live speed when warm-and-in-band, else exact baseline | `eta_adaptive_uses_live_speed_when_warm`, `eta_adaptive_falls_back_when_cold` (M1) |
| D2 | the adaptive ETA is **never `∞`** and **never worse than baseline** in any edge case | `eta_adaptive_stopped_courier_never_infinite`, `eta_adaptive_gps_glitch_clamped_out`, `prop_adaptive_never_worse` (M1/§6.1) |
| D3 | the customer port threads `observed_speed_mps`; `None` reproduces legacy output exactly | `tracking_view_none_matches_legacy`, `tracking_view_warm_speed_beats_baseline_gap` (M2) |
| D4 | the EMA step rejects single-ping spikes and resets on re-route | `speed_ema_rejects_single_spike`, `speed_ema_resets_on_route_version_change` (M3) |
| D5 | *(accuracy, if data exists)* adaptive ETA MAE < static-baseline MAE on a replay where real pace ≠ planned pace | `eta_adaptive_beats_baseline_on_replay` — see §9 |
| D-NOREG | existing `eta_basic` (`geo.rs:366`) + all `ports/customer.rs` tests stay green; no dep added | `cargo test -p <kernel>` |
| D-BUILD | kernel builds & full `cargo test --lib` green incl. new REDs now GREEN, no new dependency | `cargo test`; `git diff Cargo.toml` empty |

---

## 9. Verification plan (standard §2 items 5, 10)

1. **Bounded-degradation floor (always required — D2, §6.1).** The property test proves P96 can
   never ship a *worse* ETA than today, independent of whether any historical ground truth exists.
   This is the safety net that lets the change ship on ordinary discipline.
2. **Accuracy oracle (D5 — preferred).** On a replayed ping track where the courier's real speed
   differs from the plan's `baseline_s`, the adaptive ETA's mean-absolute-error against the actual
   arrival time must **beat** the static-baseline ETA. *If a real replay fixture exists* (check
   `apps/courier` / kernel test fixtures for recorded `TrackFrame`/ping tracks), use it. *If not*,
   construct a synthetic-but-honest track: a constant real speed `v_real` held for N pings with
   `baseline_s` chosen so the planned pace ≠ `v_real`; assert adaptive MAE < baseline MAE, and — the
   falsifiable half — **if adaptive does not beat baseline, keep the baseline** (the research doc's
   own acceptance rule: it must earn the swap or it doesn't ship the swap; the fallback still ships).
3. **No regression.** `eta_basic` and the customer-port suite must stay green untouched (D-NOREG) —
   proof that the fallback path is byte-identical to today.

---

## 10. Rollout notes — plainly, this is small and low-risk (standard §2 items 11, 13)

- **Not a red-line change.** No money, auth, RLS, migration, bulk-data, courier-scoring, or
  cross-node surface is touched. It does **not** need the Triadic Council, the independent
  adversarial-forgery gate, or the measure-first NO-GO gate that P92 (crypto) and the mesh
  blueprints require. Ordinary RED→GREEN + the §6.1 property test is sufficient.
- **Bulkhead / fail-safe by construction.** The failure mode is *fall back to the static baseline*,
  which always exists and is the current shipped behaviour. A bug in the smoother or the gate can
  only cost accuracy in the confident case; it can never produce `∞`, a negative ETA, or a
  cross-order leak (I1–I4). There is nothing to roll back to that isn't already the default path.
- **Feature-flag optional, not required.** If a cautious rollout is wanted, the caller can pass
  `None` for `observed_speed_mps` to disable the adaptive path globally with zero code change —
  the flag *is* the parameter. No config plumbing needed.
- **Isolated diff.** One file gains ~4 constants + 1 function (`geo.rs`); one file gains one param
  on three constructors and one call-site swap (`ports/customer.rs`); one caller gains a 2-line EMA
  step. No wire/schema/UI-type change. Reviewable in a single sitting.

---

## 11. Links to docs & memory + worker instructions (standard §2 items 7, 18)

**Depends on / cites:**
- `docs/research/OPUS-HIGHERABSTRACTION-PRODUCT-SCAN-2026-07-19.md` §3 (the finding), §3 falsifiable
  acceptance (replay MAE test).
- Standing decision honoured: `docs/design/GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-2026-07-16.md`
  §1/§3/§5 (`:41-42`, `:293-303`, `:401-403`) — TimesFM-for-ETA rejected; P96 is the deterministic,
  no-ML completion its premise assumed.
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract; this small unit satisfies the
  load-bearing subset — items 1,2,3,4,5,6,7,10,11,13,18,19 — and honestly marks the crypto/mesh
  items N/A).
- Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md` (structure, not scale).

**Existing code this blueprint edits/extends (exact targets — dowiz `kernel`):**
- **EDIT** `kernel/src/geo.rs` — add `ETA_MIN_SPEED_MPS`/`ETA_MAX_SPEED_MPS`/`ETA_MIN_PINGS`/
  `ETA_SPEED_ALPHA` + `eta_seconds_adaptive`; **do NOT modify** `eta_seconds` or `ema_next`
  (they are the tested fallback + smoother).
- **EDIT** `kernel/src/ports/customer.rs` — add `observed_speed_mps: Option<(f64, u32)>` to
  `TrackingView::from_positions` (`:193`), `from_kalman` (`:219`), `TrackingAuthority::tracking_view`
  (`:308`); swap the `eta_seconds` call at `:203` for `eta_seconds_adaptive`. `None` = legacy.
- **EDIT** the ping-folding caller (the surface that already folds `TrackFrame`/rejects out-of-order
  pings via `geo::is_out_of_order`) — maintain `(v_hat, pings)`, step with `ema_next`, reset on
  `route_version` change, pass `Some((v_hat, pings))` into the ETA constructor.
- **DO NOT TOUCH** routing (`router.rs`), zones (`geo::point_in_polygon`), the trust Kalman
  (`domain.rs:296-323`), or any render/UI type — out of scope.

**For the worker with zero session context — exact acceptance path:**
1. Write the four constants + `eta_seconds_adaptive` in `geo.rs`; write its M1 RED tests first
   (types → tests → code — item 3); they fail before the function exists, pass after.
2. Add `prop_adaptive_never_worse` (§6.1) — the load-bearing safety property.
3. Thread `observed_speed_mps` through the three `customer.rs` constructors; `tracking_view_none_
   matches_legacy` proves the fallback is byte-identical.
4. Add the EMA step + `route_version` reset in the ping-folding caller; M3 REDs green.
5. `cargo test --lib` fully green including `eta_basic` and the customer-port suite (D-NOREG);
   `git diff` on any `Cargo.toml` must be empty (no new dependency).
6. Attempt the D5 replay-accuracy oracle; if adaptive does not beat baseline on a real/synthetic
   track, ship the fallback-only wiring and record that adaptive-swap did not clear the bar — the
   safety net still holds.
7. This is not a red-line change: no council, no forgery gate, no operator gate required.
