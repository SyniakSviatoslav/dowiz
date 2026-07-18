# BLUEPRINT P-B — State/Consistency + Living Memory (Wave 2, Fable, 2026-07-17)

> **Phase P-B of the CORE-ROADMAP** (`docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §3, row P-B).
> Written against the §2 twenty-point contract — every numbered contract item is satisfied in a
> named section below (§ map at the end). Absorbs, does not re-derive:
> `bebop2-mesh-tensor-hermetic-2026-07-17/11-BATCH2-state-consistency-findings.md` (doc 11),
> `19-SYSTEM-COHERENCE-AND-AUTHORITY-BOUNDARY-REDO.md` (doc 19),
> `BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md` (§A/§E: W1-L2, W1-L10, W1-L11).
>
> Scope: the three Wave-1 correctness-closure items of the tile→normalize→hash→snapshot chain —
> (1) the `commit_after_decide` exactly-once port, (2) the normalize-before-hash fix as a
> type-system invariant, (3) the drift-gated snapshot admission — plus the living-memory framing
> of the retained base. Everything else in the state cluster (epoch gossip, MMR, Merkle
> bisection, reorder buffer, rolling truncation) stays where doc 11's build-order put it
> (steps 3–8, unchanged by this blueprint).

---

## 1. Ground truth (contract item 1 — every cite verified THIS pass, 2026-07-17)

**Branch fact (matters for sequencing):** the live checkout is now on `feat/p19-growth-engine`
(HEAD `f01f9bb6b`), which carries forward `feat/harness-llm-backend`'s tree. All statements below
are against this HEAD, re-read this session.

| # | Claim | Cite (live read) |
|---|---|---|
| G1 | **Exactly-once bug is STILL LIVE.** `commit_after_decide` deduplicates on the raw `ev.event_id()` (`kernel/src/event_log.rs:348-352`) but persists via `self.append(ev)` (`event_log.rs:359`), and `append` rebinds a zero `prev` to the current tip **before** computing the stored id (`event_log.rs:297-302`). Dedup key ≠ storage key on any non-empty log ⇒ a replay re-runs `decide` and double-commits. | `kernel/src/event_log.rs:293-312, 339-361` |
| G2 | The regression test for G1 is **ABSENT** on this branch (`grep replay_on_nonempty` → no hit); the existing `dup_event_is_idempotent_no_state_change` (`event_log.rs:533`) commits onto an **empty** log (tip `None` ⇒ no rebind), so it cannot catch G1. | `kernel/src/event_log.rs:533` + grep this session |
| G3 | The **fix already exists, tested, on the sibling branch** `feat/agentic-mesh-protocol-2026-07-17` (commit `f30189262`, worktree `/root/dowiz-agentic-mesh`): `commit_after_decide` persists via `self.append_raw(ev)` (`/root/dowiz-agentic-mesh/kernel/src/event_log.rs:380`), with the P07 §2 doc-comment (`:340-356`) and regression test `commit_after_decide_replay_on_nonempty_log_is_true_duplicate` (`:602`). `git branch --contains f30189262` → only the agentic-mesh branch. **NOT merged here.** | diff of the two files this session |
| G4 | **Normalize-before-hash bug is STILL LIVE.** `slem_cached` content-addresses the **raw** matrix: `matrix_content_address(a)` at `kernel/src/spectral_cache.rs:114`, which hashes raw `x.to_bits()` (`spectral_cache.rs:103`) with inline FNV-1a-64 constants (`:95-96`). No normalization anywhere on the hash path. | `kernel/src/spectral_cache.rs:94-130` |
| G5 | The canonicalizer exists and is hash-compatible: `Csr::row_normalize` (`kernel/src/csr.rs:125-152`) — fixed-index-order row sum (`:134`), correctly-rounded IEEE-754 division (`:142`), deterministic self-loop for dangling rows (`:135-138`). Rational + fixed-order, no transcendentals. | `kernel/src/csr.rs:118-152` |
| G6 | The determinism boundary that makes G5 sound: integer/rational fixed-order ops are cross-target bit-identical; transcendental libm paths are **per-target only**. | `kernel/src/rng.rs:18-29` (doc comment) |
| G7 | `slem_cached` has exactly **one production caller**: `kernel/src/markov.rs:209` (transition matrix `a`, already row-stochastic by construction). Refactor blast radius = 1 call site + module tests. | grep this session |
| G8 | **No arena/snapshot module exists yet.** `kernel/src/arena.rs` does not exist; zero `arena|BumpArena` hits in `kernel/src`. The arena is design-only: `BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md` W5 (`kernel/src/arena.rs` planned), snapshot-then-drop at that blueprint's §3.1.2 ("dropped after a rebuild snapshots them into a retained base graph — degrade-closed"). | `ls`/grep this session + arena blueprint §7 |
| G9 | The drift machinery to gate with exists: `DriftClass` (`kernel/src/spectral.rs:316`), `classify_drift` with `BAND = 1e-6` around ρ=1 (`spectral.rs:342-347`; Damped/Resonant admitted, Unstable rejected); `noether::step_preserves` (`kernel/src/noether.rs:19`); the commit-path precedent `commit_after_decide_drift_gate` (`kernel/src/event_log.rs:389-419`, reject pre-persist at `:409-414`). | live reads this session |
| G10 | `Csr` constructors available: `from_edges` (`csr.rs:79`). **No `from_dense`, no `to_dense`** — both must be added (small, §4). `DecompCache` keys on `&str` roots (`spectral_cache.rs:62`), with `recomputes` falsifier counter (`:11-16`). | live reads this session |
| G11 | Regression ledger exists and is the ratchet authority: `docs/regressions/REGRESSION-LEDGER.md` (rows through #20; "every future fix adds a guardrail with a red→green proof and a ledger row BEFORE it is done"). | live read this session |

**Answer to the sequencing question this blueprint was asked:** the exactly-once fix is
**NOT landed** on the current branch — it is a **port** (G1–G3), not new design, and it is
step 1 of everything below.

---

## 2. Fix 1 — exactly-once `commit_after_decide` (port of the mesh-branch fix)

### 2.1 The bug, restated once (doc 11 §12, re-verified G1)

`commit_after_decide` checks `store.contains(raw_id)` where `raw_id = ev.event_id()` computed
over the caller's bytes (`prev = [0;32]` for local-first callers), then persists through
`append`, which rebinds `prev → tip` and stores under the **chain-bound** id. Original commit
onto a non-empty log ⇒ stored id ≠ raw id ⇒ a replay of the same logical event misses dedup,
**re-runs `decide` (side effect twice), appends a second row**. Money-law violation class: a
replayed `SettlementClaimed` re-runs its hashlock side effect (doc 11 §12). This is the RC-3
fail-open shape (Hermetic RC-3: a fail-open hole under the cause-and-effect substrate) living
in the one primitive every gossip/patch concept in the cluster assumes.

### 2.2 Port plan (agent-executable; do NOT cherry-pick)

Commit `f30189262` also carries B3 `TokenBucket::release` and B1 `AgentBridge` — out of P-B
scope. Port the `event_log.rs` hunk only, **test first**:

1. **RED:** copy test `commit_after_decide_replay_on_nonempty_log_is_true_duplicate` from
   `/root/dowiz-agentic-mesh/kernel/src/event_log.rs:602` (whole `#[test]` fn) into
   `kernel/src/event_log.rs`'s test module. Run `cargo test -p kernel
   commit_after_decide_replay` — it MUST FAIL on current code (decide runs twice / len grows).
   If it passes unfixed, the port is wrong — stop.
2. **GREEN:** apply the two-part fix from the mesh file (`:366-381`): (a) keep dedup on the raw
   id, (b) change `event_log.rs:359` from `self.append(ev)` to `self.append_raw(ev)`. Port the
   P07 §2 doc-comment block (`:340-356`) verbatim — it is the spec of the semantic change.
3. **Semantic change, stated honestly (do not hide it):** decide-gated commits become
   idempotent-by-content and **chain-independent** — a zero-`prev` event is stored with
   `prev = [0;32]` (no tip rebind; `append_raw` takes `prev` verbatim, `event_log.rs:314-329`).
   Callers wanting explicit local-first chaining call `append` directly (the ported doc-comment
   says exactly this). Before landing, run `grep -rn "commit_after_decide" kernel/ engine/` and
   confirm no caller depends on the rebind (this session: only in-module tests + the
   drift-gate delegate at `event_log.rs:418`).
4. Full suite: `cargo test -p kernel` — all existing event_log tests (`:533-741`, both poles:
   Law-reject vs Store-fault) must stay green unmodified. Any test rewritten to pass = the
   fix is wrong (test-integrity rule).

**DoD (falsifiable):** the replay test exists on this branch and is green; asserted event
sequence is `[Committed(id), Duplicate(id)]` with the SAME id both times; `decide` invocation
count == 1 across the pair; `log.len()` unchanged by the replay. Regression-ledger row §11.

---

## 3. Fix 2 — normalize-before-hash, encoded in the type system

### 3.1 What "normalize" means for a tile — the canonical form (from doc 19 §1.3, made exact)

A tile (sub-graph / sub-tensor; the operator's "stash" unit) is in **canonical form** iff:

- **N1 — Row-stochastic:** every row sums to 1, produced by `Csr::row_normalize`
  (`csr.rs:125-152`): fixed-index-order sum, correctly-rounded IEEE-754 division, deterministic
  self-loop `Â[i][i] = 1` for a dangling (all-zero) row. This is doc 19's "(c) NORMALIZATION —
  THE ROOT DEPENDENCY": only a canonical tile has a cross-node-meaningful hash.
- **N2 — Structurally canonical:** within each row, entries in strictly ascending column order;
  explicit zero entries dropped (a stored `0.0` and an absent entry are the SAME tile).
- **N3 — Rational/fixed-order only:** no transcendental function anywhere on the
  canonicalize-or-hash path (no softmax/`exp` pooling). Grounds: IEEE-754 mandates
  correctly-rounded `+ − × /` (cross-target bit-identical) but NOT transcendentals
  (`rng.rs:18-29`) — the same physics that killed the RGB-seed codec (V2 §A).

**Convergence guarantee tiers (honest, not overstated):**
- identical raw bytes → identical address: unconditional;
- raw tiles differing by an exact **power-of-two** scale factor → identical address:
  unconditional (`c·v` exact for pow-2 `c`, so the scaled row-sum is the scaled sum exactly,
  and correctly-rounded `(c·v)/(c·s)` equals correctly-rounded `v/s`);
- arbitrary rational scale → last-ulp divergence is possible (`c·v` rounds before division).
  Named upgrade path with a falsifiable trigger: an **integer-scaled rational canonical form**
  (hash `(col, num/gcd)` integers per row) — build it if and only if a cross-node address
  mismatch on logically-identical tiles at non-pow-2 scale is ever observed in telemetry.

### 3.2 The type-level invariant (contract items 4, 6, 14 — hazard-safety as math, not runtime check)

The bug class "hashed a tile that was not canonicalized" becomes a **compile error**, by three
structural facts: (i) `NormalizedTile` has a private field and its only constructors run the
canonicalizer; (ii) `TileAddress` has no public constructor and its only producer is a method
on `NormalizedTile`; (iii) the raw-matrix hash `matrix_content_address` is demoted to private.
There is no runtime check to forget and no reviewer vigilance to depend on — doc 19 Part 2
axis 1 verbatim: deleting the check is a **compile-time hole**, absence-is-visible; this is
the zero-authority class (arithmetic invariant; the check IS the construction).

**Exact signatures (predefined types & constants — nothing stringly-typed left open):**

```rust
// ── kernel/src/csr.rs (extended — canonicalization is Csr's domain) ─────────

impl Csr {
    /// Dense round-trips (small helpers, needed by NormalizedTile + admit()).
    /// from_dense DROPS explicit zeros (N2) and emits ascending-column rows.
    pub fn from_dense(a: &[Vec<f64>]) -> Csr;
    pub fn to_dense(&self) -> Vec<Vec<f64>>;
}

/// A tile in CANONICAL form (N1+N2+N3, §3.1).
/// INVARIANT (type-encoded, not runtime-checked): the field is private and the
/// only constructors run `row_normalize` — a NormalizedTile that skipped
/// canonicalization is UNREPRESENTABLE. No `&mut` accessor exists.
pub struct NormalizedTile {
    csr: Csr, // private
}

impl NormalizedTile {
    /// The only ways in. Both canonicalize (N1: row_normalize; N2: via from_dense
    /// / a sort-dedup pass on the Csr path; N3: by construction — rational ops only).
    pub fn canonicalize(raw: &Csr) -> NormalizedTile;
    pub fn from_dense(a: &[Vec<f64>]) -> NormalizedTile;

    /// THE ONLY PRODUCER of a TileAddress in the crate.
    /// FNV-1a-64 over the canonical CSR bytes, length-framed, fixed order:
    /// nrows, then per row: frame(i), then (col_idx[k] as u64, val[k].to_bits())
    /// ascending k. O(nnz), not O(n²).
    pub fn content_address(&self) -> TileAddress;

    /// Read-only views (eigensolve input; snapshot retention). No mutation path.
    pub fn as_csr(&self) -> &Csr;
    pub fn to_dense(&self) -> Vec<Vec<f64>>;
}

/// Opaque content-address of a CANONICAL tile. No public constructor —
/// possession of a TileAddress is proof normalization ran (Curry-Howard cheap).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TileAddress(u64);

impl TileAddress {
    /// Hex form for DecompCache's &str key path (reuse, not a second authority).
    pub fn as_hex(&self) -> String; // format!("{:016x}", self.0)
}

/// FNV-1a-64 single authority (today duplicated as inline literals in
/// spectral_cache.rs:95-96 and the memory_store snapshot_root — consolidate).
pub const FNV_OFFSET_64: u64 = 0xcbf2_9ce4_8422_2325;
pub const FNV_PRIME_64: u64 = 0x0000_0100_0000_01b3;

// ── kernel/src/spectral_cache.rs (changed) ──────────────────────────────────

/// CHANGED SIGNATURE (the fix): the cache key AND the eigensolve input both
/// derive from the SAME NormalizedTile — key/payload coherence by construction.
pub fn slem_cached(cache: &mut DecompCache, tile: &NormalizedTile) -> f64;

/// DEMOTED to private: `fn matrix_content_address(...)` (was pub at :94).
/// The raw-matrix hash ceases to be a public entry point; the compiler is the
/// gate. (DecompCache itself stays &str-keyed and general — the invariant
/// enforced is "tile hashing goes through normalization", nothing broader.)
```

**Caller migration (G7 — exactly one):** `kernel/src/markov.rs:209` becomes

```rust
let tile = crate::csr::NormalizedTile::from_dense(&a);
let slem = crate::spectral_cache::slem_cached(&mut decomp_cache, &tile);
```

`a` is already row-stochastic there, so canonicalization is value-idempotent up to ulp
(row sums are 1.0±ulp from the counts division; re-dividing perturbs entries ≤ a few ulp ⇒
slem shifts ≤ ulp-scale). Markov's downstream thresholds (`gap > 1e-12`, `markov.rs:212`)
are tolerance-guarded; DoD requires the whole markov test module green **unmodified**.

### 3.3 The key/payload-coherence catch (new this pass — a bug the naive fix would create)

The naive reading of W1-L10 ("just hash the normalized form") is **wrong on its own**: if the
address were computed on the canonical form but `eigenvalues` still ran on the raw matrix, two
raw tiles sharing a canonical form would share a cache key while carrying **different raw
spectra** — whichever node computed first would poison the shared key (a cross-node stale-serve,
worse than the bug being fixed). The signature above forecloses it structurally: `slem_cached`
receives only the `NormalizedTile`, so key and payload cannot come from different objects.
For row-stochastic canonical tiles ρ = 1 exactly and SLEM is the operative quantity — which is
precisely what `slem` measures; no semantic loss.

---

## 4. Fix 3 — drift-gated snapshot admission (`RetainedBase`)

### 4.1 Status honesty

`kernel/src/arena.rs` does **not exist** (G8) — bridge-gap #2 (doc 19 §1.5, V2 W1-L11) is a gap
in a *designed* pass, not in running code. Per contract item 3 (spec precedes test precedes
code), P-B lands the **admission types and gate now**, so that when the arena blueprint's W5
builds the maintenance pass, an ungated retain path is *unbuildable* — the only constructor of a
retained snapshot is the gate. This is Ananke-by-construction, not a review reminder for W5.

### 4.2 The gate must run on the RAW dynamics, not the canonical form (anti-vacuity finding, new this pass)

A row-stochastic tile has ρ = 1 **always** (Perron root of a stochastic matrix). Running
`classify_drift` on the `NormalizedTile` would classify every snapshot `Resonant` and never
reject anything — a vacuous gate that *looks* wired (RC-2 shape: a verification organ without
teeth). Doc 19's graph already says this correctly — "(d) … measured on (e) [the tile
substrate]", not on (c)'s output. Therefore: **drift admission runs on the raw rebuilt operator
`W` (the dynamics a rebuild would induce); the hash runs on the canonical form.** Two forms,
two roles, one pipeline — exactly `(e)→(c)→(a)→(b) gated by (d)`.

### 4.3 Exact signatures

```rust
// ── kernel/src/spectral_cache.rs (new section; reuse-first home: it already
//    owns the content-address + cache layer the snapshot keys into) ──────────

/// A drift-admitted, canonicalized, content-addressed retained snapshot —
/// node (b) of the doc-19 pipeline, with (d) as its ONLY door.
/// INVARIANT (type-encoded): the only constructor is `admit`, which runs
/// `classify_drift` on the RAW operator BEFORE canonicalization+addressing.
/// A retained Unstable base is UNREPRESENTABLE.
pub struct RetainedBase {
    tile: NormalizedTile,   // private
    address: TileAddress,   // private
    epoch: u64,             // private — logical, max-merge, NO wall-clock (doc 11 §7/§8)
}

/// Law-pole rejection (mirror of CommitError::Rejected — never retry; nothing retained).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotRejected {
    /// ρ > 1 + BAND on the raw rebuilt operator: retaining it would snapshot a
    /// divergent dynamics. "Organism endures by NOT persisting"
    /// (same law as event_log.rs:409-414).
    UnstableSpectrum,
}

impl RetainedBase {
    /// Verify-before-persist, INLINE on the causal path (doc 19 Part 2, axis 1):
    /// the RetainedBase the caller wants cannot exist unless the gate ran.
    /// Order inside: classify_drift(raw.to_dense()) → reject Unstable →
    /// NormalizedTile::canonicalize(raw) → content_address() → construct.
    pub fn admit(raw: &Csr, epoch: u64) -> Result<RetainedBase, SnapshotRejected>;

    pub fn tile(&self) -> &NormalizedTile;
    pub fn address(&self) -> TileAddress;
    pub fn epoch(&self) -> u64;
}
```

No `intervention` bypass parameter: the SOURCE-OF-HYDRA lift (`event_log.rs:400-416`) is a
commit-path directive about foreign mutations; a *snapshot* is retention of our own rebuild —
there is no operator directive lifting it, so none is offered (offering one would be a
fail-open door nobody asked for).

**Scaling note on `admit` cost:** `classify_drift` takes dense (`spectral.rs:342`), so `admit`
densifies — O(n²) memory at the maintenance-pass boundary (off the request path; the arena
blueprint §3.1.2 puts all expensive work there). Axis and change-point stated in §7.

---

## 5. Spec-driven, event-driven TDD plan + adversarial tests (contract items 3, 5)

Order is spec (types above) → RED test → code. Tests assert **sequences of outcomes/events**,
not only end-state. All tests live in the modules they pin; names are final.

| Test (permanent) | Asserts | RED before / GREEN after |
|---|---|---|
| `commit_after_decide_replay_on_nonempty_log_is_true_duplicate` (ported, event_log.rs) | outcome sequence `[Committed(id), Duplicate(id)]`, same id; decide-count == 1; len unchanged | RED on `self.append(ev)`, GREEN on `self.append_raw(ev)` (§2.2) |
| `adversarial_two_node_hash_divergence_without_normalization` (spectral_cache.rs) | **reproduces the bug as its RED half, permanently**: builds tile `A` and `B = 2^k·A` (same logical tile, two "nodes"/scales); asserts (i) the raw FNV path (reconstructed inline in the test — the old `matrix_content_address` body) yields **different** hashes (the divergence two nodes WOULD get), AND (ii) `NormalizedTile::from_dense(A).content_address() == …(B).content_address()` | (i) proves the hazard is real forever; (ii) is the fix; if (i) ever fails, the raw path stopped diverging and the whole design premise must be re-derived |
| `canonical_address_and_spectrum_derive_from_same_object` (spectral_cache.rs) | two raw tiles with the same canonical form get the same address AND `slem_cached` serves the identical slem for both (cache hit, `recomputes` == 0 across the pair — reusing the DecompCache falsifier counters, spectral_cache.rs:11-16) | guards the §3.3 key/payload-poisoning class |
| `unstable_raw_rebuild_is_refused_retention` (spectral_cache.rs) | `RetainedBase::admit(raw ρ=2.0, e)` → `Err(SnapshotRejected::UnstableSpectrum)`; `admit(raw ρ=0.5, e)` → `Ok` with address == canonical tile's address and epoch == e | RED trivially before `admit` exists; GREEN after |
| `drift_gate_measures_raw_dynamics_not_normalized_form` (spectral_cache.rs) | **anti-vacuity chaos test (the intentionally-breaking one):** a raw operator with ρ = 2 whose row-normalized form necessarily has ρ = 1 (Resonant) must STILL be refused. If a future refactor moves the gate after canonicalization, this test goes red | kills the vacuous-gate variant (§4.2) permanently |
| `canonicalize_drops_explicit_zeros_and_sorts_columns` (csr.rs) | `from_dense` with explicit `0.0` entries and permuted insertion order reaches the same `TileAddress` as the clean build (N2) | pins structural canonicality |

Compile-time cases (not runnable tests — the point is they don't compile): constructing
`NormalizedTile { csr }` outside `csr.rs`, constructing `TileAddress(…)` outside `csr.rs`,
constructing `RetainedBase { … }` outside `spectral_cache.rs`, calling the demoted
`matrix_content_address` from another module. Cheap CI backstop (smart index, item 14): a grep
gate asserting `pub fn matrix_content_address` does not exist in `kernel/src/` (ledger §11).

> **WAVE F enforcement added:** the grep backstop is now an *active* CI job
> `no-pub-raw-matrix-hash` in `.github/workflows/ci.yml` (BLUEPRINT-P-B §5 note-4).
> `matrix_content_address` is demoted to `fn` (private) in `kernel/src/spectral_cache.rs`,
> so the gate passes today; if it is ever re-promoted to `pub`, the job goes RED and blocks
> merge into the canonical repo. The drift-gate `RetainedBase::admit` is the only internal
> caller and reaches it via the private path.

---

## 6. DoD (contract item 2 — machine-checkable, RED→GREEN)

1. All six §5 tests present and green; the two RED halves demonstrably failed first (§2.2 step 1
   for the port; `adversarial_two_node_hash_divergence…` clause (i) is a permanent in-test RED).
2. `cargo test -p kernel` fully green with **zero existing tests modified** (event_log both-poles
   suite `event_log.rs:533-741`, markov module, DecompCache falsifiers `spectral_cache.rs:146-203`).
3. `grep -rn "pub fn matrix_content_address" kernel/src/` → empty.
4. `grep -rn "slem_cached" kernel/src/` → only the `&NormalizedTile` signature and its callers.
5. Bench recorded per §8 (numbers in `BENCH_HISTORY.md`, not estimates).
6. Three ledger rows appended per §11.

---

## 7. Schemas & scaling axes (contract item 8)

| Shape | Axis | Holds until / change point (falsifiable) |
|---|---|---|
| `TileAddress(u64)` FNV-1a-64 | # distinct canonical tiles alive | birthday bound ≈ 2³² tiles before collision odds are material; and it is a **convergence fingerprint, NOT tamper-evidence** — the moment an address is used as a *security* boundary (adversarial cross-node proof), re-derive over SHA3 per doc 11 §3's FNV/SHA3 split. Both limits stated now so neither is rediscovered |
| `NormalizedTile` (CSR) | nnz per tile | O(nnz) hash + canonicalize; fine to ~10⁶ nnz per maintenance pass; beyond that the arena `_in` variants (arena blueprint W5) own the allocation story |
| `RetainedBase::admit` densify | tile n | O(n²) dense for `classify_drift`; acceptable at maintenance-boundary n ≤ ~10³. Change point: a measured maintenance pass where densify+eigensolve exceeds the pass budget ⇒ swap to a sparse power-iteration ρ-estimate over `Csr::spmv` (fixed-order, csr.rs:154-177 — machinery already deterministic). Do not build it before the number exists |
| `epoch: u64` | events/epochs | logical max-merge counter (doc 11 §8); no wall-clock ever (doc 11 §7 REJECT of HLC physical half); u64 does not roll over in any physical deployment |

---

## 8. Benchmarks + telemetry (contract item 10)

- Bench (criterion, recorded in `BENCH_HISTORY.md` per the repo's bench-track discipline):
  `canonicalize+content_address` vs the old raw-FNV address at n ∈ {64, 256, 1024} (dense-source
  path, markov-shaped). Expected ~2× address-path cost (one extra O(nnz) pass); the *measured*
  number is the deliverable, and the DecompCache hit path is unchanged (address computed per
  call before and after).
- `RetainedBase::admit` cost at n ∈ {64, 256, 1024} (densify + classify_drift), so §7's change
  point has a baseline.
- Telemetry hooks already exist and are reused, not duplicated: `DecompCache::recomputes`
  (no-thrash/no-stale falsifier, spectral_cache.rs:11-16) now also witnesses cross-scale
  convergence (a scaled rebuild of the same logical tile must be a HIT — recomputes stays 0);
  `SnapshotRejected` occurrences are the drift-gate refusal signal (count them where the arena
  maintenance pass lands, W5).

---

## 9. Isolation, mesh, rollback-as-math (contract items 11, 12, 13)

- **Bulkhead (11):** the whole cache/snapshot layer is advisory over an idempotent upstream —
  on any address mismatch or cache absence the fallback is recompute (the existing
  `get_or_recompute` MISS path); nothing here can block or corrupt the commit path. The one
  shared mutable resource (`DecompCache`) is `&mut`-only, no interior locking
  (spectral_cache.rs:36-41) — contention is structurally impossible, not managed.
- **Mesh (12):** everything in P-B is **node-local computation**. What the fix buys the mesh:
  a `TileAddress` is 8 bytes and cross-node-convergent, so tile-level cache warm-sharing and
  Merkle-side comparison become *possible* over the existing transport
  (`SyncFrame`/`discovery.rs`, ≤1 MiB budget — V2 §A propagation layer); **no new carrier, no
  new payload introduced by this phase.**
- **Rollback/self-healing, named per the idea-185 taxonomy (13):**
  *Snapshot Re-entry* = `RetainedBase` at the last admitted epoch — cheap regenerative recovery
  from the last valid canonical state (the working in-memory half is `hydra::boot_verify`'s
  replay, doc 19 §1.2). *Self-Termination* = the admission boundary — an Unstable retained base
  is an **unrepresentable state** (type-level), not a supervisor's decision. Neither word is
  used loosely: the first is a constructor call, the second is a missing constructor.

---

## 10. Living memory — time / topology / data-flow (contract item 15)

Cross-reference: memory `internal-retrieval-living-memory-arc-2026-07-14` (living-memory =
tiered, demote-never-delete, recall by personalized PageRank).

- **Time:** `epoch` on `RetainedBase` is the temporal tier key. Snapshot-then-drop (arena
  blueprint §3.1.2) is exactly the living-memory demotion move: old observations are **demoted
  into the retained base, never deleted** — and the event log underneath stays append-only
  (truncation remains council-gated, doc 11 §10), so no history is lost by compaction.
- **Topology:** the tile IS the topology — a co-access/derivation sub-graph; recall over it is
  `personalized_pagerank` (csr.rs:228+), which requires the row-stochastic canonical form —
  the same `NormalizedTile` this blueprint mints. One canonical object serves hash, spectrum,
  and recall: storage is not flat here, by construction.
- **Data-flow:** `TileAddress` is the recall key; `DecompCache` is the hot tier;
  `RetainedBase` is the warm tier; re-derivation from the event log is the cold floor.
  Demote-never-delete holds at every step.

---

## 11. Regression tracking (contract item 17) — ledger rows to append

Append to `docs/regressions/REGRESSION-LEDGER.md` (next free rows; red→green proof per rule):

| # | Symptom | Guardrail |
|---|---|---|
| 21 | Replayed decide-gated event re-runs `decide` and double-commits on a non-empty log (dedup key ≠ storage key) | `unit`: `commit_after_decide_replay_on_nonempty_log_is_true_duplicate` (kernel/src/event_log.rs) |
| 22 | Two nodes hash the same logical tile to different content-ids (raw-bits hashing before canonicalization) → Merkle roots never converge, caches never hit cross-node, silently | `unit` + type-system: `adversarial_two_node_hash_divergence_without_normalization`; `NormalizedTile`/`TileAddress` privacy; `CI-gate` grep: no `pub fn matrix_content_address` |
| 23 | An Unstable (ρ>1) rebuild retained as a base snapshot; or the gate made vacuous by measuring the normalized (always-ρ=1) form | `unit`: `unstable_raw_rebuild_is_refused_retention` + `drift_gate_measures_raw_dynamics_not_normalized_form`; type-system: `RetainedBase` sole-constructor gate |

---

## 12. Agent-executable instructions (contract item 18 — zero-session-context)

Preconditions: repo `/root/dowiz`, branch carrying `kernel/` (verify `git branch --show-current`;
as of writing `feat/p19-growth-engine`). Sibling worktree `/root/dowiz-agentic-mesh` must exist
(source of the §2 port). All commands from repo root. Do not modify any existing test.

1. **Port exactly-once** — follow §2.2 steps 1–4 literally (test first, RED verified, then the
   one-call fix + doc-comment). Files: `kernel/src/event_log.rs` only.
2. **Csr helpers** — add `from_dense`/`to_dense` to `kernel/src/csr.rs` (§3.2 signatures);
   `from_dense` drops zeros, emits ascending-column rows. Unit-test round-trip on the existing
   csr test fixtures (triangle/path oracles per the module's convention).
3. **Types** — add `NormalizedTile`, `TileAddress`, `FNV_OFFSET_64`/`FNV_PRIME_64` to
   `kernel/src/csr.rs` exactly as specified in §3.2 (privacy is the invariant — no `pub` fields,
   no extra constructors, no `&mut` accessor).
4. **Rewire the hash path** — in `kernel/src/spectral_cache.rs`: change `slem_cached` to the
   `&NormalizedTile` signature; route the key through `tile.content_address().as_hex()` and the
   eigensolve through `tile.to_dense()`; demote `matrix_content_address` to private (keep its
   body — the adversarial test reconstructs the raw path inline, it does not call it). Update
   the single caller `kernel/src/markov.rs:209` per §3.2.
5. **Snapshot admission** — add `RetainedBase` + `SnapshotRejected` to
   `kernel/src/spectral_cache.rs` per §4.3 (gate on `classify_drift(raw.to_dense())` FIRST,
   canonicalize second, address third; no bypass parameter).
6. **Tests** — write the remaining five §5 tests, names verbatim, RED-first where the table
   says so.
7. **Bench** — §8 benches; append measured numbers to `BENCH_HISTORY.md`.
8. **Ledger** — append §11's three rows to `docs/regressions/REGRESSION-LEDGER.md`.
9. **Gate** — `cargo test -p kernel` green; §6 grep checks pass; commit with the contextual
   format, citing this blueprint and `f30189262` as the port source.

Acceptance = §6 verbatim. Out of scope (do NOT build here): epoch gossip, MMR, Merkle
bisection, reorder buffer, log truncation, the arena maintenance pass itself (doc 11 build-order
steps 3–8 and arena blueprint W5 own those; W5 will consume `RetainedBase::admit` as its only
retain path).

---

## 13. Reuse-first accounting (contract item 19)

Every piece extends an existing module; no new file is created. `NormalizedTile` wraps the
existing `row_normalize` (not a second normalizer); `TileAddress` reuses the FNV-1a-64 already
in the codebase (consolidating two inline literal sites into named consts); `slem_cached` keeps
`DecompCache` and its falsifier counters untouched; `RetainedBase` reuses `classify_drift` and
mirrors the Law-pole rejection shape of `CommitError::Rejected`. The one signature break
(`slem_cached`) has a blast radius of exactly one production caller (G7) — the refactor is not
avoided to avoid responsibility; it is the fix. Rejected cheaper alternatives: a runtime
`assert!(is_normalized)` (vigilance-dependent, absence-invisible — exactly what doc 19 Part 2
proves inferior) and a parallel `slem_cached_normalized` beside the raw one (leaves the buggy
public path alive = dual authority, the ADR-RCI round-1 error class, doc 11 §11).

## 14. Hermetic principles honored (contract item 20)

- **P1 MENTALISM** — the spec is the source of truth: the invariant lives in the type
  definitions (§3.2/§4.3), and code is derived from them; this document precedes the code.
- **P2 CORRESPONDENCE** — one concept, one primitive: ONE canonical form, ONE address producer,
  ONE FNV constant authority; the parallel raw-hash path is removed, not deprecated.
- **P4 POLARITY** — `SnapshotRejected` keeps the Law-reject pole typed and distinct from any
  durability fault, mirroring `CommitError`'s two poles; the port preserves both poles' tests.
- **P6 CAUSE-AND-EFFECT** — determinism as law: N3 keeps the hash path inside the
  correctly-rounded IEEE-754 subset (`rng.rs:18-29`); no effect (cache hit, snapshot key)
  without its declared cause (canonical bytes).
- **P7 GENDER / RC-2** — no self-certified pass: §4.2's anti-vacuity test exists precisely so
  the gate cannot silently become an organ without teeth; RC-3 (fail-open hole) is the class
  the §2 port closes.

## 15. Linux-discipline verdicts (contract item 9, reusing the adoption framework)

Per `BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-ADOPTION-2026-07-17.md`'s vocabulary:
§2 port = **GAP closure** (a known-correct fix stranded on a sibling branch);
type-level invariant = **REINFORCES** (never-break-callers: one caller, value-preserving for
already-canonical input); anti-vacuity test = **EXTENDS** (regression-as-institution);
runtime-assert alternative = **DOES-NOT-TRANSFER** (rejected §13).

## 16. Links (contract item 7)

Docs: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the contract) ·
`bebop2-mesh-tensor-hermetic-2026-07-17/11-…` §12 + build-order (supersedes nothing; this
blueprint EXECUTES its steps 1–2 and the two doc-19 bridges) · `19-…` Parts 1–2 (the pipeline +
authority boundary this encodes) · `BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md` §A/§E
(W1-L2/L10/L11 → this document is their execution spec) ·
`BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md` (W5 consumer of `RetainedBase`) ·
`hermetic-architecture-2026-07-16/HERMETIC-ARCHITECTURE-PRINCIPLES.md` ·
`docs/regressions/REGRESSION-LEDGER.md`.
Memory: `harness-llm-backend-and-hermetic-remediation-2026-07-17.md` (branch context) ·
`internal-retrieval-living-memory-arc-2026-07-14.md` (§10) ·
`sovereign-architecture-19-phase-roadmap-2026-07-17.md` (P06 remains the cross-cutting blocker;
P-B does not touch it and is not blocked by it).

### Contract-item map (self-audit)

1→§1 · 2→§6 · 3→§5 · 4→§3.2/§4.3 · 5→§5 (anti-vacuity + adversarial rows) · 6→§3.2/§4.2/§9 ·
7→§16 · 8→§7 · 9→§15 · 10→§8 · 11→§9 · 12→§9 · 13→§9 · 14→§5 (compile cases + CI grep) ·
15→§10 · 16→§3/§4 (spectral/csr machinery reused, no new solver) · 17→§11 · 18→§12 · 19→§13 ·
20→§14.
