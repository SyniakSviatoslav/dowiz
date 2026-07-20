# BLUEPRINT — Item 54: Sentinel — Read-Time Integrity Check for Critical LIVE In-Memory Structs

- **Date:** 2026-07-19 · **Tier:** roadmap §J (fourth wave) · **Status:** BLUEPRINT v1 (planning
  artifact, no code). Operator-reversed 2026-07-19 from an earlier draft rejection (§1.1). Dispatch
  after {item 47 wiring + item 50}; the FDR-branch-merge prerequisite is **satisfied** (§0);
  **registry enumeration is startable now** (this doc does it, §2.4).
- **Sources (read this session):**
  `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §J item 54 (lines 867–896);
  `KLEENE-TRUTHFULNESS-VALIDITY-SYNTHESIS-2026-07-19.md` §2.3 (proportionality RULING, the
  operator-reversal record); `docs/audits/hardening/CHECKLIST.md`;
  `BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md` (style/depth template).
- **Ground-truth code cited (branch `main`, verified in-tree this session):**
  `kernel/src/fdr/ring.rs` (CRC32 primitive + the `Alarm`-fsync path); `kernel/src/fdr/mod.rs` (the
  wasm gate on `ring`); `kernel/src/fdr/schema.rs` (`Kind::Alarm`); `kernel/src/ports/agent/cap.rs`
  (`AnchorRoster`, `RevocationSet` — the present-day candidate authority structs);
  `kernel/src/ports/agent/admission.rs` (`Admitter.admitted` live capability set);
  `kernel/src/order_machine.rs` (`FSM_ADJ` — the const that does NOT qualify, §2.4).
- **Upstream:** item 47 (`Invariants` table, the highest-value target — unbuilt); item 21
  (gain-schedule — unbuilt); item 40 (read-only weight checksum — the plane boundary). This item's
  strongest *canonical* instance rides item 47; its strongest *present-day* instances ride nothing
  new (§2.4).
- **Downstream:** composes with item 47's `Rejection` seam and item 9's `Result<Permit, Tripped>`
  when it lands (does NOT gate on item 9, roadmap:888).

---

## 0. Dependency-status correction

The roadmap gates item 54 on *"the FDR branch merge (for the `Alarm` record)"* (lines 868–869, 901).
**That merge has happened** — `main` HEAD `6701bbb6f` includes the whole FDR module. Verified live:
`kernel/src/fdr/ring.rs` (CRC32 + fsync-on-`Alarm`), `kernel/src/fdr/schema.rs:189` (`Kind::Alarm`),
`kernel/src/fdr/ring.rs:134` (`sync_data` on `Alarm`/`PostMortem`). So the `Alarm` fault-record and
the CRC32 primitive are **already in-tree**; the only genuinely-unbuilt prerequisites for the
*canonical* target are items 47 (`Invariants`) and 50 (`Rejection` composition). Present-day
candidate structs (§2.4) need **neither** — they exist now.

## 1. Scope / goal

Add a read-time (transition-point) integrity check to a **narrow, enumerable set of long-lived,
mutable authority structs** whose in-memory corruption (a hardware bit-flip on non-ECC consumer RAM)
would silently mis-authorize / mis-decide, and which have **no at-rest backing that already verifies
them**. On mismatch: one fsynced FDR `Alarm` + a deterministic fail-closed path. Reuses the in-kernel
CRC32 (no new primitive, no new algorithm, no external crate).

### 1.1 The premise (operator-reversed, load-bearing)

The target is **local, offline-first, consumer-grade hardware that typically LACKS ECC** (roadmap:
870–872; DECISIONS.md D0's local-first invariant). An earlier draft rejected the Sentinel on a
"commodity ECC cloud hardware" argument; the operator reversed it on two grounds (synthesis §2.3):
(i) space-grade *engineering quality* is the standard regardless of substrate, and (ii) the
deployment premise was factually wrong — no ECC ⇒ the in-memory bit-flip fault class is *higher*, not
negligible. This blueprint is written under the corrected premise; the old rejection is superseded.

**Non-goals:** NOT every struct (that would be the theater the reversal still guards against —
scope axis §3.1); NOT per-field-read (transition-point frequency only, §3.3); NOT a cryptographic
hash (the threat is a hardware fault, not an in-memory adversary — CRC32 suffices); NOT a re-cover of
item-40's read-only weights (plane boundary, §3.4); NOT a new dependency or algorithm.

## 2. Current-state grounding

### 2.1 The live-struct read-time pattern is genuinely absent

Every existing integrity mechanism is **at-rest**, verified this session:
- `kernel/src/backup.rs` content-addressed verify-on-get (at-rest CAS);
- `kernel/src/event_log.rs` `verify_chain` (at-rest chain-walk);
- `kernel/src/fdr/ring.rs` per-line CRC32 on the durable ring (`ring.rs:65`, `:120`, `:263` — at-rest
  on disk).
None recompute-and-compare a **live in-memory struct at use time**. The gap the roadmap names
(roadmap:873–874) is real.

### 2.2 The primitive to reuse — CRC32, with a wasm caveat that is a real design input

`kernel/src/fdr/ring.rs:65` `pub fn crc32(data: &[u8]) -> u32` (IEEE reflected, table-on-first-use
`ring.rs:44–62`, KAT-checked `ring.rs:334` against `0xCBF43926`). This is "the in-kernel CRC32
already built for the FDR module" the synthesis mandates reusing (roadmap:880–882; P2 max-nativeness).

**Verified caveat (not in the synthesis):** the `ring` module is `#[cfg(not(target_arch = "wasm32"))]`
(`kernel/src/fdr/mod.rs:52–53`), so `fdr::ring::crc32` is **not compiled on wasm32**. Item 47's
`admit` (over which the `Invariants` sentinel runs) is part of the kernel decision plane that
compiles to WASM (CLAUDE.md: "kernel … compiles to WASM"). Therefore item 54's use of CRC32 in the
decision path requires **lifting `crc32` (and its table) out of the wasm-gated `ring` module into an
always-compiled location** — a behavior-preserving move (e.g. `kernel/src/fdr/mod.rs` top-level, or a
`crate::crc32` module), keeping ONE implementation shared by items 40/51/54. This is a concrete,
small, structural prerequisite the executor must do first — flagged in §7.1. (Alternative:
default-build `event_log::sha3_256`, but that abandons the shared-CRC max-nativeness goal and costs
more.)

### 2.3 The Safe-State record already exists and self-fsyncs

`Kind::Alarm` (`kernel/src/fdr/schema.rs:189`) is the fault-evidence record kind;
`kernel/src/fdr/ring.rs:134` fsyncs (`sync_data`) on every `Alarm`/`PostMortem` append (power-loss
durable). So "emit ONE fsynced FDR `Alarm` on mismatch" (roadmap:885–887) is a call to an existing,
tested path — no new durability machinery.

### 2.4 The critical-struct registry — enumerated now, per-struct justified (3-axis test)

A struct qualifies iff **(a) long-lived, (b) an authority input to a money/safety/decision path, and
(c) has no at-rest backing that already verifies it** (roadmap:875–878). Applying the test honestly to
the live tree:

**CANONICAL targets (named in the synthesis; their PARENT structs are unbuilt):**
| Candidate | (a) long-lived | (b) authority | (c) no at-rest backing | Status |
|---|---|---|---|---|
| item 47 `Invariants` table | yes (loaded once, immutable-after-init) | **highest** — a flipped bound silently mis-certifies *every* subsequent `admit` (roadmap:877) | yes | **unbuilt** (item 47 spec-level — see item-50 blueprint §2.2) |
| item 21 gain-schedule / decision-config | yes | yes (decision tuning) | yes | **unbuilt** (item 21) |
| live inference config (`ActiveAIContext`-class) | yes | yes (advice path) | yes | **unbuilt**; distinct from item-40 read-only weights (§3.4) |

**PRESENT-DAY targets (exist NOW, qualify under the same test — this doc's grounding contribution):**
| Candidate | Citation | Why it qualifies |
|---|---|---|
| **`AnchorRoster`** (trust-root set) | `kernel/src/ports/agent/cap.rs:377` (`enroll` `:387`, `remove` `:391`) | (a) long-lived roster of trusted anchor keys; (b) a flipped anchor key silently changes which roots authorize the entire capability chain in `verify_chain` — a security-decision authority; (c) held live in memory, no per-use at-rest re-verify. **Qualifies.** |
| **`RevocationSet`** (revoked keys/caps) | `kernel/src/ports/agent/cap.rs:412` (`revoke_key` `:423`, `is_revoked_key` `:431`, `merge` `:439`) | (a) long-lived, grows via anti-entropy `merge`; (b) a flipped revocation bit silently **un-revokes** a revoked key → admits a revoked agent; (c) live, no per-use at-rest verify. **Qualifies (mutable — the merge path is a real re-hash site, §3.3).** |
| **`Admitter.admitted`** (live capability set) | `kernel/src/ports/agent/admission.rs:350` (`admitted: HashMap<[u8;32], AdmissionRecord>`) | (a) live for the admitter's lifetime; (b) drives semantic-re-admission idempotency + granted-envelope authority; (c) no at-rest backing (the WORM event log records the *decision*, but the live grant map is separate). **Qualifies, lower priority** (a flip here degrades to a re-admission, not a trust-root breach). |

**EXPLICITLY EXCLUDED — the honest boundary calls:**
- **`FSM_ADJ`** (`kernel/src/order_machine.rs:208` — `const FSM_ADJ: [u16; 12] = build_adjacency()`)
  and `FSM_GOLDEN_SIGNATURE` (`order_machine.rs:513`) are `const` / immutable-after-compile —
  **read-only static data ⇒ item 40's plane** (build-time golden CRC32 over static data), NOT item
  54's live-mutable plane. The task's "order FSM state" hint resolves to: there is no long-lived
  *mutable* FSM authority table to sentinel; the transition-legality authority is a `const`, so it is
  item 40's job. Named here so the boundary is explicit, not silently skipped (roadmap:889 plane
  distinction).
- Transient hot-loop scratch (e.g. arena buffers, `kernel/src/arena.rs`) — excluded (not long-lived,
  not authority).
- Anything already at-rest-verified (`event_log` chain, `backup` CAS) — excluded (axis c fails).

## 3. Implementation plan

### 3.1 Scope axis — the registry above, nothing wider

Build the Sentinel for the qualifying set only. Recommended landing order by value:
`Invariants` (when item 47 lands) > `AnchorRoster` / `RevocationSet` (present-day, land first as the
mechanism's first customer) > `Admitter.admitted` > gain-schedule (rides item 21) > inference config.
Each struct's qualification is recorded with the 3-axis justification (§2.4 is the seed).

### 3.2 Primitive axis — one shared CRC32, lifted to always-compiled

1. **Lift `crc32` + `CRC_TABLE`** out of `#[cfg(not(target_arch = "wasm32"))]` `fdr::ring`
   (`ring.rs:44–72`) into an always-compiled module (behavior-preserving move; the KAT test moves with
   it). `ring.rs` re-exports/uses it. This is a §7.1 prerequisite the moment the sentinel runs in a
   wasm-compiled path.
2. **The sentinel wrapper.** A small `struct Sentineled<T> { inner: T, crc: u32 }` (or a trait
   `IntegrityChecked` with `fn checksum(&self) -> u32` + `fn verify(&self) -> Result<(), Corrupt>`)
   over the registry structs. `checksum` CRCs the struct's authority bytes (the roster's sorted keys,
   the revocation set's sorted entries — reuse the existing `snapshot_sorted` helpers seen in
   `admission.rs:1036`/`:1051` so the hashed form is canonical and mutation-order-independent).

### 3.3 Frequency axis — transition points, not per-field-read

- **Read/use time (once per authority-use):** `verify()` once per `admit` over the `Invariants`
  (amortized across the whole admission), once per `verify_chain` over the `AnchorRoster` /
  `RevocationSet` — NOT per field access (bounds the hot-path tax, roadmap:882–885).
- **Recompute-and-store on mutation:** on the rare, centralized write sites only —
  `AnchorRoster::enroll`/`remove` (`cap.rs:387`/`:391`), `RevocationSet::revoke_key`/`merge`
  (`cap.rs:423`/`:439`). These are the handful of re-hash sites; a missed one manufactures a false
  trip, so they are bounded to these central mutators (roadmap:883–885). An **immutable-after-init**
  struct (the `Invariants` table) is a pure read-time check with **zero re-hash burden** — the ideal
  case, no false-trip surface.

### 3.4 Relationship to item 40 — the boundary, one shared CRC

Item 40 IS the sentinel pattern for **read-only static WEIGHTS** (build-time golden CRC32 over static
data). Item 54 scopes to **live MUTABLE authority structs item 40 structurally cannot touch**
(`Invariants`, gain-schedule, inference config, and the present-day roster/revocation structs). Same
CRC implementation (post-lift), same Safe-State semantics, complementary planes (roadmap:888–889). The
overlap is a boundary, not a reason to skip — item 54 does not re-cover weight integrity.

### 3.5 Safe State on mismatch — fail-closed

A CRC mismatch is hardware/memory-fault evidence (item-40 semantics). Hard-fail:
1. emit ONE fsynced FDR `Alarm` (the existing `Kind::Alarm` + fsync path, §2.3) naming the corrupted
   struct;
2. take the deterministic fail-closed path — for `Invariants`, **REFUSE the admission** (a corrupted
   invariant table certifies nothing) via item 47's `Rejection` seam; for `AnchorRoster`/
   `RevocationSet`, refuse the capability verification (deny-closed). Composes with item 9's
   `Result<Permit, Tripped>` when it lands (does NOT gate on item 9).

## 4. Required tests / proofs (CHECKLIST.md 5-point standard)

1. **Oracle.** Behavioral: a planted single-bit corruption of a registered struct (behind a
   test-only `cfg`, via a raw-pointer flip — mirroring item-40's planted-fault test) demonstrably
   trips the Safe-State path AND writes the `Alarm`, recovered via `fdr::ring::recover`
   (`ring.rs:230`). An **uncorrupted** run is checksum-silent (no `Alarm`). `mutate-then-read` (via a
   central mutator) passes — proves re-hash correctness. Corpus/behavioral oracle → manifest
   `N/A(behavioral-oracle)` for the strong differential form; the CRC32 itself already has a KAT
   (`ring.rs:334`).
2. **dudect — `N/A`.** CRC32 over authority bytes is not secret-dependent-timing-sensitive (the
   threat is a hardware fault, not a timing side channel). Records `N/A(no-secret-timing)`.
3. **Debug cross-check.** `debug_assert!(self.verify().is_ok())` at each authority-use entry
   (compiled out of release) — continuous verification at zero production cost, the CHECKLIST item-3
   idiom.
4. **Assembly spot-check — `N/A`** (no new branch-free CT path).
5. **Kani — optional.** The CRC32 fold is enumerable/table-driven; if a machine proof of "verify()
   never panics / never false-negatives on a single-bit flip" is wanted, it is a small `#[cfg(kani)]`
   harness joining item 7's `mode=kani` rows — not required for landing.

**RED→GREEN proofs (P7, in the PR):**
- planted single-bit corruption → Safe-State path taken + `Alarm` written (red→green; restore →
  silent);
- an **uncorrupted** run writes **no** `Alarm` (guards against a vacuous always-trip);
- `mutate-then-read` through a central mutator passes (re-hash correctness — proves the §3.3 write
  sites are wired);
- CI re-executes the planted-fault test (the item-6 re-execute-not-presence-check discipline);
- `cargo tree -e no-dev` is **byte-unchanged** (existing CRC32 reused, zero new dependency and zero
  new algorithm — the max-nativeness law, roadmap:894–896) — asserted by the existing zero-dep-gate.

## 5. Falsifiable acceptance criteria

- A single-bit flip of any registered struct's authority bytes trips fail-closed AND produces exactly
  one recoverable `Alarm` naming that struct.
- A clean run over the full registry produces **zero** `Alarm` records (no false trips).
- Mutating a registered struct through its central mutator then reading it passes (no false trip on
  legitimate mutation).
- `cargo tree -e no-dev` and `Cargo.lock` are byte-unchanged (no new dependency; CRC32 lift is a
  pure code move).
- The critical-struct registry is enumerated in-code with a per-struct 3-axis justification (why
  critical, why no at-rest backing) — a reviewer can read each qualification; `FSM_ADJ` is explicitly
  documented as excluded-because-`const` (item-40 plane).
- The `crc32` function is callable from a wasm-compiled kernel path (proves the §3.2 lift landed).

## 6. Dependency gates (honest)

| Gate | Status | Effect |
|---|---|---|
| FDR merge (CRC32 + `Alarm`) | **MET** (§0 — stale roadmap flag corrected) | primitive + fault record live in-tree. |
| CRC32 wasm-lift | **OPEN — small structural prereq** (§2.2/§3.2) | required for the `Invariants`/decision-plane instance; a behavior-preserving move, not a new algorithm. |
| Item 47 `Invariants` + item 50 `Rejection` | **NOT MET** (unbuilt) | blocks only the **canonical** `Invariants` instance. The **present-day** roster/revocation instances (§2.4) need neither and can land first as the mechanism's first customer. |
| Item 21 gain-schedule | **NOT MET** (unbuilt) | blocks only the gain-schedule instance. |
| Item 40 (weights checksum) | sibling spec | shared CRC + Safe-State semantics; complementary plane, not a blocker. |
| Item 9 (breaker) | not required | composition only; design does NOT gate on it (roadmap:888). |

## 7. Operator / executor decision points (flagged)

1. **CRC32 lift location (executor).** Where the lifted `crc32`/`CRC_TABLE` lives (`fdr` top-level vs
   a new `crate::crc32`), and confirming the move is behavior-preserving (the `ring.rs:334` KAT moves
   with it and stays green). Recommend `fdr::crc32` at the `fdr/mod.rs` level (drop the wasm gate on
   just that function), minimal blast radius. Executor's call; a pure move, no algorithm change.
2. **Present-day-first vs canonical-first landing (operator).** This blueprint recommends landing the
   `AnchorRoster`/`RevocationSet` instances **first** (they exist now, need no unbuilt item, and are a
   security-decision authority) as the Sentinel's first proven customer — then adding the `Invariants`
   instance when item 47 lands. If the operator prefers to wait for the canonical `Invariants` target
   only, the present-day instances are deferred — flag, do not assume.
3. **Whether `Admitter.admitted` is in-scope for v1.** It qualifies but is lower value (a flip
   degrades to a re-admission, not a trust breach). Recommend deferring it behind roster/revocation;
   executor confirms at dispatch.
4. **Canonical hashed form for mutable sets.** The roster/revocation CRC must be over a canonical
   (sorted) byte form so `merge`/`enroll` order does not spuriously change the checksum — reuse
   `snapshot_sorted` (`admission.rs:1036`). Executor confirms every mutator re-hashes the sorted form.
