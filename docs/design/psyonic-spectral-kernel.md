# Psyonic Spectral Kernel — Architecture & Evolved Roadmap

> Status: **v2** (2026-07-14). Synthesised from four codebase research lanes (kernel, loop
> detector, engine, design corpus) + the Fable math/physics-primary pass (188k tokens, 31 tool
> uses, evidence-verified). Governing artifact — every future iteration serves this. Written as
> architecture; the roadmap is licensed to **self-modify existing organs** and **grow new ones**,
> under the discipline in §9. Every claim marked **PROVEN** / **PROVABLE** / **CONJECTURE**.

## 0. Thesis (one sentence)

**Proven equations on bare metal, grounded at the edges by thin bridges, each collapsing a legacy
sprawl to its minimal form — and the one invariant that governs all of it is eigenvalue position
relative to unity, with the spectral gap `γ = 1 − |λ₂|` as the master dial for both stability and
iteration speed.**

## 1. Anatomy (mind ⟷ body)

| Part | Is | Rule |
|---|---|---|
| **Body** | the bare-metal Rust kernel | equations *run* here — zero abstraction distance between formulation and act |
| **Skeleton** | red-lines: integer money, the FSM Law, auth/RLS/PQ-crypto | rigid, load-bearing; the field **never** deforms it 🔴 |
| **Mind (psyonic-spectral)** | spectral operators — cognition-as-dynamics, `|λ|` vs 1 | metacognition = the spectrum of the system's own behaviour |
| **Senses** | thin code bridges (I/O, net, DB, WASM, external APIs, LLM judge) | the only external ground (§4); stays code |
| **Metabolism** | the reverse-engineering loop | converts legacy tissue → proven kernel-math tissue |

Not metaphor where it counts: dynamical-systems theory models "what a system does" as eigenvalue
position — `|λ|<1` converges, `=1` oscillates, `>1` diverges. The attractor detector literally
computes the spectrum of the system's own tool-stream: **self-awareness as an eigenvalue computation.**

## 2. The one invariant

`|λ|` vs the unit circle is the whole cognitive readout — **Damped** `<1` / **Resonant** `≈1` (limit
cycle, e.g. `μ≈−1` period-2) / **Unstable** `>1`. And `γ = 1 − |λ₂|` is dual-purpose: **mixing time**
`τ ≈ 1/γ` (how trapped) *and* **iteration budget** `k ≈ ln(1/tol)/ln(1/|λ₂|)` (steps a method needs).
*(Built + PROVEN: `kernel/src/spectral.rs`, 9/9 VbM.)*

## 3. The governing boundary (ASCEND / BRIDGE)

- **BRIDGE** — code, only at external edges (I/O, net, DB, WASM/host, adapters, LLM judge). Minimal.
  These are the DPI grounding anchors. Do **not** ascend them.
- **ASCEND** — all internal logic → a proven kernel equation/operator. The roadmap is ASCEND-only,
  each item naming the bridge that grounds it.
- **Why exact, not compromise (PROVEN, corpus §DPI):** internal reflection is a Markov chain, so
  `I(ground; stateₜ)` is a **supermartingale — meaning leaks monotonically**, refilled only by an
  external term `Gₜ` (retrieval/tool/verify/human). The bridges *are* the `Gₜ` injection points. An
  ASCEND item with no grounding bridge is a red flag (→ mesh-λ₂ is quarantined, §6).
- **Math is the authority:** an equation enters the kernel only PROVEN — derivation, theorem, or a
  falsifiable VbM RED test. Never prose.
- **Self-simplifying (§7):** each rewrite reduces to its one essential equation; the anti-pattern is
  compressing by silently dropping the empirical residue — admissible only as *H↓ ∧ claim-set equal*.

## 4. Current organs (built + proven) — and one live hazard

- **`kernel/src/spectral.rs`** — general eigensolver (Faddeev-LeVerrier → Durand-Kerner), spectral gap
  γ, Laplacian Fiedler λ₂, DMD drift class. 9/9 green, cross-checked vs `ρ`. PROVEN.
- **`kernel/src/order_machine.rs`** — ρ, μ, Kahn topo, reachability, `fsm_graph_report`,
  `FSM_GOLDEN_SIGNATURE` gate *(dead until runtime-wired — R4)*. PROVEN.
- 🔥 **DUAL-AUTHORITY HAZARD:** the FL+DK eigen-core now exists **twice** — `spectral.rs:103-189`
  (Rust) and `markov_attractor.py:141-189` (Python) — with **no parity gate**. This is the exact
  silent-drift risk the kernel exists to kill. Resolving it is **R1**.

## 5. Evolved roadmap (ASCEND only; ordered by compression × proof-strength)

Each: `{ equation · GROW|SELF-MOD · proof-class + RED-test · legacy collapsed · grounding bridge · red-line }`.

**R1 ⭐ · SELF-MOD — Markov attractor detector → `kernel::markov`** *(front-runner, §8)*
Port `markov_attractor.py` (287 LOC) to Rust reusing `spectral.rs`; add continuous `gap()`,
`mixing_time()≈1/γ`, `budget(tol)=⌈ln(1/tol)/ln(1/slem)⌉`; ~30-LOC CLI `[[bin]]` bridge so hooks call
one native binary.
· eq: `verdict = Φ(π, H, spec(Â))`, upgraded with the continuous dial `γ = 1 − slem(Â′)`.
· PROVABLE→PROVEN: analytic spectra + frozen parity vs the 12-assertion Python corpus + damping
  theorem `|λ₂(Â′)| ≤ 1−d` (Haveliwala–Kamvar) ⟹ `γ ≥ d`; RED = FL sign-flip + the μ≈−1 case.
· collapses: 287 LOC Python + ~120 LOC duplicated eigen-core → ~100 LOC Rust; kills dual authority;
  removes Python from the per-tool-call path (133 ms → one native exec); binary tripwire → measured γ.
· bridge: `transcript_events.py` tokenizer + hook shells (kept — the DPI ground). · red-line: none.

**R2 · SELF-MOD — Closed-form spring/modal motion → analytic `Spring::at(t)`**
· eq (ζ=1, PROVEN by substitution): **`y(t) = (y₀ + (v₀+ωy₀)·t)·e^(−ωt)`**; overshoot ⟺ `v₀ < −ωy₀`.
· collapses: `motion.rs` integrator + `loop_.rs` dt≤0.02 corridor + substep cap + the `DT_STABLE`
  cross-crate pin — all artifacts of *discretising a solvable ODE* — into 1 equation (3 ζ-branches).
  Spring channels become dt-free/exact at any frame rate. Modal extension (§6e) makes the whole field
  analytic between layout changes.
· RED = the `v₀ < −ωy₀` boundary. · bridge: rAF/DOM thin-shell. · red-line: none (money-never-a-field
  boundary `money_guard.rs` untouched + re-asserted).

**R3 · GROW — Absorbing-chain funnel → `kernel::analytics`**
· eq (PROVEN this session, exact): the lifecycle is an absorbing chain; transient `Q` is nilpotent
  (`Q⁵=0`, DAG) ⟹ **`N = I+Q+Q²+Q³+Q⁴`** (no inversion), **`t = N·1`**, **`B = N·R`**. Verified:
  `t = [1.9167, 2.75, 2.5, 1.5, 1.0]`, `B` rows sum to 1.
· collapses: the "zero-readers" measurement gap + a would-be dashboard genre (~200-LOC/specimen) → 2
  equations (~60 LOC). Outputs today's unanswerable numbers: `P(Delivered|channel)`, expected-steps.
· guard/RED: verify `Q^{|T|}=0` before the finite sum; a Reopen edge breaks nilpotency ⟹ **refuse**
  (wired to the golden-signature gate). · bridge: order-events ingestion. · red-line: none (measurement
  only — **any pricing/money coupling is OUT-OF-SCOPE** 🔴).

**R4 · SELF-MOD — Golden-signature generator-pin** (A7): pin `GOLDEN_ADJ:[u16;10]`, *derive* the 8
report fields (each a theorem about A); keep field-agreement tests. MDL cleanup, PROVEN. · bridge: none
(pure structure).

**R5 · SELF-MOD — bench verdict → reuse `DriftClass`** (A4): `class = band(c/b − 1, θ)`; 113 → ~15 LOC.
**R6 · GROW — `analyze.mjs` flake/regression stats → `kernel::stats`** (switching-rate = 2-state Markov).
**R7 · GROW — Fiedler zone partition** (`geo.rs`): `cut = sign(v₂)`, conductance `≤ √(2λ₂)` (Cheeger,
PROVABLE). **Needs a new `spectral::jacobi_sym` (eigen*vectors* — charpoly gives values only).**
**R8 · GROW — DMD/AR(1) drift fit** for loop-signals (`μ̂=Σx_{k+1}xₖ/Σxₖ²`); heed the corpus de-bias
`μ_fb=√(μ_f·μ_b)`. Feeds the hydraulic loop. PROVABLE.

## 6. Quarantined — do NOT build

- **mesh-λ₂ (bebop)** — **CONJECTURE, no grounding bridge.** bebop convergence is a **CRDT lattice-join
  fixpoint** (pull anti-entropy + Merkle equality), proven by exact set algebra; there is **no peer
  topology to run λ₂ on**. A spectral layer could only predict *latency* given a contact graph that
  doesn't exist. Revisit only if a who-pulled-from-whom bridge is added.
- **L5 semantic-entropy machinery** — advisory-only by DPI; internal-only signals are meaningless.
- **money / auth / RLS / migrations / `money.ts` deletion** — 🔴 red-line, operator-gated.

## 7. Self-simplifying equations — compression × proof (ranked)

| Rank | Item | Essential form (the whole thing) | Compression | Proof |
|---|---|---|---|---|
| 1 | R3 Funnel | `t=N·1, B=N·R, N=I+Q+Q²+Q³+Q⁴` (exact: DAG ⟹ Q⁵=0) | ~200-LOC dashboard genre → 2 eqs (~60 LOC) | **MAX** |
| 2 | R1 Markov | "last-40 outcomes are a chain Â; trapped ⟺ escape≤0.05 ∧ failure; cyclic ⟺ H≤0.5 ∨ ∃λ:|λ|≥.85∧|argλ|≥.6; budget=ln(1/tol)/(1−slem)" | 287 LOC Py + 120 dup eigen → ~100 LOC Rust, −1 language on hot path | **HIGH** |
| 3 | R2 Spring | `y(t)=(y₀+(v₀+ωy₀)t)e^(−ωt)` | ~90 LOC integrator+corridor+2 pins → 1 eq | **HIGH** |
| 4 | R4 Golden | "the lifecycle IS the adjacency A; every field is a theorem about A" | 12-line pinned struct → 10 ints + derivations | **HIGH** |

MDL, stated honestly: `K(x)` is machine-independent up to O(1) (**PROVEN**) but **uncomputable**
(**PROVEN**) — "shortest description" is a *direction*, optimised as MDL `min L(H)+L(D|H)`. The equation
is `H`; thresholds/business-rules/presets are the irreducible residue `L(D|H)` — keep it explicit and
small, never silently dropped (that violates the `H↓ ∧ claims-preserved` pincer).

## 8. Highest-leverage next iteration — R1, and why

It is the **only legacy math on the agent's own hot path** (every tool call); it is a **live
dual-authority hazard** (identical FL+DK in Python *and* `spectral.rs`, no parity gate); its proof is
the cheapest of the top three (the 12-case Python corpus is a ready-made frozen oracle, and
`spectral.rs` already passed its analytic fixtures); and it unlocks the continuous γ→τ→budget dial that
upgrades the self-improvement loop from a binary tripwire to a measured control signal — the master-dial
thesis made operational where it observably pays first. Touches no red-line.

## 9. Verification doctrine (all five layers required)

1. **Analytic ground truth** — diagonal / cycle Cₙ (roots of unity) / path Laplacian `2−2cos(kπ/n)` /
   nilpotent (all-zero) / stochastic (`λ=1`) / damped (`|λ₂|≤1−d`). Tight 1e-6 on distinct roots.
2. **Legacy-oracle parity** — freeze the legacy test corpus as shared JSON; kernel + legacy must emit
   identical claims in CI until the legacy is deleted.
3. **Property/invariant** — `Σλ=tr`, `Πλ=(−1)ⁿc₀`, Gershgorin, `B` rows sum to 1, `(I−Q)N=I`.
4. **Determinism** — native x86_64 vs wasm32, bitwise on basic-ops paths; **shrink the libm surface**
   (replace `Complex::abs`'s `hypot` with `√(re²+im²)` — an immediate `spectral.rs` fix).
5. **RED (falsifiability)** — a mutation that must fail (FL sign-flip breaks P₃; μ≈−1 into a symmetric
   solver; Reopen edge ⟹ refuse; `v₀<−ωy₀` ⟹ overshoot).

## 10. Corrective findings (fold into the affected work)

- **Field-UI damping (corrects the corpus):** scalar `γ` critically-damps *at most one* mode
  (`ζ_k ∝ 1/√λ_k`). All-mode `ζ_k=1` requires **operator damping `Γ = 2c·L^{1/2}`** — PROVEN by the
  corpus's own modal dictionary; the per-widget tuning problem collapses into one operator identity.
- **`motion.rs:87` `heat_kernel_delay`** is ballistic (`t=d/c`), not diffusive (`t∝d²/α`) — misnomer.
- **Four eigensolvers** across the two repos (Py FL+DK, Rust FL+DK, bebop Jacobi, bebop QR) —
  consolidation is itself a rewrite target; R1 removes one, `jacobi_sym` (R7) should absorb bebop's.
- **Eigenvalue position is necessary, not sufficient** — non-normal transient growth (pseudospectra);
  any drift verdict on an *estimated* operator stays advisory + carries a `‖J‖₂` caveat. Verdicts on
  *exact structural* operators (FSM adjacency) may gate.

## 11. Growth discipline

Every iteration: **research → reverse-engineer → prove (VbM red→green) → admit (the deterministic
`verify-self-mod` classifier: no regression on entropy/benchmark/floor) → land (kernel via cargo;
hooks via operator `!`).** Three invariants never yield: (1) the mind never overrides the skeleton
(no money/auth/RLS/migrations); (2) math is the authority; (3) governance self-mod stays
human-`!`-activated — the agent designs/proves, the operator lands.
