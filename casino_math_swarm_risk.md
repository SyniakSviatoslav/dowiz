# Casino Math for Agent-Swarm Risk & Dynamic Lane Sizing
*Precise equations + sources. Notation: q = 1 − p unless stated.*

## (1) Expected Value — swarm completion
Discrete general form:
```
EV = Σ_i  p_i · x_i
```
N independent executors ("lanes"), each with success prob p, success value v, per-lane cost c (failure value 0):
```
EV_lane   = p·v  − (1−p)·c
EV_swarm  = N·(p·v − (1−p)·c) = N·p·v − N·(1−p)·c
```
Probability the swarm completes ≥1 task (at least one lane succeeds), value V on first/any success:
```
P(≥1 success) = 1 − (1−p)^N
EV_completion ≈ V · [1 − (1−p)^N]
```
*Source: standard probability / expectation definition (e.g. Wikipedia "Expected value").*

## (2) Kelly fraction — how many lanes to commit
For a bet that pays **b:1 net odds** (stake 1, win returns b profit, lose stake), edge p, q = 1−p:
```
f* = (b·p − q) / b = p − q/b
```
`f*` = growth-optimal fraction of bankroll (token/time budget B) to stake. Map odds to lanes:
committing one lane costs stake s and yields net value (v−s), so b = (v−s)/s; then growth-optimal parallel width:
```
L* = f* · (B / s) = (p − q/b) · (B / s)
```
Use ½·Kelly (L*/2) for a safety margin. *Source: Kelly, J.L. (1956), "A New Interpretation of Information Rate," Bell Syst. Tech. J. 35(4):917–926, DOI 10.1002/j.1538-7305.1956.tb03809.x.*

## (3) Gambler's ruin — probability the swarm blows its budget
Random walk, win p / lose q, absorbing barriers 0 (broke) and N (target), starting at k:
```
P_ruin = [ (q/p)^k − (q/p)^N ] / [ 1 − (q/p)^N ] ,   p ≠ q
```
Special cases:
- p > q, **no upper bound** → `P_ruin = (q/p)^k`  (the form in the brief).
- p ≤ q (unfavorable or fair) → `P_ruin = 1` (certain ruin).

Token-budget view: budget B = k·s units, each failed lane = one step toward 0; per-lane failure prob q_lane, success prob p_lane=1−q_lane. Then P(swarm depletes budget) ≈ (q_lane/p_lane)^(B/s) when p_lane>q_lane.
*Source: Dubins, L.E. & Savage, L.J. (1965), How to Gamble If You Must: Inequalities for Stochastic Processes, McGraw-Hill (Dover ed. 1976); classic result due to de Moivre (1711).*

## (4) Budget concentration bounds — total token spend
Let X = total spend, μ = E[X], σ² = Var(X).
**Chebyshev** (per-lane variance σ², n i.i.d. lanes, mean spend per lane μ):
```
P(|X̄ − μ| ≥ ε) ≤ σ² / (n · ε²)        (sample average X̄)
P(|X − μ| ≥ ε)    ≤ Var(X) / ε²        (total X)
```
**Chernoff** (multiplicative, independent/bounded lanes), δ>0:
```
P( X ≥ (1+δ)·μ ) ≤ exp( −δ²·μ / (2+δ) )
```
⇒ probability of overspending beyond (1+δ)·budget decays exponentially in budget μ.
*Sources: Bienaymé–Chebyshev inequality (Chebyshev 1867); Chernoff, H. (1952), "A Measure of Asymptotic Efficiency for Tests of a Hypothesis Based on the Sum of Observations," Ann. Math. Stat. 23(4):493–507.*

---
**Verification note:** Web fetch/search was unavailable in this session, so DOIs/URLs are cited from canonical references but **unverified live** — confirm the DOIs above before external publication. The equations themselves are standard and reproduced faithfully.
