# BLUEPRINT P33 — CORE ledger hygiene: dead-artifact flag + hydraulic BP status audit (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §9). This phase IS
> `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.1's **P33** (sub-letters
> P33a–P33b). It is pure housekeeping — the lightest phase in CORE, proportioned accordingly:
> P33a flags one dead artifact directory for deletion; P33b is an AUDIT whose deliverable is a
> live-verified status table, not code. Lower urgency than everything on the critical path.
>
> **Headline of this pass:** the P33b audit was **partially executed while writing this
> blueprint** — §3.2's seed table carries fresh `file:line` evidence for 13 of the 14 BP items
> in scope, gathered 2026-07-18. What remains for the executor is classification-completion
> (consumer checks + per-item sweeps), not a from-zero sweep. Two items already resolve to
> other phases (BP-18 → P32b, BP-19 wiring → P32c) rather than to new work.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Verified 2026-07-18 against `dowiz` `main` @ `76167336a7b2ed31fee38d7161d109d462763643`
(docs-only working-tree modifications) and `bebop-repo` `main` @
`e56ba6a35258ced76752510625511f37a6367a77` (clean). Bebop paths relative to
`/root/bebop-repo/`; dowiz paths relative to `/root/dowiz/`.

| # | Claim | Fresh evidence (this pass) | Inherited (§10.5.1) | Status |
|---|---|---|---|---|
| 1 | P33a target exists | `spikes/living-knowledge/out/` = **7.7 MB**, 6 files: `semantic-cache.json`, `eval-results.json`, `eval-memory-results.json`, `eval-memory-semantic-results.json`, `eval-telemetry.jsonl`, `comparison-report.md` (fresh `ls`/`du`) | §10.5.1: "semantic-cache.json (7.7 MB) plus eval-result JSON files" | **MATCH** |
| 2 | Zero consumers of `out/` | `grep -rn "living-knowledge/out"` across `*.yml/*.yaml/*.sh/*.toml/*.rs/*.json` repo-wide (docs excluded) → **0 hits** (fresh) | §10.5.1 DoD-1 asks for exactly this check | **PRE-SATISFIED** — the DoD-1 grep already ran green this pass; the executor re-runs it at deletion time (cheap, and guards drift between passes) |
| 3 | Out-of-flag residue in the same spike dir | `spikes/living-knowledge/`: `lib/` **EMPTY**, `node_modules/` **41 entries**, `README.md` 8932 B (fresh `ls`) | not in §10.5.1 | **NEW observation** — NOT under P33a's flag (anti-scope: one target, one commit); recorded as a candidate for a separate operator decision line, nothing more |
| 4 | JS-spike source is dead | source deleted per `f9ab28ff1` "drop ALL JS/TS per operator" (inherited commit cite; consistent with §0 row 3 — only artifacts + empty lib + node_modules remain) | §10.5.1: same commit | inherited-accepted |
| 5 | P33b's authoritative BP list | `docs/design/hydraulic-loop-v2/BLUEPRINTS.md` — per-item headers live at `:227` (BP-03), `:273` (BP-04), `:587` (BP-11), `:629` (BP-12), `:656`..`:903` (BP-13..21, BP-23); its summary status block `:70-85` still shows 🔴 for items with landed code (e.g. BP-07 🔴 at `:77` vs the live consumer chain in P32 §0 row 5) | §10.5.1: "the authoritative BP list to audit against" | **MATCH with the standing caveat** — cite it for *design*, never for *status* (same stale-table class as mesh-real, P34 §0 row 20) |
| 6 | Seed evidence for the audit itself | §3.2's table — every cell gathered by fresh grep/ls this pass; per-row cites inline there | §10.5.1: "not individually verified this session" | **PARTIALLY CLOSED THIS PASS** — 13/14 items now carry at least one live cite; classification (DONE / WIRING-GAP / OPEN) still requires the §3.2 completion checks |
| 7 | BP-22 out of audit range | resolved differently (TS port deleted; `agent-governance-wasm` successor) — indexed in P32a | §10.5.1: same | MATCH — range stays BP-03, BP-04, BP-11, BP-12..21, BP-23 |

Ground truth is non-discussible; everything below builds on the fresh column only.

---

## 1. Scope — two sub-letters, and what P33 deliberately does NOT own

**P33's single sentence:** delete one confirmed-dead artifact directory (operator-flagged, one
revertable commit) and finish classifying the 14 unconfirmed hydraulic BP items against live
source so no unit is silently dropped — zero product-code changes in either sub-letter.

| Sub | Absorbs | Status (fresh, §0) | Character |
|---|---|---|---|
| P33a | JS living-knowledge spike closure (A2(mip) lineage; live replacement = P31a's native retrieval) | FLAG-READY (rows 1-2) | one verification grep + one commit (or a recorded keep-decision) |
| P33b | BP-03, BP-04, BP-11, BP-12..21, BP-23 status audit | SEEDED (row 6, §3.2) | classification completion + roadmap write-back |

**Anti-scope (binding, from §10.5.1 verbatim plus this pass's additions):**
- P33a: do not resurrect any part of the JS spike; delete nothing outside
  `spikes/living-knowledge/out/` under this flag; not a license to sweep other directories —
  one target, one commit. The `node_modules/`+empty-`lib/` residue (§0 row 3) gets its own
  operator decision line, NOT a rider on this one.
- P33b: **audit only — zero code changes during the audit itself** (§10.5.1 DoD-3). Do not
  fix, wire, or build anything found; file findings back into the roadmap. Do not mark
  anything DONE without `file:line` evidence — the BP-01 "still frozen" staleness §10.5.1
  cites is exactly the guarded failure mode, and §0 row 5's stale 🔴 table is its mirror
  image (stale in the pessimistic direction; both directions are lies).
- Do not add new P32 sub-letters yourself — WIRING-GAP findings are *proposed* for fold-in
  (§10.5.1 DoD-2); the roadmap edit is the lead's/operator's merge.
- Deletion itself (P33a step 2) is an operator/lead call — this phase's authority ends at the
  verified flag + the prepared revertable commit.

---

## 2. Predefined types & constants (standard item 4)

None. Both sub-letters are docs/process work introducing zero domain concepts. The only
"schema" is the audit table's row shape, fixed here so every row is comparable:
`| BP | Item (one line) | Evidence (file:line, fresh) | Classification | Consumer check | Disposition |`
where Classification ∈ {DONE, WIRING-GAP, OPEN, MOOT, MOVED} and Disposition names the owning
phase or the explicit park/build decision. Stated as spec; nothing to implement.

---

## 3. Build items (items 3, 5 — adapted: the "tests" here are verification commands, stated per item)

### 3.1 P33a — Dead JS-spike artifact deletion flag

**Procedure (spec → check → act, the audit analogue of spec→RED→code):**
1. Re-run the consumer grep at execution time (the §0 row 2 command, verbatim) — must still be
   0 hits. This is DoD-1 and it is the RED-analogue: a non-zero result ABORTS the deletion and
   files the consumer as a finding.
2. Delete `spikes/living-knowledge/out/` in a **single revertable commit** (nothing else in the
   commit — the revert must restore exactly the artifact set, §0 row 1's file list).
   Alternatively, record an explicit operator decision to keep — either outcome closes P33a.
3. Write one decision-request line for the out-of-flag residue (§0 row 3: `node_modules/`,
   empty `lib/`, `README.md`) into the PR/roadmap note — decision requested, not taken.

**Adversarial case (item 5, adapted):** before committing, run the grep with a deliberately
widened pattern (`living-knowledge`) to catch consumers referencing the directory by a parent
path or alias — a narrow-pattern-only check could green while a consumer reads
`spikes/living-knowledge/` wholesale. Both patterns must be 0-hit outside docs/memory.
Anu/Ananke discipline: confirmed-dead legacy is actually deleted — but only after the check
that makes "confirmed" true, twice.

### 3.2 P33b — Unconfirmed hydraulic BP status audit (seed table, this pass)

Evidence gathered live 2026-07-18. **Classification is NOT final for rows marked "pending" —
completing them is the executor's deliverable.** Rows with strong evidence state it plainly;
none of this table's DONE-evidence becomes an official DONE until its consumer check column is
green (the §10.5.1 rule: file:line for DONE/WIRING-GAP, explicit "no code found" for OPEN).

| BP | Item | Evidence (fresh, this pass) | Classification (seed) | Completion check remaining |
|---|---|---|---|---|
| 03 | Francis QR / general eigensolver | `bebop2/core/src/lyapunov.rs:395` `pub fn eigenvalues_general(a,n) -> Vec<Complex>`; consumed by `bebop2/core/src/dmd.rs:332,387,397` | **DONE-evidence** (built + consumed) | confirm test coverage in `lyapunov.rs`; then DONE |
| 04 | Diffusion sign fix (anti-diffusion) | `crates/bebop/src/coherence.rs:35-36` "BP-04 (variant A): integrate the HEAT equation u̇ = −coeff·L·u … NOT u̇ = +coeff·L·u"; RED→GREEN fixture `:157-158`. NOTE: original target "coherence.rs+field_active" — `bebop2/core/src/coherence.rs` does not exist; the landed site is `crates/bebop`. Corroborated independently by the E1 Laplacian-parity work (`kernel/src/incidence.rs`, spectral-evolution arc) | **DONE-evidence, target-MOVED** | verify the `field_active` half (second named target) landed or is moot; then DONE or split |
| 11 | Renormalizer (rate-distortion@0) | `crates/bebop/src/renormalizer.rs` exists; registered `crates/bebop/src/lib.rs:67` "BP-11: claim-preserving, budget-crediting renormalizer (rate-distortion@0)" | **BUILT** | consumer grep (`renormalizer::` outside its file) → DONE vs WIRING-GAP |
| 12 | wiring.rs → strong hash-chained AuditLog | `crates/bebop/src/wiring.rs:21` `use crate::audit::AuditLog; // BP-12: strong SHA256 hash-chained log (was weak research_patterns::AuditLog)` — wired by its own self-description | **DONE-evidence** | confirm `audit.rs` chain-verify test exists; then DONE |
| 13 | memory.rs salience-weighted decay | `crates/bebop/src/memory.rs:18-38` — `salience: f64` field `:21`, decay-span comment `:38` | **PARTIAL evidence** | confirm decay path uses salience (not hash-lottery); classify |
| 14 | field.rs semantic field-veto | `crates/bebop/src/field.rs:1` "the deterministic graph-PDE arbiter (the \"physics veto\")", override/permit `:87,:99` | **PARTIAL evidence** (veto exists) | confirm the keyword-bypass it replaces is gone; classify |
| 15 | Connect guard-bash.sh (dead hook) | `find . -name "guard-bash*"` → **no file** (fresh, node_modules excluded) | **NO CODE FOUND** | likely MOOT-by-policy: all hook gates were emptied by operator directive 2026-07-15 (CLAUDE.md hooks block). Needs an explicit MOOT ruling recorded, not a silent drop |
| 16 | agentic_git.rs non-lossy snapshot | `crates/bebop/src/agentic_git.rs:233` "therefore lossless: commit → replay yields the EXACT same state"; test `:356-357` "BP-16 RED→GREEN: a node with rich metadata…" | **DONE-evidence** (with its own RED→GREEN) | run the named test once; then DONE |
| 17 | money.rs checked arithmetic **[RED-LINE]** | `crates/bebop/src/money.rs` does **not exist** (fresh `ls` of the full src dir) | **TARGET-ABSENT** | locate the current bebop money surface (candidate: proto-cap ledger payloads / delivery-domain money legs — both already integer-i64 per P34 §0) and rule DONE-elsewhere vs MOOT vs OPEN. RED-LINE: if any fix falls out, it is operator-gated, never done inside the audit |
| 18 | Mount resonator into the 6-layer loop | zero `resonator::` call sites repo-wide (P32 §0 row 3, fresh) | **WIRING-GAP** | **fold into P32b — it IS P32b's wiring item.** Do not create a new sub-letter; record the BP-18 → P32b mapping in the roadmap (§10.5.1 DoD-2's fold-in, satisfied by mapping to an EXISTING sub-letter) |
| 19 | Instrument panel L2 aggregation | `crates/bebop/src/instrument_panel.rs` built; registered `lib.rs:39` "BP-19: aggregate 8 instruments + 4 alarm bands"; **0 non-test callers of `report`** (P32 §0 row 5, fresh) | **BUILT, stranded** | wiring belongs to **P32c's chain completion** (dmd→panel→loop); map, don't duplicate |
| 20 | Orchestration state-machine + executable preconditions | `crates/bebop/src/pddl.rs` contains precondition machinery (weak-signal grep only) | **UNCLASSIFIED** | real check: does `pddl.rs`/`mission.rs` implement BP-20's spec (executable preconditions gating orchestration)? classify from `BLUEPRINTS.md:861` |
| 21 | Kalman measurement-update (noisy-quality fusion) | `crates/bebop/src/field.rs:324` `pub fn field_kalman(measurements, q, r)` returning (estimates, gains, innovations); named consumer `loop_runtime.rs:12` doc "L6 SENSE → `field::field_kalman` (measurement update)" | **DONE-evidence** (built + named consumer) | confirm `loop_runtime` calls it (import/call, not doc only); then DONE |
| 23 | Yellow batch (small robustness fixes) | `crates/bebop/src/coherence.rs:26,51,190` "BP-23 #5 (D2, fail-closed)" markers incl. a RED note | **PARTIAL** (≥ item #5 landed) | per-item sweep against `BLUEPRINTS.md:903`'s batch list; classify each |

**Adversarial case (item 5, adapted):** the audit's own teeth — for at least one row seeded
DONE-evidence, deliberately check the *pessimistic* source (`BLUEPRINTS.md:70-85`'s 🔴 table)
and record the contradiction explicitly (e.g. BP-07 🔴 at `:77` vs its live consumer chain).
An audit that only confirms its seed table has no teeth; one that documents where the old
status table lies, in both directions, does.

**Write-back (the actual deliverable):** the completed table replaces this seed in-place (this
file is the living location), and §10.5.1's P33b entry gets a one-line pointer + per-item
statuses. WIRING-GAP findings map to existing P32 sub-letters where they fit (rows 18-19) or
are proposed as new sub-letters for the lead to merge; OPEN items get an explicit build/park
decision line each — neither silently dropped (§10.5.1 DoD-2 verbatim).

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

Housekeeping phase — most obligations are honestly N/A and said so briefly rather than padded:

- **Hazard-safety as math (6):** the hazard here is epistemic, not runtime — stale status
  claims steering the swarm into duplicate or skipped work (the BP-01 precedent, §1). The
  structural guard is the audit's evidence rule (no DONE without file:line; no OPEN without a
  recorded "no code found") plus §3.2's both-directions teeth check.
- **Scaling axes (8):** N/A — no data shapes. The audit table scales in rows; it is bounded
  (14) and closed by charter.
- **Linux-discipline verdicts (9):** deleting unregenerable build artifacts with no source =
  **ALREADY-EQUIVALENT** (no generated files in the tree without their generator); the
  evidence-or-it-didn't-happen audit rule = **REINFORCES** the repo's ground-truth culture.
- **Isolation/bulkhead (11):** P33a's blast radius is one directory, one revertable commit;
  P33b's is zero (read-only). No shared-resource boundary exists to name.
- **Mesh awareness (12):** N/A — nothing here touches a node, payload, or transport. One line,
  as the charter allows.
- **Rollback (13):** P33a claims **Snapshot-Re-entry** in its most literal form — a single
  revertable commit IS the recovery mechanism; `git revert` restores the exact artifact set.
  No other rollback vocabulary is claimed.
- **Error-propagation gates (14):** the widened-pattern grep (§3.1) is the smart-index for the
  "hidden consumer" bug class; the audit row-shape (§2) makes an evidence-free status
  unrepresentable in the table.
- **Living memory (15):** one real connection — the dead spike is the *predecessor* of the
  living-memory organ (A2(mip) → native retrieval, P31 §0 row 5); deleting its artifacts is
  the demote-never-delete rule's explicit exception: these are unregenerable OUTPUTS with no
  source, not knowledge (the knowledge lives in the Rust port and the arc docs).
- **Tensor/spectral (16):** N/A for P33a. For P33b it is context only: several audited items
  (BP-03/04/07) are the spectral machinery — the audit classifies them, never touches them.

---

## 5. DoD — falsifiable, per item (item 2)

§10.5.1's DoD kept 1:1, with the seed-table head start recorded:

| Item | §10.5.1 | State before | DONE when | Command / falsifier |
|---|---|---|---|---|
| P33a-1 | DoD-1 | consumer grep green this pass (§0 row 2) but unre-run at act time | grep (narrow + widened pattern, §3.1) re-run 0-hit at deletion time | the greps, verbatim; non-zero = abort + finding |
| P33a-2 | DoD-2 | artifacts on disk (§0 row 1) | single revertable deletion commit landed, OR an explicit operator keep-decision recorded | `git show --stat` = exactly the 6 files + dir; falsified by any unrelated hunk in the commit |
| P33b-1 | DoD-1 | 13/14 rows seeded with evidence, 0 finally classified (§3.2) | every row of BP-03/04/11/12..21/23 classified with file:line (DONE/WIRING-GAP) or recorded "no code found" (OPEN/MOOT) | the completed §3.2 table; falsified by any row whose classification cell lacks a cite |
| P33b-2 | DoD-2 | rows 18/19 pre-mapped to P32b/P32c | every WIRING-GAP mapped to an existing/proposed P32 sub-letter; every OPEN has a build/park decision line | roadmap diff shows the write-back; falsified by a WIRING-GAP or OPEN row with an empty Disposition |
| P33b-3 | DoD-3 | — | zero code changes in the audit's commits (docs only) | `git show --stat` on audit commits: no `*.rs` |

Regression rows (item 17): none — no behavior changes, hence nothing to regression-pin
(stated as a reasoned exemption, not an omission). The audit's permanence mechanism is the
write-back into §10.5.1 + this file, which future passes re-verify rather than trust
(ground-truth-over-memory, standing).

---

## 6. Benchmark plan (item 10)

N/A — no hot path, no runtime change. The only measurable: repo size −7.7 MB on P33a
completion (recorded in the deletion commit message; not a benchmark, an inventory fact).
Stated as a reasoned exemption per the charter — inventing a benchmark here would be padding.

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.1 (the P33 charter; write-back
target) · `docs/design/hydraulic-loop-v2/BLUEPRINTS.md` (the audited list — design authority,
status-stale §0 row 5) + `HYDRAULIC-LOOP-v2-PLAN.md` ·
`BLUEPRINT-P32-hydraulic-loop-wiring.md` (receives BP-18/BP-19 mappings; supplied the fresh
resonator/panel strandedness evidence, its §0 rows 3/5) ·
`BLUEPRINT-P31-math-first-residuals.md` (P31a's native retrieval = the spike's live successor)
· `BLUEPRINT-P34-mesh-kernel-wiring.md` §0 row 20 (the cite-for-design-never-status precedent)
· `docs/regressions/REGRESSION-LEDGER.md` (item 17 — exemption recorded above). Memory:
`anu-ananke-strict-discipline-feedback-2026-07-17` (**the operative rule for P33a**: actually
delete confirmed-dead legacy, after verifying not CI-wired/codegen-input — §3.1 is that rule
as procedure) · `internal-retrieval-living-memory-arc-2026-07-14` (demote-never-delete and why
this is its sanctioned exception, §4 item 15) · `ground-truth-over-proxy-2026-07-07` (the
audit's evidence rule) · `never-bypass-human-gates-2026-06-29` (P33a step 2 + BP-17's RED-LINE
row: operator authority respected) · `hydraulic-loop-v2-arc-2026-07-13` (arc context).
Supersedes: nothing — §10.5.1 remains the charter; §3.2's completed table will supersede this
file's seed version in place.

---

## 8. Hermetic principles honored (item 20)

- **P6 CAUSE-AND-EFFECT:** every status claim in §3.2 is traceable to a command run on a named
  commit — no status exists without a cause a future reader can re-run.
- **P2 CORRESPONDENCE:** one status authority per item — the completed table + roadmap
  write-back replace the stale 🔴 block as the single mirror of live source, ending the
  two-tables-one-truth drift for these 14 items.
- (Others not load-bearing for a housekeeping phase; not claimed decoratively, per Anu/Ananke.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 — 7 rows fresh; §3.2 — 13 evidence cells gathered live this pass |
| 2 DoD | §5 — §10.5.1 kept 1:1, per-item falsifiers, seed head-start recorded honestly |
| 3 spec/TDD | §3 — spec→check→act procedure; the audit's row-shape spec precedes execution (§2) |
| 4 predefined types | §2 — none needed; the table row-shape is the only schema, stated |
| 5 adversarial cases | §3.1 widened-pattern grep; §3.2 both-directions teeth vs the stale 🔴 table |
| 6 hazard-safety | §4 — epistemic hazard, structural evidence rule |
| 7 links | §7 |
| 8 scaling axes | §4 — N/A, reasoned |
| 9 Linux discipline | §4 — two verdicts |
| 10 benchmarks | §6 — reasoned exemption, one inventory fact |
| 11 isolation | §4 — one-directory blast radius / read-only |
| 12 mesh awareness | §4 — honest N/A |
| 13 rollback vocabulary | §4 — Snapshot-Re-entry, literal revertable commit |
| 14 error-propagation gates | §4 — widened grep + evidence-required row shape |
| 15 living memory | §4 — the sanctioned deletion exception, argued not assumed |
| 16 tensor/spectral | §4 — N/A / classify-only, stated |
| 17 regression ledger | §5 tail — reasoned exemption |
| 18 agent instructions | §10 |
| 19 reuse-first | the audit reuses P32/P34's fresh evidence instead of re-grepping blind; zero new machinery |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Repos: `/root/dowiz` (P33a target + all docs) and `/root/bebop-repo` (P33b's audited sources —
READ-ONLY for this phase; push nothing there). Both sub-letters are independent, anytime,
lowest-priority lanes — never preempt P34/P37/P38/P40/P41 work.

1. **T1 (P33a).** Run both greps of §3.1 (narrow `living-knowledge/out` + widened
   `living-knowledge`) across `*.yml/*.yaml/*.sh/*.toml/*.rs/*.json`, docs/memory excluded.
   Both 0-hit → delete `spikes/living-knowledge/out/` in ONE commit containing exactly that
   deletion (commit message records the −7.7 MB and the two grep commands). Any hit → abort,
   file the consumer as a finding. Add one decision-request line for `node_modules/` + empty
   `lib/` (§0 row 3) — request, don't act. If the operator has recorded a keep-decision
   instead, link it and close.
2. **T2 (P33b).** Complete §3.2: for each row, run the "Completion check remaining" cell
   against live source at current HEAD (re-verify the seed cites too — this repo's swarm
   moves; the seed is a head start, not gospel). Fill Classification + Disposition per §2's
   row shape. Execute the teeth check (document at least one contradiction with
   `BLUEPRINTS.md:70-85`'s stale block, citing both sides). **Zero code changes** — if you
   find a one-line fix begging to be made, file it, don't make it (DoD P33b-3; BP-17's row is
   RED-LINE on top). Special rows: BP-18 → record the mapping to P32b; BP-19 wiring → P32c;
   BP-15 → propose the MOOT-by-policy ruling for operator sign-off; BP-17 → locate the live
   money surface before ruling, and touch nothing.
3. **T3 (write-back).** Replace §3.2's seed table in THIS file with the completed one (dated);
   update §10.5.1's P33b entry with per-item statuses + a pointer here; propose (not merge)
   any new P32 sub-letters for genuinely new WIRING-GAPs. Push docs after every milestone;
   fetch before push, never force.

**Stop-and-flag conditions:** (i) any consumer of `out/` found (abort deletion); (ii) any
impulse to fix/wire/build during the audit; (iii) any BP-17-adjacent money edit (RED-LINE,
operator-gated); (iv) seed cites failing re-verification (stale ground truth — re-audit that
row from scratch, and note the drift so the next pass knows the half-life).
