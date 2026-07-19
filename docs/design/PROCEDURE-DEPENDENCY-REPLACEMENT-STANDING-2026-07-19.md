# Standing Procedure — Dependency Replacement / Adoption Rulings (Item 25)

> **Blueprint note for the Opus executor (Tier-1, item 25):** This doc IS the deliverable, not a
> plan for one — the procedure below is extracted from the two real precedents (verified against
> `kernel/src/slot_arena.rs`, `kernel/src/arena.rs`, `kernel/src/pq/entropy.rs`, and
> `kernel/Cargo.toml` this session, file:line cited), and is ready for items 4+29 and 5 to apply
> as-is. Verification pass: confirm the citations still match HEAD, then commit; no rewrite needed.

**Status:** STANDING. Required by roadmap §B ("Item 25 (procedure doc first) … **before**
executing items 4/5") and synthesis §18(a) ("re-audited under this exact procedure, not a
bespoke one"). Scope: every question of the form *"should this external dependency exist in a
kernel build — and if code replaces it, under what discipline?"* — both directions: adopting a
new crate (the two precedents) and retiring an incumbent one (items 4, 5, 29, 31-enactment).

---

## 1. The two precedents, as actually executed

### 1.1 Slot-arena / thunderdome (opt-in adoption, operator-overridden verdict)

Sources, all verified this session: `docs/research/OPUS-PERF-ARENA-DEEPDIVE-2026-07-18.md`
(deep-dive + §6 UPDATE), `kernel/src/slot_arena.rs` (module doc lines 1–59, tests 163–285),
`kernel/Cargo.toml` (feature comment lines 64–77, dep comment lines 146–151).

What the decision actually looked like, in order:

1. **Trigger named before work**: an operator-directed research question ("do we need
   generational-index safety?"), answered by a full deep-dive, not an inline judgment call.
2. **Evidence before verdict**: a six-way tree sweep for *current* real need (found none); a
   real crate comparison (slotmap / generational-arena / thunderdome); the crate's claimed edge
   re-verified in-house, not cited from reputation (8-byte key, niche-packed 8-byte
   `Option<Index>`; **zero transitive runtime deps**, checked against its manifest); and a
   hand-rolled `SlotArena` **drafted and compile-checked** (~150–200 lines, ABA/stale/
   double-remove tested) with an honest loss accounting ("~4 bytes bigger per handle, not yet
   as tight" — deep-dive §3/§4 DECART table).
3. **Verdict recorded with teeth**: "(c) no adoption now — neither the crate nor the
   hand-roll — but the code is drafted, compile-checked, and parked … The first plausible
   trigger is named" (deep-dive §0 item 4). The reopening trigger is concrete: an incremental
   mesh/graph index that deletes nodes while other structures hold references (§5.2).
4. **Operator ruling recorded verbatim, in the code**: the operator explicitly overrode the
   verdict ("land thunderdome now as forward-looking infrastructure, behind a feature so it
   costs the default build nothing") — recorded in **three places**: deep-dive §6 UPDATE,
   `slot_arena.rs:10–24` ("Why this exists … operator override of the 'no adoption yet'
   verdict"), and the `Cargo.toml:64–77` feature comment ("Added per operator directive
   OVERRIDING that deep-dive's own 'no adoption yet' verdict").
5. **Adoption shape = opt-in + swap-preserving seam**: `slot-arena = ["dep:thunderdome"]`
   (`Cargo.toml:77`), OFF by default; `Handle` is an opaque newtype whose `thunderdome::Index`
   payload is private (`slot_arena.rs:26–31, 72–73`), so **no call site can ever name
   `thunderdome::*`** and the backing crate is swappable for the parked hand-roll without
   touching callers. Degrade-closed like its sibling `arena.rs` (`Option`, never panic —
   `slot_arena.rs:33–49`; cf. `arena.rs:25–27`).
6. **Default-build absence verified mechanically, command recorded in the manifest**:
   "`cargo tree -p dowiz-kernel -e no-dev | grep -c thunderdome` → 0" (`Cargo.toml:74–76`).
7. **Tests assert the claim, not just compilation**: 8 tests including
   `aba_defeated_across_removal_and_slot_reuse` (`slot_arena.rs:209–226`) and
   `handle_and_option_handle_are_both_eight_bytes` (`slot_arena.rs:274–284` — "Asserted, not
   assumed"); full lib suite green both with and without the feature (deep-dive §6: 704 → 712).

### 1.2 QRNG entropy provider (opt-in new primitive, never-replace rule)

Source, verified this session: `kernel/src/pq/entropy.rs` (whole file, 182 lines), reached via
`pq` (itself `#[cfg(feature = "pq")]`-gated, `kernel/src/lib.rs:13–14`).

1. **Security model documented before the mechanism** (`entropy.rs:9–11`): "NEVER use raw
   quantum noise alone. Mix it with OS entropy so a biased/failed QRNG cannot collapse the
   seed. SHAKE256(quantum || os) gives a seed whose entropy ≥ max(H(quantum), H(os))" — cited
   to NIST SP 800-90B. The new primitive is bounded by an invariant, not by trust.
2. **The "QRNG-seeded-never-replace" standing rule is enforced in code, not just in memory**:
   `master_seed()` (`entropy.rs:86–107`) — doc comment "**Sanctioned master-seed entry point
   (operator directive 2026-07-12).** Native OS entropy is the DEFAULT and the FALLBACK";
   quantum noise can only ever **upgrade** the seed via the mix, and on ANY failure the
   function transparently returns the OS-seeded master. The explicit-quantum path
   `quantum_seeded_master()` instead **fails closed** with `Err` (`entropy.rs:58–66`) so a
   caller can never silently weaken a seed. One sanctioned entry point; the operator directive
   recorded in its doc comment.
3. **Opt-in gating, double-off**: "No network/OS call is compiled in by default — the provider
   is behind the `qrng` feature so the core stays dependency-free and auditable"
   (`entropy.rs:5–7`). Verified fact this session (item-25 re-verification, direct evidence):
   **`qrng` is not declared anywhere in `kernel/Cargo.toml` — absent from both `[features]` and
   `[dependencies]`** (`grep -n qrng kernel/Cargo.toml` → no match; `cargo build --features qrng
   -p dowiz-kernel` → `error: the package 'dowiz-kernel' does not contain this feature: qrng`).
   Because `--all-features` can only enable *declared* features, the entire
   `#[cfg(feature = "qrng")] pub mod provider` (`entropy.rs:37–148`) — **including the
   "sanctioned `master_seed()` entry point" itself** (`:96–107`, which lives inside that gated
   module) — is presently **uncompilable in every build**, default or `--all-features`. Only the
   pure mixing functions `entropy_mix`/`derive_seed` (`:17–35`) compile (under `pq`). This is
   stricter than opt-in and one notch more conservative than the synthesis §18(a) framing (which
   implies a declared feature) — but it also means the "QRNG-seeded-never-replace" enforcement
   §1.2(2) describes is currently **latent, not live**: correct + tested but never compiled, the
   same "correct-and-unreachable" class as item 2's `FileEventStore` wiring gap. Enabling it is
   an explicit manifest change that will trip the item-1 CI dependency gate and force this
   procedure to run. The standing-rule-vs-dead-code inconsistency is flagged as an owed ticket in
   §3 (new scope, filed not fixed — outside item 25's own docs-only boundary).
4. **Honest limitation + named upgrade trigger recorded inline** (`entropy.rs:112–115`): the
   transport is a plain-TCP std-only stub, "PRODUCTION MUST use a TLS client … Upgrade
   trigger: any real deployment — gate behind `qrng-tls` feature with reqwest." No open-ended
   "revisit later" — a concrete event reopens the question.
5. **Tests gated like the code**: mixing-property tests always run (`entropy.rs:154–173`); the
   network-touching test rides `#[cfg(feature = "qrng")]` (`entropy.rs:175–180`).

The same discipline has since run twice more — `gpu` (`Cargo.toml:63`, synthesis §18-GPU note)
and `agent-adapters`' wasmtime/`FuelMeter` (synthesis §25 table) — confirming this is a
pattern, not a coincidence of two files.

---

## 2. The standing procedure (numbered, mandatory)

Every dependency ruling — adopt, retire, or keep — walks these ten steps in order. "Executor"
below = whoever runs the ruling (items 4/5/29/31: the Opus executor).

1. **Name the trigger.** State, in the ruling record, the concrete event that opened the
   question (audit item, CI gate failure, new capability need). No trigger, no ruling.
2. **Sweep for real current need / real current usage.** Grep the tree: which code paths
   actually exercise what the crate provides? Cite file:line, or record "no current
   need/usage" explicitly (the deep-dive's six-way sweep is the model). For retirement
   (items 4/5): enumerate every call site of the incumbent crate — this list is the cutover's
   test surface.
3. **Verify the claimed edge yourself.** Whatever the crate (or the rewrite) is supposed to be
   better at — measure or inspect it in-house; count transitive deps with `cargo tree -e
   no-dev`. Reputation, stars, and "widely used" are not evidence (rust-native-bare-metal
   standing rule: reject appeal-to-authority).
4. **Bring the in-kernel alternative to compile-checked, test-passing state BEFORE ruling** —
   with an honest loss accounting (where it is worse than the incumbent, quantified). If the
   ruling ends "keep the crate," the alternative is **parked** in the record, not landed — so
   a future swap is a copy-in-and-test job, not a design pass.
5. **Rule into exactly one of three terminal states.** (a) **Removed outright** — the
   in-kernel code takes over (items 4/5's target). (b) **Kept, opt-in** — behind a feature
   flag, OFF by default, absent from the default tree (slot-arena/qrng/gpu shape). (c)
   **Kept, legitimate boundary** — a §13(c) syscall/wire/ABI crossing; surface-minimization
   target only. "Keep unconditionally in the default build" is not a permitted outcome unless
   (c) is ruled explicitly, with the boundary named.
6. **Preserve the fallback/rollback path.** Call sites bind to a kernel-owned seam (opaque
   wrapper, single sanctioned entry point) and never the third-party symbols directly, so the
   backing implementation is swappable without touching callers (`Handle` precedent). The
   proven incumbent remains the DEFAULT and the FALLBACK until the replacement has carried
   real load: a new primitive may only ever **upgrade** the proven path, never silently
   replace it, and any failure of the new path falls back transparently (`master_seed()`
   precedent). For retirements, the last released artifact with the incumbent is the rollback;
   the cutover commit must be revertable in isolation.
7. **Test coverage required before cutover.** Tests must assert the *claimed properties*, not
   compilation: parity/byte-compatibility against the incumbent where outputs are comparable
   (item 4's proof is literally "log output byte-compatible"); the specific safety/size/
   perf claims asserted, not assumed (`assert_eq!(size_of::<Handle>(), 8)` precedent); the
   full suite green in **both** configurations (feature on/off, or pre/post cutover).
8. **Verify default-build absence mechanically.** `cargo tree -e no-dev --locked --offline |
   grep -c <crate>` → 0 for state (a) and (b); the exact command written into the manifest
   comment next to the feature/removal (Cargo.toml:74–76 precedent). Update the item-1 CI
   gate allowlist in the same change — it only ever shrinks.
9. **Record the ruling where the next reader will trip over it — three places.** (i) The
   seam's module doc: "Why this exists," the verdict, and who overrode what, verbatim if an
   operator ruling exists (`slot_arena.rs:10–24` format). (ii) The Cargo.toml comment on the
   feature/dependency line: the invariant plus its verification one-liner. (iii) The
   deep-dive/design doc, with an UPDATE section if the verdict was later overridden
   (deep-dive §6 format). A ruling that lives only in a chat transcript does not exist.
10. **Name the reopening trigger.** One concrete, observable future event that reopens the
    question ("an incremental mesh/graph index that deletes nodes…"; "any real deployment →
    `qrng-tls`"). Then stop. **Done =** terminal state reached + step-7 proofs green +
    step-8 command → 0 + step-9 records written + this trigger named.

---

## 3. Binding: who must apply this, now

- **Items 4 + 29 (+ `JsonWriter`) — logger/FDR rewrite** (`tracing`, `tracing-subscriber`):
  MUST run steps 1–10 per crate. Expected terminal state is (a) removed outright; the
  roadmap's own proof line ("`cargo tree` drops 13+ crates, log output byte-compatible,
  post-mortem readback test") is steps 7–8 instantiated. Note: the `telemetry` feature's
  `SpanMetricsLayer` (`kernel/Cargo.toml` telemetry comment) currently *reuses* the
  already-linked `tracing` — the item-4 ruling must cover that consumer too, or the crate
  survives in a feature branch of the tree.
- **Item 5 — `regex` retirement**: MUST run steps 1–10; roadmap §B says explicitly "Ruling
  recorded per item 25's procedure." Expected terminal state (a); proof is step 8 ("zero
  external crates") plus step 7 ("existing parsing tests green").
- **Item 31 (enactment half)** — per-crate allowlist + shared JSON-parse primitive for the
  seven serde carriers: each rewrite-candidate row goes through this lifecycle, rulings
  recorded in each manifest "in the `slot_arena.rs` format" (synthesis item 31). Depends on
  this doc per roadmap §C.
- **The §0.1/item-1 CI gate** cites this doc in its failure message (synthesis item 25 proof
  clause), so every future tripped gate routes its resolver here.
- Owed ticket, filed not fixed (roadmap §A item 31 note): the dual in-kernel Keccak-f[1600]
  (`event_log.rs:67` vs `pq/keccak.rs:156`) dedup is an *internal* duplication, not a
  third-party ruling — it applies steps 5–9 with "incumbent" read as the surviving copy.
- Owed ticket, filed not fixed (surfaced by item 25's re-verification, new scope): the `qrng`
  feature is **undeclared** in `kernel/Cargo.toml` (verified §1.2(3)), so the entire QRNG
  provider — including the sanctioned `master_seed()` entry point — is dead code that never
  compiles in any build. A documented **"QRNG-seeded-never-replace" standing rule (MEMORY,
  `integration-ports-reactive-arc`) with genuinely-uncompilable code behind it is a real
  standing-rule-vs-reality inconsistency**, in the same "correct-and-unreachable" class as
  item 2's `FileEventStore`. This is NOT an item-25 fix (item 25 is docs-only); it wants its own
  small follow-up build ticket to either (a) declare the `qrng` feature so the provider compiles
  and this procedure governs its enabling, or (b) explicitly re-scope the never-replace rule to
  the always-compiled `entropy_mix`/`derive_seed` seam and mark the provider as a parked sketch.
