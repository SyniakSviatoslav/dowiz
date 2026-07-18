# BLUEPRINT P-C — Safety / Self-Healing / Self-Terminating (2026-07-17)

> **Phase:** P-C of `CORE-ROADMAP-STANDARD-2026-07-17.md` §3 ("Circuit breakers, invariants, the
> watchdog/authority boundary"). **Absorbs:** Batch 3 (`bebop2-mesh-tensor-hermetic-2026-07-17/
> 12-BATCH3-safety-selfhealing-findings.md`), doc 19 Part 2 (finite-anchored-authority toy proof),
> synthesis V2 §D + T-6/W3-L4. **New this pass (exactly two builds):** (1) the `integrity_check`
> hysteresis band, (2) restart-intensity as a launch-path predicate. Written against the §2
> 20-point contract; the compliance map is §12.
>
> Status: PROPOSED. Zero operator gates required (no money/auth/RLS/migration surface touched).
> Executable by an agent with zero session context via §10.
>
> **Correction (2026-07-18, session verification pass):** Build 1 (the hysteresis band, §3) has
> **LANDED on `main`** — commit `a50d44ab0`, `HysteresisBand` now live at `hydra.rs:85`, including
> the `!rho.is_finite()` guard on the hydra path (ground truth per
> `ROADMAP-UPDATE-SESSION-SYNTHESIS-2026-07-18.md` §0, re-verified there via `git show main`).
> The status line above is retained for provenance; treat §3 as a record of the landed design, and
> §1's pre-fix line numbers (`hydra.rs:180-195` etc.) as the *pre-landing* state they were verified
> against. Build 2 (§4 restart predicate) was **not re-verified** in that pass — do not assume
> either way; check `bounded_drainer.rs` on live `main` before executing §10 steps 7–9. Full
> session fold-in: §13.

---

## 0. Why this layer exists (context for a reader with zero session history)

Layer C is the kernel's answer to a question every long-running autonomous system faces: **what
keeps the organism safe when something inside it goes wrong, given that this codebase has banned
the standard answer?** The standard answer is a watchdog — a standing supervisor process that
samples health and intervenes. This repo rejected that shape on proven grounds (doc 19's compiled
toy, §2.2/§4.2 below): a watchdog can be silently absent (deleting it leaves no compile error, no
signal — the bad state just persists unobserved), it samples *after the fact* (the toy observed
the bad state three times even with the watchdog running), and it regresses infinitely ("who
watches the watcher"). Layer C's replacement doctrine is that safety must live **in the causal
path of the guarded action itself**: a check whose deletion is a compile error, a bound that is
arithmetic over a monotone fact, an invariant that makes the unsafe state unrepresentable rather
than detected-later.

Concretely, the kernel already had most of this (budget debit, drift gate, fuel trap — §2.1), but
two real gaps remained, and they are this blueprint's two builds:

1. **The organism's own integrity flip could be made to flap.** `integrity_check` compares the
   baseline's spectral radius ρ against 1.0 with no memory: one healthy sample re-opens commits,
   one bad sample locks them. An adversary (or a marginal re-seed) that dithers the baseline
   across the ρ=1 line gets a commit accepted *every second check* — fail-closed in name,
   oscillating in practice. The fix (§3) is a classical two-threshold hysteresis band plus an
   N-consecutive-healthy dwell: tripping stays instantaneous (fail-closed latency unchanged), but
   release now demands sustained, provably-Damped evidence. This is a debounce filter stated as a
   deterministic finite automaton, not a supervisor.
2. **A crash-looping process could relaunch forever.** A dead process cannot check itself, and the
   ban on watchdogs does not repeal that fact — so the restart bound (§4) is placed at the one
   moment something *outside* the dead process necessarily acts: the launch. `launch_permitted` is
   a pure function over the append-only launch ledger (OTP's MaxR/MaxT restart-intensity, made a
   launch-path predicate), and the `LaunchToken` proof-of-admission type makes bypassing it a
   compile error. The residual authority — custody of the ledger file that survives the crash — is
   named honestly in §2.2 as the *finite anchor*, not defined away.

Everything else in the Layer-C cluster (breaker module F1a, M7 heal, wasm clamps, P06's
independent-verification leg) is deliberately out of scope here and owned elsewhere (§5, §11).
The layer's one-sentence law, applied throughout: **safety is physics (types + arithmetic on the
causal path), never bureaucracy (a remembered ritual or a standing monitor).**

---

## 1. Ground truth (verified THIS pass, live reads 2026-07-17 — not inherited)

Every claim below was re-read from the working tree this session, per contract item 1.

- **`kernel/src/hydra.rs:180-195` — `integrity_check` has NO hysteresis.** The flip is the
  instantaneous single-threshold predicate `rho < 1.0 && rho.is_finite()` (`hydra.rs:186`):
  auto-restore `Locked→Live` at `:188-190` (one healthy sample suffices), lock at `:191-193`
  (one bad sample suffices). Confirms Batch 3 §5's gap verbatim; nothing has changed since doc 19.
- **`hydra.rs:227`** — `commit` calls `integrity_check()` on the critical path, so a flapping
  state directly alternates commit-refuse/commit-accept.
- **`hydra.rs:253-265` — `boot_verify`** re-derives baseline ρ and hard-`assert!`s
  `rho < 1.0 && rho.is_finite()` (`:258-263`) with the operator-directive message ("re-seed from
  golden, not endure... kill-switch is the only safe stop"). Single boot-time sample — no
  oscillation is possible there; left unchanged by this blueprint.
- **`hydra.rs:183-185`** — the baseline spec is explicit: "Baseline must remain a
  contracting/Damped organism (ρ<1)". This pins the trigger threshold decision in §3.2.
- **`kernel/src/spectral.rs:342-351` — `classify_drift`** uses a function-local
  `const BAND: f64 = 1e-6` (`:344`): Damped iff ρ < 1−1e-6, Unstable iff ρ > 1+1e-6, Resonant
  in the closed band between. `spectral_radius` at `spectral.rs:217`.
- **`grep -rn hysteresis kernel/src/` → zero hits** (this pass). The token exists nowhere in the
  kernel; Batch 3 §5's finding stands.
- **`kernel/src/bounded_drainer.rs:27-82`** — `BoundedDrainer` (`new`/`remaining`/`total_run`/
  `is_done`/`tick`), std-only, degrade-closed, debits `TokenBucket` per unit. This is the
  crash-loop *subject* (Phase 27 §2.1: "a crash-looping drainer relaunches forever") and the
  natural home for the launch gate (§4, reuse argument §11).
- **`kernel/src/budget.rs:113` `ComputeBudget::debit`**, **`:158` `Err(JobError::BudgetExceeded)`**
  — the degrade-closed arithmetic idiom the launch predicate mirrors ("`debit` IS the gate").
- **`kernel/src/event_log.rs:389` `commit_after_decide_drift_gate`** — inline
  verify-before-persist; the Pattern-B shape proven in the doc 19 toy.
- **P06 `key_V` remains a stub:** `DECART-P06-bebop2-crypto-dep.md:10` — "P06's V1 merge-gate
  currently runs **unsigned** (`signed:false` at `tools/ci-truth/src/main.rs:423`)". Load-bearing
  for §2's honesty: nothing in this blueprint closes RC-2.
- **Existing hydra tests that must stay green untouched:** `hydra_integrity_live_vs_locked`
  (`hydra.rs:502-513`), `hydra_commit_refused_while_locked` (`:518-531`),
  `breach_alert_bytes_roundtrip_and_reject_bad_len` (`:686-704`), `hydra_boot_verify_clean_baseline`
  (`:434-437`). None exercises `Locked→Live` auto-restore, so the release-side change breaks no
  existing assertion.

---

## 2. Authority classification — the two categories, stated precisely (contract items 6, 13)

Per synthesis V2 §D (tested against the compiled toy, doc 19 §2.2-2.4), the two categories below
are DISTINCT and must never be conflated. This blueprint touches both and moves neither.

### 2.1 Genuine zero-authority (authority dissolves into arithmetic/types)

The check IS the computation; it bottoms out at the compiler / IEEE-754 / a runtime trap. No
identity is trusted; deleting the check is a compile-time hole (absence-is-visible):

| Invariant | Mechanism | Cite (verified §1) |
|---|---|---|
| budget | `ComputeBudget::debit` refuses past ceiling, no spend recorded | `budget.rs:113`, `:158` |
| money | integer money legs in kernel decide arithmetic (dual-authority flip is P-G-gated, untouched here) | V2 §D classification |
| drift ρ | `commit_after_decide_drift_gate` rejects Unstable pre-persist; bad state never produced | `event_log.rs:389` |
| fuel | wasmtime `OutOfFuel` trap terminates the guest — substrate physics | Batch 3 §2 (`fuel.rs:91-208`) |

The operator's "remove authority entirely" is **literally achieved** for this class (V2 §D axis 3).

### 2.2 Finite anchored authority (irreducible; minimized, never zero)

You cannot type external tamper or a dead process out of existence (Batch 6 §4.2, doc 19 §2.4).
The honest word is "a finite structural authority" — inline + independent, event-triggered,
terminating at a planted anchor; NOT a watchdog, NOT self-certification:

- **Tamper leg** (`integrity_check`, `boot_verify`, `key_V`, `WorkReceipt`): `integrity_check` is
  inline (no watchdog, no liveness regress) **but self-certified** — a compromised node checking
  itself (Hermetic RC-2). The independent half is **P06 `key_V`**, still `signed:false`
  (`DECART-P06...:10`). Chain bottoms out finitely: `commit → key_V verdict → anchor` (doc 19
  toy regress probe — "No infinite tower").
- **Crash-loop leg** (this blueprint's §4): a crashed/hung process cannot check itself (V2 item
  139, the liveness leg). The anchor is the surviving launcher/substrate (parent process or
  systemd) plus the append-only launch ledger. The predicate reduces this authority to **zero
  discretion** (a pure function computes the verdict) but not **zero anchor** (something outside
  the dead process must hold the monotone fact).

**Explicit non-conflation statements:**
1. Adding hysteresis to `integrity_check` (§3) improves the *dynamics* of the tamper leg's local
   check. It does NOT move `integrity_check` into class 2.1, does NOT close RC-2, and does NOT
   substitute for P06 `key_V`. The leg stays finite-anchored pending P06.
2. The restart predicate (§4) has a class-2.1 *verdict* (pure arithmetic over a slice) resting on
   a class-2.2 *fact custody* (the ledger survives the crash outside the process). Claiming the
   crash-loop leg is "zero-authority" would be the over-claim V2 §D corrects; this blueprint does
   not make it.
3. A hung-but-alive process is out of the launch predicate's scope BY DESIGN — that leg is the
   wasmtime `OutOfFuel` trap (item 139's SPLIT verdict: fuel trap + launch predicate + systemd
   StartLimit as substrate-physics, together, no standing monitor).

---

## 3. Build 1 — the `integrity_check` hysteresis band

### 3.1 The bug, stated as math (prove it first — §7 test T1 is RED against today's code)

Let `ρ_k` be the spectral radius measured at check `k`. Today's state update
(`hydra.rs:186-193`) is memoryless: `state_k = Locked iff ¬(ρ_k < 1 ∧ finite(ρ_k))`. For any
baseline sequence with `ρ_k` alternating across 1.0 (period 2, arbitrarily small amplitude ε>0:
`1−ε, 1+ε, 1−ε, ...`), the state flaps `Live, Locked, Live, ...` every check, and since `commit`
gates on it (`hydra.rs:227`), commits alternate accept/refuse — the oscillation Batch 3 §5 names.
Note `spectral_radius` is a deterministic pure function of the topology, so the dither source is
real repeated mutation of `base_edges` near the boundary (progressive tamper, marginal re-seed),
not float jitter — which is exactly the adversarial case: **a tamper process that dithers the
baseline gets the organism to accept a commit every second check.** That is the hazard.

### 3.2 The fix — asymmetric two-threshold band + N-consecutive-healthy release

**Predefined types and constants (spec precedes tests precedes code — contract items 3, 4):**

```rust
// kernel/src/spectral.rs — promote the local BAND to the single authority
// (P2 Correspondence / P3 Vibration: one concept, one named constant):
/// Tolerance band around ρ=1 for drift classification AND the integrity
/// hysteresis derivation. Was function-local `BAND` in `classify_drift`.
pub const DRIFT_BAND: f64 = 1e-6;
// classify_drift's body switches to DRIFT_BAND; behavior bit-identical.
```

```rust
// kernel/src/hydra.rs
/// Two-threshold hysteresis band for the Live<->Locked flip (Batch 3 §5 fix).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HysteresisBand {
    /// Lock (fail-closed) when ρ >= trigger or ρ non-finite. Trips in ONE check.
    pub trigger: f64,
    /// Eligible to release only when ρ <= release. Strictly < trigger (enforced below).
    pub release: f64,
    /// Consecutive checks with ρ <= release required before Locked->Live.
    pub healthy_checks: u32,
}

pub const INTEGRITY_BAND: HysteresisBand = HysteresisBand {
    trigger: 1.0,                                      // unchanged fail-closed line
    release: 1.0 - 2.0 * crate::spectral::DRIFT_BAND,  // = 0.999998
    healthy_checks: 3,
};

// Compile-time enforcement (contract item 14 — the bug class becomes a build
// failure): a band with trigger == release, an inverted band, or a gap
// narrower than the full Resonant band width cannot compile.
const _: () = assert!(INTEGRITY_BAND.release < INTEGRITY_BAND.trigger);
const _: () = assert!(
    INTEGRITY_BAND.trigger - INTEGRITY_BAND.release >= 2.0 * crate::spectral::DRIFT_BAND
);
const _: () = assert!(INTEGRITY_BAND.healthy_checks >= 2);
```

(Const f64 comparison/arithmetic in `const _: () = assert!(...)` is stable Rust. Fallback if the
toolchain refuses: compare `to_bits()` — valid here because both operands are positive finite
floats near 1.0, where the IEEE-754 bit pattern order equals numeric order. Verify at
implementation; primary form expected to compile.)

**Exact values and the formula that derives them:**

- **`trigger = 1.0`, unchanged.** The baseline spec is ρ<1 (`hydra.rs:183-185`); a Resonant
  *baseline* is not a valid baseline even though a Resonant *mutation* may pass the drift gate
  (different objects: baseline health vs mutation admission — do not "harmonize" them). Hysteresis
  must never delay the fail-closed pole: trip latency stays exactly one check. Every existing
  lock-side test stays green byte-for-byte.
- **`release = trigger − 2·DRIFT_BAND = 0.999998`.** Derivation: the release point must sit
  strictly inside the Damped class (ρ < 1 − DRIFT_BAND) with at least one full DRIFT_BAND of
  margin below the Damped boundary, so classification and release provably agree (any ρ at or
  below release is Damped, never Resonant). Equivalently: **the hysteresis gap must be at least
  the full width of the Resonant tolerance band (2·DRIFT_BAND)**, so no single ρ value can be
  simultaneously "healthy enough to release" and "within tolerance of the lock line." Equality is
  chosen as the minimal sufficient gap — looser only delays legitimate recovery.
  General formula for reuse (breaker, admission): `release = trigger − 2·(classification band)`.
- **`healthy_checks = 3`.** The band alone defeats sub-band dither; it does NOT defeat
  full-amplitude dither (ρ swinging from ≥trigger to ≤release each check). The consecutive-streak
  dwell does: with N required and streak reset on any non-healthy sample, a full-amplitude
  period-2 oscillation never accumulates N≥2 consecutive healthy checks — it locks once and holds.
  N=2 is the minimal sufficient value; **N=3** is chosen to also defeat any period-≤3 pattern and
  to be one idiom with the Phase 27 breaker's `probe_successes` close-hysteresis
  (`BLUEPRINT-FAULT-ISOLATION...:345,359` — P2 Correspondence: same shape, one idiom).

**State machine (replaces `hydra.rs:186-193`; `Hydra` gains one field `healthy_streak: u32`,
init 0 in `new`):**

```rust
pub fn integrity_check(&mut self) -> OrganismState {
    let adj = topology_adjacency(self.nodes, &self.base_edges);
    let rho = spectral_radius(&adj);
    if !(rho < INTEGRITY_BAND.trigger) || !rho.is_finite() {
        // Trip pole: instantaneous, one check — fail-closed latency unchanged.
        // (`!(rho < t)` also catches NaN; is_finite kept for the +inf pole and clarity.)
        self.state = OrganismState::Locked;
        self.healthy_streak = 0;
    } else if self.state == OrganismState::Locked {
        if rho <= INTEGRITY_BAND.release {
            self.healthy_streak += 1;
            if self.healthy_streak >= INTEGRITY_BAND.healthy_checks {
                self.state = OrganismState::Live;
                self.healthy_streak = 0;
            }
        } else {
            // Dead band (release < ρ < trigger): hold the lock, reset the streak.
            self.healthy_streak = 0;
        }
    }
    // Live with ρ < trigger: stays Live (the band is sticky in both directions —
    // identical to today's Live-side behavior; zero behavior change while Live).
    self.state
}

/// Owner-visible introspection (same pattern as `state()`; telemetry hook, item 10).
pub fn healthy_streak(&self) -> u32 { self.healthy_streak }
```

### 3.3 Post-fix impossibility argument (hazard safety as math, contract item 6)

A `Locked→Live→Locked` flap cycle now requires: (a) ρ descending to ≤ 0.999998, (b) staying
there for 3 consecutive checks, then (c) ascending to ≥ 1.0 — a genuine sustained spectral
excursion of the full Resonant-band width, i.e. an actual topology change of that magnitude,
sustained across 3 measurement events. Because `spectral_radius` is deterministic (bit-identical
ρ for an unchanged `base_edges`), no measurement graze, float jitter, or single transient can
produce a flap. **Collapse direction is safe-directed:** every ambiguous sequence (dead-band
dwell, interrupted streak, alternation) resolves to Locked, never to Live. Designed consequence,
stated not hidden: a baseline whose true ρ permanently sits in (0.999998, 1.0) that gets Locked
once will never auto-release — correct, because a baseline within 2e-6 of instability is not
evidence of health; the documented recovery is owner re-seed (`hydra.rs:73-74`), unchanged.
Second consequence: recovery after a real transient now costs 3 clean checks instead of 1 (two
extra commit refusals for a caller retrying immediately) — the evidence-demanding direction.

---

## 4. Build 2 — restart-intensity as a launch-path predicate (T-6, item 139)

### 4.1 Spec: types, constants, exact signature (contract item 4)

Location: `kernel/src/bounded_drainer.rs` (reuse argument §11). Std-only, zero deps.

```rust
/// OTP MaxR/MaxT restart-intensity bound as a PURE LAUNCH-PATH PREDICATE.
/// (Synthesis §7 T-6 / V2 W3-L4: "a monotone relaunch fact checked by a pure
/// predicate IN the launch path — degrade-closed refuse-to-launch; a standing
/// sampler process is never built.")
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestartBudget {
    /// Max launches permitted inside any sliding window (OTP MaxR).
    pub max_restarts: u32,
    /// Sliding window length, milliseconds (OTP MaxT).
    pub window_ms: u64,
}

/// Kernel default: 5 relaunches per rolling 60 s. A sustained crash loop
/// (mean time-to-crash < 12 s) is stopped within one minute; slower periodic
/// restarts (< 5/min) are legitimate. The systemd substrate mirror, where a
/// unit exists, MUST copy these numbers (StartLimitBurst=5,
/// StartLimitIntervalSec=60) so both planes enforce the same physics —
/// Phase 27 §3.4 leaves unit-existence (unverified); resolve at implementation.
pub const DRAINER_RESTART_BUDGET: RestartBudget =
    RestartBudget { max_restarts: 5, window_ms: 60_000 };

const _: () = assert!(DRAINER_RESTART_BUDGET.max_restarts >= 1);
const _: () = assert!(DRAINER_RESTART_BUDGET.window_ms > 0);

/// Proof-of-admission token. The ONLY constructor is `launch_permitted` (the
/// field is module-private). A launcher entry point written as
/// `fn run_drainer(token: LaunchToken, ...)` therefore CANNOT be invoked
/// without the predicate having run — bypass is a compile error, not a
/// runtime gap (doc 19 §2.3 axis 1: absence-is-visible).
pub struct LaunchToken { _private: () }

#[derive(Debug, PartialEq, Eq)]
pub enum LaunchRefused {
    /// MaxR launches already inside the MaxT window.
    IntensityExceeded { attempts_in_window: u32, max_restarts: u32, window_ms: u64 },
    /// `now_ms` earlier than the last recorded launch. A rewound clock could
    /// smuggle launches past the window; unprovable headroom refuses (fail-closed).
    ClockRewound { last_launch_ms: u64, now_ms: u64 },
}
// impl Display for LaunchRefused — formats the ONE Blocker line the launcher
// emits (Phase 27 §3.4: "stop relaunching + one Blocker line"). e.g.:
// "BLOCKER: launch refused — 5 launches in 60000 ms (max 5); lane stays down"

/// The predicate. PURE and TOTAL over the monotone launch-attempt history:
/// `prior_launches_ms` is append-only with non-decreasing timestamps (the
/// launcher appends the grant time before exec; entries are never edited).
/// An entry `t` is in-window iff `now_ms − t < window_ms` (strict).
pub fn launch_permitted(
    budget: &RestartBudget,
    prior_launches_ms: &[u64],
    now_ms: u64,
) -> Result<LaunchToken, LaunchRefused> {
    if let Some(&last) = prior_launches_ms.last() {
        if now_ms < last {
            return Err(LaunchRefused::ClockRewound { last_launch_ms: last, now_ms });
        }
    }
    let attempts_in_window = prior_launches_ms.iter().rev()
        .take_while(|&&t| now_ms - t < budget.window_ms)
        .count() as u32;
    if attempts_in_window >= budget.max_restarts {
        Err(LaunchRefused::IntensityExceeded {
            attempts_in_window,
            max_restarts: budget.max_restarts,
            window_ms: budget.window_ms,
        })
    } else {
        Ok(LaunchToken { _private: () })
    }
}
```

**The monotone fact it checks:** the append-only launch-attempt history (timestamps of prior
*granted* launches). Monotone in two senses: entries are only ever appended (a crash cannot
un-happen), and timestamps are non-decreasing (violation = `ClockRewound`, refused fail-closed).
Refused attempts are NOT appended (they did not run; MaxR counts restarts, and a pure cheap
refusal loop runs no unpaid work — degrade-closed, same pole as `BoundedDrainer::tick`).

**Persistence plane (outside the kernel primitive, stated for the executor):** for in-process
relaunch loops the history is a `Vec<u64>` owned by the loop. For cross-process crash-loops —
the real case, the process died — the launcher (lib.sh-style parent or systemd) appends
`now_ms` to a per-drainer append-only ledger file before `exec`, reads it into a slice, and
calls `launch_permitted`. The kernel primitive stays pure over `&[u64]`; custody of the file is
the §2.2 finite anchor.

### 4.2 Why this is provably NOT a watchdog (the doc 19 distinction, cited exactly)

Doc 19 §2.3 established the categorical test with a compiled toy: deleting a watchdog is an
**invisible silent gap** ("`balance` sits at −20 forever, no signal" — Pattern A absent-watchdog
run), while deleting an inline verify is a **compile-time hole** ("the `Ok(value)` the caller
wants cannot be produced without the check, because the check is on the causal path of the
result"). Applying each axis:

1. **Liveness axis (doc 19 §2.3.1):** there is no standing process, no thread, no poll interval.
   The predicate executes only on the launch event itself. "Who watches the watcher" never
   arises — there is no watcher to keep alive. Deleting the check does not silently disable
   enforcement: the launcher no longer compiles, because `LaunchToken` is unconstructable
   outside the module. Absence-is-visible.
2. **Representability axis (doc 19 §2.3.2):** a watchdog samples state after the fact (the toy
   observed the bad state 3 times even WITH the watchdog running). Here there is no sampling
   window and no TOCTOU race: the history is not a sampled proxy but the authoritative record
   written by the launch path itself, and the sixth launch is never produced — refused before
   exec, mirroring "endures by NOT persisting."
3. **Checked at launch time, not a standing sampler:** it is a pure function over a monotone
   fact, evaluated at the one moment a launch is attempted — the exact structural form T-6
   ruled admissible ("like `debit` IS the gate") and V2 item 139 adopted (SPLIT verdict: fuel
   trap for the hung guest + this predicate for the crash-loop + systemd StartLimit as
   substrate-physics, "never a standing monitor").

---

## 5. The three-way split, as math not metaphor (contract item 13)

Per the operator's synthesis (idea #185; Batch 3 §6), each mechanism must name its leg:

- **Hysteresis (§3) = Self-Healing**, the emergent-math leg: the `Locked→Live` restore remains
  emergent from the spectral measurement itself (no supervisor — Batch 3 §6b(iii) preserved),
  now formalized as a deterministic finite automaton whose release transition requires a
  certificate of N=3 samples in the provably-Damped region. The error-correcting property:
  transient perturbations are absorbed (organism re-Lives) while sustained ones are held (Locked)
  — a debounce filter in the exact signal-processing sense, with a stated passband
  (ρ ≤ 0.999998 sustained ≥ 3 checks) and stopband (any excursion touching ρ ≥ 1.0).
- **Restart predicate (§4) = Self-Termination**, the hard-invariant-boundary leg: past MaxR/MaxT
  the relaunch is unrepresentable (no `LaunchToken` exists), not a supervisor's decision. Same
  class as `debit`/`BudgetExceeded` — the boundary is arithmetic over a monotone counter.
- **Snapshot Re-entry: untouched here.** The durable snapshot + restore-drill gap stays owned by
  P-B/P12 (Batch 3 §6c, Hermetic #4). This blueprint uses the word for nothing.

M7 topological heal (Batch 3 §9 item 6) and `breaker.rs` (§9 item 2, Phase 27 Wave F1a) are
absorbed-by-reference, not duplicated: the breaker shares §3's release idiom
(`probe_successes` ≡ `healthy_checks` — build it against `INTEGRITY_BAND`'s formula when F1a
lands). Wasm boundary clamps (A9/A16) stay Phase 27 Wave F1c.

## 6. Hermetic principle citations (contract items 6, 20 — stated, not hand-waved)

- **The Monocoque / physics-over-bureaucracy argument maps to P4 — POLARITY**, specifically.
  Batch 3 §0 states the mapping explicitly: "the dialogue arrives independently at the Hermetic
  Principle of Polarity" (`12-BATCH3...:39-41`), and the Hermetic verdict line
  (`HERMETIC-ARCHITECTURE-PRINCIPLES.md:315-317`) draws the earned/aspired boundary exactly
  there: what the compiler/arithmetic enforces is an earned pole (physics); what a remembered
  ritual enforces merely aspires (bureaucracy). Both builds land on the earned side: the band is
  a typed constant with compile-time shape enforcement; the token is a type.
- **P4 in miniature (hysteresis):** two named poles (trip/release), asymmetric by construction
  (1 check vs 3), with the collapse safe-directed — every ambiguous input resolves toward
  Locked (`HERMETIC...:66-78` "collapses are typed and safe-directed").
- **P5 RHYTHM:** both swings wired and structurally guaranteed to fire (`HERMETIC...:80`) — the
  trip swing and the release swing are both code paths with dedicated tests, not rituals.
- **P6 CAUSE-AND-EFFECT:** no effect escapes its declared cause (`HERMETIC...:95`) — a launch's
  only possible cause is the predicate's verdict (the token), and a lock's only possible cause
  is a measured ρ excursion (deterministic function of topology).
- **P7 GENDER (no self-certified done):** honestly NOT satisfied by `integrity_check` alone and
  not claimed — the check is self-certified (RC-2, `HERMETIC...:168`) until P06 `key_V` lands.
  §2.2 states this rather than papering over it.

## 7. Adversarial / chaos tests, including the intentionally-failing ones (item 5)

TDD order per item 3: spec (§3.2/§4.1 types) → tests below → implementation. Tests assert on
STATE SEQUENCES (the events), not only end-states.

**T1 — `hydra_integrity_flap_without_hysteresis_regression` (RED FIRST — proves the bug).**
Adversarial oscillation inducer. Base `base()` (ρ=0) plus one self-loop edge on node 0 whose
weight `w` the test rewrites between checks (a self-loop gives ρ = w exactly — diagonal
dominance; and the tests module already mutates `h.base_edges` directly, `hydra.rs:506`).
Dither for 8 checks alternating `w = 1.0 − DRIFT_BAND/2` (= 0.9999995) and
`w = 1.0 + DRIFT_BAND/2` (= 1.0000005); record the 8-state sequence; assert the number of
`Live↔Locked` transitions ≤ 2. **Against today's `hydra.rs:186-193` this FAILS (7 transitions)
— run it before implementing and record the failure.** Post-fix: one transition (locks at the
first high sample; 0.9999995 > release=0.999998, so the dither never re-releases). GREEN.

**T2 — `hydra_locked_release_requires_streak`.** From Locked, set `w = 0.9990` (≤ release);
assert the check sequence is exactly `[Locked, Locked, Live]` (dwell = 3, released on the 3rd).

**T3 — `hydra_dead_band_holds_lock` (adversarial graze).** From Locked, sequence of weights
`[0.999, 0.999, 0.999999, 0.999, 0.999, 0.999]` — the third sample sits inside the dead band
(release < 0.999999 < trigger) and must RESET the streak: assert states
`[Locked, Locked, Locked, Locked, Locked, Live]`. An intermittent graze cannot ratchet a release.

**T4 — `hydra_trigger_trips_in_one_check`.** Live, one check at `w = 1.0` exactly → Locked
immediately. Fail-closed latency provably unchanged.

**T5 — `restart_gate_refuses_sixth_launch_in_window` (the constructed "unsafe launch" the
predicate must catch).** History = 5 grants at `t = 0, 10_000, 20_000, 30_000, 40_000`;
attempt at `now = 59_999` → `Err(IntensityExceeded { attempts_in_window: 5, .. })` (the sixth
launch is refused — this IS the crash-loop scenario). Attempt at `now = 60_000` → `Ok` (entry
t=0 has aged out: 60_000 − 0 = 60_000, not < 60_000 — strict-inequality boundary pinned).

**T6 — `restart_gate_clock_rewind_fails_closed` (adversarial clock).** History `[..., 50_000]`,
`now = 49_999` → `Err(ClockRewound { last_launch_ms: 50_000, now_ms: 49_999 })`. A rewound
clock cannot smuggle a launch.

**T7 — `restart_gate_slow_periodic_crash_is_legitimate`.** A launch every 15_000 ms (4/min),
20 iterations → every attempt `Ok`. The bound does not punish legitimate restart behavior.

**T8 — `restart_gate_first_launch_always_permitted`.** Empty history → `Ok`.

**Compile-time RED checks (manual, recorded in the commit message):** (a) flip
`INTEGRITY_BAND.release` to `1.0` → `cargo build` MUST fail on the const assert; flip back.
(b) attempt `LaunchToken { _private: () }` from outside `bounded_drainer` (e.g. a scratch file
in `engine/`) → MUST fail to compile; delete the scratch.

## 8. DoD — falsifiable, machine-checkable (item 2)

1. T1 executed against unmodified `hydra.rs` and observed RED (output pasted into the commit).
2. `DRIFT_BAND` promoted in `spectral.rs`; `classify_drift` behavior bit-identical (existing
   spectral tests green untouched).
3. All of T1-T8 GREEN; full kernel suite green: `cd /root/dowiz/kernel && cargo test`
   (re-verify the live count against git, do not trust the remembered 446 — ROADMAP-GROUND-TRUTH
   rule). No existing test modified.
4. Both compile-time RED checks performed and recorded.
5. `BreachAlert` wire bytes untouched: `breach_alert_bytes_roundtrip_and_reject_bad_len` green
   (40-byte layout, `hydra.rs:97-102` — mesh payload budget unchanged, item 12).
6. `boot_verify` untouched: `hydra_boot_verify_clean_baseline` green.
7. `docs/regressions/REGRESSION-LEDGER.md` gains: REG-PC-01 = T1, REG-PC-02 = T3,
   REG-PC-03 = T5, REG-PC-04 = T6 (permanent, item 17).

## 9. Scaling, isolation, memory, benchmarks (items 8, 10, 11, 12, 15, 16)

- **Schemas & scaling axis (8):** `HysteresisBand`/`RestartBudget` are O(1) constants —
  timeless until per-drainer-class policy arrives, at which point the named breakpoint is:
  the consts become `HubPolicy` fields (Phase 27 §3.3 shape). Launch history scales with
  crash-rate × window; the predicate reads only in-window entries, so entries older than
  `window_ms` may be compacted to a count without changing any verdict (proof: they cannot
  satisfy `now − t < window_ms`) — the living-memory demote-never-delete pattern
  (`internal-retrieval-living-memory-arc-2026-07-14`, item 15).
- **Bulkhead (11):** hysteresis state is per-`Hydra` (per-organism, inside the process-per-hub
  boundary — Phase 27 §1.1, unchanged). One ledger file per drainer: a crash-looping drainer
  exhausts only its own restart budget, never a sibling's.
- **Mesh (12):** both mechanisms are node-local, zero new wire payload, no gossip.
  Side benefit: hysteresis reduces `BreachAlert` re-raise churn under dither (fewer
  Locked-entry events; witness rows were already idempotent, `hydra.rs:303-313`).
- **Benchmarks & telemetry (10):** neither path is hot. `integrity_check` already pays
  O(nodes²) `spectral_radius` per commit (`hydra.rs:227`); the additions are two comparisons
  and a counter — no measured number is claimed and none is required (the contract binds
  hot-path changes; this is a control path). Falsifier if disputed: criterion bench of
  `integrity_check` at nodes=64 before/after, acceptance delta < 1%. Telemetry hooks:
  `healthy_streak()` getter (owner-visible, `state()` pattern) and
  `Display for LaunchRefused` = the one Blocker line.
- **Tensor/spectral (16):** reuses `spectral_radius`/`classify_drift` unchanged; promotes
  `DRIFT_BAND` to single authority; introduces no transcendental (all comparisons rational —
  no eqc-rs form needed, consistent with the `rng.rs:22-28` determinism boundary).
- **Linux-discipline verdicts (9, per `BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-ADOPTION`):**
  hysteresis = EXTENDS (the kernel-thermal-throttling trip/clear-point idiom applied to the
  organism state machine); restart gate = ALREADY-EQUIVALENT at the substrate (systemd
  StartLimit) + EXTENDS in-kernel (the pure predicate mirrors the same numbers so the bound
  holds even where no systemd unit exists).

## 10. Agent-executable instructions (item 18 — zero prior context required)

1. Branch off `feat/harness-llm-backend` (or the operator-named integration branch).
2. `kernel/src/spectral.rs`: add `pub const DRIFT_BAND: f64 = 1e-6;` above `classify_drift`
   (`:342`); replace the local `BAND` uses with it. Run `cd /root/dowiz/kernel && cargo test
   spectral` — all green, none modified.
3. `kernel/src/hydra.rs` tests module: add T1 exactly as §7 specifies. Run it; confirm RED;
   save the output.
4. `kernel/src/hydra.rs`: add `HysteresisBand`, `INTEGRITY_BAND`, the three `const _` asserts
   (§3.2); add field `healthy_streak: u32` to `Hydra` (init 0 at `:168`); replace the body of
   `integrity_check` (`:180-195`) with §3.2's state machine; add `healthy_streak()` getter.
   Touch nothing else in the file — `boot_verify`, `commit`, breach paths unchanged.
5. Add T2, T3, T4. Run the full kernel suite; all green, existing tests untouched.
6. Perform compile-time RED check (a): flip release to 1.0, observe build failure, revert.
7. `kernel/src/bounded_drainer.rs`: append §4.1's types and `launch_permitted` verbatim
   (including the `Display` impl); add T5-T8. Run suite; green. Perform RED check (b).
8. Update `docs/regressions/REGRESSION-LEDGER.md` per §8.7.
9. At implementation time resolve Phase 27 §3.4's "(unverified)": check whether any systemd
   unit exists for a drainer/spool process (`systemctl list-units | grep -i 'spool\|drain'`,
   plus `/etc/systemd/system/`). If yes: set `StartLimitBurst=5`, `StartLimitIntervalSec=60`,
   `Restart=on-failure`, `RestartSec` backoff, mirroring `DRAINER_RESTART_BUDGET`. If no:
   note "lib.sh-lane only" in the commit; the file-ledger path from §4.1 is the sole plane.
10. Commit per contextual-commit discipline, citing this blueprint. Do not wire the fuel loop,
    build `breaker.rs`, or touch P06 — out of scope (§5, §11).

## 11. Reuse-first justification and explicit non-goals (item 19)

- No new module: `HysteresisBand` lives beside the state machine it governs (`hydra.rs`);
  the launch gate lives in `bounded_drainer.rs` because the drainer is the named crash-loop
  subject (Phase 27 §2.1) and the file already hosts the degrade-closed/`TokenBucket` idiom the
  predicate mirrors. Extension suffices; a `restart_gate.rs` module would add a boundary with
  one occupant.
- `DRIFT_BAND` promotion is the required refactor, not avoided: two magic `1e-6`s (one local,
  one implied by the new band) would be the exact dual-authority drift RC-4 warns about.
- **Non-goals, explicit:** P06 `key_V` (the tamper leg's independent half — separate phase,
  still `signed:false`); `breaker.rs` (Phase 27 F1a — shares §3's idiom when built); M7
  topological heal; durable snapshot/restore-drill (P-B/P12); fuel invoke-wiring +
  `FUEL_PER_UNIT` pin (B4); wasm clamps A9/A16 (F1c).

## 12. Links & contract compliance map (items 7, 20)

**Docs:** `CORE-ROADMAP-STANDARD-2026-07-17.md` (§2 contract, §3 P-C row) ·
`12-BATCH3-safety-selfhealing-findings.md` (§5 hysteresis gap, §2 restart gap, §6 three-way
split, §9 build order items 1+4) · `19-SYSTEM-COHERENCE-AND-AUTHORITY-BOUNDARY-REDO.md` Part 2
(toy proof; §2.3 compile-hole vs silent-gap; §2.4 finite-anchored verdict) ·
`BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md` §D, item 139, W3-L4 ·
`BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS.md` §7 T-6 ·
`BLUEPRINT-FAULT-ISOLATION-DECENTRALIZED-ARCHITECTURE-2026-07-17.md` §2.1, §3.2, §3.4 ·
`hermetic-architecture-2026-07-16/HERMETIC-ARCHITECTURE-PRINCIPLES.md` (P4/P5/P6/P7, RC-2,
:315-317) · `sovereign-roadmap-2026-07-16/DECART-P06-bebop2-crypto-dep.md`.
**Memory:** `sovereign-architecture-19-phase-roadmap-2026-07-17.md` (P06 three-way blocker) ·
`harness-llm-backend-and-hermetic-remediation-2026-07-17.md` (key_V `signed:false`) ·
`internal-retrieval-living-memory-arc-2026-07-14.md`.

| Contract item | Where honored |
|---|---|
| 1 ground truth | §1 (all cites re-read this pass) |
| 2 DoD | §8 |
| 3 spec/event TDD | §3.2, §4.1 types first; §7 sequence assertions |
| 4 predefined types | `HysteresisBand`, `INTEGRITY_BAND`, `DRIFT_BAND`, `RestartBudget`, `LaunchToken`, `LaunchRefused`, `DRAINER_RESTART_BUDGET` |
| 5 adversarial tests | §7 T1 (RED-first), T3, T5, T6 |
| 6 hazard math | §3.3, §4.2, §2 |
| 7 links | §12 |
| 8 scaling | §9 |
| 9 Linux discipline | §9 |
| 10 bench/telemetry | §9 (cold-path justification + falsifier) |
| 11 bulkhead | §9 |
| 12 mesh | §9 |
| 13 three-way split as math | §5 |
| 14 smart index | §3.2 const asserts, `LaunchToken` type, §8.7 ledger |
| 15 living memory | §9 (compaction-below-horizon) |
| 16 tensor/spectral | §9 |
| 17 regression | §8.7 (REG-PC-01..04) |
| 18 agent instructions | §10 |
| 19 reuse-first | §11 |
| 20 Hermetic | §6 |

---

## 13. Session research fold-in (2026-07-18) — verification CRITICALs + round-2 containment, Layer-C-owned

Added after the 2026-07-17 writing pass; sources read in full, none of §1–§12 is retracted.
Sources: `docs/design/ROADMAP-UPDATE-SESSION-SYNTHESIS-2026-07-18.md` (§0–§2, the per-repo
verification CRITICALs) and `docs/design/fail-operational-layout-versioning-2026-07-17/round-2/
BLUEPRINT-ROUND-2-MASTER-SYNTHESIS.md` (§2.3, §6 — the Layer-C-routed artifacts).

### 13.1 Landing status (what changed on `main` since this blueprint was written)

- **Build 1 LANDED** (`a50d44ab0`, `hydra.rs:85` `HysteresisBand` — see the header correction).
- The neighbouring Layer-B drift-gate this blueprint cross-cites also landed (`RetainedBase`
  admission, `7f2fc6880`/`fc330a622`) — **and is currently NaN-fail-open** because the
  `spectral_radius` NaN fold (`spectral.rs:218` `fold(0.0, f64::max)`) did not land with it:
  NaN spectrum → ρ=0.0 → `Damped` → admitted. That fix is **Layer B's owned DoD** (the synthesis
  promotes it to URGENT, §2 there), but the *law it applies is this layer's* value-bound /
  self-termination law (§2.1's class), and it is the most-corroborated finding of the session
  (dowiz V3 4.1 + spectral-evolution V1 #4 + the drift-gate chain). Note the asymmetry this
  blueprint already encoded: the **hydra path is guarded** (`!rho.is_finite()` shipped in Build 1);
  the `classify_drift`/`spectral_radius` path shared by the Layer-B gate is not. One ~5-line fix
  closes all three corroborated findings.

### 13.2 Verification CRITICALs that land in Layer C (red-team pass, `verification-2026-07-17/`, still-live @ `main 87da9ccd4`)

These are *found defects in shipped code*, folded in as owned Layer-C follow-ups — each one is an
instance of a §2 class failing in practice, which is exactly why it belongs here:

| Finding | Why it is a Layer-C item | Class it defeats |
|---|---|---|
| **`budget.rs` NaN/negative `estimate` flips degrade-closed → degrade-open** (V1 #5, HIGH): no `is_finite()`/`>= 0` guard on the estimate input, so a NaN or negative estimate permanently passes the ceiling check | §2.1 lists `ComputeBudget::debit` as *genuine zero-authority* ("refuses past ceiling"). That classification is correct **for well-formed inputs only** — the arithmetic totality assumption fails at NaN, the same hole-shape as the spectral fold. Fix = the same value-bound law (`is_finite && >= 0` at the boundary), RED-first | degrade-closed (§2.1 budget row) |
| **`budget.rs:147,156` `.lock().unwrap()` poison-cascade** (V1 #6, MED): hardened in `token_bucket.rs` (the P-H A6 fix landed), but *relocated, not eliminated* — the same pattern survives in `budget.rs` | The A6 chaos scenario (BLUEPRINT-P-H §2.4) proved this class RED against `token_bucket.rs` and forced the fix; the identical pattern one file over is the bulkhead lesson half-applied. Follow-up: extend the A6 fix (or degrade-closed deny) to `budget.rs`, ledger row per the P-H precedent | bulkhead / poison containment |
| **drift-gate `intervention == true` is an unauthenticated bool; no real `fn kill` exists** (V3 4.10, HIGH): any caller can lift the spectrum gate; the "kill-switch is the only safe stop" doctrine (`hydra.rs` boot message, §1) has no corresponding kill mechanism | The gate-lift is a *self-certified* authority claim — precisely the RC-2 shape §2.2 says only P06 `key_V` can close. Until then, the honest posture: `intervention` is an operator-ceremony input, and its unauthenticated reachability is a **named open hole**, not a feature | finite-anchored authority (§2.2) |
| **`noether.rs` Lyapunov gate primitive is fail-OPEN on NaN** (spectral V1 #2): `NaN > tol == false` admits, no `is_finite` guard (worked around in the *test*, not the *gate*); per-step-only tolerance admits unbounded cumulative growth | Same value-bound law, third instance. The 12×12 unit-weight *application* survives; the exported *primitive* does not — do not reuse `noether.rs`'s gate as a Layer-C invariant primitive until guarded | value-bound totality |

The pattern across all four (and 13.1) is one lesson, stated once: **every §2.1 "zero-authority
arithmetic" claim is conditional on input totality — NaN/negative/poison inputs re-open the very
authority the arithmetic dissolved.** The §3.2 state machine already models this correctly
(`!(rho < t)` catches NaN by comparison semantics, plus an explicit `is_finite`); the follow-ups
above are that same discipline applied to the remaining unguarded boundaries.

### 13.3 Round-2 fail-operational artifacts routed to Layer C (round-2 master synthesis §6)

The round-2 pass (FEC / CSC-LAW / CWR / LaneFrameHeader / DeltaPatch) self-mapped its artifacts
onto the Layer axis; three land here, absorbed by reference (designs live in the round-2 docs —
this section only records ownership and the reuse links):

- **Sandbox-tier containment is Layer-C substrate** (round-2 §6: "WASM gate (built), microVM VMM
  follow-up, C′ tier restriction"). Pattern C′ / CSC-LAW (Fable-B) lets a *contained* bridge
  self-certify its own work — admissible **only** because three containment layers (spatial WASM
  deny-by-default import gate · structural sealed `BridgeResult` · authority red-line
  un-nameability) bound the certifier's strongest lie to a red-line-free, fail-operational lane.
  The containment layers are bulkheading in this layer's exact §2 sense; the scope/grant law half
  is Layer D's. Named open item inherited: **the microVM tier has a probe but no VMM launch** —
  C′ cannot certify the native tier until that lands (round-2 §5.2). Test B-T6
  (`csc_never_granted_to_inprocess_tier`) is the Layer-C pin.
- **The deterministic circuit-breaker eviction predicate** (Fable-C §4.1, ADOPT): an integer
  counter of *consecutive* boolean gate refusals with a fixed bound N → intake disabled. This is
  **the same idiom as §3.2's `healthy_checks` release streak** (round-2 independently converged on
  the consecutive-count shape; health scores and innovation-magnitude triggers were explicitly
  rejected for the slot — NO-SCORING held). When the Phase-27 F1a `breaker.rs` is built, all three
  (this band, the round-2 eviction predicate, the breaker's `probe_successes`) must share §3.2's
  one formula — one idiom, three instances, per P2 Correspondence and §5's existing note.
- **Honest boundary carried forward, not re-litigated:** CSC-LAW's **RC-2-broad residual stays
  open** (a well-formed translation of *wrong content* is structurally undetectable; pinned
  executably by B-T4/E-T4, closure only via witness/N-version, both DEFER-WITH-TRIGGER — round-2
  §5.1). This matches §2.2/§6-P7's stance here exactly: containment bounds and makes visible; it
  never converts self-certification into independent verification. P06 `key_V` remains the
  independent-verification leg's blocker, unchanged.
