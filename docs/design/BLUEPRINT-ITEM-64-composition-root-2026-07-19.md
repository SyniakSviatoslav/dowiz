# BLUEPRINT — Item 64: Capability-Secure Declarative Composition Root

- **Date:** 2026-07-19 · **Tier:** 1-class build (roadmap §K) · **Status:** BLUEPRINT (planning
  artifact, no code) — dispatchable now; the only §K item backed by a *proven* defect.
- **Sources (read this session):**
  `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §K item 64 (lines 1101–1120) + the §K
  dependency line (1248–1255); `BLUEPRINT-P-FILE-EVENT-STORE-WIRING-GAP-2026-07-19.md` (the filed
  defect this item closes); `BLUEPRINT-ITEM-02-file-event-store-verification-2026-07-19.md` (item
  2's original proof condition); `docs/audits/hardening/CHECKLIST.md` (the 5-point standard).
  Ground-truth for all code citations: this worktree at HEAD `6701bbb6f`.
- **Upstream (all landed, cited live):** durable store `FileEventStore` (`kernel/src/hydra.rs:923`,
  `::open` :934, `impl EventStore` :1032); FSM graph-proof kit
  (`has_cycle`/`topological_order`/`reachable` in `kernel/src/order_machine.rs`, item-7 inventory);
  refuse-the-adapter pattern (`kernel/src/isolation/microvm.rs:76`); FDR read-only recovery
  (`kernel/src/fdr/ring.rs:227`); clean-stop marker (`Kind::CleanShutdown`, `kernel/src/fdr/schema.rs:194`).
- **Downstream:** item 65 (typed capability tokens — this root is their SOLE minter); item 66
  (durable-log scrub — pointless until this root wires the store); item 73/74 (§L — the gate/registry
  live at/behind this root).
- **SUBSUMES:** `BLUEPRINT-P-FILE-EVENT-STORE-WIRING-GAP-2026-07-19.md` — that Tier-1 fix ("wire a
  durable composition root") is a strict subset of this item's deliverable (i).

---

## 0. THE PROVEN DEFECT — read this first

Item 64 is the single §K item whose need is not argued but *measured*. The roadmap's own words
(line 1102): "the only one backed by a PROVEN defect: item 2's finding that NO production composition
root constructs the durable store."

Re-verified this session against live HEAD, not taken on citation:

- `FileEventStore` (the durable, IO-safe, `Result`-typed audit store) is constructed in **zero
  production code paths.** Every construction site is under `#[cfg(test)]` or a `tests/` binary —
  `kernel/src/hydra.rs:{1107,1123,1154,1175,1207}` all sit past the `mod file_store_tests` boundary
  at `:1085`, and `agent-adapters/tests/e2e_admission.rs:92` is an integration binary
  (`BLUEPRINT-P-FILE-EVENT-STORE-WIRING-GAP` §VERDICT(a), grep-confirmed).
- **No binary crate builds any durable `Hydra`/`EventLog`/`FileEventStore` at all.** The two live host
  binaries construct their state without touching durability: `tools/native-spa-server/src/main.rs:48`
  calls `ApiState::build_default()` (no store); `agent-loop/src/main.rs` wires an `AgentLoop` over an
  `OllamaAdapter` with a `FixtureOrders` source and never names `EventLog`. Every production
  `EventLog::new(...)` in the kernel is over the *in-memory* `MemEventStore`
  (`kernel/src/hub_supervisor.rs:366`, `kernel/src/decision/import.rs:207`, etc.).

The consequence the wiring-gap blueprint states plainly (its one-sentence summary): "a correct,
tested, unreachable audit trail is materially different from 'not yet built'." Item 64 is the fix,
but generalized past a single `let store = FileEventStore::open(...)` line into the OS pattern that
makes such a gap *structurally* unrepresentable for every future durable resource.

---

## 1. Scope / goal

Replace the flat, ad-hoc `main()` wiring of the host binaries with a **declarative,
dependency-ordered init** — a composition root — with four properties, in dependency order:

- **(i) Explicit init order from a declared DAG**, validated by the *existing* `order_machine`
  proof kit reused over module-init nodes. A cyclic init dependency is a caught startup error, not a
  runtime surprise. This closes item 2's defect: the root constructs the durable
  `FileEventStore`/`EventLog`.
- **(ii) Fail-closed capability declaration per module.** Each module declares the
  ports/capabilities it requires and refuses to init if one is absent — generalizing
  `isolation/microvm.rs`'s refuse-the-adapter posture from deployment gating to module init.
- **(iii) FDR recover-readback before normal operation** — the declared home for item 48's
  recover-then-run sequence.
- **(iv) Sole minter of item 65's in-process capability tokens** — seL4's "init task holds all
  capabilities and delegates," sized to one process.

**Non-goals.** No new dependency; no async framework change; no touch to `money.rs`/`order_machine.rs`
transition logic / auth / RLS / orders (the root is *upstream* of all handlers and adds none). Does
not choose a service mesh, config format, or DI container — the DAG is plain Rust data (a static
slice of nodes + declared edges), not a framework.

## 2. Verified current state (grounded)

| Fact | Citation (live HEAD) |
|---|---|
| Durable store exists + is IO-safe (`Result`-typed insert, sync-before-index) | `kernel/src/hydra.rs:923` (struct), `:1032` (`impl EventStore`), `:1036` (`insert` → `Result<(),StoreError>`) |
| Durable store constructed only in test code | `hydra.rs:{1107,1123,1154,1175,1207}` (all > `mod file_store_tests` @ `:1085`) |
| Host binary #1 wires no store | `tools/native-spa-server/src/main.rs:48` (`ApiState::build_default()`) |
| Host binary #2 wires no store | `agent-loop/src/main.rs:16-34` (AgentLoop over adapter + fixture) |
| FSM graph-proof kit (topo-sort / cycle detection) | `kernel/src/order_machine.rs` — `has_cycle`/`topological_order`/`reachable` (item-7 target inventory §2) |
| Fail-closed adapter refusal precedent | `kernel/src/isolation/microvm.rs:61` (`can_accept_native_adapter`), `:76` (`register_adapter → Result<(),AdapterRejected>`), `:89` (unknown ⇒ refuse-by-default) |
| Delegation / attenuation machinery for token minting | `kernel/src/capability_cert.rs:208` (`SelfSignedRoot`), `:227` (`mint`), `:368` (`CertDelegation`), `:382` (`may_delegate`) |
| FDR read-only recovery (never truncates) | `kernel/src/fdr/ring.rs:227` (`recover`), `:65` (`crc32`) |
| Clean-stop marker record kind | `kernel/src/fdr/schema.rs:194` (`Kind::CleanShutdown`) |

## 3. Implementation plan (numbered)

The root lives in a NEW always-compiled kernel module (proposed `kernel/src/compose/mod.rs`) plus a
thin per-binary adapter. Nothing here is feature-gated: the composition root is the always-on floor.

1. **Declare the init DAG as plain data.** A `const`/`static` slice of `InitNode { id, requires:
   &[NodeId], provides: &[Capability] }`. Zero runtime graph mutation — the DAG is source data, like
   `FSM_ADJ` (`order_machine.rs:208`). Node ids are a closed enum with pinned discriminants (the
   `scope.rs` discipline), so a reorder/rename is a mechanically-caught diff.
2. **Derive the init order by topological sort over the DAG**, reusing `order_machine`'s
   `topological_order`/`has_cycle` *as-is* on the module-init adjacency. The derivation is a pure
   function `fn init_order(dag) -> Result<Vec<NodeId>, InitError::CyclicDependency(NodeId)>`. Order
   comes from the DAG, never from source order — a permuted declaration must yield the identical
   sequence (§5 acceptance).
3. **Construct the durable store in the derived order** (deliverable (i)): the root opens the
   `FileEventStore` at an operator-supplied path, wraps it in `EventLog` (`event_log.rs:288`), and
   surfaces `StoreError` fail-closed (never `let _ =`). This is the exact line the wiring-gap
   blueprint says is missing everywhere.
4. **Fail-closed capability check per node** (deliverable (ii)). Before a module's constructor runs,
   the root asserts every declared `requires` capability was `provides`-satisfied by an
   already-initialized upstream node; an unsatisfied requirement returns
   `InitError::CapabilityAbsent { node, capability }` and aborts startup. This is
   `microvm::register_adapter`'s refuse-by-default (`:89`) lifted from adapter-registration to
   module-init: absence is a typed startup error, not a silent None.
5. **Run the FDR recover-readback at the declared point** (deliverable (iii)): after the store node
   inits and before any handler node, call `fdr::ring::recover` (`ring.rs:227`, read-only — never
   truncates the crashed writer's segments) and record the outcome. On a dirty stop (no trailing
   `Kind::CleanShutdown`, `schema.rs:194`) the root emits the post-mortem record item 48 specifies.
   This is where item 48 "declared to live."
6. **Expose the token-minting seam** (deliverable (iv)) as a `pub(crate)` constructor visible ONLY
   inside `compose/` (visibility is the enforcement — see item 65 §3). The root is the one site that
   can construct item 65's zero-sized `CoreWriteCapability`; it delegates attenuated tokens downstream
   using the existing `capability_cert` attenuation model (`:227`/`:368`) rather than inventing a new
   memory-capability system.
7. **Adapt the two host binaries** to call `compose::boot()` instead of ad-hoc wiring:
   `native-spa-server/src/main.rs` and `agent-loop/src/main.rs` become thin shells that call the root
   and then serve/loop. `MemEventStore` remains the correct in-memory default for tests — the root is
   the *production* path only.

## 4. Required tests / proofs (CHECKLIST.md 5-point mapping)

The composition root is control-flow/data-structure code, not a secret-dependent-timing path, so the
5-point standard maps with honest N/As (each stated, none faked):

- **Item 1 (oracle).** The order-invariance oracle: two source-permuted declarations of the same DAG
  must produce byte-identical derived init sequences (property test over a corpus of permutations).
  Plus item 2's original proof condition, finally dischargeable: **a cited line in a production binary
  constructs the durable store** — a test asserting `compose::boot()` yields a live `EventLog<FileEventStore>`.
- **Item 5 (formal/graph proof).** The cyclic-init red→green: a planted cyclic `requires` declaration
  makes `init_order` return `InitError::CyclicDependency` at startup — reusing the same
  `has_cycle`-backed check that already guards the FSM (`order_machine.rs`), so the guarantee is the
  proven kit's, not a new one. Deleting the cycle → green.
- **Item 2 (dudect):** **N/A(not-a-timing-path)** — init runs once at boot with no secret input; a
  Welch-t harness has nothing to measure. Recorded, not faked.
- **Item 3 (debug cross-check).** `debug_assert!` that the derived order satisfies every edge
  (each node appears after all its `requires`) — a per-boot, zero-release-cost re-check of the
  topo-sort against the raw DAG.
- **Item 4 (asm spot-check):** **N/A(no-branch-free-crypto)** — the root contains no constant-time
  code; the arXiv:2410.13489 incident class does not apply.

Additional load-bearing proofs (from the roadmap's own §K proof clause, made concrete):

- A module declaring a capability no upstream node `provides` **refuses init fail-closed** (test asserts
  `InitError::CapabilityAbsent`) — red→green.
- **kill-9 recovery test still green through the new root**: crash mid-append, boot via `compose::boot()`,
  assert the FDR recover-readback runs and the durable chain verifies (`verify_chain`,
  `event_log.rs:481`). The recovery guarantee must survive the rewiring, not be re-proven weaker.

## 5. Falsifiable acceptance criteria

1. There exists a production (`src/main.rs`, non-`#[cfg(test)]`) line constructing
   `FileEventStore`/`EventLog` — grep-verifiable, closing wiring-gap §VERDICT(a). **Falsifier:** the
   wiring-gap grep still finds only test-code sites → FAIL.
2. `init_order` on a planted cyclic DAG returns a typed `CyclicDependency` at startup; the same on the
   real DAG returns a total order. **Falsifier:** a cycle boots successfully → FAIL.
3. A permuted-declaration DAG yields the identical derived sequence (order from the DAG, not source).
   **Falsifier:** reordering the node declarations changes boot order → FAIL.
4. A module with an unsatisfied `requires` aborts boot with `CapabilityAbsent`. **Falsifier:** it
   silently inits with a missing capability → FAIL.
5. The kill-9 recovery test passes end-to-end through `compose::boot()`. **Falsifier:** recovery
   regresses under the new root → FAIL.
6. Zero new external crates: `cd kernel && cargo tree -e no-dev` byte-unchanged. **Falsifier:** any
   dependency added → FAIL.

## 6. Dependency gates (honest)

- **Ready to dispatch now.** Every upstream primitive is landed and cited live (§2). No blueprint
  prerequisite; the roadmap classes it "dispatchable now" (line 1104).
- **Blocks item 65** (its sole token-minter) and **item 66** (scrubbing an unwired store is
  pointless). Both are inert until this lands.
- **Interlocks with item 48** (FDR recover-readback place) — item 48 is spec-level in the roadmap
  (no blueprint yet); the *mechanism* it needs (`fdr::ring::recover`) is already live, so §3 step 5
  can be built against the real recovery function and item 48's higher-level policy layered later.

## 7. Operator-decision points (flagged)

- **Which binary owns the root, and the store path.** The wiring-gap blueprint explicitly declines to
  choose ("does not choose which binary owns it"). Both host binaries are candidates; a shared
  `compose::boot(config)` called by each is the proposed neutral answer, but the *path* and *whether a
  headless daemon should own a single canonical durable log* is an operator ruling, not an engineering
  default. **Recommend:** operator names the durable-store owner + path before deliverable (i) lands.
- **Node-id discriminant allocation.** Following `scope.rs`, init-node ids get pinned bytes; if the
  §L governance arc later wants the DAG machine-readable in `HOT-PATHS.tsv` idiom (item 74), the id
  space should be reserved jointly. Flagged, not resolved here.
