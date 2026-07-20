# BLUEPRINT — Item 62: FDR Relational Linkage (`span_id` + `parent_span_id: Reading<u64>`) + the wasm clock leg

- **Date:** 2026-07-19 · **Tier:** code (roadmap §K, item 62) · **Status:** BLUEPRINT (planning
  artifact, no code).
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §K item 62
  (lines 1064–1080); `RESEARCH-RESOURCE-FOOTPRINT-ZERO-BLINDSPOT-RELATIONAL-TELEMETRY-2026-07-19.md`
  thread 3 (the decisive flat-FDR finding) + G4 (wasm clock);
  `PROCEDURE-TELEMETRY-COMPLETENESS-STANDING-2026-07-19.md` steps 5 (linkage) + 9 (wasm);
  ground-truth code: `kernel/src/fdr/schema.rs`, `kernel/src/fdr/mod.rs`, `kernel/src/fdr/ring.rs`,
  `kernel/src/fdr/pmu.rs`.
- **Prerequisites:** the **exec-branch FDR merge** (extends the FDR envelope). **Parallel with item
  58** (both after the FDR merge; no ordering between them). Coordinates its wasm clause with **item
  60** (one `performance.now()` design, not two).

---

## 1. Scope & goal

**Goal.** The FDR schema is **flat/unlinked** — `seq` conveys temporal succession but never *causal
parentage*. A recovered ring cannot reconstruct a call tree. Add `span_id` + `parent_span_id:
Reading<u64>` on the P3 plane (extending, never replacing, the envelope), reduce OTel cross-process
propagation to passing one `u64`, and state the wasm clock leg (G4) so the FDR plan no longer
*silently* excludes the wasm surface.

**Non-goals.**
- NOT a replacement of `seq` (temporal ordering stays; parentage is *additional*).
- NOT OpenTelemetry / a tracing dependency (procedure step 8; the whole point of the FDR rewrite,
  items 4+29, was to drop `tracing`). Propagation = passing a bare `u64`.
- NOT a determinism-touching change (span ids are P3, never a hash/replay input).

## 2. Current-state grounding

- `kernel/src/fdr/schema.rs:210–230` — `FdrEvent { seq, ts_unix_ns, mono_ns, level, kind, name, hw,
  pmu: Option<PmuStamp>, fields }`. **A grep over `schema.rs` for parent/trace/span/caller = zero
  hits.** `seq` is "the recovery ordering key" (`:213`) — temporal succession, **never** causal
  parentage. The envelope is FLAT.
- `kernel/src/fdr/schema.rs:264–285` — `to_json` emits a fixed field order; `pmu` rides "ONLY on
  verdict-emission records; absent otherwise, so every other FDR record serializes byte-identically"
  (`:274–279`). **This is the exact optional-field precedent** item 62 extends: a new field absent on
  a surface ⇒ byte-identical to before.
- `kernel/src/fdr/schema.rs:61–65` — `Reading<T> = Value(T) | Unavailable(Absence)`, always
  serialized (value or named absence). `parent_span_id` uses this to say "this is a root" via a named
  absence — **no magic `0`, no missing key.** A new `Absence::NoParent` variant is required (the
  closed reason set, `:25–56`).
- `kernel/src/fdr/schema.rs:232–262` — `FdrEvent::stamp` is `#[cfg(not(target_arch = "wasm32"))]`
  because `SystemTime::now()`/`mono_now_ns()` **panic on wasm32** (`:236–237`). The FDR write path is
  "never reached on wasm (no sink is ever installed there)". **This is the G4 finding:** the FDR plan
  structurally EXCLUDES the wasm surface silently — item 62 must state the wasm-safe clock or the
  named absence for the wasm pub fns.
- `kernel/src/fdr/mod.rs:365–367` — `next_seq` is a process-global `AtomicU64::fetch_add`; a
  `span_id` counter is the identical cheap primitive (per-process monotone `u64`).
- `kernel/src/fdr/ring.rs:1–39` — A/B segment ring with CRC32 lines + kill-9 recovery; a recovered
  ring is a list of `FdrEvent`s. **Reconstructing a call tree** = walking `parent_span_id` links over
  the recovered list (the proof surface for step 4).

## 3. Implementation plan (numbered)

1. **Add `Absence::NoParent`** to the closed reason enum (`schema.rs:25–56`) + its `as_str()`
   (`"no_parent"`, greppable). This is the named-absence doctrine covering "this is a root."
2. **Extend the envelope (never replace):** add `span_id: u64` (per-process counter, the `next_seq`
   twin) + `parent_span_id: Reading<u64>` to `FdrEvent` (`schema.rs:210`). A root record carries
   `parent_span_id: Unavailable(NoParent)`. Serialize both in `to_json` with the optional-field
   discipline: **surfaces without linkage stay byte-identical** (a record that does not participate in
   a span tree may omit/absence the fields exactly as `pmu` is omitted when `None`).
3. **Per-process `span_id` minter:** a `static SPAN_SEQ: AtomicU64` (mirroring `sink::SINK.seq`,
   `mod.rs:332,365`), `fetch_add(1, Relaxed)`. The active span's id is threaded to child records
   (via the `SpanGuard`/emission site) as their `parent_span_id`.
4. **Cross-process edges seed the parent id across the boundary.** The two boundaries:
   (i) **subprocess spawns** (`living_knowledge.rs` — item 61 step 2's `wait4` record) and (ii) the
   **agent↔LLM boundary** (item 59). Seeding = passing the parent's `span_id: u64` across the boundary
   (env var / arg / header — one `u64`), so the child's root record carries the parent's id instead of
   `NoParent`. **OTel propagation reduced to passing one `u64`** (no context object, no dependency).
5. **The wasm clock leg (G4):** state the wasm-safe clock for the 24 wasm pub fns —
   `performance.now()` via a single imported binding **(the SAME binding item 60 imports for the
   engine — one design, not two)** OR a named `Absence` for surfaces that genuinely cannot time on
   wasm. `FdrEvent::stamp` may no longer be *silently* `cfg`'d off wasm without the plan stating which
   applies: today no sink is installed on wasm (so no record is written), which is a *legitimate*
   named absence — but item 62 makes that statement explicit rather than an accident of `cfg`.
6. **Cost, honest:** ~16 bytes (`u64` + `Reading<u64>`) + one counter increment per record, on the
   **P3 plane** — never touches determinism, never a hash/gate input.

## 4. Required tests / proofs (CHECKLIST.md 5-point standard)

| Checklist item | Disposition for item 62 |
|---|---|
| 1. **Oracle** | **Nested spans reconstruct a correct call tree from a recovered ring** — a test that walks `parent_span_id` links over a recovered `fdr/ring.rs` segment and asserts the tree shape (the recovery+readback path is the oracle). **Root records carry the literal `"no_parent"` reason** (greppable). **Records on surfaces without linkage stay byte-identical** (optional-field regression, the `pmu`-absence precedent, `schema.rs:325–343`). |
| 2. **Dudect** | **N/A** — span ids are monotone counters, not secret-dependent; no CT branch. |
| 3. **Debug cross-check** | **N/A** — the ids are counters, not arithmetic with a per-call reference. |
| 4. **ASM spot-check** | **N/A** — no branch-free hot path. |
| 5. **Kani/formal** | **N/A** — the property is "the tree reconstructs; roots are named; P3 firewall holds," oracle-class. |

**P3 firewall grep proof (procedure step 7, the highest-class invariant here):** grep confirms **no
`span_id`/`parent_span_id` feeds any hash, signature, idempotency, replay, or gate-verdict surface** —
the linkage is pure forensics (the item-27 §4.5 precedent, made mandatory again because a causal-id
leaking into `event_id()`/replay would be a determinism break).

**wasm-cdylib-green proof (procedure step 9):** the wasm build compiles with the stated clock leg (the
`performance.now()` import or the named absence); no `Instant::now()`/`SystemTime::now()` on the wasm
path (the existing `stamp` `cfg` guard stays correct, now *stated* not accidental).

## 5. Falsifiable acceptance criteria

1. `FdrEvent` carries `span_id: u64` + `parent_span_id: Reading<u64>`; nested spans in a recovered
   ring **reconstruct a correct call tree** (a test walks the parent links).
2. Root records carry the literal `"no_parent"` reason string — greppable, never a magic `0` or a
   missing key.
3. Records on surfaces without linkage are **byte-identical** to pre-item-62 (optional-field proof).
4. The P3 grep proof is green: **no span id feeds any hash/gate/replay surface.**
5. A cross-process edge (subprocess or agent↔LLM) seeds the parent id — a child's root record carries
   the parent's `span_id`, not `NoParent`.
6. The wasm cdylib stays green with the stated clock leg (import or named absence).

**Falsifier:** a span id in `event_id()`/any replay input; a magic `0` for a root; a broken existing
FDR record (non-byte-identical without linkage); an `Instant::now()` on the wasm path.

## 6. Dependency gates

- **Upstream:** the **exec-branch FDR merge** (item 62 extends the FDR envelope; `fdr/` present in
  this worktree's base, so the gate is the merge). **Parallel with item 58** — no ordering between
  them (58 adds the `work` field; 62 adds the linkage fields; both are additive on the same envelope,
  land in either order, but coordinate the `to_json` field-order edit to avoid a merge conflict).
- **Coordination:** **item 60** (wasm clock) — steps 5 here and item 60 step 4 are **one
  `performance.now()` design**; land the shared binding once. **Item 61** (subprocess `wait4` record)
  and **item 59** (agent↔LLM) are the two cross-process edges item 62 seeds — item 62 provides the
  `span_id` those records carry; land 62's minter so 59/61 can seed across their boundaries.
- **Downstream:** items 59/61 consume the cross-process seeding (a `u64` passed across their
  boundaries). Item 62 does not gate them (they can record `NoParent` roots until 62's seeding lands),
  but the full causal tree needs 62's ids.

## 7. Operator-decision points & accepted risks

- **[NOTE, not operator] Field-order coordination with item 58.** Both 58 (`work`) and 62
  (`span_id`/`parent_span_id`) edit `FdrEvent`'s field list and `to_json` field order. Because both
  are additive-optional, the merge is mechanical, but the executor should land them against a shared
  field-order convention (append new optional fields after `pmu`, before `fields`) to keep the two
  parallel branches conflict-free. Not an operator decision; recorded for the executor.
- **[ACCEPTED] ~16 bytes + a counter per record.** The honest cost is stated (procedure §0 triangle
  awareness); it is P3, so it never taxes determinism. On high-frequency `Event`-kind records the
  linkage may be `NoParent`/absent (they are not span-tree participants), keeping the cheap path
  cheap. **Owner:** arc lead.
- **[ACCEPTED] wasm named-absence is legitimate.** Today the FDR sink is never installed on wasm, so
  "no record on wasm" is a truthful named absence, not a gap. Item 62 makes that *statement* explicit
  (per procedure step 9) rather than an accident of `cfg`; it does not force wasm FDR recording where
  no sink exists. **Owner:** arc lead.
