# BLUEPRINT — Phase 7: MONEY-LAW CLOSURE (red-line correctness before persistence)

> Master plan: `R2-MERGED-PHASE-ROADMAP.md` Phase 7. Anchors: **S5, S9**. Canon: `ARCHITECTURE.md`
> §2 (S5 fail-closed Result; S9 integer + event-sourcing + saga-compensation). Primary evidence:
> `R1-C-kernel-service-compute-storage-gap-analysis.md` §0.2, S5, S9, §2.3/§2.8 — which **corrected**
> the original `SYNTHESIZED-BLUEPRINT-PLAN-2026-07-16.md` P0-A2 acceptance criteria (they were
> insufficient). This blueprint uses R1-C's corrected framing throughout.
>
> **Depends on:** Phase 1 (CI truth floor — kernel tests must run in CI to gate this red-line diff).
> **Strengthened by:** Phase 6 (V1 verifier re-executes this diff independently once it exists — §8).
> **Parallel-safe with:** Phases 5, 6, 8.
> **Scope:** planning only. No code is written here. This touches red-line money code and earns a
> careful separate implementation pass informed by this document.

---

## 1. Current-state evidence (exact file:line)

### 1.1 The `event_log.rs` dedup bug (P0-A2, corrected)

The bug is a **divergence between the id used for the dedup check and the id under which the event is
stored**, caused by prev-chaining happening on one path but not the other.

- `MeshEvent::event_id()` (`kernel/src/event_log.rs:148-155`) hashes
  `sha3_256(prev ‖ actor_pubkey ‖ actor_seq ‖ payload)`. **`prev` is part of the idempotency key.**
- `append()` (`event_log.rs:257-273`) **binds `prev` to the tip first** (lines 261-265: if `ev.prev`
  is zero and a tip exists, `ev.prev = tip`), **then** hashes (`let id = ev.event_id();`, line 266),
  then checks `contains` (267) and stores under that chained id (270-271).
- `commit_after_decide()` (`event_log.rs:283-302`) computes `let id = ev.event_id();` at **line 292
  — BEFORE any prev-chaining** (the event still carries the caller's `prev`, typically zero), checks
  `self.store.contains(&id)` at line 294, runs `decide`, then calls `self.append(ev)` at line 300 —
  where `append` **re-binds `prev` to the tip and re-hashes to a DIFFERENT id**.

**Concrete failure (replay onto a non-empty log):**
1. Commit event A (`prev=0`) onto an empty log. `append` sees `tip=None`, so `prev` stays `0`; stored
   under `id_A = hash(prev=0, A)`; tip ← `id_A`.
2. Commit event B (`prev=0`) via `commit_after_decide`. Dedup check hashes `hash(prev=0, B)` → miss;
   `decide` runs; `append` binds `prev := id_A`, stores under `id_B = hash(prev=id_A, B)`; tip ← `id_B`.
3. **Replay B (`prev=0`) again.** Dedup check hashes `hash(prev=0, B)` — which is **not** `id_B`
   (`= hash(prev=id_A, B)`). `contains` **misses**. `decide` **re-runs**. `append` binds
   `prev := id_B` and commits a **second copy of B** under `hash(prev=id_B, B)`. Duplicate committed.

**Why the existing test hides it.** `dup_event_is_idempotent_no_state_change` (`event_log.rs:433-462`)
commits its first event onto an **empty** log, where `tip=None` means the chained id equals the
unchained id, so the second replay's dedup check happens to match. The genesis case is the *one* case
where the two ids coincide. The bug only manifests when the log is already non-empty. R1-C §0.2:
"P0-A2's stated acceptance ('byte-identical event IDs') is **insufficient** — it needs the RED
replay-on-non-empty-log test." The original blueprint (`SYNTHESIZED-BLUEPRINT` P0-A2, lines 124-131)
framed this as "hash once, thread the value through" with acceptance "existing tests stay green" —
which a naive one-hash refactor could satisfy while preserving the dedup bug. **Not the fix we use.**

### 1.2 FSM is happy-path only (P0-A4, part 1)

- `allowed_next` (`kernel/src/order_machine.rs:64-78`): `InDelivery → [Delivered]` (line 71);
  `Delivered / Rejected / Cancelled / Scheduled / PickedUp` all terminal with `&[]` (lines 72-76).
- The only cancel edges are `Pending → [Rejected, Cancelled]` (line 67) — **pre-commitment** cancels,
  reachable only from `Pending`. Once `Confirmed`, an order **cannot be cancelled or refunded at all**;
  there is no compensating edge and no compensating terminal state. `is_terminal` (lines 54-59) knows
  nothing of compensation.
- Grep for `saga`/`reversal`/`compensat` across kernel = zero (R1-C S9). The golden drift-gate
  `FSM_GOLDEN_SIGNATURE` (`order_machine.rs:472-483`) pins the current 10-state / 9-edge shape; adding
  states **will** trip `verify_fsm_signature_against` (lines 507-541).

### 1.3 `money.rs` has no reversal primitive; money is not evented (P0-A4, parts 2-3)

- `Money::checked_add` (`kernel/src/money.rs:70-87`) is the **only** arithmetic primitive on `Money`.
  No `checked_neg`, no `checked_sub`, no compensating-credit. Reversal is unrepresentable.
- Money is **event-logged nowhere**: no `LedgerEntry` event type exists; the event log
  (`event_log.rs`) carries opaque `payload` bytes but nothing routes money through
  `commit_after_decide`. S9's "event-sourcing" leg is unbuilt for money (R1-C S9: "money is not
  evented").

### 1.4 `estimate_order_total` silently swallows tax overflow (S5 deviation)

- `estimate_order_total` (`money.rs:216-232`) line 218:
  `let tax_total = apply_tax(...).unwrap_or(0);`. `apply_tax` (`money.rs:94-112`) correctly returns
  `Err("tax overflow…")` on a pathological `subtotal × rate` (line 111), but the caller **swallows it
  to `0`** — a silent default in money-adjacent code, contradicting the module's own contract
  ("Returns `None` … the caller must degrade, never show a number it can't back", `money.rs:162-164`)
  and S5 "fail-closed Result always." R1-C §2.8 flags this explicitly.

### 1.5 Engine panic-as-fail-closed is real but undocumented (S5 ambiguity)

- `FieldEquilibrium::assert_stable` (`engine/src/field_frame.rs:53-70`) **panics** (three `assert!`s)
  when `dt` exceeds the CFL stability bound or coefficients are non-physical — deliberate (a divergent
  step must never reach the integrator) but a **named exception** to S5's "Result always" that canon
  never records. R1-C S5: "should be recorded as a named exception."

---

## 2. Fix design — `event_log.rs` (corrected operation ordering)

**Root cause:** two code paths bind `prev` at different times relative to hashing. **Fix:** make
prev-binding happen **before** the dedup hash on *both* paths, hash **once**, and check `contains`
against the **chained** id — the same id under which the event will be stored.

Corrected `commit_after_decide` operation order:

```
1. bind_prev(&mut ev)         // if ev.prev == 0 and tip exists, ev.prev = tip   (was: not done here)
2. let id = ev.event_id()     // hash the CHAINED event exactly once
3. if store.contains(&id) → return Duplicate(id)   // dedup on the id we will actually store
4. let decision = decide(&ev)?                     // decide only on a genuinely new event
5. store.insert(id, ev); store.set_tip(id)         // commit under the already-computed id
```

Implementation shape (design, not final code): extract the prev-binding currently inlined in `append`
(lines 261-265) into a private `fn bind_prev(&self, ev: &mut MeshEvent)`; call it at the top of both
`append` and `commit_after_decide`. Then compute the id once and thread it through a private
`fn commit_bound(&mut self, id, ev)` that skips re-binding and re-hashing (so hashing is done exactly
once per commit — this also satisfies the original P0-A2 "hash once" intent as a side effect, but the
**ordering** is the load-bearing change, not the hash count). `append`'s public contract is unchanged
(it still binds-then-hashes-then-checks, which was always correct); only `commit_after_decide` moves
its bind ahead of its dedup check. `commit_after_decide_drift_gate` (lines 330-358) is untouched — it
delegates to `commit_after_decide` and inherits the fix.

**Invariant established:** for every commit path, the id used for the dedup check is byte-identical to
the id under which the event is (or already was) stored. A replay of the same logical event — whatever
`prev` the caller supplies — resolves to the same chained id and is caught as `Duplicate`.

---

## 3. FSM compensation-state design (`order_machine.rs`)

Add a compensation sub-flow reachable from every **post-commitment** state (money has moved after
`Confirmed`, so a plain `Cancelled` — reachable only from `Pending` — is semantically wrong there).

**New states** (`OrderStatus` enum, plus `from_str`/`as_str`/`is_terminal`):
- `Refunding` — non-terminal compensating state (reversal in progress).
- `CompensatedRefund` — **terminal** compensated state (`is_terminal → true`).

**New edges** (added to `allowed_next`, all forward — the graph must stay a DAG so directed ρ = 0):
- `Confirmed → Refunding`
- `Preparing → Refunding`
- `Ready → Refunding`
- `InDelivery → Refunding`
- `Refunding → CompensatedRefund`

`from_str`/`as_str` gain `"REFUNDING"` / `"COMPENSATED_REFUND"`; `LIFECYCLE_STATES`
(`order_machine.rs:173-184`) and `idx_of` (189-202) extend to 12 entries (Refunding=10,
CompensatedRefund=11). `Cancelled`/`Rejected` keep their current pre-commitment meaning.

**Deliberate golden-signature re-key (named acceptance criterion — never a silent constant edit).**
Adding states trips `FSM_GOLDEN_SIGNATURE` (`order_machine.rs:472-483`). The re-key MUST be produced
by **running `fsm_graph_report()` on the new graph** and pinning the emitted values with a recorded
rationale in the constant's doc-comment (like the existing 2026-07-14 note at lines 462-471) — the
implementer must not hand-edit the numbers to make the gate pass. Hand-derived expectation from the
proposed edge set (for review; confirm by execution):

| field | old | new (expected) | why |
|---|---|---|---|
| vertices | 10 | 12 | +Refunding, +CompensatedRefund |
| edges | 9 | 14 | +5 compensation edges |
| is_acyclic | true | true | all new edges forward → still a DAG |
| cyclomatic μ | 1 | 4 | μ = \|E\|−\|V\|+c = 14−12+2 (Scheduled still orphan, c=2) |
| spectral_radius | 0.0 | 0.0 | acyclic ⇒ nilpotent adjacency |
| reachable_from_pending | 767 | 3839 | +bit10 +bit11 (Pending→Confirmed→Refunding→CompensatedRefund) |
| reachable_states | 9 | 11 | +2 |
| topological_len | Some(10) | Some(12) | acyclic ⇒ full linear extension |

The re-key lands as a **reviewed diff** to `FSM_GOLDEN_SIGNATURE` with the rationale line — visible in
change history, exactly what the drift-gate exists to force (its own doc-comment, lines 469-471, names
this: "upgrade trigger = a deliberate lifecycle change that bumps `FSM_GOLDEN_SIGNATURE` with a
recorded rationale").

---

## 4. Money reversal-primitive design (`money.rs`)

Add fail-closed compensating arithmetic alongside `checked_add` (lines 70-87):

- `Money::checked_neg(self) -> Result<Money, String>` — additive inverse (the compensating credit of a
  debit). Fail-closed on `i64::MIN` (its negation overflows): `self.minor.checked_neg().ok_or(...)`.
  Preserves `currency`.
- `Money::checked_sub(self, other) -> Result<Money, String>` — cross-currency fail-closed (same guard
  as `checked_add`, lines 71-77), then `checked_sub` on `minor`.

**Reversal invariant (the correctness property):** for any `m: Money`,
`m.checked_add(m.checked_neg()?)?  ==  Money::new(0, m.currency)`. A compensating credit is defined as
the `checked_neg` of the original debit, so a debit and its reversal **net to exactly zero** by
construction — the falsifiable done-test's "ledger entries net to EXACTLY zero" reduces to this
algebraic identity plus the fold in §5. RED tests: `checked_neg(i64::MIN)` is `Err`; cross-currency
`checked_sub` is `Err`.

---

## 5. Event-sourcing for money (route through `commit_after_decide`)

Make money **event-sourced**, not merely computed. Money movements become ledger-entry events on the
same content-addressed log the FSM already uses — so the §2 dedup fix is a **hard precondition** (a
buggy dedup would double-commit a debit).

- **`LedgerEntry`** (new type): `{ order_id: [u8;32], kind: LedgerKind, amount: Money,
  ref_entry: Option<[u8;32]> }` where `LedgerKind ∈ {Debit, Credit, Reversal}`. `Reversal` carries the
  content-id of the entry it reverses in `ref_entry`, and its `amount` is the `checked_neg` of that
  entry's amount (§4).
- **Commit path:** serialize a `LedgerEntry` deterministically into the `MeshEvent.payload`
  (`event_log.rs:142`) and commit via `commit_after_decide`. The `decide` closure enforces the ledger
  Law: currency match against the order's currency (M5), `Reversal` must reference an existing prior
  entry, and no entry may be double-applied (dedup handles literal replay; `decide` rejects a second
  `Reversal` of an already-reversed entry). Rejection ⇒ nothing persists (the existing no-partial-commit
  contract, `event_log.rs:294-298`).
- **Replay reducer:** fold the log's `LedgerEntry` events into a running per-order balance, in chain
  order (`prev` links give a total order per node). A `CompensatedRefund`-terminal order's entries
  (`Debit …` + `Reversal(-…)`) **sum to zero**.
- **Determinism proof:** because every entry is content-addressed and chained, replaying the same event
  log from genesis reproduces a **byte-identical** final balance and tip id. This is the
  "money-event replay reproduces identical final state" done-test — a direct consequence of §2's
  chained-id invariant.

Storage remains `MemEventStore` this phase (`event_log.rs:187`); the durable `PgEventStore`/file
adapter is **Phase 12**, gated on this dedup fix (R2 Phase 12 depends on Phase 7; E28 "dedup
precondition fixed in 7"). We persist nothing durably until the id is correct.

---

## 6. Tax-overflow fix (`money.rs:216-232`)

Replace the silent `.unwrap_or(0)` at line 218 with **fail-closed degrade**, matching the module's own
"never show a number it can't back" contract (lines 162-164) and the existing unknown-fee degrade
(`compute_delivery_fee → None → total: None`, lines 205-224):

- On `apply_tax(...) == Err`, the estimate degrades: `total → None` and the tax field carries "unknown"
  (change `tax_total: i64` to `Option<i64>`, or set `total = None` and a `tax_known: false` flag) — the
  caller degrades exactly as it already does for a distance-tiered unknown fee, instead of showing a
  fabricated zero-tax total.
- This is the minimal, pattern-matching fix (preferred over widening the signature to
  `Result<OrderTotalEstimate, String>`, which would ripple through callers for a display mirror). The
  implementation pass picks the field shape; the **behavioral requirement** is: a tax-overflow input
  produces `total: None`, never a wrong number. RED test: an `estimate_order_total` whose `apply_tax`
  overflows returns `total == None` (fails on today's `unwrap_or(0)`, passes after).

**S5 documentation (also this phase):** add a canon note (ARCHITECTURE §2 S5 or a doc-comment on
`assert_stable`) recording **panic-as-fail-closed** as the ONE sanctioned deviation from "Result
always" — an unstable integrator step (`field_frame.rs:53-70`) is an invariant violation, not a
recoverable condition; a `Result` there would invite an `unwrap_or` that silently continues into
divergence (precisely the anti-pattern §6 removes from money). The persistence boundary uses the
Result-based fail-closed form instead (`commit_after_decide` / drift-gate). Name both, so S5 is
unambiguous.

---

## 7. Acceptance criteria (numbered checklist)

1. **Corrected dedup ordering.** `commit_after_decide` binds `prev` **before** computing the dedup id;
   the id checked against `contains` is byte-identical to the id stored. (Code review confirms the
   ordering, not just a hash-count reduction.)
2. **The missing RED test — replay on a NON-empty log.** New test: commit A (`prev=0`), commit B
   (`prev=0`) — B is now chained onto a non-empty log — then **replay B (`prev=0`)**. Assert:
   `AppendOutcome::Duplicate`, `decide` invoked **exactly once** across both B appends, and
   `log.len()` unchanged (`== 2`). **Demonstrate both states:** this test **FAILS on pre-fix code**
   (len becomes 3, decide re-runs) and **PASSES after the fix** — record both runs, not only the green
   one. (The genesis-only `dup_event_is_idempotent_no_state_change`, lines 433-462, stays green
   throughout — proving it never covered this case.)
3. **Compensation states + edges** added to `order_machine.rs`: `Refunding`, `CompensatedRefund`, and
   the five forward edges of §3; `is_terminal(CompensatedRefund) == true`, `is_terminal(Refunding) ==
   false`; `from_str`/`as_str` round-trip the new strings.
4. **Cancel-after-confirm reaches a terminal compensated state.**
   `fold_transitions(Confirmed, &[Refunding, CompensatedRefund]) == Ok(CompensatedRefund)`, and the
   order's ledger entries (the original debit + its `Reversal`) **net to exactly zero**.
5. **Money reversal primitive.** `checked_neg`/`checked_sub` exist and are fail-closed;
   `m + neg(m) == 0` for all valid `m`; `checked_neg(i64::MIN)` and cross-currency `checked_sub` are
   `Err`.
6. **Money is event-sourced.** `LedgerEntry` events commit through `commit_after_decide` with a
   `decide` Law; a rejected entry persists nothing; **replay from the event log reproduces an identical
   final balance and tip id** (determinism proof).
7. **Deliberate, reviewed drift-gate re-key.** `FSM_GOLDEN_SIGNATURE` is updated by running
   `fsm_graph_report()` on the new graph and pinning the emitted values with a recorded rationale
   doc-comment — a **named diff visible in change history**, never a silent constant edit.
   `green_live_signature_matches_golden` (lines 941-950) passes against the new signature.
8. **Tax overflow fails closed.** `estimate_order_total` on a tax-overflow input yields `total: None`
   (degrade), never `0`; RED before, GREEN after.
9. **S5 convention documented.** Panic-as-fail-closed (`field_frame.rs:53-70`) recorded as the single
   named exception to "Result always"; the Result-based persistence-boundary fail-close named alongside.
10. **CI-gated.** All of the above run under Phase 1's restored `cargo test` CI job (S9's blocking-CI
    leg); the kernel suite (currently 337/0) grows by the new RED/GREEN tests and stays green.

---

## 8. Phase-6 verifier consumption (this is the first real consumer)

Phase 7 is explicitly named in R2 as **"the first real consumer of the V5-C verifier"** on a red-line
diff. Sequencing (Phase 7 depends on Phase 1, is parallel-safe with Phase 6):

- **Under Phase 1 (hard dependency):** this money diff first lands gated by Phase 1's *unsigned* V5-C
  local re-exec harness — a fresh clean checkout re-executes the kernel suite (including criteria 2, 4,
  6) and emits RED|GREEN + rationale. That is the floor that makes any "GREEN" here trustworthy.
- **Once Phase 6 exists (strengthening):** Phase 6 builds split-identity K/V signing (diff-signer
  `key_K`, verdict-signer `key_V`, `K≠V`) and a CI merge gate requiring both signatures GREEN on
  **red-line paths** (`money.rs`/`order_machine.rs`/`event_log.rs` are exactly those paths). This
  money change — a genuine red-line diff with a real RED→GREEN transition — is the **canonical first
  diff to be independently re-verified** by that mechanism: `key_K` signs `sha3(commit) ‖ sha3(diff)`;
  an independent context re-runs the suite and, only if the replay-on-non-empty-log RED test and the
  ledger-nets-to-zero test pass, emits a `key_V`-signed GREEN verdict carrying the standing residue
  line ("identity ≠ person"). A `key_K`-signed-only or RED verdict cannot merge.
- **Stated relationship:** Phase 7 proves the verifier has real red-line work to do; Phase 6 proves the
  verifier is independent. Land Phase 7 under Phase 1's harness now; when both exist, re-verify **this
  same diff** through Phase 6's signed gate as the proof-of-life for the whole V1 mechanism. Do not
  block Phase 7 on Phase 6 (they are parallel) — but record this diff's sha3 so Phase 6 can retroverify
  it as its first case.

---

*Phase 7 blueprint. Evidence re-read against the live tree 2026-07-16 (`event_log.rs`,
`order_machine.rs`, `money.rs`, `engine/src/field_frame.rs`). Corrected P0-A2 framing per R1-C §0.2/§2.3
— the replay-on-non-empty-log RED test is the acceptance criterion the original blueprint omitted.
No code written; red-line money/orders/event-log changes earn a careful separate implementation pass
informed by this document.*

---

## 9 — Planning-protocol completion appendix (2026-07-17, decorrelated pass)

> Independent verifier pass. Re-checked every cited file:line on the actual branch this blueprint lives
> on (`feat/harness-llm-backend`, HEAD `cc3d5c916`), and specifically checked whether either of the two
> fixes this blueprint designs (§2 dedup, §6 tax-overflow) have already landed — since this is exactly
> the kind of claim recent commits can make stale.

### (i) Citation-verification results

**Corrected — §1.4/§6 tax-overflow fix IS ALREADY BUILT on this branch.** `kernel/src/money.rs`'s
`estimate_order_total` no longer has the `.unwrap_or(0)` §1.4 describes. Live code today:
`apply_tax(...).ok()` feeds `tax_total: Option<i64>`, and `total` is computed as `match (delivery_fee,
tax_total) { (Some,Some) => Some(...), _ => None }` — i.e. a tax-overflow degrades `total` to `None`
exactly as §6's fix design specifies, down to reusing the existing fee-unknown-degrade pattern the
blueprint names as the template. A test comment at `money.rs:425` ("Pre-fix `.unwrap_or(0)`:
tax_total=0, total=Some(1200) — a fabricated zero-tax total") documents the RED-before/GREEN-after this
blueprint's acceptance criterion #8 asks for. **This landed via commit `aedba0133` ("fix(hermetic):
P07 §6 money tax-overflow fix (row #11)")** — done-test #8 in §7 is satisfied today.

**NOT stale — §1.1/§2's dedup bug is still live on this branch, but a fix exists on a sibling branch.**
`kernel/src/event_log.rs`'s `commit_after_decide` (currently at line 339, `event_id()` at 148, `append()`
at 293) still shows the exact pre-fix pattern: it computes `id = ev.event_id()` from the caller-supplied
(unbound) `prev` at line 348, dedup-checks that id, then commits via `self.append(ev)` at line 359 —
`append` itself rebinds `prev` to the tip and re-hashes, reproducing the divergence §1.1 describes. **A
fix exists — commit `f30189262` ("feat(agentic-mesh): P07 dedup fix...")** — but it lives on branch
`feat/agentic-mesh-protocol-2026-07-17`, confirmed via `git merge-base --is-ancestor f30189262 HEAD` →
**not an ancestor of this branch's HEAD.** So relative to the branch this blueprint (and this appendix)
actually lives on, §1.1/§2's "not yet fixed" framing remains accurate — but it will go stale the moment
that branch merges, and worth flagging precisely because **the landed fix takes a structurally different
approach than this blueprint designs**: §2 proposes reordering `bind_prev` to run *before* the hash on
both paths, keeping prev-chaining; the actual fix instead makes `commit_after_decide` stop prev-chaining
entirely and persist under the event's raw content-id via a new `append_raw` (justified in the commit
message: prev-chaining is "provably incompatible with true replay-idempotence for zero-prev events").
Both satisfy this blueprint's own §7 acceptance criteria #1-#2 (dedup id == stored id; replay-on-
non-empty-log caught as `Duplicate`, `decide` runs exactly once) — but a future merge of that branch will
make §2's specific "reorder bind_prev" design (not its acceptance criteria) obsolete. Not corrected in
the blueprint body per this task's read-only-except-appendix rule; flagged here so the next reader does
not implement §2 as literally written once the other branch lands.

**Re-verified, unchanged:** `order_machine.rs` — `is_terminal:54`, `allowed_next:64`,
`FSM_GOLDEN_SIGNATURE:472`, `verify_fsm_signature_against:507` all match the cited ranges exactly;
`Refunding`/`CompensatedRefund`/`LedgerEntry`/`checked_neg`/`checked_sub` — zero grep hits anywhere in
the kernel, confirming §3, §4, §5 (compensation states, reversal primitives, event-sourced money) remain
entirely unbuilt, exactly as designed and not yet touched.

**Minor drift:** the genesis-only test `dup_event_is_idempotent_no_state_change` cited at
`event_log.rs:433-462` now sits at **line 533** (other commits — G9 hub convergence, the Воля АНУ
organism work, the H1 fail-open fix — inserted ~100 lines above it since this blueprint was written).
Content and behavior unchanged; only the line range needs re-pinning.

### (ii) DECART

**No DECART owed.** Every change this blueprint designs is internal kernel logic (`event_log.rs`,
`order_machine.rs`, `money.rs`) — no new crate, service, or external dependency anywhere in the
document.

### (iii) 2-question doubt audit

**Q1 — least confident about (concrete):**
1. I read the `f30189262` diff and commit message but did not check out that branch and run its test
   suite myself — I am trusting the commit message's "kernel: 403 passed... zero regressions" claim,
   not independently re-executing it.
2. I did not check whether `Hydra::commit` (named in the `f30189262` message as `commit_after_decide`'s
   "only production caller" that "no longer prev-chains") is actually the only caller on *this* branch
   too, or whether this branch's `hydra.rs`/other callers assume prev-chaining still happens — if any
   caller here relies on `commit_after_decide` chaining `prev`, merging that fix later would be a real
   behavioral break this pass did not check for.
3. I did not verify whether the FSM golden-signature re-key hand-derivation in §3 (the table predicting
   vertices=12, edges=14, μ=4, reachable_from_pending=3839) is numerically correct — the blueprint itself
   says "confirm by execution," and I did not execute it (no compensation states exist to run
   `fsm_graph_report()` against yet).
4. I did not confirm the "Phase 12 depends on Phase 7 for the dedup precondition" claim (§5, "E28")
   against Phase 12's own blueprint text — out of my assigned files, flagged rather than assumed.
5. §1.5's engine-panic-as-fail-closed citation (`field_frame.rs:53-70`) was not re-read this pass; I
   relied on it being orthogonal to the money/event-log changes I was checking.
6. I did not check whether any code on this branch already partially anticipates the LedgerEntry/
   reversal design (e.g. a stub type or a TODO) that a plain grep for the exact names would miss —
   only exact-string greps were run.

**Q2 — biggest thing I might be missing:** this blueprint's own §8 already names the risk that Phase 7
could land as a red-line diff before Phase 6's signed verifier exists — what it doesn't anticipate is
that the dedup fix would land on a **different, not-yet-merged branch**, meaning that when that branch
does merge, the merge itself becomes exactly the kind of red-line diff (`event_log.rs`) whose commit
message will need re-verification against §7's acceptance criteria, but under a design the original
author of that fix (a decorrelated pass per its own commit message) chose independently rather than by
reading this blueprint's §2. That is actually a small positive signal — two independent passes converged
on satisfying the same acceptance criteria via different means — but it means whoever merges that
branch should re-run §7's criteria #1-#2 against the merged code, not assume this blueprint's §2 prose
still describes what's there.

### (iv) Anu & Ananke check

**Anu.** The one dependency this blueprint states plainly — "Depends on: Phase 1... kernel tests must
run in CI to gate this red-line diff" — is not merely asserted; it is now **directly checkable and
true**: `tools/ci-truth/src/main.rs:228-230` greps the diff range for exactly `money.rs`,
`order_machine.rs`, `event_log.rs` — the identical three files this blueprint calls its red-line
surface. Re-deriving this claim (rather than trusting the roadmap table) confirms it holds, and holds
more strongly than when written (the gate now exists in code, not just in a phase-dependency table).
The Q1.2 gap above (whether the cross-branch fix breaks a same-branch caller) is the one place this
pass could not fully re-derive a safety claim from evidence in front of it — flagged rather than
asserted clean.

**Ananke.** What survives on structure alone: the acceptance criteria (§7, 10 items) are genuinely
falsifiable and, per (i) above, one of them (criterion #8, tax overflow) has already been mechanically
satisfied and could be checked today by running the test suite — nobody needs to remember this
blueprint exists for that fix to stay correct, because it is pinned by a passing test. What does NOT
survive on structure alone: the relationship between this blueprint's §2 design and the actual,
differently-shaped fix on the sibling branch depends entirely on a human (or a future pass like this
one) noticing the divergence before treating §2's prose as an implementation spec — nothing here or in
the sibling branch's commit cross-links the two, so a merge could silently orphan this blueprint's §2
narrative while its acceptance criteria stay satisfied by coincidence of correct design, not by
traceable linkage. Flagged here as the concrete fix: when that branch merges, add one line to §2 or to
the merge commit noting "implemented via `append_raw`, not reordered `bind_prev` — see commit
`f30189262`" so the record closes instead of silently diverging.
