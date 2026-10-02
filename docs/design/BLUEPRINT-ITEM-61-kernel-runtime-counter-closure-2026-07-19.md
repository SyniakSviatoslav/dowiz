# BLUEPRINT — Item 61: Kernel Runtime-Counter Closure (durability · subprocess · eigensolver · crypto spans; gaps G5 + G6 + G7 + G8)

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** This is a July-2026 planning document. The architecture it plans (per-node rusqlite, dtn7/BPv7, a QUIC bearer, pgrust, Astro, the native SPA server, mesh crates as a runtime) has **no code on `main` at `9ace8f28`**: `rusqlite` appears only in `tools/deep-clean`; `dtn7`, `bp7`, `quinn`, `pgrust`, `wgpu` have 0 hits under `workers/` and `crates/`; `tools/native-spa-server` was deleted on 2026-10-01 (`b1865597`). **It is history, not status, and it is kept unchanged below this banner — not archived, not deleted (operator 2026-10-02).** What the product became and what is next: `docs/design/ROADMAP-2026-09-22.md` (entry point; its NEXT section is dated 2026-10-02). Where this file's themes went: the event store/log → the hub's append-only log, phases 1-7 of `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md` (SHIPPED) and the bebop DAG store (DG1-DG10, LANDED); the money law and the order FSM → `crates/dowiz-core/src/order_machine.rs` + the conservation audit (8 laws, live); telemetry → DW6/W-TELEM (done, unmerged) and BN8; PQ/crypto → ML-KEM-768 ACVP (`8ae71778`), F15-F17 (Wave N3-A), the seal waits for the operator's keypair (OA-3); mesh/DTN/P2P → refused with measured numbers (`docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` §7.3); local AI/LLM → Wave L decisions (no cloud voice), the VOICE research row; GPU/WebGPU/tensors → withdrawn by the operator (C-1, `7cdcf0ab`); Kani/TLA+/Lean → Lean as a cross-check only (DG6 `01783eb4`, `sorryAx` 0); product UI → Phases A-C of the live roadmap, all LANDED; CI/gates → the 28 gates under `tools/gates/` run by `run-all`; MCP/agents → P14-P20 (Waves N2-B, N4.1). The 13 `crates/dowiz-core` modules of this era that nothing calls are **kept** and get a caller or a header under row N4.2 (never a deletion).


- **Date:** 2026-07-19 · **Tier:** code (roadmap §K, item 61) · **Status:** BLUEPRINT (planning
  artifact, no code).
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §K item 61
  (lines 1048–1063); `AUDIT-TELEMETRY-EVERYWHERE-AI-OPTIONAL-OS-2026-07-19.md` (gaps G5/G6/G7/G8);
  item 58 blueprint (`BLUEPRINT-ITEMS-57-58-…`); `PROCEDURE-TELEMETRY-COMPLETENESS-STANDING-2026-07-19.md`;
  ground-truth code: `kernel/src/event_log.rs`, `kernel/src/living_knowledge.rs`,
  `kernel/src/spectral.rs`, `kernel/src/householder.rs`, `kernel/src/span_metrics/instrument.rs`,
  `kernel/src/fdr/{mod.rs,pmu.rs,ring.rs}`, `docs/audits/hardening/HOT-PATHS.tsv`.
- **Prerequisites:** **item 58** (`(work, cost)` pair + the `FdrRecordsAppended` /
  `EigensolvesCompleted` / `SignaturesVerified` workload-kinds). Peer of items 59/60 (the three
  consumers of item 58).

---

## 1. Scope & goal

**Goal.** Four kernel hot surfaces run "dark" — they do measurable work with **no ongoing telemetry
feed**. Close all four with runtime spans/counters emitting item-58 `(work, cost)` pairs, so
operator-gated decisions (group-commit, eigensolver choice, crypto latency) have a *live* data feed
instead of a one-time bench number.

**Non-goals.**
- NOT a change to any decision/money/FSM logic (spans wrap; they never alter behavior — the
  `span_metrics/instrument.rs` precedent forwards 1:1).
- NOT a new dependency (`wait4`/rusage via a raw syscall like `pmu.rs`'s `perf_event_open`; spans via
  `fdr::info_span!`; no `libc`).
- NOT making any telemetry value a decision input (P3 firewall).

## 2. Current-state grounding (four dark surfaces)

### 2a. Durability counters (gap G5)

- `kernel/src/event_log.rs:308–327` — `EventLog::append` chains `prev`, dedups, and
  `store.insert(id, ev)?` then `set_tip`. `:194,235` — `EventStore::insert` (the `FileEventStore`
  fsync-before-claim path). **No continuous counters** (events / Δticks / fsync count) are kept.
- item 26 measured `event-log append ~637 µs p50` **once, at bench time**; the operator-gated **53×
  group-commit decision has NO ongoing data feed** — the number is frozen, not live.

### 2b. Subprocess spawns (gap G6)

- `kernel/src/living_knowledge.rs:88–95` — `Command::new("sh").arg("-c").arg(&self.bridge_cmd)
  .spawn()` spawns a child. `:107–109` — `child.wait_with_output()` waits, but captures **no
  duration and no rusage** (`wait_with_output` gives status+stdio, not `getrusage`). A hung or
  expensive child is **currently invisible to FDR** (adjacent to item 48's liveness class).

### 2c. Eigensolver spans (gap G7)

- `kernel/src/spectral.rs` and `kernel/src/householder.rs` are **HOT-PATHS `@ZONE`s**
  (`HOT-PATHS.tsv:30,29`) with oracle/dbg coverage — but **no runtime spans**. `spectral.rs`'s
  iterative QR / `householder.rs`'s reductions do the heaviest deterministic float work in the kernel;
  **cycles/eigensolve is the cleanest Tier-C efficiency metric available** (a fixed-ish schedule over
  a known matrix size).

### 2d. Crypto-span double-gating (gap G8)

- `kernel/src/span_metrics/instrument.rs:83–94` — the single `mldsa_verify` span wrapper is
  `#[cfg(all(feature = "telemetry", feature = "pq"))]`. So a **`pq`-only production build** (crypto
  on, telemetry off) has **zero crypto-latency telemetry** — a silent dark zone on the signature path.
  (Compare `cap_verify_chain` at `:71–81`, gated on `telemetry` alone; the crypto one is
  double-gated.)

## 3. Implementation plan (numbered)

1. **(a) Durability continuous counters.** Add running counters to the `EventLog`/`FileEventStore`
   append path: events appended, Δticks, and fsync count (the `FileEventStore` already issues
   `sync_all`; count the calls). Emit them as item-58 `(work: FdrRecordsAppended / events, cost:
   Δticks ⊕ fsync-count)` pairs on a `SpanClose` FDR record — recoverable from the FDR ring after N
   appends. This gives the group-commit decision a **live feed** replacing the frozen 637 µs bench.
   The counter increments are P3 (they never gate the append's durability barrier at
   `event_log.rs:322–326`).
2. **(b) Subprocess duration + rusage + FDR record.** Replace/augment `wait_with_output`
   (`living_knowledge.rs:107`) so the child is reaped via **`wait4(2)`** (a raw syscall in the exact
   zero-dep style as `pmu.rs`'s hand-rolled `perf_event_open`, `pmu.rs:251–326`) capturing `rusage`
   (user/sys time, maxrss), plus wall-duration via the wasm-safe clock, plus an FDR record. A
   hung/expensive child becomes **observable** (composes with item 48's liveness class — the FDR
   record is the evidence a child ran long). Fail-closed behavior is unchanged (`living_knowledge`
   still returns `Err` on any spawn/IO error, `:11`).
3. **(c) Eigensolver spans.** Add `fdr::info_span!` spans to the `spectral.rs` / `householder.rs`
   eigensolve entry points, workload-kind `EigensolvesCompleted`. The `SpanClose` record carries the
   item-58 `(work: EigensolvesCompleted Δcount, cost: Δcycles ⊕ Δticks)` pair — **cycles/eigensolve
   is the cleanest Tier-C metric** (PMU cycles available under CAP_PERFMON on this host,
   `pmu.rs:395–410`). Fill the `HOT-PATHS.tsv` `eff` cells for these zones (item 57 requirement).
4. **(d) Fix the crypto-span double-gating.** Two acceptable resolutions (executor picks, ledgers the
   choice): **(i)** make the `mldsa_verify` span compile under **`pq` alone** (drop the `telemetry`
   co-gate, keeping the *emission* behind a runtime sink-installed check so a `pq`-only build still
   pays only the cheap `SINK_ACTIVE` load) — closing the dark zone; OR **(ii)** if the span must stay
   double-gated for a stated cost reason, add an explicit **`gap:` row in `HOT-PATHS.tsv`** naming the
   pq-only-no-crypto-telemetry dark zone (no silent dark zone — item 57's zero-un-named-blind-spots
   law). Workload-kind `SignaturesVerified`.
5. **Wasm/plane discipline.** All spans use `fdr::info_span!` (already wasm-gated and no-op when no
   sink is installed, `fdr/mod.rs:216–251`); the `wait4` syscall (step 2) is native-only (subprocess
   spawning is not a wasm capability — a named absence on wasm per procedure step 9). Every emitted
   value is P3 (grep-firewall proof).

## 4. Required tests / proofs (CHECKLIST.md 5-point standard)

| Checklist item | Disposition for item 61 |
|---|---|
| 1. **Oracle** | **(a)** counters recoverable from the FDR ring after N appends in a test (the `fdr/ring.rs` recovery path is the readback oracle). **(b)** the child-process record carries **real rusage** (a **planted slow child** is observable — red→green). **(c)** eigensolver spans emit under load with the HOT-PATHS `eff` rows filled. **(d)** the pq-only build **either emits crypto spans or carries the ledgered `gap:` row** (gate-checked by the extended `hardening-gate`). |
| 2. **Dudect** | **N/A for spans / EXISTING for `mldsa_verify`.** The span wrapper forwards 1:1 to `pq::dsa::verify` (`instrument.rs:92–93`) — it introduces **no secret-dependent branch**. The crypto CT property remains `pq/dsa`'s own concern (the `KNOWN-RED(P91.2)` tag-compare ledger, unchanged). Item 61 must NOT let the span's timing become a side-channel: the span brackets, it does not branch on the verify result mid-computation. Record this explicitly. |
| 3. **Debug cross-check** | **N/A(measured-spans)** for the new counters; the existing `spectral`/`householder` oracle+dbg coverage (`HOT-PATHS.tsv:44` Vieta cross-check) is **untouched**. |
| 4. **ASM spot-check** | **N/A** — the spans add no branch-free arithmetic; the crypto path's own asm spot-check obligation is unchanged. |
| 5. **Kani/formal** | **N/A** — the property is "the counter/record is recoverable and pairs are lossless," an oracle concern. |

**Plane-firewall proof (procedure step 7, mandatory):** grep proof that none of the four new
counters/spans feeds any hash/gate/replay/decision surface — in particular the durability counters
(step 1) never gate the `store.insert(id, ev)?` durability barrier (`event_log.rs:322–326`), and the
crypto span never influences the verify verdict.

**Subprocess-liveness composition:** the step-2 record composes with item 48's liveness class — a
planted slow child produces an FDR record with a large rusage/duration; assert the record exists and
carries the real numbers (never a fabricated `0`).

## 5. Falsifiable acceptance criteria

1. After N appends, the durability counters (events / Δticks / fsync) are **recoverable from the FDR
   ring** in a test; the group-commit decision now has a live feed (not the frozen 637 µs).
2. A planted slow child yields an FDR record carrying **real `wait4` rusage + duration** (red→green:
   today `wait_with_output` captures neither).
3. Eigensolver spans emit under load; the `spectral`/`householder` HOT-PATHS `eff` cells are filled
   with `EigensolvesCompleted`.
4. The pq-only (no-telemetry) build **either** emits `mldsa_verify` crypto spans **or** carries a
   ledgered `gap:` row in `HOT-PATHS.tsv` — the gate confirms one of the two (no silent dark zone).
5. Grep-firewall proof green: no new counter/span value reaches a decision/durability-barrier/hash
   surface.
6. `cargo tree -e no-dev` byte-unchanged (`wait4` via raw syscall, spans via `fdr` — zero new dep).

**Falsifier:** a counter gating the durability barrier; a fabricated `0` rusage on a real child; a
pq-only dark crypto zone with neither span nor `gap:` row; a new dependency.

## 6. Dependency gates

- **Upstream:** **item 58** (the `(work, cost)` pair + the three workload-kinds
  `FdrRecordsAppended`/`EigensolvesCompleted`/`SignaturesVerified`). Transitively item 57 (procedure)
  → the FDR merge. All four surfaces write item 58's schema.
- **Compose-with:** **item 48** (subprocess liveness — step 2's FDR record is item 48's evidence
  surface; they compose, item 61 does not gate on 48). **Item 57** — step 3/4 fill/ledger `eff` cells
  the item-57 gate then enforces.
- **Peer:** items **59** (agent turns) and **60** (engine frames) — the three item-58 consumers; no
  ordering between them, all after 58.
- **Downstream:** none — leaf consumer.

## 7. Operator-decision points & accepted risks

- **[OPERATOR] Crypto-span gating choice (d).** Whether `mldsa_verify` timing compiles under `pq`
  alone (closing the dark zone in production crypto builds) or stays double-gated with a ledgered
  `gap:` row is a real cost-vs-visibility tradeoff on a red-line (crypto) path. Recommended: compile
  under `pq` with the emission behind the cheap runtime sink check (visibility with near-zero cost).
  Flagged because it touches the signature-verify hot path in production builds. **Owner:** operator.
- **[OPERATOR] Group-commit decision.** The live durability feed (step 1) exists to *inform* the
  operator-gated 53× group-commit decision — item 61 provides the data, it does NOT make the decision
  (that stays operator-gated per the existing ledger). **Owner:** operator.
- **[ACCEPTED] `wait4` raw syscall.** Reaping the child via a hand-rolled `wait4` (zero-dep, the
  `perf_event_open` precedent) is more code than `wait_with_output` but keeps the empty-allowlist
  gate green. Native-only; a named absence on wasm (subprocess is not a wasm capability). **Owner:**
  executor.
- **[ACCEPTED] Span-timing-not-a-side-channel.** The `mldsa_verify` span brackets the whole verify;
  it does not branch on secret data mid-call, so it adds no timing channel. Recorded explicitly
  because it touches crypto. **Owner:** arc lead.
