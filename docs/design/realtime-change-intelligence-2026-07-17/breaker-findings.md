# RCI — Breaker Findings (System Breaker DeliveryOS)

> Target: `proposal.md` + `docs/adr/ADR-realtime-change-intelligence.md` (Option C, event-sourced
> graph projection). Read-only attack; no fixes proposed. Each finding: `[SEVERITY] vector ·
> break-scenario/number · violated invariant`. Grounded against live code + git 2026-07-17.

---

## HIGH

### H1 — [HIGH] B-CONSIST · concurrent multi-producer chain fork silently drops deltas
§4.1 wires **four concurrent producers** into one `GraphDelta` chain (git post-commit hook,
file-save watcher, tool-outcome stream, test/CI results). The reused substrate binds `prev`
to the current tip with **no compare-and-swap** (`event_log.rs:297-311`: read `tip()`, compute
`event_id()`, `set_tip()` — last-writer-wins). Two producers that both read tip `T`, both commit
with `prev=T`, both `set_tip` → the chain becomes a **tree, not a line**; the losing branch's tip
is overwritten and lost. `fold(0..head)` walks back via `prev` from the surviving tip and **never
sees the losing branch's deltas** → topology is silently incomplete.
The cited store documents its own ceiling verbatim: *"MemEventStore is non-durable and not shared
across processes — single-node local-first only"* (`event_log.rs:18-19`). The design points 4
concurrent, cross-process producers at a substrate whose invariant is single-writer.
**Violated invariant:** "Chronology is the single authority / the fold is a complete deterministic
projection of the chain" (§3 Option C, §6). A forked chain means the single authority is lossy.

### H2 — [HIGH] B-CONSIST · rollback is NOT idempotent or safe under concurrent writes; digest DoD only holds in a quiescent system
The brief's core question. §6: `revert(to)` diffs `fold(0..head)` vs `fold(0..target)`, appends **one**
compensator whose content-id is *"a function of `(head, target)`"*, and claims retry→`Duplicate`→
exactly-once, verified by `digest(fold(0..new_head)) == digest(fold(0..target))`.
- **Not idempotent under head-advance.** If any event lands between reading `head` and the retry
  (concurrent producer, or the first compensator itself already landed then the process crashed),
  the retry reads `head' ≠ head`, computes content-id `f(head', target) ≠ f(head, target)` → **not a
  `Duplicate`; a second, different compensator is appended.** The exactly-once claim holds only if
  `head` is pinned — which §4.1's four producers guarantee it is not.
- **Digest invariant is unsatisfiable under concurrency.** The compensator inverts the *edge diff*;
  applied after a concurrent delta `E` that added weight δ to edge `(a,b)`, the fold yields
  `weight_T(a,b) + δ`, so `digest(fold(0..new_head)) ≠ digest(fold(0..target))`. Because history is
  append-only/forward-only, `E` can never be removed → the target digest becomes **permanently
  unreachable**. The DoD can only ever pass on a quiescent replay.
- **The falsifiable test proves the wrong thing.** The RED+GREEN "mutate one edge ⇒ digest mismatch"
  verifies **fold determinism**, not concurrent-rollback safety. The proposal's assertion that
  "digest-equality of folds verifies [rollback is idempotent and safe under concurrent writes]" is
  false: the check is computed on a serial replay, never during concurrent load.
**Violated invariant:** exactly-once / idempotent-rollback (§6, ADR Decision §4). Credit: the digest
check IS genuinely falsifiable — but only for determinism, not for the property the brief asked about.

### H3 — [HIGH] B-SCALE / B-DATA · one real wide commit blows the co-change cap 20× and saturates the blast-radius predictor into uselessness
§2 pins co-change edges at **≤20k (design cap)**. A commit touching `F` files emits `C(F,2)`
co-change edges. Measured on THIS repo's actual history:
- widest commit = **896 files** → `C(896,2) = 400,960` edges — **20× the entire cap from ONE commit**;
- 5th-widest = 507 files → 128k edges (6× cap); several commits in the top-20 exceed 100 files.
These are the operator's normal commit style (native-Rust-port / format sweeps). The 896-file commit
is an **896-node clique** in co-change space: PPR seeded anywhere inside it spreads to ~all 896 nodes
→ blast radius = "everything breaks" → the primary feature (ranked consequence prediction,
`csr.rs:228-264`) degenerates to noise. A near-complete subgraph also inflates spectral radius/energy
(`spectral.rs:246,342`) → **spurious `Resonant`/`Unstable` drift**. The cap can only be honored by
pruning that silently changes graph semantics.
**Violated invariant:** the §2 back-of-envelope (≤20k edges) that underwrites the whole latency and
memory budget; and the utility premise that PPR yields a *ranked* (non-saturated) impact set.

### H4 — [HIGH] B-ANTIPATTERN / B-SEC · module import graph is structurally blind to red-line invariant-coupling — and the promotion-to-gating path can bless money/auth on that blindness
R1 is under-rated as a deferred nicety. The predictor's edges are import + co-change; **cross-cutting
invariants are not import edges**: "total must be integer cents", "row must be tenant-scoped by RLS",
"this path must run the auth check" couple files by *runtime contract*, not by `import`. Concrete:
a change inside `apps/api/src/routes/orders.ts` (flagged untested hotspot, CLAUDE.md) that breaks the
money-integer or tenant-scope invariant produces **low predicted blast radius** because the RLS
policy / `money.rs` rounding authority does not `import` orders.ts — the predictor reports "safe."
The design's **own DoD (c)** proves the blindness: it validates blast-radius against the E1
"unpinned Laplacian sign split between `engine/src/field_frame.rs:92` and `csr.rs:307`." Verified:
`field_frame.rs` laplacian is a standalone f32 stencil that **does not import `csr.rs`** (only
`engine/src/bridge.rs:16` imports `Csr`, a different symbol). So the coupling in the chosen benchmark
is a **sign convention with no import edge** — exactly the class the module graph cannot see. The
validation incident is one the predictor is structurally guaranteed to miss.
Escalation to HIGH: §7 + R6 let precision@k **earn gating power** (`RCI_BLOCKING`). Precision measured
on the common case (ordinary imports) can clear the 0.6 floor while systematically **excluding the
red-line failure mode it is blind to** (survivorship bias in the DoD) → a money/auth/RLS change with
invisible invariant-coupling passes the fail-closed drift gate (`event_log.rs:389-419`) with a false
low-blast-radius green.
**Violated invariant:** money/auth/RLS red-line ("verified, not asserted"); the PSR §4 "no unbacked
`will break` claim gates" rule, inverted into "a blindness-biased baseline earns the right to gate."

---

## MED

### M1 — [MED] B-SCALE · the ≤32-supernode quotient "structural invariant" is already violated by the target repo (40 > 32, today)
§2/§4/§7 declare the ≤32-supernode quotient a *structural invariant* (mirrors `spectral.rs:197`
`n<=32`), with supernode = "top-level dir/package," and §7 says n>32 is a **hard cap, refuse**.
`ls -d */` = **40 top-level dirs today** (59 if packages/*+apps/* granularity). So on the very repo
RCI targets, the quotient graph has n≈40 > 32 on day one. Two outcomes, both bad: (a) the §7
refuse-path fires → **no drift analysis produced at all**; or (b) `eigenvalues()` silently auto-takes
the `n>32` Faddeev-LeVerrier path (`spectral.rs:206-213`) — the one §2 calls "FORBIDDEN." The
alternative (squeeze 40 dirs into ≤32 buckets) merges distinct packages (e.g. `kernel`+`engine`),
destroying the per-package drift signal R5 claims to preserve.
**Violated invariant:** the design's own "≤32 is structural, not a tuning choice" (§2 line 108).

### M2 — [MED] B-SEC / B-DATA · `TestResult`/`meta` payload fields bypass the claim-check discipline → secret/PII leak vector
§8 claims "payloads carry SHA3 path-hashes + metadata, never contents … leaks nothing beyond repo
structure." But the `GraphDelta` TLV includes `kind: …|TestResult` and a free-form `meta` field
(§4.1). Test failure records are the classic secret/PII carrier: assertion diffs dump fixture data
(customer emails, sample orders), stack traces print env values, `run_fail` output can include tokens.
"metadata" and `TestResult` are **not hashes** — they are free-form text folded into an append-only,
potentially-synced chain that §8 says "leaks nothing if exfiltrated." The claim-check guarantee
covers file *contents* only; it has a hole exactly where error text lives.
**Violated invariant:** claim-check / "no PII, no secrets in the chain" (§8, B-SEC).

### M3 — [MED] B-CONSIST · tenant isolation is inherited-broken for the v2 the proposal itself contemplates
§8 answers the cross-tenant question "by construction — no cross-tenant data to read (v1 no DB)."
True for v1's scope. But RCI is built on the kernel event/graph model that the DELIVERY-FLOWS audit
grep-verified has **zero `tenant`/`location`/`org`/`hub_id` identifiers** (audit §5.1, row 16;
`MeshEvent` = `prev/actor_pubkey/actor_seq/payload`, no tenant field). The isolation the audit
recommends is **process-per-hub** (Option A), i.e. isolation-by-process, not by data field. §8/§5/R7
explicitly contemplate a v2 that "analyzes per-tenant runtime error streams" — but a cross-tenant
*change-intelligence* view is by definition **one process folding many hubs' deltas into one CSR**,
which breaks process-per-hub isolation, and the folded graph has **no tenant discriminant to
re-partition on**. R7's mitigation (RLS ENABLE+FORCE on future pgrust tables) protects rows *at rest*
— it does **not** partition an in-RAM CSR that has already merged tenants, nor does it add the missing
tenant dimension to `GraphDelta`. The design inherits precisely the gap the audit flagged.
**Violated invariant:** cross-tenant isolation (B-SEC); RLS-FORCE-on-every-table is a storage control,
not a substitute for a tenant-scoped projection.

### M4 — [MED] B-OPS · escalation-floor is unbootstrappable (R6 chicken-and-egg)
R6 "FIX in design": only findings above the measured precision floor emit `ESC-` records. But the
floor is scored by `recall@k`/`precision@k` against *later-arriving failures* (§4.1 analyzer 1). At
genesis there is **no failure history** → floor undefined. Either the system emits below-floor
(default) → a cold RCI on a 1.2k-node graph floods `escalations.jsonl` with unbacked predictions,
burying real LNC/PSR arbitration items; or it emits nothing until enough incidents accumulate → RCI
produces **zero escalations for months** and is inert. Both defeat R6's stated fix.
**Violated invariant:** B-OPS bounded/prioritized alerting; "no unbacked-prediction flood" (R6).

### M5 — [MED] B-SCALE · the per-delta "O(row) ≈ µs fold" latency line is false for the cited array-CSR; `energy()` on the full graph is a latent O(n⁴)/11.5 MB trap
§2 latency table: "CSR fold of one delta (edge merge) O(row) ≈ µs" citing `Csr::from_edges`. But
`from_edges` (`csr.rs:79-115`) **rebuilds the whole CSR** (per-row bucket + sort + merge over all
edges) — it is not an incremental single-edge insert, and the contiguous `col_idx/val` arrays cannot
insert one edge in O(row) (that needs an O(nnz) array shift). Real per-event cost is a full
O(E log E) rebuild ≈ 0.3 ms at E=20k (≈3 ms at 10× growth), not "µs." The total still lands inside
100 ms, so the budget survives — but the line item is mis-costed, and any analyzer that calls
`Csr::energy()` (`csr.rs:207-209`) on the **full** n=1.2k graph materializes a dense
1.2k×1.2k×8B ≈ **11.5 MB** `Vec<Vec<f64>>` and invokes `eigenvalues` on n=1.2k = the **forbidden
O(n⁴)** path. Nothing in the cited code enforces the §7 "hard cap, refuse"; it is a proposal-level
control absent from `eigenvalues()` (auto-selects n≤32 vs dense with no refusal).
**Violated invariant:** the §2 headroom derivation's per-item honesty; "O(n⁴) unreachable by
construction" (§7) — the construction is not in code.

---

## LOW

### L1 — [LOW] B-OPS · DoD (e) "zero .py under tools/loop-signals" is stated against an incomplete inventory
§9(e)/ADR require "zero `.py` files." §5/§4.1 name only `transcript_events.py` for porting. But
`find tools/loop-signals -name '*.py'` = **2 files**: `transcript_events.py` **and**
`test_transcript_e2e.py`. Meeting DoD (e) forces either leaving DoD unmet or **deleting a test** to
go green (test-integrity red-line) — with no Rust replacement for the e2e test named anywhere.
**Violated invariant:** DoD verifiability; "don't delete tests to pass" (test-integrity rules).

### L2 — [LOW] B-CONSIST · fold-determinism is conflated with chain-determinism (arrival-order race)
§6 "identical event prefix ⇒ identical CSR bytes" is real. But with §4.1's four async producers the
**prefix order itself is race-determined** (append interleaving of hook/watcher/CI). So "same repo
state ⇒ same analysis" is false: two runs over the same working tree can produce different chains →
different CSRs → different drift/blast verdicts. Replay of a *fixed* chain is reproducible (fine for
debug); reproducibility *across observations of the same repo* is not, and the proposal's determinism
framing does not distinguish them.
**Violated invariant:** the implied "one deterministic view of a given repo state."

### L3 — [LOW] B-SEC · path-hash "confidentiality" framing is hollow
§8: hashing paths means the chain "leaks nothing if exfiltrated beyond repo structure." SHA3(path)
over a **low-entropy, fully-enumerable** file tree is trivially invertible by dictionary (hash every
known path, match). The hashing adds no confidentiality over the paths themselves; it only avoids
carrying *contents*. The accepted leak (repo structure) is fine, but the claim that path-hashing
provides protection is empty. Combined with M2 (`meta`/`TestResult` free-form), the "leaks nothing"
assurance is over-stated.
**Violated invariant:** accurate B-SEC threat statement (not a data breach, but a false assurance).

---

## Regression note (round 1 — no prior revision)
No prior `breaker-findings.md` existed; this is the initial attack pass. Central-thesis breakers
(H1, H2) and the primary-feature breaker (H3) target Option C's load-bearing claims (single-authority
completeness, idempotent rollback, bounded PPR). The design's advisory/fail-open default correctly
caps the *blast radius of being wrong* — but H4 shows the promotion-to-gating path (RCI_BLOCKING)
routes that blindness onto red-line surfaces, which is where the severity concentrates.
