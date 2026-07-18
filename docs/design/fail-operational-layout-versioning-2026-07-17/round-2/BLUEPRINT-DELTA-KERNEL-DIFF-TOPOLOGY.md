# ROUND-2 — Delta-Kernel / Adapter-as-Diff-Generator Topology (Part 4 adjudication) (2026-07-17)

> **Status: design blueprint, no code, no commits.** Adjudicates `00-SOURCE-DIALOGUE.md` Part 4
> (`:234-265` — "Adapter as Diff Generator": kernel state read-only to the adapter, adapter
> writes patches into a separate staging buffer, kernel atomically integrates the diff;
> "zero-cost air-gap") against (a) the kernel's actual event-sourcing code (`kernel/src/event_log.rs`,
> live-read in full this pass), (b) Fable-B's CSC-LAW three-layer containment
> (`BLUEPRINT-SELF-CERTIFYING-BRIDGE-CONTAINMENT.md`), (c) Fable-D's platform-reality findings
> (`BLUEPRINT-MMU-ISOLATION-HEADER-STRUCT-RECONCILIATION.md` §1-§2), and (d) the mesh-masterwork
> verdict ledger items #44/#59/#107/#110/#127/#154
> (`../../bebop2-mesh-tensor-hermetic-2026-07-17/BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md`).
>
> **Operator ruling on Part 4 (verbatim, binding):** "ядро має працювати з дельтами змін - а не
> станом. Досліди детальніше, що ядра і дельт я переконаний" — the kernel must work with deltas
> of change, not state; research deeper; about kernel-and-deltas I am convinced. Same weight as
> the FEC and self-cert rulings. The job of this document is to ground that conviction in what
> is actually true of this codebase and these platforms — including where it is *already true in
> shipped code*, which turns out to be most of it.
>
> **Part 5** (`00-SOURCE-DIALOGUE.md:270-320`, appended 2026-07-18, direct continuation, **no
> operator ruling attached**) poses four further mechanism questions — stateful vs stateless
> adapters, dynamic squashing, squash-validate vs atomic-rollback, snapshot-checkpointing —
> plus a closing 5-point stack and a "Git for real-time memory" analogy. Adjudicated at the
> same depth in §10; nothing there is treated as ratified beyond what prior rulings already
> cover.

---

## 0. TL;DR — six verdicts

1. **The conviction is correct — and at the kernel level it is already the built architecture,
   not a proposal.** The kernel's own state-transition model is event-sourced end to end:
   content-addressed delta events (`MeshEvent`), decide-before-persist commit
   (`commit_after_decide`), hash-chained log, idempotent replay, integrity walk
   (`event_log.rs`, §2.1 below, all live-read). The mesh sync layer already ships deltas, not
   snapshots (masterwork #59 ALREADY-EQUIVALENT, `anti_entropy.rs::diff` re-verified live this
   pass). "Ядро працює з дельтами" is a description of `kernel/src/event_log.rs`, not a change
   request.
2. **The three-way split (§2):** *already-built* = kernel-internal event-sourcing + delta sync +
   decide/drift-gated atomic commit; *already-adopted this round* = the adapter-facing memory
   topology (WASM no-pointer containment = CSC Layer 1; Fable-D's Part-3 ADOPT-EQUIVALENT covers
   the same claim in MMU vocabulary) + the ingest-gate integration discipline (CSC Layer 2,
   blueprint-stage); *genuinely new in Part 4* = a small, nameable residue — the patch vocabulary
   (`DeltaPatch`/`PatchOp`) at the adapter boundary, the explicit read-side contract
   (`KernelStateView`, a real cost decision Part 4 prices at zero), one new fault arm, and the
   delta-determinism caveat law (§7).
3. **Relation to CSC-LAW (§3): answer (b), with a sharp edge.** Part 4's memory-permission
   topology is not an *additional* layer to combine with CSC-LAW — it **is CSC-LAW's Layer 1**
   (spatial containment), which the WASM tier already implements more strongly than Part 4 asks
   (the adapter cannot even *read* kernel memory without an explicit grant, let alone write).
   And it makes **no part of CSC's machinery unnecessary**: Part 4's "no checks needed" claim
   conflates *spatial bounds checks* (genuinely free by construction) with *content validation
   at ingest* (still mandatory — Part 4's own closing line concedes it: "ви валідуєте лише
   результат (дифф)"). Layer 2 (sealed `BridgeResult` ingest gate) IS the mechanism by which
   "kernel integrates the diff on its own terms"; remove it and the topology has no integration
   discipline.
4. **Zero-cost honesty check (§4):** the **write-side** air-gap is genuinely zero-cost on the
   real platforms — in WASM no write-capable (or any) pointer to kernel state exists by
   construction. The **read-side** is *not* free and Part 4 silently assumes it is: WASM linear
   memory does NOT give the guest read access to host state; the kernel must either copy the
   needed state slice into the guest (O(view) memcpy per epoch — small for this codebase's
   frame-sized workloads, unbenchmarked) or expose per-call host-read imports (chattier, each an
   auditable grant). Genuine zero-copy read-only sharing exists only on the hub tier (sealed
   memfd / virtio-shm mapped RO — Fable-D §1.4), never on phones. So the trade is not
   "bounds-checks vs nothing"; it is "bounds-checks (already ~free on 64-bit JIT via guard
   pages; interpreter-cost on iOS regardless) vs copy-in (small, O(view), needs a number only
   when a state-reading adapter exists)".
5. **RC-2-broad (§6): NOT closed — the parent hypothesis is confirmed.** Memory topology
   constrains *where* bytes can be written, never *whether* they are semantically correct. A
   well-formed, canonical, in-bounds diff of the wrong content is integrated exactly as before.
   Two honest narrowings that are reach-improvements, not correctness-improvements: a delta
   names its write targets, so unnamed state is provably untouched (blast-radius shrinks; CSC
   T4's residual gap gets smaller *in area*, not closed), and canonical content-addressed deltas
   make the N-version/round-trip-witness closure mechanisms cheaper to run (a 32-byte id
   compare). CSC T4 (`residual_semantic_gap_pinned`) survives Part 4 unchanged and must not be
   retired.
6. **The determinism claim ("дельти простіше порівняти, ніж загальний стан") (§7): true under
   two conditions, and actively misleading without them.** Delta-equality implies state-equality
   **only if** (i) deltas have one canonical encoding (built: content-address + framing
   hard-reject) and (ii) apply/fold is bit-deterministic. Condition (ii) is exactly what this
   session's determinism findings threaten: `householder.rs:29-75` runtime-dispatches FMA (same
   binary, different bits across FMA/non-FMA hosts — the divergence named today in
   `BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md` §risk-3), transcendentals differ between
   native and wasm32 (`math-first-architecture-blueprint.md:161`), and masterwork #52's contract
   (fixed-summation-order f64 + integer-where-cross-target) exists precisely because of this.
   Under a non-deterministic fold, delta comparison is *worse* than state comparison: identical
   logs, divergent states, and the comparison layer reports agreement. §7 states the law and §9
   T5 makes it executable.

7. **Part 5's four questions (§10), one line each:** Stateful-vs-Stateless → **STATELESS-ABSOLUTE
   is the law** (the kernel's own event model is already stateless in this sense; increments were
   already foreclosed by the ABSOLUTE-OP LAW; sparse-absolute stateful = DEFER-WITH-TRIGGER on
   measured airtime pain). Dynamic Squashing → **SPLIT**: at the telemetry tier it is CWR's
   Kalman `update()` described from the buffer angle (ALREADY-EQUIVALENT); below CWR it is a
   triggered mechanism with a binding LAW-LANE UNSQUASHABILITY rule. Squash-validate vs
   Atomic-Rollback → **a false dichotomy; the recommendation is degrade-closed restated** —
   agreed, and explicitly *not* a new design decision. Snapshot-Checkpointing → **EXTENSION of
   adopted #151/#152/#153**; the one new detail (snapshot delivered INTO adapter memory) is
   realized by existing pieces (`KernelStateView` refresh or reinstantiation). The "Git for
   real-time memory" analogy is structurally wrong in three ways and is replaced (§10.5).

**Final verdict: ADOPT-EQUIVALENT** — the load-bearing topology (delta-native kernel, no-pointer
adapter containment, atomic decide-gated integration) is already built or already adopted this
round — **with one named caveat** (the "zero-cost" framing prices the read side and content
validation at zero; both are real, §4) **and a small genuinely-new residue** (§8's types), each
DEFER-WITH-TRIGGER or NEW-small, none a rebuild. Part 5 reinforces rather than disturbs this
verdict: two of its four mechanisms are restatements of adopted doctrine, the other two are
triggered extensions of existing pieces (§10).

---

## 1. The ruling, parsed precisely — three distinct claims inside one sentence

"Ядро має працювати з дельтами змін, а не станом" contains three separable claims with three
different truth-statuses. Blurring them is how a correct conviction turns into redundant work:

| # | Claim | Status |
|---|---|---|
| K1 | The kernel's *own* state-transition substrate should be a stream of change-events, with state derived, not primary | **Already built** (§2.1) — this is the decide/fold Law of `event_log.rs` |
| K2 | *Cross-node sync* should ship deltas, not snapshots | **Already built** (§2.1, masterwork #59) — `anti_entropy.rs::diff` computes the exact missing suffix |
| K3 | The *untrusted adapter boundary* should be: read-only view in, diff out, kernel integrates atomically | **Mostly already adopted** (§2.2, CSC Layers 1-2 + Fable-D §2); the genuinely new residue is §2.3 |

Part 4 itself is only about K3 (its subject is the adapter). The operator's sentence is broader
and is *most strongly* confirmed by K1 — the part that already exists. This document therefore
does not "introduce" the delta kernel; it verifies it exists, extends its discipline explicitly
to the K3 boundary, and prices the two things Part 4 got wrong (§4, §7).

---

## 2. The three-way split, with live evidence

### 2.1 ALREADY BUILT — the kernel is delta-native at its own state-transition level

All of the following was live-read this pass in `kernel/src/event_log.rs` (dowiz worktree) and
`bebop2/core/src/anti_entropy.rs` (bebop2 worktree):

1. **The unit of change is a content-addressed delta, not a state image.** `MeshEvent`
   (`event_log.rs:127-156`) is `(prev, actor_pubkey, actor_seq, payload)` — an *intent*, hashed
   to a 32-byte content-id that is simultaneously its identity, its idempotency key, and its
   chain link. The module docstring (`:1-8`) states the Law: "commits signed intents to a
   content-addressed event-log locally, running the kernel `decide`/`fold` Law *before* any
   network IO." State is what you get by folding; it is never the thing written.
2. **Integration is already atomic and already on the kernel's own terms.**
   `commit_after_decide` (`:366-391`) is precisely Part 4's "ядро бере дані ... і інтегрує їх у
   свій стан. Це один крок, який ви робите на своїх умовах" (`:252`): dedup on the raw
   content-id first (replay = structural no-op, `decide` never re-run — the P07 §2 fix, test
   `:679-751`), then the Law (`decide`) gates, then the durability barrier gates the tip
   (`append`/`append_raw` `:302-338` — `insert?` short-circuits *before* `set_tip`, so a failed
   fsync never fabricates a commit, H1 tests `:892-935`). Rejection persists nothing
   (`:753-764`). The two failure poles are typed and distinct (`CommitError::{Rejected,Store}`,
   `:269-275`). The drift gate (`commit_after_decide_drift_gate` `:419-449`) is a second
   kernel-side pre-persist check on the same one-step integration point.
3. **Delta comparison infrastructure exists.** `verify_chain` (`:475-504`) recomputes every
   content-id along the `prev` chain — comparing deltas *is* comparing hashes. On the sync
   side, `anti_entropy.rs::digest`/`diff` (`:35-107`, re-verified live) compares per-seq rolling
   hashes and yields the exact suffix to pull — masterwork **#59 "Sparse Delta Updates" =
   ALREADY-EQUIVALENT** ("anti-entropy ships deltas not snapshots"), confirmed against source,
   not just the ledger.
4. **The "atomic integration" mechanisms Part 4 needs are in the adopted ledger — reuse, don't
   re-derive** (task point 4): masterwork **#127** (verify-before-persist gating a handoff
   *before* the atomic swap) is **ADOPT** — and its adopted form *is* the
   drift-gate/decide-before-commit shape above, i.e. already code. Masterwork **#44/#107**
   (atomic `Arc`/pointer swap for concurrently-read derived state) are **DEFER-WITH-TRIGGER**
   ("no `AtomicPtr` consumer today"); the diff topology does not create that consumer — fold is
   single-threaded in-kernel — so the trigger stands unfired. Masterwork **#110/#154**
   (CoW Master/Leased tiles; CoW page snapshots) are **DEFER**, riding W3-L9 and P12
   respectively; §4's `KernelStateView` names #154 as its future zero-copy upgrade path, under
   the same trigger, not now.

**Consequence:** nothing in K1/K2 is new work. The event log needs no modification to "become"
delta-based. It is the existing proof that the operator's conviction was already the system's
design axiom.

### 2.2 ALREADY ADOPTED THIS ROUND — the adapter-facing memory topology

Part 4's K3 topology maps clause-by-clause onto rulings landed earlier this round:

| Part 4 (`:243-259`) | Already-adopted mechanism | Delta |
|---|---|---|
| "Kernel State ... маркується для адаптера як Read-Only" | WASM tier: kernel state is **not addressable at all** from the guest — separate linear memory, deny-by-default imports (`wasm-host/src/lib.rs:111,139-219`, live-read; CSC §2.1 Layer 1; Fable-D §2 row 2: "WASM is *stricter*: the adapter cannot name kernel memory at all, vs. Part 3 where it merely lacks write permission") | Part 4 asks for less than what is built. The *read* half is a genuine open design point — §4 — because "read-only access" must be explicitly provided, not merely permitted |
| "Adapter Sandbox: окрема, невелика область пам'яті (буфер), куди той може писати" | The guest's own linear memory + the sandbox output channel read only after clean return (CSC §2.2 item 2: a trap mid-write delivers zero bytes) | none |
| "Немає перевірок меж, бо він фізично не має адреси" | True and already the certified property: OOB inside the guest traps (`BridgeFault::Trapped`); OOB *outside* the guest is inexpressible. On 64-bit JIT hosts wasmtime's own bounds mechanism is guard pages — near-zero cost (Fable-D §2 row 3) | none — but see §3 for what this does NOT exempt |
| "Ядро ... інтегрує їх у свій стан. Один крок, на своїх умовах" | `commit_after_decide` (built, §2.1) behind the `BridgeResult` ingest gate (CSC §2.2, blueprint-stage) | none at the event-log level; the ingest gate is adopted-designed, not yet built — status honesty in §11 |
| "Це вимагає від адаптера бути диф-генератором ... не спрацює для legacy in-place коду" (self-critique) | **Already satisfied by construction here.** No adapter in this architecture has ever had in-place access to kernel structures — the WASM boundary forbids it. There is no legacy in-place-mutation adapter to convert; every adapter is already output-only | the rewrite cost Part 4 warns about is zero in this codebase — a point *for* adoption-equivalence |

This is the same adjudication shape Fable-D reached for Part 3 (**ADOPT-EQUIVALENT** — bare-metal
vocabulary independently re-deriving the certified WASM containment). Part 4 is Part 3's
sequel with the access vector flipped 180°, and the flip lands on machinery that already exists:
Part 3 said "the adapter cannot write outside its buffer"; Part 4 says "the adapter cannot write
into the kernel *at all* — it proposes". The codebase's answer to both is one mechanism: the
adapter's entire output universe is its return bytes, and those bytes reach state only through
the gate + `commit_after_decide`.

### 2.3 GENUINELY NEW — the residue, named precisely

Four artifacts in Part 4 are not covered by §2.1/§2.2. Each is small; none is a rebuild:

- **N1 — the patch vocabulary (`DeltaPatch`/`PatchOp`, §5).** Today's designed adapter output
  (CSC/Fable-D) is a *whole translated frame* per lane per epoch — which is already "a delta"
  at frame granularity (a keyed update, not a state image). Part 4's finer claim — output as an
  explicit op-list against named targets — adds two real properties frame-replacement lacks:
  per-op scope checking (each op names its target; out-of-lane targets are rejectable
  *per-op*), and the untouched-state guarantee (unnamed keys provably unchanged — §6 n1).
  Needed only when an adapter's output is sparse relative to its lane; for whole-frame
  translators it degenerates to today's design (one `Put` of one frame). **NEW-small**, with the
  frame-degenerate case as the mandatory floor.
- **N2 — the read-side contract (`KernelStateView`, §4).** Part 4 says "адаптер читає Kernel
  State" as if reading were ambient. In WASM it is not: the view must be constructed. This is a
  real design decision with a real (small) cost, and it is scope-relevant: *what an adapter may
  read* is an authority grant symmetrical to what it may write. Today's only adapter class
  (layout translators) reads nothing but its own input frame — so `KernelStateView` is
  **DEFER-WITH-TRIGGER**: trigger = the first adapter whose output is a function of kernel
  state rather than of its input alone.
- **N3 — one fault arm.** Per-op lane-scope violation (`LaneScopeReject { op_index }`) is
  observably distinct from decode failure (`DecodeReject` — malformed bytes) and from
  instantiation-time `ScopeViolation` (import-level). Proposed as the single addition to
  Fable-B's seven-arm `BridgeFault`; flagged explicitly because Fable-D's blueprint promised no
  new variants and this one is new — it exists only if N1 is built.
- **N4 — the delta-determinism caveat law (§7).** Not in Part 4 at all (Part 4 asserts the
  comparison claim unconditionally); required by this session's own findings.

---

## 3. Relation to CSC-LAW — (a), (b), or (c)?

The task's three options, adjudicated against CSC's own structure:

- **(a) "a genuinely different, ADDITIONAL layer combinable with CSC-LAW" — no.** CSC-LAW is
  already three layers: Layer 1 spatial (WASM/microVM containment), Layer 2 structural (sealed
  `BridgeResult`, loud faults), Layer 3 authority (lanes, unstrippable provenance, red-line
  un-nameability). Part 4's memory-permission topology is a description of **Layer 1** — there
  is no fourth place to bolt it on. Combining it with CSC-LAW would be combining CSC-LAW with
  itself.
- **(b) "the same thing described differently" — yes, for the containment claim.** With one
  asymmetry worth recording: Part 4's *letter* (a write-protected but readable mapping of
  kernel state) is actually **weaker** than the built Layer 1 (no mapping at all). The
  dialogue's "no write-capable pointer exists" is realized here as "no pointer exists,
  write-capable or otherwise" — which is why the read side becomes an explicit design point
  (§4) instead of a free property.
- **(c) "makes part of CSC-LAW's machinery unnecessary" — no, and this is the load-bearing
  correction.** Part 4's rhetorical frame ("Немає перевірок меж... Ви не імітуєте проміжок
  перевірками") invites the reading that gate checks can be dropped. Two different check
  classes are being conflated:
  1. *Spatial bounds checks* ("did the adapter write outside its buffer?") — genuinely
     unnecessary, by construction, exactly as Part 4 says. Also already unnecessary: nobody
     designed such checks this round, because WASM made them moot from the start.
  2. *Content validation at ingest* (strict decode, canonicalization, value bounds, per-op
     scope, provenance wrapping — CSC Layer 2/3) — **fully retained**. A diff arriving in a
     topologically clean buffer is still untrusted bytes. Part 4 itself concedes this in its
     closing sentence: the patch *is* validated ("ви валідуєте лише результат (дифф), який
     приходить в одному і тому ж форматі"). What Part 4 removes is validation *of the
     computation process*; CSC never had any — the C′ thesis was already "the kernel is
     indifferent to what the adapter claims about its work; it re-checks what is checkable and
     bounds the rest."

  Furthermore, the sealed two-variant `BridgeResult` is not made redundant by the staging
  buffer — it is the *type-level shadow of the buffer handoff*: "kernel integrates on its own
  terms" has to be a function somewhere, and that function is the ingest gate constructing
  `Translated`/`Failed`. Delete Layer 2 and "atomic integration on the kernel's terms" becomes
  a memcpy of unvalidated bytes with extra steps.

**Ruling: (b)** — Part 4 is CSC Layer 1 restated in access-vector vocabulary, contributing the
N1/N2 residue at Layers 2/3's edge, and removing nothing.

---

## 4. The read side, designed honestly per platform — and the zero-cost claim priced

### 4.1 What WASM linear memory actually gives you (task point 5)

The WASM model: a module reads/writes **only its own linear memory**; every external effect is
an explicitly linked import. This maps exactly onto Part 4's write-side claim — and *overshoots*
it: the guest also cannot **read** host memory. "Kernel state is read-only to the adapter" is
therefore not free/automatic; it must be one of:

| Mechanism | How | Cost | Where admissible |
|---|---|---|---|
| **M1 copy-in** (default) | Host writes the epoch's state-view slice into guest linear memory (or passes it as call args) before invoking the adapter | one memcpy, O(view bytes), per adapter per epoch. At DDR bandwidth (~10-30 GB/s) a frame-scale view (≤ 64 KiB) is sub-10 µs — invisible next to instantiation and gate costs. **No benchmark exists; number needed only when N2's trigger fires** | phones (both OSes) + hub; the only option under iOS's interpreter tier |
| **M2 host-read imports** | Guest calls `read-state(key) → bytes`; each import individually granted deny-by-default (the existing `allowed_imports_for_scope` shape) | per-call host-boundary crossing + copy of returned bytes; chatty for large read-sets, but每 call is auditable authority | when the read-set is sparse and data-dependent (adapter doesn't know its keys upfront) |
| **M3 zero-copy RO mapping** | Sealed memfd (`F_SEAL_SHRINK\|F_SEAL_GROW`) or virtio-shm mapped read-only into the adapter's address space — a hardware-enforced read-only view, the literal Part 4 letter | true zero-copy; setup cost only | **hub only** (Fable-D §1.4), post-VMM; never phones (Fable-D §1.2-1.3: no page-grant facility for unprivileged apps). DEFER with the microVM VMM follow-up; masterwork #154's CoW snapshot is the eventual view-construction substrate |

**Design ruling:** M1 is the contract (`KernelStateView` = an immutable, epoch-stamped,
read-scope-checked *copy*); M2 is the escape hatch for sparse reads; M3 is a hub-tier
optimization behind its existing triggers. The view is a copy **by design, not merely by
limitation**: a copy is torn-read-proof (the adapter sees one consistent epoch, never a state
mid-fold) — the same reason Part 1's dialogue wanted immutable pinned snapshots (`:45-46`).

### 4.2 The "zero-cost air-gap" claim, priced

Part 4's frame: checks cost 5-10% on the hot path; topology costs nothing. On the real
platforms:

- **Write side: the zero-cost claim is TRUE** — and was already banked. Escape is
  inexpressible, not checked-for. No work item exists.
- **Read side: NOT zero.** Cost = M1 copy-in (small, unbenchmarked, trigger-gated) or M2 call
  overhead. Part 4 never mentions this because its mental model is a shared address space with
  a write-protected mapping — which no phone target provides (Fable-D §1.1-1.3).
- **Validation side: NOT zero and not removable** (§3c) — decode + canonicalize + bounds + scope
  at ingest stays. This was never a *bounds* check, so Part 4's "no bounds checks" is compatible;
  its broader "no checks in the hot path" reading is not.
- **The 5-10% premise itself is moot here:** the sacrifice Part 4 is engineered to avoid
  (software MPU-simulation checks) was never the plan — Fable-D dissolved the raw-MMU work item;
  WASM SFI's own cost profile (near-zero on 64-bit JIT via guard pages; interpreter-class on
  iOS regardless of topology) is paid for containment, and the diff topology neither adds to
  nor subtracts from it.

**Net: the honest accounting is "topology trades one small cost for another smaller one", not
"topology is free".** Bounds-checking was already free; copy-in is the new small cost; content
validation was never on the table for removal.

---

## 5. The staging-buffer → atomic-integration mechanism — reuse, concretely

No new commit mechanism is designed. The pipeline, every stage already adopted or built:

```
adapter (WASM guest)
  │  writes DeltaPatch bytes into its own linear memory; returns cleanly
  ▼
sandbox output channel            ── CSC §2.2: read only after clean return;
  │                                  trap ⇒ zero bytes ⇒ Failed(Trapped)
  ▼
ingest gate (CSC Layer 2)         ── strict decode of DeltaPatch (canonical op order,
  │                                  op count ≤ MAX_PATCH_OPS, values in bounds)
  │                               ── per-op lane-scope check (N3: LaneScopeReject)
  │                               ── wraps as Translated(Provenanced<DeltaPatch>)
  ▼
commit_after_decide               ── BUILT (event_log.rs:366-391): dedup by content-id
  │                                  (replay = structural no-op) → decide Law →
  │                                  drift gate variant where wired (:419-449) →
  │                                  durability barrier → tip advance
  ▼
fold                              ── derived state updated; deterministic-apply law (§7)
```

Reuse citations (task point 4, explicitly): the **one-step integration on the kernel's terms**
is `commit_after_decide` — built, tested, with the P07 replay-idempotency fix that is *exactly*
the property a diff-stream needs (a re-delivered diff must not double-apply; test `:679-751`).
The **verify-before-swap** discipline is masterwork **#127 (ADOPT)**, whose adopted realization
is this same decide/drift-gate-before-persist path. The **atomic pointer swap** (#44/#107) and
**CoW lease/snapshot** (#110/#154) remain **DEFER-WITH-TRIGGER** — their triggers (a concurrent
reader of folded state; durable-snapshot P12) are not fired by this blueprint, and no substitute
mechanism is invented.

One law derived from existing rulings, made explicit for the patch vocabulary:

> **ABSOLUTE-OP LAW:** `PatchOp` carries absolute assignments (`Put`/`Remove`), never
> read-modify-write (`Add`/increment). Rationale: (i) idempotent apply — the content-id dedup
> makes a replayed *event* a no-op, and absolute ops extend that guarantee from the log to the
> state (a double-applied `Put` is harmless; a double-applied `Add` is the P07 money-bug shape
> one level up); (ii) masterwork **#53 (REJECT-ON-CORRECTNESS)** already refused soft/merge
> semantics over non-commutative state — increments are the entry ramp to exactly that.

---

## 6. RC-2-broad — does the topology change Fable-B's finding?

**No. The parent hypothesis is confirmed, verified independently here rather than assumed.**

Walk the theorem (CSC §3) with a diff substituted for a frame: correctness of the adapter's
output means `patch = g(input, view)` for the ideal computation `g`. To detect a wrong patch the
kernel must evaluate `R(input, view, patch)` — which requires re-implementing `g`, which the
kernel by design cannot do (it lacks the legacy layout / the adapter's domain knowledge; that is
why the adapter exists). Nothing about the *shape* of the output (frame vs op-list) or the
*permission topology* of the buffer it arrived in adds one bit of evaluable information to the
kernel. A canonical, in-bounds, in-scope, drift-passing `DeltaPatch` whose values are simply
*wrong* is integrated. The strongest lie is unchanged; only its envelope got tidier.

Two genuine narrowings — stated precisely so they are not mistaken for closure:

- **n1 — footprint narrowing (reach, not correctness).** A diff names its write targets;
  everything unnamed is untouched **by construction** (§9 T1 makes this falsifiable). Under
  frame replacement, wrongness could be anywhere in the frame; under a diff, wrongness is
  confined to the named keys. This shrinks the blast radius and makes the provenance
  blast-radius query ("everything adapter A touched since epoch E" — CSC Layer 3 item 1) both
  cheaper and finer-grained (key-level, not lane-level). It is a Layer-3-class improvement:
  containment-in-space-and-time gets tighter. The residual gap's *area* shrinks; its *existence*
  does not change.
- **n2 — cheaper closure mechanisms, not new ones.** The only real closures remain CSC §3's
  round-trip witness (bijective lanes) and N-version translation (lossy lanes), both
  DEFER-WITH-TRIGGER there. Canonical content-addressed diffs make N-version comparison a
  32-byte id `memcmp` and make witness inputs smaller. Cost reduction on the existing
  mechanisms; zero verification power of its own.

One tempting non-mechanism, pre-rejected: "a diff touching 10,000 keys when the typical is 3 is
suspicious" — a size-anomaly *score* is the #33/NO-SCORING rejected class. The admissible form
is the deterministic budget already used everywhere else: `MAX_PATCH_OPS` as a hard bound
(TokenBucket/`budget.rs` shape) — a boolean gate, not a suspicion metric.

**Consequence:** CSC T4 (`residual_semantic_gap_pinned` — the well-formed wrong tile IS accepted
in-lane) must be carried over to the patch form unchanged (§9 T4). Any future claim that "the
diff topology closed RC-2-broad" is falsified by that test's continued necessity.

---

## 7. The determinism claim, evaluated against this session's own findings

Part 4: "дельти простіше порівняти, ніж загальний стан" — deltas are easier to compare than
full state, simplifying security and determinism. Adjudication in three parts:

1. **Where it is simply true (and built):** comparing *what happened* is O(1) per event by
   content-id (`MeshEvent::event_id`, `:145-156`); divergence localization is O(log-free linear
   scan of digests) with exact-suffix pull (`anti_entropy.rs::diff`); replay is structurally
   idempotent. Full-state comparison offers none of that granularity. For the log layer, the
   claim stands.
2. **The condition Part 4 omits:** delta comparison certifies *state* agreement only through
   the implication `same log + deterministic fold ⇒ same state`. The first conjunct is what
   deltas make cheap to check. The second is an assumption — and this session's own findings
   show it is the fragile one for any float-touching fold:
   - `householder.rs:29-75` (live-read this pass): `dot()` runtime-dispatches an FMA path
     (`_mm256_fmadd_pd`, fused rounding) vs a scalar loop — the same binary produces
     different low bits on FMA vs non-FMA hosts; named today as a byte-determinism risk in
     `BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md` ("brittle across FMA/non-FMA
     codepaths").
   - `math-first-architecture-blueprint.md:161`: `log2/sqrt/hypot` not bit-identical between
     native x86_64 and wasm32 — and phones-vs-hub is *exactly* a cross-target fold.
   - Masterwork **#52 (ALREADY-EQUIVALENT: "the concern IS the governing constraint")**: the
     contract is fixed-summation-order f64 + **integer-where-cross-target**; R2's grounding
     pins the hardest lane (money = discrete integer channel, never interpolated,
     `[VERIFIED-CODE]`); R4:202 shows the FEC layer holding the same bar (same symbols ⇒
     bit-identical reconstruction or clean failure).
3. **The failure mode if the condition is ignored — worse than state comparison:** two nodes
   exchange delta digests, agree perfectly, and hold divergent folded states (an FMA host and a
   wasm32 phone folding the same float lane). State-hash comparison would have *caught* this;
   delta comparison *hides* it. So the claim, taken unconditionally, inverts: deltas are easier
   to compare, and easier to be wrong about what the comparison proves.

> **DELTA-DETERMINISM LAW (the caveat, stated once):** delta-equality is proof of state-equality
> **iff** (i) the delta encoding is canonical and content-addressed (built: `event_id` +
> `framing.rs` hard-reject + T6-class golden-byte pins) and (ii) the fold over any
> cross-node-compared lane is bit-deterministic — integer/fixed-point per masterwork #52 and the
> R2 money-channel discipline, or fixed-summation-order f64 with no runtime ISA dispatch on the
> fold path (the `dot()` FMA dispatch is admissible in *per-host advisory* lanes only, never in
> a lane whose fold output is cross-node compared). Where (ii) is not yet proven for a lane, the
> Part-1-adopted state-hash heartbeat (`00-SOURCE-DIALOGUE.md:47-48`, `Hash(Snapshot_N)`
> exchange) is **retained as the falsifier**, not retired as redundant — it is the only observer
> of the hidden-divergence failure mode.

Verdict on the claim: **true only under conditions (i)+(ii); adopted with the law above as its
binding caveat.** The heartbeat state-hash cross-check must not be dropped on the strength of
Part 4's sentence.

---

## 8. Verdict and predefined types

**ADOPT-EQUIVALENT.** The kernel already works with deltas (K1/K2 built and tested); the
adapter topology is CSC Layer 1 + the ingest discipline already adopted this round (K3);
`commit_after_decide` is already the atomic integration step Part 4 describes. The "zero-cost"
framing is corrected, not adopted: write-side free (banked), read-side = explicit copy-in
(M1, trigger-gated), content validation non-negotiable (§3c). The genuinely new residue, all
small:

```rust
// N2 — the read-side contract (DEFER-WITH-TRIGGER: first adapter whose output
// is a function of kernel state, not just its input frame). A COPY by design:
// immutable, one consistent epoch, torn-read-proof. Read scope is an authority
// grant symmetric to write-lane scope — deny-by-default, part of the Scope
// grant, never adapter-requested at runtime.
pub struct KernelStateView {
    pub epoch_id: u64,          // the epoch this view is a snapshot of
    pub lane: LaneId,           // the single lane this view exposes
    pub bytes: Box<[u8]>,       // canonical encoding; M1 copy-in
}
pub const MAX_STATE_VIEW_BYTES: usize = 64 * 1024; // placeholder pending bench (M1 cost number)

// N1 — the staging-buffer patch vocabulary (NEW-small). Frame-degenerate floor:
// a whole-frame translator emits exactly one Put of one frame — today's design
// unchanged. ABSOLUTE-OP LAW (§5): no read-modify-write op exists or may be added.
pub struct DeltaPatch {
    pub base_epoch: u64,        // the view epoch this patch was computed against
    pub ops: Vec<PatchOp>,      // canonical order (sorted by (lane,key)); decode-enforced
}
pub enum PatchOp {
    Put { lane: LaneId, key: u64, value: Box<[u8]> },   // absolute assignment
    Remove { lane: LaneId, key: u64 },
}
pub const MAX_PATCH_OPS: u32 = 256; // hard budget (TokenBucket shape), NOT an anomaly score

// N3 — the single addition to Fable-B's BridgeFault (exists only if N1 lands):
// per-op lane-scope violation, distinct from DecodeReject (malformed) and from
// instantiation-time ScopeViolation (import-level).
//   LaneScopeReject { op_index: u32 }
```

Everything else by reference: `BridgeResult`/`Provenanced`/ingest gate (CSC §2.2, unchanged);
`LaneFrameHeader` (Fable-D §5, unchanged — a `DeltaPatch` is a lane payload under it);
`commit_after_decide`/drift gate (built); #44/#107/#110/#154 defers (untouched, triggers
unfired); DELTA-DETERMINISM LAW (§7) as a decode/fold discipline, not a type.

---

## 9. DoD — falsifiable tests

Adversary: the CSC malicious WASM adapter, now emitting patches. RED-first where a hole is
proven; T4 and T5 are the two that must never be skipped.

| # | Test | Proves | Expected |
|---|---|---|---|
| T1 | `delta_untouched_state_is_byte_identical` | §6 n1 footprint guarantee | apply a patch naming keys K; assert every key ∉ K in the lane is byte-identical pre/post; a frame-replacement implementation cannot pass this shape — the test is the executable difference between diff and frame semantics |
| T2 | `patch_op_out_of_lane_scope_rejected` | N3 / Layer 3 per-op reach | a patch containing one op naming an ungranted lane → `Failed(LaneScopeReject{op_index})`, zero ops applied (all-or-nothing — no partial patch) |
| T3 | `patch_replay_is_structural_noop` | §5 reuse of P07 discipline at patch granularity | same `DeltaPatch` event committed twice via `commit_after_decide` → second is `Duplicate`, decide not re-run, state byte-identical (extends `commit_after_decide_replay_on_nonempty_log_is_true_duplicate`, `event_log.rs:679-751`) |
| T4 | `wrong_content_patch_accepted_in_lane` | §6 — RC-2-broad is NOT closed by topology | a canonical, in-scope, in-bounds patch with semantically wrong values IS integrated; the test *asserts acceptance*, carrying CSC T4's pin over to the diff form. If a later mechanism flips it, both this doc and CSC §4 must be revised together |
| T5 | `delta_equality_does_not_imply_state_equality_under_float_fold` | §7 law, condition (ii) | RED-form demonstration with no hardware dependency: fold the same delta log with two summation orders over a catastrophic-cancellation f64 fixture → identical logs, divergent state hashes (float non-associativity); GREEN: integer/fixed-order fold → identical hashes. Pins WHY the heartbeat state-hash survives |
| T6 | `state_view_is_epoch_stamped_copy` | §4 M1 contract (when N2 fires) | mutate the view buffer after handoff → kernel state unaffected; view of epoch E never reflects a fold that happened during the adapter's run (torn-read-proof) |
| T7 | `patch_op_budget_is_boolean_not_score` | §6 pre-rejection | ops > `MAX_PATCH_OPS` → typed reject; grep-guard: no code path computes a "typical patch size" or any comparative anomaly metric (NO-SCORING) |
| T8 | `absolute_op_law_pinned` | §5 | compile-time/exhaustiveness: `PatchOp` has no arithmetic/increment arm; adding one breaks this match-exhaustiveness test and forces revisiting #53's rejection |

Falsification criteria: the ADOPT-EQUIVALENT verdict is wrong iff K1/K2 evidence fails
re-verification (event log not delta-native — refuted by live read) or the WASM tier fails CSC
T1-T3 (containment not real). The zero-cost correction is wrong iff M1 copy-in measures at
≥ the 5-10% Part 4 fears on a real adapter workload (bench required at N2's trigger — record the
number in `BENCH_HISTORY.md` either way). The §7 caveat is wrong iff T5's RED form cannot be
constructed — i.e., iff float folds are shown bit-stable across the real host set, which the
`householder.rs` dispatch already disproves for FMA-divergent hosts.

---

## 10. Part 5 addendum — Stateful/Stateless, Squashing, Rollback, Snapshot-Checkpoint

> Part 5 arrived after §§1-9 were complete and directly continues Part 4's open questions. No
> operator ruling is attached to it; every verdict below is derived from already-ratified
> rulings and live-read code, and is marked where it merely restates one. Part 5's own §1-§2
> ("Patch(State, Delta) — ядро бачить лише що патч не валідний"; "ядро ніколи не читає пам'ять
> адаптера як готову структуру — лише як пропозицію змін") are the ingest-gate design of §5
> restated and need no separate adjudication — they confirm §3(c): the patch IS validated,
> which is CSC Layer 2, which Part 4's "no checks" rhetoric had appeared to remove.

### 10.1 Stateful vs Stateless adapter — STATELESS-ABSOLUTE is the law; sparse-stateful is a triggered optimization

The dialogue's question: should the adapter track its own copy of kernel state to emit smaller
deltas (Stateful — "економія каналу"), or send self-contained absolute values (Stateless)?

**First, the consistency argument the coordinator's question names — verified, and it is
decisive:** the kernel's own event model is already "stateless" in exactly the dialogue's
sense. A `MeshEvent` payload is a self-contained intent; the P07 fix (`event_log.rs:349-365`)
deliberately made the content-id **chain-independent** so a replay is a structural no-op
regardless of the receiver's chain position; the sender never tracks receiver-side state —
divergence is handled *pull-side* by the anti-entropy digest exchange (`anti_entropy.rs:35-107`),
not by sender-side belief. A Stateful adapter would introduce, at the adapter boundary, exactly
the sender-tracks-receiver coupling the kernel's own substrate was engineered not to have.

**Second, the dangerous half of Stateful is already foreclosed.** "Smaller delta" against a
believed base has two forms:

- **Relative-value deltas (increments)** — already rejected by the ABSOLUTE-OP LAW (§5):
  non-idempotent (the P07 money-bug shape one level up), and the entry ramp to masterwork #53's
  rejected soft-merge-over-non-commutative-state. Drift here is *corrupting* (wrong increments
  applied to a base the adapter imagined). Not admissible in any form.
- **Sparse-absolute deltas** (send only the keys that changed vs the tracked base; values still
  absolute `Put`s) — structurally admissible: a wrong base produces *omission-staleness*
  (unsent keys stay stale), never corruption of named keys. Bounded deterministically by the
  existing `DeltaPatch.base_epoch` field (§8): a patch whose base epoch falls outside the
  lane's declared window → typed reject (Part 5's own ordering rule `:296-298`, made a *gate*,
  not a default queue; queuing is per-lane opt-in). Recovery = §10.4's checkpoint.

**Third, the bandwidth case, priced with this session's real numbers rather than assumed:**
the courier profile is V5 §4's cellular reality (200-2000 ms RTT, 1-10 % loss — the same
numbers the FEC blueprint calibrates against). Three observations kill the "економія каналу"
case at current workloads:

1. Payloads on the latency-critical lane are frame-scale (a `LaneFrameHeader` + a small frame);
   the absolute-vs-sparse saving on a tens-of-bytes payload is noise next to the 37-50 % parity
   overhead the FEC blueprint *deliberately spends* on that same lane (FEC §4.1-4.2) — radio
   airtime is the battery cost, and the corpus already decided frame-scale airtime is cheap
   enough to buy reliability with.
2. A 1-10 % loss channel is toxic to base-tracking: every lost patch desyncs the adapter's
   believed base (Part 5's own Achilles-heel paragraph), forcing checkpoint pushes whose
   airtime devours the sparse savings. Stateless absolute frames are **supersedable** — FEC
   §4.1's own load-bearing property ("the next position update replaces the lost one") *only
   holds for absolute values*. Stateful deltas would break the existing loss-recovery story.
3. The adopted telemetry lane is already stateless: CWR's Kalman `update()` consumes absolute
   sensor measurements. There is no stateful consumer to serve.

**Verdict: STATELESS-ABSOLUTE default for all lanes** (consistency with the built kernel +
the loss profile + CWR). **SPARSE-ABSOLUTE-STATEFUL = DEFER-WITH-TRIGGER**: a lane whose
per-epoch payload is ≫ frame-scale AND measured airtime pain under the V5 netem recipes, number
appended to `BENCH_HISTORY.md` first (the FEC blueprint's own calibration discipline). Never
increments, under any trigger.

### 10.2 Dynamic Squashing — SPLIT: at the CWR tier it IS the Kalman update; below it, a triggered mechanism with one binding law

The coordinator's question — is squashing just CWR from a different angle? — resolves cleanly
into a tier split:

- **Telemetry/CWR tier: ALREADY-EQUIVALENT.** A Kalman `update()` *is* squashing done
  optimally: N measurements collapse into one state estimate, covariance-weighted, with the
  commit cadence decoupled from the measurement cadence — which is precisely Part 5's "ядро
  обробляє пачками" outcome. A naive last-writer-wins squash on measurements is strictly
  *worse* than the filter (discards the information the fusion uses). The telemetry lane
  therefore needs no second mechanism; Dynamic Squashing at this tier is CWR described from
  the buffer angle.
- **Raw patch layer (below CWR): a real, distinct mechanism — admissible only under a law.**
  Composition of ABSOLUTE-OP patches is per-key last-writer-wins — deterministic, associative,
  O(ops). But it is only sound where intermediate values carry no obligations:

  > **LAW-LANE UNSQUASHABILITY:** any lane whose `decide` validates *transitions* (order FSM,
  > money, capability/revocation) must see every delta individually — squashing
  > `Pending→Confirmed→Delivered` into `Pending→Delivered` erases a Law edge that
  > `assert_transition` exists to check. Squash composition is definable only for lanes whose
  > semantics are pure last-writer-wins state. (Post-commit squashing of the hash-chained log
  > is inexpressible — append-only — which is correct and needs no rule.)

  Design pinned for when the trigger fires: **gate-before-squash** — each arriving delta passes
  the structural gate (decode/content-address/scope) *before* entering the squash buffer, so a
  structurally-corrupt delta never contaminates a composition (this dissolves half of §10.3's
  scenario by construction); composition then runs over gated deltas only; one `decide` at
  commit. The hosting seam already exists: `BoundedDrainer` (`kernel/src/bounded_drainer.rs`,
  live-read — at most `k` units per tick, `TokenBucket`-debited, degrade-closed early stop) is
  the batch-drain half; squash = that drainer plus a compose step.
- **Workload honesty:** the 1 kHz navigation example is the dialogue collaborator's drone
  domain, not this codebase's — courier telemetry is ~0.2-1 Hz. No commit-rate pressure exists
  today. **DEFER-WITH-TRIGGER:** measured commit-rate pressure on a squashable
  (non-Law, last-writer-wins) lane.

### 10.3 Squash-validate vs Atomic-Rollback — a false dichotomy; the recommendation is degrade-closed restated

The dialogue asks: validate the squashed result (expensive) or atomically discard the whole
batch on failure (cheap — its own recommendation)? Adjudicated against what is already decided:

1. **The dichotomy is false.** The final squashed patch is ALWAYS gated — the O(patch)
   decode/canonicalize/bounds/scope pass is the check §3(c) established as non-removable and
   which runs on every commit regardless. "Validation after squashing" is not the expensive
   option; it is the mandatory floor. The genuinely expensive readings of "validate" are:
   re-validating each component delta (unnecessary — gate-before-squash already did, §10.2) and
   verifying *semantic* correctness (impossible for any validator — §6's RC-2-broad theorem,
   which squashing neither worsens nor improves).
2. **On failure, the answer is already doctrine, mechanism by mechanism.** Discard the whole
   batch; kernel remains at last-good state; typed fault; deterministic circuit-breaker after N
   consecutive failures; resync signal (= §10.4's checkpoint push). Every element exists:
   decide-rejection-persists-nothing (`event_log.rs:753-764` test), CSC Layer-2 no-partial,
   this blueprint's T2 all-or-nothing patch application, Fable-C §4.1's integer
   consecutive-failure breaker, R2's degrade-closed doctrine. **Verdict: ALREADY-EQUIVALENT —
   the dialogue independently re-derived degrade-closed for the squash buffer. Agreed with its
   recommendation, and recorded plainly as a restatement, not a new design decision.**
3. **The bottleneck it asks about ("чи бачите вузьке місце"), named honestly: resync storms.**
   A flapping adapter that repeatedly fails at commit converts atomic rollback into repeated
   full-snapshot pushes — the real cost center of the discard-whole policy on a lossy channel.
   Bounded by the existing breaker (N consecutive ⇒ intake disabled, cool-down/operator
   readmission — never a health score), which caps resync frequency at N-per-disable-cycle.
   No new mechanism needed; the bound must simply be tested (T11).

### 10.4 Snapshot-Checkpointing — EXTENSION of adopted #151/#152/#153; one new detail, realized by existing pieces

Masterwork ledger, checked directly (task point 4 discipline — cite, don't re-derive):

- **#151 Rolling/"Golden State" snapshot: PARTIAL-EXISTING + ADOPT-the-bridge** —
  checkpoint/restore built (`snapshot_payloads`), cheap-regen rides item #130 + arena, durable
  half = P12, bridge-gap #2 = drift-gate the snapshot. Part 5's "Золотий зліпок" is this item
  by name.
- **#152 (checksum-vs-expected, auto-reseed-from-golden past threshold): ADOPT-the-structural-form**
  — the inline degrade-closed corruption path. **#153: preventive vs reactive reset kept
  distinct.** Part 5's "кожні N епох (або за сигналом ядра)" maps exactly onto that adopted
  pair: periodic = preventive rhythm; on-failure resync (§10.3) = reactive.
- **The genuinely new detail Part 5 adds — the delivery target:** the snapshot is written *into
  adapter-readable memory* ("жорсткий ресет ... просто перезапис пам'яті"). #151 never
  specified an adapter-facing delivery. Realized entirely by pieces this blueprint already
  names: a **`KernelStateView` refresh** (M1 copy-in of the golden snapshot at the checkpoint
  epoch, replacing the adapter's tracked base — the host writing into guest linear memory is
  the one direction the WASM model does permit), or — the strongest reset, already built as the
  isolation teardown — **reinstantiate the adapter** (fresh linear memory, zero inherited
  belief). No new mechanism; a composition of #151 + N2 + a scheduling rule.
- **Degeneracy check that closes the loop with §10.1:** for the STATELESS-ABSOLUTE default
  adapter, Snapshot-Checkpointing degenerates to *nothing* — there is no adapter-side base to
  correct. The entire checkpoint apparatus is a cost of going Stateful, which is one more
  argument for the stateless default; it activates only with §10.1's trigger.
- **Coherence:** Snapshot-Checkpointing, §7's retained state-hash heartbeat, and #152's
  checksum-vs-expected are one loop, not three mechanisms: compare hashes on heartbeat;
  mismatch ⇒ preventive golden-snapshot push or reactive teardown+reinstantiate.

### 10.5 The closing 5-point stack, and the "Git for real-time memory" analogy

Part 5's summary stack, each point mapped to its adjudicated home — no point is new:

| Part 5 stack | Adjudicated home |
|---|---|
| Transport = zero-copy segmented buffers | Synthesis row 4 (cleartext carrier REJECT-ON-PHYSICS); zero-copy adopted at the lane boundary only; wire = signed envelope + FEC shard layer |
| Structure = header-driven | Fable-D's `LaneFrameHeader` (32 B, decode laws, no Confidence/CRC slots) |
| Data-flow = Stateful-Delta + Dynamic Squashing | **Corrected** to STATELESS-ABSOLUTE default (§10.1) + tier-split squashing (§10.2) |
| Security = topology isolation | §3: CSC Layer 1 (built, WASM); content validation retained |
| Reliability = Atomic Rollback | §10.3: degrade-closed restated |

**The analogy, sanity-checked: replaced.** "Git for real-time memory" (kernel = main, adapters
= feature branches, resync = `git reset --hard origin/main`) is communicatively appealing and
structurally wrong in three load-bearing ways: (i) git branches are persistent, independently
authoritative, and may diverge arbitrarily — an adapter holds no authoritative state and its
staging buffer is single-use per epoch; (ii) git merging is three-way with conflict resolution
and preserves both parents' history — the kernel never merges adapter history; it applies one
diff whole or rejects it whole (#53 rejected the soft-merge shape explicitly); (iii)
`git reset --hard` is the *receiver* resetting itself by its own choice — here the *kernel*
pushes the reset into the adapter, the direction is inverted. The accurate one-liner, kept as
this blueprint's illustrative framing:

> **An adapter is an untrusted contributor emailing a single patch against a pinned base
> revision: the maintainer's gate applies it cleanly to mainline or rejects it whole — there
> are no branches, no merges, and a contributor whose patches stop applying gets their checkout
> wiped and re-cloned from mainline.**

The dialogue's phrase may be quoted as the source's own framing, always with this correction
beside it.

### 10.6 Part-5 DoD additions

| # | Test | Proves | Expected |
|---|---|---|---|
| T9 | `stale_base_patch_rejected` | §10.1 base-epoch gate | patch with `base_epoch` outside the lane's declared window → typed reject, nothing applied, observable event (no silent queue unless the lane opted in) |
| T10 | `law_lane_unsquashable` | §10.2 law | compose is not implemented for Law-lane delta types (trybuild compile-fail); telemetry-lane compose is deterministic and associative (property test: `compose(a, compose(b, c)) == compose(compose(a, b), c)` and equals sequential apply) |
| T11 | `squash_failure_discards_whole_batch_and_bounds_resync` | §10.3 | inject a commit-time failure on a squashed batch → state at last-good, typed fault, resync signal; after N consecutive → intake disabled (resync-storm bound is the breaker, not a score) |
| T12 | `checkpoint_resets_stateful_base` | §10.4 (only if §10.1's trigger ever fires) | a sparse-absolute adapter with a deliberately desynced base emits omission-stale patches → heartbeat hash mismatch → golden-snapshot view push → next patch correct; the reinstantiation path passes the same assertion |

---

## 11. Exists-today vs to-build

| Piece | Status | Where |
|---|---|---|
| Delta-native kernel substrate (content-addressed events, decide-gated atomic commit, replay idempotency, chain verify, typed failure poles) | **Built + tested** | `kernel/src/event_log.rs:127-505` (live-read in full) |
| Delta-based sync (exact-suffix pull) | **Built + tested** (masterwork #59 ALREADY-EQUIVALENT, re-verified) | `bebop2/core/src/anti_entropy.rs:35-130` |
| Adapter containment topology ("no write-capable pointer") | **Built + tested (WASM tier)**; microVM probe-only | `wasm-host/src/lib.rs:111,139-219,244-296`; `isolation/microvm.rs` (per CSC §2.1) |
| Ingest gate / `BridgeResult` substrate ("integrate on kernel's terms", typed) | **Adopted-designed, NOT built** (Fable-B blueprint) | CSC §2.2 |
| Verify-before-persist atomic integration | **Built** (= masterwork #127's adopted form) | `event_log.rs:366-449` |
| Atomic `Arc` swap / CoW lease / CoW page snapshot | **DEFER — triggers unfired by this doc** | masterwork #44/#107/#110/#154 |
| `DeltaPatch`/`PatchOp` + ABSOLUTE-OP LAW + `LaneScopeReject` + `MAX_PATCH_OPS` | **NEW-small (this blueprint)** — frame-degenerate floor mandatory | §5, §8 |
| `KernelStateView` + `MAX_STATE_VIEW_BYTES` + M1 copy-in bench | **DEFER-WITH-TRIGGER** (first state-reading adapter) | §4, §8 |
| DELTA-DETERMINISM LAW + retained state-hash heartbeat | **NEW (law, not code)** — binds existing #52/R2 discipline to the delta-comparison claim | §7 |
| T1-T8 | **NEW** | §9 |
| Raw "mark kernel pages read-only for the adapter" work item | **NONE — dissolved** (read side is M1/M2/M3; write side already stronger than asked) | §3, §4 |
| STATELESS-ABSOLUTE default (Part 5 Q1) | **Law (this doc)** — restates kernel's own model; sparse-stateful DEFER-WITH-TRIGGER (netem-measured airtime pain) | §10.1 |
| Dynamic Squashing, telemetry tier | **ALREADY-EQUIVALENT** (= CWR Kalman `update()`) | §10.2 |
| Dynamic Squashing, raw patch layer + LAW-LANE UNSQUASHABILITY + gate-before-squash | **DEFER-WITH-TRIGGER** (no commit-rate pressure exists; seam = `BoundedDrainer`, built) | §10.2 |
| Atomic-Rollback-on-squash-failure | **ALREADY-EQUIVALENT** (degrade-closed restated; resync-storm bound = existing breaker) | §10.3 |
| Snapshot-Checkpointing | **EXTENSION of #151/#152/#153** — new delivery-target detail realized by `KernelStateView` refresh / reinstantiation; degenerates to nothing under the stateless default | §10.4 |
| "Git for real-time memory" analogy | **REPLACED** (single-patch-against-pinned-base framing; three structural corrections recorded) | §10.5 |
| T9-T12 | **NEW** (T12 conditional on §10.1's trigger) | §10.6 |

## 12. Provenance

Live-read in full this pass: `00-SOURCE-DIALOGUE.md` (all five parts; Part 4 `:234-265` with the
operator ruling `:236-239`; Part 5 `:270-320`, re-read after its append, no ruling attached),
`kernel/src/event_log.rs` (all 936 lines),
`BLUEPRINT-SELF-CERTIFYING-BRIDGE-CONTAINMENT.md` (whole),
`BLUEPRINT-MMU-ISOLATION-HEADER-STRUCT-RECONCILIATION.md` (whole),
`bebop2/wasm-host/src/lib.rs` (whole, at `/root/bebop2-verify-redteam/`),
`bebop2/core/src/anti_entropy.rs:1-130` (same worktree — grounding masterwork #59's citation),
`kernel/src/householder.rs:20-79` (the FMA runtime dispatch, grounding §7).
Read in relevant part: `BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md` items
#39-#62, #104-#113, #127, #154 + W3-L9/W4-L8 lanes;
`BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md:370-390` (FMA byte-determinism risk);
`math-first-architecture-blueprint.md:161` (native-vs-wasm32 transcendental divergence);
`R2-fail-operational-vs-degrade-closed-grounding.md` (integer money channel);
`R4-reed-solomon-fec-fit-grounding.md:202` (bit-identical RS reconstruction).
For the Part-5 addendum (§10), additionally: `BLUEPRINT-REED-SOLOMON-FEC.md` §2.4/§4 (the V5 §4
cellular profile 200-2000 ms RTT / 1-10 % loss, the supersedability property, the airtime-cost
and netem-calibration discipline), masterwork items #151-#155 (`:536-545` of the V2 synthesis,
read directly), and `kernel/src/bounded_drainer.rs:1-40` (live-read — the batch-drain seam).
Platform statements in §4 restate Fable-D §1's already-adjudicated ground truth plus the WASM
memory model (guest linear memory only; host state unreadable without copy-in or imports —
stated conservatively; the M1 cost figure is an order-of-magnitude estimate flagged for bench,
not a measurement). No product code touched; no commits; output confined to the round-2
directory as instructed.
