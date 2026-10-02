# BLUEPRINT — Item 2: `FileEventStore` wiring verification (2026-07-19)

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** This is a July-2026 planning document. The architecture it plans (per-node rusqlite, dtn7/BPv7, a QUIC bearer, pgrust, Astro, the native SPA server, mesh crates as a runtime) has **no code on `main` at `9ace8f28`**: `rusqlite` appears only in `tools/deep-clean`; `dtn7`, `bp7`, `quinn`, `pgrust`, `wgpu` have 0 hits under `workers/` and `crates/`; `tools/native-spa-server` was deleted on 2026-10-01 (`b1865597`). **It is history, not status, and it is kept unchanged below this banner — not archived, not deleted (operator 2026-10-02).** What the product became and what is next: `docs/design/ROADMAP-2026-09-22.md` (entry point; its NEXT section is dated 2026-10-02). Where this file's themes went: the event store/log → the hub's append-only log, phases 1-7 of `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md` (SHIPPED) and the bebop DAG store (DG1-DG10, LANDED); the money law and the order FSM → `crates/dowiz-core/src/order_machine.rs` + the conservation audit (8 laws, live); telemetry → DW6/W-TELEM (done, unmerged) and BN8; PQ/crypto → ML-KEM-768 ACVP (`8ae71778`), F15-F17 (Wave N3-A), the seal waits for the operator's keypair (OA-3); mesh/DTN/P2P → refused with measured numbers (`docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` §7.3); local AI/LLM → Wave L decisions (no cloud voice), the VOICE research row; GPU/WebGPU/tensors → withdrawn by the operator (C-1, `7cdcf0ab`); Kani/TLA+/Lean → Lean as a cross-check only (DG6 `01783eb4`, `sorryAx` 0); product UI → Phases A-C of the live roadmap, all LANDED; CI/gates → the 28 gates under `tools/gates/` run by `run-all`; MCP/agents → P14-P20 (Waves N2-B, N4.1). The 13 `crates/dowiz-core` modules of this era that nothing calls are **kept** and get a caller or a header under row N4.2 (never a deletion).


> Roadmap: `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §A Item 2. Original finding:
> `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §1.4/§9.2/§10-P4 ("a wired store that
> swallows IO is arguably worse than an unwired one"). Tier 0, read-only verification.
> **Status: blueprint authoring already ran both checks against live HEAD (`e10ea4e54`).
> Sub-question (b) PASSES; sub-question (a) FAILS. Executor's job = re-confirm + file the defect.**

## 1. Ground truth — exact locations at HEAD

| What | Where |
|---|---|
| `EventStore` trait | `kernel/src/event_log.rs:182` |
| `Result`-typed trait `insert` | `kernel/src/event_log.rs:188` — `fn insert(&mut self, id: [u8; 32], ev: MeshEvent) -> Result<(), StoreError>;` |
| `StoreError` enum | `kernel/src/event_log.rs:167` |
| `MemEventStore` (non-durable default) | `kernel/src/event_log.rs:209` |
| `FileEventStore` struct (name unchanged, NOT renamed) | `kernel/src/hydra.rs:923` |
| `FileEventStore::open` | `kernel/src/hydra.rs:934` |
| `FileEventStore` `insert` impl | `kernel/src/hydra.rs:1036` (inside `impl EventStore for FileEventStore`, :1032) |
| Typed IO-error propagation (`StoreError::Open/Write/Flush/Sync`) | `kernel/src/hydra.rs:1051-1060` |
| Regression test proving no-swallow | `kernel/src/hydra.rs:1188-1218` (`file_store_tests`, asserts `Err(StoreError::Open(_))`) |

Fix landed in commit `4dec04218` "fix(hermetic): H1 event-log fail-open fix". Index-advance
happens only after `sync_all` succeeds (H1 §2.2 ordering comment, hydra.rs:1062-1064).

## 2. PASS / FAIL criteria

**(b) Result-typed insert — PASS (verified this session).**
- PASS = trait signature at `event_log.rs:188` returns `Result<(), StoreError>` AND the
  `FileEventStore` impl at `hydra.rs:1036` maps each of open/write/flush/sync through `?` with a
  typed `StoreError` variant (no `let _ =` on any IO call in the insert body).
- FAIL = any `let _ =` or ignored `Result` on an IO step inside `hydra.rs:1036-1070`, or an
  `insert` returning `()`.

**(a) Production wiring — FAIL (verified this session).**
- PASS would be = a `FileEventStore::open(...)` call in NON-test code reachable from a real
  composition root (a `main()`, a hub binary, an adapter's production constructor).
- FAIL (current state) = every `FileEventStore::open` site is test code:
  `agent-adapters/tests/e2e_admission.rs:92`, and `kernel/src/hydra.rs:1107/1123/1154/1175/1207`
  — all inside `#[cfg(test)]` mods (`mod tests` :405, `mod file_store_tests` :1085).
  The only NON-test store construction in the repo is `MemEventStore`:
  `kernel/src/hub_supervisor.rs:366` (`MemStateStore::new`, production code — its `#[cfg(test)]`
  starts at :710). No binary crate (`agent-loop`, `tools/*`, `agent-adapters/src`) constructs any
  `EventLog` at all — they import only `sha3_256` from `event_log`.
- The 2026-07-19 WIRING WAVE (`e12a5e323` B4 = `BackupOrgan::snapshot_and_restore_local`; P-H =
  `verify_chain_before_trust`) is **adjacent, not this**: it wired backup/verify surfaces, not a
  durable-store construction. Item 2(a) was NOT resolved by the wave.

## 3. Executor commands

```sh
# (b) trait + impl signatures and no-swallow body
grep -n "fn insert" kernel/src/event_log.rs kernel/src/hydra.rs
sed -n '1036,1070p' kernel/src/hydra.rs          # expect four map_err(StoreError::...) + ?
grep -n "let _ =" kernel/src/hydra.rs | awk -F: '$1>1032 && $1<1084'   # expect empty

# (a) every construction site, then subtract test code
grep -rn "FileEventStore::open" --include="*.rs" . | grep -v target | grep -v ".worktrees"
# classify each hit: file under tests/ OR line inside a #[cfg(test)] mod ⇒ not production
grep -n "#\[cfg(test)\]" kernel/src/hydra.rs      # test mods at 404 and 1084

# regression test for (b)
cargo test -p dowiz-kernel file_store   # includes the Err(StoreError::Open) no-swallow test
```

## 4. Gotchas

- **Three store types, don't conflate**: `MemEventStore` (event_log.rs:209, non-durable default),
  `FileEventStore` (hydra.rs:923, durable), and `FaultyStore` (hydra.rs test-only injector,
  :766 context). `ChaosStore` (chaos.rs:292) wraps `MemEventStore` — chaos harness, not durable.
- Naive `grep -v test` misses `#[cfg(test)]` mods inside `src/*.rs` — hydra.rs's own
  `FileEventStore::open` hits look like src code by path but are all under test mods. Check line
  numbers against the mod boundaries (:405, :1085).
- There is currently **no hub binary at all** that builds a `Hydra`/`EventLog` — the kernel is a
  library and no composition root exists yet. So the precise defect shape is "no durable
  composition root exists", not "a root exists and picks Mem". §235 of the synthesis already
  rules where the fix legitimately lives: the composition root (`main()`/config layer).
- `agent-adapters/tests/e2e_admission.rs` uses the REAL `FileEventStore` — good coverage, but a
  test binary is not production wiring; don't let it satisfy (a).

## 5. Handoff — on confirming the (a) defect

1. File `docs/design/BLUEPRINT-P##-durable-event-store-composition-root-2026-07-19.md` (pick the
   next free P## from `CORE-ROADMAP-INDEX.md`) scoping: which binary becomes the composition
   root, config for the log path, and `MemEventStore`-stays-default-for-tests.
2. Add one row to `CORE-ROADMAP-INDEX.md` per its own maintenance rule (line 213: "new planning
   doc ⇒ one new row here") — for the defect doc AND for this blueprint (this file has no row
   yet; it was deliberately not added mid-session to avoid colliding with today's concurrent
   writers on that hot file).
3. Note in the defect doc that (b) is already closed by `4dec04218` — the defect is wiring-only,
   which shrinks it to Tier-1-sized work (roadmap §B territory, near Item 9's breaker entry note
   "best entered after item 2's finding").
