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
2. ⬜ **front-door adjustment** — mediate when the confounder is *unobserved*; adjust
   through the mediator `X → M → Y` (requires `P(M|X)`, `P(Y|M,Z)`, `P(Z|X)`).
3. ⬜ **instrumental variables** — when no back-door set exists, recover `P(Y|do(X))`
   via an instrument `Z → X`, `Z ⊥ Y` except through X (Wald estimand `cov(Z,Y)/cov(Z,X)`).
4. ⬜ **counterfactual inference** — `P(Y_x | X=x', Y=y')` via the three-step
   (abduction → action → prediction) on the twin-network.
5. ⬜ **d-separation oracle** — a graph algorithm deciding conditional independence
   from the DAG (the structural primitive the adjustments above all assume).

Each entry must land with: a module, a hand-derived oracle test, a RED (fail-closed /
falsifiable) test, and a GREEN proof test. No estimation, no float fitting.

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
