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

---

## Extended Context

The framing "the CLT was imprisoned in one test" undersells why an uncertainty primitive is
load-bearing rather than tidy. This whole session has been built around one discipline —
**honest claims** — and its two named failure shapes: **Mentalism** (the map is not the territory;
a stated number is a claim *about* reality, not reality) and **RC-1 self-certification** (a claim
substituting for a check — the `HERMETIC…` "52s GREEN on a 1610-line diff" case, MEMORY.md's
BRAIN-TOPOLOGY watch). A point estimate shipped as a headline — `recall@5 = 1.0` — is not a smaller,
softer version of that failure. It is *structurally the same object*: a number nobody can falsify
because it does not state its own confidence. `1.0` with no interval makes exactly the assertion an
unverified "done" makes — "trust the value, there is nothing left to check" — and it is wrong in the
same way, because 12/12 lucky Bernoulli trials cannot license certainty. An interval is the check
travelling *with* the claim; without it, the claim is self-certifying by construction.

The first real consumer this unblocks is concrete and already computed elsewhere this session: the
living-knowledge **recall@5** metric, whose live in-kernel oracle is **12 queries**
(`living_knowledge.rs:269-322`, `retrieval/tests.rs:202-241`), each a single-relevant-doc Bernoulli
trial, so `1.0` means 12/12. The honest bound on that exact sample is a **Wilson 95% lower bound
≈ 0.76** — meaning the true recall could sit as low as ≈0.76 and still be perfectly consistent with
12 clean passes. Once this primitive ships, a reader of that metric sees **`1.0 [0.76, 1.0] n=12`**
instead of a bare `1.0`.

That change is materially more honest for one reason that survives adversarial reading: the bare `1.0`
is *unfalsifiable as displayed* — no reader can tell whether it rests on 12 trials or 12 000 — whereas
`1.0 [0.76, 1.0] n=12` states its own weakness on its face. It publishes the sample size (so the
reader knows how much evidence stands behind it), publishes the floor (so "as good as 1.0, as weak as
0.76" is legible without re-deriving anything), and makes the memory arc's own standing "NEXT: bigger
oracle" a *quantified* action rather than a vibe — the interval says exactly what a bigger oracle
buys (0.76 → 0.88 at n=29 → higher). The number stops being a trophy and becomes a measurement.

## Definition of Done

Distinct from §4's falsifiable acceptance criteria (which are per-test RED/GREEN oracles), the DoD
below is the set of structural gates that must ALL hold for the work to be considered complete. Each
is checkable; none is a matter of taste.

- **D1 — Layering gate (downward-only).** `kernel/src/stats.rs` must depend on **nothing above the
  leaf layer** — concretely: `stats.rs` must **not** `use` `evals.rs` or `causal.rs`; only the reverse
  is permitted (`causal.rs` and `evals.rs` import *down* into `stats.rs`). This restates §2's location
  decision as an enforceable rule, and the reasoning is re-checked and still sound: the consumers span
  `causal.rs` (foundational Pearl stack), `evals.rs` (harness), and the retrieval/recall tests; if the
  primitive lived in `evals.rs`, `causal.rs` would have to import *upward* from the eval-harness layer
  — a layering inversion P2 forbids. A zero-dep sibling of `rng.rs`/`money.rs`/`noether.rs` that every
  layer imports downward is the only home that avoids the inversion. **Verification:** a grep for
  `use crate::evals` or `use crate::causal` inside `stats.rs` returns zero hits, and the crate compiles
  with `stats` declared *before* `evals`/`causal` have any bearing on it.
- **D2 — Regression-safety gate (no verdict may flip).** Substituting the primitive
  (`within_clt_envelope`) into `causal.rs`'s existing `empirical_converges_to_analytic_as_n_grows`
  test must **not flip that test's — or any existing test's — pass/fail verdict.** This is proven, not
  asserted, by a named procedure: **run the full `causal.rs` test suite immediately before the
  substitution and immediately after, and diff the two verdict sets — they must be identical
  (same tests present, same green/red for each).** The substitution is only safe because
  `within_clt_envelope(err, n, se_factor, 6.0)` is the **byte-identical expression** to the inline
  `err * (n as f64).sqrt() < se_factor * 6.0`, so no numeric drift is possible; D2 is the demonstration
  that this identity was actually preserved and not merely intended. A deliberately inverted primitive
  (inequality reversed) MUST make the before/after diff go red — that is the falsifier that proves the
  gate has teeth. This gate generalizes beyond `causal.rs`: **no existing test anywhere in the kernel
  suite may change verdict** as a result of this change; the suite grows by new RED/GREEN tests only.
- **D3 — Single-source gate (P2).** After the rewrite, the inline `err * … .sqrt() < … * 6.0` pattern
  exists in exactly one place (`stats.rs` / its test); a grep for it elsewhere returns the single
  relocated call. The √N law is written once.
- **D4 — Number-reproduction gate.** The Wilson bounds are **recomputed by the implementer**, not
  copied from this doc: `wilson_interval(12,12,1.96).0` ≈ 0.7575 and `(29,29,1.96).0` ≈ 0.8830 are
  reproduced to their pinned digits, and a failing recall query (11/12) is shown to *move* the lower
  bound rather than leave it at an assertable constant.
- **D5 — Determinism gate (P6).** `bootstrap_interval` draws only through `crate::rng::Rng::next_index`
  and reproduces bit-identically across a serialize→re-read second-process run; `stats.rs` contains no
  `std::time`, no thread RNG, no HashMap-order leak into any emitted number.
- **D6 — Zero-dep / suite-green gate.** `stats.rs` adds no crate to `Cargo.toml`; the full kernel suite
  is green after the change (D2's "after" run *is* this run).
- **D7 — Backward-compatibility gate.** The existing point-estimate functions (`brier`/`ece`/`aurc`)
  and the hand-`tol` `RegressionGate::new` constructor remain present and unchanged in behavior; the
  interval-returning companions and the SE-derived constructor are **additive**. No existing caller is
  broken by this change (this is the flip side of D2 for non-test callers).
- **D8 — Misuse-warning gate.** Each analytic primitive that assumes large-n iid (`mean_se`,
  `normal_interval`, `within_clt_envelope`) carries a doc-comment stating that assumption and pointing
  to `wilson_interval` for small-n binomial and to §2's block-bootstrap for dependent streams (see
  Safety below — this is currently the *only* guardrail against the EMA-stream misuse, and D8 makes
  writing it non-optional).

## Event-Driven Architecture Treatment

**Stated plainly: a pure statistics/math leaf is not naturally event-sourced, and this blueprint does
not force that framing onto it.** `mean_se`, `normal_interval`, `wilson_interval`,
`within_clt_envelope`, and `bootstrap_interval` are referentially transparent functions of their
inputs — same arguments, same output, no state, no time, no I/O. There is no lifecycle to log, no
prior-event dependency, nothing to replay. `stats.rs` sits beside `rng.rs`/`money.rs`/`noether.rs`,
which are event-sourced by *nobody*. Inventing an event stream for a `sqrt` would be exactly the
metaphor-forcing the arc's V6 discipline rejects.

The one genuine question is downstream: when a computed interval gets **attached to a metric that a
verdict-bearing consumer acts on**, does the provenance of that interval (which primitive, what sample
size, what `z`) survive into the record a future auditor reads? The honest, code-checked answer starts
by correcting the question's implied plumbing. **`RegressionGate`'s RED verdict does not flow through
the content-addressed `MeshEvent` log at all.** The gate is explicitly *pure* — "the gate and EMA are
pure (no fs) so they stay testable offline. Writing to disk is the caller's act… never hidden inside
the kernel" (`evals.rs:415-426`). A RED verdict is surfaced by the caller writing an **`EvalRow`** to a
separate **append-only JSONL trace** (`run-history.jsonl` via `EvalRow::append_to`, `evals.rs:489`),
consumed by the Node `analyze.mjs` pipeline — *not* a `MeshEvent` whose `payload: Vec<u8>` is hashed
into the SHA3-256 content chain (`event_log.rs:134-153`). So the literal "carry provenance into the
event's payload" scenario does not arise today: there is no `MeshEvent` in the regression path.

But the analogous provenance question **is real one layer down, at the JSONL trace** — and there it is
a genuine, small gap, not a covered case. `EvalRow`'s schema is fixed and byte-locked to `analyze.mjs`
(`timestamp`, `config_version`, `category`, `subagent`, `model`, `passed`, `gating_failed`,
`soft_failed`, `checks[]` — `evals.rs:441-452`). If the interval-aware constructor (§3 step 6) fires
RED because `tol = z · SE` was crossed, the row written to `run-history.jsonl` records only the failed
check *names* (`gating_failed`/`soft_failed` are `Vec<String>`); it records **nothing** about the `SE`,
`n`, `z`, or which `stats.rs` primitive produced the tolerance. A future auditor reading that line
cannot reconstruct *why* the regression was flagged without re-deriving the math from the raw window —
if the raw window is even still available. **Concrete, minimal design to close it, without breaking the
byte-lock:** the interval-aware constructor emits an *additive*, optional sidecar object on the row —
`tol_provenance: { primitive: "wilson"|"mean_se"|"block_bootstrap", primitive_version, n, z, se }` —
serialized as an extra JSON key. This is safe against the `analyze.mjs` byte-compat invariant
(`evals.rs:415-421`) precisely because JSON is forward-compatible: the Node consumer reads only the
keys it knows and ignores unknown ones, so the existing pipeline "lights up with zero changes" exactly
as the comment promises. The provenance rides *only* on rows the SE-derived path produced; hand-`tol`
rows carry nothing new. If a RED verdict is ever promoted into the real `MeshEvent` log later (E3
auto-apply territory, gated), those same sidecar bytes are precisely what would go into `payload` —
so designing the sidecar now makes the future event self-explaining for free. This is a real-but-minor
gap the interval-aware constructor should carry; it is not a blocker and does not touch red-line paths.

## Long-Term Consequences, Safety, Scalability

**(a) Scalability — the bootstrap's cost, measured against real consumers.** `bootstrap_interval` is
O(resamples × N) — the standard concern is thousands of resamples over a large sample. The check that
matters is whether any *realistic* consumer in this codebase is large-N. It is not. The one genuinely
large number in the arc — `causal.rs`'s empirical gate at `N ∈ {200, 2 000, 20 000, 200 000}` — uses
the **analytic** `within_clt_envelope` (a single `sqrt` and a compare, O(1)), **never the bootstrap**.
The bootstrap's actual consumers are all small: `aurc` over an eval-set (tens–hundreds of samples), the
`RegressionGate` window (a handful), and the 12-query recall oracle (which uses `wilson_interval`
anyway, not the bootstrap). So at realistic sizes — say 2 000 resamples × a few hundred samples ≈ 10⁶
cheap float ops — this is milliseconds, a non-issue in practice. The honest ceiling to name: the day
someone bootstraps `aurc` over an eval-set of thousands *with* thousands of resamples (≈10⁷–10⁸ ops),
it starts to bite. That is not a current consumer; the guardrail is a one-line doc-comment on
`bootstrap_interval` noting the O(resamples × N) cost and a sane default resample cap, so the cost is
visible at the call site rather than discovered in a slow suite.

**(b) Safety — the misuse risk, named.** The sharp risk is a caller applying the **CLT/normal-approx**
interval (`mean_se` → `normal_interval`, or `within_clt_envelope`) to a sample that is **genuinely
small or non-iid** — most concretely the EMA-smoothed `RegressionGate` stream that §2 already flags as
needing a *different* treatment. Running `mean_se` over the autocorrelated EMA history would badly
*under*-estimate the true error (effective sample size ≪ n) and yield an interval that **looks
rigorous but is confidently wrong** — the self-certification failure wearing a lab coat. **What
actually prevents this today: a documentation convention, not an enforced barrier — state this
honestly.** The blueprint as written ships *separate named free functions* (`normal_interval` vs
`wilson_interval` vs the §2 block-bootstrap) plus the §2 "which-bound-for-which-consumer" table plus
the D8 doc-comment warnings. That is discipline, not enforcement: nothing at the type level stops a
caller from handing a 12-sample or an EMA-smoothed slice to `mean_se`/`normal_interval` and getting a
plausible-looking wrong answer. The *cheapest real hardening*, if this risk is judged worth enforcing,
is a type-level distinction that makes the wrong call hard to write — e.g. returning distinct types
(`CltInterval` for large-n iid vs `WilsonInterval` for small-n binomial) so a consumer cannot silently
treat one as the other, or requiring an explicit `iid`/`n` witness argument the caller must supply
consciously. **The blueprint does not currently mandate that type barrier — so as it stands the
guardrail is a convention (D8) that a hurried caller can bypass.** Named here so the choice is
deliberate, not defaulted.

**(c) Ethics / long-term — the Ananke question.** The recall@5 example shows a real number *changing
meaning* once honestly bounded: `1.0` becomes "1.0, and could be as low as 0.76." The organizational
risk is that the primitive gets *built and then its output ignored* — the interval computed but the
dashboard, report, or memory line still quoting the bare point estimate. Is there a structural
enforcement point, or does honest reporting stay a discipline that can quietly lapse? **The honest
answer: it is mostly convention, with exactly one structural foothold.** By deliberate design (D7,
backward-compatibility), the bare `brier`/`ece`/`aurc` functions survive *and* `RegressionGate`'s
hand-`tol` constructor survives; the interval companions are *additive*. Nothing type-forces a
consumer to consume the interval — a dashboard can keep calling `brier(...)` and print the scalar
forever, and a gate caller can keep passing a hand `tol` and never compute an SE. The interval can be
computed-then-ignored, or never computed at all. **The one place the good outcome fires
structurally** is the recall@5 test (§3 step 4): it *asserts* the Wilson lower bound, so if that bound
regresses the suite goes red — a genuine ratchet, but only for that single consumer, and only inside
the test suite, not in whatever dashboard a human actually reads. **Ananke verdict: the good outcome
does NOT fire structurally in general.** Honest reporting here depends on someone remembering to look
at the interval — precisely the "does the good outcome fire structurally, or does it depend on
someone remembering?" trap. The only thing that would make it structural is removing/deprecating the
bare point-estimate returns (or making the interval the sole return type), which the blueprint
consciously declines for backward-compatibility. That trade is stated, not hidden: **today the
interval is available everywhere and enforced almost nowhere** — flagged so a later owner can decide
whether to convert the recall-test foothold into a broader consumption gate rather than assume the
discipline holds itself.

---

*E2 blueprint. Evidence re-read against the live tree on `feat/spectral-energy-flow-evolution`
2026-07-16 (`causal.rs`, `evals.rs`, `rng.rs`, `csr.rs`, `living_knowledge.rs`, `retrieval/tests.rs`,
`HERMETIC-ARCHITECTURE-PRINCIPLES.md`). Wilson bounds computed, not guessed. No code written or edited.*
