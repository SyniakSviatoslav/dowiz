# Roadmap-wide Blueprint-Gap Audit — 2026-07-19

> **Scope.** Apply the meta-gap-audit lens (used today on P75–P96 + Q-series) to the **entire**
> project roadmap, not just today's wave: find every roadmap item that is referenced / planned /
> tracked but has **no corresponding full written blueprint file**. Audit-only — zero product code,
> no blueprints written, no branches touched. This doc is the sole artifact.
>
> **Method (falsifiable, not asserted).** (1) Read `CORE-ROADMAP-INDEX.md` in full + navigated
> every phase-index section of `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` (§2, §8.1–8.12,
> §10.2, §11–§19). (2) Enumerated all 86 on-disk `BLUEPRINT-P*.md` files + the non-`P##`-named phase
> blueprints across `docs/design/` and its arc subdirectories. (3) Machine-checked **every** markdown
> blueprint link in both the index and the master roadmap for a resolving file on disk. (4) Cross-referenced
> each P-number, W-number, Q-number, and letter-arc ID (DK/FE/DZ/RW/IP/EC/OPS/MESH/BP/S-series) against
> its blueprint home. (5) Grep-swept the whole corpus for "blueprint pending / to write / unwritten /
> missing" markers. Every existence claim below was checked against the working tree on 2026-07-19.

---

## 0. Headline — the honest result

**Blueprint coverage of the roadmap is effectively complete.** After today's P75–P96 wave landed,
there is **no phase-level blueprint gap left**. Concretely:

- **Every P-number P01–P96 either has a real blueprint file on disk, or is P84 — a deliberately
  reserved, operator-gated number with no file by design.** No exceptions.
- **Zero dangling references.** Every `.md` blueprint link in `CORE-ROADMAP-INDEX.md` and in the
  286 KB master roadmap resolves to a file that exists. The index's own honesty rule ("rows marked
  OPEN/MISSING are never linked to files that don't exist") holds under machine check.
- **The letter-arcs (DK/FE/DZ/RW/IP/EC/OPS/MESH/BP/S-series) are covered by design** — each arc was
  consolidated into a P31–P46 phase blueprint (index §9 mapping), rather than each of its ~150 units
  getting a standalone document. That is the intended model, not a gap.

**Genuine category-(a) gaps: 1 small item** (a promised disposition *audit*, not a feature blueprint).
Everything else resolves to category (b) correctly-deferred or (c) done. This is a "good news" audit;
per the standing anti-manufacture directive it is reported as such rather than inflated into a work list.

**Count summary**

| Category | Count | What |
|---|---|---|
| (a) Genuinely missing | **1** (small) | The P33b-style disposition audit for un-homed arc sub-units (MESH-14, IP-21, IP-01..07/09/17/18) |
| (b) Correctly deferred / rejected | ~9 classes | P84 reserved; DZ-10 voice; P53 arti-hosting + mesh-over-Tor; P54 fine-tuning; pgrust rebuild (proposal, "no separate blueprint needed"); P30-ledger REJECT-on-physics set; 5 NO-TARGET closed scans; assorted named-trigger defers |
| (c) Already done / shipped | large | P01/P02/P07/P18/P19 merged; CORE P31–P33 ~90%; P34–P42/P47/P49 partial-with-commits; W17–W22 green; all 22 P75–P96 blueprinted this wave |
| Not a blueprint-absence gap (flagged separately) | 4 | META-GAP-AUDIT G2–G5 — DoD-completeness / prerequisite-tracking fixes **inside** existing blueprints, plus one stale doc marker |

---

## 1. The blueprint-bearing universe and its coverage

### 1.1 P-series P01–P96 — full mapping (all resolve except reserved P84)

| Range | Blueprint home | Status |
|---|---|---|
| P01–P19 | `sovereign-roadmap-2026-07-16/BLUEPRINT-P01..P19-*.md` | ✅ all 19 present |
| P20 | `DEMO-MARKETING-PIPELINE-REFACTOR-2026-07-17.md` | ✅ (non-`P##` name; roadmap §8.1) |
| P21 | `CORE-ROADMAP-2026-07-17/BLUEPRINT-P21-local-llm-hermes-native.md` (+ `LOCAL-AI-LOCAL-AGENTS-RESEARCH-2026-07-17.md`) | ✅ |
| P22 | `BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md` | ✅ |
| P23 | `BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` | ✅ |
| P24 | `BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17.md` | ✅ |
| P25 | `BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md` | ✅ |
| P26 | `BLUEPRINT-MEMORY-OPTIMIZATION-FLOW-ANALYSIS-2026-07-17.md` | ✅ |
| P27 | `BLUEPRINT-FAULT-ISOLATION-DECENTRALIZED-ARCHITECTURE-2026-07-17.md` | ✅ |
| P28 | `BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md` | ✅ |
| P29 | `BLUEPRINT-LATENCY-ELIMINATION-RESEARCH-AND-BRAINSTORM-2026-07-17.md` | ✅ |
| P30 | `bebop2-mesh-tensor-hermetic-2026-07-17/BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS.md` (+`-V2`, `INDEX.md`) | ✅ |
| P31–P83 | `CORE-ROADMAP-2026-07-17/BLUEPRINT-P31..P83-*.md` (incl. `P34B`, combined `P47-P50`, own `P48`) | ✅ all present |
| **P84** | **none — RESERVED, operator-gated** | **category (b) — see §3** |
| P85–P96 | `CORE-ROADMAP-2026-07-17/BLUEPRINT-P85..P96-*.md` | ✅ all 12 present |

**Note on the "P20–P30 are standalone files" trap.** The index (§1) warns that P20–P30 "exist only as
standalone blueprint files indexed from SOVEREIGN §8.1–§8.12" — i.e. they are **not** named
`BLUEPRINT-P20..P30`. A naive `find BLUEPRINT-P2*` returns only P21 and looks like nine missing phases.
Resolving each via §8.1–§8.12 shows all eleven map to real, differently-named files (table above). This
was the single most likely false-positive in the audit; it is a naming convention, not a gap.

### 1.2 W-series, Q-series, cross-cutting arcs

| Track | Blueprint home | Status |
|---|---|---|
| W17–W22 (FINISH-ALL-6 swarm) | `BLUEPRINT-W17..W22-*.md` | ✅ all 6 present (no W1–W16 exist as separate blueprint-needing items — they are internal P30 lane labels) |
| Q1–Q4 governance | `CORE-ROADMAP-2026-07-17/BLUEPRINT-Q-SERIES-VERIFICATION-OBSERVABILITY-2026-07-19.md` | ✅ single doc covers all four |
| Agentic mesh (B1–B4) | `agentic-mesh-protocol-2026-07-17/AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md` | ✅ |
| Spectral evolution (E1–E3) | `spectral-energy-flow-evolution-2026-07-16/SPECTRAL-EVOLUTION-CONSOLIDATED.md` | ✅ |
| Living Interface (feeds P16/Layer G) | `living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md` (+ `BLUEPRINT-P07/P08` sonification/viz) | ✅ |
| Fail-operational round-2 | `fail-operational-layout-versioning-2026-07-17/round-2/BLUEPRINT-ROUND-2-MASTER-SYNTHESIS.md` | ✅ |
| Native pgrust tenant rebuild | `BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` | ✅ (proposal; doc states "no separate blueprint needed") |
| Layer A–I roll-ups | `CORE-ROADMAP-2026-07-17/BLUEPRINT-P-A..P-I-*.md` | ✅ all 9 present |

### 1.3 Letter-arcs → phase-blueprint homes (index §9) — covered by design, not individually

| Arc (unit range) | Absorbed into (blueprint) | Coverage |
|---|---|---|
| field-ui-engine — FE-01..17 | P38 (`BLUEPRINT-P38-webgpu-render-engine.md`) | ✅ phase blueprint |
| dowiz-interfaces — DZ-01..12 | P38 (P38b) | ✅ (DZ-10 voice deferred → §3) |
| rust-engine-rewrite — RW-01..12 | P37 + P38 | ✅ |
| docker-swap — DK-01..10 | P35 (`BLUEPRINT-P35-docker-swap-wasm-runtime.md`) | ✅ |
| mesh-real — MESH-01..13 | P34 + P34B | ✅ (MESH-14 un-homed → §2) |
| ops-reliability — OPS-01..22 | P45 (`BLUEPRINT-P45-ops-security-monitoring.md`) | ✅ |
| ecosystem-strategy — EC-* | P44 + P46 | ✅ |
| integration-ports — IP-* | P42 + P43 (+ existing P22) | partial (IP-01..07/09/17/18/21 un-homed → §2) |
| math-first — S0–S7 | P31 (`BLUEPRINT-P31-math-first-residuals.md`) | ✅ |
| hydraulic-loop-v2 — BP-01..23 | P32 + P33 (`BLUEPRINT-P33-core-ledger-hygiene.md`, audit/flag) | ✅ (BP-03/04/11/12–21/23 status-audited by P33, covered) |

The consolidation model is explicit and load-bearing: an arc unit does **not** earn a standalone
blueprint; it earns a line-item inside its phase's blueprint. Under that model every arc unit with a
phase home is covered. The only exceptions are the units §10.0 itself flags as having **no** phase home.

---

## 2. Category (a) — genuine gaps

**One item, small, and it is an audit rather than a feature blueprint.**

### GAP-A1 — the un-homed arc-unit disposition audit ("P33b-style follow-up")

- **What it is.** Master roadmap §10.0 (integrity-check notes) explicitly names arc units that were
  **not** absorbed into any P31–P46 phase during the 2026-07-18 assembly and deliberately were *not*
  invented into scope:
  - **MESH-14** — resolve-contradictions + RED-suite + status-from-live-test CI-lint (mesh-real's 14th
    unit; confirmed present in `mesh-real/BLUEPRINTS-MESH-REAL.md`, unnamed by any §10.5 draft).
  - **IP-21**, and **IP-01..07 / IP-09 / IP-17 / IP-18** — asserted by §10.5.5's absorption ledger to be
    PROTOCOL/AGENT/CORE scope, but only IP-08 is *explicitly* absorbed (→ P42). The rest are asserted,
    not placed.
- **Why it is category (a) and not (b).** The units are named and their disposition is deferred to "a
  P33b-style follow-up audit" — but that audit **does not exist on disk**, so the deferral has no
  artifact. Nothing tracks whether these ~9 units are real work, already-done, or dissolved. It is a
  genuine coverage hole, just a shallow triage-shaped one.
- **What it should cover.** A single short audit doc that, per unit, rules: absorbed-into-existing-phase
  (name it) / dissolved-with-reason / genuinely-new-work (assign a phase). No new mechanism; this is the
  same disposition pattern §9's would-be-lost ledger already used for L1–L6.
- **Scope/complexity: SMALL.** ~9 units, all already located in source arc docs; one research+ruling pass,
  likely folds into P33 (`BLUEPRINT-P33-core-ledger-hygiene.md` is already the "cleanup + status audit"
  phase — this is arguably a P33 §-addendum, not even a new file).

That is the complete category-(a) list. There is no second genuine gap.

---

## 3. Category (b) — correctly deferred / rejected (do NOT manufacture blueprints for these)

These are explicit, reasoned "not now / not ever, here's why" decisions. They are healthy as-is; writing
blueprints for them would be inventing work the roadmap deliberately excluded.

- **P84** — reserved number for the "golden state-digest regression gate" (D-1). **NEEDS-OPERATOR-DECISION**,
  deliberately unproposed because it touches money/FSM red-line surfaces (ledger §5 OD-7; master roadmap
  §19.1; SYNTHESIS-PERFORMANCE-AUDIT §4 D-1). No file is the *correct* state until ruled.
- **DZ-10 voice** — Phase-9b-deferred, not lost (index §9).
- **P53 Tor** — the phase is blueprinted, but *within* it: arti-hosting deferred on the Tor Project's own
  maturity warnings; mesh-transport-over-Tor deferred with a named trigger. Correct partial-defer.
- **P54 fine-tuning** — explicitly DEFERRED against the operator's own glossary criteria; zero LoRA/QLoRA
  by design.
- **Native pgrust tenant rebuild** — proposal-only, red-line, `/council`-gated; its own §0/§4 carry DoD +
  anti-scope and the index states "no separate blueprint needed."
- **P30 verdict-ledger REJECT-on-physics set** — batch-accept crypto, reputation/scoring/watchdogs/proxies,
  distributed-inference, Mixtral/MoE, etc. — rejected with citations; re-proposing is re-litigation.
- **The 5 NO-TARGET closed scans** (master roadmap §19.1) — BitNet, QKD, fraud-scoring, bit-slicing,
  energy-currency — "no target for the technique exists here, often by standing policy," each with a named
  reopening trigger. Ledger §2 explicitly warns: do not re-litigate a NO-TARGET item as "never investigated."

---

## 4. Category (c) — already done / shipped (no blueprint needed; they're implemented)

High level (per master roadmap §10.2 status cells, §8.4, MEMORY ground-truth, live commit hashes):

- **Merged to main:** P01, P02, P07, P18, P19 (P18 = public-flip prep).
- **CORE ~90%:** P31 (S0/S1/S2/S4 DONE), P32a DONE, P33 audit-only. Dominant failure mode is
  built-but-unwired code, not missing plans.
- **Partial with landed commits (2026-07-18 swarm):** P40 (`agent/loop.rs`), P41 (`ports/llm.rs` AiMode),
  P42 (`ports/mcp.rs`+`tool.rs`), P47 (`ports/payment.rs`), P49 (`ports/customer.rs`), G3 render shell.
- **W17–W22:** green (FINISH-ALL-6 swarm).
- **This wave:** all 22 P75–P96 blueprints written + registered; Q-series written. G1 (P95/P96 ledger
  orphan) closed — both now carry index rows.

---

## 5. Not a blueprint-absence gap — but flagged for honesty (already tracked)

These are **not** category (a): the blueprints exist. They are DoD-completeness / prerequisite-tracking
fixes **inside** existing files, already caught by `META-GAP-AUDIT-2026-07-19.md` (findings G2–G5). Listed
so this audit doesn't silently imply they're clean, and so no one double-counts them as "missing blueprints."

- **G2 (HIGH)** — P86/P87/P88 omit P38's mandatory WebGL2/CPU-floor DoD line; P88's atomics policy is
  silent on the fallback rung. Fix = add the DoD line + a scope clause. *Blueprint edit, not a new doc.*
- **G3 (HIGH)** — no blueprint *owns* the same-runner A/B bench-regression CI job for the dowiz `engine`
  crate or the bebop repo; P75's schema is complete but its running gate is kernel-only. This is the one
  finding that edges toward a real coverage hole, but the remedy is a **scope amendment to P75/P81/P82**,
  not a new blueprint. Already has a written recommendation.
- **G4 (HIGH)** — P91's FIPS-203 conformance spine names ML-KEM ACVP vectors that don't exist in-repo and
  pins no provenance. Fix = track "acquire+pin vectors + write KEM loader" as an explicit P91 prerequisite.
- **G5 (MED–HIGH)** — no crypto blueprint (P85/P91/P92) mandates a constant-time / `dudect` check. Fix =
  add one review line to each §3/§8 surface.

Plus one **doc-hygiene** staleness (not a gap, flag-only, not fixed here per write-scope):
`SYNTHESIS-MESH-MAJOR-REFACTOR-PLAN-2026-07-19.md` §§368–369 still say P93/P94 "Full blueprint not yet
written" — both files now exist on disk (773 / 554 lines). The synthesis line predates the blueprints by
minutes and should be corrected when that doc is next touched.

---

## 6. Prioritization recommendation

Because the genuine gap surface is tiny, prioritization is short and should **not** be treated as a
multi-item backlog:

1. **First / only real "write" item — GAP-A1 (SMALL).** Fold the un-homed-unit disposition into a P33
   addendum (or a one-page P33b audit). ~9 named units, all locatable in existing arc docs; a single
   research+ruling pass. Do this because un-homed units are the one place the "nothing lost" invariant is
   currently unproven.
2. **Then the four META-GAP DoD fixes (G2–G5) — blueprint *edits*, not new blueprints.** Sequence by
   build-readiness: G3 (unblocks P81/P82 gate DoDs) and G4 (unblocks P91's compliance proof) are on the
   critical path of items that will actually be built soon; G2 is build-gated behind the OD-11 GPU decision
   so it can wait for that ruling; G5 is a cheap one-line hardening on each crypto surface. These belong to
   the **Q1 claim-verification gate** the Q-series already designed — route them there, don't spawn a new
   process.
3. **Everything else: leave exactly as-is.** P84 and the category-(b) set are correctly deferred/rejected;
   touching them manufactures work the roadmap deliberately excluded. The correct action for the doc-hygiene
   staleness (P93/P94 markers) is a one-line correction on next edit, not a task.

**Bottom line:** the roadmap does not have a blueprint-coverage problem. It has (a) one shallow disposition
audit owed on ~9 orphan arc-units, and (b) four in-place DoD refinements already tracked by today's meta-gap
audit. There is no large hidden backlog of unblueprinted phases; the P01–P96 + W + Q + arc structure is
covered end-to-end.

---

*Audit performed 2026-07-19 against the working tree. Read-only: no blueprints written, no branches touched,
no product code. Sources: `CORE-ROADMAP-INDEX.md`; `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`
§2/§8/§10/§19; `CORE-ROADMAP-2026-07-17/{MASTER-STATUS-LEDGER,META-GAP-AUDIT}-2026-07-19.md`; on-disk file
enumeration + machine link-resolution of every blueprint reference in the index and master roadmap.*
