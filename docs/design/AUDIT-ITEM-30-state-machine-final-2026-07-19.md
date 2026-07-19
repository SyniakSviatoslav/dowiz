# AUDIT — Roadmap Item 30: State-Machine Proliferation (Tier 0, FINAL)

> Executes `BLUEPRINT-ITEM-30-state-machine-audit-2026-07-19.md` against the **live tree**.
> Roadmap: `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §A / §G.2. Baseline commit for
> the shared FSM machinery: `94c29146b` (order_machine hardening, item 3). Every cell below was
> re-verified by grep/read against the working tree on 2026-07-19 — line numbers are NOT inherited
> from the blueprint.
>
> **Two trees, one source.** The four kernel modules are byte-identical between `main`
> (HEAD `df9d5ec86`) and the exec branch base — line numbers cited here are the **main-checkout**
> numbers. The one code change this audit produced (the `resume()` fix, see §4) lives on branch
> `exec/space-grade-tier0-2026-07-19` (commit `707848dfd`), not yet merged to main; main still
> carries the buggy lines cited in §4.

## 1. Machinery-sharing check — the core question

`grep -nE 'order_machine|has_cycle|topological_order|GOLDEN_SIGNATURE|FSM_GOLDEN|allowed_next|LIFECYCLE_STATES'`
across all four candidate files (`capability_cert.rs`, `hub_provisioning.rs`, `hub_supervisor.rs`,
`hydra.rs`): **ZERO hits.** None of the four routes through the shared FSM proof kit
(`order_machine.rs`'s const-adjacency / Kahn `topological_order` / `has_cycle` / `FSM_GOLDEN_SIGNATURE`).

**Verdict on §22's worst fear ("five copies of the same machinery"): does NOT materialize.** These
are four structurally *different* state shapes, not four reinventions of the order FSM. No collapse
into the shared kit is forced by any of the four; the shape mismatch is real, not accidental. See
the forcing reasons in each ticket (§3).

## 2. The 5-column proof table

Row 0 is the shared baseline (`order_machine.rs`, the one module that IS the shared machinery).
Rows 1–4 are the audit targets.

| # | module | state-def (file:line) | transition-fn (file:line — every state-assigning/returning site) | shared / independent | ticket |
|---|--------|-----------------------|------------------------------------------------------------------|----------------------|--------|
| 0 | `kernel/src/order_machine.rs` | `OrderStatus` enum `:8`; `LIFECYCLE_STATES` const `:199`; `idx_of` `:217` | `allowed_next` `:78` (const adjacency); proof kit: `topological_order` `:243` (Kahn), `has_cycle` `:542`, `FSM_SPECTRAL_RADIUS` `:334`, `FSM_GOLDEN_SIGNATURE` `:465` | **SHARED** (it is the shared machinery) | — (baseline / reference; no ticket) |
| 1 | `kernel/src/capability_cert.rs` | `RotationState` enum `:542-551` (2 data-carrying variants: `Stable{suite}`, `Overlapping{old,new,overlap_until}`) | `accepts()` `:556-571` — clock-windowed acceptance **predicate**. **No production transition fn exists**: `RotationState` is constructed only in tests (`:1482`, `:1489`); rotate/retire are external + doc-only (`:538-540`). | **INDEPENDENT** | **I30-T1** (PARITY-PIN) |
| 2 | `kernel/src/hub_provisioning.rs` | `PoolSlotState` enum `:152-162` (4 data-carrying: `Provisioning`, `Warm`, `Claimed{owner}`, `Suspended{owner,state_snapshot}`; §16.57 no-reclaim invariant lives only in a comment `:157`) | **Scattered inline assignments, no single transition fn:** `refill()` `:633` → `Warm` `:658`; `claim()` `:757` (guard `:775`) → `Claimed` `:811`; `suspend()` `:826` → `Suspended` `:838`; `resume()` `:846` → `Claimed` `:858` | **INDEPENDENT** | **I30-T2** (DEFECT-FIXED + PARITY-PIN; COLLAPSE-candidate noted, deferred) |
| 3 | `kernel/src/hub_supervisor.rs` | `Slot` enum `:433-436`; `UpdateState` enum `:442-476` (7 states, data-carrying; promote-without-snapshot/health made UNREPRESENTABLE per §5.1, doc `:438-440`); `RollbackTrigger` `:478-483` | Pure decision fns: `decide_promote()` `:536` (returns `PromoteStep`, matches `UpdateState` `:546-554`), `decide_rollback()` `:568` (returns `RollbackStep`). Drivers effecting the transition: `drive_promote()` `:616-682`, `drive_restore()` `:689+`. No fn assigns a raw `UpdateState` in production — state is threaded as a param + `StateStore`. | **INDEPENDENT** | **I30-T3** (PARITY-PIN) |
| 4 | `kernel/src/hydra.rs` | `OrganismState` enum `:76-79` (2 states: `Live`, `Locked`); `HysteresisBand` `:85-92`; `INTEGRITY_BAND` const `:94-98`; compile-time band asserts `:103-113` | `integrity_check()` `:219-242` — ρ-driven hysteresis flip; assigns `Locked` `:225`, `Live` `:231` | **INDEPENDENT** | **I30-T4** (PARITY-PIN) |

## 3. Tickets (RC-4 format: finding-id · independent construct · file:line · resolution)

- **I30-T1 · `capability_cert.rs` `RotationState` 2-state clock-windowed acceptance predicate ·
  `:542-571` · PARITY-PIN.**
  *Forcing reason (why COLLAPSE doesn't fit):* the states carry data (alg suites + a clock window)
  and the only "transition" is time-driven acceptance, not a graph edge — there is **no production
  transition function at all** (rotate/retire happen externally; the module holds only the
  `accepts()` predicate). Const-adjacency / cycle / topological-order machinery is *structurally
  inapplicable* to a 2-variant clock predicate. *Pin:* a test asserting `accepts()` exhaustiveness
  over both variants **and** the overlap-window boundary (`now == overlap_until` accepts both old &
  new; `now > overlap_until` accepts only new — the `:564-568` branch).

- **I30-T2 · `hub_provisioning.rs` `PoolSlotState` 4-state pool machine with transitions scattered
  across four call sites · `:152-162`, `:658/:811/:838/:858` · DEFECT-FIXED + PARITY-PIN;
  COLLAPSE-candidate noted (deferred beyond Tier 0).**
  This is the **highest-risk shape** of the four: no single transition fn, `matches!` guards inline,
  and the §16.57 no-reclaim invariant enforced only by convention.
  - *Defect (confirmed, fixed):* see §4 — `resume()` silently zeroed the owner. Fixed on
    `exec/space-grade-tier0-2026-07-19` (`707848dfd`) with red→green regression guard.
  - *Pin (recommended next):* a test binding the two invariants the convention leaves implicit —
    (a) **no-reclaim**: no transition path from `Claimed`/`Suspended` back to `Warm`; (b)
    **owner continuity**: the owner id is invariant across `claim → suspend → resume`.
  - *COLLAPSE candidate (deferred):* the four scattered assignments are the one place a single
    `fn transition(&mut Slot, Event) -> Result<(), ProvisionError>` would materially reduce risk —
    but that is a restructuring, out of this read-only Tier-0 pass's scope. *Forcing reason it is
    NOT a collapse into the shared FSM kit:* `PoolSlotState` variants carry data (`owner`, snapshot)
    and the machine is not the order lifecycle — the shared const-adjacency/golden-signature kit
    models a data-free 12-state lifecycle and does not apply.

- **I30-T3 · `hub_supervisor.rs` `UpdateState` 7-state linear promote pipeline via pure decide fns +
  type-level unrepresentability · `:442-476`, `:536`, `:568`, `:616` · PARITY-PIN.**
  *Forcing reason:* this is a **linear DAG** enforced by type-level unrepresentability
  (`Promoted` has no producer except through `SnapshotTaken`→`HealthPassed`, §5.1) plus **pure total
  functions** — there are no cycles to detect and no adjacency to golden-sign, so the FSM cycle/topo
  kit is overkill, not a fit. *Pin:* a test asserting `decide_promote` is total and monotone along
  `Idle→Fetched→Migrated→SnapshotTaken→HealthPassed→Promoted` (each non-terminal state yields exactly
  the next step; terminal `Promoted`/`RolledBack`/`Failed` yield `Refuse`) — i.e. the linear order is
  exhaustive and acyclic.

- **I30-T4 · `hydra.rs` `OrganismState` 2-state Live/Locked ρ-hysteresis toggle · `:76-79`,
  `:219-242` · PARITY-PIN.**
  *Forcing reason:* a 2-state hysteresis toggle whose release is path-dependent (a healthy streak),
  driven by `crate::spectral::spectral_radius` against `INTEGRITY_BAND`. It reuses the **spectral**
  machinery (not the FSM kit) and **already carries its own proof machinery** — the compile-time
  const asserts `:103-113` (band ordering, gap ≥ 2·DRIFT_BAND, `healthy_checks ≥ 2`) are the in-situ
  parity pin. *Pin (add runtime coverage):* trip in one check at ρ ≥ `trigger`; release only after
  `healthy_checks` consecutive ρ ≤ `release`; no flip in the dead-band `(release, trigger)`.

## 4. The one confirmed defect — `resume()` owner-zeroing (finding I30-D1)

**True defect count from this audit: exactly 1 (confirmed by a red test), not 2.** The phrase
"state-machine proliferation with 2 confirmed silent defects" that circulated in-session was searched
across `docs/design/**`, `git log --all --grep` ("silent defect", "proliferation"), and the memory
corpus by the blueprint and re-checked here — **it has no written source**. It is not repeated as
fact. The audit establishes one real defect, row-by-row.

**The defect (main `hub_provisioning.rs`, still present on `df9d5ec86`):**
`resume()` `:846-866` overwrote `slot.state = Claimed { owner: OwnerId([0u8;32]) }` (`:858-860`)
and *then* attempted to restore the real owner via
`if let PoolSlotState::Suspended { owner, .. } = &self.slots[&hub].state` (`:862-864`) — but by
that point the state had **already** been overwritten to `Claimed`, so the `if let` can never match.
Dead restore code; a resumed hub's owner was left as all-zeros — a silent capability loss (owned by
nobody / a forgeable null id, violating §16.57 ownership continuity). The existing resume test
`red_suspend_preserves_state_then_resume` `:1594-1626` asserts only
`matches!(…, Claimed { .. })` `:1625` — it never checked the owner, so the defect passed undetected.

**Proof (test-first, per repo rule "test failures = code is wrong"):** a new test
`red_resume_preserves_owner_not_zeroed` suspends a slot with a known non-zero owner, resumes, and
asserts owner preservation. RED before the fix (`left: OwnerId([0,…]) != right: OwnerId([92,139,…])`),
GREEN after. **Fix:** capture the owner in `resume()`'s initial `match` alongside the snapshot and use
it directly in the `Claimed` transition; delete the dead restore block. Fix + guard committed to
`exec/space-grade-tier0-2026-07-19` as `707848dfd` (pushed). Full kernel suite after fix:
**899 passed / 0 failed / 3 ignored**; `order_machine` golden signature
(`green_live_signature_matches_golden`) stays green.

## 5. Summary

- 4 candidate modules, **0 SHARED with the FSM proof kit**, 4 distinct shapes — §22's "five copies"
  fear does not materialize; **0 collapses forced** in this Tier-0 pass.
- Tickets: **4 PARITY-PINs** (I30-T1..T4), each with a stated forcing reason; the only
  COLLAPSE-worthy structure (hub_provisioning's scattered transitions → one transition fn) is noted
  and deferred as a restructuring beyond read-only Tier 0.
- **1 confirmed silent defect** (I30-D1, resume owner-zeroing), fixed with a red→green regression
  guard on the exec branch. The unsourced "2 confirmed silent defects" claim is not substantiated.
