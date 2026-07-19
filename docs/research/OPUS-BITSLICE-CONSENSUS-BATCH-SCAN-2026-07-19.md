# OPUS — Bit-Slicing / Batch-Classification Landing-Site Scan (bebop2 mesh)

> Research-only. No code written, no branch touched, no file modified except this doc.
> Scope: does dowiz/bebop2's mesh auth layer have a **real batch workload of many
> independent multi-valued classifications** large enough that bit-slicing (parallel
> bit-plane processing via `AND`/`OR`/`XOR` across a wide register / SIMD lane) would
> matter? All file:line citations re-resolved live against `/root/bebop-repo/bebop2`
> on 2026-07-19 (read-only). Tool-count + web-budget caveat at the end.

---

## §0 One-paragraph verdict (read this first)

**No current landing site. This is — like several other findings this session — a real,
established technique with no place to land in this specific codebase, for two independent
structural reasons that both have to be false for it to matter, and neither is.** (1) The
only genuine "many independent items processed together" hot path in the mesh
(`SyncPeer::ingest`, `sync_pull.rs:596-621`) is dominated by **per-item signature
verification**, which the earlier B4 pass already settled must stay one-at-a-time
(cofactorless single verify as sole acceptance authority) — the cheap downstream label is a
single bit, not a bottleneck. (2) The thing bit-slicing would classify in bulk — **multi-valued
per-node trust/reputation state** — **does not exist and is architecturally forbidden**: trust
in this mesh is binary (an anchor-rooted delegation chain verifies, or it does not), and
`NO-COURIER-SCORING` is a repo-wide CI-enforced hard rule that rejects any reputation / rating /
trust-rank / score field (`proto-wire/src/lib.rs:22-26`, `revocation.rs:25-26`, +8 more sites).
Every actual state enum in the codebase (`ClaimStatus`, `DeliveryStatus`, `KillState`,
`ApplyOutcome`) is a **per-item lifecycle FSM**, never a batch-classified population. Where a
real SIMD batch bottleneck *does* exist — the Keccak/SHAKE hash primitive — **the repo already
bit-slices it** (`core/src/keccak_x4_avx2.rs`, 4-way AVX2), which is the positive control proving
the team applies the technique exactly where a batch genuinely dominates. A hypothetical future
target is sketched in §5, but it requires a scale (thousands of nodes) and an access pattern
(periodic roster-wide sweep) the code neither has nor, for trust-state, intends to have.

---

## §1 The technique, stated precisely (so we don't chase the wrong thing)

Bit-slicing (a.k.a. SWAR — SIMD Within A Register) encodes each of *K* logical items' state
into **bit-planes**: plane *p* holds bit *p* of every item, packed one-item-per-bit-position
across a `W`-bit register (or SIMD lane). A ternary state {A, B, C} needs `ceil(log2(3)) = 2`
planes; a 4-way enum needs 2 planes; an 8-way enum needs 3. You then evaluate a classification
or a state-combining function once, branch-free, with ordinary bitwise `AND`/`OR`/`XOR`/`NOT`
over the planes — and it lands on **all `W` items simultaneously**. Parallelism factor = register
width: `W = 64` (scalar u64 SWAR), `256` (AVX2), `512` (AVX-512).

Two preconditions for it to pay, both necessary:

- **(P-scale)** `K` is large — hundreds to thousands processed *together* — so the amortized
  per-item cost of the packed op beats a scalar per-item branch. Below ~`W` items you never
  fill one lane; at `K = 2..3` it is pure overhead.
- **(P-cheap-per-item)** the per-item work being replaced is a **cheap branch/compare on
  already-materialized data**, not an expensive computation (a signature verify, an NTT, a hash)
  that dominates regardless. Bit-slicing accelerates the classification of *already-computed*
  states; it cannot accelerate computing them.

This is **not** "make crypto material low-bit" (settled, closed). It is batch-throughput for the
*labeling/combining* step over many independent decisions.

**Established precedent (grounding).** The canonical software reference is Eli Biham, *A Fast New
DES Implementation in Software* (FSE 1997) — the origin of software bit-slicing, cited by the
Wikipedia *Bit slicing* article as having "achieved significant gains in performance of DES by
using this method" (it processes `W` DES blocks in parallel across a `W`-bit word; deployed in
John the Ripper's bitslice-DES). The Wikipedia article confirms the mechanism ("multiple parallel
simple virtual machines using general logic instructions to perform SIMD... also known as SIMD
within a register (SWAR)") but, checked directly, carries **no quantified speedup figure** — so I
mark the parallelism-factor claim (`≈W`-way, minus overhead) as mechanism-true, and treat any
specific "×N" as order-of-magnitude, not a citation. Roaring Bitmaps (`roaringbitmap.org`) is the
production witness for the sibling technique (bitwise set ops on dense bit-chunks): its set
operations are "implemented as native bitwise AND/OR/ANDNOT on dense chunks," it is "often
hundreds of times faster than older formats" (vs. WAH/EWAH/Concise, **not** vs. scalar), and it
is shipped in Lucene, Elasticsearch, ClickHouse, Druid, and Spark. The exact AVX2 ops/sec figures
are in the Lemire et al. Roaring paper (arXiv:1603.06549), which I could not fetch this session
(web-search budget exhausted, see §6) — so the honest realistic range I carry below (**~4–30×**
branch-free at 256-bit width for the classify step, before Amdahl against the predicate-computation
cost) is drawn from established SWAR/bitset knowledge, **not** a live-verified benchmark row, and
is flagged as such.

---

## §2 Every batch / multi-item point in the mesh, enumerated (the actual scan)

Grep + read across `bebop2/proto-cap/src`, `bebop2/mesh-node/src`, `bebop2/proto-wire/src`,
`bebop2/core/src`, `bebop2/delivery-domain/src`. Production (non-test) callsites only unless noted.

| # | Site | file:line | Is it a batch of independent items? | Per-item work | Batch size (real) | Multi-valued state? |
|---|------|-----------|-------------------------------------|---------------|-------------------|---------------------|
| 1 | `SyncPeer::ingest(frames: &[SyncFrame])` | `proto-wire/src/sync_pull.rs:596-621` | **Yes** — the MESH-07 Sync·Pull log-segment catch-up | `f.verify()` = **signature + scope + content-id** per frame | unbounded (log catch-up can be large) | No — verdict is binary accept/reject → `IngestResult{added,dup,rejected}` counts |
| 2 | `sign::verify_many(reqs)` | `core/src/sign.rs:1000-1002` | Yes — lane-parallel INDEPENDENT-verify (BLUEPRINT-P-E §2.2 Mode 1) | full Ed25519 `verify` per req | N | No — `Vec<bool>` |
| 3 | `pq_dsa::verify_internal_bytes_many` | `core/src/pq_dsa.rs:1181-1234` | Yes — same, ML-DSA-65 | full lattice verify per req | N | No — `Vec<bool>` |
| 4 | `verify_chain(roster, chain, cap, now)` inner loop | `proto-cap/src/roster.rs:269` (`for link in chain`) | Sequential, **dependent** (link *i* binds to link *i-1*) | Ed25519 verify per link | **chain depth 1–3** | No — `Result<(), CapError>` |
| 5 | `HybridGate::check(frame, …)` | `proto-cap/src/hybrid_gate.rs:124-209` | **One frame at a time** by design | freshness → chain → red-line → revocation → Ed25519 → ML-DSA-65 → nonce insert | 1 | No — `CapResult<()>` |
| 6 | PoD quorum `DeliveryClaim` k-of-n | `delivery-domain/src/pod.rs` (`valid_signers`, `is_settled`) | Yes — count distinct valid signers | Ed25519+ML-DSA per hub sig | **k-of-n, single digits** (tests use 2-of-3) | No — settled/not-settled |
| 7 | `RevocationSet::is_revoked_key/_capability` | `proto-cap/src/revocation.rs:81-90` | No — single O(1) `HashSet` membership per call | hash lookup | 1 lookup/frame | No — binary present/absent |
| 8 | `RevocationSet::merge` | `proto-cap/src/revocation.rs:94-98` | `HashSet::extend` (union) | — | gossip delta size | No — set union |
| 9 | `mesh_consensus.rs` spectral peer-graph | `proto-cap/tests/mesh_consensus.rs` (**test only**) | Computes Fiedler λ₂ / SLEM over an N-node Laplacian | `f64` eigensolver (Faddeev-LeVerrier + Durand-Kerner) | N nodes, but it's a test | No — float eigenvalues, not per-node labels |
| 10 | `keccak_x4_avx2` | `core/src/keccak_x4_avx2.rs:1-129` | **Yes — and already SIMD-bit-sliced** (4× `__m256i` lanes) | Keccak-f[1600] permutation | 4-way | N/A (hash) — **positive control** |

**What the table says.** Only rows 1, 2, 3, 6 are genuine "many independent items together."
Rows 2/3 are already the built P-E Mode-1 batch APIs and are **verify** (settled). Row 6 is
single-digit `k`. Row 1 is the only large, real, product batch — and its per-item work is a
signature verify. **No row is a multi-valued state classification.** Every verdict column is
either binary (`bool` / accept-reject / present-absent) or a small `Result`. The state enums that
*are* multi-valued live entirely outside any batch (§3).

---

## §3 The multi-valued states that exist — all per-item FSMs, none batch-classified

- `ClaimStatus` (`proto-cap/src/claim_machine.rs:21-30`) — 4 states {Offered, Claimed, Released,
  PickedUp}. A per-claim coordination FSM. The file header (`:13-17`) states the structural
  constraint in prose: **"NO-COURIER-SCORING... The claim state carries no score / rating / trust
  / reputation / rank field."** This is the multi-valued enum closest to the task's "ternary
  Trusted/Untrusted/Indeterminate" shape — and it is deliberately *not* a trust label, and is
  handled one claim at a time.
- `DeliveryStatus` (`proto-cap/src/event_dict.rs:30`, `delivery-domain/src/lib.rs:42`),
  `KillState` (`mesh-node/src/kill_switch.rs:182`), `ApplyOutcome` (`mesh-node/src/hub_policy.rs:279`),
  `Outcome` (`core/src/deliberate.rs:69`), `EffectorReject` (`core/src/self_mod.rs:53`) — all small
  per-item enums, none evaluated across a large population in a hot loop.

**The decisive architectural fact.** The mesh does not maintain a per-node multi-valued *trust*
state at all. Trust = a signed capability chain rooted in an enrolled anchor (`node_id.rs:1-10`,
`verify_chain` at `roster.rs:250-290`). `NO-COURIER-SCORING` is enforced repo-wide with CI guards
and appears verbatim as a rejection rule at, among others: `proto-wire/src/lib.rs:22-26`
("Any PR that adds scoring will be rejected by the doc-claim gate"), `revocation.rs:25-26`
("acts on public keys and capability hashes... never on scores or reputation"),
`proto-wire/src/{handshake.rs:11-12, envelope.rs:9-10, error.rs:7, wire_codec.rs:17}`,
`ports/github/src/lib.rs:12-13` ("trust here is the shared secret (a capability), not
reputation"). This matches the standing memory line: *"trust = signed **capability**, NEVER
reputation/blacklist (rejected as echo chamber)."* **The target state bit-slicing would classify
in bulk is the exact state this codebase is committed to never holding.**

---

## §4 Connection & tension with the B4 crypto-bench finding (task item 2)

The B4 pass (`docs/design/agentic-mesh-protocol-2026-07-17/B4-crypto-groundtruth-bench-batching.md`,
landed on bebop `feat/b4-crypto-groundtruth-bench`, commit `6541ae8`) is about **batch SIGNATURE
VERIFICATION** — the operation that *computes* a trust decision. Its settled conclusions:

- Cofactorless single verify (`sign.rs::verify`, `S·B == R + k·A`, no small-order rejection) is
  the configuration where naive batch verification diverges from single (SSR-2020, "Taming the
  Many EdDSAs").
- As landed, `verify_batch` is **confirm-every-accept**: a small-order filter alone did **not**
  close the mixed-order gap (a real forgery slipped through it — regression test
  `sign.rs:1382 batch_rejects_ssr2020_mixed_order_forgery`), so every batch-accept is re-confirmed
  by a full single verify. Measured: `batch/64 = 131.2 ms` vs `64 × single = 40.3 ms` → **3.26×
  slower**. Batching the classical leg gives **NO throughput benefit** here; its only residual
  value is a sound fast-*reject*.

**The precise difference the task asks me to be careful about, and it is real:**

| Axis | B4: batch signature *verification* | Bit-slice batch *state classification* |
|------|-----------------------------------|----------------------------------------|
| Input | raw `(pubkey, msg, sig)` triples | **already-computed** labels/bits (post-verify) |
| Operation | elliptic-curve / lattice algebra | boolean `AND`/`OR`/`XOR` over bit-planes |
| Correctness risk | **Yes** — SSR-2020 forgery; batch-accept can admit what single rejects | **No** — no cryptographic algebra; a bitwise combine of booleans is exact and total by construction |
| Why no benefit *here* | soundness pin forces N re-single-verifies | **no large classify step exists to accelerate** |

So there is **no correctness tension** — bit-slicing labels carries none of the SSR-2020 hazard,
because it touches no signature algebra. But there **is a shared root cause for why neither lands**,
and it is the more important connection: **the expensive, un-batchable part is the per-item
verify; the resulting label is one bit.** In the one place a real batch exists (Sync ingest, §2
row 1), Amdahl's law kills the idea directly — you would be bit-slicing a step (the accept/reject
labeling) that is already ~free relative to the `f.verify()` that dominates each iteration.
Bit-slicing the cheap 1-bit outcome of an expensive verify saves nothing measurable. B4 removed
the batchability of the *verify*; bit-slicing would need a batchable *classification* downstream
of many cheap-to-produce states — and that downstream step does not exist at scale.

---

## §5 If a target ever emerged — concrete sketch (explicitly hypothetical, not a recommendation)

For completeness, the *shape* a real landing site would have to take, so a future reader can
recognize it if the mesh's scale/access-pattern changes:

- **The check:** a **periodic roster-wide status sweep** — classify every enrolled node into a
  small enum in one pass, e.g. `{Active, Expired, Revoked, Unknown}` (4-way → **2 bit-planes**).
- **The inputs (must already be materialized as per-node bit-vectors):** `expired_bit` =
  `!capability.is_fresh(now)` (`capability.rs`/`hybrid_gate.rs:134`), `revoked_bit` =
  `RevocationSet::is_revoked_key` (`revocation.rs:81`), `enrolled_bit` = `AnchorRoster::contains`.
  The 4-way label is a fixed boolean function of these three bits — exactly the branch-free
  bit-plane classification bit-slicing is for.
- **Batch size to matter:** `K ≳ W` = **hundreds to thousands** of nodes swept together; at 256-bit
  AVX2 you classify 256 nodes per packed op.
- **Expected speedup (honest, order-of-magnitude, not a measured row):** ~**4–30×** on the classify
  loop itself vs. a scalar match-branch, at 256-bit width — **but** only if the loop, not the
  predicate materialization (computing the three bits per node), dominates; and the three predicates
  are themselves cheap set/compare ops, so Amdahl headroom is thin even in the hypothetical.

**Why none of it holds today (each condition checked, each false):**
1. **Scale absent.** Genesis anchor rosters are single-to-double digit (`node_id.rs:117-142`
   loads a plain-text anchor file; `load_genesis` fails closed on zero; no thousands-of-nodes
   population anywhere). `K ≈ 3..20`, far below one AVX2 lane.
2. **Access pattern absent.** There is **no periodic roster-wide sweep**. Expiry is checked
   *lazily, per-frame, at gate time* (`is_fresh(now)`, `hybrid_gate.rs:134`); revocation is an
   inline O(1) `HashSet` lookup per frame (`revocation.rs:81`). Status is never materialized for
   the whole population at once — it is evaluated on the single frame in hand.
3. **Trust-state target forbidden by design.** For any *trust/reputation* variant of the sweep,
   `NO-COURIER-SCORING` (§3) rejects it at CI. Only the mechanical `{Active/Expired/Revoked}`
   lifecycle variant is even permissible, and that one has no scale (point 1) and no sweep (point 2).
4. **The team already bit-slices where it pays.** `keccak_x4_avx2.rs` (4-way AVX2 Keccak, P-E §2.3)
   is the existence proof that SIMD-batching lands here *when a real batch bottleneck exists* — the
   hash primitive, not the state. Its absence for state-classification is a signal, not an oversight.

---

## §6 Honesty ledger

- **Verdict: NO current landing site.** Real, established technique; no place to land in bebop2's
  mesh today. Both independent gates (§0) are shut, and for trust-state one of them
  (`NO-COURIER-SCORING`) is shut *on purpose and permanently*.
- **What would change the verdict:** a mesh at thousands of nodes **and** a new periodic
  roster-wide *mechanical-lifecycle* sweep (not trust/reputation). Absent both, this is a
  no-op — consistent with the session's pattern of "real technique, no landing site here."
- **Web-search caveat (must state):** the task asked for live WebSearch confirmation of precedent
  numbers; this session's WebSearch budget was **exhausted (200/200)** before I could run them, so
  the three planned searches (bitslice-DES speedup, roaring/AVX2 ops-sec, SIMD ternary classify)
  did not execute. I substituted **two `WebFetch` calls** — Wikipedia *Bit slicing* (confirmed
  Biham-1997/SWAR mechanism; **no numeric speedup on the page**) and `roaringbitmap.org` (confirmed
  native-bitwise-AND/OR/ANDNOT, "hundreds of times faster than older formats," production users;
  **no AVX2 ops/sec on the page**). The specific "×N" figures in §1/§5 are therefore
  **established-knowledge order-of-magnitude, explicitly not live-verified benchmark rows**, and are
  flagged inline as such. The one **measured** number in this doc — B4's 3.26× — is from the repo's
  own `docs/ledger/crypto-bench.jsonl` (commit `6541ae8`), re-cited from the B4 doc's independently
  re-derived audit, not re-run here.
- **All source file:line citations re-resolved live** against `/root/bebop-repo/bebop2` on
  2026-07-19 (read-only). No code written, no branch touched, no file modified except this doc.
