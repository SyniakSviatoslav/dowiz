# BLUEPRINT P76 — bebop hidden-tests un-gate + bus-lock fix (2026-07-19)

> **Standalone blueprint (bebop-repo: `bebop2/delivery-domain` + `crates/bebop`).** One coherent,
> independently buildable unit against the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md`
> §2. Research source: `docs/research/OPUS-PERF-BESTPRACTICES-PROPAGATION-2026-07-18.md` ("R8", §3.2
> G-T1 + §4.2 G-C1 + §1.2 G-E2). Synthesis source: `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` (S1)
> §3.1 Tier-A findings **A2**/**A3** + §4 flag **D-2**. Format precedent:
> `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`. Grounding tree read live this pass:
> `/root/bebop-repo` at HEAD (working tree), plus `/root/dowiz/kernel` for the red-line anchor.
>
> **RULE (repo topology — non-negotiable):** every file this blueprint changes lives in
> **`/root/bebop-repo`, NOT `/root/dowiz`** (this planning doc lives in dowiz, same pattern as P92).
> bebop-repo's live push remote is **`openbebop`** (`git@github.com:SyniakSviatoslav/OpenBebop.git`);
> its `origin` is the archived read-only legacy repo — **never push there** (memory:
> `MEMORY.md` "BEBOP `origin` REMOTE ARCHIVED"). Push P76's landed commits to `openbebop`.
>
> **One sentence:** make the already-written `bebop2/delivery-domain` split-brain / double-finalization
> settlement-safety tests **provably execute with their count visible** (they exist but are silently
> excluded from the default gate), and **land** the already-written-and-verified pub/sub bus
> snapshot-under-lock fix (which removes a latent re-entrancy self-deadlock) — both blocked from
> landing only by the operator-owned bebop **C3** ungated-keygen HARD-law red state.

---

## VERDICT (stated up front, per session discipline)

**GO.** Both halves are safe, cheap, and strictly-better; one of the two is **already written, verified,
and present in the working tree**. There is nothing to design from scratch and nothing to gamble on.

- **A2 (un-gate hidden tests) — GO, genuinely high value, near-free.** The `bebop2/delivery-domain`
  finalization / intake / hub-ring / facade tests — including the **split-brain double-finalization**
  gate on a *settlement-safety* surface — are real, falsifiable, RED→GREEN tests that **the default
  `cargo test` silently excludes** (§0.1). This directly undercuts the repo's RED→GREEN discipline: a
  developer sees green while every one of these tests is skipped. The fix is a CI job, not a code
  change. **Say it plainly: this is a genuine, cheap, high-value fix.**
- **A3 (bus-lock fix) — GO as an ABSORPTION, not an implementation.** The fix already exists as
  `/root/dowiz-perf-contention/docs/research/bebop-bus-G-C1-fix.patch`, is **already applied in the
  live `/root/bebop-repo` working tree** (verified §0.2), carries its own regression tests
  (`reentrant_handler_does_not_deadlock`, `publish_preserves_order_and_loses_no_dispatch`) and a
  criterion fan-out bench, and bebop lib was **443 tests green** on the fix branch (S1/S3). P76 must
  **describe and land** this patch, **never re-write it**. Its DoD is *"land the existing verified
  patch through the hooks + push to openbebop,"* not *"write the fix."*
- **The only gate is operator-owned.** Both halves are **commit-blocked** on the pre-existing bebop
  **C3** HARD-law red state (`scripts/ci-no-ungated-keygen.sh` fails on ungated constant-seed
  `pq_dsa::keygen`/`pq_kem::keygen_internal` — unrelated to either change) which currently freezes
  **all** hook-respecting commits on the bebop working branch (MASTER-STATUS-LEDGER §3; SYNTHESIS-WAVE3-
  CLOSEOUT §2/§4). Per the master sequence, **P85 (NTT remediation) + C3 resolution precede the entire
  bebop lane** (OD-3). P76 records this landing precondition; it does **not** own resolving it.
- **Carried, not decided: D-2 (`reputation.rs`).** P76 surfaces the courier/node-scoring red-line
  divergence (§10.2) as an operator flag (OD-8) and does **not** block the A2/A3 landing on it — the
  fix is trivial either way once ruled, and it is not in P76's edit set.

**Honesty guardrails honored throughout:** A3's blast radius is *limited today* — both bus files are
documented **offline stand-ins in the legacy `crates/bebop` TUI crate, off the mesh product path**
(§0.2, §0.5). This is a **latent** defect prevented + an already-done fix landed, **not** a live
production deadlock. The bus fix carries **no throughput-win claim** beyond "concurrent publishes no
longer serialize behind one lock + re-entrant handlers no longer self-deadlock"; the fan-out bench only
guards the happy path from regression. A2, by contrast, is on the **real mesh product path**
(`bebop2/delivery-domain`) and is the higher-value of the two.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." Every claim below was read from source **this pass**
> (`/root/bebop-repo` HEAD working tree, `/root/dowiz/kernel`), not inherited from R8/S1 shorthand.
> **Three corrections** to the source docs' shorthand are made here because a correct blueprint
> requires them (§0.3 anchor path, §0.4 missing template, §0.2 patch-already-applied).

### 0.1 A2 — the hidden tests: they exist, they are safety-critical, they never run by default

The four test modules are gated behind `#[cfg(all(feature = "kernel-rlib", test))]`, and the `kernel-rlib`
feature is **default-OFF**:

| Element | Cite (live) | State |
|---|---|---|
| `finalization.rs` test module | `bebop2/delivery-domain/src/finalization.rs:172` | `#[cfg(all(feature = "kernel-rlib", test))]` |
| `intake.rs` test module | `bebop2/delivery-domain/src/intake.rs:294` | `#[cfg(all(feature = "kernel-rlib", test))]` |
| `hub_ring.rs` test module | `bebop2/delivery-domain/src/hub_ring.rs:94` | `#[cfg(all(feature = "kernel-rlib", test))]` |
| `lib.rs` `facade_tests` | `bebop2/delivery-domain/src/lib.rs:382` | `#[cfg(all(feature = "kernel-rlib", test))]` |
| feature default | `bebop2/delivery-domain/Cargo.toml:15` | `default = []` — **`kernel-rlib` OFF** |
| the four modules themselves | `lib.rs:373-380` | **`pub mod hub_ring/intake/pod/finalization` are each `#[cfg(feature = "kernel-rlib")]`** — the whole module, not only its tests |

**What the excluded tests actually assert (the safety surface):**

- **Split-brain / double-finalization** — `finalization.rs`, testing `PartitionMerge::reconcile` /
  `detect_conflict` (`finalization.rs:104-169`), the F46 runtime rule that *"if the mesh partitions and
  two hubs each finalize the same order to different terminal statuses, merging must NOT silently accept
  both"* (`finalization.rs:1-16`). Verified named tests:
  - `ac7_red_double_finalize_detected` (`finalization.rs:189`) — hub1 settles ord-555 → **Delivered**,
    hub2 → **Cancelled** in one merge window; asserts the conflict is **detected** and `reconcile`
    returns `Err` (quarantine), **never a silent winner**. This is the split-brain falsifier.
  - `ac7_green_convergent_finalize_merges` (`:213`) — two hubs agree on Delivered ⇒ merge accepts.
  - `ac7_green_tampered_chain_rejected` (`:225`) — a corrupted `prev_hash` fails the hash-chain even when
    statuses agree (tamper-evident).
  - `ac7_same_hub_repeat_is_convergent_not_conflict` (`:241`) — non-terminal→terminal advance is a legal
    lifecycle step, not a double-finalize.
- **Order intake against the REAL kernel Law** — `intake.rs`, e.g. `ac1_owner_signed_frame_folds_on_two_hubs`
  (`intake.rs:322`, cross-hub convergence via `admit_and_fold`) and `ac2_forged_pending_to_delivered_rejected_everywhere`
  (`:361`, a validly-signed but **illegal** Pending→Delivered jump rejected on every receiver via the
  kernel's `assert_transition`).
- **Hub-ring HRW ownership** — `hub_ring.rs`, the `ac11_*` suite (`:105-167+`): deterministic rendezvous
  owner, no-SPOF replica promotion, distinct replica sets.
- **WIRE→LAW→MONEY facade** — `lib.rs` `facade_tests` (`:383+`), building **real** Ed25519-signed
  capability frames + anchor-rooted delegation chains through `KernelFacade` and `dowiz_kernel::domain::place_order`.

**Consequence:** running the default `cargo test -p bebop-delivery-domain` compiles **none** of these
modules and reports green — the split-brain gate on the settlement path is invisible to the gate. The
tests are already written; only their execution is missing.

### 0.2 A3 — the bus fix is ALREADY WRITTEN, VERIFIED, and PRESENT IN THE WORKING TREE (absorb, do not re-implement)

The G-C1 defect R8 described is: `Portkey::publish` / `Mesh::publish` held the single `Arc<Mutex<Inner>>`
bus lock **across the entire subscriber-dispatch loop**, so (a) every publish serialized behind the
slowest handler and (b) any handler that re-entered the bus (`subscribe`/`publish`/`unsubscribe`)
re-locked the same **non-reentrant** `std::sync::Mutex` and **self-deadlocked**. The fix — `Box<dyn Fn>`
→ `Arc<dyn Fn>`, snapshot the handler `Arc`s under the lock, drop the guard, dispatch outside — is
captured verbatim in `/root/dowiz-perf-contention/docs/research/bebop-bus-G-C1-fix.patch` and is **already
applied in the live `/root/bebop-repo` working tree**:

| Element | Cite (live, verified this pass) | State |
|---|---|---|
| `portkey.rs` handler map | `crates/bebop/src/portkey.rs:40` | `handlers: HashMap<SubId, Arc<dyn Fn(&Envelope) + Send + Sync>>` (was `Box`) |
| `portkey.rs` publish | `crates/bebop/src/portkey.rs:95-113` | snapshot `Arc`s under lock → **guard dropped at `:106`** → dispatch loop outside the lock |
| `portkey.rs` regression tests | `:196` `publish_preserves_order_and_loses_no_dispatch`, `:220` `reentrant_handler_does_not_deadlock` | both present, under **plain `#[cfg(test)]`** (`:132`) |
| `zenoh.rs` handler map + publish | `crates/bebop/src/zenoh.rs:29`, `:88-111` (guard dropped `:104`) | same fix applied; `log` still written under the lock before dispatch |
| `zenoh.rs` regression test | `crates/bebop/src/zenoh.rs:170` `reentrant_handler_does_not_deadlock` | present |
| fan-out bench | `crates/bebop/benches/criterion.rs:58` `bench_portkey_publish_fanout`, group at `:82-87` | `portkey/publish_fanout_8subs` — happy-path regression guard |

The live source matches the patch's intent byte-for-byte. **Per the status record**
(MASTER-STATUS-LEDGER §0; SYNTHESIS-WAVE3-CLOSEOUT §2), the change **could not be committed** because
bebop HEAD sits in a pre-existing **C3** HARD-law-red state and `--no-verify` was correctly denied — so
the fix is **present in the working tree but unlanded through the hooks**, preserved as an applyable
patch. (This blueprint did **not** run git — per its planning-only constraint — so it states the commit
state from the cited ledger, and the *source* state from a live read.)

**Therefore P76's A3 work is landing, not writing.** The bebop lib was **443 tests green** with the fix
on the branch (S1/S3). The correctness win — re-entrant handlers no longer deadlock, publishes no longer
serialize — is proven by the two regression tests; the fan-out bench only guards the happy path.

### 0.3 CORRECTION — the NO-COURIER-SCORING red-line anchor is in the dowiz kernel, not bebop2/core

S1 §4 D-2 and R8 §1.2 cite `event_log.rs:22` for NO-COURIER-SCORING. Verified live: `bebop2/core/src/event_log.rs:22`
is `use alloc::vec::Vec;` — **not** the anchor. The real red-line comment is in the **dowiz kernel**:

> `/root/dowiz/kernel/src/event_log.rs:22-23`:
> `//! CI GUARD: NO-COURIER-SCORING — events carry an actor_pubkey (identity),`
> `//! never a score. The log is neutral, idempotent plumbing.`

This is the canonical anchor for the D-2 flag (§10.2). Trust the file, not the shorthand.

### 0.4 CORRECTION — the R8 "fix template" `llm-adapters/src/cache.rs:107-122` does not exist in bebop-repo (immaterial)

R8 §4.2 and S1 §3.1-A3 name `llm-adapters/src/cache.rs:107-122` as the lock/read/unlock/work template.
Verified: **`find /root/bebop-repo -name cache.rs` → no result.** That path is a **dowiz-repo** kernel-
adapter crate, not a bebop-repo file. Since the bus fix is **already written and applied** (§0.2), the
template is **immaterial** — the patch itself is the authoritative reference implementation of the
snapshot-under-lock discipline. Do not cite a path that does not resolve in the repo being edited.

### 0.5 The two halves sit on DIFFERENT paths — which is why their value differs

- **A2 → `bebop2/delivery-domain`** — the **real mesh product path** (the P13 delivery-on-protocol
  spine; the WIRE→LAW→MONEY settlement surface). Safety-critical; high value.
- **A3 → `crates/bebop`** — the **legacy dev-tooling TUI crate**, confirmed **off the mesh product path**
  (S1 §2 second reconciliation; S1 §6 E13; `portkey.rs:1-10`/`zenoh.rs:1-10` document both as "offline
  stand-in… NOT the network stack… the seam where a real mesh transport would plug in"). Latent value;
  the fix prevents a defect that ships only when a real handler does work or re-publishes.

This is the honest blast-radius framing R8 states and P76 preserves.

### 0.6 Why A2's four modules are feature-gated (the fact that decides the fix, §5)

`bebop2/delivery-domain/Cargo.toml` keeps the **default build dependency-free and offline-clean**
(`Cargo.toml:8-13`, MESH-01a): `bebop2-core` (Ed25519 + ML-DSA-65 + SHA3) is `optional = true` under
`[dependencies]` (`:38`, pulled in only by `kernel-rlib` at `:16`) and separately a **non-optional
`[dev-dependencies]`** (`:42`); the real kernel is `dowiz-kernel = { path = "../../../dowiz/kernel", …,
optional = true }` (`:25`). The `pub mod` declarations are `#[cfg(feature = "kernel-rlib")]`
(`lib.rs:373-380`) **precisely so the crypto + kernel deps stay out of the default graph.** This is the
load-bearing reason plain `#[cfg(test)]` is **not** the right lever (§5).

---

## 1. Prior-art / reuse map — adopt, don't invent (standard §2 item 19)

| Prior art | What it is | How P76 uses it — and what it does NOT take |
|---|---|---|
| **`bebop-bus-G-C1-fix.patch`** (already written + verified) | `Box`→`Arc`, snapshot-under-lock, dispatch-outside-lock, + 3 regression tests + 1 bench | **This IS the A3 implementation.** P76 absorbs and lands it verbatim. **NOT taken:** any re-derivation — re-writing verified crypto-adjacent concurrency code would re-introduce the exact risk the review already cleared (the B4/SSR-2020 lesson: green-tested ≠ safe; here it is already reviewed-green, so re-touching it is pure downside). |
| **snapshot-under-lock / "lock → read → unlock → work" critical-section discipline** | acquire, clone the minimal handles, release, then do the slow/re-entrant work outside | The patch's shape. The canonical in-repo exemplar R8 named (`llm-adapters/src/cache.rs`) lives in **dowiz**, not bebop-repo (§0.4) — so the **patch itself** is P76's in-tree template. **NOT taken:** a reader-writer lock, a channel dispatcher, or an async rewrite — over-engineering for an offline stand-in (ponytail); `Arc`-snapshot is the minimal viable fix. |
| **plain `#[cfg(test)]`** (the default Rust test convention, e.g. `portkey.rs:132`, `zenoh.rs:119`) | tests compiled in every `cargo test`, no feature flag | The bus regression tests already use it. **NOT taken for A2** — the delivery-domain modules genuinely need the `kernel-rlib` deps (§5), so plain `#[cfg(test)]` would break the offline-clean default build. |
| **CI feature-matrix leg** (standard cross-feature CI practice) | a CI job runs the suite under an explicit non-default feature set | **This IS the A2 mechanism** — one job runs `cargo test -p bebop-delivery-domain --features kernel-rlib`, making all four modules' tests execute with visible counts, **without** dragging crypto/kernel into the default graph. Mirrors how the kernel's own `pq`-feature tests are exercised in CI. |
| **`REGRESSION-LEDGER.md`** (`docs/regressions/`) | the permanent record of named regression tests | P76 adds one row per landed half (§9). |

**Net:** P76 **adds no dependency**, **invents no primitive**, and **writes no new fix logic** — it lands
one already-written patch and adds one CI job.

---

## 2. Scope — what P76 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P76 OWNS

1. **A2 — make the hidden `delivery-domain` tests provably execute** via a CI matrix leg
   `cargo test -p bebop-delivery-domain --features kernel-rlib`, with the split-brain / double-
   finalization test names and count **visible in CI output** (§4 M1). Owns the choice of mechanism
   (CI leg vs plain `#[cfg(test)]`) and decides it (§5).
2. **A3 — land the already-verified bus patch** (`portkey.rs`/`zenoh.rs`/`benches/criterion.rs`) through
   the hooks and push to `openbebop`, once the C3 gate clears (§4 M2). Owns the *landing*, not the code.
3. **Recording the C3/P85 landing precondition** as a named, operator-owned gate (§10.1, OD-3).
4. **Carrying the D-2 `reputation.rs` flag** as a scope note routed to the operator, without blocking
   A2/A3 on it (§10.2, OD-8).
5. The two REGRESSION-LEDGER entries (§9).

### 2.2 P76 does NOT own (anti-scope — prevents collision & scope-creep)

- **The bebop C3 ungated-keygen fix.** That is an open operator/council-gated crypto item predating this
  wave (SYNTHESIS-WAVE3-CLOSEOUT §4). P76 is *blocked by* it and *names* it; P76 does **not** resolve it,
  and does **not** propose an `--no-verify` bypass (never-bypass-human-gates).
- **Deciding `reputation.rs`.** Delete vs event-source is a governance ruling (OD-8) routed to the
  operator (§10.2). P76 neither deletes nor refactors it.
- **Re-implementing the bus fix.** It is written, applied, and reviewed-green (§0.2). Re-touching it is
  out of scope and pure risk.
- **Any performance rewrite of `crates/bebop`.** The legacy TUI crate's Dijkstra/CH routing etc. are
  out of scope (S1 §6 E13). P76 touches `crates/bebop` **only** to land the already-applied bus patch.
- **The bebop money-ledger port (G-E1) / `budget`/`intake` test-bar raises (D7).** Separate items
  (S1 §3.4 D8, R8 §5 rows 3/6/7); not P76.
- **P85 (NTT remediation), P78 (bebop MerkleDigest/hub_ring perf), P82 (bebop bench expansion).**
  Sibling bebop-lane blueprints; P78 is sequenced **after** P76 in the same repo to avoid CI churn
  overlap (S1 §5).

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs (in tree, verified live):** the four gated modules + `Cargo.toml` (§0.1, §0.6); the
applied bus fix + patch (§0.2); the bebop CI harness that would host the new matrix leg; the dowiz kernel
at `../../../dowiz/kernel` (path-linked under `kernel-rlib`).

**Landing precondition (operator-owned):** bebop **C3** resolution **and** **P85** closure — *"P85 + C3
resolution precede the entire bebop lane"* (MASTER-STATUS-LEDGER §3, OD-3). Until then A2's CI leg can be
*written* but a bebop commit that adds it **cannot land through the hooks**, and A3's patch **cannot be
committed**.

**Consumers:** the bebop CI gate (gains real coverage of the settlement-safety tests); any future reader
who runs `cargo test` and would otherwise be misled by a green that skipped the split-brain gate.

### 2.4 Honest reconciliation with the source docs (standard §2 item 6)

S1 §3.1 left the A2 mechanism as an **either/or** ("plain `#[cfg(test)]` … or a CI matrix leg"). P76
**decides** it (§5) on verified ground truth: plain `#[cfg(test)]` would break the crate's offline-clean
default-build invariant, so the **CI matrix leg** is correct. This is a *strengthening* of the source, not
a divergence — the source explicitly delegated this call to the blueprint (S1 §4: *"engineering decisions
the blueprints decide … `#[cfg(test)]` vs CI-matrix-leg for A2 (P76)"*).

---

## 3. Predefined items & constants — named BEFORE implementation (standard §2 item 4)

This blueprint is a **test-visibility un-gate + a verified-patch landing** — it deliberately introduces
**no new Rust domain types** (inventing types here would be the over-engineering the ponytail rule
forbids). The named artifacts are the CI job, the count assertion, and the ledger rows:

```yaml
# bebop-repo CI (the workflow that runs the bebop test gate) — NEW matrix leg.
# Name is the wire contract other docs cite; keep it stable.
job: delivery-domain-kernel-rlib
run: cargo test -p bebop-delivery-domain --features kernel-rlib -- --nocapture
# Acceptance signal (machine-checkable): the job log MUST contain each of these
# test names, and the summary line MUST show a non-zero pass count for the crate:
must_execute:
  - ac7_red_double_finalize_detected          # split-brain falsifier (finalization.rs:189)
  - ac7_green_convergent_finalize_merges       # finalization.rs:213
  - ac7_green_tampered_chain_rejected          # finalization.rs:225
  - ac1_owner_signed_frame_folds_on_two_hubs   # intake real-kernel-Law (intake.rs:322)
  - ac2_forged_pending_to_delivered_rejected_everywhere  # intake.rs:361
  - ac11_no_spof_owner_removal_promotes_replica          # hub_ring HRW (hub_ring.rs:127)
count_guard: "delivery-domain test result: ok. N passed" with N > 0   # a zero-test 'ok' is a FAIL, not a pass
```

```text
# docs/regressions/REGRESSION-LEDGER.md — TWO new rows (bebop-repo):
P76-A2  delivery-domain kernel-rlib CI leg    → split-brain/double-finalization tests provably run
P76-A3  bus snapshot-under-lock (portkey/zenoh) → reentrant_handler_does_not_deadlock permanent guard
```

**No magic numbers, no stringly-typed placeholders.** The only "value" is the job name, pinned above.

---

## 4. Build items — spec → RED test → code (standard §2 items 2, 3, 5)

Two items. Neither writes fix logic; both are falsifiable.

### 4.1 M1 — un-gate: make the delivery-domain safety tests provably execute (CI matrix leg)

- **Spec.** Add a CI job `delivery-domain-kernel-rlib` (§3) to the bebop test workflow that runs
  `cargo test -p bebop-delivery-domain --features kernel-rlib -- --nocapture`. The job's log MUST list the
  `must_execute` test names (§3) and MUST show a **non-zero** pass count for the crate. The **default**
  `cargo test` job is left unchanged (the modules stay `#[cfg(feature = "kernel-rlib")]` so the offline-
  clean default build is preserved, §5). **No source file in `bebop2/delivery-domain` changes** —
  the tests are already written; only their execution is added.
- **RED (the deliberately-broken-reconcile falsifier).** Before wiring the job, prove the tests are truly
  falsifiable and truly not running today, in two moves:
  1. **They don't run today:** `cargo test -p bebop-delivery-domain` (default features) → the summary
     shows **0** of the `ac7_*`/`ac1_*`/`ac11_*` tests; grepping the log for `ac7_red_double_finalize_detected`
     yields nothing. *(This is the silent-green being exposed.)*
  2. **They are real:** temporarily break `PartitionMerge::detect_conflict` (`finalization.rs:113-137`) so
     it returns `None` on a genuine terminal disagreement, then run
     `cargo test -p bebop-delivery-domain --features kernel-rlib` → `ac7_red_double_finalize_detected`
     goes **RED** (`"split-brain must be detected"` / `"double-finalize must NOT merge"` assertions fire).
     Revert the break. This proves the CI leg has real signal, not a vacuous green.
- **Code.** Add the one CI job. (Optionally add a `make test-delivery-domain` / `cargo xtask` convenience
  alias mirroring it, so a local run is one command — non-load-bearing.)
- **GREEN.** The new CI leg runs, its log contains the `must_execute` names, and the count guard passes
  (`N > 0`). The default job stays green **and dependency-free**. A future regression that re-gates or
  deletes a split-brain test now shows up as a **dropped test count** in the CI leg (§8 falsifier).

### 4.2 M2 — absorb + land the already-verified bus patch (NOT a re-implementation)

- **Spec.** The fix is `bebop-bus-G-C1-fix.patch` (§0.2), **already applied in the working tree**:
  `portkey.rs` / `zenoh.rs` handler maps are `Arc<dyn Fn>`; both `publish` fns **snapshot handler `Arc`s
  under the lock, drop the guard, and dispatch outside it** (`portkey.rs:95-113`, `zenoh.rs:88-111`);
  delivery order + fan-out count + the under-lock delivery log are preserved. P76's job is to **land** it
  through the hooks and push to `openbebop` — **once the C3 gate clears** (§10.1). **Do not modify the
  patch.**
- **RED (already written; would fail on the pre-fix shape).** The regression tests carried by the patch
  are the falsifiers — they go RED (hang / assert-fail) on the old lock-across-dispatch code and GREEN on
  the fix:
  - `reentrant_handler_does_not_deadlock` (`portkey.rs:220`, `zenoh.rs:170`) — a handler that re-publishes
    from inside its own dispatch **hangs forever** under the old non-reentrant-Mutex-across-dispatch shape;
    completes under snapshot-under-lock. (A RED here is a *deadlocked test run*, the sharpest possible
    falsifier.)
  - `publish_preserves_order_and_loses_no_dispatch` (`portkey.rs:196`) — asserts all three subscribers
    fire in subscription order (`n == 3`, order `[1,2,3]`), proving the `Arc`-snapshot loses no dispatch
    and preserves ordering.
- **Code (landing, not writing).** With C3 resolved: run the existing bebop test + hook gate, confirm the
  **443-green** state holds, commit the already-applied change (message crediting G-C1 / A3 / this
  blueprint), and **push to `openbebop`** (never `origin`, §RULE). If for any reason the working tree has
  drifted from the patch, re-apply `bebop-bus-G-C1-fix.patch` cleanly rather than hand-editing.
- **GREEN.** `cargo test -p bebop` green (incl. both regression tests + the existing bus tests); the
  `portkey/publish_fanout_8subs` bench runs; the change is committed and pushed to `openbebop`.

---

## 5. The A2 engineering decision the blueprint makes (CI matrix leg — with reasoning)

**Decision: a CI matrix leg (`cargo test -p bebop-delivery-domain --features kernel-rlib`), NOT plain
`#[cfg(test)]`.** Reasoning from verified ground truth (§0.1, §0.6):

1. **The four modules are `#[cfg(feature = "kernel-rlib")]`-gated at the `pub mod` site** (`lib.rs:373-380`),
   not merely at the test module. They compile **only** under the feature.
2. **They compile only under the feature because they depend on `bebop2-core`** (Ed25519/ML-DSA/SHA3) —
   e.g. `finalization.rs:36 use bebop2_core::hash::sha3_256;` at module level, and the intake/facade tests
   additionally exercise the **real dowiz kernel Law** (`dowiz_kernel::domain::place_order`,
   `assert_transition`, `OrderStatus`). Both `bebop2-core` and `dowiz-kernel` are **`optional = true`**
   deps pulled in **only** by `kernel-rlib` (`Cargo.toml:16,25,38`).
3. **The gating is a deliberate invariant, not an accident** (`Cargo.toml:8-13`, `lib.rs:363-372`,
   MESH-01a): the **default build must stay dependency-free and offline-clean**. Converting the modules'
   tests to plain `#[cfg(test)]` would drag `bebop2-core` (crypto) and/or the whole kernel into the
   default dependency graph — **breaking the very invariant the feature exists to protect.**
4. **A partial un-gate is worse.** `finalization.rs` + `hub_ring.rs` *tests* happen to use only
   `bebop2-core` (a dev-dependency), so one could imagine un-gating just those — but their **modules** are
   gated for the offline-clean reason (they compile the crypto hash at module scope), and `intake` +
   `facade_tests` genuinely need the kernel. Splitting the gate per-module adds complexity and still can't
   put intake/facade in the default gate. The **single** correct lever that runs **all four** with visible
   counts and preserves the invariant is the **feature-on CI leg**.

**Conclusion:** the CI matrix leg is the minimum-viable, invariant-preserving fix. It makes every hidden
safety test provably execute (with names + counts in the CI log) while the default `cargo test` stays
dependency-free. This is a stronger result than the source's undecided either/or — and it is decided from
source, not preference.

---

## 6. Adversarial self-check — real effort to find the failure mode (standard §2 items 3, 5)

- **"Is the CI leg vacuously green?"** No — guarded two ways: the **count guard** (`N > 0`; a zero-test
  `ok` is a FAIL, §3), and the **must_execute name list** (the specific split-brain test names must appear
  in the log). The M1 RED step deliberately breaks `detect_conflict` and confirms
  `ac7_red_double_finalize_detected` turns RED, proving live signal.
- **"Could the feature build silently skip the kernel tests even with `--features kernel-rlib`?"** The
  name-list guard catches this: if `ac1_*`/`ac2_*` (the intake real-kernel-Law tests) don't appear, the
  job fails. This also catches a broken `../../../dowiz/kernel` path link.
- **"Does un-gating change any behavior on the product path?"** No. M1 adds **no source change** to
  `bebop2/delivery-domain`; it only *runs* existing tests. The default build/graph is untouched (§5).
- **"Could landing the bus patch regress the happy path?"** Guarded by the existing bus tests
  (`publish_reaches_subscriber`, `mesh_fanout_to_all_nodes`, `unsubscribe_stops_delivery`,
  `leave_stops_node_receiving`) + `publish_preserves_order_and_loses_no_dispatch` (order/fan-out
  preserved) + the `publish_fanout_8subs` bench (no throughput regression). The delivery **log** is still
  written under the lock before dispatch (`portkey.rs:98`, `zenoh.rs:98-100`), so log-order semantics are
  unchanged.
- **"Does dispatching outside the lock introduce a data race or a lost update?"** No new shared mutable
  state is touched outside the lock — only the immutable `Arc<dyn Fn>` handles (cloned under the lock) are
  invoked; `Send + Sync` bounds are unchanged. A handler that re-enters the bus now takes a fresh lock in
  its own `publish` (which is the whole point) rather than deadlocking on a held one.
- **"Is there a hidden ordering hazard in the re-entrant case?"** The re-entrant `publish` runs *after*
  the outer guard is dropped, so its own snapshot sees the current subscriber set; this is strictly more
  correct than the old hang. The regression test asserts the cascade completes with the expected count.
- **"Am I inflating A3's importance?"** No — stated repeatedly: `crates/bebop` is a legacy offline
  stand-in off the product path (§0.5); the fix prevents a **latent** deadlock and lands already-done
  work. No production-deadlock claim, no speedup number.
- **"Could C3 be quietly bypassed to land faster?"** No — that is an operator/council gate
  (never-bypass-human-gates); P76 records it (§10.1) and waits.

---

## 7. AI/system-hazard safety, argued from structure (standard §2 item 6)

- **A2 makes an unsafe state *observable*, not merely asserted.** The hazard class is *a settlement-safety
  regression shipping green because its test never ran*. After M1, the split-brain / double-finalization
  test is a **compile-and-run gate in CI with a machine-checked name+count** (§3) — so a future edit that
  weakens `PartitionMerge::reconcile` (e.g. re-introduces last-write-wins) turns the gate **RED**, and a
  future edit that re-hides the test turns the **count guard** RED. The unsafe state is caught at CI time,
  not discovered in a merged partition in production.
- **A3 removes a liveness hazard by construction.** The deadlock was *structural* — a non-reentrant
  `std::sync::Mutex` held across a call that could re-take it. The fix makes the guard's lifetime end
  **before** any handler runs (`portkey.rs:106`, `zenoh.rs:104`), so "a handler holding the bus lock while
  re-entering the bus" is **unrepresentable** in the new control flow. The `reentrant_handler_does_not_deadlock`
  test is the executable proof.

Both are grounded in the finite-anchored-authority / fail-closed doctrine: the safety property is enforced
by the *shape of the code and the gate*, not by a prose promise.

---

## 8. Benchmarks + telemetry (standard §2 item 10)

**A2's "win" is test VISIBILITY, not speed — stated honestly.** There is no latency number to report; the
deliverable is *coverage*. The falsifier is behavioral, not numeric:

> the split-brain/double-finalization test **count appears in CI output**, and a **deliberately-broken
> `reconcile`** makes `ac7_red_double_finalize_detected` go **RED** (§4.1 RED step). If either fails, M1
> is not done.

**A3 ships the fan-out bench already in the patch** (`crates/bebop/benches/criterion.rs:58`,
`portkey/publish_fanout_8subs` — 8 subscribers, one publish snapshots + dispatches to all). Its role is a
**regression guard on the single-publish happy path**, not a speedup claim. **No throughput-win number is
manufactured** — the honest statement of the win is exactly: *concurrent publishes no longer serialize
behind the bus lock, and re-entrant handlers no longer deadlock* (proven by the unit tests, not the bench).

**Telemetry:** none added — both bus files are offline in-process stand-ins with a deterministic delivery
log already used for test assertions (`Portkey::deliveries`, `Mesh::delivery_count`); a metrics hook here
would be over-engineering for a legacy crate (ponytail). The CI leg *is* the regression telemetry for A2.

---

## 9. DoD — falsifiable, RED→GREEN, machine-checkable (standard §2 item 2)

| # | Done when… | Falsifier (RED test / check) |
|---|---|---|
| D1 | the CI leg `delivery-domain-kernel-rlib` runs `cargo test -p bebop-delivery-domain --features kernel-rlib` and its log contains every `must_execute` name (§3) with a **non-zero** crate pass count | grep the CI log for `ac7_red_double_finalize_detected` etc.; **count guard** `N > 0` |
| D2 | the excluded tests are proven real (not vacuous): a deliberately-broken `detect_conflict` makes `ac7_red_double_finalize_detected` RED under the feature | the M1 RED step (§4.1); revert after |
| D3 | the **default** `cargo test -p bebop-delivery-domain` stays green **and** dependency-free (no crypto/kernel pulled into the default graph) | `cargo tree -p bebop-delivery-domain` (no `bebop2-core`/`dowiz-kernel` in default) still holds |
| D4 | the bus fix is landed: `portkey.rs`/`zenoh.rs` use `Arc<dyn Fn>` + snapshot-under-lock + dispatch-outside, committed through the hooks | `reentrant_handler_does_not_deadlock` (portkey + zenoh), `publish_preserves_order_and_loses_no_dispatch` GREEN; `cargo test -p bebop` green |
| D5 | the fan-out bench is present and runs | `cargo bench -p bebop -- portkey/publish_fanout_8subs` executes |
| D6 | the landed bebop commit is pushed to **`openbebop`** (never `origin`) | `git remote -v` shows the push target = openbebop (operator/lead verifies at land time) |
| D-GATE | the **C3 + P85 landing precondition** is recorded and respected — no `--no-verify` bypass | OD-3 in MASTER-STATUS-LEDGER §5; bebop pre-commit hook green at land time |
| D-FLAG | the **D-2 `reputation.rs`** flag is carried to the operator as a scope note, A2/A3 NOT blocked on it | OD-8 present; §10.2 in this doc |
| D-LEDGER | two REGRESSION-LEDGER rows exist (P76-A2, P76-A3) | `docs/regressions/REGRESSION-LEDGER.md` grep |
| D-NOREG | existing bebop + delivery-domain default tests stay green (no regression) | `cargo test -p bebop`, `cargo test -p bebop-delivery-domain` |

---

## 10. The landing gate + the carried flag (the two operator-owned items)

### 10.1 Landing gate — bebop C3 + P85 (OD-3) — RECORD, do not resolve

Both P76 halves are **commit-blocked** until the operator resolves the pre-existing bebop **C3**
ungated-keygen HARD-law red state (`scripts/ci-no-ungated-keygen.sh` fails on ungated constant-seed
`pq_dsa::keygen` / `pq_kem::keygen_internal` — **unrelated** to either P76 change; a clean worktree at
HEAD with zero crypto edits still trips it — SYNTHESIS-WAVE3-CLOSEOUT §2/§4). Per the master sequence,
**"P85 + C3 resolution precede the entire bebop lane"** (MASTER-STATUS-LEDGER §3), because the same red
state freezes **all** hook-respecting bebop commits — including M1's CI-workflow commit and M2's bus
landing. **P76 records this gate and does not touch it.** Resolution paths (operator's call, OD-3): (a)
fix/gate the keygen so `ci-no-ungated-keygen.sh` passes, or (b) an explicit, recorded `--no-verify`
ruling for these two changes. **Default if unruled:** the CI leg is written but unlanded; the bus patch
stays a file + a dirty working tree. **P76 never self-authorizes a bypass** (never-bypass-human-gates).

### 10.2 Carried flag — D-2 `reputation.rs` (OD-8) — SURFACE, do not decide

`crates/bebop/src/reputation.rs` is **courier/node scoring**: `TrustRecord { deliveries, suspensions }`
(`:20-25`), a `score()` in `[0,1]` (`:69`) feeding a `risk_premium()` "cost surface" that the router
consumes (`:85`), plus a recency `decay()` (`:55`). Its own doc calls it *"the node-trust primitive"* and
*"the network's trust graph is the asset."* This **directly collides** with the canonical NO-COURIER-SCORING
red line (**`/root/dowiz/kernel/src/event_log.rs:22-23`**, §0.3): *trust is a signed **capability**, never a
reputation **score***. Two facts sharpen the flag: it is a **public module** (`crates/bebop/src/lib.rs:68`)
but has **no runtime caller** — only a comment mention in `matcher.rs:311` (grep-verified, §0 method); and
it lives in the **legacy** crate (§0.5). **P76 does NOT decide it.** Options for the operator (OD-8):
(a) **delete** per the red line (it is unwired dead-ish code, the low-risk default the canonical stance
suggests), or (b) **event-source** it (append-only POD/suspension events, derived counters) *if node
scoring is ever ruled admissible*. Either way the A2/A3 fix is trivial and **unblocked** — `reputation.rs`
is **not in P76's edit set**. Surface, route, move on.

---

## 11. Cross-cutting obligations (standard §2 items 6, 8, 9, 11–16, 20)

- **Isolation / bulkhead (item 11).** A2 changes **only CI**, never the crate's build graph — the offline-
  clean default build is the bulkhead, preserved by keeping the modules feature-gated (§5). A3's failure
  mode is contained to the legacy `crates/bebop` stand-in; it has **no path** to the product mesh, ledger,
  or money surfaces (different crate, §0.5).
- **Error-propagation / smart index (item 14).** The bug classes P76 addresses become **CI-time**
  failures: *a hidden safety test* → the CI leg's name+count guard (a re-gated or deleted split-brain test
  drops the count → RED); *a re-introduced lock-across-dispatch deadlock* → `reentrant_handler_does_not_deadlock`
  hangs → RED. Not runtime surprises.
- **Rollback / self-healing as math (item 13).** **Self-termination** — the bus fix makes "a handler
  holding the bus lock while re-entering the bus" an **unrepresentable** control-flow state (guard dropped
  before dispatch, §7), a hard invariant, not a supervisor's choice. **Self-healing is NOT claimed** (a
  dropped in-process delivery is not error-corrected — claiming it would be false). Rollback for A2 is
  trivial (remove the CI job); for A3 it is `git revert` of the landed commit or dropping the patch.
- **Mesh awareness (item 12).** A2's tests are on the **mesh product path** (`bebop2/delivery-domain` P13
  spine — the WIRE→LAW→MONEY fold); the split-brain rule is a **partition-merge / gossip-convergence**
  property (`finalization.rs:1-16`). A3 is **not** on any wire path — `crates/bebop`'s bus is an in-process
  offline stand-in (`portkey.rs:9-10`), node-local, no transport, no payload budget.
- **Schemas & scaling axis (item 8).** No schema is added. The scaling axis touched is *test count in CI*
  (grows with the delivery-domain suite; the name+count guard scales with it) and *subscriber fan-out per
  publish* for the bus (`Vec<Arc<dyn Fn>>` snapshot, O(subscribers) clones — the fan-out bench pins the
  n=8 shape; it changes shape only if a stand-in ever holds ≳10⁴ subscribers, which it never will as a
  legacy TUI stand-in — stated, not timeless).
- **Living-memory awareness (item 15).** **N/A, honestly** — this blueprint adds no persisted data and no
  temporal/topological access pattern; it un-gates tests and lands a concurrency patch.
- **Tensor/spectral (item 16).** **N/A, honestly** — no linear-algebra kernel is involved; forcing
  `spectral.rs` here would be pure over-engineering (ponytail).
- **Linux engineering discipline (item 9).** **REINFORCES** the fail-closed / lock-then-drop-then-work
  critical-section discipline (the bus fix) and the RED→GREEN falsifiable-test culture (making the hidden
  REDs actually run). **ALREADY-EQUIVALENT** on feature-gated builds (the crate already uses Cargo feature
  gating correctly; P76 just adds the CI leg that exercises the gate). **EXTENDS** the CI matrix to a
  feature it did not cover. **DOES-NOT-TRANSFER** — no daemon, no new subsystem.
- **Hermetic principles (item 20).** **Polarity / no-middle:** a test either runs in a gate or it does
  not — P76 removes the false-green middle where a safety test *appears* covered but is excluded. **Cause &
  Effect:** the bus fix makes the *cause* of a delivery (a publish) complete before any *effect* (a
  handler, possibly re-publishing) runs, so cause never blocks on its own effect. **Correspondence:** the
  CI gate's green now *corresponds* to the tests actually executing — "as reported (green), so run
  (executed)" — the report is self-describing, not asserted.

---

## 12. Standard-compliance map (all 20 points — standard §2)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (every cite verified this pass; 3 corrections: §0.2 patch-applied, §0.3 anchor path, §0.4 missing template) |
| 2 | Falsifiable DoD | §9 (D1–D-NOREG, each a RED→GREEN check) |
| 3 | Spec→test→code, event-driven | §4 (spec-first per M; the split-brain tests assert on merge-window *event sequences* — two finalizations → conflict) |
| 4 | Predefined types & constants | §3 (the CI job name + count guard + ledger rows; no gratuitous new types — honest) |
| 5 | Adversarial/breaking tests | §4.1 RED (deliberately-broken `reconcile`), §4.2 RED (deadlock-on-old-shape), §6 (self-attack) |
| 6 | Hazard-safety from structure | §7 (unrepresentable lock-across-dispatch; CI-observable split-brain regression) |
| 7 | Links to docs & memory | §13 |
| 8 | Schemas with scaling axis | §11 (test-count / fan-out axes; no schema) |
| 9 | Linux engineering discipline | §11 (REINFORCES/ALREADY-EQUIVALENT/EXTENDS/DOES-NOT-TRANSFER) |
| 10 | Benchmarks + telemetry | §8 (fan-out bench = regression guard; A2 win = visibility, no manufactured speedup) |
| 11 | Isolation / bulkhead | §11 (CI-only for A2; legacy-crate containment for A3) |
| 12 | Mesh awareness | §11, §0.5 (A2 on product path; A3 off it) |
| 13 | Rollback/self-heal as math | §11 (self-termination = unrepresentable deadlock; self-healing NOT claimed) |
| 14 | Error-propagation / smart index | §11 (CI name+count guard; deadlock-test guard) |
| 15 | Living-memory awareness | §11 (N/A, honestly) |
| 16 | Tensor/spectral where applicable | §11 (N/A, honestly) |
| 17 | Regression tracking | §9 D-LEDGER (two REGRESSION-LEDGER rows) |
| 18 | Clear worker instructions | §13 |
| 19 | Reuse-first, upgrade-if-needed | §1 (absorb the patch; CI leg not new machinery), §2.4 (decide the source's either/or), §5 |
| 20 | Hermetic principles | §11 (polarity / cause-effect / correspondence) |

---

## 13. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `docs/research/OPUS-PERF-BESTPRACTICES-PROPAGATION-2026-07-18.md` ("R8") — §3.2 G-T1 (hidden tests),
  §4.2 G-C1 (bus lock), §1.2 G-E2 (`reputation.rs` = event-sourcing gap **and** NO-COURIER-SCORING
  divergence).
- `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` (S1) — §3.1 A2/A3 (Tier-A), §4 D-2, §5 W0 wave table
  (P76 = Wave 0 bebop lane; P78 sequenced after it), §2 second reconciliation + §6 E13 (`crates/bebop`
  = legacy TUI crate off the mesh product path).
- `SYNTHESIS-WAVE3-CLOSEOUT-2026-07-18.md` — §2 (the bus fix "FIXED + VERIFIED, COMMIT-BLOCKED" row;
  443 tests green), §4 (the C3 explanation verbatim).
- `MASTER-STATUS-LEDGER-2026-07-19.md` — §3 (P85 + C3 precede the entire bebop lane), §4 item 2 (P76
  absorbs the patch, carries D-2 without blocking), §5 OD-3 (C3 / `--no-verify` ruling) + OD-8 (D-2).
- `/root/dowiz-perf-contention/docs/research/bebop-bus-G-C1-fix.patch` — the authoritative A3 template.
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract).
- Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`.
- Memory: `MEMORY.md` ("BEBOP `origin` REMOTE ARCHIVED" → push to `openbebop`), `never-bypass-human-gates-2026-06-29.md`,
  `worktree-remote-push-collision-avoidance-2026-07-18.md`, `verified-by-math-2026-07-07.md`.

**Existing code this blueprint touches (exact targets, bebop-repo — NOT dowiz):**
- **NEW (CI only)** the bebop test workflow — add the `delivery-domain-kernel-rlib` matrix leg (§3, §4.1).
  **No source change in `bebop2/delivery-domain`.**
- **LAND (already applied, do not modify)** `crates/bebop/src/portkey.rs`, `crates/bebop/src/zenoh.rs`,
  `crates/bebop/benches/criterion.rs` — the G-C1 patch (§0.2, §4.2).
- **ADD** two rows to `docs/regressions/REGRESSION-LEDGER.md` (§3).
- **DO NOT TOUCH** `crates/bebop/src/reputation.rs` (carried flag, §10.2), the bebop C3 keygen surface
  (operator-gated, §10.1), or anything else in `crates/bebop` (legacy, out of scope).

**For the worker with zero session context — exact acceptance path:**
1. **Confirm the landing gate is open.** Do NOT commit anything bebop-side until the operator has resolved
   **C3** (or recorded an explicit `--no-verify` ruling) **and P85 has closed** (OD-3). If it is not open,
   *write* the CI leg locally, verify it, and STOP — report "landing-blocked on C3/P85." Never self-bypass.
2. **M1 (A2):** run `cargo test -p bebop-delivery-domain` (default) and confirm the `ac7_*`/`ac1_*`/`ac11_*`
   tests are **absent** from the summary (the silent-green). Then run
   `cargo test -p bebop-delivery-domain --features kernel-rlib -- --nocapture` and confirm they run.
   Do the RED falsifier (break `detect_conflict`, watch `ac7_red_double_finalize_detected` go RED, revert).
   Add the CI job (§3). Verify `cargo tree` shows the default graph unchanged (no crypto/kernel).
3. **M2 (A3):** confirm `portkey.rs`/`zenoh.rs`/`benches/criterion.rs` already carry the fix (§0.2); if the
   tree drifted, re-apply `bebop-bus-G-C1-fix.patch` cleanly (never hand-edit). Run `cargo test -p bebop`
   (incl. both `reentrant_handler_does_not_deadlock` + `publish_preserves_order_and_loses_no_dispatch`) and
   `cargo bench -p bebop -- portkey/publish_fanout_8subs`. Commit through the (now-green) hooks and
   **push to `openbebop`**.
4. Add the two REGRESSION-LEDGER rows (§3).
5. **Carry, do not decide, D-2:** ensure `reputation.rs` is surfaced to the operator as OD-8; do not edit
   it; do not block A2/A3 on it.
6. Anti-scope: do not re-write the bus fix; do not touch the C3 keygen; do not optimize `crates/bebop`;
   do not pull crypto/kernel into the delivery-domain **default** build (the CI leg is the only lever).
