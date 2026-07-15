# Deliberative Governance Layer — Design (Researcher #1 of 3)

Grounded in kernel casino math (`ev`, `kelly_fraction`, `ruin_prob`, `chernoff_token_tail`,
`Recalibrator`/EMA per `casino_math_swarm_risk.md` + `swarm_math_backbone.md`), the
`swarm_exec` orchestrator in `tools/telemetry/topics.sh`, and the DECART rule
(`docs/operating-model/integration-decart-rule.md`).

## A. EV-Driven Multi-Layer Tiering

Replace "always cheapest executor" with a **measured track-record store** keyed by (model × task_type).

**JSONL `logs/tier_track.jsonl` — one line per completed task slot:**
```json
{"ts":"2026-..T..Z","model":"qwen3-30b-a3b-q4","task_type":"codegen","p":0.82,"v":120,"c":6,"tokens":2400,"ok":true}
```
**Rolled-up record (per model × task_type), rewritten incrementally:**
```json
{"model":"qwen3-30b-a3b-q4","task_type":"codegen","n":412,"p_hat":0.81,"v_hat":118,"c_hat":6.1,"p_var":0.0019}
```
The rollup is the measured track record. After every task, feed the actual outcome to the
kernel `Recalibrator` (EMA): `Recalibrator.update(p_actual)`, `Recalibrator.update(v_actual)`,
so `p_hat`/`v_hat` track reality instead of assumption.

**Selection (per incoming task of type τ, bankroll k, budget B, stake s_r per lane):**
1. For each candidate route r ∈ {cheap, mid, expensive} with measured (p_r, v_r, c_r):
   - `EV_r = ev(p_r, v_r, c_r) = p_r·v_r − (1−p_r)·c_r` — kernel `ev`.
   - `R_r = ruin_prob(p_r, k, s_r, B)` — kernel `ruin_prob`, gambler's-ruin `(q/p)^k`.
2. Reject any r with `R_r > ε_cap` (ε_cap = 0.02).
3. Pick `r* = argmax_{survivors} EV_r`.
4. Lane width from kernel `kelly_fraction`: `b_r=(v_r−s_r)/s_r`, `f* = kelly_fraction(p_r,b_r)`,
   `L_r = floor(½·f*·(B/s_r))` (½-Kelly margin).
5. Spend guard: if `chernoff_token_tail(μ,δ,n) > ε_spend`, shrink `L_r`.

This is genuinely multi-layer: cheap tier is the *default* but escalates automatically
whenever its `ruin_prob` breaches `ε_cap` or `EV_r ≤ 0` (unfavorable lane ⇒ `ruin_prob`=1).
Tier choice is data-driven, never hardcoded-cheapest.

## B. Research: Generate-then-Argue Loop

Run K adversarial rounds **before adoption**:

- **GEN** — generator agent emits a candidate claim/blueprint + evidence + `ev` estimate of value.
- **ARGUE** — critic agent (escalated to a *capable* tier, since critique needs skill) attacks:
  states the single strongest *falsifiable* objection (mechanizes the DECART mandatory probe)
  and estimates downside via `ev` if the claim is false.
- **ADJUDICATE** — RED/GREEN verifier gate. Sustained falsifiable objection ⇒ RED ⇒ re-dispatch
  GEN with the objection as a hard constraint; else GREEN.
- **STOP** when (a) GREEN after K rounds with no sustained objection, or (b) `ruin_prob` of
  further research > `ε_cap` ⇒ adopt best-so-far.

Each round appends to `logs/research_argue.jsonl`:
`{"round","claim","critic_obj","gate","ev_claim","critic_downside"}`.
The generator's survival-`p` per task_type feeds the `Recalibrator` in (A), so weak generators
auto-demote to cheaper lanes.

**Command flow:**
```
swarm_exec --research <topic>
  → GEN blueprints (tier from A)
  → fan-out N executor lanes (L_r from kelly_fraction)
  → ARGUE critic tier (capable model)
  → adjudicate gate; RED → re-dispatch with objection (≤K)
  → GREEN → adopt + emit DECART report
       (candidates × criteria table, falsifiable DECISION line, mandatory probe = argue loop)
```
The argue loop *is* the DECART probe, mechanized and measured.
