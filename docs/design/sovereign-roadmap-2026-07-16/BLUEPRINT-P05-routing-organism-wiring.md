# BLUEPRINT — Phase 5: ROUTING ORGANISM WIRING (P0-C1: 100% compute → live)

> **Master roadmap:** `R2-MERGED-PHASE-ROADMAP.md` Phase 5 (Wave 0).
> **Anchors:** E13, E14, E15, E19, E20, F6.
> **Depends on:** — (Wave 0). **Parallel-safe with:** Phases 1, 2, 3, 4.
> **Primary sources (reused verbatim, not re-derived):**
> `HK05-REALTIME-MODEL-ROUTING-INTEGRATION-2026-07-16.md` (status audit, exact
> line numbers), `R1-B-hub-autonomy-agent-infra-gap-analysis.md` §E15/E19/E20/F6,
> `SYNTHESIZED-BLUEPRINT-PLAN-2026-07-16.md` §2 Cluster C **P0-C1** (this phase =
> "exactly P0-C1 + the four extras named in the target").
>
> **This is dev-tooling for agent sessions working ON dowiz, not a
> dowiz/DeliveryOS product feature.** See §8. Do not overclaim it as a product
> capability.

---

## 1. Current-state evidence (compute = 100% built + tested; wiring = 0%)

Every claim below is a code read from R1-B §E15 (which re-verified HK05 with the
`grep -a` correction — the CLI file carries a non-UTF8 byte that silently blanks
plain `grep`, so the ops **are** present). Line numbers are load-bearing.

**Compute engine — all built, all tested, all CLI-dispatchable:**

| Symbol | Location | Note |
|---|---|---|
| `TaskFeatures` | `hermes-kernel/kernel/src/routing.rs:33` | cheap pre-prompt features (msg length, tool-chain depth, scope) |
| `Complexity{Simple,Moderate,Complex}` | `routing.rs:43` | the bucket enum |
| `classify_complexity()` | `routing.rs:67` | heuristic, no model call |
| `rank_models_for_bucket(bucket, history, available)` | `routing.rs:114` | **literally calls** `harmonic_centrality` (`:26,156`) over a per-bucket success-rate graph |
| `ev`, `kelly_fraction`, `ruin_prob` | `control.rs:137,149,158` | EV = p·v − (1−p)·c; ruin-cap gate |
| `lane_size`, `pid_parallelism` | `control.rs:182,191` | adaptive lane width from arrival-rate / service-time |
| `ev_route_select`, `jury_aggregate` | `control.rs:224,259` | route selector; 3-vote jury |
| `Budget`, `Recalibrator` | `control.rs` (per HK05 table) | spend accounting primitives |
| CLI ops: `op_classify_complexity`, `op_rank_models`, `op_gov_route`, `op_gov_lane` | `cli/src/main.rs:199,220,545,572` (dispatch `:394-413`) | all live ops, binary really does this |
| HK-03 `verification.rs` `SessionState` FSM (`Complete` legal only from `Verified`) | `hermes-kernel/kernel/src/verification.rs:1-29`; CLI op `cli/src/main.rs:82,409` | evidence-backed session-close gate |
| `FalseClaimMeter`, `ananke_check`, `decide` | `hermes-kernel/kernel/src/governance.rs:1-25` | claimed-vs-verified audit |
| `harmonic_centrality` (dowiz mirror) | `dowiz/kernel/src/harmonic.rs:26` (ported per `:3-4` "HK-05/HK-06") | 10+ tests, wasm-wired |
| `TokenBucket` | `proto-wire/src/transport_policy.rs:30` (tested `:189`) | pure accounting primitive, reusable |
| `deliberate()` author↔mirror dialogue (2-lap cap) | `bebop2/core/src/deliberate.rs:1-17` | live in `self_mod_loop.rs` |

**The live gap — `tools/telemetry/governance.sh` (the path agent sessions use today):**

- `gov_route()` (`:43-73`) folds `track_record.jsonl` **by `task` only** and calls
  **only** `op_gov_route`. It **never** calls `op_classify_complexity` or
  `op_rank_models` — so `rank_models_for_bucket` / `harmonic_centrality`, though
  fully built, are **never reached from live code**.
- `gov_lane_width` (`:178`) takes **manual args**; it is never fed by
  `lib.sh::resource_sample()` / `bench_run()`, which already collect live
  arrival-rate / service-time. Lane width (the N=4/N=8 in `swarm_proof.py`) is a
  **hardcoded constant**.
- `gov_research` (`:75-80`) only logs `"dispatched"` to the precedent JSONL +
  Telegram; the argue loop runs agent-side, **unenforced** at the routing tier.
- The session-close **verification_gate is never invoked** from `governance.sh`.
- There is **no per-agent spend budget** in the reject path.

Dispatch table already present (`governance.sh:340-364`):
`record/route/lane/research/hard/judge/gate/precedent/meta/falseclaim/learn/anu/ananke/decide`.
`gov_kern` (`:2-32`) is the thin JSON bridge to `KERNEL_BIN`. **Every op this
phase needs is already dispatchable — this is wiring, not new compute.**

---

## 2. The three-call wiring plan for `governance.sh` (P0-C1, verbatim from SYNTHESIZED §2-C)

Sketch of the intended call sites in `gov_route()` (planning pseudocode — no
script is edited in this phase):

```sh
gov_route() {           # $1 = task JSON (prompt text, tool-chain hint, scope)
  # (1) CLASSIFY on task entry — cheap, no model call
  bucket=$(gov_kern classify_complexity "$(task_features "$1")" | jq -r .bucket)
  #      -> op_classify_complexity  (main.rs:199)  ->  Simple|Moderate|Complex

  # (2) RANK folding track_record by (bucket, model), NOT task_type only
  history=$(fold_track_record --by "bucket,model" --bucket "$bucket")   # see §3
  ranked=$(gov_kern rank_models "$(jq -n --argjson h "$history" \
             --arg b "$bucket" --argjson a "$AVAILABLE" \
             '{bucket:$b, history:$h, available:$a}')")
  #      -> op_rank_models (main.rs:220) -> rank_models_for_bucket
  #         (routing.rs:114) -> harmonic_centrality (routing.rs:26,156)

  # (3) LANE width fed by LIVE telemetry (not a hardcoded arg)
  read arrival service <<<"$(resource_sample)"      # lib.sh already collects this
  lane=$(gov_kern gov_lane "$(jq -n --argjson ar "$arrival" \
           --argjson st "$service" --argjson u "$U_TARGET" \
           '{arrival_rate:$ar, service_time:$st, u_target:$u}')")
  #      -> op_gov_lane (main.rs:572) -> lane_size (control.rs:182)
  #         + pid_parallelism (control.rs:191)

  top=$(echo "$ranked" | jq -r '.[0].model')
  # EV gate stays the existing op_gov_route (control.rs ev/ruin_prob):
  gov_kern gov_route "$(route_payload "$top" "$lane")"   # + §4 budget check first
}
```

Net effect: the fixed N=4/N=8 benchmark becomes **adaptive** — a `Simple` bucket
gets a narrow cheap lane with a cheap model; a `Complex` bucket gets a wider lane
with the costlier architect/executor; and lane width **breathes** with load
instead of being a launch-time constant. This is exactly the "three already-built
pieces, connect them" mechanism HK05 §4 describes — nothing new is invented.

---

## 3. `track_record.jsonl` schema migration (backward-compatible)

Today each line folds by `task` only. Add one field:

```jsonc
// existing line (still valid after migration):
{ "task": "refactor", "model": "haiku", "p": 0.91, "v": 1.0, "cost": 0.4 }

// new line:
{ "task": "refactor", "bucket": "Complex", "model": "opus",
  "p": 0.88, "v": 1.0, "cost": 3.1 }
```

**Backward-compat rule (verbatim from P0-C1 / HK05 §4):** a **missing `bucket`
field defaults to `Simple`**. No rewrite of history, no migration script, no
version bump on the file. The fold in step (2) groups by `(bucket, model)`;
legacy rows land in the `Simple` bucket, which is the safe (cheapest-route) side
— an unclassified record can never inflate a route. New writes (from the routing
call in §2 and from `verification_gate` outcomes in §6) carry `bucket`. The
`FalseClaimMeter` / precedent consumers already tolerate extra keys (JSON-object
fold), so no downstream reader breaks.

---

## 4. TokenBucket / per-agent spend budget (F6 / E19 — the reject path)

**Extra beyond P0-C1.** `TokenBucket` already exists and is tested
(`transport_policy.rs:30,189`); `Budget`/`Recalibrator` already exist in
`control.rs`. This phase **reuses, does not rebuild** (E19 GAP is explicit:
"reuse at two more tiers").

Design:
- A per-agent (per-session) spend `Budget` is threaded into `gov_route()`
  **before** the EV gate. Each candidate route carries its `cost` (already in
  `track_record`); the estimated spend is drawn against the agent's `TokenBucket`.
- **An over-budget call must be refused, not silently downgraded or allowed**
  (F6 LOCK "+ TokenBucket"; the target's own wording). The reject path returns a
  typed `BudgetExhausted` reject from `op_gov_route`, logged to the precedent
  store like any other reject — the session gets a hard "no", not a quiet cheaper
  model.
- The bucket refills per the `Recalibrator` cadence; refill/ceiling are operator
  config, local-only (M8), never exfiltrated.

Falsifier: inject a spend that exceeds the remaining bucket → the call is
**refused** (RED→GREEN test in §7 item 3).

---

## 5. `deliberate()` / jury wiring for Complex-bucket routes (E20)

**Extra beyond P0-C1.** The primitives exist (`deliberate.rs:1-17` live in
`self_mod_loop.rs`; `jury_aggregate` at `control.rs:259`; `governance.sh` has
`judge`/`gate`/`research` verbs) but are **unenforced at the routing tier**
(R1-B §E20 GAP: "`gov_research` only logs dispatched … the argue loop runs
agent-side, unenforced").

Wiring rule: **when `bucket == Complex`, the route must pass through
`deliberate()` / jury before dispatch**, and the **verdict must be recorded in
the precedent store**. Sketch inside `gov_route()`:

```sh
if [ "$bucket" = "Complex" ]; then
  verdict=$(gov_kern judge "$(deliberate_payload "$top" "$1")")   # deliberate()/jury
  gov_kern precedent "$verdict"        # verdict lands in the precedent JSONL
  [ "$(echo "$verdict" | jq -r .adopt)" = "true" ] || return 1    # not adopted -> no dispatch
fi
```

Simple/Moderate buckets are unaffected (no debate tax on cheap work). Only
`Complex` — the expensive, wide-lane routes — carries the paired-debate gate.

Falsifier: after a Complex-bucket route runs, **its debate verdict appears in the
precedent store** (§7 item 4).

---

## 6. HK-03 `verification_gate` at session close (E9 verifier half)

**Extra beyond P0-C1.** HK-03 (`verification.rs:1-29`) is built and
CLI-dispatchable (`cli/src/main.rs:82,409`): `SessionState` is an FSM where
`Complete` is **legal only from `Verified`**, evidence-backed. R1-B §E9 GAP:
"session-close `verification_gate` is not invoked from the live `governance.sh`
path."

Wiring rule: the session-close path in `governance.sh` (the point that would
emit a `"complete"` signal) must first call the `gate` op:

```sh
gov_close() {
  state=$(gov_kern gate "$(session_evidence)")     # -> verification_gate (main.rs:409)
  if [ "$(echo "$state" | jq -r .state)" != "Verified" ]; then
    echo "REFUSED: session edited code with failing/absent evidence" >&2
    return 1                       # cannot emit "complete"
  fi
  emit_complete
}
```

**A session that edited code with failing evidence cannot emit "complete"** — the
`Complete` transition is unreachable from any non-`Verified` state, and this phase
makes that FSM actually gate the live close, not just exist as a dispatchable op.
This is the routing organism's honesty backstop; note it is *self-context* here
(same agent claims and checks) — the *independent* verifier signature is Phase 6
(V1), not this phase. This phase only wires the existing self-check into the live
path.

---

## 7. Acceptance criteria (numbered, RED→GREEN, falsifiable)

The done-test is falsifiable, not vibes. Each item must be demonstrated RED on
pre-wiring code and GREEN after:

1. **Bucket changes the route.** The *exact same task*, classified `Complex`,
   receives a **different (wider / more expensive) route** than the same task
   classified `Simple`. (HK05 §4 verbatim; P0-C1 acceptance.)
2. **Live lane width breathes.** Lane width **visibly changes** when injected
   arrival-rate telemetry changes (fed via `resource_sample()` → `lane_size` /
   `pid_parallelism`), not a hardcoded constant.
3. **Over-budget is refused.** An over-budget call is **refused** (typed
   `BudgetExhausted` reject) — not silently downgraded, not silently allowed.
4. **Debate verdict is recorded.** A `Complex`-bucket route's `deliberate()` /
   jury verdict **appears in the precedent store** after the call.
5. **Session-close honesty.** A session that **edited code with failing evidence
   cannot successfully emit a "complete" signal** (HK-03 FSM: `Complete` only
   from `Verified`).
6. **Backward-compat.** A legacy `track_record.jsonl` line with **no `bucket`
   field** is folded as `Simple` and produces a valid (cheap-side) route; no
   reader errors.
7. **No new compute, no new deps.** Only `governance.sh` (+ the fold helper) and
   the `track_record.jsonl` schema change; the `hermes-kernel` binary is
   unchanged (every op already dispatchable). Zero new crates.

---

## 8. Explicit scope note (dev-tooling, NOT a product feature)

Quoting HK05 §5 and SYNTHESIZED §2-C's own framing, carried here deliberately:

> **HK-05/HK-09 routing is dev-tooling for agent sessions working *on* dowiz, not
> a dowiz/DeliveryOS product feature.** "dowiz today doesn't run its own LLM
> agents in production that would need this router." Forcing the connection to a
> shipped delivery-platform capability "would be the same overclaiming this
> project already rejects elsewhere."

Corollaries, load-bearing:

- **E13/E14 are explicitly NOT built here.** This phase formalizes the **current
  managed-advisory tier as correct-for-now** (headroom proxy + hosted models).
  Self-hosting llama.cpp (120k★ MIT) / vLLM (86k★ Apache-2.0) as the *execution*
  tier is a **separate blueprint that ships in Phase 15**, gated on **GPU-unlock**
  — an external trigger (operator / network `cargo add wgpu`, ARCHITECTURE §8),
  **not this phase's job** (R2 dependency graph: E13-execution ⟵ GPU-unlock).
- The one legitimate *future* hook (HK05 §5): *if* dowiz ever ships an AI-agent
  product feature (e.g. an owner-panel assistant), EV-driven tiered routing is
  already a proven pattern to reuse under the same `bebop mcp` port — but that is
  a **conditional future hook, not present-day scope**, and must not be written
  into this blueprint as a product claim.
- Everything here obeys M8 (local-only telemetry, never exfiltrated) and the
  SCOPE RULE (this wiring is the operator's own dev-time tooling; a sovereign hub
  may route however it likes).

---

*Phase 5 of the 19-phase R2 master roadmap. Wave 0, parallel-safe with Phases
1–4, no hard dependencies. Every current-state line traces to a code read in
HK05 or R1-B §E15/E19/E20/F6; the wiring plan reuses SYNTHESIZED §2 Cluster C
P0-C1 verbatim plus the four named extras (budget, debate, session gate, live
lane). Planning blueprint only — no scripts edited.*
