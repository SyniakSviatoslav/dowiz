# Research / Grounding — Crash-Consistency, Formal Verification, Guardian Pattern, Hard Real-Time Determinism

**Date:** 2026-07-19 · **Role:** grounding/investigation ONLY (Opus). Establishes ground truth against
real code for the 11 questions in the RAW-PROMPT-5 arc. **No synthesis, no blueprint, no recommendation,
no roadmap** — a later Fable pass consumes this. Every claim is tagged **GROUNDED** (verified against live
source this pass) or **NOT FOUND / DOES NOT EXIST**.

**Source material read first:** `docs/design/RAW-PROMPT-5-crash-consistency-formal-verification-fail-fast-guardian-2026-07-19.md`
(verbatim), `docs/design/DETERMINISTIC-AI-INFERENCE-SYNTHESIS-2026-07-19.md`, and
`docs/design/SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` items 33–44 (committed `191f509b6`).

**Critical location fact (GROUNDED):** the FDR module does **not** exist on `main`. `main`'s
`kernel/src/fdr/` is absent (`ls` errors). The FDR module lives ONLY in the isolated worktree
`/root/dowiz-wt-space-grade-exec`, branch `exec/space-grade-tier0-2026-07-19`, at
`kernel/src/fdr/{ring,mod,schema,pmu,json,macros}.rs`. That worktree also carries `kernel/src/ct_gate.rs`,
absent from `main`. All FDR/ct_gate citations below are into that worktree path. All other kernel citations
are into `/root/dowiz` `main`.

---

## 1. Why the kill-9 FDR test achieved zero-torn-tail without fsync — exact mechanism

**File:** `/root/dowiz-wt-space-grade-exec/kernel/src/fdr/ring.rs` (read in full).

- **A/B segment layout (GROUNDED).** Two alternating fixed-cap append-only segment files,
  `fdr.a.jsonl` / `fdr.b.jsonl` (`SEG_A`/`SEG_B`, ring.rs:35-36), each capped at `DEFAULT_SEG_CAP = 1 MiB`
  (ring.rs:33). One segment is active at a time (`FdrRing.active: 0=>A, 1=>B`, ring.rs:77-84). At the cap,
  `switch()` fsyncs the current segment, then `create+truncate`s the OTHER and makes it active
  (ring.rs:129-131, 141-152). The A/B pair is a **bounded-size retention ring** (last-N-seconds), NOT a
  shadow-paging state pair.
- **Record framing (GROUNDED).** Each append writes `line = <payload>|<crc:08x>\n` (ring.rs:118-128).
  Payload is `FdrEvent::to_json()`; `'\n'` is escaped inside the JSON writer so it unambiguously delimits
  records (ring.rs:121-122).
- **CRC32 usage (GROUNDED).** Hand-rolled CRC32 (IEEE 802.3, reflected, `0xFFFFFFFF` init/final-xor),
  table-on-first-use via `OnceLock` (ring.rs:41-72). Known-vector test pins `crc32("123456789") ==
  0xCBF43926` (ring.rs:334-337). The CRC is appended to every line and is the sole read-time validity gate.
- **Why torn/partial writes are detectable + skippable on read (GROUNDED).** `recover()` (ring.rs:230-284,
  **READ-ONLY**, never truncates) reads both segments, splits on `'\n'`, and:
  - the element AFTER the final `'\n'` that is non-empty ⇒ a process died mid-write ⇒ **torn tail**,
    counted and dropped (ring.rs:242-256);
  - a complete line's trailing `|<crc>` is re-verified `crc32(payload) == want`; mismatch ⇒ `crc_failures++`
    and drop (ring.rs:257-278). Recovered records are sorted by `seq` (ring.rs:281).
  Tests pin both behaviors: `torn_tail_line_is_dropped_valid_records_survive` (ring.rs:382-403) and
  `crc_corruption_is_dropped` (ring.rs:405-423).
- **Why NO fsync suffices for kill-9 (GROUNDED).** The module doc states it precisely (ring.rs:13-21):
  surviving `kill -9` (process death) needs ONLY that `write(2)` reached the OS page cache — a fresh reader
  of the same file sees those bytes because the page cache outlives the process. `append()` writes without
  fsync; only `Kind::Alarm | Kind::PostMortem` records call `sync_data()` (ring.rs:134-136), and `switch()`
  fsyncs on rotation (ring.rs:142). Power-loss (not kill-9) is what fsync is for — explicitly separated in
  the doc.
- **Classification (GROUNDED): this is a Sequential Append-only Log, NOT an Atomic Pointer Swap, and not a
  hybrid.** The module doc says so directly: "two alternating fixed-cap append-only segment files … 
  Append-only segments are simpler to prove correct under torn writes than an in-place byte-ring cursor"
  (ring.rs:6-11). There is **no pointer that is updated only after CRC confirmation** — recovery reads BOTH
  whole segments and merges by `seq`; correctness comes from per-record CRC + monotone `seq`, not from an
  atomically-swapped active-state pointer. The doc also explicitly **corrects the earlier synthesis's
  "mmap-backed ring — pure std" description** as wrong (`std` has no mmap; that would need a new dep)
  (ring.rs:4-8).

---

## 2. Does an LSM-style "WAL + periodic snapshot" already exist anywhere in dowiz?

**Answer: NOT FOUND as such** — no WAL-plus-periodic-state-snapshot mechanism exists whose purpose is to
shorten crash-recovery replay. The adjacent pieces are real but serve different purposes:

- **`kernel/src/event_log.rs` (GROUNDED): pure append-only content-addressed hash-chain log, NO snapshot.**
  Event id = `sha3_256(prev, actor_pubkey, actor_seq, payload)` making replays idempotent
  (event_log.rs:1-8). A grep for `snapshot|checkpoint` returns nothing; only `replay`/`fold` references
  (event_log.rs:159, 325, 356-376). There is no periodic snapshot to skip replay from.
- **`kernel/src/hub_supervisor.rs` M4 (GROUNDED): a snapshot mechanism paired with the log, but for UPDATE
  ROLLBACK, not replay-speedup.** `StateSnapshot`/`EpochHash` (hub_supervisor.rs:326-340): the "snapshot"
  IS the event-log chain-tip content-id (`epoch`), `restore` re-points the tip and re-folds the projection
  (`StateStore::snapshot/restore/chain_tip`, hub_supervisor.rs:342-412). Purpose (module doc + M7 tests):
  capture the pre-promote epoch so a crash-looping release can roll back code AND state
  (hub_supervisor.rs:320-324, 1171-1244). It is a **log-position pointer**, not a materialized state dump,
  and it exists to make deployment rollback safe — not to bound startup replay time.
- **`kernel/src/hydra.rs` "anti-tamper checkpoint" (GROUNDED, different meaning).** `integrity_check()` is
  documented "G9 — anti-tamper checkpoint" (hydra.rs:211) but it re-derives the baseline spectral radius to
  detect tampering (hydra.rs:218-227) — "checkpoint" here = integrity probe, not a state snapshot.
- **FDR itself (GROUNDED): no snapshot.** `fdr/ring.rs` has A/B append-only segments + full replay on
  recovery only; no state-snapshot construct exists in `fdr/`.

So the raw prompt's "Hybrid/LSM-style (WAL + periodic snapshot)" **DOES NOT EXIST**. The nearest real
adjacency is `event_log` (append-only log) + `hub_supervisor`'s epoch-pointer snapshot (rollback), which is
a different composition for a different goal.

---

## 3. Formal-verification tooling precedent

- **Kani — NOT FOUND (not used anywhere).** No `#[kani::proof]`, no `kani::` call site anywhere in `main`
  or in the worktree (`grep` clean in both `kernel/src` trees). No `kani-verifier` dev-dependency in any
  `Cargo.toml` and no `kani` token in `Cargo.lock`. Kani appears only as **planned/aspirational text** in
  the roadmap ("Kani interleaving proof already scoped in item 8", ROADMAP §0 line 18; "Kani" in items 8).
  Actual usage: DOES NOT EXIST.
- **proptest — GROUNDED, real but narrow.** Real dev-dependency: `kernel/Cargo.toml:161` (`proptest =
  "1.11"`). Used in exactly two files: `kernel/src/ports/payment.rs:636-662`
  (`b3_reconciliation_folded_eq_fold_derived`, 400 cases) and `kernel/src/ports/payment_provider.rs:1120-1128`
  (400 cases). **The dominant kernel test pattern is NOT proptest — it is hand-rolled deterministic
  oracle/differential/exhaustive corpora**, e.g. the TS "legacy oracle" for the order FSM
  (`order_machine.rs:3-4`, `green_terminal_set_matches_oracle` :764, power-iteration `spectral_radius_oracle`
  :981-1031), MST hand-oracle (`dsu.rs:200`), hand-derived stats oracle (`stats.rs:8,191,211`), the P77
  differential harness (`spool.rs:287`), and the FDR tests themselves (`fdr/ring.rs:318-447`, fixed
  vectors + a simulated torn write, no proptest). The kill-9 durability work this session used hand-rolled
  deterministic tests, not property-based `proptest`.
- **TLA+ — NOT FOUND.** No `.tla` files anywhere; no `docs/design/*TLA*` files. TLA+/TLC appears only as
  **planned** roadmap items (item 10 "TLA+ spec of decision-import + order FSM", ROADMAP:250; item 11
  ARINC-653 "design doc + TLC model", ROADMAP:21,256). Actual artifacts: DO NOT EXIST.
- **dudect/Welch-t — GROUNDED (worktree only).** `/root/dowiz-wt-space-grade-exec/kernel/src/ct_gate.rs` is
  a real zero-dep dudect-style constant-time gate (Welch's t-test, |t| < 4.5 cutoff, ships with a
  planted-leak self-test), ported std-only (ct_gate.rs:1-20). It is on the exec worktree branch, not on
  `main`.

---

## 4. seL4-style capability-based memory isolation

- **`kernel/src/capability_cert.rs` — the item-30 characterization is accurate for one part, but the file
  as a whole is a DIFFERENT "capability."** GROUNDED: `RotationState` IS a 2-variant enum
  (`Stable{suite}` / `Overlapping{old,new,overlap_until}`, capability_cert.rs:541-551) with a
  time-windowed `accepts(suite, now)` predicate (capability_cert.rs:553-572). But that is only the
  crypto-suite-rotation sub-part. **The module overall is a biscuit/UCAN-style capability-CERT chain**:
  hybrid ML-DSA-65 ⊕ Ed25519 signed delegation with scope attenuation, self-signed roots, revocation, and
  `verify_chain_hybrid` (capability_cert.rs:1-33 doc, `HybridSig` :120, `SelfSignedRoot` :207,
  `CertDelegation` :367, `verify_chain_hybrid` :776-867). So "capability" here means the
  **object-capability / authority-token sense** (a signed, delegable, scope-attenuated grant of *authority*,
  UCAN/biscuit lineage) — NOT the seL4 **memory-access-token** sense. The two terms are genuinely distinct,
  as the question suspected: this file governs *who may act on what*, not *which memory a module may touch*.
- **In-kernel memory/module isolation boundary — NOT FOUND (no seL4-style per-module memory isolation).**
  The kernel is a single Rust process; inter-module isolation is Rust's compile-time type/borrow system, not
  a runtime memory boundary. What IS real:
  - **`kernel/src/isolation/microvm.rs` (GROUNDED, but a deployment gate, not in-kernel memory isolation).**
    "Isolation tiering — host-capability probes and adapter registration gates … a node without KVM refuses
    `native-process` adapters instead of running them unsandboxed" (`isolation/mod.rs:1-8`). This decides
    whether EXTERNAL microVM/process sandboxing is available and fails closed; it does not isolate one
    kernel module's memory from another.
  - **WASM sandbox boundary (GROUNDED).** The kernel compiles to `wasm32` (widespread
    `#[cfg(target_arch = "wasm32")]` gating; `kernel/src/wasm.rs`, `lib.rs`, `arena.rs`). A wasm linear-memory
    sandbox is a real memory boundary — but it isolates the WHOLE kernel from its host, not one module from
    another inside the kernel.
  - **Feature-gate compile boundaries (GROUNDED).** `pq`, `gpu`, `pgrust`, `slot-arena` are compile-time
    inclusion/exclusion (e.g. `hub_supervisor.rs` gated behind `pq`; `slot_arena.rs:1-8` behind
    `slot-arena`). These are build-time surface control, not runtime fault isolation.
  No MMU/page-capability boundary preventing one module's fault from corrupting another's memory exists.

---

## 5. Erlang/OTP "supervisor tree" precedent

**File:** `kernel/src/hub_supervisor.rs` (read in full). **Confirmed accurate to the item-30 audit and NOT
an OTP supervisor tree.**

- GROUNDED: `UpdateState` enum (hub_supervisor.rs:441-476) is an **A/B-slot atomic-flip software-UPDATE
  state machine** (`Idle → Fetched → Migrated → SnapshotTaken → HealthPassed → Promoted / RolledBack /
  Failed`). `decide_promote` / `decide_rollback` are pure decision fns (hub_supervisor.rs:536-573);
  `drive_promote` / `drive_restore` are the kernel-local drivers (hub_supervisor.rs:616-708).
- GROUNDED: it DOES implement a crash-triggered restart/rollback, but at **release-deployment granularity,
  not process/actor granularity.** `CRASH_LOOP_MAX_RESTARTS = 3` (hub_supervisor.rs:422),
  `RollbackTrigger::CrashLoop` (hub_supervisor.rs:479-480), and the M7 test
  `rollback_after_schema_migration_restores_state_and_code` (hub_supervisor.rs:1173-1244) show a
  crash-looping NEW RELEASE auto-rolling-back BOTH code (slot flip) and state (snapshot restore) to the
  prior version. This is "detect a bad deploy crash-looping, revert it" — NOT an Erlang supervisor that
  restarts individual crashed worker processes/actors under a hierarchy.
- "Supervisor" here = **update-rollout supervisor**. There is no per-process/per-actor restart hierarchy,
  no "let it crash and restart the child" per-worker pattern. That OTP concept: DOES NOT EXIST in code.

---

## 6. TMR (Triple Modular Redundancy) precedent

- **True TMR (3× redundant computation + majority vote) — NOT FOUND.** Greps for
  `triple modular | TMR | majority(-)vote | triple(-)redundan | voter | quorum | replica` across
  `kernel/src` return nothing. No redundant-compute-and-vote mechanism exists anywhere in the kernel.
- **Single-checksum / integrity-detection (a DIFFERENT mechanism) — GROUNDED, real and extensive.** These
  detect corruption of a SINGLE copy; they do not recompute-and-vote: FDR CRC32 per record
  (`fdr/ring.rs:41-72,257-278`), event-log SHA3-256 hash chain (`event_log.rs:29+`), backup AES-256-GCM
  auth tags + STREAM final-flag (`hub_supervisor.rs:280-318`), hydra spectral-radius integrity check
  (`hydra.rs:211-227`), capability-cert SHA3 transcript hashes (`capability_cert.rs:531`). The clean
  distinction the question asks for: **integrity checksums are real; redundant-vote TMR is absent.**
- **Planned only:** roadmap item 12 "SIHFT triple-vote pilot" is ruled **PURSUE, design-only** (ROADMAP §0
  line 22, §E:259-260) — Software-Implemented Hardware Fault Tolerance / triple-vote is a future design
  item, not implemented.

---

## 7. Guardian pattern / "запобіжники" ↔ existing roadmap items 40 & 9

- **Item 40 vs the "Guardian" pattern — related in shape, DIFFERENT in purpose; and item 40 is not built.**
  GROUNDED: item 40 (ROADMAP:432-440) is a "per-layer golden-checksum oracle + hard-fail": build-time
  golden CRC32 per layer over pinned vectors, runtime spot-check, hard-fail to safe state on mismatch. Its
  own text frames the mismatch as **hardware/memory-fault evidence (a bit-flip), explicitly "not a model
  error."** That is an **integrity/fault-detection** check on the computation. The raw prompt's **Guardian**
  is different: the kernel checks the AI's *decision output* against hard **semantic safety invariants**
  (e.g. `Result.velocity < MAX_SAFE_SPEED`) and rejects hallucinations, with a deterministic fallback
  (RAW-PROMPT-5 lines 470-500). Item 40 detects corrupted bits; Guardian rejects unsafe-but-well-formed
  advice. They share only the "kernel verifies, hard-fail to safe state" shape. Both are **planned, not
  built** (items 33–44 are planning-only). No `guardian`/`sanity` module exists (`ls`/grep clean).
  - **Closest EXISTING "verify-output-before-trust" mechanism (GROUNDED):** `kernel/src/decision/import.rs`
    `import_unit` — "the import-time verify-before-persist gate" that does "independent replay — full
    harvested instance set replayed through the candidate unit AND compared to the local oracle's expected
    verdict; ANY disagreement ⇒ reject" (import.rs:1-12). This verifies a *foreign compiled DecisionUnit*
    against an oracle before trusting it — the same "verify against invariants, reject if invalid" shape as
    Guardian, but applied to compiled decision units at import time, not to live per-inference AI output.
- **Item 9 (breaker) vs "запобіжники" (safeguards/fuses) — partial, conceptual match; breaker NOT built.**
  GROUNDED: item 9 is "build `kernel/src/breaker/` … typed `Result<Permit, Tripped>`, unconstructible
  tripped-but-permitting state, `CommitError` alarms routed in" — "the pivot point of the entire roadmap"
  (ROADMAP:246-249). `kernel/src/breaker*` **DOES NOT EXIST** (`ls` errors) — item 9 is unbuilt. A circuit
  breaker is literally a "запобіжник" (Ukrainian for fuse/circuit-breaker), so the concepts overlap; but
  the operator directive's "із запобіжниками" (with safeguards) is broader than a single circuit-breaker
  primitive — it reads across the breaker AND the Guardian AND checksum/invariant checks. Item 9's breaker
  is ONE specific safeguard mechanism, not the whole "safeguards" concept, and it is not yet implemented.

---

## 8. "AI-optional" baseline — does core logic run without AI?

- **`attention.rs` doc comment — GROUNDED, citation confirmed exactly.** `kernel/src/attention.rs:17-21`
  reads verbatim: "the trained-attention path … is deliberately NOT here — the kernel stays non-AI
  (deterministic pure functions); learning lives in `online` / `micrograd` at the edge if ever needed."
  Also relevant: lines 13-15 — "DETERMINISM: … bit-reproducible across native / wasm32. Float is fine here:
  this is dynamics/affinity, never money." The file is a reference scalar softmax + attention (a "lens"),
  with no learned weights.
- **Core state machines have ZERO AI dependency (even optional/feature-gated) — GROUNDED.** Import-graph
  grep across `order_machine.rs`, `decision/import.rs`, `decision/mod.rs`, `hydra.rs`: none import
  `micrograd`, `online`, `attention`, `neural`, `tensor`, or `infer`. `decision/import.rs` imports only the
  decision registry, `event_log`, and `metrics` (import.rs:37-39); `hydra.rs` imports `event_log` and
  `spectral` (hydra.rs:21-22). The order FSM (`order_machine.rs`) is a 1:1 port of the deterministic TS
  order-machine (order_machine.rs:3-4). The kernel's decision/order/organism logic is fully deterministic
  and AI-free today. (Consistent with the AI-inference synthesis §1.1's finding that no inference engine
  exists in-repo at all.)

---

## 9. Hard real-time determinism principles already present

- **`no_std` — NOT FOUND in the kernel.** The only `#![no_std]` in the workspace is
  `tools/nfc-pod-flipper/src/main.rs:15` (an unrelated embedded tool). The kernel is `std`-based (uses
  `std::fs`, `std::sync`, `std::time` throughout, e.g. `fdr/ring.rs:25-27`). Kernel `no_std`: DOES NOT
  EXIST.
- **Static-allocation posture — GROUNDED, with nuance (fixed-cap bump, not compile-time-static/no-heap).**
  `kernel/src/arena.rs` `BumpArena`: a fixed-capacity `Vec<u8>`-backed bump region, pointer-bump alloc,
  `O(1)` reset, "NEVER grows its region and NEVER panics on exhaustion" — degrade-closed: exhaustion returns
  `None` and the caller falls back to a heap `Vec` (arena.rs:1-32). `T: Copy + Default` removes Drop
  hazards at compile time (arena.rs:14-17). `kernel/src/slot_arena.rs` is a generational-index arena OFF by
  default (behind the `slot-arena` feature, forward-looking, no current consumer, slot_arena.rs:1-24). So
  "static allocation" here = a fixed-capacity arena that avoids per-pass heap churn, NOT compile-time static
  arrays / a heapless kernel.
- **Unbounded loop in a hot/critical path — NONE FOUND (in the sampled files).** No bare `loop {}` in
  `order_machine.rs`, `markov.rs`, `hydra.rs`, or `decision/`. The `while` loops sampled are all bounded:
  bitmask-drain loops that clear bits (`while frontier != 0` / `while f != 0` /
  `while row != 0`, order_machine.rs:304-307, 1001, 1103 — bounded by word width), union-find path
  compression (`while parent[x] != x`, order_machine.rs:643 — bounded by tree depth), and queue drains
  (`while let Some(u) = queue.first()`, order_machine.rs:255). No obviously-unbounded critical-path loop
  found.
- **WCET analysis tooling/documentation — NOT FOUND anywhere.** Grep for `wcet | worst-case execution`
  across `kernel`, `tools`, and `docs/design` returns nothing; the term appears **zero** times even in the
  space-grade roadmap and synthesis docs. "WCET" occurs only inside RAW-PROMPT-5 itself (the source
  dialogue). No static call-graph/WCET tooling exists. DOES NOT EXIST.

---

## 10. Fixed-point vs floating-point posture (non-AI kernel)

- **The non-AI kernel core uses FLOATING-POINT (f64) extensively today — GROUNDED.**
  - `kernel/src/spectral.rs`: ~91 `f64`/`f32` occurrences vs 2 integer types — float-based (spectral
    radius, eigen work).
  - `kernel/src/markov.rs`: ~32 `f64`/`f32` occurrences — float-based (transition probabilities, power
    iteration).
  - `kernel/src/token_bucket.rs` (GCRA-adjacent): `tokens: f64`, `capacity: f64`, `refill_rate: f64`,
    `try_acquire(n: f64)`, `available() -> f64`, elapsed via `as_secs_f64()` (token_bucket.rs:31-38,44,70,
    92,115,167).
  - `kernel/src/attention.rs`: `f64` throughout (its doc explicitly permits float — "never money",
    attention.rs:14-15).
- **Fixed-point exists only where explicitly forced (GROUNDED, per the AI-inference synthesis §1.1, not
  re-derived here):** `tools/eqc-rs/src/cordic.rs` integer Q30 CORDIC (replaced platform-dependent
  `sin`/`cos` in `empirical_identify`), `eqc_gen.rs` integer money-law organs (`div_half_up`), and integer
  cents in `money.rs`. These are the exceptions; the general spectral/markov/GCRA numeric plane is `f64`.
- Plain factual consequence for the next pass: the operator's "100% deterministic, works without AI"
  directive spans code that is **currently f64** (spectral, markov, token-bucket), which the i8/fixed-point
  resolution of items 33–44 covered only for the (non-existent) AI subsystem, not for these.

---

## 11. Existing panic/crash posture

- **FDR `PostMortem` — GROUNDED, real crash-recovery forensics, but a recovery SUMMARY, not a
  register/stack core-dump.** `Kind::PostMortem` is a closed enum variant (`fdr/schema.rs:184-195`).
  `emit_post_mortem(dir, &Recovery)` (`fdr/ring.rs:286-316`) writes ONE `PostMortem` record into a FRESH
  log `fdr.postmortem.jsonl` (never clobbering the recovered segments), fsynced, carrying
  `recovered`/`first_seq`/`last_seq`/`torn_tail`/`crc_failures` (ring.rs:298-304). Its presence/absence is
  driven by whether a `CleanShutdown` marker was recovered (`Recovery.clean`, ring.rs:159-173, 282). So the
  naming overlap with the raw prompt's "PostMortem forensics on crash" is **real, not coincidental** — it
  genuinely records what survived a dirty stop and how much was torn/corrupt — but it summarizes the
  RECOVERY, it does not capture process registers/stack/last-input the way the raw prompt's black-box
  framing implies. (Routing it into the durable `EventLog` is explicitly DEFERRED, ring.rs:286-289.)
- **Custom panic handler / `set_hook` / `unreachable_unchecked` — NOT FOUND.** No `#[panic_handler]`
  anywhere in the workspace (non-`target`). No `std::panic::set_hook`. No `unreachable_unchecked`. The only
  panic-related APIs are `std::panic::catch_unwind`, and every occurrence is inside `#[cfg(test)]`
  fault-injection/chaos tests (`backup.rs:772`, `spectral_cache.rs:509`, `chaos.rs:716,725`,
  `field_eigenmodes.rs:755`) — none is a production panic-handling customization. The raw prompt's
  "`panic_handler` as unreachable code, verified" idea has **no in-repo precedent**. (Consistent with §9:
  the kernel is `std`, so it inherits the default `std` unwinding panic runtime; a custom `#[panic_handler]`
  is a `no_std`/bare-metal construct that does not apply to the current kernel.)

---

## Index of primary citations (all paths absolute)

- FDR (worktree only): `/root/dowiz-wt-space-grade-exec/kernel/src/fdr/ring.rs`,
  `/root/dowiz-wt-space-grade-exec/kernel/src/fdr/schema.rs`,
  `/root/dowiz-wt-space-grade-exec/kernel/src/fdr/mod.rs`,
  `/root/dowiz-wt-space-grade-exec/kernel/src/ct_gate.rs`
- Main kernel: `/root/dowiz/kernel/src/capability_cert.rs`, `/root/dowiz/kernel/src/hub_supervisor.rs`,
  `/root/dowiz/kernel/src/attention.rs`, `/root/dowiz/kernel/src/event_log.rs`, `/root/dowiz/kernel/src/hydra.rs`,
  `/root/dowiz/kernel/src/decision/import.rs`, `/root/dowiz/kernel/src/order_machine.rs`,
  `/root/dowiz/kernel/src/spectral.rs`, `/root/dowiz/kernel/src/markov.rs`,
  `/root/dowiz/kernel/src/token_bucket.rs`, `/root/dowiz/kernel/src/arena.rs`,
  `/root/dowiz/kernel/src/slot_arena.rs`, `/root/dowiz/kernel/src/isolation/mod.rs`,
  `/root/dowiz/kernel/src/ports/payment.rs`, `/root/dowiz/kernel/src/ports/payment_provider.rs`,
  `/root/dowiz/kernel/Cargo.toml`
- Roadmap/synthesis: `/root/dowiz/docs/design/SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md`
  (items 9, 10, 11, 12, 40; §0 rulings), `/root/dowiz/docs/design/DETERMINISTIC-AI-INFERENCE-SYNTHESIS-2026-07-19.md`
- Absent (verified DOES NOT EXIST): `kernel/src/breaker*`, any `guardian`/`sanity` module, any `.tla` file,
  any `#[kani::proof]`/`kani::`, any `#[panic_handler]`, any `wcet` reference, any TMR/vote/quorum construct.
