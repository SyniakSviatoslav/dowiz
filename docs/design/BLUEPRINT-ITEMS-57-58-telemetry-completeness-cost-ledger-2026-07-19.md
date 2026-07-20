# BLUEPRINT — Items 57 + 58: Telemetry-Completeness Standing Procedure (ratified) · Work-Normalized Cost Ledger

- **Date:** 2026-07-19 · **Tier:** enforcement-spine + schema (roadmap §K, items 57 & 58) ·
  **Status:** BLUEPRINT (planning artifact, no code).
- **Grouping rationale:** these two are the *foundation* of the telemetry arc — item 57 is the
  binding *law* (the procedure + the HOT-PATHS `eff` enforcement) and item 58 is the *schema* that
  law governs (the work/cost pair record). They share a proof surface and a dependency edge (58
  depends on 57), and every downstream consumer (items 59/60/61) depends on **both**. Grouping them
  keeps the law and its first data structure in one place; the three consumers stay in their own docs.
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §K items 57
  (lines 990–1006) + 58 (lines 1007–1022);
  `PROCEDURE-TELEMETRY-COMPLETENESS-STANDING-2026-07-19.md` (the 13-step procedure item 57
  ratifies — read in full); `AUDIT-TELEMETRY-EVERYWHERE-AI-OPTIONAL-OS-2026-07-19.md` §1.3 (the
  work-normalized cost ledger + `eff`-column mechanism); ground-truth code: `kernel/src/fdr/
  schema.rs`, `kernel/src/fdr/pmu.rs`, `kernel/src/fdr/mod.rs`, `docs/audits/hardening/
  {CHECKLIST.md,HOT-PATHS.tsv}`, `scripts/hardening-gate.sh` (referenced by CHECKLIST).
- **Prerequisites:** item 57 has **none** (the procedure doc already exists; ratification is an
  operator act + the mechanical `eff` extension). Item 58 depends on **item 57** (the procedure must
  be binding first) **+ the exec-branch FDR merge** (it emits on `SpanClose`-class FDR records).

---

# PART A — ITEM 57: Telemetry-Completeness Standing Procedure ratified + HOT-PATHS accounting columns

## A.1 Scope & goal

**Goal.** Make `PROCEDURE-TELEMETRY-COMPLETENESS-STANDING-2026-07-19.md` **binding** on every future
blueprint in this arc (the item-25 pattern), and add the *mechanical* enforcement: an `eff` column on
`HOT-PATHS.tsv` so **every hot-zone function is accounted for** — either it names its
workload-kind/span, or it carries a ledgered `gap:` reason. This is the honest form of the operator's
"enforced everywhere": **zero UN-NAMED blind spots** — 100% coverage of the *accounting*, not the
impossible 100% of universal timing.

**The impossibility triangle (state it, never violate it — procedure §0):** you cannot simultaneously
have (1) a stamp on 100% of call sites, (2) zero overhead on paths measured to the microsecond (item
26: FDR encode ~3.9 µs, event-log append ~637 µs), and (3) byte-deterministic replay — and on
`wasm32`, `Instant::now()` *panics*. The achievable 100% is 100% of the *accounting*: every function
classified, every absence named, mechanically checked.

**Non-goals.**
- NOT a new checklist (extends item-6's `hardening-gate` machinery, never a parallel gate).
- NOT universal runtime timing (forbidden by the triangle).
- NOT any new dependency (procedure step 8: `macro_rules!` / `/proc`-`/sys` std reads only).

## A.2 Current-state grounding

- `PROCEDURE-TELEMETRY-COMPLETENESS-STANDING-2026-07-19.md` — **already written**, 13 steps
  (1–10 = footprint/blind-spot/linkage/wasm; 11–13 = cost-oracle), with 3 terminal states
  ((a) COMPLETE / (b) COMPLETE-WITH-LEDGERED-GAPS / (c) NOT-COMPLETE) mirroring item 25. It is
  "drafted standard of record" until item 57 ratifies it (line 6–8). Ratification = the missing
  piece.
- `docs/audits/hardening/HOT-PATHS.tsv` — 9 `@ZONE` lines + 18 data rows; columns are
  `path_prefix | features | filter | min_tests | mode | checklist | gap` (header lines 8–26). There
  is **no `eff` column today** — the accounting the procedure requires is not yet mechanized.
- `docs/audits/hardening/CHECKLIST.md` — the standing law; §"Honest gaps" already ledgers missing
  coverage in the manifest's `gap` column (the exact mechanism item 57 extends). The
  "re-execute, never presence-check" §10/P7 correction is the anti-forgery core item 57 reuses.
- Audit G9 (recorded, ruled by item 57): the cheap-path FDR envelope is *one relaxed atomic load
  when disabled* (`kernel/src/fdr/mod.rs:117–124`, `SINK_ACTIVE.load(Relaxed)` in `event_enabled`) —
  the always-compiled floor; heavy stamps stay feature-gated (`StampPolicy::Full` reads `/proc`+`/sys`
  only when a sink is installed).

## A.3 Implementation plan (numbered)

1. **Ratify the procedure.** Cross-link `PROCEDURE-TELEMETRY-COMPLETENESS-STANDING-2026-07-19.md`
   from `docs/audits/hardening/CHECKLIST.md` (a "Standing procedures" reference line) so the gate's
   law-of-record points at it. Record the ratification in the roadmap-arc tracking (NOT by editing
   the roadmap file — a tracking note / the arc's own ledger).
2. **Add the `eff` column to `HOT-PATHS.tsv`.** Extend the tab-separated schema with one column,
   `eff`, whose value is either a workload-kind/span name (from item 58's closed enum) or a
   `gap:<reason>` token. Update the header comment block (lines 8–26) to document the column.
3. **Classify every hot-zone function.** Each function under an `@ZONE` prefix is tagged in its row's
   `eff` cell as one of `INSTRUMENTED(<workload-kind|span>)` | `CHEAP(SamplingDisabled)` |
   `EXCLUDED(<reason>)`. A zone whose functions are not all accounted for gets a `gap:` row.
4. **Extend the gate (`scripts/hardening-gate.sh`), never fork it.** Add a check: a hot-zone row
   carrying neither an `eff` value nor a `gap:` reason ⇒ RED. Reuse the anti-forgery clause (a
   planted blank `eff` cell must go RED, mirroring item 6's planted-no-manifest-row demo).
5. **Record the G9 standing posture.** Write the ruling into the procedure doc's §3 (already present)
   AND into `kernel/src/fdr/mod.rs`'s module doc **when next touched** (not a gratuitous edit): the
   cheap-path envelope (one relaxed atomic load) is the always-compiled floor; heavy stamps stay
   `telemetry`-gated. The default binary is never fully dark, never taxed.

**Note (scope discipline):** items 57 does NOT itself fill every `eff` cell with a real measurement —
it makes the *accounting* mandatory. Filling `INSTRUMENTED` cells with real spans is items 58–61's
job; item 57 guarantees an unfilled cell is *visible and RED*, not silent.

## A.4 Required tests / proofs (CHECKLIST.md 5-point standard)

| Checklist item | Disposition for item 57 |
|---|---|
| 1. **Oracle** | The gate itself is the oracle: it re-executes (parses live), never presence-checks a report. Procedure doc cross-linked from CHECKLIST.md; the extended gate re-derives coverage from the TSV each run. |
| 2. **Dudect** | **N/A** — no secret-timed code; item 57 is a CI-script + doc change. |
| 3. **Debug cross-check** | **N/A** — no per-call arithmetic reference. |
| 4. **ASM spot-check** | **N/A** — no branch-free path. |
| 5. **Kani/formal** | **N/A** — the property is a gate-coverage property, not a bounded-model one. |

**Anti-forgery (the load-bearing proof, item-6 §2.4 precedent):** a **planted hot-zone row carrying
neither `eff` nor `gap:` ⇒ gate exit 1 (RED)**; adding the `eff`/`gap:` value ⇒ exit 0. Demonstrated
and recorded in the PR, exactly as item 6 demonstrated the no-manifest-row RED path. This is what
makes "zero un-named blind spots" a re-executed property rather than a slogan.

## A.5 Falsifiable acceptance criteria

1. `CHECKLIST.md` links the procedure doc; the procedure is recorded BINDING for the arc.
2. `HOT-PATHS.tsv` has an `eff` column; every existing hot-zone row carries an `eff` value or a
   `gap:` reason (zero blank).
3. The extended `hardening-gate` goes RED on a planted blank-`eff` hot-zone row and GREEN when filled
   (recorded RED→GREEN in the PR).
4. G9 ruling recorded in the procedure doc (+ `fdr/mod.rs` doc when next touched).
5. `cargo tree -e no-dev` byte-unchanged (no dependency added).

---

# PART B — ITEM 58: Work-Normalized Cost Ledger

## B.1 Scope & goal

**Goal.** On `SpanClose`-class FDR records for a *named workload*, emit the pair
`(work: {kind, Δcount}, cost: HwStamp-delta ⊕ PmuStamp-delta)` — **pairs of raw `u64`, never
ratios**. Efficiency (per-joule / per-cycle / per-tick) is a *consumer* concern; the record stays
lossless. This is procedure steps 1–2 made concrete, and the schema every downstream consumer
(items 59/60/61) writes into.

**Non-goals.**
- NOT a ratio field in the schema (procedure step 2: a ratio forecloses every later question the raw
  pair could answer — a structural violation).
- NOT a cross-tier efficiency number (E vs C vs T) — structurally uncomputable (absent counters are
  absent), enforced by construction.
- NOT an open-ended workload enum — a *closed* enum seeded from work units that already exist.

## B.2 Current-state grounding

- `kernel/src/fdr/schema.rs:90–102` — `HwStamp { cpu_ticks, rss_kb, joules_uj }`, each a
  `Reading<u64>` (`:61–65`), always serialized (value or named absence). This is the `cost` side's
  Tier-T/E carrier.
- `kernel/src/fdr/pmu.rs:37–69` — `PmuStamp` (Tier-A rdtsc/faults/ctxt + Tier-B PMU
  instructions/cycles/misses), all `Reading<u64>`; `PmuStamp::delta(start, end)` (`:98–118`) is the
  **one sanctioned bracketed subtraction**, absence-propagating, still emitted as raw counts. This is
  the `cost` side's Tier-C carrier. `PmuStation::bracket` (`:416–421`) is the ready-made
  before/after window primitive.
- `kernel/src/fdr/schema.rs:184–208` — `Kind { Event, SpanClose, Alarm, PostMortem, Tuning,
  CleanShutdown }`; `SpanClose` is the record class item 58 targets.
- `kernel/src/fdr/schema.rs:210–230` — `FdrEvent` has `hw` (non-optional) + `pmu: Option<PmuStamp>`
  (optional, `:223–228`) + `fields: Vec<(&'static str, String)>`. The `pmu`-optional discipline
  ("absent ⇒ byte-identical to pre-item-27") is the exact precedent for adding an optional `work`
  field.
- **No `work` / `WorkloadKind` type exists today** — grep-confirmed the schema carries cost
  (`hw`/`pmu`) but never the *work denominator*. Item 58 adds the numerator's *kind + Δcount*.

## B.3 Implementation plan (numbered)

1. **Closed `WorkloadKind` enum** (in `fdr/schema.rs`, `Copy`, seeded from existing work units):
   `DecisionUnitsImported`, `FdrRecordsAppended`, `TransitionsFolded`, `TokensGenerated`,
   `FramesRendered`, `EigensolvesCompleted`, `SignaturesVerified`. Closed-enum growth only (the
   item-48 `Heartbeat` precedent for disciplined closed-enum extension). Each has a stable
   `as_str()` (greppable serialized name, mirroring `Kind::as_str` / `Absence::as_str`).
2. **`Work` struct:** `{ kind: WorkloadKind, delta_count: u64 }` — the numerator. Raw `u64`, no rate.
3. **Optional `work` field on `FdrEvent`** (`work: Option<Work>`), following the `pmu: Option<…>`
   discipline: present ONLY on `SpanClose`-class records for a named workload; absent ⇒ record
   byte-identical to pre-item-58 (all other FDR records unchanged). Serialized as a nested
   `"work":{"kind":"…","delta_count":N}` object, always-present-when-Some.
4. **The pair is (work, cost).** The `cost` side is ALREADY the record's `hw`-delta ⊕ `pmu`-delta —
   consumers subtract span-open from span-close `HwStamp`/`PmuStamp` (the `PmuStamp::delta` primitive
   already exists; an `HwStamp::delta` sibling may be added the same absence-propagating way if a
   consumer needs the pre-computed hw delta). **No ratio field is ever added** — assert this
   structurally (step B.4).
5. **Self-describing degradation ladder (per field, via `Reading<T>` — already the carrier):**
   - **Tier E** (per-joule) lights up automatically on RAPL hosts (`joules_uj` is a `Value`).
   - **Tier C** (per-cycle/instruction) lights up on PMU hosts (`hw_cpu_cycles`/`hw_instructions`
     are `Value`s — this agent host reads them under CAP_PERFMON; a paranoid host degrades to
     `PermissionDenied`).
   - **Tier T** (per-tick/wall) is the floor this dev host actually runs at (`cpu_ticks`/`mono_ns`
     always available on Linux). Honest, not aspirational.
   A cross-tier comparison is **structurally uncomputable** — an absent counter is `Unavailable`, so
   `work/cycles` on a host without cycles simply does not exist. On hosts where **C and T are both
   live**, `work/cycles` vs `work/ticks` must agree within a stated band — a *free self-test of the
   counters* (a gross disagreement means a counter is lying).

## B.4 Required tests / proofs (CHECKLIST.md 5-point standard)

| Checklist item | Disposition for item 58 |
|---|---|
| 1. **Oracle** | Schema round-trip tests (the `FdrEvent::to_json` deterministic-serialization oracle, `schema.rs:317–343` precedent) — a `SpanClose` record with a `Work` serializes the pair; **named-absence serialization proof**: the literal `unavailable` reason for the RAPL-less/paranoid tier is greppable in the emitted record (procedure step 10 red→green, the `rapl_absent_host_reports_named_absence_not_missing_key` precedent, `schema.rs:292–302`). |
| 2. **Dudect** | **N/A** — telemetry values are P3-plane, not secret-dependent. |
| 3. **Debug cross-check** | **N/A(schema)** — no per-call arithmetic reference; the pair is recorded raw. |
| 4. **ASM spot-check** | **N/A** — no branch-free hot path. |
| 5. **Kani/formal** | **N/A** — the property is "the record is a lossless pair," a schema/oracle concern. |

**Structural pair-not-ratio proof (load-bearing, procedure step 2):** assert that **no ratio field
exists in the schema** — a compile-time / test-level assertion that `Work` and `FdrEvent` carry only
raw `u64` counts and named absences, no `f64` efficiency field. The strongest form: there is no
`per_*`/`ratio`/`efficiency` field name in the schema; a reviewer grep confirms it.

**Cross-tier consistency band test:** where both C and T are live (this host), assert `work/cycles`
and `work/ticks` agree within the stated band — green here, structurally absent where a tier is
unavailable.

## B.5 Falsifiable acceptance criteria

1. A `WorkloadKind`-tagged `SpanClose` record serializes `(work, cost)` as a raw-`u64` pair;
   tokens/sec (or eigensolves/joule, etc.) is derivable **consumer-side** from one record's raw pair.
2. The named-absence serialization proof is green on this RAPL-less/paranoid host: the emitted record
   carries the literal `"unavailable":"no_rapl_interface"` (Tier-E) — greppable, never a missing key.
3. Structural pair-not-ratio proof green: no ratio field exists in the schema.
4. Cross-tier band test green where C and T are both live.
5. All FDR records **without** a `Work` are byte-identical to pre-item-58 (optional-field discipline).
6. `WorkloadKind` is a closed enum (a new work unit requires a conscious variant + `as_str`).

**Falsifier:** any ratio/`f64`-efficiency field in the schema; a fabricated `0` where a counter is
absent; a non-`SpanClose` record carrying a `Work`; a cross-tier ratio computed as if comparable.

## B.6 Dependency gates (both items)

- **Item 57:** no prerequisites (procedure exists) — ratification is operator + mechanical `eff`
  extension. **READY once dispatched.**
- **Item 58:** depends on **item 57** (the procedure must be binding so 58's pairs/absence rules are
  enforced, not merely conventional) **AND the exec-branch FDR merge** (it emits on `SpanClose` FDR
  records; the `fdr/` module is present in this worktree's base, so the gate is the merge).
- **Downstream:** items **59, 60, 61 all depend on item 58** — they are its first consumers, each
  emitting `(work, cost)` pairs under one `WorkloadKind` (59 → `TokensGenerated`; 60 →
  `FramesRendered`; 61 → `FdrRecordsAppended`/`EigensolvesCompleted`/`SignaturesVerified`). Item 62
  (relational linkage) is **parallel** with item 58 (both after the FDR merge, no ordering between
  them).

## B.7 Operator-decision points & accepted risks

- **[OPERATOR] Ratifying the procedure as BINDING (item 57).** The procedure makes 13 steps mandatory
  for every future arc blueprint — real friction on future work. Recommended (it is the honest form
  of "enforced everywhere"), but ratification is an explicit operator act by the procedure's own
  terms (line 6–8). **Owner:** operator.
- **[ACCEPTED] Tier-T is the honest floor here.** This dev host is RAPL-less and `perf_event_paranoid
  = 4`, so Tier-E is a named absence and Tier-B PMU is `PermissionDenied` unless the process holds
  CAP_PERFMON. The ledger degrades truthfully to Tier-T (wall/ticks). This is *correct*, not a gap —
  the schema lights up higher tiers automatically on capable hosts with zero schema change. **Owner:**
  arc lead.
- **[ACCEPTED] Closed-enum churn.** A new workload-kind requires a conscious enum edit + `as_str` +
  a HOT-PATHS `eff` cell. This friction is the point (it forces accounting). **Owner:** arc lead.
- **[NOTE] `HwStamp::delta` is optional.** Whether to precompute an hw-delta in-kernel or leave the
  subtraction to the consumer is an implementation call; `PmuStamp::delta` already exists, so parity
  is cheap if a consumer wants it. Not an operator decision.
