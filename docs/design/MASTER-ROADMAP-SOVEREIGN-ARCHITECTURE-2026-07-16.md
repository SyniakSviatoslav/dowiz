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
> `MASTER-INTEGRATION-PLAN-2026-07-14.md`, and `MASTER-ROADMAP-MVP-2026-07-12.md` (root) — all
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

---
