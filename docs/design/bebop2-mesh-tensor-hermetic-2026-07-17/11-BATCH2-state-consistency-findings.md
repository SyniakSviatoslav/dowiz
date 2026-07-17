# BATCH 2 — State / Consistency Layer: audit findings (2026-07-17)

> **Role:** research + audit (NOT implementation). Evaluates the state/consistency cluster of
> the pasted "Bebop2 mesh" brainstorm (`00-SOURCE-PROMPT.md`, `01-RAW-DIALOGUE-PART-A.md`)
> against the **live** dowiz + bebop2 code. Per operator override for this arc: **complexity /
> rewrite-cost is NOT a valid rejection reason — only physics/correctness is.** Every
> load-bearing claim carries a `file:line` citation and an epistemics tag; a claim I could not
> ground in live code is tagged as such rather than asserted (Anu). The build-order at the end is
> organized so the good outcome is forced by structure, not by a future reader's diligence (Ananke).
>
> **Cluster scope:** CRDTs · vector clocks · Merkle-DAG / Merkle Mountain Ranges · sparse/patch
> (delta) updates vs full snapshots · StateRoot hashing for divergence detection · Merkle-bisection
> for the exact divergent instruction · Hybrid Logical Clocks (HLC) · gossip epoch propagation ·
> "Sync-Debt" / unordered-patch buffering · rolling snapshot / checkpoint-restore with adaptive
> epoch length.

## Epistemics legend

- `[VERIFIED f:l]` — read the live source at that line this session; asserted content checked against it.
- `[DERIVED]` — a conclusion I reasoned out from cited code (e.g. a semilattice property from the merge fn), not a line I can point at verbatim.
- `[INFERENCE]` — plausible, cross-file inference; weaker than DERIVED.
- `[HISTORICAL-STALE]` — sourced from a dated doc that describes an EARLIER tree state; flagged, not carried forward as current truth.

---

## 0. The one fact that governs this whole cluster

**Both repos carry an explicit, load-bearing invariant that money/order state is event-sourced and
NEVER CRDT-merged.**

- `bebop-repo/bebop2/core/src/event_log.rs:4` — *"Design invariant: money is event-sourced, NEVER CRDT-merged."* `[VERIFIED]`
- `bebop-repo/bebop2/core/src/anti_entropy.rs:10-14` — *"The log is event-sourced (never CRDT-merged), so a fork can only be resolved by truncating to the divergence point and re-appending the authoritative suffix … forks are detected and reported but require an out-of-band reset."* `[VERIFIED]`
- `bebop-repo/bebop2/core/src/lib.rs:358` — same invariant restated at the crate root. `[VERIFIED]`
- `dowiz/kernel/src/event_log.rs:6-8` — the dowiz kernel event-log is content-addressed with the same "duplicate is a *structural* no-op" idempotency. `[VERIFIED]`

This is not a stylistic preference; it is the axis the whole cluster splits on. The mesh already
runs **two different convergence models side-by-side**, and every dialogue concept lands differently
on each:

| Model | Where | Semantics | Multi-writer? | Money-safe? |
|---|---|---|---|---|
| **`SyncPeer` set-union** | `proto-wire/src/sync_pull.rs:522-621` | grow-only set of signed, content-addressed events | **yes** (per-actor sub-chains, union of all) | commutative-only payloads |
| **`EventLog` linear chain** | `core/src/event_log.rs:66-172` | single ordered SHA3 hash-chain | **no** (single writer; fork ⇒ out-of-band reset) | money / ordered lifecycle |

`[DERIVED from the two cited modules]`

The dialogue treats "the mesh state" as one homogeneous thing to CRDT-merge. **It is not.** The
correct reading — which the code already encodes — is: the *commutative* half is already a CRDT; the
*non-commutative* (money/order-sequence) half is deliberately kept non-mergeable and only ever
"catches up," never "merges." The just-landed RCI ADR independently re-derived exactly this split
(`resolution-round2.md` F-1, §3–§5 of the ADR) — see §11.

---

## 1. CRDTs (conflict-free replicated data types)

**Verdict: ALREADY-EQUIVALENT for the commutative half · REJECT-on-correctness for the money/order half.**

### 1a. The commutative half is *already* a CvRDT (a signed G-Set), by construction

`SyncPeer` (`proto-wire/src/sync_pull.rs:522-527`) holds `frames: HashMap<content_id, SyncFrame>` +
`merkle: MerkleLog` + `max_seq: HashMap<actor, u64>`. Its merge operation is `ingest`
(`sync_pull.rs:596-621`): verify → skip if `content_id` already present (`:606-609`) → else insert
and `merkle.add` (`:610-618`). `MerkleLog::add` (`sync_pull.rs:449-454`) is an idempotent insert into
a **sorted set**. `[VERIFIED]`

That is precisely a **state-based grow-only set (G-Set) CvRDT**: the join is set-union, which is
commutative, associative, and idempotent — the three join-semilattice laws. `[DERIVED]` And it is not
merely designed that way, it is **proven** that way:

- `two_diverged_nodes_converge_identical_after_pull` (`sync_pull.rs:807-846`) — two offline-diverged
  nodes reach an **identical Merkle root** after bidirectional anti-entropy. `[VERIFIED]`
- `n_node_convergence_is_graph_fixed_point` / `..._seed_sweep` (`sync_pull.rs:1053-1180`) — a seeded
  adversarial schedule (3–6 nodes, ~50% link drops on intermediate rounds, duplicate delivery every
  step) ends with **every node's root == the root of the union of all authored content-ids**, and
  re-ingesting the union is a pure no-op. That is the CvRDT least-upper-bound property stated as a
  graph fixed-point and fuzzed over 24 seeds. `[VERIFIED]`

**Consequence:** the dialogue's "add CRDTs for eventual consistency" is, for the event-set layer,
already built, tested, and hardened. Adopting a CRDT *library* here would be a REJECT under DECART
(re-implementing a proven, zero-dep primitive) — but the operator's complexity override doesn't
apply because this isn't a complexity objection, it's a **duplication-of-a-proven-primitive**
objection (Anu: the decision "add a CRDT" is not derivable once the G-Set already exists and passes
its fixed-point test).

### 1b. The money/order half must NOT become a mergeable CRDT — this is correctness, not taste

The dialogue's speculative-execution / market-consensus turns implicitly want mergeable last-writer-wins
or counter CRDTs over order/settlement state. Applying CvRDT merge to the money chain is REJECT-**on-correctness**:

- The `EventLog` money chain is single-writer-ordered; `apply_pull` (`core/src/anti_entropy.rs:121-132`)
  **refuses** any pulled `seq` that doesn't continue the local chain (`:123-128` returns
  `EventLogError{reason:"fork/overlap"}`). A fork is *surfaced*, never silently reconciled. `[VERIFIED]`
- LWW/counter merge over settlement would re-introduce exactly the double-spend / conflicting-ledger-append
  hazard the bebop2 red-team named as the load-bearing unsolved money problem
  (`docs/red-team/2026-07-13/B4-architecture-decentralization.md:71-79`, F6/F7). `[HISTORICAL-STALE]`
  for the "no ledger exists" framing (a ledger chain now exists), but the *money-merge hazard* it
  identifies is timeless. `[VERIFIED that the fix-line names the exact split adopted]` — F6's own
  prescribed fix (`:73`) is *"CRDT for the commutative parts, BFT ordering for money,"* which is the
  split the code took (commutative set-union built; money kept ordered — BFT ordering itself is NOT
  built, see §7/§9).

**Falsifiable statement of the red-line:** a test that appends two conflicting `SettlementRecorded`
payloads at the same `seq` on two nodes and asserts they *merge to one value* would be a money-law
violation; the current code makes that test impossible to write green (`apply_pull` rejects it). Keep
it impossible.

---

## 2. Vector clocks

**Verdict: ALREADY-EQUIVALENT (as a per-actor sequence version-vector) · EXTEND only if causal-conflict detection is later needed.**

The `PullRequest.watermark: HashMap<[u8;32], u64>` (`sync_pull.rs:487-491`) is a **version vector**:
one component per actor pubkey, holding that actor's highest folded `seq`. `SyncPeer.max_seq`
(`sync_pull.rs:525`) maintains it; `make_pull_request` (`sync_pull.rs:587-591`) ships it; `pull`
(`sync_pull.rs:575-584`) returns exactly the events an actor is behind on (`f.seq > last`). `[VERIFIED]`

This is the *state-based anti-entropy* use of a version vector (compute the delta each peer is missing),
which is what the mesh actually needs. It is **not** used for causal-happens-before conflict detection —
and it doesn't need to be, because causal integrity is already carried two other ways:

- each actor's events form a `prev`-linked hash sub-chain (`SyncFrame.prev`, `sync_pull.rs:141`), so
  causal order *within* an actor is content-enforced; `[VERIFIED]`
- cross-actor events are commutative set members (they have no required order — §1a), so there is no
  cross-actor "conflict" to detect. `[DERIVED]`

**Where a fuller vector clock would earn its place (DEFER-WITH-TRIGGER):** if the mesh ever needs a
*single* object mutated by *multiple* actors concurrently (not the current model — each event is
authored by exactly one actor), you'd need per-object vector clocks to flag concurrent updates. Trigger:
a concrete multi-writer-single-object requirement lands (e.g. a shared claim edited by two couriers).
Until then, adding a full VV is undurable machinery for a conflict class the model structurally
excludes. `[INFERENCE]`

---

## 3. StateRoot hashing for divergence detection

**Verdict: ALREADY-EQUIVALENT (three independent implementations, all tested).**

Divergence detection by comparing a single root hash is fully built:

- **Crypto Merkle root over the event set** — `MerkleLog::root()` (`sync_pull.rs:457-481`): sorted
  content-id leaves, recursive `sha3_256(left||right)` pair-hash, odd leaf self-paired, empty ⇒ zero
  root. "Same folded set ⇒ same root" is the convergence fingerprint. `[VERIFIED]`
- **Rolling chain root** — `EventLog::root_hash()` (`core/src/event_log.rs:101-103`) = tip rolling
  hash `h_{n-1}`; two logs converge iff `root_hash` equal (asserted across `mesh_sync_integration.rs`
  tests, e.g. `:299-303`, `:372-379`, `:574-575`, `:592-593`). `[VERIFIED]`
- **Non-crypto peer-set root** — `PeerDirectory::snapshot_root()` (`proto-wire/src/discovery.rs:97-104`)
  = FNV-1a over id-sorted `(peer,endpoint)`; gossip convergence is detected by equal snapshot roots
  (`mesh_sync_integration.rs:688-709`). `[VERIFIED]`
- **Kernel-side store root** — `MemoryStore::snapshot_root()` (`dowiz/kernel/src/retrieval/memory_store.rs:85`,
  spec at `:31-36`): FNV-1a over length-framed `(key,value)` pairs, order-independent; pg/in-mem
  parity asserted (`:332-338`). `[VERIFIED]`

The dialogue's "StateRoot for divergence detection" is therefore ADOPT-is-moot. **One honest gap** is
consistency of choice: two of the four roots are FNV (fast, non-cryptographic → not tamper-evident),
two are SHA3 (tamper-evident). That's *correct as-is* (the FNV roots cover non-adversarial
convergence-fingerprinting: peer directories and local caches), but any future use of a root as a
*security* boundary must use the SHA3 variety. Flag, not a fix. `[DERIVED]`

---

## 4. Merkle-DAG / Merkle Mountain Ranges (MMR) for patch history

**Verdict: DEFER-WITH-FALSIFIABLE-TRIGGER (performance, not correctness) · one sub-part REJECT (a second content-addressed authority).**

What exists: a Merkle **tree over a sorted set** (`MerkleLog`, §3) and a hash **chain** (`EventLog`, a
degenerate DAG). What does NOT exist: an append-optimized accumulator (MMR) with O(log n) inclusion
proofs, and no Merkle-**DAG** of patches. `[VERIFIED — grep for `mmr|mountain|merkle.?dag` across
`bebop2` + `dowiz/kernel` returns nothing]`

The real, measurable inefficiency the MMR idea targets: `MerkleLog::add` re-sorts the whole leaf vector
on every insert (`sync_pull.rs:452`, `sort_unstable`) → O(n log n) per add, and `root()`
(`sync_pull.rs:457-481`) recomputes the entire tree from scratch on every call → O(n log n) per root.
For a mesh that computes a root on every anti-entropy round, that is O(n log n) per round where an MMR
gives O(log n) amortized append + O(1) cached root. `[DERIVED]`

- **DEFER-WITH-TRIGGER (MMR as a perf swap under `MerkleLog`):** the join-semilattice contract
  (§1a) is preserved by an MMR — it's the same set, cheaper. Trigger: a criterion bench (per the
  mandatory-telemetry doctrine, `AGENTS.md` "Mandatory native telemetry & benchmarks") shows
  `root()` + `add()` dominating an anti-entropy round at realistic n (e.g. n ≳ 10⁴ leaves). Until that
  number exists, this is premature. The operator's complexity override says "don't reject for
  rewrite-cost" — noted; the reason to DEFER here is **absence of a measured need**, i.e. Anu (the
  decision "build an MMR now" is not yet derivable from any evidence), not complexity. `[INFERENCE]`
- **REJECT-on-correctness (a NEW Merkle-DAG *authority* for patch history):** building a separate
  content-addressed patch-DAG on top of the already-content-addressed event log is the **exact
  construction the RCI Triadic Council just overturned** — round-1 Option C "rebuilt git's own
  content-addressed hash-DAG one level up," creating the dual-authority hazard
  (`docs/adr/ADR-realtime-change-intelligence.md:44-50`). `[VERIFIED]` A second authority that can
  silently desync from the first is a correctness defect regardless of how cheap it is. The mesh
  already *has* its Merkle-DAG: the per-actor `prev`-chains + the content-id set. Extend that; do not
  mint a parallel one.

---

## 5. Sparse / patch-based (delta) state updates vs full snapshots

**Verdict: ALREADY-EQUIVALENT at the event-shipping layer · out-of-cluster at the payload layer.**

Anti-entropy already ships **deltas, not snapshots**. `diff` (`core/src/anti_entropy.rs:75-107`)
computes the exact missing suffix `[pull_from, pull_from+pull_len)`; the puller requests only that
range (`mesh_sync_integration.rs:239-243`, `sync_pull.rs:575-584` for the set model). A full-snapshot
resync is never sent on the sync path. `[VERIFIED]`

Full-snapshot machinery also exists, but as the *at-rest / restore* path, not the *sync* path:
`EventLog::snapshot_payloads()` + `rebuild_from_payloads()` (`core/src/event_log.rs:115-134`), used by
`at_rest.rs`. `[VERIFIED]` That's the correct division (delta on the wire, snapshot on disk).

The dialogue's "sparse/patch tensor deltas (COO/CSR, Z-order)" operate *inside* a single event payload
(compressing the state a payload carries), which is the **tensor/algorithms cluster**, not this one.
No state-consistency change is implied here; cross-referenced, not adopted. `[INFERENCE]`

---

## 6. Merkle-bisection to find the exact divergent instruction

**Verdict: DEFER-WITH-FALSIFIABLE-TRIGGER (bandwidth/round-trip tradeoff) · REJECT the "reputation-weighted" framing (red-line).**

Not implemented. `[VERIFIED — no `bisect` symbol anywhere in `bebop2`/`dowiz/kernel`]` The current
divergence finder, `diff` (`core/src/anti_entropy.rs:75-107`), is a **linear O(n) scan** over the full
per-seq digest vectors (`:78-83`), and computing those digests already ships/recomputes O(n) hashes
(`digest`, `:35-48`). `[VERIFIED]`

Merkle-bisection's actual win: two peers locate the first divergent leaf in **O(log n) round-trips
exchanging O(log n) hashes**, *without* either side materializing or shipping the whole O(n) digest —
the classic Merkle-tree-diff (à la Dolev/Merkle range reconciliation). For the current design (small
local-first logs, batch pulls over one QUIC connection per direction —
`mesh_sync_integration.rs:28-33`) the linear scan is cheaper in *round-trips* and only loses on
*bandwidth*, which is not the bottleneck yet. `[DERIVED]`

- **DEFER-WITH-TRIGGER:** adopt Merkle range-reconciliation when a bench shows the per-round digest
  materialization (O(n) hashing in `digest`) or its transfer dominates, i.e. logs grow past the point
  where shipping the whole fingerprint each round is affordable. Falsifiable trigger: `digest()` cost
  or digest-bytes-on-wire > the anti-entropy round budget at measured n. `[INFERENCE]`
- **REJECT (correctness/red-line):** the dialogue pairs bisection with *"reputation-weighted bisection
  dispute resolution."* The reputation weighting violates the standing **NO-COURIER-SCORING /
  NO-AGENT-SCORING** red-line (`dowiz/kernel/src/event_log.rs:22-23`; ADR `:119-121`;
  `claim_machine.rs:13-17`). `[VERIFIED]` Bisection to find *where* two logs diverge is a neutral
  mechanism and is fine; weighting the outcome by an actor reputation score is not. Keep the mechanism,
  drop the weighting.

---

## 7. Hybrid Logical Clocks (HLC) for asynchronous epoch sync

**Verdict: REJECT-on-correctness for the physical-clock half · ALREADY-EQUIVALENT for the logical half.**

No HLC exists. `[VERIFIED — `hlc|hybrid.?logical|lamport` grep across core/proto-wire/proto-cap
returns nothing but unrelated wall-clock hits]` And the codebase has a **deliberate, structural
avoidance of wall-clock in the ordering/consensus path**:

- capability expiry uses a **monotonic counter, explicitly "no wall-clock dependency"**
  (`proto-cap/src/hybrid_gate.rs:104`); `[VERIFIED]`
- wall-clock (`SystemTime::now()`) appears only at the *transport* edge for freshness checks
  (`proto-wire/src/iroh_transport.rs:391`, `stdio_transport.rs:267-268`, `wss_transport.rs:606-611`),
  never as an ordering key inside the Law; `[VERIFIED]`
- event ordering is by **per-actor `seq` + content-address**, not timestamp (§1a, §2). `[VERIFIED]`

The dialogue's *own* determinism requirements section demands **"no `SystemTime::now()`"** in the
consensus path. HLC's defining feature is that it *folds physical wall-clock into the logical
timestamp*. Introducing HLC as an **ordering authority** would inject a non-reproducible, node-local
physical reading into the very path both the dialogue and the code insist must be deterministic —
that is a self-contradiction in the brainstorm, and adopting it would break the replay-determinism the
event-log gates on (`event_log.rs` idempotency + `verify` recomputation). REJECT-on-correctness. `[DERIVED]`

- **ALREADY-EQUIVALENT (the logical half):** HLC's *logical* component (per-node monotone counter that
  takes the causal max on receive) is functionally what per-actor `seq` + `max_seq` already provide
  for anti-entropy (§2). The mesh's "epoch" need — *which events has this peer causally seen* — is met
  without a clock. `[DERIVED]`
- **DEFER-WITH-TRIGGER (bounded physical time as an *advisory field only*):** if the product later
  needs wall-clock semantics (e.g. "offer expires in real 5 min" across nodes), carry it as an
  **advisory, signed, non-ordering** payload field, exactly as capability expiry already treats time
  (a value checked at the edge, never an ordering key). Trigger: a concrete real-time-bounded product
  requirement. This keeps time out of the Law. `[INFERENCE]`

---

## 8. Gossip-based epoch propagation

**Verdict: EXTEND-EXISTING (gossip is built and 3-node-proven; "epoch" is the missing thin layer).**

Full-roster anti-entropy **gossip** is implemented and tested end-to-end over real QUIC:
`GossipAgent` (`proto-wire/src/discovery.rs:148-255`) with `tick` (dial known peers, exchange rosters,
merge — `:222-254`) and `listen_loop`/`handle_conn` (`:295-374`); convergence proven in
`gossip_converges_3node` (`mesh_sync_integration.rs:608-721`) — three agents seeded one-peer-each
reach an identical `snapshot_root` purely by gossip, no DHT, no central registry. The design note is
explicit that this is *"NOT a DHT: periodic full-roster exchange between allow-listed peers"*
(`discovery.rs:5-7`). `[VERIFIED]`

What's missing is only the **"epoch"** notion itself: there is no monotonic epoch/round counter that
gossip carries and that nodes advance on. The transport is done; the payload concept is not. This is
EXTEND-EXISTING: an epoch is a small monotone integer (a Lamport-style max-merge counter — which is
deterministic, unlike HLC §7) gossiped alongside the roster, converging by `max`. It plugs into the
existing `merge`/`snapshot_root` path without new transport. `[DERIVED]` No correctness objection;
the only discipline is that the epoch counter stay **logical** (max-merge, no wall-clock — §7).

---

## 9. "Sync-Debt" / unordered-patch buffering for out-of-order delivery

**Verdict: DEFER-WITH-FALSIFIABLE-TRIGGER · with a real correctness caveat naming which model it applies to.**

The two models handle out-of-order delivery oppositely:

- **`SyncPeer` set model already tolerates arbitrary order** — `ingest` (`sync_pull.rs:596-621`) folds
  any verified event whose content-id is new, in any order; duplicates are no-ops. The n-node fuzz
  (`:1053-1180`) delivers under partition + duplication and still converges. So for the commutative
  half, "unordered-patch buffering" is **ALREADY-EQUIVALENT** — there is no debt to track, the set
  absorbs any order. `[VERIFIED]`
- **`EventLog` linear model refuses gaps** — `apply_pull` (`core/src/anti_entropy.rs:121-132`) errors
  if a pulled `seq != log.len()` (`:123-128`). It does **not** buffer a seq=5 event while waiting for
  seq=4; it rejects the batch. `[VERIFIED]` Today this never bites because the pull always fetches a
  *contiguous* suffix in order (`diff` returns a range; `sync_missing` replays it ascending —
  `mesh_sync_integration.rs:239-249`, note `:216-218` "arrive in ascending seq order — no forks").
  `[VERIFIED]`

So a genuine reorder-buffer ("Sync-Debt": hold gap-ahead events until the gap fills) is **only needed
if per-event push-gossip replaces contiguous batch-pull** on the ordered chain. That is not the
current transport. DEFER-WITH-TRIGGER: adopt a bounded reorder buffer for the linear chain when/if a
push-gossip path that can deliver seq-gaps lands. Falsifiable trigger: a test that pushes seq=5 before
seq=4 on the ordered chain and currently gets an `EventLogError` rather than eventual convergence.
`[DERIVED]` Caveat that must be written into any such buffer: it may reorder *within one actor's
chain* only up to the `prev`-link check; it must **not** become a back-door merge of a fork (which
stays out-of-band per §0/§1b).

---

## 10. Rolling snapshot / checkpoint-restore with adaptive epoch length

**Verdict: PARTIAL-EXISTING (checkpoint/restore built) · rolling-truncation DEFER-WITH-TRIGGER and currently BLOCKED by the append-only invariant · adaptive-epoch DEFER.**

- **Checkpoint + restore: built.** `snapshot_payloads()`/`rebuild_from_payloads()`
  (`core/src/event_log.rs:115-134`) is a full snapshot→rebuild that reconstructs the identical hash
  chain deterministically; `at_rest.rs` persists it. The RCI ADR's *"rollback = re-derivation"*
  (`ADR-realtime-change-intelligence.md:91-96`) is the same idea for the derived layer: recompute at
  any `--at <sha>`, idempotent because pure. `[VERIFIED]`
- **Rolling snapshot with log truncation: not built, and structurally blocked.** `EventLog` is
  **append-only with no truncation API** (`core/src/anti_entropy.rs:13` "`EventLog` exposes only
  append (no truncation)"). `[VERIFIED]` A rolling checkpoint that compacts old events into a snapshot
  and *drops* them would need a truncation primitive that intentionally does not exist — because
  truncation is how you'd hide a fork or lose money history. So this is DEFER-WITH-TRIGGER **and**
  gated: it cannot be added casually; it would require its own council pass (it touches the money
  red-line: what may be compacted, and proof that a compacted-away event's effect is preserved in the
  snapshot root). Trigger: measured replay/verify cost of a long chain exceeding budget. `[DERIVED]`
- **Adaptive epoch length: DEFER.** No epoch exists yet (§8); "adaptive length" is an optimization of a
  primitive not built. Sequenced strictly after §8. `[INFERENCE]`

---

## 11. Cross-cutting: extend the RCI "dual-keyed determinism" ruling — do NOT re-litigate it

The just-landed ADR (`docs/adr/ADR-realtime-change-intelligence.md`, PROPOSED, two full Triadic
Council RESOLVE loops, 0 unresolved CRITICAL/HIGH) already resolved the exact problem this cluster
circles — how to detect and reason about state divergence without minting a second authority. Per the
task's instruction, **extend it, don't reopen it.** `[VERIFIED]`

Its F-1 resolution (`ADR:8-9,63-74,163-170`) is the key reusable ruling for the dialogue's
StateRoot/divergence model: **split state by commutativity and key each half to its own authority.**

- structural/commutative half → keyed to a content hash `(git HEAD, tree)`, machine-invariant,
  historically replayable via `--at <sha>`; `[VERIFIED ADR:63-70,192-193]`
- stream/non-commutative half → a machine-local co-authority (`state.json`), **always fully
  re-folded** in canonical order `(timestamp, stream_id, line_index)` because it is non-commutative,
  keyed by `state_input_digest`, and *declared* non-reconstructible-at-a-historical-sha rather than
  falsely guaranteed. `[VERIFIED ADR:63-70,171-174,203-206]`

This is the same commutative/non-commutative split the mesh code encodes (§0). The dialogue's
"StateRoot for divergence + Merkle-bisection to the divergent instruction" should be **framed within**
this ruling: divergence detection on the commutative half is set-root comparison (§3, cheap, order-free);
divergence on the non-commutative half is first-differing-`seq` (`diff`, §6) with the honest admission
that the non-commutative half must be fully re-derived, not incrementally merged.

**And the ADR's most important warning for this cluster:** round-1 was overturned precisely for
*"rebuilding git's own content-addressed hash-DAG one level up"* — the dual-authority hazard
(`ADR:44-50`). Several dialogue ideas (a new Merkle-DAG of patches §4; a CRDT merge layer over money
§1b) would re-commit that exact error. The council already paid for this lesson; spend it, don't
re-earn it.

---

## 12. Cross-cutting: exactly-once semantics — a LIVE bug on the audited branch

The dialogue's patch/gossip model needs **exactly-once** application (a replayed patch must not
re-run its side effect). The kernel's mechanism for this is `commit_after_decide`
(`dowiz/kernel/src/event_log.rs:339-361`): dedup on the content-id, and *"a duplicate never re-runs
decide."* `[VERIFIED]`

**Finding (HIGH, verified this session):** on the branch under audit (`feat/harness-llm-backend`),
`commit_after_decide` computes the dedup id from the raw event but then persists via `self.append(ev)`
(`event_log.rs:359`), and `append` rebinds a zero `prev` to the current tip
(`event_log.rs:297-301`) — storing under a **different, chain-bound** id than the raw id the dedup
check tested. Therefore, an event originally committed onto a **non-empty** log, then replayed,
misses the dedup check, **re-runs `decide`, and double-commits.** `[VERIFIED — `grep` confirms
`self.append(ev)` at :359 and the non-empty-replay regression test is ABSENT on this branch]`

The existing test `dup_event_is_idempotent_no_state_change` (`event_log.rs:532-561`) does **not** catch
it because it commits the original onto an *empty* log (tip `None` ⇒ no rebind ⇒ stored under the raw
id). The bug only manifests on a non-empty log. `[DERIVED]`

The fix already exists in the sibling worktree: `dowiz-agentic-mesh` (branch
`feat/agentic-mesh-protocol-2026-07-17`) `kernel/src/event_log.rs:380` uses `self.append_raw(ev)`
(keys dedup and storage on the same stable raw content-id, no rebind) and adds the regression test
`commit_after_decide_replay_on_nonempty_log_is_true_duplicate` (`:582-673`). The doc comment there
(`:339-356`) names it the **P07 §2** defect and ties it to the money-law red-line: *"a replayed
`SettlementClaimed` must never re-run its hashlock side effect."* `[VERIFIED via diff of the two files]`

**This is the single highest-leverage state-consistency fix available**, because every gossip/patch
concept above assumes exactly-once, and the primitive that guarantees it is currently broken on the
mainline branch. It is a port of an already-written, already-tested fix — not new design.

---

## Prioritized build-order (smallest, most-load-bearing kernel abstraction first)

Ordered by real dependency (Anu-derived, not draft order) and by "the good outcome is forced, not
hoped" (Ananke). Each item names a **falsifiable done-check**. This is a build *order*, not a
blueprint — the blueprint is out of scope per the task.

1. **Port the `append_raw` exactly-once fix + its regression test to `feat/harness-llm-backend`
   (§12).** Foundational: every later item assumes exactly-once. Done-check: the non-empty-log replay
   test (`commit_after_decide_replay_on_nonempty_log_is_true_duplicate`) is present and green on this
   branch; `decide` call-count == 1 across a replay; `log.len()` unchanged. **Correctness red-line
   (money) — not a perf item.**

2. **Write the invariant down as an executable negative test: "money/order state never merges" (§1b).**
   Before adding any new sync capability, pin the red-line so a later CRDT-merge PR fails loudly.
   Done-check: a test that attempts to reconcile two conflicting same-`seq` `SettlementRecorded`
   payloads asserts `apply_pull` returns `EventLogError{fork/overlap}` (impossible to merge-green).

3. **Add a logical epoch counter to the gossip payload (§8, EXTEND-EXISTING).** Smallest new
   abstraction: a `u64` max-merge counter riding the existing `GossipAgent`/`snapshot_root` path.
   Done-check: a 3-node variant of `gossip_converges_3node` where nodes start at different epochs and
   all converge to `max`, with the epoch reflected in a root; **assert no `SystemTime` on the path**.

4. **Bench `MerkleLog::add`/`root` and `anti_entropy::digest` to get the numbers §4/§6 gate on
   (mandatory-telemetry doctrine).** This is the *evidence-gathering* step that turns two DEFERs into
   either a GO or a documented "not yet." Done-check: a committed criterion bench + `baseline.json`
   entry for root-recompute and digest cost at n ∈ {10², 10³, 10⁴}; regression-gated per
   `bench_track.py`.

5. **(Conditional on step 4 numbers) MMR swap under `MerkleLog` (§4, perf).** Only if step 4 shows
   root/add dominating a round. Done-check: MMR passes the *existing* `n_node_convergence` fixed-point
   suite unchanged (same set semantics) **and** the step-4 bench shows the improvement; DECART report
   inline (new internal structure, even if zero-dep).

6. **(Conditional on step 4 numbers) Merkle range-reconciliation replacing the linear `diff` scan
   (§6, perf/bandwidth).** Only if digest materialization/transfer is shown to dominate. Done-check:
   O(log n) hashes exchanged to locate the first divergent leaf on a synthetic large log, converging
   to the same result as the linear `diff`; **no reputation weighting anywhere** (red-line).

7. **(Deferred, gated) reorder buffer for the linear chain (§9) — ONLY if a per-event push path lands.**
   Done-check: seq=5-before-seq=4 push converges instead of erroring, **without** ever merging a fork.

8. **(Deferred, council-gated) rolling checkpoint + truncation (§10).** Blocked on the append-only /
   money red-line; requires its own council pass. Not before a measured replay-cost trigger.

**Explicitly NOT on the list (REJECT-on-correctness, from above):** mergeable-CRDT (LWW/counter)
semantics over money/order state (§1b); a new parallel Merkle-DAG *authority* for patch history (§4,
dual-authority hazard); HLC / wall-clock as an ordering key (§7); reputation-weighted dispute
resolution (§6, NO-AGENT-SCORING red-line).

---

### Provenance / least-confident notes (the 2-question ritual, applied to this audit)

- I did **not** run the bebop2 or kernel test suites this session — I read the tests and cite what
  they *assert*; I did not re-confirm they currently pass on HEAD. The exactly-once finding (§12) is
  the exception: I verified the buggy `append` call site and the missing test by direct `grep`, so
  that one is ground-truth, not test-run-confirmed. `[stated limit]`
- The B4 red-team doc (§0/§1b citation) is **`[HISTORICAL-STALE]`** — dated 2026-07-13, describing a
  tree where "no ledger exists"; the ledger/event-log/claim-machine code I read post-dates it. I used
  it **only** for its money-merge *hazard framing* (which is timeless) and its F6 fix-line, never as a
  current inventory. `[stated limit]`
- Biggest thing I might be missing: whether a *third* consistency model (beyond `SyncPeer` set-union
  and `EventLog` chain) is used in the production node binary wiring (which is out of both crates I
  read). The two models are the crate-level truth; the node binary's exact composition of them was
  not read. If the node binary merges them in a way I didn't see, §2/§9's "no multi-writer-single-object"
  claim would need re-checking. `[stated risk]`
