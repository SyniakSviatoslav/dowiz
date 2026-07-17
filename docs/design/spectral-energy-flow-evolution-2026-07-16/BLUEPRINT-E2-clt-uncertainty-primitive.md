# BLUEPRINT — E2: CLT UNCERTAINTY PRIMITIVE (intervals for every reported eval scalar)

> Source finding: `RESEARCH-AND-REASONING.md` §3.2 (cluster 2, STRONG), re-confirmed by
> `RESEARCH-VERIFICATION.md` Check 2 and by a live re-read for this blueprint. This is a **genuinely
> new capability**, not a Hermetic-audit remediation row — there is no audit finding numbered for it.
> What it *does* close is the named repo-wide habit the BRAIN-TOPOLOGY self-watch calls
> **"no falsifiable statistic"** (`HERMETIC-ARCHITECTURE-PRINCIPLES.md:159,335`; the phrase itself in
> `RESEARCH-AND-REASONING.md:235`): point estimates shipped as headlines with no uncertainty, so a
> claim replaces a check (RC-1 self-certification). It also pre-empts a latent P2 violation: the CLT
> logic exists **once, inside a single test**, and will be hand-duplicated the moment a second consumer
> needs it.
>
> **Depends on:** none. Pure leaf math; no red-line paths (no money/auth/RLS/migrations).
> **Parallel-safe with:** E1, fully. E3-Phase-A, logically (neither consumes the other's output) but
> ⚠ CORRECTED (`SPECTRAL-EVOLUTION-CONSOLIDATED.md` §4/§5): **not file-disjoint** — this blueprint's
> steps 5-6 (`brier_ci`/`ece_ci`/`aurc_ci`, the interval-aware `RegressionGate` constructor) and E3's
> steps 1/4 both edit `kernel/src/evals.rs`. Land this blueprint first (smaller diff; its SE-derived
> tolerance is a soft quality input to E3's gate, not a hard dependency) and serialize E3's `evals.rs`
> edits after it.
> **Scope:** planning artifact only. No code is written or edited here.

---

## §0 — The problem

The Central Limit Theorem is already implemented in this codebase, correctly and from first
principles — but it is imprisoned inside one test (`kernel/src/causal.rs:2226-2256`). Everywhere else a
number is reported, it is reported **naked**: `evals.rs`'s `brier`/`ece`/`aurc` scores are point
estimates with no error bar (`evals.rs:331,348,382`); the headline "recall@5 = 1.0" that this session's
own memory cites repeatedly is asserted as an exact equality over a small oracle with no confidence
interval (`living_knowledge.rs:314`, `retrieval/tests.rs:240`); and `RegressionGate`'s alarm threshold
`tol` is a hand-picked constant passed by the caller (`evals.rs:557`), not derived from the metric's own
sampling error. A grep across `kernel/src` and `engine/src` for `wilson|clopper|confidence_interval|
bootstrap|mean_se|std_error` returns **zero hits**; `se_factor` appears only as four lines inside the
one causal test. The fix is to promote the √N law the kernel already wrote into a small, seeded,
reusable primitive and attach a real interval to every scalar that is currently reported bare.

## §1 — Current-state evidence (live re-read)

**The CLT already derived, `causal.rs:2226-2256`.** The test derives the estimator's *true* asymptotic
standard error — no magic constant (docstring `:2218-2221`):

```
se_factor = sqrt( Σ_z  P(Z=z)² · p_yz·(1 − p_yz) / P(X=1, Z=z) )          // :2238-2243
assert:   err · √N  <  se_factor · 6.0    for N ∈ {200, 2_000, 20_000, 200_000}   // :2249-2250
```

`se_factor` is the asymptotic std of `error·√N` for the back-door ratio estimator; the gate says the
error shrinks like 1/√N and stays inside a 6σ normal envelope. **This is the CLT, expressed as a
falsifiable test.** It is not exposed anywhere — `causal.rs` imports `rng` but no stats module exists to
import.

**Consumers reporting point estimates with no interval:**
- `brier(prob, outcome)` — `evals.rs:331`; `ece(...)` — `:348`; `aurc(...)` — `:382`. Each returns a
  bare `f64`. `brier` and `ece` are means of per-sample terms (a `mean_se` fits directly); `aurc` is a
  rank-cumulative statistic (not a simple mean — needs bootstrap).
- **Recall@5 headline.** The live in-kernel oracle is **12 queries**, each with exactly one relevant
  doc, so each query is a Bernoulli trial and mean recall@5 = 1.0 means **12/12 successes**
  (`living_knowledge.rs:269-322`, `retrieval/tests.rs:202-241`; the `recall_at_k` scorer at
  `csr.rs:387`). The "29-question" figure in memory is the earlier `harness-hardening` oracle
  (2026-07-07); both are asserted as exact `1.0` with no interval. This is the **concrete first
  consumer** of the new primitive.
- `RegressionGate::new(window, tol, lower_is_better)` — `evals.rs:557`; the caller supplies `tol` by
  feel. `observe` pushes each sample through an EMA (`:571`) and alarms on a monotone degradation streak
  (`:583-596`). The gated quantity is therefore an **EMA-smoothed, serially-correlated** stream — which
  matters for §2.

**Seeded substrate.** `rng.rs` is the only sanctioned randomness: `Rng::new(seed, stream)` →
`next_index(n)` (rejection-sampled, unbiased, `:100`), pinned to published SplitMix64/PCG64 reference
vectors (`:154-196`). Its own doctrine (`:20-29`) warns that only the **integer** stream is
cross-platform bit-identical; transcendental float paths are per-target. Any bootstrap MUST draw through
this — never `std::time`/ambient entropy (P6 Cause-and-Effect, `HERMETIC…:95-108`).

## §2 — Target-state design

**Primitives (one implementation each, P2 "one concept, one primitive"):**

```rust
// Analytic — pure +,−,×,÷,√ only (IEEE-754-exact cross-target; P6-clean).
fn mean_se(samples: &[f64]) -> f64;                       // Bessel-corrected std / √n
fn normal_interval(point: f64, se: f64, z: f64) -> (f64, f64);   // point ± z·se
fn wilson_interval(k: u64, n: u64, z: f64) -> (f64, f64);        // small-n binomial
// Exact form of the causal gate, relocated bit-identically:
fn within_clt_envelope(error: f64, n: usize, asymptotic_se: f64, z: f64) -> bool
    // ≡  error * (n as f64).sqrt() < asymptotic_se * z
// Seeded resampling — for non-analytic estimators (aurc, ece) and dependent streams.
fn bootstrap_interval(samples: &[f64], stat: impl Fn(&[f64]) -> f64,
                      resamples: usize, z: f64, rng: &mut crate::rng::Rng) -> (f64, f64);
```

**Location — a new leaf `kernel/src/stats.rs`, and here I diverge from the edit-don't-create bias with a
stated reason.** The consumers span layers: `causal.rs` (foundational Pearl stack), `evals.rs` (harness),
and the `retrieval`/`living_knowledge` recall tests. `evals.rs` already imports *downward* from `csr`,
`kalman`, `spectral`, `noether`, `rng`. If the primitive lived in `evals.rs`, then `causal.rs` would
have to import *upward* from the eval-harness layer — a layering inversion, and exactly the kind of
tangle P2 forbids. A zero-dep leaf `stats.rs` (a sibling of `rng.rs`/`money.rs`/`noether.rs`) that every
layer depends on downward is the correct single home. This is the CLAUDE.md-sanctioned "justify a new
file" case, not a reflex new module.

**Which bound for which consumer (be precise):**

| Consumer | n | Model | Bound | Why |
|---|---|---|---|---|
| `causal.rs` empirical gate | 200…200 000 | mean, large n | `within_clt_envelope` (CLT/normal) | large n, finite variance — normal envelope is exact-in-the-limit; keep the domain-specific `se_factor` derivation in the test |
| `brier`, `ece` | eval-set size | mean of per-sample terms | `mean_se` → `normal_interval` | each is a sample mean; CLT applies directly |
| `aurc` | eval-set size | rank-cumulative | seeded `bootstrap_interval` | not a simple mean; no closed-form SE |
| **recall@5 headline** | **12 (was 29)** | binomial k/n | **`wilson_interval`** | small n, p̂ at the boundary (1.0) — normal/Wald gives the absurd [1,1]; Wilson does not |
| `RegressionGate` tol | window | EMA-smoothed → **non-iid** | see below | serial correlation breaks CLT |

**Why Wilson, not Wald or Clopper-Pearson, for recall@5.** At p̂ = 1.0 the Wald interval collapses to
`[1, 1]` — it claims certainty from 12 lucky trials, the self-certification failure in miniature. Wilson
does not degenerate: for k = n successes it reduces to `n/(n + z²)`, giving (95%, z = 1.96) a lower bound
of **≈ 0.76 for n = 12** and **≈ 0.88 for n = 29** (computed; the implementer must reproduce these
exactly). Clopper-Pearson is the exact alternative (≈ 0.74 / ≈ 0.88) but needs an inverse-beta quantile —
a **transcendental**, which the `rng.rs:20-29` reproducibility doctrine flags as per-target-only. Wilson
uses one `sqrt` over rationals and stays P6-clean cross-platform. **Recommend Wilson as canonical**; note
Clopper-Pearson in a comment as the exact-but-transcendental option, deliberately not shipped.

**The hard case, named honestly — EMA-smoothed streams (`RegressionGate`).** The gated series is
`EmaTracker`-smoothed, so consecutive values are strongly autocorrelated; a `mean_se` over them would
badly *under*estimate the true error (effective sample size ≪ n) and the gate would fire on noise.
**Recommendation, in priority order:**
1. **Preferred — move the statistic upstream of the smoothing.** Derive `tol` from the SE of the **raw**
   per-run metric window (`mean_se`, or `wilson_interval` when the metric is a pass/fail rate). Separate
   eval runs are far closer to iid than their EMA outputs; the EMA stays as the *display* trend line
   while the *decision* reads the raw independent samples. Simplest, and it makes the iid assumption
   defensible instead of quietly false.
2. **If the decision genuinely must run on a dependent stream** — a seeded **circular moving-block
   bootstrap** over `rng.rs` (block length ℓ ≈ ⌈n^(1/3)⌉): resample contiguous blocks so local
   autocorrelation is preserved, needs no iid assumption, reuses `bootstrap_interval`'s seeded-resample
   core, stays deterministic under P6.
3. **Rejected — a plain Hoeffding bound.** Hoeffding *also* assumes independence; its valid
   dependent-data form (Azuma/martingale) is more machinery for a *looser* bound than the block bootstrap
   already yields. Do not use it here.

Everything seeded and deterministic: `bootstrap_interval` takes `&mut Rng` and draws only via
`next_index`; there is no `std::time`, no thread RNG, no HashMap-order leak into any emitted number (P6).

## §3 — Migration steps

1. Add `kernel/src/stats.rs` with the six signatures above; register `pub mod stats;` in `lib.rs`.
   Zero new dependencies (matches the kernel invariant).
2. Unit-pin each primitive with a hand oracle: `mean_se([x;n]) == 0`; `wilson_interval(12,12,1.96).0`
   ≈ 0.7575 and `(29,29,1.96).0` ≈ 0.8830; `bootstrap_interval` reproducible for a fixed seed
   (serialize→re-read second-process test, the audit-#19 shape the repo already uses in `rng.rs:203`).
3. **Rewrite the `causal.rs` test to CALL the primitive** — the P2 close. Replace the inline
   `err * (n as f64).sqrt() < se_factor * 6.0` with
   `within_clt_envelope(err, n, se_factor, 6.0)`. The `se_factor` *derivation* (domain-specific to the
   back-door estimator) stays in the test; only the envelope predicate moves. Because
   `within_clt_envelope` is the **byte-identical expression**, the test's pass/fail is provably
   unchanged — this is what makes the substitution safe.
4. Attach intervals at the recall consumers: alongside `assert_eq!(mean, 1.0, …)`
   (`retrieval/tests.rs:240`, `living_knowledge.rs:314`) compute `wilson_interval(successes, n, 1.96)`
   and assert its lower bound, and surface it in the `println!` headline so the reported string carries
   the interval, not a bare `1.0`.
5. Offer interval-returning companions in `evals.rs` (`brier_ci`, `ece_ci`, `aurc_ci`) that call the
   primitive; keep the existing point-estimate fns for callers that only want the scalar.
6. Add an **interval-aware `RegressionGate` constructor** (`from_se` / `with_derived_tol`) that computes
   `tol = z · SE` from the raw window per §2, leaving the current hand-tol constructor in place for
   backward compatibility.

## §4 — Acceptance criteria (numbered, falsifiable)

1. **Regression-safe substitution.** With `within_clt_envelope` substituted into
   `empirical_converges_to_analytic_as_n_grows`, the test passes for the identical
   `N ∈ {200, 2 000, 20 000, 200 000}` and fails if the primitive's inequality direction is flipped —
   demonstrated by running both (green after, red on a deliberately inverted primitive). Because the
   expression is byte-identical, no numeric drift is possible.
2. **`se_factor` no longer duplicable.** After the rewrite, a grep for the inline
   `err * … .sqrt() < … * 6.0` pattern outside `stats.rs`/its test returns the single relocated call;
   the CLT logic exists in exactly one place (P2 satisfied).
3. **Recall@5 carries a real interval.** The recall tests emit
   "recall@5 = 1.0, 95% Wilson lower bound = L". For the live 12-query oracle **L ≈ 0.76**; for the
   historical 29-query oracle **L ≈ 0.88** (the implementer computes both for real — do not hardcode
   without recomputing). A failing query (11/12) must move L, not leave it at an assertable constant.
4. **Wilson does not degenerate.** `wilson_interval(12,12,1.96)` returns a lower bound in (0,1),
   provably ≠ the Wald `[1,1]`; a hand test pins `n/(n+z²)`.
5. **Interval-aware gate beats the hand tol (the honest falsifier).** Construct a synthetic metric
   stream: a true-flat mean with per-run noise of magnitude σ, run for `window` steps. The old
   `RegressionGate` with a hand `tol < z·σ` **fires** (false positive on noise); the new SE-derived gate
   (`tol = z · mean_se(raw window)`) **does not fire**. Then inject a real shift of `> z·σ`: the
   SE-derived gate **does** fire. Both directions asserted — the gate's false-positive rate becomes a
   stated, testable quantity instead of a feel-picked constant.
6. **Determinism (P6).** `bootstrap_interval` with a fixed `Rng` seed reproduces bit-identically across a
   serialize→re-read second-process run; no `std::time`/ambient entropy anywhere in `stats.rs`.
7. **Zero-dep / CI.** `stats.rs` adds no crate; the kernel suite stays green and grows by the new
   RED/GREEN tests.

## §5 — What this unblocks

- Turns the recall@5 headline from a self-certified `1.0` into a **falsifiable** statement — directly
  answering the memory arc's own "NEXT bigger oracle": the Wilson lower bound quantifies *exactly* how
  much a bigger oracle would buy (0.76 → 0.88 → higher as n grows).
- Gives `RegressionGate` a principled, stated false-positive rate — the prerequisite for the Self-Harness
  loop (`RESEARCH-AND-REASONING.md` §3.3) to make **non-regressive acceptance** decisions that are not
  noise, and for the E3 `SelfAdaptator` accept/reject to be statistically grounded.
- Provides the one seeded interval primitive every future eval surface (`verify_retrieval.rs`'s bounded
  retry could gain a statistical stop rule) draws on — one implementation, downward dependency, no fifth
  hand-rolled √N law.
- Counteracts the named "no falsifiable statistic" habit at the layer where it does the most damage:
  every reported number now carries the check that would refute it.

*E2 blueprint. Evidence re-read against the live tree on `feat/spectral-energy-flow-evolution`
2026-07-16 (`causal.rs`, `evals.rs`, `rng.rs`, `csr.rs`, `living_knowledge.rs`, `retrieval/tests.rs`,
`HERMETIC-ARCHITECTURE-PRINCIPLES.md`). Wilson bounds computed, not guessed. No code written or edited.*
