# BLUEPRINT H1 — EVENT-LOG FAIL-OPEN FIX (typed durability pole)

> Closes audit finding **★RC-3** (ranked #1, HIGH) from `HERMETIC-ARCHITECTURE-PRINCIPLES.md` §2/§3.
> Principle: **Polarity V2 × Cause-and-Effect** — restore the missing failure pole on the event-log
> substrate everything else replays from. Anchor: RC-3 "Fix direction" (`fn insert(...) -> Result<(),
> StoreError>` propagated into `AppendOutcome`, plus a RED-first IO-fault-injection test).
> **Depends on:** nothing (surgical, single file-pair; can land before or independent of Phase 7).
> **Precedes / unblocks:** P12 §4.2 (`FileEventStore` durability semantics are un-implementable-as-
> written on today's infallible trait — see §5); strengthens P07 §5 (money event-sourcing rides this
> substrate). **Parallel-safe with:** P01, P04. Coordinates-with (collision): the actively-landing G9
> breach-witness surface in `hydra.rs` (§1, §2.4).
> **Planning artifact only.** No `.rs` file is written here. This touches the kernel event-log
> substrate (a de-facto red-line path) and earns a careful separate implementation pass.

---

## §0 — The problem (live file:line, re-verified 2026-07-16 against HEAD `2a0558e0d`)

`EventStore::insert` is typed **infallible** — `fn insert(&mut self, id: [u8; 32], ev: MeshEvent);`
at **`event_log.rs:166`** (trait spans 162–183). There is no failure pole in the port at all. The
durable implementation `FileEventStore::insert` (**`hydra.rs:825–852`**) swallows every IO `Result`:
the file open is `if let Ok(mut f) = OpenOptions::new()…` (`:840–844`) — an open failure falls
straight through, skipping the write entirely — and each of `write_all`/`flush`/`sync_all` is
discarded with `let _ =` (`:845–847`). Whether or not any byte reached disk, the in-memory map, tip,
and count still advance unconditionally (`:849–851`), `insert` returns `()`, and the caller receives
`AppendOutcome::Committed` (`event_log.rs:272` in `append`, threaded out through `commit_after_decide`
at `:317`). A durability failure is byte-indistinguishable from a durable commit — the exact
Polarity V2 pole-collapse, sitting under the cause-and-effect substrate that replay, `boot_verify`
restore, and the G9 anti-silent-heal breach witness all quantify over.

**Re-verification result:** every audit citation is CONFIRMED exact. `event_log.rs:162-166,272,317`
and `hydra.rs:825-852` (struct at `:712`) are all accurate; `event_log.rs`/`hydra.rs` have not
shifted since the audit baseline (the audit commit IS HEAD). The G9 breach-witness commits
(`1701eabd1`→`d0e71cec9`) all predate HEAD and did **not** touch `FileEventStore::insert` (last
touched by `82e52c02e`) — but they DID add the two `append_raw` callers that this fix collides with
(§2.4).

**Not already covered.** P07 §2 touches `event_log.rs` but only for the dedup-ordering bug and keeps
`MemEventStore` (§5: "Storage remains `MemEventStore` this phase") — it never changes `insert`'s
signature. P01/P04 only name `event_log.rs` as a CI red-line glob. P12 §4.2 designs the *durability
semantics* (fsync-before-ack, torn-write truncation) but was written treating `FileEventStore` as
NOT-YET-BUILT (its §1 evidence "the only implementation is `MemEventStore`" is now **stale**) and
*assumes* a fallible insert without ever designing the trait change, the error type, or the swallow
fix. **H1 is the precondition P12 §4.2 silently presumes.** No duplication.

---

## §1 — Current-state evidence (exact signatures + full caller blast radius)

**The port (infallible):**
- `event_log.rs:166` — `fn insert(&mut self, id: [u8; 32], ev: MeshEvent);` (returns `()`).
- `event_log.rs:200–208` — `impl EventStore for MemEventStore` (`insert` = `HashSet::insert`, no IO).
- `hydra.rs:821–852` — `impl EventStore for FileEventStore`; `insert` at `:825–852` (the swallow).

**The two — and only two — call sites of the trait `insert` method** (grep-verified; all other
`.insert(` hits in the kernel are `HashMap`/`HashSet`/`BTreeMap` calls, including `hydra.rs:849`
which is the concrete impl's own `by_id` map write, not a trait call):
- `event_log.rs:270` — inside `EventLog::append` (`:257–273`); returns `AppendOutcome::Committed(id)`.
- `event_log.rs:287` — inside `EventLog::append_raw` (`:282–290`); returns `AppendOutcome::Committed(id)`.

**Propagation surface — everything that returns / consumes `AppendOutcome`:**
- `EventLog::append` (`:257`) ← called by `commit_after_decide` (`:317`) and tests (`:534,:539`).
- `EventLog::append_raw` (`:282`) ← called by **`Hydra::raise_breach_alarm` (`hydra.rs:310`)** and
  **`Hydra::ingest_peer_breach` (`hydra.rs:342`)** — the G9 collision (§2.4).
- `EventLog::commit_after_decide` (`:300`) → `Result<(AppendOutcome, Option<T>), DecideRejected>`;
  ← called by `commit_after_decide_drift_gate` (`:374`) and tests (`:458,:470,:486,:513`).
- `EventLog::commit_after_decide_drift_gate` (`:347`) → same Result; ← called by `Hydra::commit`
  (`hydra.rs:242`).
- `Hydra::commit` (`hydra.rs:214`) → `Result<(AppendOutcome, Option<T>), DecideRejected>`; ← tests
  (`:387,:404,:421,:521,:950`).

**Both `EventStore` impls must change:** `MemEventStore` (`event_log.rs:200`) and `FileEventStore`
(`hydra.rs:821`).

---

## §2 — Target-state design

### 2.1 The error type (a real durability taxonomy)

A durable append has four failure points; the enum names each so a caller/operator can tell an
open-permission problem from a lost-fsync (the genuinely dangerous one). Kept `Debug + Clone +
PartialEq + Eq` (carries a rendered `io::Error` string, not the non-`Clone`/non-`Eq` `io::Error`
itself, so test assertions and the `Copy` success type are unaffected):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Open(String),   // backing file could not be opened for append (was: silent fall-through)
    Write(String),  // write_all of the event line failed
    Flush(String),  // flush of buffered bytes to the OS failed
    Sync(String),   // sync_all/fsync failed — bytes may not have reached stable storage
}
```

### 2.2 The port — add the failure pole

```rust
fn insert(&mut self, id: [u8; 32], ev: MeshEvent) -> Result<(), StoreError>;
```

- `MemEventStore::insert` → `Ok(())` always (a `HashSet` insert does no IO; honest — it can't fail).
- `FileEventStore::insert` → replace `if let Ok(f) = …` with `let mut f = OpenOptions::…().open(&self.path).map_err(|e| StoreError::Open(e.to_string()))?;` and each `let _ = f.write_all/flush/sync_all` with `.map_err(|e| StoreError::Write/Flush/Sync(e.to_string()))?`. **Order is load-bearing:** the in-memory `by_id`/`tip`/`count` advance (`:849–851`) MUST move to AFTER the `sync_all?` succeeds — so in-memory state never claims an event the disk doesn't hold. The idempotent-duplicate early-return (`:826–828`) stays `Ok(())` (nothing to persist).

### 2.3 `AppendOutcome` stays a success typology — the failure rides `Result`

`AppendOutcome` gains **no new variant**. A `Failed` arm would be a pole-collapse-in-disguise:
`Committed`/`Duplicate` are both *successes the caller then chains a tip from*; "the append may not
have happened" is not an *outcome*, it is the absence of one — which is precisely what `Result`
encodes (P4: "collapses are typed and safe-directed"; same discipline as finding #11's money
`unwrap_or(0)` → `Result`). So:

- `EventLog::append` / `append_raw` → **`Result<AppendOutcome, StoreError>`.** `self.store.insert(id, ev)?` short-circuits BEFORE `set_tip`; only on `Ok` does the tip advance and `Ok(AppendOutcome::Committed(id))` return. The `Duplicate` path takes no IO → `Ok(AppendOutcome::Duplicate(id))`.
- `commit_after_decide` / `commit_after_decide_drift_gate` / `Hydra::commit` already return `Result<_, DecideRejected>`. They now have **two distinct failure poles** that must not blur — a Law rejection (correct, never retry, nothing persisted) vs a store fault (event accepted, durably lost, safe to retry / must alarm). Unify without collapsing them:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitError {
    Rejected(DecideRejected),  // Law/drift/Locked refused it — do NOT retry
    Store(StoreError),         // accepted but not durable — retry / raise alarm
}
```

  These three fns become `Result<(AppendOutcome, Option<T>), CommitError>`: decide/drift/Locked
  rejections map to `CommitError::Rejected`; the inner `self.append(ev)?` maps its `StoreError` to
  `CommitError::Store`.

### 2.4 G9 collision — the breach witness MUST surface a lost write

`raise_breach_alarm` (`hydra.rs:286–315`, freshly landed `b5b583e49`) and `ingest_peer_breach`
(`:329–343`) call `self.log.append_raw(witness)` and **discard the return**. That witness row is the
entire anti-silent-heal guarantee ("a tampered core can never silently heal"). If it fails to reach
disk, returning a cheerful `Some(BreachAlert)` reintroduces the exact fail-open RC-3 names. So:

- `raise_breach_alarm(...) -> Result<Option<BreachAlert>, StoreError>` — `self.log.append_raw(witness)?` before constructing the alert; a lost witness returns `Err`, never `Ok(Some(_))`.
- `ingest_peer_breach(&mut self, alert) -> Result<(), StoreError>` — propagate the `append_raw?`.

This is the only place H1 touches actively-landing G9 code; `FileEventStore::insert` itself is
untouched by G9 and collision-free.

---

## §3 — Migration steps (in order)

1. **Add `StoreError` + `CommitError`** to `event_log.rs` (no callers yet → compiles clean).
2. **Change the `EventStore::insert` trait signature** to `-> Result<(), StoreError>`; update both
   impls (`MemEventStore` → `Ok(())`; `FileEventStore` → `?`-map each IO step, move state-advance
   after `sync_all?`). The compiler now flags every downstream break — follow it.
3. **Propagate through `EventLog::append` / `append_raw`** → `Result<AppendOutcome, StoreError>`
   (insert `?` before `set_tip`).
4. **Propagate through `commit_after_decide` / `commit_after_decide_drift_gate`** →
   `Result<(AppendOutcome, Option<T>), CommitError>`.
5. **Propagate through `Hydra::commit` / `raise_breach_alarm` / `ingest_peer_breach`** (§2.4).
6. **Update existing call sites/tests** (`event_log.rs` tests `:450–608`, `hydra.rs` tests including
   the `file_store_tests` at `:867–970`): `.unwrap()`/`?` the new `Ok` path; `matches!(res,
   Err(DecideRejected(_)))` becomes `Err(CommitError::Rejected(_))`.
7. **Add the RED-first fault-injection test** (§4.1) — write it, watch it fail to even express on the
   old infallible trait, then pass after.

---

## §4 — Acceptance criteria (numbered, falsifiable)

1. **The port has a failure pole.** `EventStore::insert` returns `Result<(), StoreError>`; both
   `MemEventStore` and `FileEventStore` compile against it; `StoreError` has `Open/Write/Flush/Sync`.
2. **★RED-first IO-fault-injection test (load-bearing).** A test-only `FaultyStore` implements
   `EventStore` with `insert` hardcoded to `Err(StoreError::Sync("simulated fsync failure".into()))`
   (models a full disk / read-only mount). Assert `EventLog::new(FaultyStore).append(ev)` returns
   `Err(StoreError::Sync(_))` — **NOT** `Ok(AppendOutcome::Committed(_))` — and that `log.tip()` is
   still `None` and `log.len() == 0` (no in-memory advance on a failed durability barrier). This test
   **cannot be written at all on the pre-fix infallible trait** (that is the RED); it passes after.
3. **`commit_after_decide` distinguishes the two poles.** Over a `FaultyStore`, a *decide-accepted*
   event yields `Err(CommitError::Store(_))` (accepted-but-not-durable); a *decide-rejected* event
   yields `Err(CommitError::Rejected(_))` with nothing attempted on the store. The two are never
   conflated.
4. **`FileEventStore` no longer swallows.** Point a `FileEventStore` at a read-only directory (open
   fails) → `insert` returns `Err(StoreError::Open(_))`; the in-memory `by_id`/`tip`/`count` do NOT
   advance; the caller sees `Err`, never `Committed`. (Falsifies the current `if let Ok(f)`
   fall-through.)
5. **G9 witness surfaces loss.** `raise_breach_alarm` over a `FaultyStore` returns `Err(StoreError)`,
   not `Ok(Some(alert))` — an undelivered anti-silent-heal witness is reported, not masked.
6. **Success path unchanged.** All existing green tests (`event_log.rs`, `hydra.rs`
   `file_store_tests`, the durable closed-loop) pass after mechanical `Ok`-unwrapping — the happy
   path stays byte-for-byte behaviorally identical; only the failure path gained a type.

---

## §5 — What this unblocks downstream

- **P12 §4.2 becomes implementable as written.** Its crash-safety contract — "`append` does
  write + fsync *before* returning `Committed(id)` and before updating the in-memory tip" — is
  *impossible* on today's `-> ()` trait (a caller cannot learn the fsync failed). H1 is the trait
  precondition; P12 then layers atomic-rename/torn-write-truncation on top. (P12's stale §1 evidence
  should also be corrected: `FileEventStore` already exists.)
- **`boot_verify` / replay (`hydra.rs:252`, PRINCIPLE-6) get a trustworthy substrate.** Deterministic
  replay over a log that silently dropped a write reproduces the wrong history *deterministically*;
  once `Committed` means on-disk, "replay reproduces the tip event-id" is a claim about durable bytes.
- **G9 anti-silent-heal is honestly durable.** The WORM breach witness (`raise_breach_alarm` /
  `ingest_peer_breach`) can no longer report a compromise recorded that never reached disk.
- **P07 §5 money event-sourcing is safe to persist.** A `LedgerEntry` debit that returns `Committed`
  while its bytes were lost is a phantom-money bug; H1 makes the persistence boundary fail-closed, in
  the same spirit as P07 §6's tax-overflow `unwrap_or(0)` removal (audit finding #11).

---

*Blueprint H1 complete. Builds nothing. One signature change (`insert -> Result<(), StoreError>`),
propagated through two `EventLog` appends, the three commit paths (via `CommitError`), and the two G9
witness fns — turning a byte-indistinguishable fail-open into a typed, RED-tested failure pole under
the cause-and-effect substrate every downstream guarantee replays from.*
