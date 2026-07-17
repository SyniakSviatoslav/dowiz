# P07 §2 dedup-fix — decorrelated verification

**Scope.** Uncommitted change in `kernel/src/event_log.rs`: `commit_after_decide`
now persists via `append_raw` (content-only id) instead of `append`
(tip-rebinding hash-chain). Read-only review; no edits. Worktree
`feat/agentic-mesh-protocol-2026-07-17`.

**Verdict up front.** (a) SAFE to keep as-is. (b) No *realized* integrity
property is lost for production (`Hydra::commit`); what is lost is a **latent,
unverified** chaining property. (c) **Keep the fix at the applied scope** — the
narrower alternatives are either wrong or leave the bug live. Details below.

---

## 1. Caller trace — the fix sits on the ONLY production commit path

- `commit_after_decide` (`event_log.rs:357`) — sole non-test caller is
  `commit_after_decide_drift_gate` at `event_log.rs:439`.
- `commit_after_decide_drift_gate` (`event_log.rs:410`) — sole non-test caller
  is `Hydra::commit` at `hydra.rs:244`.
- `Hydra::commit` (`hydra.rs:214`) is documented as *"The ONLY public surface
  (G7: source-hiding)"* (`hydra.rs:202`). Every decide-gated / drift-gated
  commit funnels through here. Confirmed: the fix changes the kernel's primary
  production decide-gated persistence path, exactly as claimed. (The other
  `append_raw` callers — `hydra.rs:313`, `:346` — are the breach-witness /
  peer-breach WORM rows, which were *already* content-addressed by design and
  are unaffected.)

## 2. What prev-chaining was FOR — and whether removal loses tamper-evidence

The module advertises a hash chain: `MeshEvent.prev` is *"the content-id of the
preceding event at this node"* and `event_id = SHA3-256(prev ‖ actor_pubkey ‖
actor_seq ‖ payload)` (`event_log.rs:129-155`). In principle a broken `prev`
link makes reorder/drop/insert detectable.

**The decisive finding: nothing in the kernel ever verifies a chain link.** A
whole-tree grep for reads of `.prev` returns only three sites: the hash input
(`event_log.rs:150`), the rebind itself (`:297-299`), and file serialization
(`hydra.rs:863`). There is **no chain-walk verifier** anywhere. Concretely:

- `FileEventStore::open` (`hydra.rs:754-786`) replays each line, recomputes
  `ev.event_id()`, and sets `tip = last valid line`. It **never** asserts
  `line[N+1].prev == id(line[N])`. A reordered or spliced file replays without
  complaint.
- Hydra's tamper detection — `integrity_check` (`hydra.rs:180`) and
  `boot_verify` (`hydra.rs:253`) — recomputes the **baseline spectral radius**
  from `self.base_edges` (`hydra.rs:181`, `:254`) and fails closed to `Locked`
  if ρ ≥ 1. This is **completely independent of the event log**: `base_edges` is
  set once in `new` (`hydra.rs:163-167`) and never mutated by a commit
  (`commit` clones it into a local at `:235`; the only writes to `base_edges`
  are in `#[cfg(test)]` tamper simulations, `:506/:520/:549/:713`). So Hydra's
  integrity guarantee neither uses nor needs the log's `prev` chain.
- `actor_seq` is in the hash but no code validates its monotonicity or gaps —
  the token appears only in `event_log.rs` and `hydra.rs`, never in a
  reorder/sequence check. `order_machine.rs`'s `fold_transitions`
  (`order_machine.rs:140`) folds a caller-supplied `&[OrderStatus]` slice and
  never reads the event log or `actor_seq`. So "actor_seq + decide catches
  reordering" is **not implemented** today either.

**Conclusion for (2):** removing prev-chaining removes a property that was never
enforced. It is latent data with no verifier. Tamper-evidence for the actual
production organism runs through the spectral `integrity_check`/`boot_verify`
path, which is chain-independent. No realized detection capability is lost.

(Aside, pre-existing: `MeshEvent` has **no signature field** — the module's
"verifies signatures" note, `event_log.rs:274`, is aspirational. Not caused by,
and not worsened by, this fix.)

## 3. hydra.rs reliance on chaining — none

`Hydra::boot_verify` / `integrity_check` (§2) derive from `base_edges`, not the
log. The durable round-trip test `hydra_durable_closed_loop_across_restart`
(`hydra.rs:958`) exercises FileEventStore reopen + `boot_verify` and passes
without any chain check. Nothing in hydra.rs behaves differently — correctly or
incorrectly — now that `commit_after_decide` events are content-addressed. All
16 hydra unit tests pass GREEN under the fix.

## 4. Was a narrower fix viable? No — and this is the crux

The B2 blueprint originally named the fix **"bind-prev-before-dedup"**
(`BLUEPRINT-B2-…:261,363`): rebind `prev` to the tip *before* computing the
dedup id, so check-id == store-id. **That approach does not actually fix the
DoD case.** By the time a replay arrives, the tip has advanced (the whole
premise of "replay onto a *non-empty*/advanced log"). Rebinding the replay to
the *current* tip yields `H(tip_now ‖ …)`, which differs from the original's
`H(tip_then ‖ …)` — so the replay still misses dedup and still double-commits.
**Chaining and content-idempotent-replay are mutually exclusive**: a
position-dependent id cannot also be replay-stable. Because B2 requires a
replayed `SettlementClaimed` to be a structural no-op (a *money-law* property,
not cosmetic), the id MUST be content-only. `append_raw` is the only approach
that delivers this. The applied fix is not just adequate — it is the correct
one, and the blueprint's own named approach would have shipped a still-broken
DoD.

Could a *new separate variant* (content-addressed, for B2 only) have been added
while leaving `commit_after_decide` on `append`? No: `Hydra::commit` — the live
production path — passes zero-`prev` events through `commit_after_decide`, so
leaving it on `append` **keeps the exact double-commit bug live on the
production path**. The bug lives in the shared primitive precisely because that
primitive is shared; fixing it there is the right scope. Narrowing would either
retain the defect for `Hydra::commit` or require touching the same primitive
anyway.

## 5. Residual note (minor, non-blocking)

Post-fix, `EventLog::append` (the chaining primitive) has **zero non-test
callers** (all `.append(` production hits at `spool.rs`/`spine.rs`/`evals.rs`
are unrelated methods on other types; only tests at `event_log.rs:729/734/817`
hit `EventLog::append`). The hash-chain is now fully dormant infrastructure. The
standalone test `local_first_chaining_binds_prev_to_tip` (`event_log.rs:727`)
still passes but only asserts the id *changes* under rebind — it tests no
detection capability. If tamper-evident *ordering* is ever wanted, it must be
built fresh regardless (no verifier exists today) and reconciled with the
content-idempotency requirement. Recommend a one-line doc note on `append`
marking it reserved/explicit-chaining-only, and stating chain-link verification
is not implemented, so no future reader assumes tamper-evidence that isn't
there.

## 6. Test evidence

`cargo test --lib event_log::` → 13 passed / 0 failed, including the new
`commit_after_decide_replay_on_nonempty_log_is_true_duplicate` and the retained
`local_first_chaining_binds_prev_to_tip`. `cargo test --lib hydra::` → 16 passed
/ 0 failed. RED→GREEN claim substantiated; no regression in the production
caller.

---

**Final answer.** (a) SAFE. (b) Loses only a latent, never-verified chaining
property; **no** realized tamper-evidence property that matters for
`Hydra::commit` — severity effectively nil today (Hydra integrity is spectral,
chain-independent; no chain-walk verifier, replay-link check, or actor_seq
validation exists). (c) **Keep as-is.** It is the correct scope; the
blueprint's "bind-prev-before-dedup" alternative would not have satisfied the
DoD, and a new-variant narrowing would leave the production path still buggy.
