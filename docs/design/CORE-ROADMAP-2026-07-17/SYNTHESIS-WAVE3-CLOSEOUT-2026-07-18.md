# SYNTHESIS — Wave-3 Closeout: folding the six late-landing 2026-07-18 items into the P75–P89 plan (P90–P91 proposed)

> **Planning document — writes no product code.** Third and final synthesis of the 2026-07-18
> performance/physics research wave. Companion to and EXTENSION of
> `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` (S1: Tiers A–E, P75–P83) and
> `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` (S2: P85–P89, divergence ledger,
> money/oracle exclusion). Both stand unchanged; this pass folds in the SIX research/status items
> that landed after S2 was written, proposes **P90** and **P91** as new blueprint units (scope
> only — the full 20-point blueprints per `CORE-ROADMAP-STANDARD-2026-07-17.md` are a follow-up
> writing pass), refreshes P85's live status, and gives items 5 and 6 permanent closed-with-
> evidence records so no future session re-litigates them. P84 remains reserved (D-1 golden
> digest, operator-gated).
>
> **Honesty preserved:** two of the six items are genuine, well-evidenced "no" answers (BitNet
> quantization; trust-boundary crypto removal). They are recorded as CLOSED with the evidence
> that closed them — negative results are load-bearing deliverables here, exactly as in S1 §6.

---

## 0. Inputs (all six read/verified in full this pass)

| # | Item | Source (verified live) |
|---|---|---|
| I1 | ML-KEM NTT implemented in bebop2, `--no-verify` bypass | `docs/research/OPUS-PERF-NTT-IMPLEMENTATION-2026-07-18.md` + bebop-repo branch state |
| I2 | thunderdome `slot_arena.rs` landed local-only on dowiz main | `docs/research/OPUS-PERF-ARENA-DEEPDIVE-2026-07-18.md` §6 (commit `a857cd71a`) |
| I3 | Contention-bench verified fixes (R17 — previously ABSENT per S2 §0) | worktree `/root/dowiz-perf-contention`, local branch `perf/contention-bench-2026-07-18`, commits `8c865805b` + `8256dbffb`; `docs/research/OPUS-PERF-CONTENTION-BENCH-RESULTS-2026-07-18.md` + `docs/research/bebop-bus-G-C1-fix.patch` (both exist **only in that worktree/branch**, not on main) |
| I4 | Kernel `pq/kem.rs` wrong-ring investigation (dedicated follow-up) | `docs/research/OPUS-KEM-RING-BUG-INVESTIGATION-2026-07-18.md` |
| I5 | BitNet/ternary quantization landing-site scan | `docs/research/OPUS-TERNARY-BITNET-QUANTIZATION-SCAN-2026-07-18.md` |
| I6 | Trust-boundary closed-channel scan | `docs/research/OPUS-TRUST-BOUNDARY-CLOSED-CHANNEL-SCAN-2026-07-18.md` |

---

## 1. Status table (the one-glance closeout)

| # | Item | Status | Disposition |
|---|---|---|---|
| I1 | bebop2 NTT (`986646a`, hook-bypassed) | **DONE-LOCAL-UNPUSHED · PROCESS-RED** | Covered by **P85** (S2 §4.1) — adequate in scope; live status refreshed in §4 below (still unpushed; now sits at the BASE of the shared `perf/bus-contention-2026-07-18` branch; its tree's pre-existing C3 red state is demonstrably blocking other verified work) |
| I2 | thunderdome `slot_arena.rs` (`a857cd71a`, dowiz main local-only) | **DONE-LOCAL-UNPUSHED · NEEDS-OPERATOR-DECISION (push)** | No new P-number — it is P86's substrate, already registered in S2 §2 row 1 / §7 E3′. Recorded here as DONE-LOCALLY; the only open action is the push decision (§6) |
| I3 | Contention-bench verified fixes | **DONE-LOCAL-UNPUSHED (branch) · NEEDS-BLUEPRINT → P90** + one NEEDS-OPERATOR-DECISION (GCRA) | §2 below. Discharges S2's R17 fold-in obligation |
| I4 | Kernel `pq/kem.rs` wrong ring + false compliance claims | **NEEDS-BLUEPRINT → P91** (fix-before-wiring; NOT an active incident) + one NEEDS-OPERATOR-DECISION (docs-only header sub-item) | §3 below. Upgrades the S2 §7 "NEW / FLAGGED, own red-line lane" row to a numbered unit |
| I5 | BitNet b1.58 ternary quantization | **CLOSED-NO-ACTION** | §5.1 — no landing site exists, by policy and by scan; permanent record |
| I6 | Trust-boundary closed-channel crypto removal | **CLOSED-NO-ACTION (design validated)** | §5.2 — the principle is already correctly applied; the one channel that spends heavy crypto provably needs it; permanent record |

**Unpushed-artifact ledger (push hygiene, consolidated — flagged for the operator-facing
session, not performed by this research pass):**

| Artifact | Where | Push/merge state |
|---|---|---|
| `986646a` (NTT) | bebop-repo, base of branch `perf/bus-contention-2026-07-18` | unpushed; P85-quarantined |
| `a857cd71a` (slot_arena) | dowiz `main` (local) | unpushed |
| `8c865805b` + `8256dbffb` (contention fixes + docs) | worktree `/root/dowiz-perf-contention`, local branch `perf/contention-bench-2026-07-18` | unpushed, unmerged — per the worktree/remote-push precedent (confirmed data loss, 2026-07-18), the branch should be pushed to remote promptly; worktree dirs are disposable, the pushed branch is truth |
| `bebop-bus-G-C1-fix.patch` | file in that worktree's `docs/research/` | uncommittable in bebop-repo until C3 resolves (§2) |

---

## 2. P90 — Contention-Bench Verified Fixes (proposed; scope for the blueprint-writing pass)

**What it is.** S1 E12 ruled the three flagged Mutex sites GATED-bench-first, and S2 §0 R17
recorded the contention-bench report as ABSENT with a fold-in obligation. The report has now
landed (worktree-only, I3) and it did the work properly: real multi-threaded contended
benchmarks (N ∈ {1,2,4,8} threads on ONE shared object, `kernel/benches/contention.rs`, new
`[[bench]]`), acted only where numbers justified it, and produced two evidence-backed
non-findings alongside the fixes. P90 registers the results as roadmap truth and carries the
three open ends.

**Measured results (from `OPUS-PERF-CONTENTION-BENCH-RESULTS-2026-07-18.md`, commit
`8c865805b`; kernel `cargo test --lib` = 637 green on the branch):**

| Site | Decision | Numbers |
|---|---|---|
| `budget.rs` (S1 A2/E12) | **FIXED** — `Mutex<f64>` → lock-free `AtomicU64` CAS; ceiling re-checked every retry (degrade-closed preserved); new falsifier `budget_atomic_never_over_grants` (8 threads, exactly-ceiling grants) | **2.0×** @1t, 1.28× @2–4t, tie @8t; no regime where the Mutex wins |
| `token_bucket.rs` (S1 A1/E12) | **PARTIAL FIX** — monotonic clock read hoisted outside the lock; same algorithm, same over-grant invariant, zero test change | **+6–18%** under contention |
| GCRA lock-free rewrite of token_bucket | **BENCHED, NOT SHIPPED — OPERATOR-GATED** | **1.3–3.6×** (3.66× @8t) — but it is an algorithm swap on a DoS/rate-limit SECURITY primitive, and the realistic dispatch path (one `try_acquire` then a long LLM call) has low real contention (@1t the gap is only 1.29×) |
| admission seen-set (S1 A3) + bebop `hybrid_gate` seen-set (S1 E10) | **NO ACTION — contention proven negligible** | raw lock is contended (sharded ~9× faster @8t) but with realistic per-frame crypto (~3µs stand-in, 10–50× cheaper than a real verify) before the O(1) insert, mutex and 16-shard set CONVERGE at every thread count. E10/E12's caution vindicated by measurement |
| bebop bus publish (S1 A3-Tier-A / P76 G-C1) | **FIXED + VERIFIED, COMMIT-BLOCKED** — snapshot-`Arc`-handles-under-lock, dispatch outside; re-entrancy-no-deadlock + order-preservation tests; bebop lib 443 green | could NOT be committed: bebop HEAD (`986646a`) is in a **pre-existing C3 HARD-law-red state** (`scripts/ci-no-ungated-keygen.sh` fails on ungated constant-seed `pq_dsa::keygen`/`pq_kem::keygen_internal` — unrelated to the bus change; a clean worktree at HEAD with zero crypto edits still trips it), and `--no-verify` was correctly denied by the environment's permission classifier. Preserved as applyable `bebop-bus-G-C1-fix.patch` |

**What remains (P90's open ends — the blueprint must carry all three):**

1. **OPERATOR DECISION — GCRA swap.** The 3.6× is real but only under a non-representative
   8-way hammer; shipping an algorithm change on a security primitive for that win is the
   operator's call, per never-bypass-human-gates. The bench (`contended_token_bucket/gcra_atomic`)
   stands ready as the evidence either way. Default if unruled: NOT shipped.
2. **Push/merge decision on `perf/contention-bench-2026-07-18`.** The budget CAS fix, the
   token_bucket clock hoist, the contention bench harness, and the results doc all exist only on
   that local branch. Until merged, dowiz main still runs the slower Mutex code AND lacks the
   contended benches that S1 P80-C1 calls for — merging satisfies P80's contended-bench sub-item
   as already-done (the blueprint-writing pass for P80 must NOT re-specify those benches).
3. **Resolve bebop's pre-existing C3 keygen gate before the bus patch can land.** The C3 red
   state (ungated pq_dsa/pq_kem keygen — open operator/council-gated crypto work, predating this
   wave) currently freezes ALL hook-respecting commits on the branch. Until it is resolved (or an
   explicit operator `--no-verify` ruling is recorded), P76's bus-fix half stays a patch file.
   **P76's blueprint must absorb the patch rather than re-implement** — the fix is done and
   verified; only landing is blocked.

**Ledger deltas this registers (cite, don't re-derive):** S1 **E12** → exercised: benches now
exist; budget shipped (on-branch), token_bucket partial shipped, GCRA operator-gated. S1
**E10** (`HybridGate.seen`) → upgraded from "no evidence" to "measured negligible". S2 §0
**R17 fold-in obligation → DISCHARGED** (feeds P88's CPU-domain boundary exactly as S2 §4.4
anticipated: the data moves specific CPU sites, the GPU-domain default is untouched).

**DoD sketch (for the writing pass):** branch merged (or an explicit no-merge ruling recorded);
GCRA ruling recorded either way; C3-resolution path named with owner; P76/P80 blueprints
cross-referenced so neither re-does landed work. **Depends on:** operator rulings only.
**Blocks:** nothing (P76's bus half is release-blocked by C3, not by P90).

---

## 3. P91 — kernel `pq/kem.rs` Ring Correction (proposed; scope for the blueprint-writing pass)

**What it is.** The dedicated follow-up (I4) CONFIRMED by live source read what R11 §1.2 first
flagged: dowiz's OWN `kernel/src/pq/kem.rs` (separate codebase from bebop2's `pq_kem.rs`) is
**not ML-KEM-768 / FIPS-203** — it implements the **cyclic** ring `Z_q[x]/(x²⁵⁶−1)` (complete
8-layer NTT, ROOT=17, pointwise mul; test at `kem.rs:429` uses `(i+j) % N` with no sign flip)
instead of the negacyclic `x²⁵⁶+1`, uses **η1=3** (the ML-KEM-512 value; 768 requires 2), and
emits a 1536-byte ciphertext (spec: 1088, du=10/dv=4 packing). It is internally self-consistent
— its own round-trip tests pass trivially in the wrong ring, which is exactly why its suite
cannot see the bug.

**Blast radius (decisive, verified):** NOT live. The whole `pq` module is feature-gated
off-by-default; no dependent crate enables it; the consumer chain `kem.rs → hybrid.rs →
volume.rs` terminates with zero callers of `volume.rs` anywhere in the tree. It touches none of
cert issuance, signing, auth, RLS, money, or orders (the `hybrid` on those paths is the
Ed25519⊕ML-DSA *signature* seam, a different primitive). **Fix-before-wiring priority — not an
active incident.**

**The severity amplifier (the trap):** the file's header (`kem.rs:1-12`) claims *"ML-KEM-768
(FIPS 203) … NTT is provably a ring isomorphism … Upgrade path: none needed"*, and the
introducing commit `0a85184b0` claims *"107 KAT tests byte-exact vs NIST ACVP (… ML-KEM-768)"*.
**Both claims are false for the KEM:** `kat/acvp/` contains ML-DSA vectors ONLY; **no KEM ACVP
vector or KEM KAT exists anywhere in the repo.** Anyone who trusts the header and wires
`volume.rs` ships a non-standard, unvetted lattice scheme believing it FIPS-203.

**Scope (three parts, in order):**

1. **P91.0 — immediate, near-zero-risk, SEPARABLE sub-item (flagged for operator decision):**
   correct the false compliance comments in the `kem.rs` header only — strike "FIPS 203 /
   ML-KEM-768 / ring isomorphism / Upgrade path: none needed", add a prominent
   `// NOT FIPS-203: cyclic ring + η1=3, do NOT wire — see OPUS-KEM-RING-BUG-INVESTIGATION-2026-07-18.md`.
   **This changes zero behavior — it only removes a false claim — so it is safe even under a
   code-freeze.** The distinction is flagged explicitly so the operator can approve this
   comment-only defusal independently of (and ahead of) the real fix; it still goes through the
   file owner because even a comment on a red-line file deserves eyes.
2. **P91.1 — the real fix (red-line lane):** replace the ring layer with correct negacyclic
   arithmetic and set η1=2, fix the du=10/dv=4 ciphertext packing (1088 B), reconcile the
   one-seed vs two-seed (`d`,`z`) FO structure. **Port bebop2's now-proven code rather than
   re-derive** — its `poly_mul` (schoolbook negacyclic) or its exhaustively-proven NTT
   (`poly_mul_ntt`, 0/65 536 basis-pair mismatches). Porting is gated on **P85 completing
   first** (the bebop2 NTT is itself process-quarantined until its bypassed review is
   remediated — building the kernel fix on an unreviewed crypto artifact would compound A4).
   Adopting/sharing bebop2's module wholesale is in-scope as an alternative to patching, since
   the divergence is structural, not parametric.
3. **P91.2 — the gate:** ship RED first. Real NIST ACVP ML-KEM-768 vectors (encaps/decaps) into
   `kat/acvp/` + byte-exact tests + a negacyclic-wrap KAT (`x²⁵⁵·x == −1`), and the **same
   3-model review rigor as the bebop2 NTT work** (which P85 exists to enforce). "Its own tests
   pass" is inadmissible as evidence — that is the self-consistency trap that hid this bug.

**DoD sketch:** header claims corrected (P91.0); kernel KEM byte-exact against real ACVP
ML-KEM-768 vectors; negacyclic-wrap KAT green; review attestations on file; `volume.rs` remains
un-wired until all of the above. **Depends on:** P85 (for the port source), operator ruling on
P91.0's early execution. **Blocks:** any future wiring of `pq::volume` / at-rest volume crypto
(P2/D4 lane).

---

## 4. P85 status refresh (delta to S2 §3/§4.1 — the entry itself stands, scope unchanged)

S2's P85 (NTT red-line process remediation) is **adequate in scope** — re-run the four skipped
deterministic gates, execute the 3-model review for real, or escalate for recorded retroactive
sign-off; quarantine D-9 wire-in until closed. Nothing in the new evidence changes that recipe.
Three live-status facts are appended so citations stay current:

1. **Still unpushed, still unremediated.** `986646a` remains at the base of bebop-repo branch
   `perf/bus-contention-2026-07-18` with no upstream; `.review/` still holds no attestation for
   it. Nothing has landed on top of it *through the hooks* — and nothing can (fact 3).
2. **The branch is now SHARED.** The contention-bench lane (I3/P90) targets the same branch for
   its bebop-side work. The bypass commit is no longer an isolated tip — it is the base other
   verified work wants to land on, which raises P85 from "clean up before wire-in" to "clean up
   before the branch can carry anything else."
3. **The tree's pre-existing C3 red state is now demonstrably blocking verified work.** The bus
   G-C1 fix (443 tests green) could not be committed because `986646a`'s tree fails
   `ci-no-ungated-keygen.sh` on pre-existing ungated keygen — an issue that predates and is
   unrelated to the NTT commit, but which the `--no-verify` bypass sailed past without
   surfacing. Had the NTT commit gone through the hooks, C3 would have been forced into the
   open a day earlier. This is the concrete, non-hypothetical cost of the bypass, and resolving
   C3 (operator/council-gated) is now a named precondition shared by P85's follow-up commits
   and P90's bus patch.

**Binding consequence (restated):** R11/I1 remains a *blocked* item, never a *completed* one,
in every citation until P85's DoD is met. The D-9 wire-in trigger still reads: P82 bench
evidence AND operator sign-off AND P85 complete. P91.1 adds itself as a downstream consumer of
P85's output.

---

## 5. Closed items — permanent records (why closed, what evidence closed them)

### 5.1 I5 — BitNet b1.58 / ternary weight quantization: CLOSED, no landing site

Closed because the technique's substrate does not exist in this codebase and is excluded by
standing policy, not merely absent by accident. BitNet b1.58 requires large *trained* neural
weight matrices consumed by matmul-heavy inference (weights ternarized to {-1,0,1} via
quantization-aware training — primary source arXiv 2402.17764, WebFetch-verified); the scan
(I5) confirmed by code read + policy citation that dowiz hosts **no trained NN weight matrix
anywhere**: the kernel is deterministic pure functions by explicit design
(`attention.rs:17` "no learned weights"; `retrieval/recall.rs` header keeps the ONNX semantic
model outside the kernel boundary), prior operator-directed syntheses rejected trained
rerankers/TimesFM (`GAUSSIAN-SPLATTING-…-2026-07-16.md`, `SYSTEMS-GPU-ML-KERNEL-…-2026-07-16.md`),
the one learning surface (`online.rs` scalar SGD / `micrograd.rs`) is the wrong shape (scalar
regression where the weight's *value* is the answer — ternarizing it forbids the correct
answer), and the one real trained artifact (bge-small ONNX) is external/build-time, not in-repo.
The nearest conceivable adjacent move (sign-bucketed pre-rank before exact PPR/spectral) is
plain deterministic bucketing — *not* BitNet — and was itself assessed net-negative at current
scale (n≤32 / few-hundred-note graphs; bit-exact-reproducibility contracts). **Only named
reopening trigger:** if the operator ever reverses the "semantic signal out of kernel scope"
stance and commissions a self-hosted in-repo embedding model, THAT new artifact would be the
one legitimate BitNet candidate — a new-component proposal, never a retrofit. Do not re-scan.

### 5.2 I6 — Closed-channel cheap-ordering substitution: CLOSED, design validated

Closed because the investigation found the codebase **already applies the principle correctly
everywhere it is valid**, and the one channel spending heavyweight crypto provably cannot use
the substitute. The premise (two mutually-trusted endpoints on a closed channel need only
monotonic-sequence ordering, not per-message asymmetric signing) requires all of: operator-owned
endpoints, no reachable adversary perimeter, and existing heavyweight crypto to remove. The scan
(I6) checked every seam: kernel↔engine is a same-process crate call (no channel — moot);
`event_log.rs` `set_tip` is single-writer/in-process and **already ordering-only** (per-actor
monotonic `actor_seq` + SHA3 content-addressed chain, zero signing — the principle, already
implemented); app↔pgrust is loopback `127.0.0.1` + RLS with zero signing (already cheap);
capability-cert/P06 K/V signing exists to bind authorship against a *self-certifying insider* —
non-repudiation, which a counter fundamentally cannot provide. The mesh node↔node channel — the
only place hybrid Ed25519⊕ML-DSA rides a real wire — fails the closed-channel premise **by the
project's own design and red-team record**: the design docs call the relay "semi-trusted"
(`MESH-REAL-PLAN.md:91`), and `bebop2/docs/red-team/2026-07-13/B2-protocol-authz.md` holds LIVE
demonstrated PoCs against exactly that path (expired-capability accepted at `now=0`; PQ-forgery
where a single Ed25519 sig passes). Dropping signing on a path with demonstrated forgery PoCs
would be a security regression, full stop. **Watch-item (not a change):** if a same-host
multi-process backend split over a Unix-domain socket ever appears AND someone proposes putting
capability signing on that loopback IPC, that would be the first genuine candidate — it does
not exist today. Do not stretch to manufacture a target; do not re-scan.

---

## 6. Consolidated open operator decisions from this closeout

| # | Decision | Unit | Default if unruled |
|---|---|---|---|
| W3-1 | GCRA lock-free swap on `token_bucket` (3.6× benched, security primitive) | P90 | NOT shipped; Mutex + clock-hoist stands |
| W3-2 | Push/merge `perf/contention-bench-2026-07-18` to remote/main | P90 | Branch stays local — against the push-after-milestone precedent; should be ruled promptly |
| W3-3 | Resolve bebop pre-existing C3 ungated-keygen red state (or explicit `--no-verify` ruling for the bus patch) | P85 / P90 | Bus patch stays a file; bebop branch stays commit-frozen |
| W3-4 | Push `a857cd71a` (slot_arena) from dowiz main-local | I2 | Stays local — same push-precedent concern |
| W3-5 | Execute P91.0 (comment-only false-claim removal in `kem.rs` header) ahead of the full P91 fix | P91 | Header keeps falsely claiming FIPS-203 until P91.1 — the trap stays armed |
| W3-6 | P85 closure path: real 3-model review vs recorded retroactive sign-off | P85 | Quarantine holds: no NTT wire-in, no Montgomery, no dependent work on `986646a` |

---

*Cross-references: `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` (S1 — tiers, P75–P83, §6
rejections E10/E12) · `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` (S2 — P85–P89, §0
R17 obligation, §7 override log) · `docs/research/OPUS-PERF-NTT-IMPLEMENTATION-2026-07-18.md` ·
`docs/research/OPUS-PERF-ARENA-DEEPDIVE-2026-07-18.md` §6 ·
`docs/research/OPUS-KEM-RING-BUG-INVESTIGATION-2026-07-18.md` ·
`docs/research/OPUS-TERNARY-BITNET-QUANTIZATION-SCAN-2026-07-18.md` ·
`docs/research/OPUS-TRUST-BOUNDARY-CLOSED-CHANNEL-SCAN-2026-07-18.md` ·
worktree `/root/dowiz-perf-contention` `docs/research/OPUS-PERF-CONTENTION-BENCH-RESULTS-2026-07-18.md`
+ `bebop-bus-G-C1-fix.patch` (branch-only) · `CORE-ROADMAP-STANDARD-2026-07-17.md` (20-point
contract for the P90/P91 writing pass) · memory: `crypto-safe-first-pass-2026-07-14.md`,
`worktree-remote-push-collision-avoidance-2026-07-18.md`,
`performance-priority-over-minimal-change-2026-07-17.md`.*
