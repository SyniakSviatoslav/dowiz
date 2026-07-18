# OPUS Performance Audit — bebop2 mesh protocol (`/root/bebop-repo/`)

**Date:** 2026-07-18
**Auditor:** Opus (research pass, born from the Performance Standing Rule — atomicity & branchless, `dowiz/.claude/CLAUDE.md` §2026-07-18)
**Scope audited (by Read + grep):**
`bebop2/core/src` · `bebop2/proto-cap/src` · `bebop2/proto-wire/src` · `bebop2/delivery-domain/src` · `bebop2/mesh-node/src` · `crates/bebop/src` (confirmed legacy/dev-tooling, lighter pass — see §5).

**Method:** for-loop density map per file → targeted reads of the crypto verify path, HRW matchers, anti-entropy set-reconciliation, lock/atomic sites, and the heaviest numerical kernels. Every Big-O below was read from the actual source, not assumed.

**Bench inventory (repo-wide, load-bearing for prioritisation):** exactly **two** bench targets exist —
- `bebop2/core/benches/verify_lane.rs` (ML-DSA-65 + Ed25519 scalar-vs-lane verify, zero-dep timer) — the ONLY perf coverage of the crypto path.
- `crates/bebop/benches/criterion.rs` (`loop_cycle/benign`, `wire/benign`) — legacy TUI crate only.
- `bebop2/core/examples/bench_eigensolve.rs` — an *example*, not a criterion gate.

Everything else in scope is **unbenchmarked**. Prioritisation weight = (real-scale-risk × currently-uncovered-by-bench).

---

## 0. Bottom line

The mesh critical paths are in good shape. The **capability-verify path is O(1) per signature / O(chain-depth) per frame with no accidental super-linear scaling** (task item #3 — clean). The **HRW courier matcher is exactly O(n) weight + O(n log n) sort with weights precomputed correctly** (task item #1 headline — clean). The per-frame wire codec is linear.

Four genuine findings, none catastrophic, ranked below. The single highest-value, lowest-risk fix is **P1 (`MerkleDigest::add`)** — a real O(n²·log n) that scales with event-log size and has a one-line, strictly-better fix that mirrors code already in the same function.

**Branchless/atomicity: net action = NONE right now** — no candidate carries the benchmark the rule *requires* before a branchless/lock rewrite. The rule explicitly forbids blanket application; §4 records the candidates considered and why each is gated on a missing bench rather than actioned.

---

## 1. Complexity findings (ranked)

### P1 — `MerkleDigest::add` sorts the whole leaf vector on every insert → O(n²·log n) to build a digest
- **File:** `bebop2/proto-wire/src/sync_pull.rs:449-453`
- **Code:** `add` does `self.leaves.push(id); self.leaves.sort_unstable();` on every new id.
- **Real complexity:** each `add` is **O(n log n)**. Ingesting a batch of *m* new frames (`SyncNode::ingest`, `sync_pull.rs:596-621`, calls `merkle.add` per frame) is **O(m·n log n)**; building an n-event digest from empty is **O(n²·log n)**.
- **Bounded-small vs scales-with-data:** **scales with data** — `n` = event-log size, which grows with mesh/order volume. This is the anti-entropy fold path (`ingest`), exercised on every pull-sync.
- **Bench status:** **uncovered** — there is no sync/anti-entropy bench at all.
- **Recommendation (low-risk, strictly better):** remove `sort_unstable()` from `add` (leave it an O(1) `push`; dedup is already handled by the `seen: HashSet` at line 450). Sort inside `root()` on the clone it **already makes** at `sync_pull.rs:461` (`let mut level = self.leaves.clone();` → add `level.sort_unstable();`). `add` becomes O(1) amortised; `root()` stays O(n log n) (it already does an O(n) tree-fold). Net: `ingest` of a batch drops from O(m·n log n) to O(m) for the adds + one O(n log n) per `root()` *call* (roots are computed on-demand for digest comparison, not per-add). Order-stability of the root is preserved because the sort still happens before the fold. **Add a criterion bench for `ingest`/`root` to lock the win in** (verified-by-math culture).

### P2 — `hub_ring::ranked` recomputes the HRW hash inside the sort comparator → O(n log n) hashes vs optimal O(n)
- **File:** `bebop2/delivery-domain/src/hub_ring.rs:52-60` (and callers `owner_hub`/`is_owner`/`is_replica`, lines 78-92)
- **Code:** `ranked.sort_by(|a,b| hrw_weight(order_id,&b.pubkey).cmp(&hrw_weight(order_id,&a.pubkey))...)` — `hrw_weight` is called **twice per comparison**, i.e. **O(n log n)** FNV-1a hashes over 40-byte buffers, when each hub's weight need only be computed **once** (O(n)).
- **Contrast (the correct pattern already exists in-repo):** the sibling courier matcher `bebop2/proto-cap/src/matcher.rs:64-70` does the **decorate-sort-undecorate** (Schwartzian) transform correctly — maps to `(weight, key)` first, then sorts precomputed weights. `hub_ring::ranked` is a regression from that pattern.
- **Extra:** `owner_hub` (`hub_ring.rs:78-80`) calls `assign(...,0)` which runs a **full O(n log n) sort** only to take `ranked[0]` — an argmax that is O(n) with a single scan.
- **Real complexity:** O(n log n) hashes today; O(n) achievable. **Per-order** — `is_owner`/`owner_hub` are the "who owns this order?" checks, so cost multiplies by order volume.
- **Bounded-small vs scales-with-data:** `n` (hub count) is **bounded-small** (a mesh has ~3–50 hubs), but the call is **per-order** so aggregate cost scales with order throughput.
- **Bench status:** **uncovered.**
- **Recommendation:** rewrite `ranked` to precompute `(hrw_weight(order_id,&h.pubkey), h)` once, sort the tuples (exactly like `matcher::assign`); make `owner_hub` a single `max_by` scan (O(n), no sort). Zero behavioural change (same total order). Low-risk, mirrors existing blessed code.

### P3 — ML-KEM-768 production `poly_mul` is schoolbook O(n²), not NTT O(n log n)
- **File:** `bebop2/core/src/pq_kem.rs:296-327` (used by keygen/encaps/decaps at `pq_kem.rs:505, 521, 565, 627`)
- **Real complexity:** schoolbook ring multiply in R_q = Z_q[x]/(x²⁵⁶+1), **O(N²)** with N=256 → up to **65,536** inner iterations per poly-mul (early `continue` on zero coeffs mitigates only sparse inputs). ML-KEM-768 does K=3 poly-muls in inner products and K×K in matrix steps, so a keygen/encaps is on the order of **10⁵–10⁶** modular mults. A correct NTT would make each multiply **O(N log N)** (~2–3k ops) → roughly **~100× on this path.**
- **This is a deliberate, documented correctness-first choice**, not an oversight: `pq_kem.rs:329-335` records that a shipped NTT was found incorrect (forward/inverse were not a valid pair; basemul didn't reproduce schoolbook products) and was ripped out rather than ship a subtly-wrong fast path. `pq_dsa` (ML-DSA-65), by contrast, **does** use a verified NTT in production (`pq_dsa.rs:198 fn ntt`, `poly_pointwise_montgomery`) — so the two PQ schemes diverge here on purpose.
- **Bounded-small vs scales-with-data:** the KEM runs **per handshake / session establishment**, NOT per-frame. So it is bounded per-connection, not per-message — the blast radius is session-setup latency, not steady-state throughput.
- **Bench status:** **uncovered for KEM.** `verify_lane.rs` benches only ML-DSA-65 + Ed25519 *verify*; there is **no** KEM keygen/encaps/decaps bench.
- **Recommendation:** **measure before touching.** Add a criterion bench for `pq_kem` keygen/encaps/decaps to quantify real handshake cost. Only if handshake latency is proven to matter should an NTT be re-introduced — and the codebase's own note (335) sets the bar correctly: any NTT must ship with a verifier proving `intt(ntt(a))==a` **and** `intt(basemul(ntt(a),ntt(b)))==schoolbook(a,b)`. **Do NOT ship an unproven NTT** — correctness over speed is the right call until a bench says otherwise.

### P4 (LOW) — `SyncNode::pull` is an O(n) full-log scan per pull request
- **File:** `bebop2/proto-wire/src/sync_pull.rs:575-583`
- **Real complexity:** `for f in self.frames.values() { if f.seq > watermark ... }` — **O(n)** over the entire folded log per pull, n = log size.
- **Bounded-small vs scales-with-data:** scales with log size, but it is a **pull *response*** (periodic anti-entropy, not per-frame) and returns a batch, so amortised cost is modest.
- **Bench status:** uncovered.
- **Recommendation:** only if `pull-rate × log-size` is ever measured hot, add a secondary index `BTreeMap<actor, BTreeMap<seq, content_id>>` for O(log n + k) range scans from the watermark. Not worth it speculatively (ponytail/YAGNI) — note and move on.

### P5 (LOW / off mesh hot-path) — `linalg::charpoly` is O(n⁴)
- **File:** `bebop2/core/src/linalg.rs:104-128`
- **Real complexity:** Faddeev–LeVerrier with a **naive O(n³) `matmul` (`linalg.rs:75-91`) inside a `for k in 2..=n` loop → O(n⁴)** characteristic polynomial; `eigenvalues` (`linalg.rs:203`) builds on it via Durand–Kerner.
- **Bounded-small vs scales-with-data:** **bounded-small** — this is the sovereign math/cognitive core, and the eigensolve work was capped at n≤32 (Jacobi/Householder refactor, per project memory). O(n⁴) at n≤32 ≈ 10⁶ ops — tolerable but not pretty. **Off the mesh per-frame path** (runs in the deliberate/self-model loop).
- **Bench status:** an `examples/bench_eigensolve.rs` exists but is not a gate; `charpoly` itself is uncovered.
- **Recommendation:** note only. Do not optimise without a profile showing it on a hot path — it is not on one today.

---

## 2. Verified-clean paths (reported honestly — the sweep was NOT all findings)

- **`proto-cap::matcher::assign` (HRW courier matcher)** — `matcher.rs:63-73`. O(n) weight + O(n log n) sort, **weights precomputed once** (correct Schwartzian). Deterministic tie-break by pubkey. **No hidden O(n²).** Exactly the expected HRW complexity. `max.truncate` bounds output. ✓ (Task item #1 headline: clean.)
- **Capability-verify path is O(1)/sig, O(chain-depth)/frame** (Task item #3: clean):
  - `hybrid_gate::check` (`hybrid_gate.rs:124-209`) — cheap pre-checks (expiry) → `verify_chain` → red-line → revocation → classical Ed25519 verify → PQ ML-DSA verify → nonce record. Cost is dominated by the two signature verifies; everything else is O(1) or O(chain-depth).
  - `roster::verify_chain` (`roster.rs:252-316`) — **O(L)** links, each one Ed25519 `verify_signature` (dominant) + `is_subset_of` scope check. `is_subset_of` (`roster.rs:84-85`, `scope.rs:168`) is **O(g₁·g₂)** via `Vec::contains`, but g (grants per scope) and L (chain depth) are **bounded-small** (UCAN chains are shallow, grants are a handful). Crypto dominates. ✓
  - `pq_dsa::verify_internal_bytes_many` (`pq_dsa.rs:1063-1068`) — **O(n) batch, O(1)/sig**; `.map()` of independent full per-item verifies, AVX2 lane-parallelism confined to ExpandA (Keccak×4), **no cross-signature algebra** (not batch-accept). This is exactly the expected batch shape and does **not** scale worse than O(1)/sig. ✓ (This is the direct answer to task item #3's "flag if verify scales worse than expected" — it does not.)
  - `redline::check` (`redline.rs:97-125`) — on the verify hot path (`hybrid_gate.rs:151`) but O(scope_grants × allowlist × allow_grants), **all three bounded-small**. Note, not a risk.
- **`pq_dsa` (ML-DSA-65) production uses a verified NTT** (`pq_dsa.rs:198`) — O(n log n) multiply. ✓ (Contrast with pq_kem P3.)
- **`fft::fft` is a real radix-2 Cooley–Tukey FFT, O(n log n)** (`fft.rs:87`). The O(n²) `dft_oracle` (`fft.rs:155`) is **test-only** (independent verification oracle). ✓
- **`wire_codec::encode_frame`/`decode_frame`** (`wire_codec.rs:198-300`) — the per-frame codec is **linear** in fields + chain-length + payload. No nesting, no O(n²). ✓
- **`delivery-domain` intake/finalization, `node_id` identity/keygen** — for-loops are linear byte/field passes; no quadratic on the identity or intake paths. ✓

---

## 3. Core numerical kernels — heavy, but off the mesh per-frame path (context, not action items)

`core/src` is both the mesh core *and* the sovereign math/cognitive kernel. The heaviest files by loop density — `kalman.rs` (55 KB, 107 loops), `pq_dsa.rs`/`pq_kem.rs` (crypto), `field.rs` (38 KB), `lyapunov.rs` (32 KB), `dmd.rs` (28 KB, online DMD/SVD), `resonator.rs`, `micrograd.rs` — carry **inherent** matrix/graph/spectral complexity (Gram matrices O(n²·m), Householder/Jacobi SVD, streaming DMD updates O(n·r)). These run in the **deliberate / self-model loop, not the per-message mesh path**, and **none are benchmarked**. `dmd.rs:67-95` (Gram + QR-ish reduction) and `dmd.rs:259-300` (online update) are the densest. Recommendation: **characterize, don't optimise** — if the cognitive loop is ever shown (by a profile) to be latency-sensitive, add benches first. No evidence today that any of these is hot on a product path.

---

## 4. Branchless / atomicity sweep — candidates considered, hotness evidence, and why net action = NONE

The Standing Rule requires a **benchmark proving hotness + poor branch prediction / lock contention** before any branchless or lock rewrite, and explicitly forbids blanket application to cold paths. Applying that gate honestly:

| Candidate | Site | Hotness evidence | Verdict |
|---|---|---|---|
| `poly_mul` data-dependent branches (`if a[i]==0 continue`, `if b[j]==0 continue`) inside the O(N²) inner loop | `pq_kem.rs:299,304` | On the KEM handshake path; branches predict poorly on random-looking coefficients — **but no KEM bench exists**, and the *right* fix is algorithmic (NTT, P3), not branchless-ifying the schoolbook loop | **Gated on the missing KEM bench (P3). Do not branchless-ify a loop that should be replaced.** |
| `HybridGate.seen` single `Mutex<HashSet>` serialises the nonce-record step across concurrent frame verifies | `hybrid_gate.rs:67,194-206` | The lock is held only for an **O(1) HashSet insert AFTER the dominant crypto** (Ed25519 + ML-DSA verify, µs–ms). It is **not** the contended bottleneck, and **no bench proves contention** | **Considered and rejected** for lack of evidence. If a many-core node with tiny frames ever profiles the seen-lock as hot, a striped/sharded lock or a lock-free concurrent set is the move — *only then*. |
| `discovery` `Arc<Mutex<PeerDirectory>>` taken repeatedly (lock→to_wire→unlock, lock→merge→unlock) | `discovery.rs:150-153, 222-250, 338-356` | On the **periodic gossip/tick path (cold)**, not per-frame. No contention evidence | **No action.** Correct as written. |
| `montgomery_reduce`, FNV-1a `hrw_weight`, poly arithmetic | `pq_dsa.rs:73`, `matcher.rs:45` | Already tight branchless arithmetic loops; `montgomery_reduce` is already branch-free | **No opportunity.** Already optimal shape. |

**Honest conclusion:** the single most useful "atomicity/branchless" action is a **precondition, not a rewrite** — *write the missing benchmarks* (KEM handshake, sync ingest, hub-ring assign). Under the rule's own evidence bar, nothing qualifies for a branchless/lock rewrite today. The P1/P2 wins are **algorithmic** (remove redundant work), which the rule ranks above micro-optimisation anyway.

---

## 5. `crates/bebop` (23,776 lines) — confirmed legacy/dev-tooling, lighter pass

Confirmed by its own doc comment (`crates/bebop/src/lib.rs:1-10`): this is the **host-agent TUI logic** ("outfit, vault, copilot, multipilot, launch… the ratatui TUI binary"), explicitly distinct from the sovereign mesh core. It is **dev-tooling, off the mesh product path**, and carries its own criterion bench (`loop_cycle/benign`, `wire/benign`). Given the scope guidance to confirm relevance before deep effort, I did **not** deep-audit all 23 K lines. Spot checks:
- `cost_estimate.rs` — real graph algorithms (spatial k-d filter, Dijkstra `route:209`, Contraction-Hierarchies `build_shortcuts:157-190` with the expected O(V·deg²) preprocessing triple-loop). **Textbook-appropriate** for a routing engine; not a defect.
- `matcher.rs::match_orders:74-93` — O(orders × route-cost) by design (one `hybrid_route` per order over a radius-filtered graph). Expected shape, small graph.

No action recommended in this crate for this pass; it is not on the mesh critical path and its algorithmic code is appropriate for its purpose.

---

## 6. Prioritised action list

| # | Action | Risk | Scale-risk × uncovered | Effort |
|---|---|---|---|---|
| **P1** | `MerkleDigest::add`: drop the per-insert sort; sort in `root()` (already clones). **+ add an `ingest`/`root` bench.** | Very low (strictly better, order-stable preserved) | **High** — scales with log size, fully uncovered | ~5 lines |
| **P2** | `hub_ring::ranked`: precompute weights once (mirror `matcher::assign`); make `owner_hub` an O(n) `max_by`. | Very low (same total order) | Medium — per-order, uncovered | ~10 lines |
| **P3** | Add a criterion bench for `pq_kem` keygen/encaps/decaps. **Then** decide on NTT — only with a verified pair, never speculatively. | N/A (bench only) | Medium — per-handshake, uncovered; algorithmic ~100× *if* proven hot | bench first |
| **P4** | `SyncNode::pull`: add a per-actor seq index **only if** measured hot. | Low | Low (periodic, batched) | defer |
| **P5** | `linalg::charpoly` O(n⁴): note only; do not optimise off-hot-path. | N/A | Low (n≤32, off mesh path) | none |

**The sweep was thorough and is NOT padded** — the mesh critical paths (courier HRW matcher, capability verify, wire codec, ML-DSA verify) are genuinely clean and reported as such. The four real findings are algorithmic-and-cheap-to-fix (P1/P2) or measure-first (P3/P4), and the branchless/atomicity gate correctly yields no rewrite under the rule's own evidence requirement.
