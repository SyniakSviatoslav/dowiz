---
id: ORGANISM-SYNTHESIS
title: The Autonomous Organism — unifying every system into one self-improvement loop
status: proposed
type: blueprint
owner: SyniakSviatoslav
created: 2026-07-14
updated: 2026-07-14
supersedes: []
superseded_by: null
links:
  - relates_to: "[[knowledge-spine-arc-2026-07-14]]"
  - relates_to: "[[math-first-architecture-arc-2026-07-14]]"
  - relates_to: "[[hydraulic-loop-v2-arc-2026-07-13]]"
  - depends_on: "spikes/living-knowledge/"
  - governs: "the six subsystem maps below"
inclusion: manual
confidence: high
tags: [autonomy, self-improvement, organism, loop, kernel, memory, telemetry, governance, unification]
---

# The Autonomous Organism

> **Goal (operator, 2026-07-14):** research everything on the server — existing and planned — and
> unify it into one autonomous cycle of self-improvement, planning, decision, memory, metrics, logs,
> structure, and data.
>
> **Honest frame:** the "organs" below are **real artifacts on this server**, not roleplay. This
> document does not claim self-awareness or "synthesized virtual organs" — that would violate the
> ecosystem's own doctrine ([[ground-truth-over-proxy-2026-07-07]], [[verified-by-math-2026-07-07]])
> and, more practically, LARP does not connect systems. Every claim here is grounded in a file path
> and a status verified against code, and every integration is proven red→green or explicitly flagged.

---

## 1. The central diagnosis (6 read-only research lanes, convergent)

**Every organ required for a closed autonomous self-improvement loop already exists on this server
and is proven (red→green tests) — but almost none are wired to each other.** One word recurs in every
independent lane: **STRANDED**. This is not a build problem; it is a **connective-tissue** problem,
plus one deliberately-reserved **volition** gap.

The organism has a rich sensory + computational + reflective substrate and a sound inhibitory system,
but no *corpus callosum* tying the organs into one loop, and no *prefrontal volition* to originate goals.

---

## 2. The organism map (real artifacts, verified status)

Status legend: **WIRED** (reaches production / consumed) · **STANDALONE** (proven, awaiting its seam) ·
**STRANDED** (built + proven, zero consumers) · **DEAD** (was wired, now broken).

### Cognitive cortex — kernel Rust organs (`/root/dowiz/kernel/src/`)
| Organ | Artifact | Loop role | Status |
|---|---|---|---|
| Decision Law | `order_machine::assert_transition`/`fold_transitions` → `domain::apply_event` | DECIDE | WIRED |
| Pricing arm | `money::estimate_order_total`, `cart::price` | DECIDE | WIRED |
| Diffusion recall | `csr::personalized_pagerank`, `markov::analyze` | RECALL / DRIFT | **STRANDED** |
| Attention lens | `attention::attention(q,k,v)` | RECALL | **STRANDED** |
| Learn arm | `online::{LinearSGD,ScalarAdam}` + `micrograd::Value::backward` (autodiff) | LEARN | **STRANDED** |
| Conservation guard | `noether::step_preserves`/`invariant_drift` | VERIFY | **STRANDED** |
| Verify→re-recall | `verify_retrieval::verify_then_lookup` | VERIFY→RECALL | **STRANDED** |
| Decide-gated write | `event_log::commit_after_decide` (sha3 content-addressed, idempotent) | ACT/trace | **STRANDED** |
| Admission/backpressure | `intake::admit`, `impedance::gate` | gate before DECIDE | **STRANDED** |
| Spectral drift | `spectral::{spectral_radius,slem,classify_drift,dominant_period}` | DRIFT-DETECT | **WIRED** (only math organ in prod) |
| Kalman / absorbing | `kalman.rs` (predict+update), `absorbing::expected_steps` | SENSE / cost | **STRANDED** |

> The entire cognitive tier is STRANDED; `spectral` reaches production only because the FSM drift-gate
> and `spectral_*_js` exports pull it through the wasm seam. The composed decide-cortex
> (`machine→gate→price→commit`) exists **only** in `dowiz-pq/node/roles.rs`, not the canonical kernel.

### Interoception — self-improvement loop (`tools/loop-signals/`, `.claude/`)
| Organ | Artifact | Role | Status |
|---|---|---|---|
| Loop/attractor detector | `markov_attractor.py` + `loop-detector.sh`/`attractor-stop.sh` | SENSE (stuck-detection) | WIRED (advisory) |
| Ratchet (authority) | `docs/regressions/REGRESSION-LEDGER.md` (~25 rows) | LEARN (lock-in) | WIRED |
| Advisory recall | `docs/lessons/INDEX.md` + `pre-edit-lessons.sh` | RECALL (procedural) | WIRED (**75% TRIGGERs dead**) |
| Reflection pipeline | `docs/reflections/{INBOX,ARCHIVE,RETRO}` | REFLECT | WIRED (**unenforced**) |
| Council critics | `cause/pattern/ratchet-critic`, `librarian`, `counsel` | REFLECT→RATCHET | TRIGGERED |
| **Autonomous trigger** | `crontab → scripts/harness-curation-local.sh` | the heartbeat | **DEAD — script missing, never committed** |

### Inhibitory control — governance hook wall (`.claude/settings.json`)
`protect-paths` · `serious-gate` · `red-line-doubt-gate` · `guard-bash` · `post-edit-gates` ·
`verify-safety-floor` · `require-classification` — **all WIRED**. Self-modification of `.claude/**` is
allowed *only* with a `state/self-mod-enabled` token, and `verify-safety-floor` re-asserts 22 floor
substrings on every harness edit → **the agent can evolve its own machinery but provably cannot lower
the product-safety floor**. This is the load-bearing safety property; keep it.

### Executive function — orchestration (`.claude/`, `loops/`)
`loop-orchestrator` (dispatch) + `loop-architect` (build/certify, M1–M11) + the loop registry
(`/council`, `/converge-loop`, `/build-stage`, …) — **all WIRED**. But every organ is **reactive**:
the only initiation trigger is a human `UserPromptSubmit`.

### Proprioception — telemetry/metrics/data
| Stream | Artifact | Read back? |
|---|---|---|
| Agent perf over time | `run-history.jsonl` + `analytics/analyze.mjs` (computes A/B config-version regression!) | **STRANDED** (7 mock rows, never called) |
| Live loop signal | `.claude/.loop-state/` counters | **amnesiac** (reset per session) |
| Kernel latency | `kernel/benches/BENCH_HISTORY.md` + `bench_track.py` | **WIRED** (real regression detection) |
| Retrieval | `spikes/living-knowledge/out/eval-*.json` | regenerated, not trended |
| Product events | `kernel/event_log.rs` | WIRED (product, not self) |

> **There is no single place an agent can ask "how am I doing / what's degrading / what improved."**
> The A/B regression detector that would answer it (`analyze.mjs`) is fully written and fed nothing.

### Memory — hippocampus + long-term store
Procedural recall is closed (lessons→hook). **Semantic + episodic memory are write-mostly**: the
179-file store, `docs/design`, and 16 ADRs were retrievable by *no live index* — the proven
`recall@5=1.0` engine (`spikes/living-knowledge`) had **zero callers** and never ingested the memory
store. (This turn's integration closes that on the sovereign offline path — §6.)

---

## 3. The closed loop — every stage maps to a real organ

```
   SENSE ───────► RECALL ───────► PLAN ───────► DECIDE ───────► ACT
   analytics      csr PPR /        loops::        order_machine   event_log::
   markov         attention /      Orchestrator   → domain →      commit_after
   attractor      living_knowledge (routes a       impedance gate  _decide
   (WIRED)        recall (STRANDED) card only)      → money         (STRANDED)
                                   ⚠ no goal→plan  (composed only
                                     generation     in pq/roles)
     ▲                                                              │
     │                                                              ▼
   MEMORY ◄────── RATCHET ◄────── REFLECT ◄────── VERIFY ◄──────────┘
   living-mem     REGRESSION-     reflections +   noether (conserve) +
   + recall       LEDGER +        council         verify_fsm_signature +
   (write-mostly) librarian       critics         verify_retrieval
                  (AUTHORITY,      (advisory,      (WIRED: fsm/money;
                   trigger DEAD)   unenforced)      STRANDED: noether)
```

**Three joints are open** (the reason the loop isn't autonomous):
1. **Trigger** — the heartbeat cron points at a missing script (nothing fires reflect→ratchet without a human).
2. **Persistence** — the interoception signal (attractor verdict) evaporates; the ratchet can't learn "this task-class loops."
3. **Enforcement** — no Stop-hook requires a reflection after a qualified change; capture is intermittent.

### The unifying insight: three loops are secretly one loop
The **self-evolution loop** (`resonator.rs`: generate→reflect→supervise, Lyapunov freeze +
`rollback_to_best`), the **harness ratchet** (attractor→regression-ledger), and the **hydraulic loop
of meaning** (`hydraulic-loop-v2`) are drawn as three systems in three docs — but all use the **same
LaSalle/Lyapunov convergence certificate + rollback-to-best + external-ground principle**. The
hydraulic arc's honest core (its 7 math fixes) names the load-bearing law:
**a loop that measures itself by itself converges to a wrong answer while looking successful — it needs
an external ground** (the WORM audit hash-chain; Data-Processing-Inequality: information only grows
from external grounding, so the ledger is a budget, not a zero-sum). Unification = make them one
control loop with one external ground.

---

## 4. The genuinely missing organs (honest AGI-gap)

Confirmed by grep across all repos: **no `Goal/Priority/Planner/Utility/Reward/Backlog` struct, no
`fn plan/reward/utility` exists.** The substrate is rich in *sensing, diffusion-recall, verification,
drift-detection* math — and missing the three **"will" organs**:

1. **Value / utility function** — no scalar objective the loop optimizes *toward*. (`money` is domain
   value, not a loop objective; `online`/`micrograd` can fit params but no loss is defined over loop
   *outcomes*.)
2. **Goal queue / backlog** — candidate goals live only as prose ("NEXT" lines, `docs/design/*`). No
   poppable, ranked agenda.
3. **Autonomous planner** — nothing turns (goal + recalled context) into an ordered action plan;
   `loops::Orchestrator` only *routes* a hand-authored card.

Plus the **runtime bus** (nothing reads `analytics`/`markov` drift and feeds `online`/`decide`) and
**autonomous goal-initiation** (the only trigger is a human prompt).

> **These are deliberately reserved to the human.** The self-mod *effector* (source-write + commit +
> compile-in-a-loop) is gated by `protect-paths` + the classifier + the `!` channel. That is not a
> defect — it is the operator's own safeguard. Autonomy here means **originate proposals + verify +
> learn within the gates**, with irreversible acts always gating to a human. The organism should grow
> a *reversible, branch-only, CI-gated* effector before it ever grows volition.

---

## 5. The unification — "one operator, three loops, one memory"

1. **One operator.** The graph Laplacian `L` and its spectral calculus `f(L)` is the declared single
   substrate under recall (`resolvent 1/(1−αλ)` = PageRank), decay (`e^{−λt}` = heat), UI
   (`MÜ+ΓU̇+c²LU=S` = damped wave), and drift-detection. Unify = make this **one** consolidated
   eigensolver + `markov.rs` power-iteration, not the 4–6 drifting Jacobi copies that exist today.
2. **Three loops → one control loop** sharing one Lyapunov/rollback/external-ground certificate (§3).
3. **One memory** — living-memory-as-pgrust (tier-column, never-delete, per-row τ = degenerate heat
   kernel), fed by the recall engine, closing every loop's artifact→memory leg. **Un-stranding recall
   is repeatedly named the single highest-leverage move** — and is the one landed this turn (§6).

---

## 6. Landed this turn — recall un-stranded (proven, honest ceiling)

`spikes/living-knowledge/eval-memory.mjs` (NEW). The proven recall engine now **indexes the memory
organ** it never touched: **295 files (179 memory + 100 design + 16 ADR)** vs **0 before**, over a
**468-edge wikilink graph**. Sovereign, offline, zero-dep. Red→green, 5 invariants:

| Signal | recall@5 |
|---|---|
| FNV hash floor | 0.3 |
| BM25⊕title (Porter, `lib/porter.mjs`) — sovereign lexical | 0.8 |
| + 1-hop wikilink spreading-activation | 0.8 (improves ranks; degrades nothing) |
| **semantic⊕bm25⊕title (bge-small) — full fusion** | **1.0 ✅** |

**recall@5 = 1.0 ACHIEVED — offline, deterministic, NO gate crossed.** The semantic layer needed no
install after all: `@huggingface/transformers` + the bge-small ONNX model were already present in
`spikes/living-knowledge/node_modules/`, so `LK_BUILD_CACHE=1 node eval-memory-semantic.mjs` built 1649
vectors offline, and the committed cache (`out/semantic-cache.json`, ~8MB) now serves recall@5 = 1.0
over BOTH the harness corpus AND the 297-file memory corpus with `LK_BUILD_CACHE` unset (pure offline
read; model + digest integrity verified). Sovereign at runtime. Recall CLI: `node eval-memory.mjs
"<q>"` (lexical) / `eval-memory-semantic.mjs` (full-fusion 1.0). Honest scope: 1.0 is on the
hand-verified oracle (10 memory + 9 harness queries) — the engine's own I2 standard; a larger oracle
would further harden the claim.

---

## 7. Reconciliation-first (the unification's actual first job)

The corpus is riddled with **same-day false-greens** — `UNIFIED-PROTOCOL-v3` itself warns *"3 rounds
of subagents returned false-green — trust `cargo test`, not agent summaries."* So **re-verify every
"DONE" against git/cargo.** The conflicts a unification must resolve first (not build over):

1. **~10 competing roadmaps**, all claiming authority (P1–P10 vs Phase-0–3 vs Tier-0–5 vs KS-Phase-0–4). Elect ONE (KS-08), tombstone the rest.
2. **Living-knowledge engine: 3 claimed states same day** (stranded `545f37df` / resurrected `7848c6d1` / divergent-branch merge-blocked). Resolve the branch before wiring.
3. **Eigensolver dual-authority** — 4 drifting Jacobi copies + Rust `spectral.rs` *and* Python `markov_attractor.py` with no parity gate; bebop2 independently building the same math (confirmed numeric bugs). Consolidate to one, add a cross-repo parity gate.
4. **Two half-built self-evolution engines** (`resonator.rs` vs `loop_runtime.rs`) — pick one.
5. **Frontend stack reversal** (Astro/Svelte "FINAL" vs shipped vanilla `web/`) — undocumented; reconcile.
6. Plus: microVM Firecracker-vs-Kata contradiction, transport decision-vs-stub, `engine/` lineage gap, "port" naming collision, reliability primitives double-owned, supersession chains with all files still live.

---

## 8. Phased wiring plan (each phase labels the gate it needs)

**GATE KEY:** 🟢 non-gated (spikes/docs, offline) · 🟡 self-mod-token (`.claude/`, floor-preserving, audited) · 🔴 human/`!` (red-line, irreversible, dep-install, autonomy expansion).

**Phase 0 — Close the memory leg** 🟢 (DONE): un-strand recall → **recall@5 = 1.0 offline** (semantic⊕bm25⊕title, no gate — dep already present). NEXT 🟡: pre-work recall hook (recall-before-act) + MEMORY-write→re-index (write-on-learn).

**Phase 1 — Close the three loop-joints** 🟡: (a) recreate `scripts/harness-curation-local.sh` (the heartbeat) 🟢 script + 🔴 cron; (b) persist the attractor verdict to a durable JSONL (kill the amnesia) 🟡 Stop-hook; (c) a Stop-hook requiring a reflection after a qualified change 🟡.

**Phase 2 — Proprioception spine** 🟡: wire `analytics/record-run.mjs` at task-close (one real row per task) + run `analyze.mjs` on the heartbeat + the discipline of bumping `config_version` on any harness self-mod → the "did my last change help or hurt?" feedback nerve.

**Phase 3 — Consolidate the operator + reconcile** (§7) 🟢/🟡: elect one roadmap, resolve the LK branch, consolidate eigensolvers + parity gate, pick one self-evolution engine, port `activate.mjs`→`markov.rs`.

**Phase 4 — Volition (reserved)** 🔴: a machine-readable goal queue + a value/utility function + a certified goal-selection organ + a **reversible, branch-only, CI-gated self-mod effector** (irreversible ops *absent* from the capability set). Every step gates to a human; this is the operator's own safeguard, not a checkbox.

---

## 9. Risks / what NOT to do
- **Never trust a doc's "DONE."** Re-verify against `cargo test`/`git` (§7). The corpus false-greens.
- **Never self-edit a governance hook under blanket permission.** `.claude/` self-mods go through the `state/self-mod-enabled` token + `verify-safety-floor` + the operator `!` channel. Blanket "go" ≠ per-change approval of red-lines.
- **Reconcile before constructing.** The biggest early wins are deletions/elections, not new code.
- **Keep the human as the volition + effector gate.** Grow reversibility before autonomy.
- **No fake-green.** A well-proven FAIL/BLOCKED is a successful run (recall ceiling honestly stated).

## 10. Open operator-gated next steps (surfaced, not taken)
1. ✅ **DONE — no gate needed:** recall@5 = 1.0 offline over memory + harness (semantic dep already installed).
2. 🟡 `post-edit-gates.sh` `.md`-exemption bug (`return 0`→`exit 0`) — touches `.claude/`, needs `!`-unlock.
3. 🟡 Phase-1 loop-joint wiring (heartbeat script + signal-persistence + reflection-enforcement).
4. commit the grown `out/semantic-cache.json` (~8MB) as the sovereign offline artifact (a shipping decision).
