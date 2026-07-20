# Formal specs — Item 10 (space-grade roadmap §D item 10)

TLA+ models of the two hand-built state machines in the kernel, checked by **TLC
at CI time only**. They are **not linked into the Rust** — the same zero-runtime
verification category as `cargo test` and Kani (synthesis §7). TLA+ proves the
*temporal / ordering* properties Kani does not target (item 7 proves the concrete
arithmetic panic/overflow-freedom). Tooling (`tla2tools.jar`) is CI-time only and
never enters `Cargo.toml`/`Cargo.lock`, so the zero-dep gate is untouched by
construction (same containment as item 7's `kani-gate`).

## Artifacts

| File | Models | Rust source of truth |
|-------|--------|----------------------|
| `OrderFsm.tla` + `.cfg` | order-lifecycle FSM (12 `OrderStatus`) | `kernel/src/order_machine.rs` |
| `OrderFsm_BROKEN.tla` + `.cfg` | falsifiability twin (injected illegal edge) | — |
| `DecisionImport.tla` + `.cfg` | import verify-before-persist gate (6 checks) | `kernel/src/decision/import.rs` |
| `DecisionImport_BROKEN.tla` + `.cfg` | falsifiability twin (epoch downgrade) | — |

## Parity bindings (model element → Rust line)

### `OrderFsm.tla` ↔ `order_machine.rs`
- The 12 `OrderStatus` variants ↔ `LIFECYCLE_STATES` (`order_machine.rs:245`).
- `IsLegal(from,to)` ↔ `allowed_next(from)` (`order_machine.rs:78`) and the
  compile-time `FSM_ADJ` bitmask (`order_machine.rs:208`). The `.cfg` pins the
  exact `STATES` constant set so the model cannot silently diverge from `FSM_ADJ`.
- `Init` = `Pending` ↔ `OrderStatus::Pending`.
- `TerminatesOrCycles` (acyclicity) ↔ `has_cycle()==false` and
  `spectral_radius()==0` (`order_machine.rs:584` / `:383`); Rust proves
  ρ=0 ⟺ DAG, TLA+ proves it over the *abstract* state space with `<>`/`[]`.
- `NoDeadlock` ↔ Rust's terminal set (`is_terminal`, `order_machine.rs:64`):
  a non-terminal state always has ≥1 legal successor.

### `DecisionImport.tla` ↔ `decision/import.rs`
- The six ordered checks (`import.rs:8–16`) are the `ImportUnit` `\/`-guarded
  action. Checks 1–3 (size / integrity / instance-set) are byte/transport-shape
  concerns, structurally satisfied by the abstract candidate; checks 4–6 are the
  modelled gates:
  - **check 4 independent replay** → `replayOK` set by `didReplayAgree`
    (`import.rs:119–128`).
  - **check 5 epoch no-downgrade** → `epochPass == candEpoch > registry[t]`
    (`import.rs:130–135`, `EpochNotNewer`).
  - **check 6 lineage parent** → `lineagePass == prev \in log` (`import.rs:137–145`).
- On any reject the action **stutters**: `registry'`/`log'` unchanged → nothing
  persisted (degrade-closed, `import.rs:78`).
- `EpochNoDowngrade` ↔ the max-merge semilattice `merge_meta`
  (`decision/mod.rs:392`) lifted to a `[]` temporal law over the registry.
- `ReplayBeforePersist` ↔ `import.rs:78` ("nothing is persisted on reject") +
  replay runs *before* the lineage append.

## Invariants checked by TLC

**OrderFsm**: `TypeOK`, `NoIllegalTransition` (`[]` every transition ∈ adjacency),
`NoDeadlock`, `TerminatesOrCycles` (ρ=0 acyclicity).

**DecisionImport**: `TypeOK`, `EpochNoDowngrade` (`[]` Live epoch non-decreasing),
`ReplayBeforePersist` (`[](appended => replay_agreed)`), `LineageClosed`
(every log entry's prev resolves), `NoDeadlock`/`NoLivelock` (the gate always
terminates in accept-or-reject).

## Falsifiability (the "verifier the author cannot forge" discipline, P7)

- `OrderFsm_BROKEN.tla` adds one illegal edge (`Pending -> Ready`) to
  `Successors`. TLC must report a **`NoIllegalTransition`** violation. The edge is
  a forward edge (no cycle), so TLC still *terminates* and reports rather than
  looping — proving the violation is real, not a state-explosion hang.
- `DecisionImport_BROKEN.tla` changes `epochPass` from `>` to `>=`, letting an
  equal-epoch candidate overwrite a Live unit. TLC must report an
  **`EpochNoDowngrade`** violation.

A passing `tlc-gate` with a GREEN broken twin would be a RED-with-reason CI
failure (the P7 failure one level up).

## Maintenance rule

If `allowed_next` / `FSM_ADJ` in `order_machine.rs` changes, the `IsLegal`
relation in `OrderFsm.tla` (and the `STATES` constant in `OrderFsm.cfg`) MUST
change in the **same diff**. A divergence is caught by `NoIllegalTransition` going
RED (or by the `tlc-gate` job). Likewise, any change to the import six-check
order or the no-downgrade law must update `DecisionImport.tla` in the same diff.

## Running locally (optional, CI does this)

```
java -jar tla2tools.jar -config OrderFsm.cfg OrderFsm.tla
java -jar tla2tools.jar -config DecisionImport.cfg DecisionImport.tla
# falsifiability:
java -jar tla2tools.jar -config OrderFsm_BROKEN.cfg OrderFsm_BROKEN.tla        # expect RED (NoIllegalTransition)
java -jar tla2tools.jar -config DecisionImport_BROKEN.cfg DecisionImport_BROKEN.tla  # expect RED (EpochNoDowngrade)
```
