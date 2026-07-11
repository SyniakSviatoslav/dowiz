# Bebop Field-Sim — Purpose, Usage & Claims Audit (Lens 2 of 2) — 2026-07-11

> Read-only research lens. Companion to `01-implementation-math.md` (sibling lens; numerics deferred
> to it — this doc does not re-audit the math). Question: WHY does the field-sim exist, WHERE is it
> actually wired, and do bebop's claims about it hold — specifically for dowiz's delivery hub.
> Evidence labels: **VERIFIED** (read in code/docs at cited path), **UNVERIFIED** (claimed, not
> reproducible from what's in the repo), **DESIGN-JUDGMENT** (my assessment, reasons given).
> Standing constraint honored throughout: **couriers are NEVER scored/rated** (operator ruling
> 2026-07-11).

---

## 0. One-paragraph verdict

The "field sim" is four Rust modules (`field.rs`, `field_physics.rs`, `geometry_field.rs`,
`wavefield.rs` in `/root/bebop-repo/crates/bebop/src/`) plus an archived TS lineage. **In the only
production-wired path, the physics is decorative**: the live "field gate" is a 5-keyword string
match onto a hardcoded 6-node toy graph whose heat-kernel outputs are constants — extensionally a
lookup table. The heavy physics (gravity/springs/waves/Lyapunov, Platonic-solid tensors, spherical
harmonics) is **orphaned**: its only callers are the benchmark example and internal cross-calls.
Meanwhile the actual dispatch/routing code bebop built (`matcher.rs` → `cost_estimate.rs`) is
**classical k-d filter + BFS + A*/Contraction-Hierarchies — no PDE — by its own explicit design**,
and bebop's own self-checking benchmark asserts the classical hybrid beats the pure wave (exits 1
otherwise). The honest reading: bebop already ran the experiment, the field lost on the tasks that
matter to a delivery hub, and the team (to their credit) wrote that down. What survives is one
plausible niche — diffusion-*ranked* change-impact ("regression radius") for the codebase itself —
and one hard red-line collision (`reputation.rs` explicitly plans courier trust feeding the cost
surface). Verdict for the hub: **research toy, keep parked**; the single candidate application is
dev-tooling, not delivery operations, and it must beat plain transitive-closure blast radius in a
falsifiable test before earning any wiring.

---

## 1. Purpose + real call-path trace: used vs orphaned

### 1.1 What each module says it is (VERIFIED, module headers)

| Module | Self-description | Size |
|---|---|---|
| `field.rs` | "deterministic graph-PDE arbiter (the physics veto)" — host handle to the `rust-core` spectral heat-kernel; + scalar Kalman + limit-cycle loop-health | 15 KB |
| `field_physics.rs` | "fundamental-mass field sim: mass=connections, gravity+springs+waves, Lyapunov gate" | 41 KB |
| `geometry_field.rs` | Platonic solids, Legendre polynomials, spherical harmonics, Nyquist criterion | 18 KB |
| `wavefield.rs` | "geometric + wave sim of the CONNECTION GRAPH (geometry, waves, cycles, divergence)" | 40 KB |

### 1.2 The ONE live end-to-end call path (VERIFIED, traced)

```
bebop CLI multipilot command
  cli.rs:210        Some(|| field_gate(&task))            — closure handed to multipilot
  field.rs:88       field_gate(task) → field_gate_verdict(task)
  field.rs:129-144    node = task.contains("secret"|"auth"|"money"|"migrat"|"rls") → 4
                             .contains("deploy") → 3 | "test" → 2 | "doc" → 5 | else → 1
  field.rs:65-84      plan_csr() — HARDCODED 6-node graph (plan-impl-test-deploy-secrets-docs)
  field.rs:26-60      field_eval → unsafe FFI → rust-core field_build/field_rank
                        (spectral heat kernel exp(-Lt), t=1.0, 20 Chebyshev terms)
  field.rs:153-161    blast_on_secrets > 0.10 ⇒ Override, else Permit; build failure ⇒ Unhealthy
  multipilot.rs:86-92 "override" ⇒ plan blocked: "field arbiter OVERRIDE — physics vetoed the plan"
```

Two more live surfaces (VERIFIED):
- **MCP tools** `field`, `wire`, `loop_health`, `wave_probe` (`mcp.rs:62,84,108,114`). `wire`
  (mcp.rs:511 → `wiring.rs:72`) composes field-gate + L5 stabilizer + forbidden-zone contract +
  target scope + living memory + audit into one fail-closed verdict. `wave_probe`
  (mcp.rs:784 → `wavefield.rs:239`) is agent-loop health telemetry (red-line cycle / divergent hub
  detection), not routing. Live only when an MCP client calls them.
- **`sealfb.rs`** (field energy → self-tightened tolerance) — `seal_tighten` used at `cli.rs:440`,
  `is_stationary` used by `stress.rs:115` and stabilizer tests.

### 1.3 The decisive structural finding (DESIGN-JUDGMENT, from VERIFIED code)

In the live path, **all discriminative work is done by `String::contains` on 5 keyword stems**.
The graph is fixed (6 nodes, 5 edges), so the heat-kernel blast for each of the 5 possible seed
nodes is a compile-time constant. `field_gate_verdict` is extensionally identical to a 5-entry
lookup table `{secrets→Override, deploy/test/doc/impl→Permit}` — the test at `field.rs:288-300`
even documents the constants (~0.66 vs ~0.06 mass). The physics adds auditability theater, not
information. The genuinely good engineering here is the **fail-closed contract** (malformed CSR /
degraded sim ⇒ `Unhealthy` ⇒ refuse, never permit — `field.rs:303-329`), which is real safety
design and is VERIFIED tested — but it does not require a PDE.

### 1.4 What is ORPHANED (VERIFIED by repo-wide grep)

- `field_physics::{simulate, build_bodies, change_impact, wave_bounce_path, field_stable}` — only
  callers: `examples/tree_vs_field_telemetry.rs` and one internal call from `wavefield.rs:585`.
  **Zero production callers.**
- `geometry_field::{spherical_harmonic, node_harmonic_field, legendre, Platonic, nyquist_unstable}`
  — only internal use by field_physics/wavefield. Zero external callers.
- `wavefield::plan_wave_gate` — **zero callers anywhere** outside its own module.
- **`matcher.rs` itself** (`match_orders`, `fingerprint`, `MatcherClient`, remote transport) — the
  "open decentralized dispatch core" — has **zero callers** outside its own tests. Not wired into
  CLI, MCP, TUI, or any binary. Same for `cost_estimate::hybrid_route` (only the orphaned matcher
  calls it). These are protocol *libraries* awaiting a node runtime — consistent with the
  local-first synthesis finding that "the NODE (wire + storage + runtime) is 0 lines"
  (`/root/dowiz/docs/design/local-first-hub-2026-07-11/SYNTHESIS.md` §2).
- The entire TS field stack the benchmark reports describe (`field-sim.ts`, `field-rust.ts`,
  field-planner, Final Arbiter) now lives in `archive/bebop-ts-src/integration/` — **archived**.

### 1.5 Where field types ARE reused — but the physics is not

`matcher.rs`, `cost_estimate.rs`, `mapping.rs` import `wavefield::{Node2D, ConnEdge, LinkKind}` as
**plain data structures** (VERIFIED, `matcher.rs:27`, `cost_estimate.rs:26`, `mapping.rs:19`). The
routing engine's own header is explicit that the wave is abandoned as an algorithm:
`cost_estimate.rs:14-17`: *"A damped wavefront … IS the Fast Marching Method / Eikonal equation
(Tsitsiklis 1995 …) So we use A*/Dijkstra with W_uv = 1/F_uv — **NO PDE solver**."* That claim is
mathematically sound — Tsitsiklis (1995) established the Dijkstra-like optimal-control solution of
the Eikonal equation, and on a *graph* (not a continuum) Dijkstra is exact, so a wave solver can
only ever reproduce it slower ([Fast marching method — Wikipedia](https://en.wikipedia.org/wiki/Fast_marching_method),
[arXiv:2603.11830](https://arxiv.org/html/2603.11830)). **VERIFIED as sound engineering; the field
sim's own successor code renounces the field.**

One more over-claim inside the matcher (VERIFIED, `matcher.rs:74-93`): despite its header ("which
courier gets which order"), `match_orders` **never selects among couriers** — each `Order{src,dst}`
arrives with its courier pre-fixed as `src`, and the "assignment" is just a route check
(`courier: o.src`). No assignment problem is solved. The industry-standard formulation (min-cost
bipartite matching / Hungarian over batches, e.g. [FoodMatch, ACM TSAS 2022](https://dl.acm.org/doi/10.1145/3494530),
[MDPI Smart Cities 2024](https://www.mdpi.com/2624-6511/7/3/47)) is absent.

---

## 2. Benchmark honesty audit

Artifacts: `crates/bebop/examples/tree_vs_field_telemetry.rs`,
`docs/design/field-vs-kdtree-scale-report-2026-07-09.md`,
`docs/design/field-sim-comparison-2026-07-09.md`, `docs/wiki/Field-Sim-Comparison.md`,
`docs/diagrams/field-sim-explainer.svg`.

### 2.1 What is honest (VERIFIED)

- **The Rust example is self-falsifying and anti-field.** `tree_vs_field_telemetry.rs:350-359`
  asserts `CH_wins = v_ch < v_un && v_ch < v_wv` and **exits 1 if the Contraction-Hierarchy A*
  does not beat both the uncontracted A* and the pure wave** on cost search. Its printed verdict:
  "wvMs >> chMs proves the layered hybrid beats the pure wave." This is the repo *disproving its
  own field-as-router thesis*, in CI-runnable form. Rare and creditable.
- **Explicit honesty notes**: "the k-d tree column is a *different operation* … shown only as a
  reference floor, never as a 'we beat it' scorecard" (`field-sim-comparison` §7); the SIMD figure
  is reported as a measured 1.08×, not inflated; the n≈2000 wasm memory ceiling is disclosed, not
  hidden (`field-vs-kdtree-scale-report` §1 note).
- CH being the right fix is consistent with the literature: CH queries run in ~1 ms on
  continental road networks ([Geisberger et al. 2008](https://link.springer.com/chapter/10.1007/978-3-540-68552-4_24));
  OSRM/GraphHopper standard practice.

### 2.2 What is NOT honest, or over-claimed

1. **Circular recall metric** (`field-vs-kdtree-scale-report` §2). recall@k is measured "vs the
   field's **own** ground-truth affected set." The field trivially scores 1.0 on its own output;
   k-d "recall degrading 0.20→0.10" against another algorithm's output is meaningless. There is no
   independent ground truth (real repo change history, real incident blast radius) anywhere in the
   harness. **UNVERIFIED capability claim built on a rigged metric.**
2. **The headline speedups are language comparisons, not method wins.** The 10.9–26.8× (report §1),
   26.8×/64–73× (wiki), and 16–73× ("squarely the 100× class the operator targeted",
   `bebop-rust-field-core` §Telemetry) all compare *Rust/WASM vs JavaScript running the same
   algorithm*. Legit engineering telemetry; over-reach the moment it's cited as evidence the field
   *method* wins anything. The wiki page ("**The unique feature of Bebop 0.4.0**") quotes these
   numbers without that caveat. **DESIGN-JUDGMENT: favorable framing.**
3. **The strawman baseline.** The "topology-respecting" validation (example lines 361-422) shows
   the wave doesn't leak to a graph-disconnected node while Euclidean k-NN would. But plain BFS —
   present in the same file as `wave_bounce_path`, O(N+E) — has the identical property with no
   physics. The one baseline that would isolate the wave's *unique* contribution (BFS/transitive
   closure with decay weighting) is never scored against it. Standard change-impact practice is
   exactly dependency-graph reachability / transitive closure ("blast radius") —
   [Axiom Refract](https://axiomrefract.com/learn/what-is-blast-radius),
   [Endor Labs](https://www.endorlabs.com/learn/vulnerability-blast-radius-how-to-measure-and-reduce-impact),
   [arXiv cs/9902008](https://arxiv.org/pdf/cs/9902008). **DESIGN-JUDGMENT: the benchmark never
   tests the field against its real competitor.**
4. **Synthetic-only data.** All graphs are deterministic spirals/rings with pseudo-random chords
   (`synth_repo`, `tree_vs_field_telemetry.rs:36-75`). No real repo import graph, no real road
   network, no real order stream. **VERIFIED.**
5. **Reproduction is broken.** Both reports' reproduce commands (`npx tsx src/integration/…`)
   point at paths that now live in `archive/bebop-ts-src/` — the documented benchmarks can no
   longer be run as written. **VERIFIED.**

Net: the *Rust example* is honest and concludes against the field; the *markdown reports and wiki*
present a capability story ("uniquely answers change-impact", "strictly more capable and 11–27×
cheaper") that rests on a circular metric, a strawman baseline, and cross-language speedups.

---

## 3. Theory docs — grounded vs over-reach, itemized

| Doc | Verdict | Reasons |
|---|---|---|
| `bebop-tensor-field-theory-2026-07-09.md` | **Grounded engineering, benefit unproven** | Heat diffusion on the graph Laplacian as rigorous spreading-activation is standard graph signal processing; the doc's own three corrections (not Newtonian; explicit Euler injects energy → velocity-Verlet; "minimal latency" only per-step) are textbook-correct and self-critical. What it never shows: that wave propagation answers any *task* better than reachability. Plausible-but-unproven. |
| `bebop-math-physics-fable-research-2026-07-11.md` | **Grounded — it is itself the prosecution** | This internal audit REJECTS the numerology explicitly: fabricated fractional-derivative identity ("REJECT"), Emden "demand black holes", redshift "trust decay", vorticity "courier loops", Noether/Fock/Catalan — all labeled "POETRY (analogy only, must not be cited as implemented physics)". Its keep-list (Fiedler λ₂, Chebyshev spectral, Kalman, Lyapunov, Cauchy–Schwarz) is legitimate applied math. Adopt its verdicts wholesale. |
| `bebop-rust-field-core-2026-07-09.md` | **Mixed** | The engineering (Chebyshev approx of exp(−Lt) — standard technique, [Hammond-line GSP literature](https://arxiv.org/pdf/1105.1891), [IEEE 5982158](https://ieeexplore.ieee.org/document/5982158/); mass-conservation KATs; memory discipline) is real and measured. Over-reach: the "Final Arbiter" (fieldCost vs pddlCost with a tunable `mismatchRatio`) is presented as physics adjudicating a planner, but no evidence anywhere connects heat-kernel mass to real plan failure; the dial is arbitrary. "Squarely the 100× class" = language-comparison inflation (§2.2 item 2). The whole arbiter stack is now archived TS anyway. |
| `bebop-research-synthesis-FIELD-DSPARK-PYDANTIC-2026-07-08.md` §0 | **Numerology dressed as physics** | Computing discrete ∇·F/∇×F over the *visit order of a similarity-ranked candidate list* and calling the result "the basic law … a fundamental physical improvement to the preciseness of reasoning" is metaphor presented as mechanism. The ranked list is not a vector field; nothing conserved, no dynamics. The div/curl arithmetic is fine; the epistemic claim is unsupported. (Flag-OFF by default — even the authors didn't trust it enough to wire it on.) |
| `bebop2/ARCHITECTURE.md` (vectors→waves, better-math-per-function) | **Grounded doctrine + rhetorical inflation** | Spectral/Krylov/Lanczos formulations, square-root Kalman, storing local operators by spectrum — legitimate numerical-methods guidance with correct citations of the right bar. Inflation: "a dense O(n²) tensor is a CRIME"; the doctrine is scale-blind — for delivery-hub-sized graphs (dozens of nodes per venue) dense/naive is optimal by simplicity. The AGC framing is aesthetic. The honest boundary ("algorithmic work is NOT overhead", don't 'optimize' PQ crypto) is genuinely good. |
| `docs/design/cycle-consistency-theorem.md` | **Grounded** | Correct linear algebra (PCA truncation error = discarded spectral tail, orthonormality/Parseval), and — unusually — a *proved blind spot* (gap=0 ⇏ correctness, with a constructive counterexample) plus "NECESSARY-not-SUFFICIENT" and shadow-first deployment. §4's fault-localization "theorem" holds only for single-coordinate injections dominating tail leakage (stated, roughly). Not field-sim per se, but the best exhibit of the repo's honest mode. |
| `geometry_field.rs` Platonic solids / spherical harmonics as node state | **Numerology in code** | Node i gets a Platonic solid by `i % 5` and carries a wave tensor over its vertices (`tree_vs_field_telemetry.rs:65-73`, `field_physics.rs`). No justification exists anywhere for why a file or courier's state should live on icosahedron vertices; no benchmark isolates any benefit of V-dim tensors over scalars. Sacred-geometry aesthetics. **DESIGN-JUDGMENT.** |

Pattern across the corpus: bebop's *self-audits are honest* (they repeatedly catch and label their
own poetry) while the *promotional surfaces* (wiki, "unique feature" claims, operator-directive
docs) re-import the poetry. The distinction between the two registers is the key to reading this
repo.

---

## 4. Relevance to the dowiz hub — verdict

### 4.1 Against dowiz's actual problems (VERIFIED against the hub review)

- **Courier dispatch.** Live incumbent: `attemptHonestDispatch` — freshest-heartbeat available
  on-shift courier; no courier ⇒ order does not advance (the F1 no-trap red line) — plus a durable
  redispatch journal and sweep workers (`2026-07-11-hub-architecture-review.md` §4.2). At venue
  scale (a handful of own couriers per location) assignment is trivial; the *standard* upgrade
  path, if ever needed, is min-cost bipartite matching on ETA (Hungarian — [ACM TSAS FoodMatch](https://dl.acm.org/doi/10.1145/3494530)),
  which is deterministic, auditable, and needs no physics. The field-sim offers nothing the
  incumbent lacks; the hub's real dispatch gap is **courier push notifications**, a product gap no
  simulation touches (§4.6 item 1).
- **Routing.** Bebop's own benchmark rules: A*/CH beats the wave (§2.1). For real streets dowiz
  already consumes ORS polylines with haversine degrade (§4.5). The field loses to the incumbent
  by bebop's own exit-1 assertion.
- **RED-LINE COLLISION (flag hard).** `reputation.rs:1-16` (VERIFIED): "*trust feeds the cost
  surface — high-trust couriers are preferred, low/unknown trust costs more (risk premium),
  suspended = unreachable.*" This is courier scoring wired into the field/cost engine by design —
  **blocked outright by the standing NO-COURIER-SCORING ruling** (also enforced in dowiz by a
  red-on-disk invariant test, hub review §4.3, and already flagged as a doctrine collision in
  `SYNTHESIS.md` §7). Any hub adoption of `EdgeCost.risk` or field per-node "sensitivity" where
  nodes represent couriers is the same violation with extra steps. Geometry/ETA-only costs are the
  compliant alternative.

### 4.2 What the field-sim is genuinely good for

(a) **Fail-closed gating architecture** (the contract, not the PDE): keyword/graph-mapped veto that
refuses on degradation — worth *imitating* in any hub gate; requires zero physics. (b) **Diffusion-
ranked change-impact**: `exp(−Lt)` over a dependency graph gives a *decaying magnitude ranking* of
affected nodes, which plain transitive closure does not — the one output with no trivial classical
equal (BFS gives the set; diffusion ranks it). (c) Deterministic, no-RNG, air-gapped discipline
with falsifiable KATs — a genuinely good engineering culture artifact.

### 4.3 What is over-claimed

"Physics veto" (a keyword table); "uniquely answers change-impact" (circular metric, no independent
ground truth, strawman baseline); 11–73× "wins" (Rust-vs-JS, same algorithm); the matcher as
courier assignment (it never selects a courier); ∇·F/∇×F as a "fundamental law" of reasoning;
Platonic/spherical-harmonic node tensors (unjustified); "the cost surface the planner reads" (that
planner stack is archived TS).

### 4.4 Earned place in the local-first hub?

**No — park it.** Nothing in the delivery-operations path (dispatch, routing, order lifecycle) is
improved by the field over the deterministic incumbents, by bebop's own measurements; the one
designed integration (reputation→cost) is red-line-blocked; and the local-first ladder
(`SYNTHESIS.md` §5) correctly gates all bebop adoption behind `kernel::decide` honesty and the
first real order. The field-sim should not appear on any P0–P5 rung.

### 4.5 The single highest-value application + its falsifiable test

**Application (dev-tooling, not delivery ops): regression-radius ranking for the dowiz codebase.**
dowiz's Task-Exit rule already demands a "regression radius" estimate per task
(`/root/dowiz/.claude/CLAUDE.md`). Today that is judgment. The field's heat kernel over the import
graph could rank files by predicted impact of a diff — the one task where diffusion's decaying
magnitudes add something over the incumbent (plain transitive-closure blast radius, the industry
standard).

**Falsifiable test (RED case included, VbM-compliant):**
1. Corpus: last N≥100 merged dowiz PRs. For each, seed = files changed; *ground truth* = files
   touched by follow-up fixes/reverts within 14 days ∪ files of tests that failed in CI on that PR
   (independent of any method — kills the circularity of §2.2 item 1).
2. Contenders on the same import graph: (A) transitive closure (incumbent), (B) git co-change
   frequency baseline, (C) heat-kernel `exp(−Lt)` top-k ranking.
3. Metric: recall@20 and MAP against ground truth. **GREEN bar**: C beats A by ≥10 points recall@20
   at p<0.05 (paired bootstrap). **RED case**: on a synthetic corpus where ground truth IS the
   1-hop neighborhood, C must NOT beat A (if it "wins" there, the harness is broken).
4. If C loses or ties: the field-sim stays parked permanently, with this document as the tombstone.

---

*Sources (web verification):* [Fast marching method / Tsitsiklis 1995](https://en.wikipedia.org/wiki/Fast_marching_method) ·
[Eikonal optimal trajectories](https://arxiv.org/html/2603.11830) ·
[Geisberger et al., Contraction Hierarchies 2008](https://link.springer.com/chapter/10.1007/978-3-540-68552-4_24) ·
[Chebyshev approximation for graph operators](https://arxiv.org/pdf/1105.1891) ·
[IEEE 5982158](https://ieeexplore.ieee.org/document/5982158/) ·
[FoodMatch: batching+matching for food delivery](https://dl.acm.org/doi/10.1145/3494530) ·
[Hungarian-algorithm delivery assignment](https://www.mdpi.com/2624-6511/7/3/47) ·
[Blast-radius / change-impact practice](https://axiomrefract.com/learn/what-is-blast-radius) ·
[Endor Labs blast radius](https://www.endorlabs.com/learn/vulnerability-blast-radius-how-to-measure-and-reduce-impact) ·
[OO change impact via transitive closure](https://arxiv.org/pdf/cs/9902008)

*Repo evidence paths are absolute under `/root/bebop-repo/` and `/root/dowiz/` as cited inline.
Both repos left as found; no code touched.*
