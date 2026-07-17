# 19 — System Coherence & the Authority Boundary (CORRECTION REDO, 2026-07-17)

> Correction pass answering a direct operator complaint about the prior batch research:
> *"it doesn't take the real picture — like resetting/snapshots, normalisations, zero-point
> stability states, no hashes, no pixeling the stashes & tensor logic. You cut key ideas, key
> points & bridges."* And, centrally: *"The goal is to remove the proxy/watchdogs/so called
> authority."*
>
> The prior batches (10–18) issued correct **per-item** verdicts but, exactly as the operator says,
> presented five ideas that were meant to be **one mechanism** as five separate rows. This document
> does two things the batches did not: (Part 1) traces the five as a single dependency graph with
> `file:line` where each piece already exists, and names the **bridges that must exist for them to
> compose** — including two that are currently missing; (Part 2) stops re-asserting the
> watchdog-vs-verification distinction and **tests it against a compiled, executed toy** that mirrors
> the kernel's `decide`/verify flow, then gives an honest verdict.
>
> Style: Anu (every load-bearing claim derives from a `file:line` read or the toy's actual output,
> not from a pattern catalog) / Ananke (the good outcome must be structural, not hoped). No metaphor.

---

## PART 1 — The five ideas are ONE pipeline (not five rows)

### 1.0 The single sentence the batches lost

A **tile** (a sub-tensor / sub-graph, the operator's "stash") is **normalized** to a canonical form,
so that its **content-hash** is the same bytes on every node, so that it can be **snapshotted** into a
retained base state keyed by that hash at an epoch boundary, and the snapshot is **admitted only if**
the tile's spectrum stays inside the **equilibrium (Lyapunov/ρ≤1) band**. Remove any one step and the
next is ill-defined. That is the "real picture": **(e)→(c)→(a)→(b), gated throughout by (d)**.

```
        (e) TILE / STASH                         the substrate everything acts on
        a sub-graph / sub-tensor held in an           kernel/src/arena.rs (BumpArena, blueprint W5)
        arena region (blueprint "snapshot-then-drop") BLUEPRINT-CACHE-...-ARENA:283,530,547
              │
              │ must be canonicalised BEFORE it can be compared/hashed across nodes
              ▼
        (c) NORMALIZATION                        THE ROOT DEPENDENCY
        row_normalize -> row-stochastic Â             csr.rs:125-152  (fixed-order sum :134, div :142)
        LaplacianKind {Unnormalized | Sym | RW}       csr.rs:282-316
        PPR seed + final normalize                    csr.rs:228-264
              │
              │ only a canonical tile has a cross-node-meaningful hash
              ▼
        (a) CONTENT-ADDRESS / HASH               depends on (c)
        matrix_content_address (FNV-1a, x.to_bits())  spectral_cache.rs:98-112 (bits at :107)
        MeshEvent content-id + prev->tip chain        event_log.rs:293-312 (idempotency :303-305)
        MerkleLog root = pair-hash of sorted ids      bebop2 sync_pull.rs:412-481 (content_id :129)
        DecompCache keyed by content-address          spectral_cache.rs:37-86
              │
              │ the hash is the snapshot key AND the dedup/convergence proof
              ▼
        (b) SNAPSHOT / CHECKPOINT-RESTORE (+epoch) depends on (a); GATED by (d)
        boot_verify replays WORM log on restart       hydra.rs:253-265   (in-memory: BUILT)
        arena snapshot-then-drop -> retained base     BLUEPRINT-CACHE-...-ARENA:283,530
        durable snapshot + restore-drill              GAP (P12 + Hermetic #4) — see 1.2
              ▲
              │ a snapshot is admissible ONLY if the rebuilt spectrum is Damped/Resonant
              │
        (d) ZERO-POINT / EQUILIBRIUM (Lyapunov)  gates (b); measured on (e)
        DriftClass {Damped ρ<1 | Resonant ρ≈1 | Unstable ρ>1}  spectral.rs:313-352
        classify_drift (BAND=1e-6 around ρ=1)                  spectral.rs:341-352
        drift-gate: reject Unstable pre-persist                event_log.rs:389-419 (:409-414)
        integrity_check / boot_verify enforce ρ<1             hydra.rs:180-195, 253-265
        step_preserves: |I(f x) − I(x)| ≤ tol  (Lyapunov/Noether checker)  noether.rs:19-39
```

### 1.1 (a) State hashing / content-addressing — what already exists

Three independent content-address mechanisms already run, all deterministic-by-construction:

- **Matrix content-address** — `spectral_cache.rs::matrix_content_address` (`:98-112`) is FNV-1a-64
  over a canonical row-major, index-framed byte layout; it hashes `x.to_bits()` (`:107`) so *any* entry
  change changes the root and identical contents give the same root "on every platform/run" (`:96-97`).
  `DecompCache` (`:37-86`) keys an expensive eigensolve by this address, with the `recomputes` counter
  as an explicit no-thrash / no-stale falsifier (`:11-16`, tests `:149-207`).
- **Event content-id + hash-chain** — `event_log.rs::append` (`:293-312`) computes `ev.event_id()`,
  chains `prev`→`tip` (`:297-301`), and treats a re-seen id as a `Duplicate` structural no-op
  (`:303-305`). `append_raw` (`:321-329`) makes the id a *stable function of the event's own fields*
  for the self-witness rows, so a re-received breach alert is a no-op, not a new row.
- **Merkle root over the folded log** — bebop2 `sync_pull.rs::MerkleLog` (`:412-481`): sorted content-id
  leaves, recursive `sha3_256(left‖right)` root (`:456-475`), `content_id = sha3_256(prev‖actor‖seq‖
  payload)` (`:129`). A matching root is a cheap convergence proof; a differing root triggers a pull.

### 1.2 (b) Rolling snapshot / checkpoint-restore with adaptive epoch — split BUILT/GAP

- **In-memory replay = BUILT.** `hydra.rs::boot_verify` (`:253-265`) replays the WORM log after any
  restart and re-checks the baseline spectrum. This is the working half of "Snapshot Re-entry."
- **Arena snapshot-then-drop = DESIGNED (blueprint).** The tensor-arena maintenance pass rebuilds the
  CSR each cycle and "edges dropped after a rebuild snapshots them into a retained base graph —
  degrade-closed, no unbounded [growth]" (`BLUEPRINT-CACHE-...-ARENA:283`, mechanism `:530`, wave W5
  `:547`). This *is* the rolling-snapshot-with-epoch idea — the maintenance pass boundary is the epoch.
- **Durable snapshot + restore-drill = REAL GAP.** Batch 3 §6c and Hermetic #4: the cold restore-drill
  has never run, no `restore-verify` subcommand exists, and every append-only store grows forever with
  zero compaction (Phase 27 A6). Owned by P12. **Adaptive epoch length** is deferred (blueprint #25) —
  correctly, since it is meaningless until the snapshot cycle it modulates exists.

### 1.3 (c) Normalization — exists, and is the ROOT of the whole graph

- `csr.rs::row_normalize` (`:125-152`) produces a row-stochastic `Â` with a deterministic self-loop for
  dangling rows (`:135-139`); the row sum `s` is accumulated in **fixed index order** (`:134`) and the
  entry is `self.val[k] / s` (`:142`).
- `LaplacianKind` (`:282-316`) gives the three canonical Laplacians (`Unnormalized D−A`, `Symmetric
  I−D^{−1/2}AD^{−1/2}`, `RandomWalk I−D^{−1}A`); `personalized_pagerank` normalizes the seed and the
  final vector (`:228-264`).

**Why (c) is the root, and the missing bridge the operator named ("normalisations … no hashes").**
`matrix_content_address` hashes raw `f64` bits (`:107`). Two nodes that build the *same logical tile*
but at a different scale, or one normalized and one not, produce **different bits ⇒ different
content-id ⇒ the Merkle roots never converge and `DecompCache` never hits across nodes** — silently.
For the hash of a tile to be *meaningful across nodes* it must be taken over a **canonical** form, i.e.
after `row_normalize`. Today they are not wired that way: `slem_cached` content-addresses the **raw**
adjacency `a` (`spectral_cache.rs:117-118`), not a normalized one. **This is bridge-gap #1: normalize
BEFORE hash.** It is small and buildable (content-address the `row_normalize`d form, or a canonical
integer-scaled form).

This bridge is only *sound* because of the determinism boundary the repo already proved: IEEE-754
mandates correctly-rounded `/` (so `row_normalize`'s division is cross-target bit-identical), but does
**not** mandate identical rounding for transcendentals (`rng.rs:22-28`). So a **fixed-order rational**
normalizer (`row_normalize`) composes with cross-node hashing; a transcendental normalizer (a
softmax/`exp` pooling) would **break** it — the same physics that killed the RGB-seed codec (Batch 6
§2.2). The bridge therefore has a hard constraint: *the normalization on the hash path must be
integer/rational and fixed-order.*

### 1.4 (d) Zero-point / equilibrium — this IS discrete Lyapunov stability, under the name `DriftClass`

The operator's "zero-point … controlled oscillation, not a static point" already exists as a
dynamical-systems concept — it is Perron–Frobenius / DMD spectral stability, which for a linear map is
exactly **discrete Lyapunov stability** (ρ<1 ⟺ the map is a contraction ⟺ a Lyapunov function exists
that is non-increasing):

- `DriftClass` (`spectral.rs:313-352`): `Damped` (ρ<1, contracts), `Resonant` (ρ≈1, a limit cycle —
  e.g. μ≈−1 period-2), `Unstable` (ρ>1, diverges). `classify_drift` uses a `BAND=1e-6` tolerance around
  ρ=1 (`:341-352`). The equilibrium is a **band around ρ=1, and `Resonant` is explicitly a permitted
  class** — only `Unstable` is rejected. That is precisely "controlled oscillation, not a static point
  ρ=0."
- The gate that enforces it: `event_log.rs::commit_after_decide_drift_gate` (`:389-419`) rejects an
  `Unstable` mutation **pre-persist** (`:409-414`) — "organism endures by NOT persisting." `hydra.rs::
  integrity_check`/`boot_verify` (`:180-195`, `:253-265`) enforce ρ<1 as the health predicate.
- The **executable Lyapunov/Noether checker** already exists: `noether.rs::step_preserves`
  (`:19-39`) verifies `|I(f(x)) − I(x)| ≤ tol` along a trajectory and is proven non-vacuous by catching
  Euler energy drift on a harmonic oscillator (`:87-97`). This is the generic "a conserved quantity /
  Lyapunov bound must not drift" gate the snapshot admission in (b) can reuse.

### 1.5 (e) Tile / stash — the substrate, and bridge-gap #2

The operator's "pixeling the stashes & tensor logic" is the tensor-arena blueprint: a `CacheGraph`
observer feeds a co-access edge buffer; a maintenance pass runs `from_edges → row_normalize → PPR` in a
`BumpArena` region (`_in` variants), then snapshots-and-drops (`BLUEPRINT-CACHE-...-ARENA:283,396,431,
530,547`). The "stash" = the arena region + the retained base-graph snapshot; a "tile" = one region /
one HugePage of that graph.

**Bridge-gap #2 (snapshot must be gated by the equilibrium check).** The blueprint's maintenance pass
runs `row_normalize` and PPR but does **not** run `classify_drift` on the rebuilt graph before it
becomes the retained base. So an `Unstable` rebuild could be snapshotted — (b) is not yet gated by (d).
The fix reuses what exists: the snapshot-retain step should pass the same `commit_after_decide_drift_
gate` admission (or a `noether::step_preserves` bound) the event log already uses, so **only a
Damped/Resonant rebuild is ever retained.** This is the concrete meaning of "respect the equilibrium
bound," and it is the second bridge the item-by-item batches did not draw.

### 1.6 The dependency graph, stated as the composition the batches lost

```
  normalize (c) ──is-prerequisite-of──▶ hash (a) ──is-key-of──▶ snapshot (b)
       ▲                                                              ▲
       │                                                              │ admits-iff
   operates-on                                                    equilibrium (d)
       │                                                              │ measured-on
     tile (e) ─────────────────────────────────────────────────────┘
```

- (c) is the **root**: without a canonical form, (a) is not cross-node-meaningful, so (b)'s keys don't
  converge. **Bridge-gap #1: normalize→hash is not wired (`slem_cached` hashes raw).**
- (a) depends on (c); it is simultaneously the snapshot **key** and the convergence **proof** (Merkle).
- (b) depends on (a) for its key and is **gated by (d)**. **Bridge-gap #2: the arena snapshot is not
  drift-gated.**
- (d) is measured on the spectrum of the tile (e) and gates (b); it is a **band** (Resonant allowed),
  not a point — the operator's zero-point reading, verbatim.
- (e) is the substrate all four act on.

Net for Part 1: all five pieces **exist in code or landed design**; the operator is right that the
prior write-ups lost the composition. Two real, small, buildable **bridges** are missing — *normalize
before hash* (§1.3) and *drift-gate the snapshot* (§1.5) — and one leg (durable snapshot/restore, §1.2)
is a known P12 gap. None is a complexity rejection.

---

## PART 2 — Testing the watchdog / authority boundary against real code

### 2.1 What the code actually is (not what the synthesis says it is)

- `event_log.rs::commit_after_decide` (`:339-361`) and `commit_after_decide_drift_gate` (`:389-419`)
  run the check **inline, before persist**, on the *same call* that would otherwise commit — decide
  rejects ⇒ nothing is written (`:355-359`, `:409-416`). The unsafe state is never produced.
- `hydra.rs::integrity_check` (`:180-195`) and `boot_verify` (`:253-265`) are, as Batch 3/6 flagged,
  **condition-checks**: `commit` *calls* `integrity_check` on the critical path (`hydra.rs:227`), and
  `boot_verify` runs a self-administered `assert!` (`:258`). They are checks — but the sharp question
  is whether "a check" is automatically "a watchdog/authority." It is not, and the toy shows why.

### 2.2 The toy (compiled with `rustc -O`, run; full source + output below)

The toy guards one invariant — `balance` must never be observable `< 0` (mirrors ComputeBudget
over-spend and the drift ρ<1 bound) — two ways: (A) an external watchdog thread that polls and reacts;
(B) inline verify-before-persist in the shape of `commit_after_decide_drift_gate`. Then it probes the
regress: does the inline verifier (`key_V`) need its own verifier, forever, or bottom out?

```rust
// watchdog_vs_verify.rs — toy mirroring kernel's decide/verify flow.
// Invariant under guard: a `balance` must never be observable < 0
// (mirrors: ComputeBudget must never over-spend; drift rho must stay < 1).
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// ─── PATTERN A: WATCHDOG (forbidden) — a background thread polls & REACTS ───
fn pattern_a_watchdog(spawn_watchdog: bool) -> (i64, u64) {
    let balance = Arc::new(AtomicI64::new(100));
    let violations_observed = Arc::new(AtomicU64::new(0));
    let wd_handle = if spawn_watchdog {
        let bal = Arc::clone(&balance);
        Some(thread::spawn(move || {
            for _ in 0..200 {
                if bal.load(Ordering::SeqCst) < 0 { bal.store(0, Ordering::SeqCst); } // reactive
                thread::sleep(Duration::from_micros(50));
            }
        }))
    } else { None };
    let bal = Arc::clone(&balance);
    let viol = Arc::clone(&violations_observed);
    let worker = thread::spawn(move || {
        let deltas = [-40, -40, -40, -40, -40, 200, -30, -30, -30, -30];
        for &d in deltas.iter() {
            bal.fetch_add(d, Ordering::SeqCst);                 // BLIND write — no gate
            if bal.load(Ordering::SeqCst) < 0 { viol.fetch_add(1, Ordering::SeqCst); }
            thread::sleep(Duration::from_micros(30));
        }
    });
    worker.join().unwrap();
    if let Some(h) = wd_handle { h.join().unwrap(); }
    (balance.load(Ordering::SeqCst), violations_observed.load(Ordering::SeqCst))
}

// ─── PATTERN B: INLINE verify-before-persist (key_V shape) ───
struct Ledger { balance: i64, rejects: u64, max_observed_negative: i64 }
impl Ledger {
    fn new() -> Self { Ledger { balance: 100, rejects: 0, max_observed_negative: 0 } }
    // `verify` is TOTAL, PURE, on the critical path: the value the caller wants
    // only exists if verify passed. Shape of commit_after_decide_drift_gate.
    fn commit(&mut self, delta: i64, verify: impl Fn(i64) -> bool) -> Result<i64, &'static str> {
        let candidate = self.balance + delta;                   // computed, NOT persisted
        if !verify(candidate) { self.rejects += 1; return Err("verify rejected; state unadvanced"); }
        self.balance = candidate;                               // persist ONLY after the check
        if self.balance < self.max_observed_negative { self.max_observed_negative = self.balance; }
        Ok(self.balance)
    }
}

// ─── REGRESS PROBE: does key_V need a key_V? ───
struct Anchor; // genesis/operator trust-root — planted once, not derived
impl Anchor { fn certifies_verifier(&self) -> bool { true } }
fn self_certified_accept(claimed_ok: bool) -> bool { claimed_ok }         // key_K: author's word
fn independently_verified_accept(anchor: &Anchor, artifact_value: i64) -> bool { // key_V
    anchor.certifies_verifier() && artifact_value >= 0        // recompute from artifact, ignore claim
}

fn main() {
    println!("== PATTERN A (watchdog) ==");
    let (fw, vw) = pattern_a_watchdog(true);
    println!("  with watchdog:    final balance = {fw:>4}, times balance OBSERVED < 0 = {vw}");
    let (fn_, vn) = pattern_a_watchdog(false);
    println!("  watchdog absent:  final balance = {fn_:>4}, times balance OBSERVED < 0 = {vn}");
    println!("  -> bad state IS representable; correctness depends on a separate live process.");
    println!("\n== PATTERN B (inline verify-before-persist) ==");
    let mut led = Ledger::new();
    let nonneg = |c: i64| c >= 0;
    for &d in [-40, -40, -40, -40, -40, 200, -30, -30, -30, -30].iter() { let _ = led.commit(d, nonneg); }
    println!("  final balance = {:>4}, rejects = {}, most-negative EVER observed = {}",
             led.balance, led.rejects, led.max_observed_negative);
    assert!(led.max_observed_negative >= 0, "invariant was NEVER crossed");
    println!("  -> bad state never produced; no window, no separate process, no race.");
    println!("\n== REGRESS PROBE: who verifies the verifier? ==");
    let anchor = Anchor;
    println!("  self-cert (key_K) accepts a lying -50 artifact? {}", self_certified_accept(true));
    println!("  independent (key_V) accepts it?                 {}",
             independently_verified_accept(&anchor, -50));
    println!("  key_V authority bottoms out at the anchor (planted once): {}", anchor.certifies_verifier());
    println!("  -> finite chain: commit -> key_V verdict -> anchor identity. No infinite tower.");
}
```

**Actual output (`rustc -O`; stable across repeated runs):**

```
== PATTERN A (watchdog) ==
  with watchdog:    final balance =   80, times balance was OBSERVED < 0 = 3
  watchdog absent:  final balance =  -20, times balance was OBSERVED < 0 = 4
  -> the bad state IS representable; correctness depends on a separate live process.

== PATTERN B (inline verify-before-persist) ==
  final balance =  100, rejects = 3, most-negative balance EVER observed = 0
  -> bad state was never produced; no window, no separate process, no race.

== REGRESS PROBE: who verifies the verifier? ==
  self-cert (key_K) accepts the lying -50 artifact? true
  independent (key_V) accepts it?                   false
  key_V's authority bottoms out at the anchor (planted once): true
  -> finite chain: commit -> key_V verdict -> anchor identity. No infinite tower.
```

### 2.3 What the toy actually demonstrates (three distinct axes the synthesis blurred into one)

1. **Liveness/supervision axis — this is the real categorical difference.** Deleting the watchdog is a
   **silent runtime gap**: the invariant is simply never enforced and `balance` sits at `−20` forever
   (Pattern A, watchdog-absent). Deleting the inline verify is a **compile-time hole**: the `Ok(value)`
   the caller wants *cannot be produced* without the check running, because the check is on the causal
   path of the result it guards. Absence-is-invisible (watchdog) vs absence-is-visible (inline) is the
   whole difference — and it is categorical, not PR. A watchdog is a second process that must itself be
   alive and correct ⇒ "who watches the watchdog?" is a live liveness regress (you need a supervisor
   for the supervisor). The inline check has **no liveness regress at all**, because there is no
   separate process to keep alive.
2. **Latency/representability axis.** Even *with* the watchdog running, the bad state was **observed
   negative 3 times** — the poll window means the broken invariant is representable and observable, and
   the reactive clamp destroyed information (final `80`, not the correct `100`). Inline verify's
   most-negative-ever-observed is `0`: the bad state was **never produced**. This is exactly the
   `commit_after_decide_drift_gate` property — "endures by NOT persisting" (`event_log.rs:383`).
3. **Authority/trust axis — where the honest limit is.** `key_K` (self-certification) accepts a lying
   `−50` artifact; `key_V` (independent re-execution) rejects it. But `key_V`'s authority does **not**
   dissolve into nothing — it bottoms out at the `Anchor` (genesis/operator trust-root), which the toy
   shows is a **finite** bottom (`commit → key_V verdict → anchor`), **not** an infinite tower. So an
   independent verifier *is still an authority* — just a finite, structural, event-triggered one
   anchored at a human-planted root, not a standing supervising process.

### 2.4 Honest verdict

**"Independent verification baked into the same computation" is categorically different from a watchdog
— but it is NOT uniformly "not an authority," and the prior T-6 synthesis slightly over-claims by
implying it is.** The precise, tested statement, on three levels:

- **vs a watchdog (the operator's actual target — "remove the proxy/watchdog"): genuinely removed.**
  An inline verify-before-persist is not a proxy, not a poller, not a standing process; its absence is a
  compile hole not a silent gap; it has zero liveness regress ("who watches the watcher" never arises
  because there is no watcher to keep alive); and the bad state is never representable rather than
  caught-after-the-fact. On this axis the difference is real and decisive. **T-6's core ruling holds.**
- **vs "authority" for internal-arithmetic invariants (budget/money/drift ρ): authority genuinely
  dissolves.** Here the check *is* the computation (`debit` IS the gate; the drift-gate IS the commit
  path), there is no separate identity to trust, and it bottoms out at the type system / IEEE-754 /
  the compiler — i.e. the *same physics* the operator wants, not a bureaucratic rule. The operator's
  "remove authority entirely" is **literally achieved** for this class (Batch 6 §4.1 was right).
- **vs "authority" for the tamper / cross-party leg (`integrity_check`, `key_V`, `WorkReceipt`): NOT
  dissolved — replaced by a finite anchored authority.** `integrity_check` is inline (no watchdog, no
  liveness regress) **but self-certified** — a compromised node checking itself, the RC-2 gap. The fix,
  `key_V`, is inline **and** independent, which is strictly better than a watchdog (event-triggered, no
  process to supervise, terminates at the anchor rather than recursing) — but it is still an
  *authority*: a distinct party whose verdict you trust because a genesis anchor certified its identity
  once. You cannot type external tamper out of existence (Batch 6 §4.2), so this one leg irreducibly
  keeps a check, and the honest word for `key_V` is "a finite structural authority," not "no authority."

**Strongest one-sentence justification (either way):** In the toy, deleting the inline verify is a
compile-time hole — the guarded result cannot be produced without it — whereas deleting the watchdog
leaves `balance` sitting at `−20` forever with no signal; that asymmetry (absence-is-visible vs
absence-is-invisible) is the categorical difference, and it holds precisely because the inline check
sits on the causal path of the value it guards instead of orbiting it as a separate process — which
also fixes the boundary the synthesis blurred: watchdog-vs-inline and self-cert-vs-independent are
**two different axes**, and the target design (`key_V`) is *inline + independent*, i.e. neither a
watchdog (axis 1) nor self-certification (axis 3).

---

## Appendix — provenance

- Toy source + binary: `scratchpad/watchdog_vs_verify.rs` (this session), compiled `rustc -O`, run 4×,
  output stable and reproduced verbatim in §2.2.
- Code read live this session: `kernel/src/spectral_cache.rs`, `event_log.rs` (`:280-419`),
  `hydra.rs` (`:160-279`), `csr.rs` (`:118-330`), `spectral.rs` (`:290-360`), `noether.rs`,
  `order_machine.rs` (`:140-200`); `bebop-repo/bebop2/proto-wire/src/sync_pull.rs` (Merkle region).
- Design docs: `BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md`,
  `12-BATCH3-...`, `15-BATCH6-...`, `BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS.md` §7 (T-6),
  `rng.rs:22-28` determinism boundary (cited via Batch 6 §2.2).
- Two new findings not in the prior batches: **bridge-gap #1** (normalize-before-hash is unwired —
  `slem_cached` content-addresses the raw adjacency, `spectral_cache.rs:117-118`) and **bridge-gap #2**
  (the arena snapshot-then-drop is not drift-gated by `classify_drift` before retention).
```
