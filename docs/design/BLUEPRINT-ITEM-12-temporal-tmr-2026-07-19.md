# BLUEPRINT — Item 12: Temporal TMR pilot (SIHFT, re-scoped) — design-only under the non-ECC premise

- **Date:** 2026-07-19 · **Tier:** 4 (roadmap §E) · **Status:** BLUEPRINT (planning artifact, no code)
  — **design-only ruling** (roadmap §0 gate, §E item 12): scoping/design can start now; the pilot
  itself needs item 9 (breaker) + Tier-1 FDR to exist first.
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §E item 12
  (lines 381–397, the TEMPORAL-TMR re-scope), §0 gate (lines 22, premise retro-correction);
  `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §6 (SIHFT, lines 137–143, retro-corrected),
  §9 item 12 (line 176); `RESEARCH-OS-ARCHITECTURE-PATTERNS-ADOPTION-2026-07-19.md` §3 (temporal-TMR);
  live source `kernel/src/event_log.rs` (event-id hash), `kernel/src/money.rs` (money gate),
  `kernel/src/order_machine.rs` (FSM transition).
- **Relationship to items 9/54:** the pilot's vote-mismatch trips item 9's breaker; it is **genuinely
  additive over item 54's Sentinel** (Sentinel guards struct bytes at-rest via CRC; temporal TMR
  guards the *evaluation itself* against compute-time transient flips — complementary halves).

---

## 1. Scope / goal (one paragraph)

Design a **temporal triple-modular-redundancy (temporal TMR)** pilot: 2–3× sequential re-execution
of a small number of the kernel's most critical **µs-scale pure functions** on one core over the same
inputs, followed by a **trivial-equality vote**; a vote-mismatch routes to an item-9 breaker trip + a
Tier-1 FDR `Alarm`, **never** an SEU-immunity claim. The re-scope (roadmap §E, merging the consistency
audit §§1.1–1.2 premise-correction with the OS-patterns §3 research) is deliberate: **spatial** TMR
(three replicas on separate silicon) is unavailable to a single-process kernel and shared-silicon-
correlated anyway, so the pilot is *temporal* — re-run in time, not in space. It is **honestly PARTIAL**
(permanent faults and software bugs corrupt all runs identically; the voter is kept a trivial equality
to minimize its own fault exposure). The premise is the **non-ECC, local, offline-first, consumer-grade
hardware** the kernel actually targets (roadmap §0 retro-correction — the same reversal applied to
item 54/Sentinel), on which the compute-time transient-flip fault class is *material, not modest*.
Design-only now: the pilot needs the breaker and FDR to trip into; scoping/sizing starts immediately.

---

## 2. Verified current state — grounded

- **The candidate functions are pure, µs-scale, no-I/O — exactly the shape temporal TMR needs.**
  The roadmap names three (§E item 12): the **money gate**, the **event-id hash**, and **FSM
  transition candidates**. Grounded:
  - **Event-id hash:** `event_log.rs` `event_id()` (used at `:317`, `:337`) → `sha3_256`
    (`event_log.rs:30`) — a pure hash over the event's fields, on the commit hot path (synthesis §6:
    "event-id hashing at commit"). Deterministic, no clock/RNG/network (MANIFEST C2).
  - **Money gate:** `money.rs` (exact integer arithmetic, zero floats — CLAUDE.md; `apply_tax` half-up
    at `money.rs:280,284` per the eqc parity pin). Pure integer evaluation.
  - **FSM transition:** `order_machine.rs` `assert_transition` (`:139`) / `fold_transitions` (`:167`)
    — pure, already dual-representation-checked.
  All three are already zero-I/O (the §1.1 discipline synthesis §6 says "generalizes"), so triplication
  is nearly free to add.
- **The breaker trip target does not exist yet** — item 9 builds it. This blueprint's vote-mismatch
  path targets item 9's `TripCause::VoteMismatch` variant, which item-9's blueprint (§3) **reserves
  now at zero code cost** so item 12 adds a variant, not a mechanism.
- **The FDR `Alarm` kind exists** (`fdr/schema.rs:186–190` `Kind::Alarm`) — the vote-mismatch FDR entry
  is a `Kind::Alarm` record. The `VoteOutcome` cause type (roadmap §E: `{Unanimous, SingleDissent(id),
  NoMajority}`) is recorded on it — both non-unanimous classes trip identically (behavioral collapse
  kept; distinct typed cause recorded — the item-50 shape).
- **Item 54 / Sentinel is a distinct, complementary mechanism.** Sentinel (space-grade wave items
  55–78) guards *struct bytes at-rest/at-transition* with a CRC; temporal TMR guards *the evaluation
  itself*. They cover different fault windows — Sentinel cannot see a bit that flips *during* the
  compute and produces a wrong result from correct inputs; temporal TMR catches exactly that. This
  additivity is the roadmap §E item 12 justification and must be stated in both designs.

---

## 3. Design (the artifact — no code now)

Deliverable: **`docs/design/TEMPORAL-TMR-PILOT-2026-07-19.md`** specifying, for the *pilot* (event-id
hash first, as synthesis §9 item 12 names it):

1. **The triple-run harness shape** (Phase-1 code, specified now):
   `fn tmr<T: Eq + Copy>(f: impl Fn() -> T, n: 2..=3) -> VoteOutcome<T>` — run `f` `n` times
   sequentially on one core, compare with trivial `==`. Return `Unanimous(v)`, `SingleDissent(v,
   dissent_idx)` (majority when n=3), or `NoMajority` (n=2 disagreement, or n=3 all-differ).
   **The voter is kept a trivial equality** deliberately (roadmap §E) — a complex voter is itself
   fault-exposed; `==` on `[u8;32]`/`i64`/enum-discriminant is the minimum-exposure comparator.
2. **The `VoteOutcome` type** (roadmap §E): `enum VoteOutcome<T> { Unanimous(T),
   SingleDissent { value: T, replica: u8 }, NoMajority }`. **Both non-unanimous classes trip
   identically** (behavioral collapse — never continue on a disagreement) while recording the distinct
   typed cause to the FDR (item-50 discipline). `SingleDissent` still trips even though a majority
   exists — the pilot does *not* use the majority value to proceed, because on non-ECC hardware a
   dissent is evidence of a live fault, not a recoverable outvote.
3. **The wiring** (Phase-1, gated on item 9): a `NoMajority`/`SingleDissent` outcome →
   `TripCause::VoteMismatch` breaker trip + `Kind::Alarm` FDR record carrying the `VoteOutcome`.
4. **The honest-limits section** (non-negotiable, carried from synthesis §6): temporal TMR is PARTIAL
   — (a) a *permanent* fault (stuck bit) corrupts all runs identically → no dissent → not caught;
   (b) a *software bug* is deterministic → all runs agree on the wrong answer → not caught; (c) it
   catches only *transient* flips during one of the runs. **No SEU-immunity claim.** The shared-silicon
   caveat (synthesis §6, OS-patterns §6): all runs share the same die's cache/ALU, so a correlated
   transient can still hit multiple runs — temporal separation reduces but does not eliminate this.
5. **Overhead sizing** (measured in Phase 1, budgeted now): 2–3× the function's own runtime, applied
   **only** to the 2–3 named µs-scale functions — not a kernel-wide cost. The design states the budget
   and the measure-and-report obligation (synthesis §9 item 12: "overhead measured and reported
   honestly").

**No `kernel/src/` code lands in the design phase.** The Cargo build, hot-path manifest, and zero-dep
gate are untouched until the Phase-1 pilot (gated on item 9).

---

## 4. Tests / proofs — 5-point hardening applicability

Applies to the **Phase-1 pilot code** (design phase produces the spec + the fault-injection proof
plan):

- **Item 1 (oracle):** the `tmr` harness's own correctness is oracle-testable — for a deterministic
  `f`, `tmr(f, n) == Unanimous(f())` always; the vote logic is exhaustively enumerable over the
  small `VoteOutcome` space (all agree / one dissents / all differ) — a `#[test]` covering each.
- **Item 3 (debug-differential):** the vote result cross-checked against a second trivial tally in
  debug builds (like `assert_transition`'s dual representation).
- **Item 5 (formal/fault-injection):** the **falsifiability proof** = a fault-injection test that
  **deliberately corrupts one replica's output** (a testkit hook forcing `f`'s second run to differ)
  and asserts the harness returns non-`Unanimous` → trips the breaker → writes the FDR entry
  (synthesis §9 item 12 proof). This is the item-12 analog of item 7's planted-overflow self-test:
  without a seeded mismatch demonstrably caught, the TMR mechanism is unfalsifiable.
- **Item 2 (dudect):** **N/A** — the vote is a trivial `==` on non-secret values (a hash digest, a
  money integer, an FSM discriminant — none secret-dependent in the CT sense). Record
  `N/A(no-secret-compare)`. (Note: if the pilot ever triples a *signature verify*, that comparator's
  CT property is `pq::dsa`'s existing concern, not TMR's.)
- **Item 4 (asm):** **N/A** — no branch-free constant-time path.

---

## 5. Acceptance criteria (falsifiable)

**Design phase (now):**
1. The design doc exists with the `tmr`/`VoteOutcome` shape, the trivial-equality-voter rationale,
   and the honest-PARTIAL limits section (permanent-fault + software-bug + shared-silicon caveats all
   stated).
2. The additivity-over-item-54 statement is present (Sentinel = at-rest CRC; temporal TMR = compute-
   time flip; named in this doc).
3. The `VoteOutcome::{Unanimous, SingleDissent, NoMajority}` design bakes both-non-unanimous-trip +
   distinct-typed-cause at zero code cost (roadmap §E, item-50 shape).
4. No code landed — `git diff` touches only `docs/`.

**Pilot phase (gated on item 9, criteria stated for handoff):** fault-injection test deliberately
corrupts one replica → breaker trips + FDR `Alarm` written (synthesis §9 item 12); overhead measured
and reported honestly for the event-id-hash pilot; no SEU-immunity claim anywhere in the code/docs.

---

## 6. Dependency gates

- **Design phase (this blueprint):** no dependency — design/scoping starts now (roadmap §0, §E).
- **Pilot phase:** gated on **item 9 (breaker, for the trip target) + Tier-1 FDR (DONE, for the Alarm
  record)**. The `TripCause::VoteMismatch` variant is reserved in item 9's blueprint so the pilot adds
  a variant, not a mechanism.
- **Sequencing vs item 54:** independent (complementary). Neither blocks the other; the additivity
  claim must appear in both docs so they are not later mistaken for duplicates.

---

## 7. Open questions (operator ruling)

1. **Trip-severity of `SingleDissent` (n=3, majority exists).** The design trips on *any* dissent
   rather than outvoting, on the non-ECC-hardware reasoning that a dissent is a live-fault signal. An
   alternative posture — "outvote a single dissent, only trip on `NoMajority`" — trades safety for
   availability. On safety-critical / money/auth paths the trip-on-any-dissent posture is clearly
   right; whether a *non*-red-line path (e.g. a display projection) should tolerate a single dissent
   is an **operator availability-vs-safety ruling**. Flagged; the design defaults to trip-on-any-
   dissent (fail-closed) until ruled otherwise.
2. **Which functions beyond the event-id-hash pilot.** The roadmap names money-gate and FSM-transition
   as candidates; whether the pilot expands to all three or stays at one is a scope call best made after
   the pilot's measured overhead is known. Named, not pre-decided.
