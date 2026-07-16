# Principle 3 — Vibration (grounded as a software-architecture principle)

> Hermetic axiom (Kybalion): *"Nothing rests; everything moves; everything vibrates."* Every
> phenomenon exists at some frequency; the difference between two things is a difference in their
> vibratory rate.
>
> This document grounds that axiom as a concrete, testable architecture principle for
> dowiz/DeliveryOS + openbebop, verifies it against real kernel/engine code, and audits the live
> tree for violations. One of 7 parallel Hermetic passes. No mysticism: every claim below cites a
> file and line.

---

## 1. The architecture-principle statement

**VIBRATION (concrete form):** *Every stateful or dynamic subsystem in this codebase has an
explicit, named, and tested RATE characteristic — a damping ratio, an oscillation period, a fixed
sampling frequency, a refill rate, a retry cadence, a drift class. That rate must be a deliberately
chosen, single-authority parameter, cross-checked by a test — never an implicit default, an
un-pinned magic literal, or an "animate/loop it and see" cadence.*

Corollaries the code already embodies:

- **A1 — Classify before you trust.** A dynamical loop must be classifiable by where its spectral
  radius ρ sits relative to the unit circle: contracting (Damped), marginal (Resonant), or
  divergent (Unstable). A subsystem that cannot state its drift class is running at an unexamined
  frequency.
- **A2 — One rate, one authority.** When a rate crosses a module or process boundary (kernel↔engine,
  drainer↔kernel), exactly one site owns the value and a test pins every mirror to it. Two
  independent "authoritative" values for the same physical cadence is the violation.
- **A3 — Fail-closed against divergence.** A chosen timestep/rate must be asserted inside its
  stability bound *before* it reaches the integrator (ζ, CFL, spiral-of-death clamp), so a divergent
  frequency can never run.
- **A4 — Money is never a field channel.** Rates govern *fields* (position, opacity, telemetry
  pacing). Monetary quantities are integer, event-sourced, and MUST NOT be interpolated by any
  easing/tween (FE-09; ARCHITECTURE.md S9).

---

## 2. Verification of the hypothesis instances against real code

All five hypothesized instances are **real and confirmed**. Four live in dowiz; one (TokenBucket)
lives in bebop.

### 2.1 `classify_drift` — a literal vibration-mode classifier for the kernel's own dynamics ✅

`kernel/src/spectral.rs:325-335`. `classify_drift(a)` computes ρ = `spectral_radius(a)` and buckets
it against the unit circle with a `BAND = 1e-6` deadband:

- ρ < 1−BAND → `DriftClass::Damped` (the loop contracts / converges),
- ρ > 1+BAND → `DriftClass::Unstable` (divergent — "more verbose each step"),
- else → `DriftClass::Resonant` (marginal / limit cycle, e.g. the μ≈−1 period-2 orbit).

The enum doc (`spectral.rs:315-323`) is explicitly a DMD |μ|-vs-1 stability reading. This is not a
metaphor for vibration — it *is* frequency-mode analysis of the kernel's own transition operator.
`dominant_period` (`spectral.rs:363-376`) closes the loop: it reads the eigenvalue nearest the unit
circle that points away from +1 and returns the literal oscillation period ℓ ≈ 2π/|arg λ| —
`Some(2.0)` for a period-2 cycle, `None` for a non-oscillatory operator (thresholds
`PERIOD_MAG=0.85`, `PERIOD_ARG=0.6`). Tested: `green_two_cycle_eigs_plus_minus_one`
(`spectral.rs:388`) asserts ρ=1, |λ₂|=1, `Resonant`, and period=2 for a directed 2-cycle;
`green_drift_class_contraction_margin_growth` (`spectral.rs:522`) proves the three-way
discrimination. **This is the anchor instance: A1 is code, not aspiration.**

### 2.2 ζ critically-damped spring — a deliberately chosen, tested vibratory rate ✅

`engine/src/motion.rs`. `Spring` (`motion.rs:14-25`) carries explicit angular frequency `omega` (ω)
and damping ratio `zeta` (ζ), integrating `ẍ + 2ζω·ẋ + ω²x = ω²·x_target` via substepped
semi-implicit Euler (`step`, `motion.rs:50-63`) that guarantees ω·dt_sub ≤ 0.1 so the discrete ζ=1
solution stays monotone. The presets encode *chosen* rates: `snappy` = ζ=1, ω≈30, friction=60
computed as exactly 2·ζ·√k (`motion.rs:66-68`); `fluid` ζ≈0.7; `playful` ζ≈0.35. The RED→GREEN
tests are the discipline in action: `zeta_one_no_overshoot` (`motion.rs:100`) proves ζ=1 never
overshoots (max_x ≤ 1+1e-3), `zeta_below_one_overshoots` (`motion.rs:123`) proves ζ<1 bounces,
`spring_is_field_channel_not_money` (`motion.rs:163`) documents A4. **This is A4 + "never animate and
see" as executable proof — ζ is a named parameter with a falsifiable overshoot test.**

### 2.3 The damped-wave field equation `MÜ+ΓU̇+c²LU=S` ✅

`engine/src/field_frame.rs:10-11` implements the operator directive
`M·U̇ = −ΓU̇ − c²·L·U + S` (semi-implicit, `U_next = (U + dt·(Γ·U̇ + c²·L·U) + dt·S)/(1+dt·M)`).
`FieldEquilibrium` (`field_frame.rs:29-38`) names the four rate/decay parameters: mass M, damping Γ
(`gamma`), wave-speed² c² (`c2`), and timestep dt. `assert_stable` (`field_frame.rs:55-68`) is A3 in
code: it panics unless `0 < dt ≤ M/(Γ+2·c²)` (a CFL-ish bound from the Laplacian's `[-4,0]` discrete
eigenvalues) and is called before every `step` (`field_frame.rs:139-140`). The damping Γ is a
first-class, fail-closed rate. **Confirmed — but see Finding 1: its dt default contradicts the
kernel's authority.**

### 2.4 `TokenBucket` refill_rate — a literal frequency governing spend (bebop) ✅

`bebop-repo/bebop2/proto-wire/src/transport_policy.rs:30-58`. `TokenBucket` holds
`refill_per_sec: u32` and `(tokens, last_refill_unix_sec)`; `try_acquire`
refills `tokens + elapsed·refill_per_sec` capped at `capacity`, fail-closed at 0. This is the F33/F6
budget throttle from `ARCHITECTURE.md` (F6 "LOCK + TokenBucket", F33 "TokenBucket throttles") — a
frequency (tokens/sec) that governs concurrency/spend. Tested: `red_token_bucket_exhaust_then_refill`
(`transport_policy.rs:188`). **Confirmed; note this rate lives in bebop, not dowiz.**

### 2.5 Fixed-timestep `DT_STABLE` — a deliberately chosen, mirror-pinned sampling frequency ✅

`kernel/src/lib.rs:180` `pub const DT_STABLE: f32 = 0.02` — declared the *single source of truth* for
"the field-sim/animation integrator," and documented (`lib.rs` header) as **50 Hz — the cadence at
which route-ping kinematics (geo) are sampled.** `engine/src/loop_.rs:19` mirrors it and the
`FixedTimestep` accumulator (`loop_.rs:52-82`) guarantees the integrator only ever sees `DT_STABLE`,
with `MAX_FRAME=0.25` spiral-of-death clamp and `MAX_SUBSTEPS=5` (A3). Both sides pin it:
`dt_stable_is_authoritative` (`kernel/src/lib.rs`) and `dt_stable_matches_kernel_contract`
(`loop_.rs:163`) both assert `==0.02` and `1/dt==50`. **This is the model implementation of A2 —
a rate crossing a module boundary, owned once, pinned by a test on both sides.**

---

## 3. Audit findings (violations & weaknesses)

### FINDING 1 — Two disagreeing "authoritative" fixed timesteps in the same UI engine — **MEDIUM**

`kernel/src/lib.rs:172-180` states the field-sim/animation integrator "MUST only ever see this dt"
(DT_STABLE = 0.02 s = **50 Hz**), pinned by tests on both kernel and engine sides. But
`engine/src/field_frame.rs:47` independently defaults `dt: 0.016` (**~60 Hz**), and
`field_frame.rs:143` integrates the damped-wave field with `let dt = eq.dt` — i.e. the field steps
at 0.016 regardless of the 0.02 accumulator cadence that `loop_.rs` is built to enforce. Nothing
links the two constants; no test asserts `FieldEquilibrium::default().dt == DT_STABLE`. If the
`FixedTimestep` loop (0.02 steps) drives the field composer, the field integrates as though dt=0.016
— a **25 % rate mismatch** between the system's sampling clock and the field's integration constant.
This directly violates A2 ("one rate, one authority"): the engine has three integrators
(`FixedTimestep` at 0.02, `Spring::step` fed 1/60≈0.0167 in its own tests at `motion.rs:104`, and
`field_frame` at 0.016) and only the first is pinned to the kernel authority. `field_frame`'s dt is
named and stability-asserted (good, A3) but decoupled from the declared single source of truth. The
kernel comment's own promise — "the field-sim integrator MUST only ever see DT_STABLE" — is
falsified by `field_frame`'s default. *Fix:* make `FieldEquilibrium::default().dt` derive from (or a
pin-test equal) `DT_STABLE`, or document that field-diffusion runs on a deliberately separate 60 Hz
clock and add the mirror-pin test proving that choice is intentional.

### FINDING 2 — Cross-process pacing rate governed only by a comment, not a pin — **LOW-MEDIUM**

`tools/telemetry/rust-spool/src/main.rs:29-30` declares `TG_MIN_GAP_S: f64 = 3.5` with the comment
*"MUST match hermes-kernel `reporting::TG_MIN_GAP_S`."* This is a rate contract (1 message / 3.5 s)
that crosses a **repo boundary** into `hermes-agent-kernel-rewrite`, yet it is enforced by nothing
but a comment — there is no shared constant and no test pinning the mirror (contrast DT_STABLE in
Finding-model 2.5, which gets `dt_stable_is_authoritative` on both sides). `async-spool` repeats the
same value as `DEFAULT_GAP_S: f64 = 3.5` (`tools/async-spool/src/main.rs:37`, overridable via
`gap_seconds()` env at line 80). The pacing frequency can silently drift out of sync with the kernel
authority the moment either side is edited. This is A2 partially honored (single value, named
constant) but the *authority link is undefended* — the exact failure mode the DT_STABLE mirror-pin
exists to prevent, left unpinned for the telemetry cadence. *Fix:* a build-time or test-time
assertion that the drainer's gap equals the kernel's `TG_MIN_GAP_S`, or a shared constant.

### FINDING 3 — Linear, un-jittered retry backoff duplicated across two spools — **LOW**

`rust-spool/src/main.rs:138,144,208` and `async-spool/src/main.rs:256,262,278,339` all back off with
`std::thread::sleep(Duration::from_secs(2 * attempt as u64))` — a **linear** ramp (2 s, 4 s, 6 s, 8 s)
capped at `MAX_ATTEMPTS = 4`, with **no jitter/decorrelation**. It is governed enough not to be a
naive hot-loop (bounded, named cap), so this is a weakness, not a red-line. But: (a) the backoff is
linear, not exponential — an unusual choice with no stated justification, i.e. an *implicit* rate
policy; (b) there is no jitter, so N spools retrying a downed Telegram/HTTP endpoint synchronize into
a thundering herd (a resonance the mesh doc's own no-SPOF ethos would want damped); (c) the slope
literal `2` and the terminal `sleep(Duration::from_secs(2))` transient-failure pause
(`rust-spool:208`, `async-spool:339`) are magic numbers duplicated across 6 sites with no shared
constant. Additionally `IDLE_POLL_S = 2` (both spools) is a **fixed** empty-queue poll with no
adaptive/exponential idle backoff — a constant 0.5 Hz file-poll regardless of how long the queue has
been empty. All named, none spectrally reasoned. *Fix:* one shared backoff helper with documented
exponential-plus-jitter, or an explicit ADR that linear-no-jitter is deliberate for a single-drainer
deployment.

### FINDING 4 — Legacy ad-hoc-easing UI is quarantined, not deleted (anti-pattern retrievable) — **INFORMATIONAL / CONFIRMED-CLOSED**

The prior "money-tween / un-ported legacy animation" concern is **now closed by quarantine**: the
entire TypeScript DOM UI sits under `apps/web/node_modules/@deliveryos/.ignored_ui/` — **not
git-tracked** (`git ls-files` empty) and **not imported** by `apps/web/src` (grep empty); the
canonical motion authority is the Rust engine (`engine/src/motion.rs`). Concretely,
`.ignored_ui/src/components/molecules/AnimatedNumber.tsx:10,22` still contains the exact violations:
a hardcoded `duration = 240` ms with an ad-hoc cubic ease-out `eased = 1 - Math.pow(1 - progress, 3)`
(not a ζ-damped spring — "animate and see"), animating a bare `value: number` with a currency
`formatter` — i.e. a **money tween**, the precise thing FE-09/A4 forbid. Because it is out of the
build, severity is informational. But quarantine-in-`node_modules` is not deletion: the anti-pattern
is physically present and would re-introduce ungoverned easings and a money-tween the moment anything
re-imports it. *Fix / watch:* delete the quarantined tree, or add a CI guard that forbids importing
`.ignored_ui`, so the closed finding cannot silently reopen.

---

## 4. Verdict

The Vibration principle is **strongly and literally instantiated** in the sovereign core, more so
than any decorative reading would predict. The kernel classifies its *own* dynamics by damping mode
(`classify_drift`, `dominant_period`), the UI motion layer runs on named ζ damping ratios with
overshoot tests, the field runs a damped-wave PDE with a fail-closed CFL bound, spend is throttled by
a literal refill-per-second bucket, and the master sampling frequency (DT_STABLE = 50 Hz) is
single-authority and mirror-pinned. A1, A3, and A4 are executable, not aspirational.

The gap is **A2 consistency of rate-authority**. The codebase has one exemplary pinned mirror
(DT_STABLE) and then several rates that are named but *unpinned* or *decoupled*: the field integrates
at 60 Hz while the loop clock is 50 Hz (Finding 1, MEDIUM); the telemetry pacing gap is a
cross-repo contract defended only by a comment (Finding 2, LOW-MED); retry/idle cadences are linear,
un-jittered, and duplicated as magic literals (Finding 3, LOW). None is a red-line, but together they
show the discipline is applied unevenly — physics gets pinned tests, plumbing gets comments. The one
legacy reservoir of truly ad-hoc, money-tweening easing is correctly quarantined out of the build
(Finding 4) but not deleted.

**Net:** principle real and load-bearing; enforcement mature at the physics core, immature at the
process/tooling edges. Highest-value single fix: pin `field_frame`'s dt to `DT_STABLE` (or prove the
60 Hz choice deliberate), collapsing the codebase to one governed integration frequency.
