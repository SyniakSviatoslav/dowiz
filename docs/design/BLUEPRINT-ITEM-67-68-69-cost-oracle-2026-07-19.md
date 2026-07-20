# BLUEPRINT — Items 67 · 68 · 69: The Cost-Oracle Trilogy

> Three cost-oracle-shaped items in one doc (they share the same joules/PMU substrate and the same
> HOT-PATHS.tsv surface): **67** classification backfill (coverage-complete, precision-honest);
> **68** ORACLE-EXACT/BOUNDED cost capture as a correctness-proof byproduct; **69** water/carbon as
> derived, constant-multiplied views of joules. Each has its own §-headed plan, proofs, gates.

- **Date:** 2026-07-19 · **Tier:** enforcement-spine (roadmap §K) · **Status:** BLUEPRINT (planning
  artifact, no code).
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §K items
  67–69 (lines 1148–1199) + §K dependency line (1251–1252); `PROCEDURE-TELEMETRY-COMPLETENESS-STANDING-2026-07-19.md`
  steps 11–13 (the cost-oracle steps this arc makes mechanical); `docs/audits/hardening/HOT-PATHS.tsv`
  + `CHECKLIST.md`; `BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md` (the Bucket-B/C split reused as
  ready-made evidence); `BLUEPRINT-ITEM-27-pmu-classifier-input-2026-07-19.md` (the landed PMU work).
  Ground truth for code citations: this worktree at HEAD `6701bbb6f`.
- **Upstream (landed, cited live):** the PMU reader (`kernel/src/fdr/pmu.rs` — `read_tsc` :127,
  `bracket` :416, `delta` :98); RAPL energy field (`kernel/src/fdr/schema.rs:101` `joules_uj`, `:160`
  `read_joules_uj`); the item-7 native exhaustive sweeps + Kani harnesses (`HOT-PATHS.tsv:36-53`);
  `Reading<T>`/`Absence` named-absence machinery (`schema.rs:25` `Absence`, `:61` `Reading`).
- **Upstream (spec-level, gating):** item 57 (telemetry-completeness procedure + HOT-PATHS `eff`
  column) — procedure doc EXISTS, needs operator ratification; item 58 (work-normalized cost ledger)
  — no blueprint yet.

---

## 0. The one honest principle across all three — read this first

Literal "100% correct cost prediction for any code" is **undecidable** (WCET reduces to halting;
roadmap line 1149). The honest achievable form, applied uniformly here, is **100% *classification*
coverage with per-class honest precision**:

- `ORACLE-EXACT` — input domain enumerated OR cost provably input-independent (evidence = the
  enumeration / constant-time proof).
- `ORACLE-BOUNDED` — fixed operation schedule (evidence = the analytic `[min,max]` derivation).
- `MEASURED-ONLY` — genuinely dynamic / I/O / probabilistic (evidence = p50/p99/CI + methodology).

*Unclassified* is the one forbidden state. This mirrors `PROCEDURE-TELEMETRY-COMPLETENESS-STANDING`
steps 11–13 verbatim (lines 92–102 of that doc). Even at the EXACT end the claim is bounded honestly:
"input-dependence of cost fully characterized," absolute cycles remain a per-host interval because
measured cycles carry host noise (roadmap line 1177).

---

# Item 67 — Cost-oracle classification backfill

## 67.1 Scope / goal

Make every `HOT-PATHS.tsv` row (and every future row, gate-enforced) carry a cost bucket with a
traceable evidence pointer. This is the *coverage* deliverable — turn doc-6 §5.2's principle into a
mechanical gate, seeded from doc-6 §5.5's grounded sample. Non-goal: capturing new numbers (that is
item 68); this item classifies and points at evidence that already exists or is owed.

## 67.2 Current state (grounded)

The manifest (`HOT-PATHS.tsv`, read this session) has data rows for `pq/dsa`, `pq/kem`, `pq/keccak`,
`event_log`, `order_machine`, `householder`, `spectral`, `token_bucket`, `retrieval/pattern`,
`fdr/json`, `ct_gate` — each already carries `features/filter/min_tests/mode/checklist/gap` columns
(`HOT-PATHS.tsv:36-53`). **None carries a cost bucket today.** The item-7 blueprint already split the
crypto/FSM surface into Bucket B (native exhaustive → these are the EXACT/BOUNDED candidates) and
Bucket C (Kani interval proofs → BOUNDED), so the classification is largely a *relabel of work already
done*, not new analysis (roadmap line 1161: "Reuses the Kani-feasibility B/C split as ready-made
evidence").

## 67.3 Implementation plan (numbered)

1. **Add a `bucket` + `evidence` column to `HOT-PATHS.tsv`** (extend, never replace — item-6 gate
   idiom). Each existing and future row gets `ORACLE-EXACT | ORACLE-BOUNDED | MEASURED-ONLY` plus an
   evidence pointer (a test name / derivation section / measurement-doc anchor).
2. **Seed from doc-6 §5.5's grounded sample:**
   - FSM 144-transition table (`order_machine::`) → **EXACT** (evidence = the item-7 exhaustive
     144-pair sweep, `HOT-PATHS.tsv:43`).
   - `ct_eq` (`ct_gate`) → **EXACT**, inherited free from its dudect proof — the CT property *is* the
     cost-constancy property (`HOT-PATHS.tsv:49`, checklist=2).
   - `ntt`/`invntt`/`householder` → **BOUNDED** via fixed schedules (evidence = the item-7 Bucket-C
     butterfly-lemma / analytic interval, `HOT-PATHS.tsv:51`, `:44`).
   - `eigh` iterative QR + event-log fsync + subprocess/agent/AI → **MEASURED-ONLY** (evidence = item
     26's 637 µs distribution as the exemplar, `BLUEPRINT-ITEM-26`).
3. **Extend the gate** (`scripts/hardening-gate.sh`) to RED on a new hot-zone row lacking a bucket —
   the same planted-row anti-forgery clause item 6 built. *Unclassified* is RED.
4. **Every evidence pointer must resolve to a real test/derivation/measurement** — re-executed on
   spot-check, never presence-checked (P7, `CHECKLIST.md` §10/P7).

## 67.4 Required proofs (CHECKLIST 5-point) + acceptance

- **Item 1 (oracle):** the gate itself — zero unclassified rows in the extended TSV; a planted new
  hot-zone row without a bucket goes RED (red→green). **Item 5:** the EXACT/BOUNDED evidence pointers
  ARE the item-7 formal/exhaustive proofs (no new proof, a pointer). **Items 2/3/4:** N/A (this is a
  manifest+gate mechanization, not new algorithmic code).
- **Falsifiable acceptance:** (a) every `HOT-PATHS.tsv` data row carries a bucket; (b) the gate is RED
  on an unbucketed hot-zone row (demonstrated red→green); (c) a spot-checked evidence pointer
  re-executes green (not merely exists). **Falsifier for (c):** an evidence pointer that names a
  non-existent/failing test passes the gate → FAIL.

## 67.5 Dependency gate (honest)

**After item 57.** Item 57 extends `HOT-PATHS.tsv` with the `eff` accounting column and ratifies the
telemetry-completeness procedure; item 67 adds the *cost* column on the same surface. The procedure
doc EXISTS (`PROCEDURE-TELEMETRY-COMPLETENESS-STANDING-2026-07-19.md`) but item 57's mechanical half
(the `eff` column + operator ratification) is not landed. **67 is blocked on 57's column mechanism +
operator ratification of the procedure.**

---

# Item 68 — ORACLE-EXACT/BOUNDED cost capture as a correctness-proof byproduct

## 68.1 Scope / goal

The same structural property that makes correctness *exhaustively provable* makes cost *exactly
knowable* — so capture cost in the SAME pass, never a separate harness (doc-6 §5.3). This is the
*evidence-generation* deliverable behind item 67's EXACT/BOUNDED rows.

## 68.2 Current state (grounded)

- The `rdtsc` reader is landed and returns a real monotone counter on this x86_64 host
  (`fdr/pmu.rs:127` `read_tsc`; the `bracket` window helper `:416` already samples before/after and
  returns an absence-propagating `delta` `:98`; `tier_a_reads_real_nonzero_counters` proves it live).
- Item 7's Bucket-B exhaustive `#[test]` sweeps exist (`HOT-PATHS.tsv:36,41,43`), and the Bucket-C
  fixed-schedule functions (`ntt`/`invntt` 8-layer/1024-butterfly, Keccak 24 rounds) have their
  proven-schedule structure (`HOT-PATHS.tsv:51`, `pq/keccak.rs:50`).
- **Plane doctrine, already enforced:** PMU/cost values live on the P3 forensic plane, categorically
  excluded from every hash/gate/verdict (`fdr/pmu.rs:24-33`). Item 68 must not break this.

## 68.3 Implementation plan (numbered)

1. **(a) Tier-A cycle capture in the exhaustive sweeps.** Add `rdtsc` capture (reuse `pmu::read_tsc`
   / `bracket`) to item 7's Bucket-B exhaustive `#[test]` sweeps, folding to a single constant / tight
   interval where control flow is input-independent (the straight-line crypto reductions —
   `caddq`/`reduce32`/rounding) and to a complete per-input cost table otherwise.
2. **(b) Analytic `[min,max]` intervals for Bucket-C.** Derive the interval for the fixed-schedule
   functions (8-layer/1024-butterfly NTT, 24 Keccak rounds — the WCET-decidable straight-line
   subclass) by the same butterfly-lemma induction item 7 uses for correctness, reused for cost.
3. **(c) MEASURED-ONLY reports p50/p99/CI**, never a fabricated point estimate.
4. **Honest caveat carried verbatim** (line 1177): even ORACLE-EXACT yields measured cycles with host
   noise — the claim is "input-dependence of cost fully characterized," absolute cycles remain a
   per-host interval. Each captured EXACT value records its stated noise interval.
5. **P3 discipline preserved:** the captured cost values are recorded as item-67 evidence and feed NO
   decision/gate/hash surface — enforced by a grep proof (§68.4), identical to `pmu.rs`'s plane rule.

## 68.4 Required proofs (CHECKLIST 5-point) + acceptance

- **Item 1 (oracle):** a generated cost table/constant per classified function with its stated noise
  interval, recorded behind item 67's rows; an **input-independence assertion** for EXACT functions
  (the cost class is identical across the swept domain — e.g. cycle count within the noise band for
  every enumerated input). **Item 5:** the BOUNDED `[min,max]` derivation is the item-7 induction,
  reused. **Items 2/3/4:** N/A — capture rides the existing correctness sweeps; it is measurement, not
  a new timing-attack surface (and the dudect gate on `ct_eq` already covers the one CT path).
- **The load-bearing P3 proof:** a grep/structural assertion that **no captured cost value feeds any
  decision or gate surface** (the same proof `pmu.rs` carries — cost is recorded input, never a
  decision variable).
- **Falsifiable acceptance:** (a) each EXACT function has a captured cost table/constant + noise
  interval; (b) the input-independence assertion holds for EXACT functions (a planted
  input-dependent-cost variant fails it — red→green); (c) the P3 grep proof is green. **Falsifier for
  (c):** any gate/hash reads a cost value → FAIL.

## 68.5 Dependency gate (honest)

**After {item 67 + item 7's native sweeps}.** Item 7 is **landed** (`BLUEPRINT-ITEM-07` shipped; the
Bucket-B sweeps + Kani harnesses are in `HOT-PATHS.tsv:36-53`). Item 67 is not (blocked on 57). So
**68 is transitively blocked on item 57 → 67**; the item-7 leg is clear.

---

# Item 69 — Water/carbon as derived, constant-multiplied views of joules

## 69.1 Scope / goal

The kernel needs NO new *measured* footprint field beyond `joules_uj` — "atoms/molecules consumption"
honestly IS silicon power draw, i.e. joules, and item 27's RAPL/PMU work already is that mechanism
(roadmap line 1186). Build a **consumer-side** conversion layer keyed on operator-supplied
`(region, deployment-class)` constants. Small, standalone.

## 69.2 Current state (grounded)

- `joules_uj` is a first-class `Reading<u64>` in `HwStamp` (`fdr/schema.rs:101`), read from RAPL
  (`:160` `read_joules_uj`), degrading to a **named** absence on every failure mode
  (`NoRaplInterface`/`PermissionDenied`/`ReadError`/`NonLinuxHost`) — this host has no RAPL so it
  serializes `"joules_uj":{"unavailable":"no_rapl_interface"}` (`:14`, `:308-314`).
- The closed `Absence` enum has 6 variants (`schema.rs:25-43`) — none yet name "regional constant
  unsupplied" or "not software-observable."
- **No water/carbon field exists anywhere** (grep this session: zero `water`/`co2e`/`carbon` hits in
  kernel source outside unrelated "watermark" tokens).

## 69.3 Implementation plan (numbered)

1. **A consumer-side derivation module** (proposed `kernel/src/fdr/footprint.rs`), NOT a new HwStamp
   field. Adding raw `water_ml`/`co2e` to `HwStamp` is a **violation** (roadmap line 1194;
   fabricating a facility-cooling number software cannot observe is procedure step 4) — the module
   *derives* from `joules_uj`, it does not *store* a footprint.
2. **The two derived views**, each a `Reading<T>`:
   - `co2e = joules × grid-carbon-intensity` (gCO₂e/kWh), degrading to a named absence when
     `joules_uj` is absent OR the regional constant is unsupplied.
   - `off-site water = joules × WUE-source` (L/kWh), same degradation.
3. **On-site water = PERMANENT named absence** — unconditional by construction. A local device cannot
   observe facility cooling; there must be NO code path that produces an on-site-water value (roadmap
   line 1193). This is the strongest honesty clause in the item.
4. **Named absences for the new failure modes.** Two candidates: absent regional constant → a new
   `Absence` variant (e.g. `NoRegionalConstant`); on-site water not observable → a new variant (e.g.
   `NotSoftwareObservable`). Extending the closed `Absence` enum is a schema change — flagged in §69.5.
   (Alternative: a footprint-local reason enum to keep `Absence` crypto/hardware-only. Operator/executor
   ruling.)
5. **Lights up automatically on a RAPL-capable deploy with zero schema change** to `HwStamp` — the
   derivation reads the same `joules_uj` field; only the operator-supplied constants gate the value.
6. **Record the SCI-rate (ISO/IEC 21031) pairing note** for ratio consumers (roadmap line 1199) —
   documentation, not a stored ratio (ratios are a consumer concern; the losslessness law forbids a
   ratio field, item 58).

## 69.4 Required proofs (CHECKLIST 5-point) + acceptance

- **Item 1 (oracle):** derivation **golden tests against hand-computed values** — given a known
  `joules_uj` and a known `(region, deployment-class)` constant, `co2e`/`off-site-water` equal the
  hand-computed product. **Item 3:** debug cross-check of the multiplication against the golden.
  **Items 2/4/5:** N/A (integer/rational multiply, no secret timing, no branch-free crypto, no
  non-enumerable property — the golden IS the proof).
- **The named-absence proofs (procedure step 10 red→green):** on this RAPL-less host every derived
  view serializes the literal `unavailable` reason (greppable). The **on-site-water absence is
  unconditional by construction** — a structural test that no input makes it a `Value`.
- **Falsifiable acceptance:** (a) golden derivation tests green on hand-computed values; (b) on a
  RAPL-less/constant-less host every derived view is a greppable named absence; (c) on-site water is
  a `Value` under NO input (grep proves no producing path). **Falsifier for (c):** any code path
  emits an on-site-water litre value → FAIL (a fabricated-facility-metric standard violation).

## 69.5 Dependency gate (honest) + operator-decision

**After item 58** (work-normalized cost ledger). Item 58 has **no blueprint yet** (spec-level in the
roadmap). Item 69 is a *derived view* consumer; the roadmap sequences it after 58's ledger lands the
consumer-side delta/pairing conventions. **69 is blocked on item 58.** (The `joules_uj` *source* is
already live, so the derivation math can be prototyped/golden-tested against the existing field ahead
of 58, but the item ships after 58.)

**Operator-decision (flagged):** (i) the `(region, deployment-class)` constant *values* are an
operator/deployment input, not an engineering default — the module must ship with the constants absent
(named-absence) and only light up when supplied. (ii) whether the two new absence reasons extend the
closed `Absence` enum or live in a footprint-local reason enum (§69.3 step 4) — a small schema-scope
ruling.

---

## Cross-item dependency summary (this doc)

| Item | Gate | Status |
|---|---|---|
| 67 | after item 57 (`eff` column + procedure ratification) | BLOCKED on 57 (procedure doc exists; column + ratification pending) |
| 68 | after {67 + item 7} | item 7 LANDED; transitively BLOCKED on 57→67 |
| 69 | after item 58 (no blueprint yet) | BLOCKED on 58 (source `joules_uj` already live) |
