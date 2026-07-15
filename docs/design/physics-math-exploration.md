# physics-math-exploration — kernel growth substrate (P9)

Operating note (2026-07-15): the operator's PRIMARY FOCUS is self-development —
reflection, metacognition, growth of thinking. The kernel (`dowiz-kernel`) is the
rigorous bare-metal substrate for that growth: every exercise must be **Verified-by-Math**
(a hand-derived oracle, a falsifiable RED test, and a GREEN proof), not a float fit.

Research queue (in order; pull the next item when the current is GREEN):

1. ✅ **back-door adjustment / do-operator (Pearl)** — DONE 2026-07-15.
   `kernel/src/causal.rs`. Observational tables → `P(Y | do(X))`; falsifiable: a
   beneficial treatment whose *observational* association is 6.6× overstated by a
   health-conscious confounder; adjustment recovers the true +0.10 causal effect.
2. ✅ **front-door adjustment (Pearl)** — DONE 2026-07-15.
   `kernel/src/causal.rs`. Handles the case back-door *cannot*: confounder `U` of
   `X,Y` is **unobserved**, but `X → M → Y` is the only path and `M` is unconfounded.
   `P(Y|do(X)) = Σ_m P(M|X)·Σ_x' P(Y|M,X=x')·P(X=x')`. Falsifiable: mediator-route test
   (flipping the `Y|M` map changes `do(X=1)` 0.65→0.25 while `do(X=0)` stays 0.45);
   no-X→-M test (effect collapses to exactly 0); trust-boundary RED (malformed P(M|X),
   P(X), oor, empty mediator).
3. ✅ **instrumental variables (Wald estimand)** — DONE 2026-07-15.
   `kernel/src/causal.rs`. The case where no back-door set is observable: a valid
   instrument `Z` (`Z → X → Y`, `Z ⊥ Y` except through X, and Z *shifts* X) recovers
   `P(Y|do(X))` via the Wald estimand `β = (E[Y|Z=1]−E[Y|Z=0])/(P(X=1|Z=1)−P(X=1|Z=0))`
   giving `do(X=1)−do(X=0)=β`. Falsifiable: β matches the hand value 0.375 exactly; a
   weaker instrument changes β (0.375→1.5) so the code is not hard-coded; the naive
   (confounded) `E[Y|X]` is reported alongside and is *larger* than β (U inflates it).
   Trust-boundary RED: an instrument that does not shift X (denominator 0) is rejected,
   out-of-range inputs rejected. Note: Wald assumes a *constant* LATE; the weak-IV case
   yields β>1 for a binary Y, which the math reports faithfully (the known limit, not
   hidden).
4. ✅ **counterfactual inference (twin-network, three-step)** — DONE 2026-07-15.
   `kernel/src/causal.rs`. The deepest Pearl rung: `P(Y_x | X=x', Y=y')` — what Y would
   have been had X=x in the world where we actually saw (x', y'). On a linear SCM
   `X=α·U, Y=β·X+γ·U` it is the three-step: (1) **abduction** recover U=x'/α from the
   observation; (2) **action** set X:=x; (3) **prediction** `Y_x = β·x + γ·(x'/α)`.
   Falsifiable: hand oracle `Y_2=4`, `Y_10` differs per observed unit (12 vs 10) proving it
   uses the *observed* U, not a population mean; a counterfactual (12) ≠ factual (6).
   Trust-boundary RED: an observation the SCM cannot generate (e.g. (4,5) when Y=1.5·X=6)
   is rejected — no silent fake value; α=0 (U unidentifiable from X) rejected; the
   confounding-free case (γ=0) still works and is consistency-checked.
5. ✅ **d-separation oracle** — DONE 2026-07-15.
   `kernel/src/causal.rs`. The structural primitive (Pearl §1.2.4) every adjustment above
   quietly assumes: a graph algorithm deciding `X ⫫ Y | given`. Implemented as an
   **active-trail BFS** over the DAG: chains/forks open iff the node ∉ given; colliders
   open iff the node (or a descendant) ∈ given — so conditioning *blocks* open paths but
   *opens* colliders (Berkson's bias). Falsifiable on the four canonical graphs: chain
   `X→Z→Y` d-connected, blocked by `Z`; fork `Z→X, Z→Y` (the back-door confounder) d-connected,
   blocked by `Z`; collider `X→Z←Y` blocked, *opened* by conditioning on `Z`; and a collider
   with descendant `W` where conditioning on `W` alone also opens the trail. Trust-boundary
   RED: degenerate `x==y` and out-of-range nodes rejected, never panics.

6. ✅ **back-door criterion verifier** — DONE 2026-07-15.
   `kernel/src/causal.rs::backdoor_criterion`. Closes the loop on `backdoor_adjust`: that
   function *assumes* a valid adjustment set; this *proves* a candidate one is valid from the
   DAG alone (no tables). A set `Z` is valid iff (1) no `Z` is a descendant of `X`, and (2) `Z`
   blocks every **back-door** path (path with an arrow into `X`). Implemented by pruning all
   forward edges out of `X` then testing `d_separated(X, Y | Z)` on the pruned graph — any
   surviving path starts with an arrow *into* `X`, i.e. is a back-door path. Verified GREEN
   (confounder `Z` satisfies it; empty set fails when confounded) + RED (degenerate/oob/x/y in
   set rejected).

7. ✅ **front-door criterion verifier** — DONE 2026-07-15.
   `kernel/src/causal.rs::frontdoor_criterion`. Companion to `frontdoor_adjust`: proves the
   graph satisfies the front-door assumptions (Pearl Def 3.4.1) from the DAG alone. A mediator
   set `M` is valid iff (1) it intercepts every directed `X→…→Y` path; (2) no open back-door
   `X..M`; (3) no open back-door `M..Y`. Implemented via directed-reachability + pruning the
   forward edges of `X` and each `M` then `d_separated` under empty conditioning. Verified GREEN
   (`M` on `X→M→Y` passes; a side-branch `M` off `X→Y` fails) + RED (degenerate/oob/empty/x/y-in-M
   rejected).

P9 causal queue COMPLETE (7/7): back-door ✅, front-door ✅, IV ✅, counterfactual ✅,
d-separation ✅, back-door criterion ✅, front-door criterion ✅. The kernel now carries a
coherent causal-inference ladder: **identify** (d-separation / criterion verifiers from a DAG)
→ **adjust** (back-door / front-door / IV) → **infer counterfactually** (twin-network). Next
self-development frontier: do-calculus *rules* over arbitrary graphs, or a full
**structural-identifiability** checker (can this effect be estimated at all, and by which
formula?).

## Done: structural-identifiability checker (ID / IDC, Shpitser–Pearl) — 2026-07-15

- **`kernel/src/cgraph.rs` (NEW)** — `CGraph`, the semi-Markovian graph type with directed
  (`parents`) + bidirected (`bidirected`, latent confounders) arcs. Supports subgraph
  constructions the ID recursion needs: `g_x_removed` (G_X̅), `g_x_incoming_removed` (G_{X bar}),
  `subgraph_on`, `d_separated_underlined` (active-trail BFS with correct bidirected-semantics —
  a direct a↔b confounding arc is open iff its endpoint is NOT conditioned). `c_components`
  returns connected components of the **bidirected subgraph only** (correct c-component def).
  Verified GREEN (confounded pair forms one c-component; d-separation respects Berkson opening)
  + RED (asymmetric bidirected adjacency, oob nodes, reflexive arcs rejected).
- **`kernel/src/causal.rs`** — `id(y, x, g)` (recursive identification, `causaleffect` id.R
  1:1 port) + `idc(y, x, z, g)` (conditional, idc.R). Returns `IdResult::{Identified{formula},
  NotIdentified{hedge}}` — fail-closed: non-identifiable effects return the **hedge witness**
  (F/F′), never a silent wrong value. Verified GREEN: chain / fork(back-door) / front-door /
  front-door-with-latent-mediator all IDENTIFIED; bow-arc M-graph (X→Y + X↔Y) NOT identified
  with a returned hedge; IDC over a mediator collapses to ID by rule 2.
- This closes the named "structural-identifiability" frontier for arbitrary graphs. The
  remaining rung is to make `IdFormula` **numerically evaluable** over observational tables and
  cross-validate it against the 7 proven estimators (the general algorithm must subsume each
  special case — Verified-by-Math).

## Done: back-door adjustment — what was learned

- The *naive* conditional `P(Y|X)` is **not** a causal quantity: it integrates over
  `P(Z|X)`, i.e. it conditions on the open back-door path and thereby *measures* the
  confounder's action on `X`. That is selection on the confounder — spurious association.
- The back-door formula `Σ_z P(Y|X,Z)·P(Z)` is the **do-calculus** expression that an RCT
  (which randomizes X, severing Z→X) would measure. It equals the interventional mean exactly.
- Verified-by-Math, not statistics: caller supplies the conditional + marginals; the module
  performs only the deterministic weighted sum. Correctness pinned by the hand-derived
  confounding example (`do(X=0)=0.45`, `do(X=1)=0.55`; `naive` reports 0.17 / 0.83 — a phantom
  +0.66 vs the true +0.10). Ratios of the gap (>3×) confirm the adjustment removes bias.
- Fail-closed at the trust boundary: every table is validated (shape, `[0,1]` range,
  marginals sum to 1) before any sum; malformed input returns `Err`, never panics.
- Growth takeaway: reasoning about *association vs causation* is exactly the operator's
  metacognition target — and it is executable, falsifiable math on the substrate, not prose.
