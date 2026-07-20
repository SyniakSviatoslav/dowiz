# BLUEPRINT — Item 10: TLA+ spec of decision-import + order FSM (CI-time, zero runtime footprint)

- **Date:** 2026-07-19 · **Tier:** 3 (roadmap §D) · **Status:** BLUEPRINT (planning artifact, no code)
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §D item 10
  (lines 372–373); `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §7 (TLA+ ruling, line
  152), §9 item 10 (line 174); live source `kernel/src/decision/import.rs`, `kernel/src/decision/mod.rs`,
  `kernel/src/order_machine.rs`.
- **Relationship to item 9:** none. "No structural dependency on the breaker; same-tier verification
  of the same state-machine family, runs in parallel with item 9" (roadmap §D). Both are Tier-3
  verification of hand-built state machines; they share no code and can be executed concurrently.

---

## 1. Scope / goal (one paragraph)

Write TLA+ specs (checked by TLC in CI, **not linked into the kernel** — the same zero-runtime
category as `cargo test` and Kani, synthesis §7) for the two hand-built state machines with real,
already-coded invariants: the **decision-import pipeline** (`kernel/src/decision/import.rs`, the
six-check verify-before-persist gate) and the **order lifecycle FSM** (`kernel/src/order_machine.rs`,
the 12-state decide/fold machine). TLC exhausts the abstract state space checking deadlock/livelock
freedom plus the two named safety invariants — **epoch-no-downgrade** and **replay-before-persist**
— at the abstract level, alongside the Rust that implements them. This is the model-level complement
to item 7's Kani (which proves panic/overflow-freedom of the concrete arithmetic): TLA+ proves the
*temporal/ordering* properties Kani does not target. The deliverable is spec files + a TLC CI job +
the proof that a deliberately-broken spec variant fails (the item-6/P7 "verifier the author cannot
forge" discipline, applied to the model checker).

---

## 2. Verified current state — grounded

- **The order FSM already carries an exhaustive in-Rust proof of its structural properties.**
  `order_machine.rs`: `FSM_ADJ` (`:208`), `fsm_graph_report()` (`:476`) computing `has_cycle`,
  `cyclomatic_number`, `topological_order`, `reachable`, `spectral_radius` (`:391`), pinned to
  `FSM_GOLDEN_SIGNATURE` (`:513`) via `verify_fsm_signature()` (`:543`). **What Rust already proves:**
  acyclicity (ρ = 0 ⟺ DAG), reachability, the golden structural signature. **What TLA+ adds that Rust
  does not:** the *temporal* statement — "from any reachable state, the machine never deadlocks
  (always has a legal successor or is a designated terminal)" and "no illegal transition is ever
  reachable via any interleaving of `decide`/`fold`" — expressed as `[]` (always) / `<>` (eventually)
  temporal operators TLC checks exhaustively over the abstract state graph.
- **The import gate's six ordered checks are documented in-source and are the spec's proof targets.**
  `import.rs:8–16` names them: (1) size, (2) integrity `sha3_256(artifact)==content_id`, (3)
  instance-set pin, (4) **independent replay** (harvested set replayed through the candidate + compared
  to the local oracle — "ANY disagreement ⇒ reject", the P06 `key_V` shape), (5) **epoch check** (never
  downgrade a Live unit — `import.rs:14`, `:55–57` `EpochNotNewer`), (6) lineage-parent-resolves.
  `import_unit()` at `import.rs:81`. The reject taxonomy `ImportReject` (`import.rs:45–62`) — every
  reachable variant maps to an adversarial case (A1–A6).
- **The epoch merge law is a proven join-semilattice already.** `decision/mod.rs:385–395` `merge_meta`
  = max-merge (higher epoch wins), with the property test `epoch_merge_is_semilattice`
  (`decision/mod.rs:550`) already asserting commutative/associative/idempotent over a 4×4×4 sweep.
  TLA+ raises this from "tested over a finite sweep" to "model-checked over the abstract state space
  with the no-downgrade *temporal* invariant" (`[](Live.epoch monotone non-decreasing)`).
- **No TLA+/TLC exists in the repo.** No `.tla`/`.cfg` files; no TLC in CI. Green field.
- **"Replay-before-persist" is a real coded invariant, not aspirational.** `import.rs:78` "On any
  reject, **nothing is persisted** to the log (degrade-closed)"; check (4) runs the replay *before*
  the lineage-row append. The TLA+ invariant is `[](persisted => replay_agreed)` — no state where an
  event is in the log but replay had disagreed.

---

## 3. Implementation plan — exact artifacts

Two specs, one shared CI job. TLA+ tooling (TLA+ Toolbox / `tla2tools.jar` TLC) is CI-time-only,
downloaded in the job like Kani's toolchain — **nothing enters `Cargo.toml`/`Cargo.lock`**; the
zero-dep gate is untouched by construction (same containment as item 7's `kani-gate`).

1. **`docs/formal/OrderFsm.tla` + `OrderFsm.cfg`.** Model the 12 `OrderStatus` variants as a TLA+
   constant set; `FSM_ADJ` as the `Next` relation; `Init` = `Pending`. Invariants:
   `TypeOK`, `NoIllegalTransition` (`[]` — every taken transition is in the adjacency relation),
   `NoDeadlock` (every non-terminal reachable state has ≥1 successor), and `TerminatesOrCycles`
   (matching the Rust ρ=0 acyclicity — TLC confirms no unintended cycle). **The `.cfg` pins the exact
   constant set so it stays in lockstep with `FSM_ADJ`** — a divergence between the `.tla` model and
   the Rust adjacency is caught by §3.3 below.
2. **`docs/formal/DecisionImport.tla` + `.cfg`.** Model the import gate as an abstract action system:
   state = `{registry: DomainTag -> epoch, log: SeqOf(content_id)}`; the `ImportUnit` action is the
   six ordered checks as a TLA+ `\/`-guarded transition. Invariants:
   `EpochNoDowngrade` (`[](\A d: registry'[d] >= registry[d])` — Live epoch never decreases),
   `ReplayBeforePersist` (`[](appended => replay_agreed)`), `LineageClosed` (`[]` every log entry's
   prev resolves in the log), and `NoDeadlock`/`NoLivelock` (the gate always terminates in
   accept-or-reject, never spins).
3. **`docs/formal/README.md`** — the parity note binding each `.tla` constant/relation to its Rust
   source line (`FSM_ADJ` `order_machine.rs:208`; the six checks `import.rs:8–16`), so a reviewer can
   confirm the model matches the code, and the maintenance rule ("if `FSM_ADJ` changes, the `.tla`
   `Next` relation changes in the same diff").
4. **`.github/workflows/ci.yml` `tlc-gate` job** — downloads `tla2tools.jar` (pinned SHA), runs
   `java -jar tla2tools.jar -config OrderFsm.cfg OrderFsm.tla` and the import spec; **asserts TLC
   reports zero invariant violations and completes state-space exploration** (a spec that fails to
   exhaust, e.g. state explosion, is RED-with-reason, not silently green). Triggered on diffs to
   `order_machine.rs`/`decision/` or the `docs/formal/` files, plus unconditionally on main.

---

## 4. Tests / proofs — 5-point hardening applicability

The 5-point checklist (`CHECKLIST.md`) governs *implementation* hot paths. Item 10 is a **model-level
verification artifact**, so the mapping is:

- **Checklist item 1 (oracle):** the Rust already carries the exhaustive oracle (`FSM_GOLDEN_SIGNATURE`,
  `epoch_merge_is_semilattice`, the six import RED→GREEN adversarial tests A1–A6). TLA+ does not replace
  it — it adds the *temporal* layer above it. N/A to re-derive; cite the existing coverage.
- **Checklist item 5 (formal proof):** item 10 **IS** the formal-proof item, at the temporal/abstract
  level (the TLA+ half of synthesis §7, where item 7 is the Kani half). Its own "self-test the verifier"
  obligation (the P7 discipline): **a deliberately-broken spec variant must fail TLC.** Concretely — a
  `OrderFsm_BROKEN.tla` that adds one illegal edge to `Next` must produce a TLC invariant violation on
  `NoIllegalTransition`; a `DecisionImport_BROKEN.tla` that lets an equal-epoch import overwrite Live
  must violate `EpochNoDowngrade`. Without this, TLC "passing" is unfalsifiable (the P7 "verifier the
  author cannot forge" failure one level up). This is the item-10 analog of item 7's
  `proof_selftest_planted_overflow`.
- **Items 2 (dudect), 3 (debug-differential), 4 (asm):** **N/A** — a model checker has no timing,
  no per-call reference, no assembly. Record `N/A(model-artifact)`.

---

## 5. Acceptance criteria (falsifiable)

Straight from synthesis §9 item 10:
1. **TLC exhausts the state space with no deadlock** for both specs (reports full exploration + zero
   invariant violations).
2. **The no-downgrade and replay-before-persist invariants hold** under TLC (`EpochNoDowngrade`,
   `ReplayBeforePersist` GREEN).
3. **A deliberately broken spec variant fails** (the `_BROKEN.tla` variants of §4 produce named TLC
   violations) — the falsifiability proof, recorded in the PR.
4. **Parity with the Rust is documented** (`docs/formal/README.md` binds each model element to its
   source line; the maintenance rule is stated).
5. **Zero-dep gate unchanged** — `cargo tree -e no-dev` identical; `tla2tools.jar` is CI-time only,
   never in `Cargo.*`.

---

## 6. Dependency gates

- **Depends on:** nothing structural. The Rust it models is already merged (`order_machine.rs`,
  `decision/import.rs` at HEAD). Runs in parallel with item 9 (roadmap §D). Independent of item 7's
  Kani (different property class, different tool).
- **Blocks:** nothing. It is a leaf verification artifact.

---

## 7. Open questions (operator ruling)

None requiring an operator ruling. One **executor** judgment call, not an operator gate: whether to
model `decide`/`fold` at the abstraction of the whole `OrderStatus` set (cheap, matches `FSM_ADJ`) or
to include the money/`Locked`-intervention guards as TLA+ preconditions (richer, larger state space).
Recommendation: start at the pure-structural abstraction (matches the already-proven `FSM_ADJ`), add
guard-modeling only if TLC completes fast enough to afford it — the ponytail default. Flagged so the
executor scopes deliberately; no operator decision needed.
