# Principle 6 — Cause and Effect (Determinism as Architectural Law)

> "Every Cause has its Effect; every Effect has its Cause; everything happens
> according to Law; chance is but a name for Law not recognized; there are many
> planes of causation, but nothing escapes the Law." — Kybalion

One of 7 parallel Hermetic-principle passes over dowiz/DeliveryOS + openbebop.
Self-contained, code-grounded, no mysticism. All file:line refs are against the
working tree on branch `feat/kernel-fsm-graph-analysis` (2026-07-16).

---

## 1. The concrete architecture-principle statement

In this codebase, **Cause and Effect grounds to determinism as a hard
architectural law**:

> **No kernel computation may produce an effect (an output byte, an idempotency
> key, a content-address, a signature, a gate verdict) that is not a pure,
> reproducible function of its declared cause (its input arguments plus fixed
> code). Every source of apparent "chance" — randomness, wall-clock time,
> hash-map iteration order, floating-point summation order, external state —
> must be either (a) eliminated, (b) named, seeded, and isolated behind a port,
> or (c) proven not to leak into any effect a test or a peer relies on. There is
> no true chance inside the kernel; there is only Law not yet recognized in the
> code.**

This is not an imported metaphor. It is the project's own written discipline:

- `ARCHITECTURE.md` §3 lists **event-sourcing**, **content-addressing (sha3)**,
  and **eqc VERIFIED-BY-MATH** as core patterns — all three are cause→effect
  reproducibility mechanisms.
- The VERIFIED-BY-MATH rule (MEMORY.md) is literally *"works? provable?
  falsifiable? Ship RED"* — a demand that every claimed effect be reproducible
  enough to fail a test if the cause changes.
- `event_log.rs` builds the whole mesh on the axiom that **the same content is
  the same event**: `event_id = sha3_256(prev ‖ actor_pubkey ‖ actor_seq ‖
  payload)` (`event_log.rs:148-155`), and a replayed identical cause is a
  *structural* `AppendOutcome::Duplicate` no-op (`event_log.rs:221-228,
  267-269`), not a timeout-based dedup. "A duplicate is a structural no-op, not
  a timeout" (module doc, `event_log.rs:7`) is exactly *"chance is but a name
  for Law not recognized"*: idempotency is made a law of the content function,
  not a probabilistic guess.

The principle has **planes of causation** in the Hermetic sense — a hierarchy of
effects with different blast radius:

| Plane | Effect | If non-deterministic → |
|-------|--------|------------------------|
| P0 | Idempotency keys / content-addresses (`event_id`, block ids) | mesh replay breaks; duplicates re-execute |
| P0 | Signatures / snapshot roots (`snapshot_root`, `witness_event_id`) | peers reject valid state; forgery detection breaks |
| P1 | Gate verdicts (`classify_drift`, `verify_fsm_signature`) | drift gate flaps; false RED/GREEN |
| P2 | Ranked query results (BM25, PPR, recall) | retrieval order jitters |
| P3 | Display/transport JSON (wasm boundary) | golden tests + downstream hashing break |

The audit's job is to find where an effect escapes the Law, and to rate it by
which plane it sits on.

---

## 2. Audit methodology

I read the four grounding files in full (`ARCHITECTURE.md`, `event_log.rs`,
`rng.rs`, `order_machine.rs`) and then the load-bearing numeric/retrieval
modules (`spectral.rs`, `retrieval/ppr.rs`, `retrieval/diffusion.rs`,
`retrieval/bm25.rs`, `retrieval/index.rs`, `retrieval/recall.rs`,
`retrieval/memory_store.rs`, `analytics.rs`, `wasm.rs`, `hydra.rs`, `evals.rs`,
`backup.rs`, `geo.rs`). I specifically grepped for the four canonical
Cause-and-Effect violation classes:

1. **Wall-clock inside deterministic code** —
   `SystemTime|Instant::now|::now()|elapsed()|UNIX_EPOCH|std::time` across
   `kernel/src`, `engine/src`, and `/root/bebop-repo/bebop2/core`.
2. **True OS RNG in computation** — `thread_rng|rand::|OsRng|getrandom|
   from_entropy` in source (excluding `target/` build artifacts).
3. **HashMap/HashSet iteration leaking into an effect** — every `HashMap`/
   `HashSet` site, then traced whether its iteration order flows into a
   `format!`/`serde_json::to_string`/`sha3`/sort-key/test-assertion, vs. being
   used only for membership (`contains`) or an order-independent reduction
   (`sum`, `count`).
4. **Unfixed float summation / transcendental order** — whether PPR, the
   eigensolver, and BM25 fix their iteration/summation order (as their docs
   claim) or merely assert it.

For each hit I distinguished a **real** violation (order leaks into a
reproducible effect) from a **quarantined** use (telemetry / test-only / display
that no signature depends on), per the principle's own carve-out: wall-clock in
a logger is Law-abiding; wall-clock in a hash is not.

---

## 3. Findings

### Compliance is the dominant result (state it first, honestly)

The kernel's **P0/P1 effects — the ones the mesh and the gates actually depend
on — are genuinely deterministic**, and not by accident:

- **`event_id`** is built by explicit byte concatenation in fixed field order
  (`event_log.rs:149-154`), never by iterating a map. Its determinism test
  (`event_id_is_deterministic_and_content_keyed`, `:433`) is real.
- **`snapshot_root`** (the one place a store hashes *all* its contents into a
  single root — a true signature) is computed over a **`BTreeMap`**, iterated in
  sorted key order, with the explicit note *"the fold is canonical regardless of
  insertion order"* (`memory_store.rs:5-6, 46, 85-94`). This is the textbook
  correct fix and it is applied exactly where it matters most.
- **`backup.rs`** content-addresses every block by `sha3_256`; its
  `HashMap<Hash,Vec<u8>>` is used only for membership and a length **sum**
  (`backup.rs:58, 133-145`) — order-independent. `restore_is_byte_identical`
  (`:218`) holds.
- **`rng.rs`** is a **named, seeded, dependency-free** SplitMix64→PCG64
  generator (`Rng::new(seed, stream)`, `rng.rs:34`) pinned to published
  reference vectors (`splitmix_reference_stream :143`, `pcg_reference_stream
  :164`). This is precisely the principle's requirement: apparent randomness
  traced to a named, seeded, predictable source. `evals.rs` similarly uses a
  seeded `Lcg` (mulberry32-family, `evals.rs:22-54`).
- **`order_machine::spectral_radius`** fixes its power-iteration count
  (`ITERS=1000`), summation order (`for i in 0..n`), and Rayleigh reduction over
  a `Vec` (`order_machine.rs:323-360`) — no map, no RNG. `markov.rs` (fixed
  `POWER_ITERS=300`, `:27`) and `retrieval/ppr.rs` do the same: **dense
  `Vec<Vec<f64>>`, fixed i-outer/j-inner order** (`ppr.rs:42-67`), with a
  determinism proof in the module doc, not just an assertion.
- **BM25, trigram index, analytics, recall fusion** all sort or reduce over a
  canonical key set before emitting: BM25 sorts query terms and dedups before
  summation (`bm25.rs:197-201`) and sorts hits by (score desc, id asc)
  (`:233-239`); the trigram `candidates()` counts into a map but **sorts the
  result** (`index.rs:144-149`); `analytics::funnel` emits in fixed
  `OrderStatus` declaration order (`analytics.rs:107-125`), and
  `orders_by_channel` sorts (`:93-101`); the recall fusion drives ordering from
  a **`BTreeSet`** and a sorted `Vec` (`recall.rs:151, 177-184`). Each carries a
  comment asserting HashMap-order independence — and here the comments are
  **true**, because a sort/BTree sits between the map and the effect.

So the naive violation classes mostly come back clean. That is the honest
headline. The interesting findings are the residue.

### Finding A — HashMap iteration leaks into the wasm `funnel` JSON (real, MEDIUM)

`wasm.rs:104-112` defines the exported ledger output:

```rust
#[derive(Serialize)]
struct LedgerOut {
    orders_by_channel: Vec<(String, u64)>,          // sorted → deterministic
    funnel: HashMap<String, Vec<(String, u64)>>,    // ← iteration-order effect
    anomalies: u64,
}
```

and `channel_ledger_logic` builds `funnel` as a `HashMap` (`wasm.rs:239-247`)
then serializes the whole struct with `serde_json::to_string(&out)`
(`wasm.rs:254`). `serde_json` serializes a `std::collections::HashMap` **in the
map's iteration order**, and `std` `HashMap` uses a per-instance
`RandomState` seed — so the **order of the top-level `funnel` object's keys is
not a function of the input**. Cargo confirms no mitigation: `serde_json` is
pulled without the `preserve_order` feature (`kernel/Cargo.toml:47`; and
`preserve_order` would not help a user `HashMap` anyway — it only affects
`serde_json::Value`).

Concretely: two invocations of the wasm `channel_ledger` entry point on the
**same input** can emit JSON strings that differ byte-for-byte (only the key
ordering of `funnel`), i.e. the *effect* (serialized bytes) is not a pure
function of the *cause* (the event list). The per-channel `stages` vectors
inside are fine (fixed enum order); it is only the outer channel→stages map that
jitters.

Severity **MEDIUM, currently latent**: this JSON is a *display/transport* effect
(plane P3). Nothing in the kernel hashes, content-addresses, or signs it today,
so no stated byte-identical guarantee is broken *right now*. But it is a genuine
violation of the principle, and it is a **trap**: the moment anyone golden-file
tests this output, diffs two snapshots, or content-addresses it for a cache key,
it breaks non-reproducibly and intermittently — the hardest class of bug.
**Fix is one line of type discipline:** make `funnel` a `BTreeMap<String,
Vec<(String,u64)>>` (or emit `Vec<(String, Vec<...>)>` built from the already
sorted `orders_by_channel`). This mirrors the `memory_store.rs` fix exactly.

### Finding B — "byte-identical across runs" is only ever tested same-process (real, LOW-MEDIUM)

Every byte-identical / reproducibility test in the kernel invokes the function
**twice inside one process** and compares:

- `ppr.rs:104` `green_ppr_byte_identical_across_runs` — two `ppr.rank(...)` calls.
- `diffusion.rs:191` `green_ppr_byte_identical_two_runs` — two `ppr.rank(...)` calls.
- `csr.rs:495` `ppr_byte_identical` — two `personalized_pagerank(...)` calls.
- `recall.rs:335` `fusion_ranking_is_deterministic` — two `fusion_rank(...)` calls.
- `bm25.rs:380` `ranking_is_deterministic_and_tie_broken_by_id` — rebuild+rerank.

None spawns a second process, serializes to disk and re-reads, or runs on a
second target. For the specific paths tested this is **adequate** — those paths
are HashMap-free dense-`Vec` numerics, so a same-process second call *is* the
whole space of variation. But the guarantee the discipline *states* is broader
than what the tests *prove*: `rng.rs:4-6` claims the generators "reproduce
bit-identically across runs, **platforms, and builds**." A same-process
`assert_eq!` cannot falsify a cross-platform or cross-build divergence. The gap
is safe today only because someone happened to keep these paths map-free and
integer-or-dense-float; the test would not *catch* a regression that reintroduced
a map (e.g. building `W` from a `HashMap` adjacency), because a fresh `HashMap`
per call could still — by luck — hash two small key sets into the same order in
one process. Severity **LOW-MEDIUM**: methodological, not a live break, but it
means the "byte-identical" phrase is a stronger claim than the harness verifies.

### Finding C — float **transcendental** determinism is claimed platform-wide but only the integer RNG actually satisfies it (real, LOW-MEDIUM)

The reproducibility claim in `rng.rs:4-6` ("bit-identical across runs,
platforms, and builds") is **true for `rng.rs` itself** — it is pure integer
arithmetic (`wrapping_add/mul`, xor, shifts, `rotate_right`), and IEEE-754 plus
two's-complement guarantee those are bit-identical everywhere. But the same
VERIFIED-BY-MATH banner sits over float paths that use **libm transcendentals**,
which IEEE-754 does **not** require to be correctly-rounded or identical across
platforms/libm versions:

- `spectral.rs`: `Complex::abs` → `.hypot()` (`:53`), `arg` → `.atan2()`
  (`:57`), `Complex::sqrt` → `.sqrt()` twice (`:84-93`); the Durand-Kerner and
  Householder eigensolvers run on these.
- `bm25.rs:207`: IDF uses `.ln()`.
- `geo.rs`: haversine/bearing use `.sin() .cos() .asin() .atan2() .sqrt()`
  (`geo.rs:19-20, 33-35, 80`), exposed on the wasm ETA/distance surface.

`+ − × ÷ √` are correctly-rounded by IEEE-754 (so PPR's dense products are
safe), but `ln/sin/cos/atan2/hypot/asin` are library functions whose last-ULP
results legitimately differ between glibc, musl, wasm's libm, and Apple libm.
The kernel's *own* signatures never hash a transcendental result (they hash
bytes and integers), so **P0 is not exposed**. But any test or peer that
compared a *spectral* or *geo* float bit-for-bit across two platforms could
diverge, and the discipline's blanket "bit-identical across platforms" does not
distinguish the safe integer plane from the unsafe transcendental plane.
Severity **LOW-MEDIUM**: real technicality, no current break, worth an explicit
carve-out in the doctrine ("transcendental floats: reproducible per-target, not
cross-target").

### Non-violations a naive audit would flag (rigor check)

- **Wall-clock**: the *only* `Instant::now()/elapsed()` in `kernel/src` is inside
  a `#[test]` micro-benchmark whose result is `eprintln!`'d, never asserted
  (`spectral.rs:615-628`). `engine/src` has **zero** wall-clock. bebop's one hit
  is `speedometer.rs:17` — telemetry by name. All correctly quarantined; none is
  a violation.
- **OS RNG**: no `thread_rng`/`OsRng` in any `kernel/`/`engine/` **source**. The
  `rand`/`getrandom`/`ring` hits are all `kernel/target/` build artifacts pulled
  transitively by `ring` (crypto), not called from kernel code.
- **`hydra.rs` `FileEventStore.by_id: HashMap`**: used only for
  `contains_key`/`get`/`insert`; `tip`/`count` are stored fields, and the
  `.iter()` at `:836` is over `ev.payload` (a `Vec<u8>`), not the map. The
  witness digest is `sha3` over explicit bytes (`:135`). Deterministic.
- **`analytics::reduce_anomalies`** iterates a `HashMap` (`:150`) but only
  **counts** anomalies (order-independent). Safe.

---

## 4. Verdict — do the kernel's "byte-identical" claims hold?

**On the plane that matters (P0/P1), yes — and provably.** Every effect the mesh
and the gates depend on is a pure function of explicit, ordered bytes: `event_id`
concatenates fields in fixed order; `snapshot_root` folds a **`BTreeMap`** in
sorted order and *says so*; block backups are `sha3` content-addresses; the
drift gate and FSM golden signature (`order_machine.rs:472-541`) are computed
from integer/dense-float graph invariants with fixed iteration counts; the RNG
is named, seeded, and pinned to reference vectors. The project did the hard part
correctly: it kept HashMap iteration *out* of every real signature, and it fixed
summation order in PPR/markov/BM25 rather than merely asserting it. The
Cause-and-Effect Law holds where breaking it would be catastrophic.

**But the claim is stated one notch broader than it is proven**, and the residue
is three genuine cracks: (A) the wasm `funnel` JSON leaks `HashMap` order into an
emitted effect — harmless as display, lethal the day it is hashed; (B) every
"byte-identical" test is same-process, so "across platforms/builds" is asserted,
not exercised; (C) the reproducibility banner covers transcendental-float paths
(`ln/sin/atan2/hypot`) whose cross-platform bit-identity IEEE-754 does not
grant, even though only the safe integer RNG fully earns the banner.

In Hermetic terms: the kernel has recognized the Law on its inner planes — no
effect there is left to chance. On its outer plane (the JSON boundary) and in the
*wording* of its guarantee, a little unrecognized chance remains — named here so
it can be brought under the Law with a `BTreeMap`, a second-process test, and one
sentence of doctrine.
