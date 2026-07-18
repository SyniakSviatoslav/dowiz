# MASTER ROADMAP — Sovereign Architecture (dowiz + openbebop), 2026-07-16

> **Single source of truth for the TARGET:** `docs/design/ARCHITECTURE.md` §0 (MESH-FOUNDATION +
> SCOPE RULE) + §8 (honest gaps), cross-locked with `docs/design/STRATEGIC-VECTORS-LOCKED-2026-07-16.md`.
> **Single source of truth for the PATH:** this document + `sovereign-roadmap-2026-07-16/R2-MERGED-PHASE-ROADMAP.md`
> (detailed reference table) + 19 per-phase blueprints in the same directory (execution detail).
>
> This document does not restate the architecture — it maps the shortest honest path from the
> repo's ACTUAL current state to that architecture, in full, with zero anchors deferred outside a
> phase and zero pre/post-MVP split, per the operator's 2026-07-16 directive. It supersedes
> `MASTER-ROADMAP-10-PHASES-2026-07-14.md`, `MASTER-EXECUTION-PLAN-2026-07-13.md`,
> `MASTER-INTEGRATION-PLAN-2026-07-14.md`, `MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md`
> (added 2026-07-17 — omitted from the original list; P-I consolidation audit §0 confirmed it is
> superseded like the rest), and `MASTER-ROADMAP-MVP-2026-07-12.md` (root) — all
>
> ⚠️ **CANON DRIFT — 2026-07-17.** §1.3/§1.6(a) "CI runs exactly 2 jobs / kernel+engine tests run nowhere
> / CONTRIBUTING.md:17 false / ARCHITECTURE.md §8 still claims Apache-2.0 mismatch" describe the pre-P01
> state and are FALSE vs HEAD. P01 `4b05ee588` added all seven CI gates; P18 corrected ARCHITECTURE.md
> §8 (LICENSE is AGPLv3 since `ac1caba40`; secret scrub CLOSED). Live repo is truth.
> pre-date the mesh-foundation pivot (`0d1935d96`) and are kept as history, not law, per
> `ARCHITECTURE.md`'s own "merge, never append" rule. It does **not** edit `ARCHITECTURE.md` itself;
> `BLUEPRINT-P02` is the proposed merge-diff, left for the operator.

---

## 0. Provenance — how this was built (so the result can be checked, not just trusted)

Following the operator's routing directive (Fable for synthesis grounded in code + research +
target architecture; Opus for blueprints), this roadmap was produced in three rounds, all
code-grounded, none self-certified without evidence:

1. **Round 1 — five parallel Fable gap-analyses**, each owning a disjoint slice of the 147 nominal
   architecture anchors (M1-12, V1-6, D1-8, S1-9, E1-62, F1-50), each grounded directly in the
   actual code (`/root/bebop-repo` for mesh/PQ-crypto, `/root/dowiz/kernel`+`engine` for
   service/compute, `/root/hermes-agent-kernel-rewrite` for dev-tooling routing), not just prior
   docs. Reports: `sovereign-roadmap-2026-07-16/R1-{A,B,C,D,E}-*.md`.
2. **Round 2 — one Fable merge pass** that resolved cross-cluster dependencies the five reports
   surfaced into each other into ONE dependency-ordered phase sequence, re-verifying every
   load-bearing claim (CI job list, LICENSE text, D5/D8 absence, E10/E36 duplication) against the
   live tree rather than trusting R1's paraphrase. Report: `sovereign-roadmap-2026-07-16/R2-MERGED-PHASE-ROADMAP.md`.
3. **Round 3 — nineteen parallel Opus blueprints**, one per phase from Round 2's table, each a
   concrete, file-and-line-grounded design document (files touched, migration steps, falsifiable
   acceptance criteria) — not implementation. Reports: `sovereign-roadmap-2026-07-16/BLUEPRINT-P{01..19}-*.md`.

Total: 5 + 1 + 19 = 25 agent passes, all read-only research/design (no product code, CI config, or
canon file was edited by this effort — every finding below is a proposal for the operator or a
future implementation pass to act on).

---

## 1. What is actually true right now (headline findings, most load-bearing first)

These surfaced independently across multiple phases' code-grounded research and materially change
what "the roadmap" means — read this before the phase table:

1. **The PQ-crypto substrate is far more built than canon implies, but unconsumed.** `cargo test`
   across all five bebop2 crates is GREEN today (232+ tests). ML-DSA-65 is ACVP-verified. But it's
   a library nobody calls: dowiz has **zero** code-level dependency on the protocol (one stray
   comment at `kernel/src/domain.rs:524`), the wire is authenticated-but-**plaintext-readable**
   (`NoopPayloadEnc`, default feature literally named `insecure-tls`), and the trust root itself is
   classically forgeable (`roster.rs` delegation chain is Ed25519-only, making the ML-DSA leg
   vacuous against the exact quantum attacker it exists to stop). → Phases 3, 9.
2. **`apps/web`, `packages/ui` (all i18n), and `packages/domain` are fully deleted**, not merely
   decoupled (`79ef316f6`/`db766de47`/`fce5738b0`; `git ls-files 'apps/*'` = 0 at HEAD and on
   `origin/main`). Every "re-plumb the product onto the mesh" framing in older docs is stale — this
   is a rebuild, with the As-Built Summary and DOWIZ-INTERFACES checklist as the feature-inventory
   ledger. → Phase 16.
3. **CI is regressed, silently.** A gitleaks gate existed (`b10a7bfe3`) and was dropped when
   `f9ab28ff1` rewired CI to drop all JS/TS. `.github/workflows/ci.yml` today runs exactly 2 jobs;
   kernel's 337 tests and engine's 47 tests run **nowhere** in CI. Every downstream "GREEN" claim
   in this repo has been unverifiable since that commit. `CONTRIBUTING.md:17` additionally makes a
   false claim (a DCO check that doesn't exist). → Phase 1, correctly sequenced as the global
   Wave-0 precondition for trusting anything downstream.
4. **The one mandated global control doesn't exist, and the thing that shares its name is its
   opposite.** M9 requires a unilateral operator kill-switch; the only "KillSwitch"-named
   construct in the codebase (`guard.rs`) is a ≥2/3 **consensus** vote registry — a governance
   mechanism, not a kill-switch, and architecturally the inverse of what M9 specifies. → Phase 10.
5. **V1 (split-identity + adversarial verification) is 0% built while its entire substrate sits
   idle.** All "verification" today is self-context (the same session that made a change judges
   it) — this is the self-certification pattern BRAIN-TOPOLOGY research already flagged as this
   project's dominant failure mode. The ML-DSA/hybrid-gate/genesis-loader primitives V1 needs are
   already finished. → Phase 6.
6. **Canon itself is stale in three checkable ways**, independent of any roadmap work: (a) LICENSE
   is **already AGPLv3** (flipped `ac1caba40`, 2026-07-14) — `ARCHITECTURE.md` §8 still claims an
   Apache-2.0 mismatch; (b) the force-push/history-scrub question is **substantively resolved**
   (H8 runbook closed, origin already at the scrubbed tip) — canon still lists it as open; (c) the
   count "**147 locked anchors**" is off by at least 2 — **D5 and D8 are referenced in the "D1-8"
   total but defined nowhere** in any revision of either canon document (verified by git
   archaeology across all four introducing/amending commits). None of these are edited here —
   `BLUEPRINT-P02` is the proposed merge-diff. → Phase 2.
7. **A real, still-open HIGH-severity finding survives from the prior crypto pass**: the `mod_l`
   constant-time side-channel (C4b, flagged 2026-07-14, never closed) sits on the same Ed25519
   code path that Phase 6's identity ceremony and Phase 10's kill-switch signing would both use —
   this is why Phase 3 (crypto hardening) gates both of them, not just wire work.
8. **Two real design contradictions block dispute/escrow (F44)**, not a missing spec: the only
   existing design (`fable-protocol-2026-07-11/F2-dispute-arbitration.md`, a complete 6-state
   machine with a written RED test) proposes a reputation-scoring jury (contradicts M12's
   NO-COURIER-SCORING law) and UMA/Kleros external arbitration (contradicts M6's zero-dependency
   law). This needs an explicit operator ruling, not a silent pick. → Phase 2 (ruling), Phase 14 (build).

---

## 2. THE 19-PHASE ROADMAP

Full detail — anchors, current→target gap, dependencies, falsifiable done-tests — lives in
[`sovereign-roadmap-2026-07-16/R2-MERGED-PHASE-ROADMAP.md`](sovereign-roadmap-2026-07-16/R2-MERGED-PHASE-ROADMAP.md)
§2. This table is the navigation index; each row links to its execution-grade blueprint.

| # | Phase | Anchors | Depends on | Blueprint |
|---|---|---|---|---|
| 1 | CI Truth Floor | V2,V3,V5,S3,S6,D6,E2,E3,E40,E52,E58,E62 | — | [BLUEPRINT-P01](sovereign-roadmap-2026-07-16/BLUEPRINT-P01-ci-truth-floor.md) |
| 2 | Canon Repair + Operator Decision Batch | D5,D8,V4,V6,E35,E42,E53,E55 | — | [BLUEPRINT-P02](sovereign-roadmap-2026-07-16/BLUEPRINT-P02-canon-repair-operator-decisions.md) |
| 3 | PQ Trust-Root Hardening | M2,M4,M12,E10(≡E36),F19,F21,F24,F26 | — | [BLUEPRINT-P03](sovereign-roadmap-2026-07-16/BLUEPRINT-P03-pq-trust-root-hardening.md) |
| 4 | Kernel Product-Math Primitives | F45,E61 | — | [BLUEPRINT-P04](sovereign-roadmap-2026-07-16/BLUEPRINT-P04-kernel-product-math.md) |
| 5 | Routing Organism Wiring | E13,E14,E15,E19,E20,F6 | — | [BLUEPRINT-P05](sovereign-roadmap-2026-07-16/BLUEPRINT-P05-routing-organism-wiring.md) |
| 6 | V1 Split-Identity + Adversarial Verifier | V1,E9 | 1, 3 | [BLUEPRINT-P06](sovereign-roadmap-2026-07-16/BLUEPRINT-P06-v1-split-identity-verifier.md) |
| 7 | Money-Law Closure | S5,S9 | 1 (strengthened by 6) | [BLUEPRINT-P07](sovereign-roadmap-2026-07-16/BLUEPRINT-P07-money-law-closure.md) |
| 8 | Typed Local Observability | M8,S7,S8,D7,E46,E47,F29,F31,F32,F36,F39,F40 | 1 | [BLUEPRINT-P08](sovereign-roadmap-2026-07-16/BLUEPRINT-P08-typed-local-observability.md) |
| 9 | Confidential, Self-Healing Wire | M3,M6,M7,D2,E31-34,E38,F11-13,F15,F16,F18,F20,F22,F23,F25,F30 | 3, 4 | [BLUEPRINT-P09](sovereign-roadmap-2026-07-16/BLUEPRINT-P09-confidential-self-healing-wire.md) |
| 10 | Hub Runtime: Policy-as-Data, Kill-Switch, Boot | M5,M9,E37,F1,F2,F5,F8,F28 | 3, 6, 9 | [BLUEPRINT-P10](sovereign-roadmap-2026-07-16/BLUEPRINT-P10-hub-runtime-kill-switch-boot.md) |
| 11 | Compute Budget & Cache | E21-25,F33-35 | 1, 8 | [BLUEPRINT-P11](sovereign-roadmap-2026-07-16/BLUEPRINT-P11-compute-budget-cache.md) |
| 12 | Durable Storage, Deploy & Ops Floor | D1,S1,S2,E11,E26-30,E48-50,F37,F38 | 7 | [BLUEPRINT-P12](sovereign-roadmap-2026-07-16/BLUEPRINT-P12-durable-storage-ops-floor.md) |
| 13 | Delivery on Protocol | M1,M10,S4,E1,E39,F17,F41-43,F46,F50 | 4, 7, 9, 10 | [BLUEPRINT-P13](sovereign-roadmap-2026-07-16/BLUEPRINT-P13-delivery-on-protocol.md) |
| 14 | Dispute/Escrow + Per-Hub Graph-Wiki | E8,E16,E51,F44,F48 | 2 (HARD), 13 | [BLUEPRINT-P14](sovereign-roadmap-2026-07-16/BLUEPRINT-P14-dispute-escrow-graph-wiki.md) |
| 15 | Living Organism Unbounded | M11,E17,E18,F3,F4,F7,F9,F10,F27 | 10 (HARD), 5, 9 | [BLUEPRINT-P15](sovereign-roadmap-2026-07-16/BLUEPRINT-P15-living-organism-unbounded.md) |
| 16 | Product UI Rebuild | D4,E12,E41,E43,E44,F49 | 4, 13 | [BLUEPRINT-P16](sovereign-roadmap-2026-07-16/BLUEPRINT-P16-product-ui-rebuild.md) |
| 17 | Demo, Splat Tiers & GPU-Unlock Closure | E4,E45,F14,F47 | 11, 16, +GPU-unlock | [BLUEPRINT-P17](sovereign-roadmap-2026-07-16/BLUEPRINT-P17-demo-splat-gpu-unlock.md) |
| 18 | Public-Flip Readiness & Execution | D3,E5,E54,E59 | 1, 2 | [BLUEPRINT-P18](sovereign-roadmap-2026-07-16/BLUEPRINT-P18-public-flip-readiness.md) |
| 19 | Ecosystem Growth Engine | E6,E7,E56,E57,E60 | 18 | [BLUEPRINT-P19](sovereign-roadmap-2026-07-16/BLUEPRINT-P19-ecosystem-growth-engine.md) |

**No pre/post-MVP split exists in this table.** Every phase is a real build-dependency step toward
the same one architecture; "later" phases are later because something earlier must exist first
(crypto before wire, wire before hub-runtime, hub-runtime + money before the product rides the
protocol), never because a phase was deemed optional or deferred by preference.

### Waves (maximum parallelism)

```
WAVE 0 (start immediately, mutually independent): P1 P2 P3 P4 P5
WAVE 1: P6◄(1,3)   P7◄(1)        P8◄(1)         P18-prep◄(1,2)
WAVE 2: P9◄(3,4)              P11◄(1,8)       P12◄(7)
WAVE 3: P10◄(3,6,9)
WAVE 4: P13◄(4,7,9,10)
WAVE 5: P14◄(2,13)   P15◄(10,5,9)   P16◄(4,13)
WAVE 6: P19◄(18)                              P17◄(11,16,+GPU-unlock)
```

**Critical path: P3 → P9 → P10 → P13 → {P14, P16} → P17** (crypto correctness → confidential wire
→ hub runtime → delivery spine → dispute/UI → demo). P1 and P2 gate everything *epistemically*
(nothing downstream is trustworthy without them) but are cheap and fully parallel — start them
first regardless of what else is picked up. P5, P8, P11, P12, and P18-prep are off-critical-path
lanes that should be fanned out early to use idle capacity.

Full adjacency list: `R2-MERGED-PHASE-ROADMAP.md` §3.

**Wave-admission classification (added 2026-07-17, per Phase 25 —
[BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md](BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md)).**
The diagram above states *dependency* width; this note adds *resource* width, because this host is
4 physical cores × 2 SMT (8 vCPU, live `lscpu`), and the two kinds of parallel work in these waves
have different bounds:

- **Every phase is two-faced at execution time.** Its agent work (research, design, writing code
  text against a worktree) is **I/O-bound-dispatch** — lanes mostly blocked on the LLM API, CPU is
  not the bound; up to **D_max = 16** such lanes run concurrently (gated by memory PSI and the
  per-workflow `min(16, cores−2)` cap, not by core count). Its verification steps (each
  blueprint's `cargo build`/`cargo test`/bench done-check) are **CPU-bound-local** — bounded by
  **4 strict-core slots** (`taskset -c 0,2,4,6`, `nice 10`; one uncapped cargo build consumes all
  4). So WAVE 0's five phases genuinely CAN fan out five-wide (and wider) on the agent face, while
  their build/test done-checks queue through the shared 4-slot CPU budget — stagger the *builds*,
  never the *agents*.
- **Predominantly CPU-bound-local at verification:** P1 (kernel 337 + engine 47 tests), P3 (5
  bebop2 crates + CT tests), P4, P5, P6, P7, P8, P9, P10, P11, P12, P13, P24 (ring + criterion
  benches) — every phase whose done-check is a Rust build/test run, which the mandatory-benchmark
  doctrine (AGENTS.md) makes nearly all of them.
- **Predominantly I/O-bound-dispatch end to end:** P2 (canon repair — doc work), P18-prep, P14's
  design/ruling half, P20's doc/asset units, P22's blueprint-stage work, and all research/blueprint
  fan-outs (the 25-pass production run of this very roadmap was this class — it ran fine on 4
  physical cores because cores were never its bound).
- **Local-inference (third class, easy to mislabel):** P21's resident-agent runtime and P15
  E13-cpu (post-O18b) wait on local Ollama — that wait IS this host's CPU doing inference, so
  these count against the CPU budget (concurrency delegated to `OLLAMA_NUM_PARALLEL`, auto ≤ 4),
  never against the 16-lane dispatch budget.
- Admission is dynamic: a pure, local, µs-scale predicate over PSI/procfs (consuming P24's gauge
  surface — never a network call, per the LOCAL-DECISION rule). Full model, thresholds, and the
  proposed standing AGENTS.md rule: the Phase 25 blueprint.

---

## 3. Operator decisions required (19 items) — the real gate on Wave 3+

This roadmap intentionally does **not** resolve these. Each lives inside a numbered phase (that IS
its coverage — an anchor blocked on a ruling is not a dropped anchor), but real engineering on
several critical-path phases cannot proceed with full confidence past Phase 2 without them. Full
Descartes-quadrant treatment (options, tradeoffs, a flagged-overridable recommendation where one is
safe to offer) is in `BLUEPRINT-P02`. Highest-leverage first:

| # | Decision | Blocks | Stakes |
|---|---|---|---|
| O3 | **F44 dispute/escrow mechanism** — the only spec contradicts M12 (NO-COURIER-SCORING) and M6 (zero-dep) | Phase 14 | Not silently resolvable; candidates exist (operator-gated arbiter capability; staked Schelling voting) |
| O4 | **F48 merge semantics** — content-address-only vs. CRDT for per-hub graph sync | Phase 14 | A dormant `crdt-fence` guard was found to fence CRDT out of money/order code ONLY — genuinely open for the knowledge-wiki |
| O1 | **D5/D8** — define (candidates found in root `DECISIONS.md`, but on a *colliding* numbering scheme — adopting them accepts the collision explicitly) or renumber the whole D-series | Every "147"/"146" claim | Affects every anchor-count statement in every doc, including this one |
| O5 | **D2/iroh** — land the crate for real, or amend canon to "quinn primary + named unlock trigger" | Phase 9 | iroh does not exist in the codebase today; canon currently claims otherwise |
| O7 | **E1/F41 "hub-ring"** — ratify the consistent-hash reading (a literal star-hub would contradict M7 no-SPOF) | Phase 13 | Two words with no formal spec anywhere until ratified |
| O19 | **I-FINAL proof home** — bebop consensus path vs. dowiz `tools/eqc`; the file the old blueprint cited doesn't exist at either candidate location (it exists at a third, legacy path) | Phase 13 (F46 full closure) | Sharper than originally scoped — see BLUEPRINT-P13 |
| O9 | **V1-B verifier isolation bar** — fresh worktree vs. separate machine vs. different model family | Phase 6 | One sentence of canon closes this |
| O8 | **F10 max sub-hub recursion depth** | Phase 15 | Numeric value only |
| O2, O6, O10-O17 | Cheap/mechanical: E10≡E36 ratification, E35 "3-tier locality" definition, S7/S8 split, M12/F25 replay bound, "BD" expansion, M9 subtree-kill semantics, E13-20 numbering, canon rewordings, EUTM brand choice, public-flip go | Bookkeeping / Phases 8,9,10,14,15,18 | `BLUEPRINT-P02` offers a recommended default for each — operator can accept or override |
| O18 | **SPLIT 2026-07-16** (was one "GPU-unlock" trigger; a triple-confirmed self-critique pass — see §7 below — found this bundled two unrelated things): **O18a** `graphics-unlock` (network `cargo add wgpu` succeeding — verified RED/403 as of 2026-07-16) stays external/environment-gated; **O18b** `model-weights-unlock` (llama.cpp CPU tier — GGUF fetch + local server) is verified GREEN-ish on this host today and requires only a DECART report + operator go, **not** an external trigger | P17 (O18a only); P15 E13-gpu (O18a); P15 E13-cpu (O18b — actionable now) | Not cheap/mechanical — O18b is the single highest-leverage unblocked item in the whole roadmap right now |

**Practical read:** Waves 0-2 (Phases 1-5, 7, 8, 11, 12) need **no operator input** to start — pure
engineering against already-diagnosed gaps. Wave 3 onward (Phases 6, 9, 10, 13, 14, 15) benefit
from or hard-require O1, O3-O9 being ruled on. Getting O1/O3/O4/O5/O7/O9 answered early is the
single highest-leverage non-engineering action available — it unblocks the entire critical path's
back half without costing any engineering time.

---

## 4. Anchor accounting — zero exceptions, proven not asserted

- Nominal canon count: **147** (M1-12 + V1-6 + D1-8 + S1-9 + E1-62 + F1-50).
- **E10 ≡ E36** — identical text ("ML-DSA hybrid") at two different anchor numbers, re-verified
  against `STRATEGIC-VECTORS-LOCKED-2026-07-16.md:89,98`. Counted once; E36 recorded as alias. → 146.
- **D5, D8 undefined** in every revision of both canon documents (git-archaeology-verified across
  all four introducing/amending commits, independently re-checked in Round 2). Carried as
  **operator-decision placeholders inside Phase 2** — that is their coverage, not a gap in this
  roadmap. → **146 distinct IDs: 144 defined + 2 placeholders**, all 147 nominal IDs accounted for.
- **One seam anchor** (E16, "spectral+BD memory") was not claimed by any of the five Round-1
  clusters; the merge pass assigned it explicitly to Phase 14 rather than let it drop silently.
- Full per-anchor → phase mapping (all ~147 IDs, one row per series): `R2-MERGED-PHASE-ROADMAP.md` §5.

**Zero anchors deferred outside a phase. Zero anchors silently dropped.** This claim is checkable,
not asserted — the accounting table it rests on is reproducible by grep against the two canon
documents plus the phase table above.

---

## 5. What this roadmap is not

- **Not an implementation.** Every one of the 19 blueprints is explicitly a planning document —
  none of the 25 research/design passes that produced this roadmap wrote or edited product code,
  CI config, or canon files. Turning a blueprint into a merged diff is the next, separate unit of
  work per phase.
- **Not a canon edit.** The corrections in §1 items 6-7 and the full decision docket in §3 are
  proposals (concretely, `BLUEPRINT-P02`'s diff) for the operator to merge into `ARCHITECTURE.md`
  by hand or by explicit delegation — this document does not touch that file, honoring its own
  "merge, never append" rule.
- **Not a re-prioritization of the architecture.** No anchor was judged more or less important than
  another; phase ORDER here reflects only build-dependency reality (what must exist before what),
  never a value judgment about which parts of the architecture matter more.

## 6. Next steps

1. Operator rules on O1, O3, O4, O5, O7, O9 (the ones that actually gate engineering, per §3) —
   everything else in the decision docket has a safe default already proposed in `BLUEPRINT-P02`.
2. Wave 0 (Phases 1-5) starts immediately — no ruling required, each is independently actionable
   from its blueprint today.
3. `BLUEPRINT-P02`'s canon-diff gets merged into `ARCHITECTURE.md` (the LICENSE/scrub/DCO
   corrections in particular are cheap, checkable, and currently make the canon assert three false
   things).
4. As each phase's implementation lands, its blueprint's falsifiable done-test is the closure
   criterion — not a subjective "looks done."

---

## 7. Follow-up pass (2026-07-16, same day) — self-critique + harness/LLM-infra research

Per the operator's standing session-closing ritual (now codified in `AGENTS.md` — the 2-question
doubt check), this roadmap was immediately subjected to an independent adversarial review, plus two
research passes the operator requested on the agent harness itself. All in
`sovereign-roadmap-2026-07-16/`:

- **[SELF-CRITIQUE-2Q-DOUBT-AUDIT.md](sovereign-roadmap-2026-07-16/SELF-CRITIQUE-2Q-DOUBT-AUDIT.md)**
  — a decorrelated Opus pass answering "what are you least confident about" (7 items, each
  investigated to a verdict, not just listed) and "what's the biggest thing missing." **Two items hit
  the big-deal threshold**, both confirmed by live probes, not just doubted: (1) Phases 5/15 gated
  ALL self-hosted LLM execution on an external "GPU-unlock" trigger, but llama.cpp is CPU-first by
  design and needs no GPU — this was a real category error, now corrected (§3 of this document, and
  in `BLUEPRINT-P05`/`BLUEPRINT-P15`/`R2-MERGED-PHASE-ROADMAP.md` directly). (2) The critical path
  (P3→P9→P10→P13→...) makes the quantum-mesh substrate load-bearing-first, while G11 ("first real
  order" — the only proof the product is wanted) sits as a late done-test rather than a Wave-0 gate —
  **this is flagged, not resolved**; it is an operator-level charter question this roadmap does not
  prejudge.
- **[HARNESS-RESEARCH-revfactory-and-agentic-teams.md](sovereign-roadmap-2026-07-16/HARNESS-RESEARCH-revfactory-and-agentic-teams.md)**
  — dowiz's telemetry *consumers* (EV/Kelly model routing, false-claim meter) are already ahead of
  the open-source agent-team-harness genre (incl. `revfactory/harness`, 8.2k★), but starving on
  hand-written data (9 rows in `track_record.jsonl`).
- **[LLM-INFRA-RESEARCH-wandr-vllm-llamacpp.md](sovereign-roadmap-2026-07-16/LLM-INFRA-RESEARCH-wandr-vllm-llamacpp.md)**
  — independently reaches the same llama.cpp/GPU-gating verdict, with a concrete `LlmBackend`
  Trait-as-Port design (managed-API default, llama.cpp/vLLM as hub-chosen adapters, per M5).
- **[HARNESS-IMPROVEMENT-SYNTHESIS-PLAN.md](sovereign-roadmap-2026-07-16/HARNESS-IMPROVEMENT-SYNTHESIS-PLAN.md)**
  — the combined, sequenced fix: **H1** auto-harvest the governance ledgers from session transcripts
  (cheapest, highest leverage — zero new deps), **H2** sovereign per-repo Telegram coverage for
  dowiz/openbebop/hermes (contract-shared, code-independent — cross-repo calls explicitly rejected as
  a central-SPOF shape), **H3** the `LlmBackend` port + operator-gated llama.cpp Tier-1 rollout.
- **[OPEN-SOURCE-CREDITS-LIST.md](sovereign-roadmap-2026-07-16/OPEN-SOURCE-CREDITS-LIST.md)** — every
  repo/service/tool integrated, borrowed from, or reverse-engineered across dowiz + bebop-repo +
  hermes-kernel's full history (~150 active dependencies, 32 design influences, 21 specs/standards,
  26 evaluated-and-rejected), for the operator to star/credit.

`BLUEPRINT-P05` §8, `BLUEPRINT-P15` §9/§10, and `R2-MERGED-PHASE-ROADMAP.md`'s O18 row / Phase-5 /
Phase-15 rows / dependency graph have already been corrected to reflect the O18 split (E13-cpu vs.
E13-gpu). `ARCHITECTURE.md:34` still needs the operator's canon-merge pass (per `BLUEPRINT-P02`'s
mechanism) — not edited here, same boundary as the rest of this roadmap.

---

## 8. Second follow-up pass (2026-07-17) — four new phases, one cross-phase addendum, a
## completeness audit, native-cleanup tracking

Same rule as §7: every claim below is either re-derived from live code/tests this session or
named as an open decision, never asserted from a prior doc's authority.

### 8.1 Four new phases (P20–P23)

None of these existed in the 19-phase table in §2. Each has an execution-grade blueprint already
written (research + DECART + 2-question doubt audit + Anu/Ananke check, same protocol as P01–P19).
Adding them here is bookkeeping, not new design work — the design already exists in the cited file.

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 20 | Demo & Marketing Pipeline Refactor | [DEMO-MARKETING-PIPELINE-REFACTOR-2026-07-17.md](DEMO-MARKETING-PIPELINE-REFACTOR-2026-07-17.md) | 7 (its DM-2 offer-redemption ledger hard-depends on P07's replay-dedup fix), 18 (all publication gated behind public-flip, mirroring P19's own boundary) | 7 work units (DM-1..DM-7); no new crate, reuses engine `compose` + a committed glyph atlas |
| 21 | Local AI / Local Agents (resident-agent plane) | [LOCAL-AI-LOCAL-AGENTS-RESEARCH-2026-07-17.md](LOCAL-AI-LOCAL-AGENTS-RESEARCH-2026-07-17.md) | 5 (routing organism, done) | Extends the already-shipped `LlmBackend`/Ollama port (harness-2026-07-16) with a plan→act→observe loop; zero new external deps per its own DECART; shares sequencing with the agentic-mesh arc (separate branch) but does not depend on it landing |
| 22 | Multi-Platform Social Auto-Posting | [BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md](BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md) | 1 (CI floor) | New `SocialPoster` port mirroring the `LlmBackend` port pattern exactly; Wave 0 = Telegram (no operator decision needed); Wave 1 (Viber) blocked on **O-SOC-1** (public media-hosting location); Wave 2 (Meta) gated on its own approval-process calendar, not a build dependency |
| 23 | Device Auth + 2FA | [BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md](BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md) | none for its P1 (a zero-dep `totp.rs` primitive, buildable today); **P3 (full HTTP wiring) depends on a dynamic admin HTTP surface that does not exist anywhere in this roadmap yet** — a real gap, named here rather than assumed away. Native Rust primitive chosen over Better Auth because the JS/TS stack (Better Auth's runtime) is fully deleted from `origin/main`, not merely paused |

**Wave placement:** P20–P23 are Wave-0-eligible in the same sense P1–P5 are (P21's `totp.rs` and
P22's Telegram adapter need no operator ruling; P20 and P23 have a named phase dependency but no
operator-decision dependency). None of them sit on the P3→P9→P10→P13 critical path (§2); they are
off-critical-path lanes exactly like P5/P8/P11/P12/P18-prep, fan out whenever capacity is idle.

### 8.2 One cross-phase addendum, not a new phase — Hub design vs. vendor market research

[HUB-DESIGN-VENDOR-MARKET-RESEARCH-2026-07-17.md](HUB-DESIGN-VENDOR-MARKET-RESEARCH-2026-07-17.md)
compared dowiz's hub design against general vendor-facing delivery-platform market patterns
(menu-as-data, external-channel bridging, multi-fleet dispatch, store-state/kitchen-load modeling,
multi-location semantics) and found six gaps (G1–G6). **This is deliberately not phase 24** — every
gap-closing item (D1–D6) extends an *existing* phase (10, 13, 15, 16) rather than standing alone, per
the addendum's own scope decision (no new phase, no new mechanism, every fix reuses something already
in the roadmap). One new operator decision came out of it:

| # | Decision | Blocks | Stakes |
|---|---|---|---|
| O20 | **Multi-location semantics** — is a "location" a sub-hub (P15 agent-recursion) or a flat intra-hub row? The legacy schema had organizations→locations; the mesh model never ruled this | Phase 16 (multi-location UI), the D6 addendum | Recommended default: intra-hub row for v1, sub-hub-as-target for later scale — flagged overridable, not forced |

The doc's own counterweight, restated because it is easy to lose in a gap list: dowiz is **ahead** of
every platform it was compared against on offline resilience, customer-data ownership, zero take-rate
economics, and courier dignity (NO-COURIER-SCORING is a genuine differentiator, not a compliance
cost). The gaps are real; so is the lead.

### 8.3 Roadmap completeness audit (2026-07-17)

A dedicated pass re-verified every blueprint in this roadmap plus H1–H4 (hermetic-remediation) and
B1/B3/B4/E1/E2 (the agentic-mesh and spectral-evolution arcs, separate worktrees) against live
code/tests, and appended a "Planning-protocol completion appendix" to 25 files where the blueprint's
own claims had drifted from what is actually built — always append-only, nothing rewritten, nothing
committed by the audit itself. Two findings apply across the whole roadmap, not to any one phase:

1. **"Landed" is branch-implicit.** The same claim ("still open" vs. "already built") can be true or
   false depending on which branch/worktree is checked out — confirmed independently on P07's dedup
   fix, B3's `TokenBucket::release`, and B4's crypto bench, each of which exists only on its own
   feature branch. Every appendix now names its branch explicitly; a reader who doesn't check the
   branch is the single most common way this roadmap goes stale in practice.
2. **The dominant staleness direction is "assumed-unbuilt, actually built."** Nine blueprints (P01,
   P07, P08, P12, H1, H2, E1, E2, and P06's own `v1-verify` gate contract — see §8.4) had real
   implementation land after their blueprint's evidence pass, with no header update recording it.
   The fix in each case was appending the landed state, not rewriting the design.

Two operator-decision items surfaced by this pass are not yet in §3's table and are added here:
**O2b** (P14's reproduced F2 dispute table silently dropped a phrase load-bearing to Contradiction A —
needs a re-derivation, not a ruling) and a note that **O18** and **O19**'s "resolved" status in three
2026-07-16 docs contradicts MEMORY.md's own record of them as still-open blockers — named, not
adjudicated, exactly per this roadmap's own rule of recording contradictions rather than picking a
side silently.

### 8.4 P06's merge-gate contract is now executable (found during the audit, not separately built)

`tools/ci-truth/src/v1.rs` implements BLUEPRINT-P06 §2–§5's anchor loader, TLV
encode/decode, and merge-gate policy as a real, tested Rust module — with signing
behind an explicit `Signer` trait whose only production implementation
(`UnsignedSigner`) honestly emits `"signed":false`, exactly mirroring
`main.rs:423`'s existing placeholder. This is *not* a violation of P06's own hard
precondition ("no signing until Phase 3 closes C4b on `mod_l`") — it contains no
signing, only the policy the signing eventually plugs into.

STATUS (2026-07-17):
- (a) **DONE** — the module now has a `#[cfg(test)]` suite (8 contract tests:
  TLV round-trip, K≠V load invariant, and the §5 merge-gate policy covering the 3
  mandated RED cases — missing attestation note, key_K==key_V self-sign, residue
  missing — plus red-line-touch honesty and GREEN-required-on-red-line). `ci-truth
  v1-verify <sha>` is wired and runnable; verified 27/27 ci-truth tests green,
  0 warnings. The contract is now falsifiable by this roadmap's own bar.
- (b) **OPEN (operator-gated)** — per P06-EXECUTION-PLAN-2026-07-17.md §2, the
  dowiz-side verifier still needs a *real* Ed25519/ML-DSA-65 verify-only
  implementation behind this contract. `v1.rs`'s `digest32` is explicitly a
  placeholder (`git hash-object`, not sha3-256), named as such in its own comment —
  not a finished crypto primitive. The `Signer` trait slot is left open for the
  bebop2 hybrid (Ed25519⊕ML-DSA) implementation that lands after Phase 3 closes C4b.
  Until then, `v1-verify` correctly emits RED on any commit lacking the two git
  notes, which is the honest Phase-1 behavior.

### 8.5 Native-only cleanup — tracked, not fully executed

Per the operator's standing direction (no Python/Node runtime code outside adapters/bridges): this
session deleted genuinely dead artifacts (14 one-off `audit/*.py` scripts that manually poked
`apps/api` endpoints deleted with the rest of the JS/TS stack; two stale root-level duplicates of
`eval-layer/{metrics,openrouter_judge,eval_runs}.py`; an unused `.venv-paddle/` OCR experiment
directory). **Not deleted, and why:**

- `tools/eqc/eqc.py` — actively wired into `.github/workflows/ci.yml`'s `eqc-proofs` job. Deleting it
  without a Rust replacement would break CI. Tracked as a named follow-up: port to Rust under Phase 1
  (CI Truth Floor) once someone picks it up — not silently left, not silently deleted.
- `tools/skillspector-rs/gen_rules.py` and `tools/skillspector/src/skillspector/`'s Python source —
  this is a legitimate bridge, not a dinosaur: `gen_rules.py` parses the Python analyzer source as its
  "source of truth" (its own comment) and generates `skillspector-rs`'s `rules.rs` from it. The Python
  never runs in production; it is a code-generation input, the exact "adapter, not runtime" exception
  the operator's own direction allows.
- `tools/loop-signals/transcript_events.py`, `tools/telemetry/test_ser.py`, `kernel/benches/bench_track.py`
  — not re-audited this pass; flagged here so they are a known open question, not an assumed-clean
  item.

### 8.6 One more phase (P24) — native runtime telemetry (2026-07-17, same protocol as §8.1)

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 24 | Native Runtime Telemetry — ring-buffer flight recorder + explainable latency events | [BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17.md](BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17.md) | 8 (consumes P08's typed-schema/local-sink design and generalizes P08 §4's claim-latency anomaly pattern from CI commit latency to any runtime latency — decided from P08's own text, not assumed; P24 re-owns none of P08's anchors) | Linux-technique port (procfs snapshot-of-byproduct counters, perf/eBPF sample+aggregate-in-place, SPSC acquire/release rings, PSI cause attribution, RRD max-preserving tiers); one new kernel module (`ring.rs`, SPSC-no-CAS per the RCI H1 lesson), zero new external deps; every anomaly logged as an explained capsule (baseline+rule+PSI+prelude), never a bare "spike detected". Off-critical-path lane like P5/P8/P11/P12 |

### 8.7 One more phase (P25) — wave scheduling & resource-classed admission (2026-07-17, same protocol as §8.1)

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 25 | Wave Scheduling & Concurrent Agentic Execution — resource-classed admission control | [BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md](BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md) | 24 (soft — consumes P24 W1b's PSI-extended gauge surface as the admission signal; pre-P24 the same `/proc/pressure` files are read directly) | Corrects the "8 cores" premise (live `lscpu`: 4 physical cores × 2 SMT); splits all wave work into CPU-bound-local (4 strict-core slots, `taskset -c 0,2,4,6`, `nice 10`), I/O-bound-dispatch (D_max = 16 default — C10K/work-stealing grounding: lanes blocked on LLM API don't occupy cores; bound is memory-per-agent + API limits), and local-inference (Ollama = CPU load, delegated to `OLLAMA_NUM_PARALLEL`). Two binding operator rules named: LOCAL-DECISION (admission computed natively from local procfs/PSI state, µs-scale, never a network round-trip) and CORE-BOUND (CPU work on real cores only by default). Retroactive wave classification appended to §2; proposed AGENTS.md standing rule in blueprint §6 (operator merges, not applied). One proposed pure module `kernel/src/admission.rs`; zero new external deps. Off-critical-path lane like P5/P8/P11/P12/P24 |

### 8.8 One more phase (P26) — memory optimization & flow analysis (2026-07-17, same protocol as §8.1)

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 26 | Memory Optimization & Flow Analysis — raising the D_max ceiling | [BLUEPRINT-MEMORY-OPTIMIZATION-FLOW-ANALYSIS-2026-07-17.md](BLUEPRINT-MEMORY-OPTIMIZATION-FLOW-ANALYSIS-2026-07-17.md) | 25 (consumes its D_max formula/admission predicate as the integration point; most units startable now) + 24 (soft — VmRSS/PSI gauges) | Web-grounded memory research applied to P25's memory-bound concurrency ceiling. ADOPTED: `kernel/src/memory_budget.rs` (`MemoryBudget`, TokenBucket's byte-budget sibling — reserve/release, no time-refill), byte-bounded LRU cache store behind the existing `BlockStore` trait (the exact-match LLM cache is unbounded today), `FileBlockStore` index-not-mirror (its `open` currently loads the whole store into RSS), and `retrieval/ppr.rs` dense→CSR delegation (O(n²)→O(nnz), removes a dual authority — the genuine reuse of the existing sparse machinery). KEPT: system allocator (no `#[global_allocator]` exists; Rust 1.32 precedent; measured trigger + `MALLOC_ARENA_MAX` fallback named). REJECTED honestly: bumpalo/hand-rolled arenas (ns saved vs 10-second network waits), PPR/graph-scored cache eviction (no production lineage, no entry graph exists), Tucker/CP/tensor-train (no embedding matrix exists), ARC (patent history, unneeded adaptivity). DEFERRED with named triggers: int8 embedding quantization (4×/~99% retention, fetched numbers — trigger: Layer-B index >100 MB), W-TinyLFU admission sketch (trigger: >10⁴ entries + measured hit-rate loss). Net effect: D_max's `MEM_PER_AGENT` becomes measured+enforced (`try_reserve` per lane) instead of an estimate — the mechanism behind P25's "raiseable to 24+". Zero new external deps. Off-critical-path lane like P5/P8/P11/P12/P24/P25 |

### 8.9 One more phase (P27) — fault-isolated decentralized architecture (2026-07-17, same protocol as §8.1)

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 27 | Fault-Isolated Decentralized Architecture — audit closure + circuit-breaker/bulkhead/supervision discipline | [BLUEPRINT-FAULT-ISOLATION-DECENTRALIZED-ARCHITECTURE-2026-07-17.md](BLUEPRINT-FAULT-ISOLATION-DECENTRALIZED-ARCHITECTURE-2026-07-17.md) | none hard; soft: 24 (breaker-snapshot surface), 26 (owns the A3 cache-cap fix — convergent double-detection recorded in both), GapWire W1 (transition→GapEvent wiring), 9/10 (per-peer breakers, not schedulable yet) | Two-part artifact: (1) a ranked 16-finding stability/performance audit of kernel/engine/tools/llm-adapters (worst: A1 head-of-line blocking wedges the live Telegram alerting pipeline forever — `rust-spool/src/main.rs:240-247` retries the queue head with no send-failure deadletter; also A2 `FileBlockStore::put` panics on disk I/O via an infallible port, A4 the Dispatcher's `workers` bound is dead code, A6 zero compaction on every append-only store with `metric.jsonl` at 2.7 MB growing live); (2) one new primitive `kernel/src/breaker.rs` (`CircuitBreaker`, `TokenBucket`'s failure-exposure sibling: EMA trip filter via `geo.rs::ema_next` + min-calls floor, Open/HalfOpen hysteresis, transition-only event emission) + bulkheads at audited seams + OTP-grade restart-intensity policy for drainers + the "every port is fallible" rule. Research grounded in Armstrong/OTP supervision, Fowler breaker, Hystrix-deprecation lesson, reliability block algebra (series→parallel under verified independence), RFC 6298 EWMA + φ-accrual (deferred, named triggers). Proposes an AGENTS.md "Fault Containment" standing section (§6 there, operator merges) tied to the existing `.specify`/openspec SDD pipeline. Zero new external deps. Off-critical-path lane like P5/P8/P11/P12/P24/P25/P26 |

### 8.10 One more phase (P28) — cache reference graph + hybrid tensor decomposition + bump arena (2026-07-17, same protocol as §8.1; OVERRIDES three P26 verdicts by explicit operator direction)

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 28 | Cache Reference Graph + Hybrid Tensor Decomposition + Bump Arena — living-memory pattern applied to the LLM cache | [BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md](BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md) | 26 (W3/W4 consume its M2 `BoundedStore` eviction seam; overrides its §0.3/§0.7/§0.8 verdicts — P26 carries a dated addendum pointing here); soft: Layer B (`HARNESS-LLM-BACKEND.md` §3.3 — semantic edges + tensor rung 3 activate when it builds), 24 (bench/telemetry surfaces) | Operator-directed override of P26's three rejects, planned forward. (1) `CacheGraph` (`llm-adapters/src/cache_graph.rs`): node = existing sha3-256 cache key interned by insertion order (chronology); edges = co-access (v1, sliding window, aggregation free via `Csr::from_edges` duplicate-summing), derivation (v1.1, `derived_from` provenance), semantic (at Layer B); query = existing deterministic `personalized_pagerank` seeded from recents (recall) → PPR-primary/LRU-tie-break eviction scorer with a replay A/B falsifier vs plain LRU — the living-memory blueprint §7 "cache prefetch" layer instantiated, HippoRAG-precedented (NeurIPS 2024). (2) Hybrid tensor ladder: (entry × entry × relation) tensor per RESCAL (X_k ≈ A·R_k·Aᵀ, ICML 2011) coupled with an (entry × feature) matrix (embeddings + PPR/degree/recency) per CMTF (Acar–Kolda–Dunlavy 2011); rung 1 buildable NOW = new deterministic `kernel/src/lowrank.rs` (fixed-K power iteration + deflation over existing `Csr::spmv`, fills `spectral_cache::Decomp`'s empty basis slot — `spectral.rs` is eigenvalues-only, live-verified); SQ/PQ quantization stays the complementary rung-4 track. (3) `kernel/src/arena.rs` `BumpArena` (zero-dep, `Vec<u8>` region, O(1) reset, `T: Copy`, degrade-closed heap fallback) at the graph/spectral rebuild site (≈2n+7 allocs/CSR rebuild, ≈n²+O(n)/dense charpoly call), claim stated on its own terms: ~2k malloc/free pairs → ≤8 bumps + 1 reset per pass, criterion A/B + Miri + byte-identical-output falsifiers. Zero new external deps. Off-critical-path lane like P5/P8/P11/P12/P24–P27 |

### 8.11 One more phase (P29) — latency elimination: Decision Compiler + measured latency levers (2026-07-17, same protocol as §8.1)

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 29 | Latency Elimination — Decision Compiler (LLM-as-one-time-compiler → native DecisionUnits) + measured dispatch-latency levers | [BLUEPRINT-LATENCY-ELIMINATION-RESEARCH-AND-BRAINSTORM-2026-07-17.md](BLUEPRINT-LATENCY-ELIMINATION-RESEARCH-AND-BRAINSTORM-2026-07-17.md) | GapWire/orchestrator arc (soft — DecisionUnit `Stale`-on-`GapEvent` invalidation rides its `triage` routing; the pilot can land advisory-only before it); 25 (soft — the cache-write stagger lands at its admission point); 21/G3 (soft — the draft-local/verify-remote trial shares its small-model precondition P-2) | Measured ground truth from today's own 1,000 API calls: p50 4.9 s / mean 10.6 s / p90 26.2 s per call, **99.3% prompt-cache-read already**, avg 1,232 output tokens ⇒ decode volume dominates (85–90%), network ≤1–2 s ceiling — so the operator's local-AI hypothesis inverts (local 4.8–10.5 tok/s is 5–15× slower than API decode) and the operator-ruled primary is the **Decision Compiler**: recurring question *shapes* compiled ONCE by the LLM into tested, provenance-stamped native Rust `DecisionUnit`s (`Decision::{Answer,Escalate}`, degrade-closed), invalidated by GapWire events (`skillspector-rs` `build.rs` rerun-if-changed semantics promoted to runtime), never self-certified (independent replay per RC-2/P7; red-line shapes operator-gated). Four in-repo precedents cited as proof of pattern (`is_redline` ci-truth `main.rs:237`, mesh `scope.rs:244`, hermes `gov_route` EV table, skillspector rules pipeline). Pilot = shape C1 model-tier routing (<1 µs decide, 30-case fixture vs policy v3.4, Stale-path test). Secondary adopt-now levers: output-token discipline + `effort: low` doer lanes, wave-dispatch cache-write stagger, 1-h TTL/pre-warm, doc-edit/wave separation (AGENTS.md rule — operator merges). Distillation deferred with named trigger; mesh cache-sharing filed to B2. Speculative section (S1–S8) kept clearly non-decided. Zero new external deps. Off-critical-path lane like P5/P8/P11/P12/P24–P28 |

### 8.12 One more phase (P30) — Bebop2 mesh masterwork synthesis (2026-07-17, same protocol as §8.1; consolidates the 9-batch tensor/state/safety/consensus/network/equations/product audit)

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 30 | Bebop2 Mesh Masterwork — 9-batch synthesis: equations-first kernel organs, exactly-once/hysteresis correctness closure, arena/breaker/eigenvector substrate, capability-Sybil-proof mesh composition, staged product→kernel migration | [bebop2-mesh-tensor-hermetic-2026-07-17/BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS.md](bebop2-mesh-tensor-hermetic-2026-07-17/BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS.md) (synthesis over batches 10–18 in the same directory; `INDEX.md` there navigates) | **None hard for Waves 1–2** (startable now: eqc-rs→`geo.rs ema_next` wiring, `event_log.rs:359` `append_raw` exactly-once port — a LIVE money-red-line bug on this branch, `hydra.rs` hysteresis, `order_machine` ρ=0 const, householder eig2x2 dedup, wasm-boundary clamps, then arena.rs/breaker.rs/eigenvector-R1-R3/gossip-epoch). **P06 (key_V)** — the standing 3-way blocker gains a 4th consumer: the DecisionUnit *signed* import-verdict form and the tamper-leg closure both plug into P06's `Signer` slot (unsigned local-replay import gate builds earlier; synthesis §6). **P28** — co-owned substrate: P30 W2 *builds* P28's `arena.rs` and rung-1 solver per the eigenvector-refactor plan (no second arena, no lowrank.rs). **P29** — design authority for DecisionUnit gossip (= Decision Compiler; P30 adds only epoch/import-gate/rollback-in-same-log). **RLS NOBYPASSRLS** (`docs/ops/P8-NOBYPASSRLS-FLAG.md`) — a SEPARATE parallel workstream, never folded in; it hard-gates only the W4 product T4 write-path lane. New operator rulings docketed: **R-1** 0x12→0x13 discriminant, **R-2** budget-unit semantics, **R-3** `RootDelegationPolicy`, **R-4** money-law eqc flip + S2 integer basis-points (+ optional C8 bilateral-memory flag). Operator verdicts applied as binding: Sybil-proof via asymmetric anchor-rooted capability issuance (Batch 7 PROVEN-VIABLE — Cheng–Friedman's own asymmetric escape class; `verify_chain` already implements it), reputation/scoring/watchdogs/proxies rejected on physics+red-line. Verdict ledger: 14 ADOPT (+4 gated), 10 EXTEND, 17 ALREADY-EQUIVALENT, 16 DEFER-with-numeric-trigger, 19 REJECT-on-physics; zero concepts dropped (Batch 6 §5.1 completeness sweep is the spine). Zero new external deps in Waves 1–2. Off-critical-path lane structure like P5/P8/P11/P12/P24–P29, but W1-L2 (exactly-once port) is a correctness red-line item and should not idle |

---

## 9. Consolidation pass (2026-07-17, Layer I) — the altitude axis, the master index, and the
## would-be-lost ledger

Appended by the CORE-ROADMAP Wave-3 consolidation pass (same append-only rule as §7/§8). Full
execution detail: `CORE-ROADMAP-2026-07-17/BLUEPRINT-P-I-consolidation.md`; ground-truth audit:
`CORE-ROADMAP-2026-07-17/P-I-audit-cross-repo-consolidation.md`.

### 9.1 The Layer A–I altitude axis and the master index

The numeric phases **P01–P30 in this document remain the sole execution numbering — nothing is
renumbered.** The CORE-ROADMAP effort's letter groupings are ratified as **`Layer A..I`** (the
former "P-A..P-I" spelling is retired from prose to kill the P-D/P04 lexical collision; on-disk
`BLUEPRINT-P-X-*.md` filenames keep their provenance names). Each Layer is an **altitude lens over
a cluster of numeric phases**, not a phase itself — the full crosswalk table (Layer ↔ numeric
phases ↔ blueprints ↔ arcs) and the navigation map of the whole planning corpus live in
**[`CORE-ROADMAP-INDEX.md`](CORE-ROADMAP-INDEX.md)**, which is this roadmap's companion index
(this doc stays the canonical WHAT/WHEN; the index is the canonical WHERE). The Layer A–I axis
descends directly from `MASTER-EXECUTION-PLAN-2026-07-13.md`'s ground→core→surface→platform
spine — lineage stated, not re-derived.

The five older master docs (`MVP-2026-07-12` root, `BUILD-SEQUENCE-UPDATED-2026-07-11`,
`INTEGRATION-PLAN-2026-07-14`, `10-PHASES-2026-07-14`, `EXECUTION-PLAN-2026-07-13`) now carry
SUPERSEDED banners pointing here — preserved in full as audit trail, never planned against.

### 9.2 Would-be-lost ledger (P-I audit §3) — all six dispositioned, zero silent drops

| ID | Item | Disposition (executed 2026-07-17) |
|---|---|---|
| L1 | Update-blob **code-signing** (ML-DSA verify vs pinned root; `kernel/src/pq/codesign.rs` is live on this branch) | Folded into **Phase 10** — boot/update-integrity unit note appended to `BLUEPRINT-P10` |
| L2 | Transport bake-off rationale (Zenoh/Reticulum/TCPCLv4/BIBE; libp2p rejected) | `docs/transport-research-2026-07-12.md` **restored from git blob `94e257fe9`** + cross-linked from `BLUEPRINT-P09` |
| L3 | Courier out-of-app notification/wake path (`NotifyHub`/VAPID lineage) | Folded into **Phase 13** — delivery semantics dissolved-by-mesh (courier node receives `MeshEvent`s directly); out-of-band device-wake kept as a P13 sub-unit, xref P08 alerting |
| L4 | Anonymous `.onion`/Tor tier | **E53-form waiver** — what: anonymity/Tor access tier; why-suspended: no vendor-node tier exists and no anonymity demand demonstrated; trigger to revisit: vendor-node tier ships AND a venue requires anonymity |
| L5 | "Lost reports" honesty ledger (13 + ~20 pre-2026-07-12 reports) | Closed-as-lost, decisions survive in `UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3`; not resurrected (would violate ground-truth discipline) |
| L6 | Self-development research queue (causal/do-operator, category-theory functorial mapping, info-geometry, integer laws) | **Deliberately NOT a numeric phase** — separate always-running axis; indexed from `CORE-ROADMAP-INDEX.md` → MEMORY `physics-math-exploration.md` |

---


## 10. Ecosystem-Component Consolidation (2026-07-18) — the second axis, the swarm-ready phase
## set, and the critical path to first-deployable dowiz

Appended by the 2026-07-18 ecosystem-consolidation pass (same append-only rule as §7/§8/§9).
Companion index registration: `CORE-ROADMAP-INDEX.md` §0 (component axis) and §9 (orphaned-arc
absorption table).

### 10.0 Why this section exists

The operator found ~12 separate planning arcs (150+ blueprint units: field-ui-engine,
dowiz-interfaces, rust-engine-rewrite, integration-ports, ecosystem-strategy, ops-reliability,
mesh-real, docker-swap, math-first-architecture, hydraulic-loop-v2) scattered outside canonical
navigation — several with ZERO reference from `CORE-ROADMAP-INDEX.md`, some carrying stale or
false "already done" claims. The operator's instruction, verbatim: **"один роадмап, не декілька…
нічого не загубити, нічого не добавляти що не критично… чітким розподілом на частини
екосистеми"** — ONE roadmap, nothing lost, nothing added beyond the critical, clearly divided by
ecosystem part, organized so a swarm can execute waves WITHOUT re-deriving anything
(self-sufficient DoD/anti-scope per phase), with the product supporting three operating modes
(no-AI / local-offline-AI / connected-AI) and the core delivery flow (orders/courier/money) never
requiring AI at all. Every claim in §10.1–10.5 was live-verified against running code/tests on
2026-07-18 by parallel research passes before being written here — not inherited from any arc
doc's own claims, several of which turned out to be false or stale at the source.

**Integrity-check notes (assembly pass 2026-07-18 — reported, not silently fixed):**

- Phase headings P31–P46: every number present exactly once across §10.5 — no gaps, no
  duplicates. Two structural notes: **P38** exists only as the deliberate split **P38a/P38b** (no
  bare P38 heading; rolled up as one P38 row in §10.2), and PROTOCOL numbered its second phase
  **P34B** (capital-letter sub-phase of P34 — same sub-letter convention as P31a/P38a, rolled
  into the P34 row in §10.2).
- Two source units are deliberately split in half across two phases, each half explicit at both
  ends (intentional, NOT double-counting): **MESH-09** (BPv7 half → P34; iroh half → P34B) and
  **P21** (executor half → P40; mode/degradation half → P41).
- Units with NO absorbing phase found (open audit items — deliberately NOT invented into scope
  here): **MESH-14** (resolve-contradictions + RED-suite + status-from-live-test CI-lint —
  mesh-real's 14th unit, confirmed present in `mesh-real/BLUEPRINTS-MESH-REAL.md` but unnamed by
  any §10.5 draft) and **IP-21**; additionally **IP-01..07/09/17/18** are asserted by §10.5.5's
  absorption ledger to be PROTOCOL/AGENT/CORE scope, but only IP-08 is explicitly absorbed
  (→ P42). Disposition of these belongs to a P33b-style follow-up audit, not to this assembly.

### 10.1 The Ecosystem-Component axis — a THIRD lens

Orthogonal to BOTH the P01–P46 numeric phases AND the Layer A–I altitude axis (§9.1) — a third
navigation lens, not a replacement for either. Row order below IS the critical path.

| Component | Mission | Owns | Position on critical path | Current completion (live-verified 2026-07-18) |
|---|---|---|---|---|
| **CORE** | decide/fold Law, event-log, money, capability primitives, spectral math, self-tuning control loops | P31–P33 | 1st — the foundation, already sufficient for everything downstream | ~90% done, not the bottleneck; dominant failure mode is built-but-unwired code |
| **PROTOCOL** | mesh, capability issuance, crypto, transport, delivery-domain (bebop2) | P34–P36 | 2nd — THE wiring lever | ~70% built and PROVEN but 100% stranded from dowiz's own kernel — the single biggest lever in the whole roadmap |
| **DELIVERY** | the dowiz product surface: UI, order/courier/payment flow, auth, demo/marketing, app-shell | P37–P39 | 3rd — the first-deployable target | ~0% deployable (no HTTP server, no rendered UI, no live deployment anywhere) but the underlying math/domain logic is mostly done in CORE+PROTOCOL — wiring-heavy, not a from-scratch build |
| **AGENT** | local/network AI, tool-use loop, MCP; three operating modes (no-AI / local-offline / connected) | P40–P42 | 4th — scaffolds in parallel with DELIVERY, offline-first by construction | substrate (LlmBackend/Ollama) shipped, but the executor loop connecting it to anything is 0% — a chat backend today, not an agent |
| **ECOSYSTEM/OPS** | external integrations, deployment, monitoring, multi-product platform | P43–P46 | 5th — explicitly and deliberately LAST | near-zero built, correctly so — nothing exists yet to integrate/deploy/monitor |

**The critical path to first-deployable dowiz:** CORE (already sufficient) → PROTOCOL **P34**
(wire the proven mesh delivery-domain into the dowiz kernel — THE highest-leverage single phase
in this entire document) → DELIVERY **P37+P38** (thin HTTP surface + WebGPU render, both largely
wiring once P34 lands) → AGENT **P40+P41** (tool-loop + three-mode proof, can scaffold in
parallel with DELIVERY, offline-first by construction) → ECOSYSTEM/OPS **P43–P46** (only once
something is live). **P34 is the single most important next action across the entire unified
roadmap** — bigger leverage than any other phase, because it converts ~70% of already-built,
already-tested protocol code from stranded to load-bearing.

### 10.2 Full P31–P46 index (swarm fast-lookup; sub-letter detail lives in §10.5)

| Phase | Component | Name | Status | Absorbs | Depends on | Blocks |
|---|---|---|---|---|---|---|
| P31 | CORE | Math-First Kernel (S0–S7 + Master Integration Tier A/B) | DONE-heavy: P31a DONE-ledger; P31b WIRING-GAP; P31c PARTIAL; P31d PLANNED; P31e PARTIAL | 17 units: S0–S7, A1–A3(mip), A6, B1–B5(mip) | nothing hard (P31d targets frozen P31a organs) | nothing on critical path |
| P32 | CORE | Hydraulic-Loop-v2 Wiring (self-tuning control loops) | P32a DONE; P32b/P32c WIRING-GAP; P32d PLANNED | 10 units: BP-01/02/05–10/22 + cross-model critic | P32d impl soft-deps AGENT LlmBackend wiring | nothing |
| P33 | CORE | CORE Ledger Hygiene (cleanup + status audit) | PLANNED (audit/flag only, zero build) | 15 units: BP-03/04/11/12–21/23 + JS-spike artifacts | nothing | P33b should precede new P32 sub-letter claims |
| P34 | PROTOCOL | Wire mesh-real's proven delivery-domain into the dowiz kernel (+ P34B planned mesh halves) | P34 WIRING-GAP (all absorbed units DONE); P34B PLANNED | 13 units: MESH-01–13 (MESH-09 halves split P34/P34B; MESH-14 unaccounted — §10.0) | CORE Law (sufficient today); P34B also P36 DoD-2 + P34 | DELIVERY P37, P13 wire-side, P34B; formalizes AGENT-arc's MESH-11 borrow |
| P35 | PROTOCOL | Docker-swap: zero-OCI runtime home (registration + finish) | PARTIAL (DK-01/02/03/07 DONE; DK-04/05/06/08 PLANNED) | 10 units: DK-01–10 | independent lane — no ordering dep on P34 | DK-06 feeds P45 deployment; wasm-host feeds AGENT tool ports |
| P36 | PROTOCOL | Bebop 5-expert remediation (2 live regressions first) | 🔴 REGRESSION ×2 (no_std wasm32 RED; insecure-TLS default-on); remainder PARTIAL | 5 units: bebop review P0–P4 (C1–C7 closures inventoried) | parallel to P34 — never serialize P34 behind it | DoD-2 blocks P34B iroh; DoD-1 blocks wasm32 consumers of bebop2-core |
| P37 | DELIVERY | Minimal HTTP/API surface for orders | PLANNED (0% — no dynamic HTTP server exists in-repo) | 1 unit: RW-09 (+ unblocks P23-P3; supplies wire half of P13) | P34 for mesh-backed data (can start against local delivery-domain) | P23-P3, P13, P39, AGENT P40 real tool target, P45 (hard) |
| P38 | DELIVERY | WebGPU render engine (P38a) + Sea & Sheet surfaces (P38b) | P38a PARTIAL (math substrate DONE, GPU path 0%); P38b PLANNED (0%) | 27 units: FE-04(=RW-04)/05–07/10–16 + RW-01/05/10/11 (P38a) · DZ-01–12 (P38b) | O18a graphics-unlock (hard, environment-gated); P38b ← P38a + P37/P34 | P38b; P17 splat-tier closure |
| P39 | DELIVERY | App-shell: installability + capability-auth wiring + offer math | PLANNED (installability undecided; P23-P1 and P20 DM-1 unblocked today) | 1 new unit (installability gap) + hosts P23-P1/P3 wiring and P20 DM-1 (their numbers unchanged) | P37 (auth wiring); P38a/b (installable target) | P17/P20 demo credibility; step-up auth for AGENT flows |
| P40 | AGENT | AgentLoop executor + tool-calling capability wiring | PLANNED (substrate DONE; loop 0 grep hits) | P21 (executor half) | P37 for the real read-order tool (scaffold now against a stub) | P41, P42 |
| P41 | AGENT | Three-mode operation: no-AI / local-offline / connected | PARTIAL (backend swappability shipped; parity proof + degradation contract missing) | P21 (mode/degradation half) + operator three-mode directive | P40 (DoD-1 no-AI proof landable today, before P40) | P42 |
| P42 | AGENT | MCP port + agent-as-capability boundary | PLANNED | 1 unit: IP-08 | P40 + P41 | ECOSYSTEM external consumption of AGENT tools |
| P43 | ECOSYSTEM/OPS | External integration ports (messenger/marketing/export/backup/hosting) | PLANNED (+1 live QRNG-endpoint bug fixable now) | 6 units: IP-11/12/13/14/19/20 (IP-10/15/16 → existing P22, not renumbered) | DELIVERY P37/P38; PROTOCOL P34 | nothing on critical path |
| P44 | ECOSYSTEM/OPS | Cache layers (EC-05) + own-RAG/own-inference scale-out | PLANNED (0/5 layers) — LOW PRIORITY / FAR-FUTURE | ~9 units: EC-05 + own-inference/RAG/chunking/gossip units of EC-03/04/06/08/12–15 | AGENT P40/P41 real traffic; DELIVERY P37 load | nothing; nothing waits on it |
| P45 | ECOSYSTEM/OPS | Deployment + monitoring floor (minimum viable ops) | PARTIAL — barely; the arc's own premise (attic) is gone | 22 units: OPS-01–22 | HARD-blocked by P37; data-layer items gated on pgrust-rebuild /council | P46 |
| P46 | ECOSYSTEM/OPS | Multi-product platform ("dowiz Local" + marketplace) | PLANNED (0%) — FURTHEST FUTURE | EC-17 + multi-product/marketplace remainder of the EC arc | everything above: P37/P38 live, P45 green (incl. off-site backup), P43 ≥1 port | nothing — terminal node of the roadmap |

### 10.3 Cross-cutting invariants (binding across components; each stated once)

1. **Three-mode operation (no-AI / local-offline-AI / connected-AI).** DELIVERY's core
   order/courier/money flow NEVER requires AI to function — already true by construction (CORE's
   decide/fold Law is pure deterministic Rust; LLM is "a feeling at the edge," never in the
   decision path). If every AGENT phase were deleted tomorrow, orders would still place, couriers
   would still match (deterministic HRW), money would still settle. P41 is the enforcement phase;
   the invariant binds DELIVERY and CORE too — they must never introduce an AI dependency in the
   critical order/money path.
2. **Offline-first / solo-island** (ARCHITECTURE.md F12; proven live by
   `delivery-domain/intake.rs::ac6_solo_island_full_flow_no_peers` — full order→delivery with
   ZERO peers). Binding on DELIVERY P37 (order placement must not require network — P37 DoD-5)
   and AGENT P41 (mode-2 network-isolated proof — P41 DoD-4).
3. **Capability-cert auth model.** Capability certificates (proto-cap, ML-DSA-signed,
   `HybridGate`/`verify_chain`/`RevocationSet` — all already built) are the PRIMARY auth model;
   conventional password+TOTP is never primary (D3 device-bound keypair primary; TOTP/WebAuthn
   are step-up only). Binding on DELIVERY P37 DoD-4 and P39.
4. **WebGPU/field-render UI, never DOM-first.** The UI is a WebGPU/WASM render of backend
   physics-field state; DOM survives ONLY as FE-15's invisible AccessKit mirror for
   screen-reader/IME input. Binding on DELIVERY P38a/P38b.
5. **Compilation-firewall pattern (repo-wide).** Consumers reach protected surfaces only through
   a facade whose lack of direct kernel imports is proven by `cargo tree` + a committed
   red-proof. Three instances, one pattern: PROTOCOL's KernelFacade
   (`proto-cap/src/facade.rs:64 submit_intent`, MESH-02), AGENT's ToolPort (P40 DoD-1), and the
   MCP layer (P42 DoD-3).
6. 🔴 **Two live regressions** demanding attention regardless of critical-path sequencing (P36
   DoD-1/DoD-2): bebop's `no_std` wasm32 build is RED right now
   (`cargo build --target wasm32-unknown-unknown --no-default-features -p bebop2-core` fails with
   `E0425` at `at_rest.rs:74`, regression from `d23e7aa` post-dating the remediation doc's own
   GREEN claim), and `proto-wire` ships `default=["insecure-tls"]`. Fix opportunistically —
   don't let them block P34, but don't let them rot either.

### 10.4 Demo/offer pipeline note (operator: do not lose)

P20 (Demo & Marketing Pipeline, DM-1..DM-8) stays live-numbered as-is — this consolidation
neither renumbers nor absorbs it. Its unblocked entry point **DM-1 (kernel discount math)** is
additionally hosted inside DELIVERY P39's DoD as the concrete next actionable step, since P39
already needs `compute_order_total` (`kernel/src/domain.rs:129`) extended. DM-2..DM-8
(publishing/marketing content pipeline) remain P20's own scope, gated behind P18 public-flip,
untouched by this consolidation.

### 10.5 Full component sections

The five component drafts, pasted verbatim below (headings demoted one level to nest here; zero
content trimmed; each drafted and live-verified by an independent parallel pass on 2026-07-18).

### 10.5.1 CORE — decide/fold Law, event-log, money, capability primitives, spectral math, self-tuning control loops

**Position on the critical path:** CORE is ~90% done and is NOT the bottleneck. The critical path runs CORE → PROTOCOL (wire stranded mesh to the dowiz kernel — the biggest lever) → DELIVERY (HTTP server + auth + UI render, ~0%) → AGENT (tool-use loop) → ECOSYSTEM/OPS. This section's job is to number and index what exists so nothing is lost, and to name the few real remaining gaps. The dominant failure mode in CORE is not missing code — it is **built-but-unwired code** (status `WIRING-GAP` below): modules that compile, pass tests, and are called by nothing.

**Absorbed source arcs:** math-first-architecture (S0–S7), Master Integration Plan Tier A/B (2026-07-14), hydraulic-loop-v2 (BP-01..23).

---

#### P31 — Math-First Kernel (S0–S7 + Master Integration Tier A/B)

##### P31a — Math-first DONE ledger
**Absorbs:** S0, S1, A3(mip), S2, S4, S7, A1(mip), A2(mip), B1, B2, B4, B5
**Status:** DONE
**Role & responsibility:** The completed body of the math-first rewrite. Indexed here so every old unit ID resolves to a P-number; no further build work.
**Blueprint:** `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P-A-kernel-primitives.md` (§11 has full Layer-A DoD for A1/A2/A3/A6); `docs/design/BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md`; `docs/design/MASTER-INTEGRATION-PLAN-2026-07-14.md`.
One-line ledger:
- **S0** eqc equation-compiler — DONE. `tools/eqc-rs/` (16/16 tests) → generated organs in `kernel/src/eqc_gen.rs` (`ema_next_f64`=A2, `apply_tax_exclusive_int`/`apply_tax_inclusive_int`=A3). Already numbered A1/A2/A3/A6 in the Layer-A canon.
- **S1 + A3(mip)** eigensolver consolidation — DONE via `kernel/src/spectral.rs::topk_symmetric` + `kernel/src/householder.rs::eigh_contig`/`eig2x2`. The old "3 duplicate Jacobi solvers, dowiz+bebop2 dual-authority" hazard is resolved.
- **S2** money=integer — DONE (`kernel/src/money.rs`, `domain.rs`, `cart.rs`, all i64).
- **S4** zero-copy bridge — DONE on the CORE side (`engine/src/bridge.rs::VertexBridge`, real CPU staging copy). GPU path stub is DELIVERY's concern, not CORE's.
- **S7** mesh-shaped substrate — PARKED by its own plan; becomes PROTOCOL's concern when it happens. Not a CORE item.
- **A1(mip)** "fix 3 broken backup scripts" — MOOT: scripts deleted with the `apps/` purge. Superseded by B4.
- **A2(mip)** living-knowledge recall — DONE in a different, better shape than planned: native Rust `kernel/src/retrieval/{ppr,bm25,recall,diffusion,spine,memory_store,index}.rs` (2879 lines, 9 tests), wired at `kernel/src/lib.rs:151,176`, consumed by the self-improvement loop via `recall.rs::PrimaryRecall`. (The JS spike it replaced is dead — see P33a.)
- **B1** Kalman filter — DONE (`kernel/src/kalman.rs`; `geo.rs::ema_next` confirmed as its bit-identical 1D special case).
- **B2** micrograd-autodiff — DONE (`kernel/src/micrograd.rs::Value::backward`).
- **B4** Rust-native backup organ — DONE (`kernel/src/backup.rs`, 702 lines, Buzhash-CDC dedup, crash-atomic). ⚠ Never exercised end-to-end — there is no live deployment to back up yet. That exercise is an **OPS/DELIVERY dependency**, not a CORE gap; ECOSYSTEM/OPS must schedule a real end-to-end backup run once minimal DELIVERY exists.
- **B5** trigram search — DONE (`kernel/src/trigram.rs` + `retrieval/index.rs::TrigramIndex`).
**Anti-scope:** Do not touch any of the above. Do not "improve" DONE organs while wiring gaps remain elsewhere.
**Depends on / blocks:** B4 end-to-end exercise blocked by DELIVERY/OPS deployment existing. Nothing here blocks the critical path.

##### P31b — CORDIC int-mode emission
**Absorbs:** A6 (Layer-A canon residual)
**Status:** WIRING-GAP
**Role & responsibility:** CORDIC exists (`tools/eqc-rs/cordic.rs`) but eqc-rs's Sin/Cos int-mode emission does not route through it. Close the last Layer-A gap so trig in integer mode is compiled, not floated.
**Blueprint:** `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P-A-kernel-primitives.md` §11 (A6 DoD already written there — reuse it, do not re-derive).
**DoD:**
1. An eqc-rs equation containing Sin/Cos compiled in int-mode emits calls into `cordic.rs` routines (verifiable by inspecting generated output).
2. eqc-rs test suite stays green (16/16 baseline) plus at least one new test asserting int-mode Sin/Cos output matches CORDIC reference values.
**Anti-scope:** Do not rewrite CORDIC — it exists and is correct. Do not add new trig ops beyond Sin/Cos. Do not touch float-mode emission.
**Depends on / blocks:** Depends on nothing. Blocks nothing on the critical path; pure Layer-A completeness.

##### P31c — no_std kernel
**Absorbs:** S3 (residual half; SIMD half of S3 is DONE — `kernel/src/simd.rs`, AVX2 softmax with bit-identical fallback)
**Status:** PARTIAL
**Role & responsibility:** Make the kernel crate `no_std`-capable so it can live on constrained/bare-metal targets (Kernel-as-MCU north star) and embed cleanly in WASM/mesh contexts. Today the crate is plain `std` — zero `#![no_std]` anywhere.
**Blueprint:** none dedicated; math-first plan S3 entry is the source. Write the alloc-boundary inventory as the first DoD step, not a separate blueprint doc.
**DoD:**
1. `kernel` compiles with `#![cfg_attr(not(feature = "std"), no_std)]` (or equivalent feature-gated split) — `cargo build --no-default-features` green.
2. Full existing test suite still green under the default `std` feature — zero behavioral drift.
3. Modules that genuinely need `std` (I/O-bound: `backup.rs`, `retrieval/memory_store.rs`, etc.) are feature-gated, not rewritten.
4. CI has a `--no-default-features` build check so the property can't silently regress.
**Anti-scope:** Do not chase `no_std` for the SIMD module beyond what falls out naturally (it's already done and bit-identical). Do not rewrite I/O organs to be allocation-free — gate them behind `std`. Do not block any other phase on this; it is not on the critical path.
**Depends on / blocks:** Soft prerequisite for PROTOCOL-side embedding of the kernel into mesh/WASM substrates and the long-horizon Kernel-as-MCU thesis. Blocks nothing in DELIVERY/AGENT.

##### P31d — Verification ladder (z3/kani proofs)
**Absorbs:** S5
**Status:** PLANNED (zero hits repo-wide — genuinely not started)
**Role & responsibility:** Machine-checked proofs for the invariants the repo already treats as red-lines: money-integer arithmetic, tax organs, fold determinism. Converts "VERIFIED-BY-MATH" from a discipline into an artifact.
**Blueprint:** none — needs one before build. First deliverable IS the blueprint (candidate-invariant inventory + tool choice kani-vs-z3 with a falsifiable comparison per the tech-selection rule).
**DoD:**
1. A blueprint doc exists naming ≥5 candidate invariants ranked by (red-line severity × proof cheapness), with tool selection justified by an honest comparison, not appeal to authority.
2. At least one proof harness lands and runs in CI (e.g. kani proof that `apply_tax_exclusive_int`/`apply_tax_inclusive_int` never overflow/never go negative for the documented input domain).
3. The proof is demonstrated RED-able: a deliberately introduced off-by-one in a scratch branch makes it fail.
**Anti-scope:** Do not attempt whole-kernel verification — ladder means cheapest-rung-first, money organs before anything else. Do not add proof tooling as a hard build dependency (CI-only). Do not let this delay PROTOCOL/DELIVERY work; it is hardening, not critical path.
**Depends on / blocks:** Depends on P31a being frozen (proofs target stable organs). Blocks nothing; strengthens everything.

##### P31e — Equation-IR at runtime + online learner bridge
**Absorbs:** S6, B3 — **these are the same open item; counted once here, never double-count**
**Status:** PARTIAL
**Role & responsibility:** eqc-rs today is build-time-only codegen; `kernel/src/online.rs` (LinearSGD/ScalarAdam online learner) exists and is tested, but the two are connected only by a comment. The gap is a runtime representation of eqc's IR that the online learner can adjust parameters of — closing the loop from "equations compiled once" to "equations that learn."
**Blueprint:** none dedicated — math-first plan S6 entry + Master Integration Plan B3 entry are the sources. A small design note (runtime-IR shape, which parameters are learnable, determinism guarantees) should precede code.
**DoD:**
1. A runtime-IR type exists in the kernel that can represent at least the already-generated organs' equation class (EMA + tax forms).
2. `online.rs::LinearSGD` or `ScalarAdam` demonstrably updates a parameter of a runtime-IR instance across ≥1 test scenario, with the pre-update evaluation bit-identical to the corresponding `eqc_gen.rs` compiled organ.
3. The comment-only link between `online.rs` and eqc is replaced by an actual call path (grep-verifiable: a real symbol reference, not prose).
4. Determinism preserved: with learning disabled, runtime-IR evaluation == compiled-organ output, bit-identical.
**Anti-scope:** Do not port all of eqc-rs into the kernel — only the minimal IR subset the existing organs need. Do not make runtime-IR the default execution path; compiled organs stay canonical, IR is the learning surface. Do not invent new optimizers — SGD/Adam exist.
**Depends on / blocks:** Depends on P31a (S0 compiler as the IR source of truth). Feeds P32 (self-tuning loops get a learnable substrate) and long-term AGENT self-improvement. Blocks nothing on the critical path.

---

#### P32 — Hydraulic-Loop-v2 Wiring (self-tuning control loops)

Code lives in bebop-repo (`bebop2/core/`, `crates/bebop/`) plus one kernel-side organ; blueprints in `docs/design/hydraulic-loop-v2/` (`HYDRAULIC-LOOP-v2-PLAN.md`, `BLUEPRINTS.md`). The pattern across this arc: nearly everything was BUILT and TESTED; almost nothing was WIRED. P32's work is connection, not construction.

##### P32a — Hydraulic DONE+wired ledger
**Absorbs:** BP-05, BP-06, BP-08, BP-09, BP-10, BP-22
**Status:** DONE
**Role & responsibility:** The hydraulic-loop items that are complete AND connected (or correctly resolved). Indexed for the record.
**Blueprint:** `docs/design/hydraulic-loop-v2/HYDRAULIC-LOOP-v2-PLAN.md` + `BLUEPRINTS.md`.
One-line ledger:
- **BP-05** PID redesign — DONE. `crates/bebop/src/governor.rs:180-233`, Jury-stable defaults (kp=1.03, ki=0.22, kd=0.20); old unstable Kd=1.5 demoted to named `default_legacy()`, kept only as a RED regression fixture.
- **BP-06** entropy-budget ledger — DONE. `crates/bebop/src/entropy_ledger.rs`, invariant `0 ≤ D_t ≤ H_max`, registered.
- **BP-08** admit() intake (Tikhonov well-posedness) — DONE AND LIVE-WIRED. `kernel/src/intake.rs` (744 lines), registered `kernel/src/lib.rs:84`, real consumer `kernel/src/loops.rs:11`. The one hydraulic item actually connected to something — the wiring template for P32b/P32c.
- **BP-09** survival-analysis persistence — DONE. `crates/bebop/src/persistence.rs` (Hungarian matching + D*=⌈log_p α⌉ Bonferroni), wired via `lib.rs:58`, consumed by `memory.rs`/`instrument_panel.rs`.
- **BP-10** orthogonometer (Goodhart decorrelation) — DONE. `crates/bebop/src/orthogonality.rs`, consumed by `loop_runtime.rs:421`.
- **BP-22** TS↔Rust reconcile — RESOLVED DIFFERENTLY: `agent-governance/resonator.ts` deleted entirely; no TS port left to reconcile; superseded by native `agent-governance-wasm`. Closed, not open.
**Anti-scope:** Do not touch. In particular do not "modernize" `default_legacy()` — it is a deliberate RED fixture.
**Depends on / blocks:** BP-08's intake→loops wiring is the reference pattern for P32b/P32c.

##### P32b — Resonator + arccos metric wiring
**Absorbs:** BP-01, BP-02
**Status:** WIRING-GAP
**Role & responsibility:** `resonator` is registered (`bebop2/core/src/lib.rs:328 pub mod resonator;` — the original plan's "still frozen, 1-line fix" framing is stale, it was unfrozen at some point) but has ZERO call sites: compiled+tested standalone, driving nothing. Likewise `algebra.rs:56 geodesic_distance` (acos) exists+tested with zero callers outside `algebra.rs` and was never plugged into resonator's `Metric` trait. The work is: pick ONE real, existing control loop and make resonator's output an input to it, with the arccos metric as its distance function.
**Blueprint:** `docs/design/hydraulic-loop-v2/HYDRAULIC-LOOP-v2-PLAN.md` (BP-01/BP-02 entries) — reuse the math there; only the wiring decision is new.
**DoD:**
1. `geodesic_distance` implements resonator's `Metric` trait (grep-verifiable trait impl, not a free function sitting nearby).
2. Resonator has ≥1 call site in a live loop consumer (candidates, in preference order: `crates/bebop/src/loop_runtime.rs`, `kernel/src/loops.rs` via the BP-08 pattern) — grep for `resonator::` outside `resonator.rs`/tests returns ≥1 hit.
3. A test exercises the loop→resonator→loop round trip (not resonator in isolation — that coverage already exists).
4. Evidence the wiring changes behavior: with resonator's contribution zeroed, at least one loop-level test result differs (guards against decorative wiring).
**Anti-scope:** **Do not rebuild resonator** — only wire its existing output into a real caller. Do not redesign the `Metric` trait. Do not invent a new consumer loop just to have a caller; if no existing loop genuinely benefits, stop and report that instead — a finding, not a failure.
**Depends on / blocks:** Independent of P32c (parallelizable). Serves the AGENT self-improvement loop (these control loops govern the agent's own loop). Blocks nothing in PROTOCOL/DELIVERY.

##### P32c — Online DMD wiring
**Absorbs:** BP-07
**Status:** WIRING-GAP
**Role & responsibility:** `dmd.rs` (OnlineDMD, rank-1 RLS, complex eigenpairs) is built, tested, and registered (`bebop2/core/src/lib.rs:316`), but only *referenced* — not called — from `field.rs:328`. Same stranded pattern as resonator. Wire its mode estimates into a live consumer so the dynamic-mode decomposition actually informs something.
**Blueprint:** `docs/design/hydraulic-loop-v2/HYDRAULIC-LOOP-v2-PLAN.md` (BP-07 entry).
**DoD:**
1. `field.rs:328`'s reference becomes an actual call: OnlineDMD is instantiated and updated with real field-state samples in a non-test code path.
2. At least one downstream decision reads DMD output (dominant eigenvalue / mode) — grep-verifiable consumer.
3. Round-trip test: feed a synthetic signal with a known dominant mode through the wired path and assert the consumer sees it (reuses `dmd.rs`'s existing test fixtures where possible).
**Anti-scope:** **Do not rebuild or extend OnlineDMD** — rank-1 RLS is done. Do not add higher-rank updates, forecasting, or new spectral features. Wiring only.
**Depends on / blocks:** Independent of P32b (parallelizable). Same consumer landscape as P32b — if both wire into `field.rs`/loop_runtime, coordinate to avoid hot-file collision (sequential landing, parallel prep).

##### P32d — Cross-model critic
**Absorbs:** the cross-model critic from hydraulic-loop-v2's original 7 math corrections (load-bearing, never assigned a BP number)
**Status:** PLANNED (not built — no multi-model-voting code found anywhere)
**Role & responsibility:** A decorrelated multi-model check on control-loop decisions — the mechanism behind the arc's math-correction discipline, and the one named item from the 7 corrections with no code at all. Distinct from the harness-level review agents (those review diffs; this critiques loop outputs).
**Blueprint:** none — needs a short design note first: what gets critiqued (which loop outputs), decorrelation requirement (different model/provider, per the research-verifier precedent), and advisory-only posture (signals, never gates — GROUND-TRUTH-over-PROXY rule).
**DoD:**
1. Design note exists specifying critic inputs (≥1 concrete loop output type), decorrelation constraint, and advisory-only integration point.
2. Minimal implementation: one loop output critiqued by ≥2 decorrelated judges with disagreement surfaced as a signal (logged/ledgered), not a gate.
3. RED-provable: a deliberately corrupted loop output triggers a critic disagreement signal in a test.
**Anti-scope:** Do not build a general "AI council" framework — one loop output, minimal voting, advisory only. Do not let critic output gate anything deterministic (violates GROUND-TRUTH-over-PROXY). Do not couple it to the AGENT phase's LlmBackend wiring timeline — design note can proceed now; implementation may reuse AGENT's LlmBackend once wired.
**Depends on / blocks:** Implementation (not design) soft-depends on AGENT's LlmBackend being wired to consumers. Blocks nothing.

---

#### P33 — CORE Ledger Hygiene (cleanup + status audit)

##### P33a — Dead JS-spike artifact deletion flag
**Absorbs:** JS living-knowledge spike closure (successor of the A2(mip) lineage; the live replacement is P31a's native Rust retrieval)
**Status:** PLANNED (flag only — deletion is an operator/lead call, not this phase's to execute unilaterally)
**Role & responsibility:** The JS living-knowledge spike is FULLY DEAD — all source deleted (`f9ab28ff1`, "drop ALL JS/TS per operator"). Orphaned build artifacts remain on disk: `spikes/living-knowledge/out/semantic-cache.json` (7.7 MB) plus eval-result JSON files — unregenerable dead weight with no source left to produce them. Per the standing Anu/Ananke discipline: confirmed-dead legacy gets actually deleted, after verifying nothing consumes it.
**Blueprint:** n/a — housekeeping.
**DoD:**
1. Verified no CI job, script, or codegen step reads `spikes/living-knowledge/out/**` (grep across repo + CI config, zero hits).
2. Files deleted in a single revertable commit, or an explicit operator decision recorded to keep them.
**Anti-scope:** Do not resurrect any part of the JS spike. Do not delete anything outside `spikes/living-knowledge/out/` under this flag. Do not treat this as license to sweep other directories — one target, one commit.
**Depends on / blocks:** Nothing. Anytime task.

##### P33b — Unconfirmed hydraulic BP status audit
**Absorbs:** BP-03 (Francis QR complex eigenvalues), BP-04 (diffusion sign fix), BP-11 (renormalizer), BP-12..23 (security/integration wave items)
**Status:** PLANNED (audit to-do, NOT a build task)
**Role & responsibility:** These BP items were not individually verified this session — their status is **unconfirmed**, neither DONE nor OPEN. Do not guess. A fresh check must classify each as DONE / WIRING-GAP / OPEN against live source, the same way BP-01/02/05–10/22 were classified above. Note BP-22 is already resolved (see P32a) — the audit range is BP-03, BP-04, BP-11, BP-12..21, BP-23.
**Blueprint:** `docs/design/hydraulic-loop-v2/BLUEPRINTS.md` (the authoritative BP list to audit against).
**DoD:**
1. Each of BP-03/04/11/12..21/23 has a live-verified status (file:line evidence for DONE/WIRING-GAP; explicit "no code found" for OPEN) recorded in this roadmap.
2. Any newly discovered WIRING-GAP items get folded into P32 as new sub-letters; any OPEN items get an explicit build/park decision — neither silently dropped.
3. Zero code changes made during the audit itself.
**Anti-scope:** Audit only — do not fix, wire, or build anything found during the check; file findings back into the roadmap instead. Do not mark anything DONE without file:line evidence (the BP-01 "still frozen" staleness above is exactly the failure mode this guards against).
**Depends on / blocks:** Should complete before any new P32 sub-letters are claimed by swarm agents (prevents duplicate/stale work). No external blocks.

---

**CORE cross-reference summary:** every source unit accounted for — S0→P31a, S1→P31a, S2→P31a, S3→P31c, S4→P31a, S5→P31d, S6→P31e, S7→P31a(parked→PROTOCOL); A1(mip)→P31a(moot), A2(mip)→P31a, A3(mip)→P31a, B1→P31a, B2→P31a, B3→P31e(=S6), B4→P31a, B5→P31a, A6→P31b; BP-01→P32b, BP-02→P32b, BP-05→P32a, BP-06→P32a, BP-07→P32c, BP-08→P32a, BP-09→P32a, BP-10→P32a, BP-22→P32a, cross-model-critic→P32d, BP-03/04/11/12..21/23→P33b, JS-spike-artifacts→P33a. Nothing dropped.

### 10.5.2 PROTOCOL — mesh, capability issuance, crypto, transport, delivery-domain (bebop2)

**Position on the critical path:** CORE (~90% done) → **PROTOCOL** → DELIVERY → AGENT → ECOSYSTEM/OPS. PROTOCOL is the single biggest lever in this roadmap: mesh-real's core delivery logic is **~70% already built, proven, and tested** in `/root/bebop-repo` — and **100% stranded**, with zero code-level connection from dowiz's own kernel to it. Wiring PROTOCOL to CORE (P34) is the #1 recommended next move for the whole project, bigger than building anything new.

**Connective-tissue finding (the most important sentence in this section):** the current P09/P10/P13 blueprints claim *"dowiz today has ZERO code-level dependency on the bebop protocol"* — true as a wiring statement, but **misleading as a build statement**: MESH-01 (delivery-domain), MESH-02 (KernelFacade), MESH-04 (claim_machine), and MESH-05 (matcher) already exist as ready-to-consume prerequisites in the sibling repo, and downstream work (agentic-mesh-protocol-2026-07-17 builds directly on MESH-11) already absorbs mesh-real informally without citing it. Nothing in this section starts from scratch; almost all of it is *registration and wiring* of finished code.

---

#### P34 — Wire mesh-real's proven delivery-domain into the dowiz kernel
**Absorbs:** MESH-01, MESH-02, MESH-03, MESH-04, MESH-05, MESH-07, MESH-09 (BPv7 half), MESH-10, MESH-11, MESH-12 — plus the wiring gap itself.
**Status:** WIRING-GAP (all absorbed units DONE; the connection is the only missing piece)
**Role & responsibility:** Make dowiz's kernel the consumer of the already-built bebop2 delivery protocol, closing the single largest value gap in the ecosystem. delivery-domain was *designed* to reuse dowiz-kernel as its decider (the KernelFacade `submit_intent` seam is exactly the compiled wire→Law→money boundary), so this phase is consumption, not construction. Once wired, DELIVERY's HTTP surface becomes trivial because the order lifecycle already exists here.
**Blueprint:** `/root/dowiz/docs/design/mesh-real/BLUEPRINTS-MESH-REAL.md` + `/root/dowiz/docs/design/mesh-real/MESH-REAL-PLAN.md` (MESH-12 resolution: `MESH-12-RESOLVED-2026-07-14.md`). Reuse; do not rewrite.

**Done inventory (one line each, verified live this session):**
- MESH-01 delivery-domain crate — `bebop2/delivery-domain/{lib.rs,intake.rs,pod.rs,finalization.rs,hub_ring.rs}`, 1844 lines, incl. proven solo-island offline test `intake.rs::ac6_solo_island_full_flow_no_peers` (full order→delivery with ZERO peers — directly satisfies the operator's "offline agent" requirement).
- MESH-02 KernelFacade — `bebop2/proto-cap/src/facade.rs:64` `submit_intent`; the compilation-firewall pattern (any port importing dowiz-kernel directly fails to build).
- MESH-03 event vocabulary — `proto-cap/src/event_dict.rs:278-299`, `DeliveryEvent::{OrderPlaced,ClaimOffered,ClaimAccepted,ClaimReleased,SettlementRecorded}`.
- MESH-04 claim_machine — `proto-cap/src/claim_machine.rs:85` `assert_transition`.
- MESH-05 matcher — `proto-cap/src/matcher.rs:63` `assign()` HRW rendezvous-hash + `hub_ring.rs:62`; deterministic, NO courier-scoring.
- MESH-07 Sync·Pull+Merkle — `proto-wire/src/sync_pull.rs:422` `MerkleLog`, 1181 lines, real.
- MESH-09 BPv7 half — `proto-wire/src/bpv7.rs`, 611 lines, hand-rolled custody/retry/expiry.
- MESH-10 WSS+rustls — `proto-wire/src/wss_transport.rs:423,511`, real TlsAcceptor/Connector.
- MESH-11 Revocation+H2 — `revocation.rs:49-114` + `hybrid_gate.rs:188-206,571` (agentic-mesh-protocol builds on this).
- MESH-12 node_id+genesis — `node_id.rs`, `H(pq_pub‖classical_pub)`, fail-closed `load_genesis`.

**DoD (falsifiable):**
1. A cargo dependency edge exists from a dowiz workspace member (kernel-adjacent adapter crate, not the web app) to `bebop2` `delivery-domain`/`proto-cap`, and it builds in CI — falsified by `cargo tree` showing no such edge.
2. The MESH-02 compilation firewall survives the wiring: dowiz consumes the protocol **only** through `KernelFacade::submit_intent` (`facade.rs:64`); a committed red-proof demonstrates that adding a direct dowiz-kernel import to any port fails the build.
3. Event-vocabulary round-trip: dowiz's order lifecycle maps 1:1 onto the five `DeliveryEvent` variants (`event_dict.rs:278-299`); an integration test folds a complete dowiz order through `claim_machine.rs:85 assert_transition` with zero illegal transitions.
4. Matcher consumption: at least one dowiz-side integration test calls `matcher.rs:63 assign()` for courier assignment and asserts determinism (identical inputs → identical assignment). No scoring, ranking, or reputation input is added (standing rejection).
5. Offline proof re-anchored: the `ac6_solo_island_full_flow_no_peers` scenario runs green **driven from the dowiz-kernel decider side** — full order→delivery with zero peers, using dowiz's Law as the fold.
6. Blueprint reconciliation: P09/P10/P13 are amended to cite MESH-01/02/04/05 by unit ID and file path; the "ZERO code-level dependency" claim is deleted or date-scoped to pre-P34. Falsified by grep still finding the unqualified claim.

**Anti-scope:** Do NOT fork or rewrite delivery-domain inside dowiz — consume the sibling crate. No new event variants. No courier-scoring/reputation (rejected as echo chamber; trust = signed capability only). No per-node storage (P34B). No transport/crypto hardening (P36). No HTTP server (DELIVERY drafter's scope).
**Depends on / blocks:** Depends on CORE decide/fold Law (~90% done — sufficient today). **Blocks DELIVERY** (its HTTP server becomes thin once this lands), blocks P34B, and formalizes what AGENT's agentic-mesh arc already borrows (MESH-11).

---

#### P34B — Finish the planned mesh halves: per-node storage, CRDT fence, iroh, ML-KEM KAT
**Absorbs:** MESH-06, MESH-08, MESH-09 (iroh half), MESH-13
**Status:** PLANNED
**Role & responsibility:** Close the four mesh-real units that were designed but never built. These are genuinely 0-30% (unlike the P34 units) and none of them gate the P34 wiring. The crypto-hygiene item (MESH-13) is real but not urgent-critical: ML-DSA-65 signing is the actually-used-today primitive; the ML-KEM path is not.
**Blueprint:** `/root/dowiz/docs/design/mesh-real/BLUEPRINTS-MESH-REAL.md` (same doc as P34; these are its unbuilt waves).

**DoD (falsifiable):**
1. MESH-08 CRDT compile-fence: a build-level mechanism (not design comments) makes introducing a CRDT merge on kernel-owned state fail compilation; red-proof committed.
2. MESH-13 ML-KEM: the current schoolbook impl (self-labeled "alternative") passes official FIPS-203 KAT vectors, or is replaced by one that does; `zeroize` applied to secret material (currently absent from the entire workspace).
3. MESH-09 iroh half: `proto-wire/Cargo.toml:51 iroh = []` is either implemented (gated behind secure TLS — hard-depends on P36 DoD-2) or formally retired with a dated decision note. An empty stub feature persisting is a fail.
4. MESH-06 per-node storage: a written decision + blueprint exists for per-node **local-first** storage. It MUST cite and explicitly distinguish `/root/dowiz/docs/design/BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` — that blueprint covers dowiz's CANONICAL-repo **hub** storage, a related-but-distinct concern. Today there are zero pgrust references anywhere in bebop-repo; conflating the two is the failure mode this DoD item exists to prevent.

**Anti-scope:** Do not block or serialize P34 behind any of this. Do not treat the hub pgrust rebuild as satisfying MESH-06 (different node role, different consistency model). No new KEM primitives beyond FIPS-203 compliance of the existing path.
**Depends on / blocks:** Depends on P34 (wiring reveals the real per-node storage shape) and on P36 DoD-2 (iroh must not inherit insecure-TLS). Blocks nothing on the critical path.

---

#### P35 — Docker-swap: give DK-01..03 a real home + finish DK-04..08
**Absorbs:** DK-01..DK-10
**Status:** PARTIAL (DK-01/02/03/07 DONE; DK-04/05/06/08 PLANNED)
**Role & responsibility:** Register and finish the zero-OCI runtime subsystem. **Omitted-finding, stated plainly: docker-swap has REAL, TESTED, WORKING code — yet it is entirely unreferenced by CORE-ROADMAP-INDEX.md and MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE.md; no P-number owns it.** This phase is that home. The working half: `bebop2/ports/telegram/src/lib.rs` is a real `wasm32-wasip2` component (zero ambient authority, one import), and `bebop2/wasm-host/src/lib.rs` is a real Scope→WASI deny-by-default host (wasmtime feature-gated) — not stubs.
**Blueprint:** `/root/dowiz/docs/design/docker-swap/BLUEPRINTS-DOCKER-SWAP.md` + `DOCKER-SWAP-PLAN.md`; DK-07 already resolved by `/root/dowiz/docs/design/microvm-isolation/ADR-NO-SANDBOX-AGENT-GOVERNANCE.md`.

**DoD (falsifiable):**
1. Registration: the unified roadmap index lists P35 as owner of DK-01..10, with DK-01/02/03/07 marked DONE and file-cited. Falsified by grep finding no DK-* reference in the index.
2. DK-06 Firecracker: an actual microVM boots and runs a workload in a test. Today `kernel/src/isolation/microvm.rs` has only a `kvm_available` probe and its own comment admits "actual VMM launch is the next unit" — that comment being still-true is the fail condition.
3. DK-08 supply chain: a CI workflow produces an SBOM (syft) and signs artifacts (cosign). Zero hits exist in any workflow file today; a run producing both artifacts is the pass.
4. DK-04 native static-server: `tools/native-spa-server` is either promoted to a deployed serving path (proven by one staging deploy) or descoped with a dated note. No third state.
5. DK-05 pgrust-as-native-systemd: decision note written, cross-referencing P34B DoD-4 (hub deployment vs per-node storage — again related-but-distinct).

**Anti-scope:** Do NOT reopen DK-07 — the no-sandbox-for-agent-governance ADR is decided. No Kubernetes (standing rejection). Do not rewrite the working DK-01/02/03 code to "modernize" it.
**Depends on / blocks:** Independent lane — runs parallel to P34, no ordering dependency on the wiring. DK-06 feeds ECOSYSTEM/OPS deployment; the wasm-host feeds AGENT's sandboxed tool ports. Note the P36 cross-wire: P36 DoD-1's `no_std` regression is on `wasm32-unknown-unknown` for `bebop2-core`, a different target than these `wasip2` components — related toolchain, not the same build.

---

#### P36 — Bebop 5-expert remediation: kill the two live regressions, close remaining P1/P2
**Absorbs:** P0-P4 of the bebop excellence review
**Status:** 🔴 REGRESSION (2 items live) — remainder PARTIAL
**Role & responsibility:** Finish the 5-expert-review remediation. Two items are not "planned work" but **actively broken/dangerous right now** — a shipped-default insecure-TLS feature and a build regression that post-dates the remediation doc's own GREEN claim (i.e., the doc is currently lying about build state). These two outrank everything else in this section for time-sensitivity.
**Blueprint:** `/root/bebop-repo/docs/design/EXCELLENCE-REVIEW-AND-REMEDIATION-2026-07-14.md` (amend its status table per DoD-1; do not rewrite the review).

**Done inventory (one line each):** C1 ML-KEM decap constant-time (`core/src/pq_kem.rs:730-746` + dudect test) · C2 getrandom nomem removed (`core/src/rng.rs:278-282,329-332`) · C3 deterministic-keygen behind test-only cfg · C4/C4b Ed25519 scalar-mul + mod_l constant-time (`sign.rs:668-739`, closed 2026-07-17/18 — this IS the already-known C4b closure, not a new item) · C6 hybrid identity from two independent seeds (`pq_dsa.rs:1135 derive_pq_seed`) · C7 serde_json bounded + TLV on the signed path · **P4 clean-slate publish DONE** — `github.com/SyniakSviatoslav/OpenBebop` public since 2026-07-14 (root `f38f2c5`, main now 67 commits ahead).

**DoD (falsifiable — regressions FIRST, in order):**
1. 🔴 **no_std regression:** `cargo build --target wasm32-unknown-unknown --no-default-features -p bebop2-core` is green. It fails RIGHT NOW with `E0425` at `at_rest.rs:74` — a regression introduced by commit `d23e7aa` (2026-07-17), AFTER the remediation doc claimed GREEN. Pass additionally requires (a) this target build added to CI so it cannot silently regress again, and (b) the remediation doc's status corrected.
2. 🔴 **C5 insecure-TLS default-on:** `proto-wire/Cargo.toml` currently ships `default=["insecure-tls"]` — a live security footgun in the default build, not a doc gap. Pass = insecure-tls removed from defaults (opt-in only) + a fence that fails CI if it ever re-enters the default feature set.
3. P1 field-math consolidation: the duplicate Chebyshev/VSA singleton in `rust-core/src/lib.rs:25-159` is removed or re-exported from the single core; grep finds exactly one implementation.
4. P2 Node/Docker cruft: root `package.json`, `docker-compose.sovereign.yml`, and the 4 unported `.mjs` CI gates are ported or deleted — after verifying none are CI-wired or codegen inputs (standing verify-before-delete directive).
5. P2 QRNG: production keygen either draws through the SeedPool (`proto-cap/src/entropy.rs::AnuQrng` is off-by-default today; real keygen uses a separate OS-only `core::rng::entropy_provider()`) or a dated decision records OS-only entropy as accepted. Per standing stance, QRNG **seeds/mixes, never replaces** OS entropy.

**Anti-scope:** No new crypto primitives. Do not re-report C4b as new work. Do not reopen P4 (publish is done). No batch-verify performance work — the batch-accept honesty question is settled (every accept re-verifies singly; correctness over speed).
**Depends on / blocks:** Runs parallel to P34 — do NOT serialize the wiring behind remediation. DoD-2 blocks P34B DoD-3 (iroh). DoD-1 blocks any `no_std`/wasm32 consumer of `bebop2-core`. Nothing here waits on any other drafter's scope.

---

*Draft note for the assembling editor: P34's DoD-6 (blueprint reconciliation) is the one item in this section that touches documents owned by other drafters (P09/P10/P13) — keep it here, since the false "zero dependency" claim is a PROTOCOL fact, but flag it during final assembly.*

*Assembly resolution (2026-07-18): confirmed — §10.5.3 DELIVERY's P13 status row and P34 DoD-6 agree (both date-scope the stale "ZERO code-level dependency" claim and name P34 as the supplier); no cross-draft conflict remained to resolve.*

### 10.5.3 DELIVERY — Product Surface (P37–P39)

**Position on the critical path:** CORE (~90% done) → PROTOCOL (mesh-real ~70% built, being wired in P34) → **DELIVERY is next**. The blunt truth: the product surface currently has **zero deployability** — no fly.toml, no live deployment, no HTTP order/API server anywhere in the repo (the only axum server, `tools/native-spa-server`, is static-file-only with zero dynamic routes). Once P34 lands, DELIVERY's order/courier/payment logic is mostly a **wiring** job (delivery-domain already has the proven flow), not new design.

**Already-landed substrate (DONE, listed for completeness, not re-scoped):** FE-01 zero-copy bridge (`engine/src/zerocopy.rs`, `engine/src/bridge.rs::VertexBridge` — caveat carried into P38a: `wasm/src/lib.rs` still returns copied `Vec`s, not the real ptr/len boundary), FE-02 SoA store (`engine/src/widget_store.rs`), FE-03 fixed-timestep loop (`engine/src/loop_.rs`), FE-08 motion/critical-damping (`engine/src/motion.rs`), FE-09 money-never-tween guard (`engine/src/money_guard.rs`); RW-02 (`kernel/src/analytics.rs::channel_ledger_js`), RW-03 (legacy money.ts/JS confirmed absent), RW-06 (`kernel/src/geo.rs`), RW-07 (`kernel/src/cart.rs`), RW-08 (`kernel/src/messenger.rs` + `money.rs`).

**Process note (one line, not a phase):** FE-17/RW-12's prescribed island-by-island migration was bypassed — `apps/web` was wholesale-deleted (`79ef316f6`, 2026-07-13) before those blueprints existed. No harm done (greenfield `web/` replaced it cleanly), but record it as a process deviation for future migrations.

**Already-numbered phases — corrected status (cross-reference only, numbers unchanged):**

| Phase | True status (verified this session) |
|---|---|
| P13 Delivery on Protocol | Its blueprint's "ZERO code-level dependency on the bebop protocol" claim is **stale** — PROTOCOL's P34 directly supplies what P13 needs. P13 becomes a wiring exercise once P34 lands; content unchanged, dependency corrected. |
| P16 Product UI Rebuild | Remains the master roadmap's home for this work; **P38a/P38b below are what actually fill it in** — no duplicate scope. |
| P17 Demo/Splat/GPU-Unlock Closure | Shares the same O18a `graphics-unlock` trigger as P38a's `cargo add wgpu`; one unlock serves both. |
| P18 Public-Flip | Landed on main; no DELIVERY correction needed. |
| P20 Demo & Marketing Pipeline (DM-1..DM-8) | **CONFIRMED 0% built** — `kernel/src/offer.rs`, `OfferKind`, `OfferRedeemed`, `PromotionType`: 0 grep hits; `compute_order_total` (`kernel/src/domain.rs:129`) explicitly excludes discounts ("No discounts in this scope"). DM-1 is the unblocked entry point (hosted in P39). |
| P22 Social Auto-Posting | **CONFIRMED 0% built** (no `SocialPoster` trait, no adapters, no `social-adapters` crate), but reuse substrate (`Spool`/`TokenBucket`/`ChannelLedger`) exists in kernel — cheap once started. Belongs more naturally in ECOSYSTEM's scope (external messenger ports); stays P22 as-is, status corrected here only. |
| P23 Device Auth+2FA | P1 (`totp.rs`, zero deps) buildable **today**, nothing blocks it. P3 (full HTTP wiring) **blocked — confirmed live** — on "no dynamic admin HTTP surface exists anywhere in this repo". P37 is the unblock. |

---

#### P37 — Minimal HTTP/API surface for orders
**Absorbs:** RW-09 (thin-shell boundary codify — the wire adapter is the second shell over the same kernel, subject to the same rule). Unblocks P23-P3; supplies the wire half of P13. No FE/DZ units live here.
**Status:** PLANNED (0% — the only axum server in the repo is static-file-only, zero dynamic routes)
**Role & responsibility:** The #1 literal blocker of the entire DELIVERY layer: expose delivery-domain's already-proven order lifecycle over a wire. This is explicitly a **thin** surface — just enough dynamic routes to place/advance/read an order — not a REST API design exercise; the order flow, state machine, and money math already exist and are tested, the server merely transports intents to `decide` and serves `fold`-derived state.
**Blueprint:** No dedicated blueprint (deliberately — the scope is "thinnest possible adapter"). The two documents that name this exact gap and constrain it: `docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P13-delivery-on-protocol.md` and `docs/design/BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` (whose P3 is blocked on this surface). Reuse them; do not write a REST spec.
**DoD:**
1. A dynamic HTTP server exists in-repo (extend `tools/native-spa-server` or a sibling crate) serving both the static `web/` assets and dynamic order routes from one binary. Falsifiable: `grep` finds ≥1 non-static route handler; the binary boots and answers a dynamic request.
2. An integration test drives one full order lifecycle **over the wire** (place → accept → pickup → deliver) and asserts the final fold-derived state matches the same sequence run directly against delivery-domain. Red today (no server), green at close.
3. Thin-shell invariant (RW-09): zero domain logic in handlers — every state change routes through kernel `decide`/`fold`; falsifiable by review gate: no order-state mutation outside kernel calls in the server crate.
4. Mutating routes authenticate via **capability certificates** (proto-cap, ML-DSA-signed, PROTOCOL's `HybridGate`/`verify_chain`/`RevocationSet` — all already built). Falsifiable: a request with a forged or revoked cert is rejected (401/403) in a test; a valid chain passes.
5. **Offline parity (ARCHITECTURE.md F12, canon-locked):** the HTTP server is NOT the only way to place an order. The WASM-in-browser local decide/fold path that `web/src/app.mjs`'s beachhead already uses is extended to real order placement — a test places an order with the server absent and the fold is identical; rejoin/sync is PROTOCOL P34's job, not P37's.
6. The server binary is runnable locally with one documented command. (Deploy packaging — fly.toml, monitoring — is ECOSYSTEM/OPS scope, P40+; P37 only guarantees a bootable binary.)
**Anti-scope:** Do NOT build a conventional REST+session/password login — auth is capability-cert-based per canon and `BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` D3 (device-bound keypair primary; TOTP/WebAuthn are step-up only). Do NOT design a full resource-oriented REST API, pagination, versioning, or an admin CRUD surface. Do NOT put any pricing/discount/state logic in handlers. Do NOT make network the required path for order placement (F12).
**Depends on / blocks:** Depends on PROTOCOL P34 for real mesh-backed order data (the server can land against local delivery-domain first). **Blocks** P23-P3 (its named live blocker), P13 wire-side wiring, P39b, and any AGENT (P4x) flow that needs an API to call.

#### P38a — WebGPU render engine completion
**Absorbs:** FE-04/RW-04 (particle→wgpu, single unit, counted once), FE-05 (SDF pipeline + GPU design-token table), FE-06 (MSDF text), FE-07 (layout field), FE-10 (Green's-function feedback), FE-11 (potential wells), FE-12 (spectral φ₂φ₃ embedding), FE-13 (constraint solver), FE-14 (lazy-render-on-settle), FE-15 (a11y mirror), FE-16 (WebGL2/SIMD fallback), RW-01 (`dowiz-engine` Cargo workspace), RW-05 (shell crate reshape — closes FE-01's caveat), RW-10 (web toolchain), RW-11 (view→wgpu migration).
**Status:** PARTIAL (math substrate DONE; GPU path and pipelines 0%)
**Role & responsibility:** Turn the tested, bit-deterministic physics-field substrate into actual pixels. This requires **no redesign**: `engine/src/field_frame.rs::compose()` already renders physics state to RGBA (real, tested, bit-deterministic), and `VertexBridge` has a real CPU staging copy — its `new_gpu()` is a stub only because the `wgpu` dependency is a network-gated `cargo add` (O18a `graphics-unlock`, verified RED/403 as of 2026-07-16 — a ONE-TIME unlock shared with P17, not an architecture question).
**Blueprint:** `docs/design/field-ui-engine/` + `docs/design/field-ui-engine.md` (FE-01..17), `docs/design/rust-engine-rewrite/` (RW-01..12), `docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P16-product-ui-rebuild.md` (P16's home, filled by this phase), `docs/design/BLUEPRINT-W21-field-ui-gpu-blocked.md` (the gpu-blocked record). Reuse; don't rewrite.
**DoD:**
1. `wgpu` added (O18a unlock); `VertexBridge::new_gpu()` real — staging buffer uploads to a GPU vertex buffer. Falsifiable: headless pixel readback matches `field_frame::compose()`'s RGBA reference (the bit-deterministic oracle already exists — use it, don't invent a new one).
2. FE-04/RW-04 particle renderer draws N particles from `widget_store` (FE-02) at the fixed timestep (FE-03) with `motion.rs` damping (FE-08). Note: the source `particle-cloud.js` no longer exists (only a README survives) — this is a reimplement-from-spec against the engine SoA store, not a port.
3. FE-05: SDF pipeline (`sdf.rs`/`scene.rs` primitives exist) gains the GPU design-token table. FE-06: MSDF glyph atlas + text draw (currently zero glyph code anywhere).
4. FE-07 real force-layout (today only a partial spectral-decode helper) + FE-10/11/12/13 each land with at least one deterministic test against kernel math.
5. FE-14 lazy-render-on-settle: falsifiable — frame callbacks stop within k ticks of field settle in a test.
6. FE-15: DOM survives ONLY as an invisible AccessKit mirror for screen-reader/IME text input; falsifiable — screen-reader tree exposes order state while zero visible DOM nodes render UI. FE-16: WebGL2/SIMD fallback flags are functional, not the current empty stubs.
7. RW-05 + FE-01 caveat closed: `wasm/src/lib.rs` exposes the real ptr/len JS boundary (no copied `Vec<u8>`/`Vec<f32>` returns); mixed-in retrieval exports separated out. RW-01: kernel/engine/wasm unified into a `dowiz-engine` Cargo workspace (currently path-deps only). RW-10: `web/package.json` graduates from bare Node script runner to a real toolchain. RW-11: the view layer that emerges here is wgpu-native from day one (no interim DOM view to migrate).
**Anti-scope:** Do NOT build a DOM admin panel — the UI is a WebGPU/WASM render of backend physics-field state (canon; DOM only per FE-15's invisible mirror). Do NOT redesign the math substrate — compose/zerocopy/widget_store/loop_/motion/money_guard are done and tested. Do NOT tween money (FE-09 guard is landed and binding). Do NOT treat `web/src/app.mjs` (204 lines, console-only, 24/24 kernel exports bound) as throwaway — it is the confirmed deliberate first step; its own header names the DOM/FieldSim pass ("G3") as a later unit that reuses these bindings.
**Depends on / blocks:** Depends on O18a `graphics-unlock` (hard, environment-gated; same trigger P17 waits on). Blocks P38b entirely and P17's splat-tier closure. Independent of P37 — the two can proceed in parallel.

#### P38b — Sea & Sheet product surfaces (dowiz-interfaces)
**Absorbs:** DZ-01..12 — all twelve, none dropped. DZ-10 (voice) is absorbed **as deliberately deferred**: fully built+tested for the old deleted stack (49/49 tests, real Whisper ASR), deleted in the 2026-07-13 purge, and intentionally re-placed at the arc's own Phase 9b ("optional integrations", after the order-critical path) — an intentional deprioritization, not an oversight. Gesture control (one checklist bullet) shares that tail.
**Status:** PLANNED (0% code — no `Intent`/`FieldPos`/`InputSource` structs exist anywhere; `web/src/app.mjs` has zero DOM/canvas)
**Role & responsibility:** The actual product interfaces built on P38a's pipelines: Sea (ambient-field client surface) and Sheet (brand-SDF) — the customer storefront and order flow as field-render, wired to real order data. This is where a customer first *sees* dowiz in the new stack.
**Blueprint:** `docs/design/dowiz-interfaces/` (DZ-01..12, Sea & Sheet). Reuse; don't rewrite.
**DoD:**
1. `Intent`/`FieldPos`/`InputSource` structs exist and are exercised by tests (currently 0 grep hits).
2. Sea and Sheet each render via P38a pipelines against real kernel state; one end-to-end pass shows an order placed through the Sea surface reaching delivery-domain fold state (via P37's wire or the F12-canon local WASM path).
3. DZ-01..09/11/12 each traceable to landed code or an explicit deferral note; DZ-10 voice + gesture remain at Phase-9b priority — pulling them forward is a scope violation, not initiative.
**Anti-scope:** No DOM-first screens (same canon as P38a). Do not resurrect the old Whisper voice stack ahead of the order-critical path. Do not fork a second design-token system — FE-05's GPU token table is the single source.
**Depends on / blocks:** Hard-depends on P38a (pipelines) and, for real order data, P37 + PROTOCOL P34. Blocks nothing downstream except demo polish (P17/P20 visual units benefit but aren't gated).

#### P39 — App-shell: installability + capability-auth wiring + offer math
**Absorbs:** The **installability gap** — the one genuinely new phase-item in this section, with no prior unit ID: the old stack had a full Svelte PWA + service worker AND a Tauri desktop installer (`apps/bootstrap-installer`), both deleted in the purge; the new stack has ZERO PWA/installability work and NO canon decision locking it in or rejecting it. It is not covered by FE-*/DZ-* (those are rendering, not app-shell packaging). Also hosts the DELIVERY-side wiring of P23 and P20's DM-1 — both keep their own numbers; P39 does not claim them.
**Status:** PLANNED (installability 0% + genuinely undecided; P23-P1 unblocked today; P20 DM-1 unblocked today)
**Role & responsibility:** The remaining product-surface pieces once P37/P38 (the real leverage points) exist: make the product installable, wire device-bound capability auth into the live surface, and give the kernel real offer/discount math so demos and marketing have something true to show.
**Blueprint:** Installability: none exists — first deliverable is the canon decision itself (PWA vs native wrapper vs both vs rejected), recorded before code. Auth: `docs/design/BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` (reuse — it already correctly targets D3 device-bound keypair primary, TOTP/WebAuthn step-up only). Offers: `docs/design/DEMO-MARKETING-PIPELINE-REFACTOR-2026-07-17.md` (P20; DM-1 is the entry point).
**DoD:**
1. A canon decision on installability exists (ADR-style, in docs/design), and — if accepted — the chosen shell (manifest + service worker, or wrapper) installs the `web/` surface and launches offline-capable per F12. Falsifiable: install + airplane-mode launch reaches the local-decide order path.
2. P23-P1 (`totp.rs`, zero deps) landed; P23-P3 wired onto P37's routes once P37 exists — step-up only, never primary auth.
3. P20 DM-1 landed: kernel discount math exists and `compute_order_total` (`kernel/src/domain.rs:129`) no longer carries "No discounts in this scope"; property tests pin money invariants (FE-09/money_guard discipline applies).
**Anti-scope:** Do not build a conventional password+TOTP login as primary — capability certificates (proto-cap, ML-DSA) are the auth model, per canon; TOTP/WebAuthn are secondary step-up only. Do not let the service worker introduce a network dependency for ordering (F12). Do not expand into P20's publishing/marketing pipeline (DM-2+ stays P20's own scope, publication gated behind P18 public-flip) or P22 social posting (ECOSYSTEM-adjacent, stays P22).
**Depends on / blocks:** P39 auth-wiring depends on P37 (P23-P3's named blocker); installability depends on P38a/P38b having something worth installing, though the canon decision and manifest skeleton need nothing. Blocks P17/P20 demo credibility (real offers, installable demo) and provides the step-up auth AGENT (P4x) flows will assume.

### 10.5.4 AGENT — local/network AI, tool-use loop, MCP

> **Scope boundary (locked invariant):** DELIVERY's core order/courier/money flow NEVER requires AI to function. This is already true by construction — CORE's decide/fold Law is pure deterministic Rust (the standing "НЕ-AI у ядрі" invariant from math-first-architecture's 7 invariants; LLM is "a feeling at the edge," a resonator-style pure-fn concern, zero I/O in the core, never in the decision path). Every phase below is additive assistance on top of an already-complete deterministic system. If every AGENT phase were deleted tomorrow, orders would still place, couriers would still match (deterministic HRW), money would still settle.
>
> **Naming discipline — two different "agents," do not conflate:**
> - **This section's agent** = the local delivery-operations assistant: `LlmBackend` (`kernel/src/ports/llm.rs`) + a to-be-built tool-use loop acting on order/courier operations.
> - **`AgentBridge`** (`kernel/src/ports/agent/{admission,cap,manifest,scope}.rs`, consumed by `agent-adapters/{cache,dispatch,mcp}.rs`) = foreign-agent admission/caging for the mesh — PROTOCOL's scope, part of the agentic-mesh-protocol arc. Zero code links it to `LlmBackend` today, and that separation is intentional.
>
> **Out of scope, flagged for awareness — self-mod effector (bebop-repo):** `bebop2/core/src/self_mod.rs` + `self_mod_loop.rs` exist with a header claiming "ACTIVATED (operator, 2026-07-16)" (commits `3696caa`/`dd431b5`). It is DORMANT (called only from its own unit tests, not wired into any live loop), narrow (mutates one in-memory Kalman q-scaler parameter, capability-gated, hard-refuses all red-lines), and it is a code-self-modification actuator, not a delivery-operations assistant. It stays outside this section's phase numbering. Caveat: its activation claim is self-asserted in commit messages and deserves independent operator confirmation before anyone treats it as authorized-live.

**Critical-path position:** CORE (~90%) → PROTOCOL (P34) → DELIVERY (P37/P38) → **AGENT (this section)** → ECOSYSTEM/OPS. AGENT depends on DELIVERY's P37 order-API surface existing (a tool loop needs something to call), but design and scaffolding proceed in parallel — see per-phase dependency notes.

---

#### P40 — AgentLoop executor + tool-calling capability wiring
**Absorbs:** P21 (resident-agent plane, executor half) · follow-on to the shipped harness-llm-backend arc (`feat/harness-llm-backend`, Ollama port Wave 0+1+consumer-wiring DONE)
**Status:** PLANNED (its substrate is DONE — the gap is everything above it)
**Role & responsibility:** Build the plan→act→observe executor that turns the existing raw chat-completion backend into an agent that can DO things. Today `LlmBackend` is real and consumed (only by `llm-adapters/src/{dispatch,cache,compose,ollama}.rs` and its own tests/benches), but `AgentLoop`/any executor has **0 grep hits anywhere in the repo** — this is the single biggest gap in AGENT's scope: a chat backend with no callers connecting it to orders. P40 also defines the tool-port interface behind a KernelFacade-style compilation firewall and un-pins tool-calling at the capability level: `Caps.tool_calling` is HARD-PINNED `false` at `llm-adapters/src/ollama.rs:59` — it is not even wired at the flag level yet.
**Blueprint:** `docs/design/harness-2026-07-16/HARNESS-LLM-BACKEND.md` covers the backend layer and already anticipates tool-calling — as a `Caps` probe ("Tool-calling/structured-output support differs per backend/model — a `Caps` probe, not assumed," §2.2 Quirks item 5) and as a `tools` field in the exact-match cache key (§3.2) — but contains **no loop design**. Build on that doc's port/adapter/firewall conventions; the loop itself needs a first design pass here.
**DoD:**
1. A `ToolPort` trait exists in the kernel-ports layer (plain structs, no serde/HTTP, mirroring `llm.rs` conventions); the loop crate consumes tools ONLY through it — `cargo tree` shows the loop crate does not import `dowiz-kernel` directly (same firewall done-check the LLM blueprint already uses: kernel shows no HTTP client, no adapter crates).
2. `Caps.tool_calling` is no longer hard-pinned `false` for Ollama — it is set by a live per-model probe, fail-closed (probe fails ⇒ `false`).
3. Exactly ONE tool ships: **read order status by ID**. A local Ollama model, given a natural-language request, calls it and returns the correct status for one test order, proven end-to-end by one test. (Deliberately minimal — this is the falsifiable "the agent can DO something" gate, not a framework.)
4. The loop is bounded: hard max-iteration cap, every tool call and result logged, a tool error surfaces as a typed loop outcome (never a silent retry-forever).
**Anti-scope:** No multi-tool framework, no tool registry, no write/mutating tools, and absolutely no money/auth/RLS/migration tools in this phase. No streaming, no re-design of `LlmBackend` (it is shipped; extend, don't rewrite). Do not touch `AgentBridge`/`agent-adapters` — that is PROTOCOL's mesh-admission surface. No autonomy: the loop executes one user-initiated request to completion, it does not schedule itself.
**Depends on / blocks:** Needs DELIVERY's P37 order-API surface for the real read-order tool target; until P37 lands, DoD items 1–2 and a no-op/echo tool loop are buildable now against a stub — do the scaffold in parallel. Explicitly does NOT require PROTOCOL's P34/P35 to be complete: a minimal read-only tool loop must work on a solo offline node, since offline-first is a hard requirement. Blocks P41 (mode parity needs a loop to be parity OF) and P42 (MCP re-exposes P40's tool port).

#### P41 — Three-mode operation: no-AI / local-offline / connected — one tool interface, swappable backend
**Absorbs:** P21 (mode/degradation half) · operator three-mode directive (verbatim requirement, the spine of this section)
**Status:** PARTIAL (backend swappability largely shipped; mode-parity proof and degradation contract are the gap)
**Role & responsibility:** Make the three operating modes an enforced, tested property rather than an intention. Mode 1 (no-AI) requires **zero new code** — CORE+PROTOCOL are AI-free by design and this phase only locks that in as a regression-proof invariant. Modes 2 and 3 must differ ONLY in which `LlmBackend` impl is selected (Ollama local vs managed/remote — both adapter families already exist per the blueprint's Tier-0 `ManagedApiAdapter` / Tier-1 Ollama split and `dispatch.rs`), never in the tool-loop shape: one port, swappable backend, no second tool-calling implementation.
**Blueprint:** `docs/design/harness-2026-07-16/HARNESS-LLM-BACKEND.md` §2.2 (one `OpenAiCompatTransport` + per-adapter `Quirks`) is the swappable-backend half; no existing blueprint covers the degradation contract — small design pass needed here (a policy note, not a document).
**DoD:**
1. **No-AI proof:** CORE/PROTOCOL test suites pass with AGENT crates absent from the build graph (`cargo tree -p dowiz-kernel` shows no llm/agent-loop crates — the blueprint's existing firewall check, promoted to a mode-1 invariant). Consistency anchor: PROTOCOL's `ac6_solo_island_full_flow_no_peers` already proves the full flow with no peers; mode 1 is that plus no-AI, and it must stay green untouched.
2. **Mode parity:** P40's single-tool test passes with the backend swapped by configuration only — zero source diff in the loop or tool port between local and connected runs.
3. **Graceful degradation:** with Ollama stopped (typed `LlmError::Unavailable`) and no network, the order/courier flow is provably unaffected and the agent surface returns a typed "assistant unavailable" outcome — never a hang, never a blocked order.
4. **Local-offline proof:** mode-2 run passes with all remote endpoints unreachable (network-isolated test), consistent with the solo-island guarantee.
**Anti-scope:** No new code for mode 1 (writing any is a design smell — reject it in review). No second tool-loop implementation for remote backends. No "smart" auto-escalation from local to remote without explicit configuration. Routing help from the model stays advisory-only — the deterministic HRW matcher remains the sole courier-assignment authority in every mode.
**Depends on / blocks:** Depends on P40 (needs the loop and one tool to prove parity over). DoD item 1 is provable TODAY, before P40 — land it first as the locked baseline. Independent of PROTOCOL P34/P35 completeness by construction (offline-first). Blocks P42 (MCP exposure must inherit the same three-mode contract).

#### P42 — MCP port + agent-as-capability boundary
**Absorbs:** IP-08 (MCP-server + agent-as-capability port — 0% built, no code found under this name)
**Status:** PLANNED
**Role & responsibility:** Give P40's tools a standard exterior: the agent calls tools via MCP, and each tool is a capability-scoped port — never a direct kernel import. This mirrors PROTOCOL's KernelFacade compilation-firewall pattern on the AGENT side: the same architectural move, applied to tool-calls. Deliberately lighter than P40/P41 — it is follow-on work that standardizes a pattern only after P40/P41 have proven it on one tool.
**Blueprint:** no existing blueprint, first design pass needed here. Reference-adjacent code exists — `agent-adapters/mcp.rs` — but it serves PROTOCOL's mesh `AgentBridge` (foreign-agent admission), not this port; study its conventions, do not repurpose it or couple to it.
**DoD:**
1. P40's read-order-status tool is additionally callable through an MCP server endpoint, same behavior, one test.
2. Capability scoping is enforced fail-closed: a tool invocation outside the granted capability scope is refused with a typed error, proven by one negative test.
3. Firewall holds: the MCP layer imports only the `ToolPort`/facade surface — `cargo tree` shows no direct `dowiz-kernel` dependency from the MCP crate.
**Anti-scope:** No foreign-agent admission, caging, or mesh exposure — that is `AgentBridge`, PROTOCOL's scope; P42 serves the LOCAL agent only. No tool-catalog expansion (still the one proven tool). No transport invention — MCP as-specified. Forward-looking cross-reference only, not scoped work: IP-05's "multimodality = superposition of intents" (voice+touch composing, not conflicting) becomes AGENT-relevant if voice ever becomes an agent input channel — that lives with DELIVERY's DZ-10 (voice, deliberately Phase-9b-deprioritized), and P42 must not front-run it.
**Depends on / blocks:** Depends on P40 (tool port) and P41 (three-mode contract it must inherit — the MCP surface must degrade exactly as gracefully). Needs no PROTOCOL P34/P35 completion: a local MCP endpoint on a solo node is the baseline case. Blocks nothing on the critical path — ECOSYSTEM/OPS integration work that wants to consume AGENT tools externally should wait for P42 rather than importing anything deeper.

### 10.5.5 ECOSYSTEM/OPS — External Integrations, Deployment, Multi-Product Platform

> **Sequencing verdict (the most important sentence in this section):** ECOSYSTEM/OPS is **explicitly LAST on the critical path** — CORE → PROTOCOL (P34) → DELIVERY (P37/P38) → AGENT (P40/P41) → **then this**. This is not a priority judgment about the work's worth; it is a statement of physical reality: there is currently **zero live deployment** (no `fly.toml`, no pgrust binary installed, `attic/` and the old `apps/` stack physically deleted). Deployment, monitoring, external integrations, and multi-product platforming only make sense once there is something real to deploy, monitor, and integrate. Building a monitoring stack for a service that does not exist is waste, and every phase below carries an anti-scope rule enforcing that.

> **Audit finding (largest silently-dropped cluster in the whole roadmap audit):** neither the integration-ports arc (IP-01..21) nor the ecosystem-strategy arc (EC-01..20) is referenced *at all* by `CORE-ROADMAP-INDEX.md` or `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` — zero hits for either arc name. Roughly **38 of the 41 combined external-facing IP+EC unit IDs had no living tracking artifact anywhere in current canon** before this section. This section is their new (and only) home. The absorption ledger:
> - **IP-10 / IP-15 / IP-16** (social/messenger marketing) → **ABSORBED INTO existing P22**, not renumbered (see below).
> - **IP-11 / IP-12 / IP-13 / IP-14 / IP-19 / IP-20** (messenger/marketing/hosting/backup/export/automation ports) → **P43**.
> - **EC-05** (five cache layers) and the own-inference / own-RAG / chunking / gossip-flows units of **EC-03/04/06/08/12–15** → **P44**.
> - **All OPS-01..22** (ops-reliability arc) → **P45**.
> - **EC-17** and the multi-product/marketplace remainder of the EC arc (incl. "dowiz Local") → **P46**.
> - EC's shared **KernelFacade** concept is already built in PROTOCOL — cross-referenced there, not re-claimed here. IP-01..09/17/18 are PROTOCOL's/AGENT's/CORE's scope and are covered by those drafters.

#### Existing P22 — Social Auto-Posting (confirmed home, not renumbered)
P22 is **confirmed 0% built** — no `SocialPoster` trait, no `TelegramAdapter`/`ViberChannelAdapter`, no `social-adapters` crate anywhere. But its blueprint (`docs/design/BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md`) already correctly cites **IP-10/IP-15/IP-16 as prior art**, which means P22 is *already* the correct numbered home for those three units. **They are ABSORBED INTO existing P22 — do not renumber, do not duplicate under P43.** Note for whoever starts it: the reusable substrate already exists in kernel (`Spool`/`TokenBucket`/`ChannelLedger`), making Wave-0 (Telegram) cheap once DELIVERY gives it something to post about.

#### P43 — External Integration Ports: Messenger / Marketing / Export / Backup-Export / Hosting
**Absorbs:** IP-11, IP-12, IP-13, IP-14, IP-19, IP-20. (IP-10/15/16 → ABSORBED INTO existing P22, not renumbered.)
**Status:** PLANNED (with two false premises corrected and one small live bug)
**Role & responsibility:** All customer/operator-facing external channels that are not social auto-posting: messenger delivery-notification ports, marketing/channel-tracking, data export, and hosting/automation ports. These follow the arc's core-immutable/integrations-as-ports doctrine: adapters at the edge, never leaking into kernel Law.
**Blueprint:** none yet — source arc: `/root/.claude/projects/-root-dowiz/memory/integration-ports-reactive-arc-2026-07-13.md`. Write a blueprint only when this phase unblocks; do not rewrite the arc doc.

**Two arc claims CONFIRMED FALSE (wrong even when written — correct the record, do not build on them):**
1. *"`?ch=` channel-tracking spine already exists at `Storefront.svelte:93`"* — **false.** `Storefront.svelte` does not exist anywhere in the current repo (it lived in the old, deleted `apps/` stack). Current `web/src` is a greenfield rebuild with zero `?ch=` code. Channel tracking is a from-scratch item, not a "wire-up" item.
2. *"Telegram already has full push+OTP as the primary messenger"* — **false.** What exists is `kernel/src/messenger.rs:33 telegram_link()`, an explicitly **non-sending** deep-link string builder (its own comment says "never sends"), plus a completely separate `tools/telemetry/*` Telegram bridge that is the OPS/governance alerting channel (heartbeat monitors) — **not** a customer/courier-facing channel. Do not conflate the two; a real customer-facing Telegram send path does not exist.

**One concrete small bug, actionable now (bug ticket, not a phase):** the QRNG entropy port is split oddly — `bebop-repo/bebop2/proto-cap/src/entropy.rs` (`AnuQrng`/`SeedPool`) uses the current ANU endpoint with fail-closed tests, while `dowiz/kernel/src/pq/entropy.rs:37-122` is a simpler feature-gated version pointing at a **deprecated legacy ANU endpoint URL**. Fix = align the dowiz copy to the current endpoint (or delete it in favor of the proto-cap one). Small, mergeable independently of this phase's gating.

**DoD:**
1. QRNG endpoint mismatch fixed: `kernel/src/pq/entropy.rs` no longer references the deprecated ANU URL, verified by its own fail-closed test.
2. One real customer-facing messenger send path exists (Telegram first) that actually transmits — falsified by `messenger.rs` still being the only "messenger" code.
3. `?ch=` channel tracking exists in the *new* `web/src` and is asserted by at least one E2E check.
4. One data-export port (orders/menu) produces a file an operator can download from a live deployment.
**Anti-scope:** Do NOT build any adapter before DELIVERY P37/P38 gives it a live order flow to notify about — a messenger port with nothing to send is dead code. Do NOT re-implement social posting here (P22 owns it). Do NOT touch the `tools/telemetry` Telegram bridge; it is OPS plumbing, not a product channel.
**Depends on / blocks:** Depends on DELIVERY P37/P38 (live order/courier flow) and PROTOCOL P34 (capability-gated egress). Blocks nothing on the critical path. QRNG bug fix (DoD-1) has no dependency and may land any time.

#### P44 — Cache Layers (EC-05) + Own-RAG / Own-Inference Scale-Out — LOW PRIORITY / FAR-FUTURE
**Absorbs:** EC-05; own-inference-beyond-Ollama, own-RAG, chunking, and gossip-flows-as-kernel-properties units of EC-03/04/06/08/12–15.
**Status:** PLANNED (0 of 5 cache layers built)
**Role & responsibility:** The ecosystem-strategy arc's flagged "only gap": five cache layers (embedding cache, Merkle re-index, prefix-disk tier, pipeline cache, semantic cache) plus eventual self-hosted inference/RAG scale-out. Verified current state: exactly **one** basic exact-match sha3-keyed cache exists (`llm-adapters/src/cache.rs`); none of the five planned layers, no own-RAG, no chunking pipeline.
**Blueprint:** none — source arc: `/root/.claude/projects/-root-dowiz/memory/ecosystem-strategy-arc-2026-07-13.md`. Do not write one yet.
**DoD (deliberately minimal — this is optimization work for a service that does not yet exist):**
1. A measured baseline exists (cache hit-rate + latency on real AGENT-loop traffic) *before* any layer is built — no layer ships without a number it improves.
2. Each layer lands only with a benchmark showing net win over the existing sha3 exact-match cache; a layer that doesn't beat it gets deleted, not kept.
**Anti-scope:** **This is explicitly NOT where swarm effort should go soon.** Do NOT build any cache layer before AGENT P40/P41 produces real inference traffic to cache — cache design against imagined workloads is the definition of premature optimization. Do NOT stand up own-inference infra while the existing Ollama port (already built, AGENT's scope) is unsaturated.
**Depends on / blocks:** Depends on AGENT P40/P41 (real traffic) and DELIVERY P37 (real product load). Blocks nothing; nothing waits on this.

#### P45 — Deployment + Monitoring Floor (minimum viable ops)
**Absorbs:** OPS-01..22 (entire ops-reliability arc).
**Status:** PARTIAL — but barely; the arc's own premise is gone
**Role & responsibility:** The minimum viable ops floor for whenever DELIVERY has something live: deploy path, dead-man's-switch monitoring, backup with off-site immutability, secrets handling. Honest inventory of what actually exists versus what the arc assumed:
- **Real:** `.github/workflows/heartbeat-monitor.yml` — a genuine external dead-man's-switch (polls `webhook.dowiz.org` every 10 min, Telegram-alerts on failure). Caveat: it watches a Cloudflare Tunnel webhook endpoint, **not "the app"** — there is no app running. The *pattern* is proven; the *target* doesn't exist yet.
- **Real:** `kernel/src/backup.rs` (702 lines, Buzhash-CDC dedup) — a native backup primitive, but a **different design** than the arc's WAL-G/rsync.net proposal, and never exercised end-to-end (nothing to back up yet).
- **Real-but-not-this:** `tools/telemetry/` (hetzner-exporter etc.) is the self-improvement loop's own harness telemetry — not a product-facing metrics stack. No Prometheus/`remote_write` anywhere.
- **Not built:** zero VictoriaMetrics / Grafana / Netdata / Gatus / SOPS / WAL-G / OpenTofu / Dokploy / PgBouncer / Cloudflare-Tunnel-config in the repo — all future-tense doc mentions only. `docs/ops/P8-SINGLE-PANE-SPEC.md` self-labels every signal `[SPEC]` and states plainly: "No canonical prod target exists."

**Superseded — resolve explicitly:** the arc's RLS-fix approach was "resurrect attic's 140 TS migrations." That path is **dead twice over**: (a) `attic/` is physically deleted, so the premise no longer exists; (b) it is **formally superseded** by `docs/design/BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` (committed 2026-07-18), whose §0 states "NOT a TS/Supabase migration… the old attic/packages-db 140 migrations are quarantined and dropped; we do not revive them," and whose §5 DECART table formally rejects attic-revival in favor of a native Rust/sqlx adapter. **The native pgrust rebuild is current canon.** It is already registered in `CORE-ROADMAP-INDEX.md` §7 as a separate red-line track, gated on operator `/council` review — **cross-referenced here, not renumbered into P45.**

**Blueprint:** `docs/ops/P8-SINGLE-PANE-SPEC.md` (monitoring, `[SPEC]`), `docs/design/BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` (data layer, separate gated track). Reuse both; write nothing new until unblocked.
**DoD:**
1. A deploy artifact exists and is reachable at a canonical prod URL (falsifies "no canonical prod target exists").
2. The heartbeat dead-man's-switch is retargeted from the tunnel webhook to the live app's health endpoint, and a deliberately induced outage produces a Telegram alert within 10 minutes.
3. `kernel/src/backup.rs` exercised end-to-end against real tenant data: backup → restore → byte-identical verification.
4. **Off-Hetzner immutable backup exists** — the arc's own #1 flagged risk, still completely unaddressed. A restore from the off-site copy succeeds with Hetzner unreachable. This is the one item that stays red until proven.
**Anti-scope:** **This phase is BLOCKED — not merely sequenced after — on DELIVERY P37 existing.** Do not stand up VictoriaMetrics/Grafana/any observability stack before there is a service emitting signals; do not write OpenTofu/Dokploy config for infrastructure that hosts nothing; do not revive attic migrations (canon forbids it and the files are gone); do not fold the pgrust rebuild into this phase's numbering (it is an operator-gated red-line track in CORE-ROADMAP-INDEX §7).
**Depends on / blocks:** Hard-blocked by DELIVERY P37 (something to deploy). Data-layer items depend on the pgrust tenant-rebuild track clearing its `/council` gate. Blocks P46 (no multi-product platform without a deployed first product). DoD-4 (off-site backup) should be first in line the moment P37 produces state worth protecting.

#### P46 — Multi-Product Platform: "dowiz Local" + Marketplace — FURTHEST FUTURE
**Absorbs:** EC-17 and the multi-product/marketplace remainder of the ecosystem-strategy arc, including "dowiz Local" (the planned second product intended to prove multi-product reuse — never shipped, zero grep hits in the repo).
**Status:** PLANNED (0%)
**Role & responsibility:** The ecosystem endgame: prove the CORE/INFRA/FLOWS decomposition by shipping a second product on the same kernel, then (and only then) generalize toward a marketplace. Nothing exists; nothing should, yet.
**Blueprint:** none — source arc: `/root/.claude/projects/-root-dowiz/memory/ecosystem-strategy-arc-2026-07-13.md`. No blueprint until the gate below is met.
**DoD:**
1. A second product ("dowiz Local" or successor) runs on the unmodified kernel with zero kernel forks — falsified by any product-specific patch to CORE.
2. Reuse is measured, not asserted: the second product's non-kernel code line count is published against the first product's.
**Anti-scope:** **Do not start this before a single product has second-tenant proof** — the original EC plan's own sequencing wisdom warned against building marketplace infrastructure before proving reuse empirically, and that warning is honored here as a hard gate, not a suggestion. No marketplace scaffolding, no plugin registry, no partner API before DoD-1 of this phase is even startable, which itself requires DELIVERY P37/P38 live with real tenants.
**Depends on / blocks:** Depends on literally everything above: DELIVERY P37/P38 live, P45 ops floor green (including off-site backup), P43 at least one working external port. Blocks nothing — it is the terminal node of the entire roadmap.
