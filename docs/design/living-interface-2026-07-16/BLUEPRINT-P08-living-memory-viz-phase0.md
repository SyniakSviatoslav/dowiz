# BLUEPRINT-P08 — Living-Memory Visualization Phase-0 (HUB tier) + the Phase-5 spectral-coords primitive

> Status: **execution-ready blueprint (2026-07-16)**. Planning only — no product code, CI config, or
> canon edited. Covers roadmap **Phase 5** (Field Semantics + Spectral Embedding — the one net-new kernel
> primitive) and **Phase 8** (Living-Memory Viz Phase-0, HUB tier). These two phases are tightly coupled:
> Phase 8 cannot render a tangle-free cluster without the Phase-5 coordinate helper, so both are specified
> here.
>
> **Primary source (already-designed, not re-litigated):**
> `docs/design/living-interface-2026-07-16/R-LM-living-memory-visualization-architecture.md` (full).
> **Sequencing/joints:** `LIVING-INTERFACE-ROADMAP.md` §1 (R-LM prerequisite row), §4 (Phase 5/8 rows),
> §5 (J1/J2/J7/J9). This document turns R-LM's recommendations into concrete module/function/wire targets.
> Every kernel fact below is cited against **live source verified 2026-07-16** (not the stale Repowise index).
>
> **⚠ Cross-reference (2026-07-16):** §2 below (the Phase-5 spectral-coords primitive) is
> **G11-relevant** and has been duplicated into `G11-FAST-PATH-CONSOLIDATED.md` §6 as part of the
> fast-path consolidation, alongside `BLUEPRINT-P00/P01/P02/P06/P09A` (those five were deleted after
> merging; **this file was NOT deleted** because §§1,3-8 cover the full HUB-tier living-memory
> visualization, which is off the G11 fast-path per `LIVING-INTERFACE-ROADMAP.md` §8 and remains the
> growth-substrate track's separate deliverable). This file stays the authoritative source for
> everything except §2, which now has two homes by design — keep them in sync if either changes.

---

## 1. Current-state evidence — what the substrate ACTUALLY is today

The whole point of Phase-0 is that **every acceptance claim binds to a test that already exists in the
kernel.** The viz invents no test data and no new correctness property — it renders numbers the kernel
already computes and proves.

### 1.1 The HUB fixture graph — `kernel/src/retrieval/diffusion.rs` (verified)
- A **frozen 20-node / 41-directed-edge wikilink graph**. `N = 20` (`diffusion.rs:22`), `SEED = 0`
  ("MEMORY.md", `:25`), `ALPHA = 0.15` (`:28`), `K = 20` fixed iterations (`:31`). Edge list
  `WIKI_EDGES` (`:38-100`) is the exact 41-edge set; `green_wikilink_fixture_shape` (`:169-176`) asserts
  `N==20 && WIKI_EDGES.len()==41`. **This is the graph the HUB builder streams in Phase-0 — do not
  invent new data.**
- **The unreachable-node ground truth (binds Phase-8 acceptance #2):** nodes `{5,6,12,16}` sit in a
  separate component; the const `UNRELATED: [usize;4] = [5,6,12,16]` (`diffusion.rs:166`).
  `green_relatedness_ranking_correct` (`:220-261`) asserts for each `u` in `UNRELATED`:
  `assert_eq!(scores[u], 0.0, ...)` (`:228-234`), and the structural clincher
  `reachable ⇔ score>0` (`:252-260`) over `reachable_from(SEED)` (`:143-156`). **PPR score is *exactly* 0
  for these four — this is what the viz must render DARK.**
- **The determinism proof pattern (binds Phase-5 #2 and Phase-8 #1):**
  `green_ppr_byte_identical_two_runs` (`diffusion.rs:191-203`) runs `ppr.rank(SEED,ALPHA,K)` twice and
  asserts both `Vec<f64>` bitwise equality *and* full-precision `{:.17e}` serialization equality. **The
  spectral-coords helper must reproduce this exact pattern for positions.**

### 1.2 The diffusion / relatedness engine — `kernel/src/csr.rs`, `kernel/src/retrieval/ppr.rs` (verified)
- `Csr { row_ptr, col_idx, val }` (`csr.rs:46`), `from_edges` (`:79`), `row_normalize` (`:125`).
- `Csr::personalized_pagerank(seed, alpha, iters) -> Vec<f64>` (`csr.rs:228`) — synchronous Jacobi PPR,
  fixed K, fixed summation order, bit-reproducible. The fixture uses the mirror engine `Ppr::rank`
  (`ppr.rs:42`), documented as a bit-for-bit reuse of `markov.rs`' accumulation order (`ppr.rs:3-16`).
  **This is the ACTIVITY the viz streams.**
- `Csr::laplacian_spmv(x, out, LaplacianKind::{Unnormalized|Normalized|RandomWalk})` (`csr.rs:307`, enum
  `:279`) — the exact `y = L·x` per-frame operator the physics-ui thesis unifies. Allocation-free.
- `recall_at_k(scores, relevant, k) -> f64` (`csr.rs:387`) and `precision_at_k` (`:409`) — deterministic
  scorers with tie-break. **These certify Phase-8 acceptance #3.**

### 1.3 The recall@5 = 1.0 proof — `kernel/src/living_knowledge.rs` (verified)
- `kernel_bm25_recall_at_5_is_one_point_zero` (`living_knowledge.rs:286-322`): a **12-doc fixture corpus**
  (`lk_fixture_corpus`, `:252-267`) + **12-query hand-verified oracle** (`lk_oracle`, `:269-284`, each
  query relevant to exactly one doc index) is ranked by the in-kernel `Bm25`, encoded into a score vector,
  and certified by `crate::csr::recall_at_k(&scores, relevant, 5)` (`:305`) asserting `r == 1.0` per query
  and `mean == 1.0` (`:307-317`). The PRIMARY delegation seam is
  `living_knowledge::recall_at_k(query,k)` (`:134-136`). **This is the exact test Phase-8 acceptance #3
  "seeding a query lights the top-5 containing the relevant record" binds to.**

### 1.4 The spectral signature — `kernel/src/spectral.rs` (verified) — and the GAP
- `graph_spectrum(adj) -> GraphSpectrum { spectral_radius, slem, spectral_gap, fiedler, energy, drift }`
  (`spectral.rs:263`, struct `:254-261`). `laplacian(adj)` builds **L = D − A** (`:287-297`).
  `algebraic_connectivity(adj) -> f64` returns the Fiedler value λ₂(L) (`:302-311`); `classify_drift`
  → `DriftClass::{Damped|Resonant|Unstable}` by ρ vs the unit circle (`:325-335`, enum `:316-323`).
  **`spectral` ships in the `LayoutKeyframe` health block unchanged.**
- ⚠️ **The load-bearing gap:** `spectral.rs` computes **eigen*values* only** — `eigenvalues(adj)`
  (`:195-214`) dispatches to `householder::eigenvalues_contig` (`householder.rs:338`, values only) for
  n≤32 or the `charpoly`+`roots` (Durand-Kerner, fixed seed `Complex(0.4,0.9)`, **no RNG**, `:141-186`)
  path for n>32. A grep for `eigenvector|coords_2d|coords_3d|spectral_embedding` across the whole kernel
  returns **nothing** (only a Perron-vector *comment* in `order_machine.rs`). **There is no eigenvector →
  coordinate function in the kernel today. That function is the ONE net-new primitive (Phase 5).**

### 1.5 The event substrate — `kernel/src/event_log.rs` (verified) — the free idempotency
- `MeshEvent { prev:[u8;32], actor_pubkey:[u8;32], actor_seq:u64, payload:Vec<u8> }` (`event_log.rs:134`)
  with content-id `event_id() = sha3_256(prev‖actor_pubkey‖actor_seq‖payload)` (`:148`, `sha3_256` at
  `:30`). `event_id_is_deterministic_and_content_keyed` proves same-content ⇒ same id (`:433-444`).
- `AppendOutcome::Duplicate([u8;32])` (`:222-227`): re-appending identical content is a **structural
  no-op** (proven `:475`). `commit_after_decide` runs `decide` before persist (`:300`);
  `commit_after_decide_drift_gate` (`:347`) additionally rejects an `Unstable` (ρ>1) mutation pre-persist.
  `EventStore` trait `get()` (`:162`) exists "for durable session-boundary re-verify (G4/G5)".
- **Consequence for the wire:** `ActivityDelta.signals[].event_id` is *this* content-id, so the client
  dedups reorders/dupes for free (J1). The `EventStore::get` seam is exactly where the capability
  session-boundary re-verify (§5) hooks.

### 1.6 The spine nodes & mesh topology (grounded via R-LM §1.2/§1.5)
- `SpineRecord { id, kind:RecordKind, payload_hash:[u8;32], prev_hash, record_hash }`,
  `RecordKind::{Memory|Identity|Intent}`, append-only with `verify_chain()` — **the spine stores only
  `payload_hash`, never the payload** (PII-free; matters for §5). `hydra::topology_adjacency(nodes,edges)`
  is the MESH-tier graph source (deferred). These are `RecordKind` = the `node_kind` byte at HUB tier.

### 1.7 The client engine substrate — `engine/` (verified)
- `dowiz-engine` crate (`engine/Cargo.toml`): pure-Rust, **zero external deps by default**, `gpu` feature
  is an EMPTY honest-`Err` stub today ("wgpu uncached" — the Phase-0 roadmap `cargo add wgpu` unblocks it).
  Depends on `dowiz-kernel` by path.
- `ParticlePool` (`engine/src/widget_store.rs:68`) is the RW-04/FE-02 SoA ring: `pos_x: Vec<f32>`,
  `pos_y: Vec<f32>`, `vel_*`, `life`, `color` — **`pos_z` does not exist yet.** The zero-copy staging in
  `engine/src/bridge.rs:70` is `[pos_x*N][pos_y*N][vel_x*N][vel_y*N]`. `money_guard.rs` (FE-09) is present.
  **No bloom pass exists** (grep confirms) — Phase-3 adds it; Phase-8 consumes it.

---

## 2. Phase-5 — the spectral-coords primitive (the ONE net-new kernel primitive)

### 2.1 Where it lives — a new sibling module, NOT an edit to `spectral.rs`
**Target: a new module `kernel/src/spectral_embedding.rs`** (sibling of `spectral.rs`), exported from
`kernel/src/lib.rs`. Rationale: `spectral.rs` is the eigen*value* engine and stays value-only; the
embedding is a distinct concern that needs an eigen*vector* source the kernel's `householder` engine does
not provide. Keeping it a sibling means Phase-5 touches `spectral.rs` **not at all** (no regression surface
on the FSM/drift-gate callers) and adds a clean, independently-testable unit.

### 2.2 Public surface
```
// kernel/src/spectral_embedding.rs
pub fn coords_2d(adj: &[Vec<f64>]) -> Vec<[f64; 2]>   // n rows → (x,y) = (φ₂, φ₃)
pub fn coords_3d(adj: &[Vec<f64>]) -> Vec<[f64; 3]>   // n rows → (x,y,z) = (φ₂, φ₃, φ₄)
```
Input is the same adjacency shape `graph_spectrum`/`laplacian` already accept (`Vec<Vec<f64>>`). For the
HUB fixture the caller passes `diffusion::wiki_row_stochastic()` symmetrized (or the raw undirected
adjacency of `WIKI_EDGES`) — the layout graph is treated as undirected (edge = relatedness) so `L = D − A`
is symmetric and its eigenvectors are real.

### 2.3 Internal design (Hall spectral embedding, FE-12)
1. Build **L = D − A** by reusing `spectral::laplacian(adj)` (`spectral.rs:287`) — no new Laplacian code.
2. Compute the **low eigenvectors** of L via the **RW-01-vendored `field-math` eigensolver** (bebop2
   `field.rs`, which per physics-ui-capture §2 already returns eigenmode vectors + `propagate_spectral`).
   This is why Phase-5 depends on Phase-2 (RW-01 vendoring): the kernel's own `householder` path returns
   eigen*values* only, so the vector source must come from the vendored solver. The embedding module is the
   **single consumer** of that solver's eigenvector output.
3. Discard φ₁ (the constant DC eigenvector for λ₁≈0 — it collapses all nodes to one point) and take the
   next low eigenvectors: **φ₂ → x, φ₃ → y, φ₄ → z** (the Fiedler vector and its successors — the axes of
   maximal cluster separation, which is exactly why spectral beats force-directed on tangle-freeness).
4. Emit `[f64;2]`/`[f64;3]` per node. (Positions are `f64` in-kernel for determinism; the wire narrows to
   `f32` — see §4.)

### 2.4 Determinism discipline — the non-obvious part
Eigenvectors have **two free choices** that would break byte-identical reproduction if left unpinned, and
both must be canonicalized inside the helper:
- **Sign:** `φ` and `−φ` are both valid eigenvectors. **Fix:** flip each eigenvector so its
  first structurally-nonzero component is positive (deterministic sign convention). Without this, two runs
  can legitimately return mirror-image layouts and the byte-identity test flaps.
- **Ordering / degeneracy:** eigenvectors for (near-)equal eigenvalues can be returned in any order or
  rotated within the eigenspace. **Fix:** sort by ascending eigenvalue with a **deterministic tie-break on
  a stable index** (mirror the tie-break discipline already in `csr::recall_at_k`, `csr.rs:387`); the
  vendored solver must use a fixed iteration count / fixed seed (paralleling `spectral::roots`' fixed
  `Complex(0.4,0.9)` seed and PPR's fixed-K). No convergence-epsilon early-out.

With sign + order pinned and the solver seed fixed, `coords_3d(adj)` run twice is **bitwise identical** —
the property Phase-5 acceptance #2 asserts, structurally the same test as
`diffusion::green_ppr_byte_identical_two_runs`.

### 2.5 Why spectral, not force-directed (the RED case)
The done-test RED case is a **naive force-directed (Fruchterman-Reingold) local-minimum tangle**: FR needs
a random init and an iteration budget, so it is nondeterministic *and* can settle into a crossed/tangled
layout on a graph with a clear cluster structure. The spectral embedding places nodes along the
Laplacian's low eigenvectors, so the `{5,6,12,16}` disconnected component separates cleanly along φ₂ (it
lives in a distinct eigenspace) — tangle-free by construction and reproducible. FE-07 SMACOF is retained
**only** as the warm-start relaxation for single-node topology changes (J9 / R-LM F-2), not the cold layout.

---

## 3. The 3-tier wire protocol — architect all three, ship only HUB

**Core rule (R-LM §2, §6):** all three tiers are the SAME primitive over a DIFFERENT graph. The wire
carries a `tier` + `epoch` + `graph_id` envelope from the first commit so MESH and NODE are **additive
later, never a breaking change.** Only the HUB builder ships in Phase-0.

### 3.1 Enumerations (fixed byte encodings — execution-ready)
```
enum Tier      : u8 { Mesh = 0, Hub = 1, Node = 2 }              // MESH/NODE builders deferred (Phase 10)
enum NodeKind  : u8 { Memory = 0, Identity = 1, Intent = 2 }      // == spine::RecordKind at HUB; hub-type at MESH
enum Zone      : u8 { Intake = 0, Memory = 1, EventLedger = 2, Mesh = 3, Spectral = 4, DeepClean = 5 }
enum SignalType: u8 { Event = 0, Decide = 1, Recall = 2, Gossip = 3, Drift = 4, CycleComplete = 5 }
enum Kind      : u8 { Order = 0, Error = 1, Notification = 2, Success = 3, Loading = 4 }   // FE-10 vocab
enum Drift     : u8 { Damped = 0, Resonant = 1, Unstable = 2 }    // == spectral::DriftClass
```
**Phase-0 emits `Tier::Hub` and, of `SignalType`, only `Recall`.** The other five `SignalType` variants
are reserved in the enum (so no wire change to add them) but never produced in Phase-0.

### 3.2 `LayoutKeyframe` — low frequency (~0.1–1 Hz / on topology change)
```
LayoutKeyframe {
  tier:      u8,            // Tier — Phase-0: always Hub(1)
  epoch:     u64,           // monotonic; every ActivityDelta pins to this
  graph_id:  [u8; 32],      // sha3_256 over the sorted (node,edge) set — content-addressed graph identity
  lod:       u8,            // decimation level (Phase-0: always 0 = full 20 nodes)
  nodes: [ NodeLayout {
      id:        u32,       // stable node index into this epoch's layout
      pos:       [f32; 3],  // coords_3d output, narrowed f64→f32 for the wire (pos_z=0 possible for 2-D)
      zone:      u8,        // Zone (derived from node-label prefix at HUB)
      node_kind: u8,        // NodeKind
      radius:    f32,       // base somatic radius (before energy scaling)
  } ],
  edges: [ EdgeLayout { src: u32, dst: u32, weight: f32 } ],   // decimated per lod (Phase-0: all 41)
  spectral: { rho: f32, slem: f32, fiedler: f32, energy: f32, drift: u8 },   // == spectral::graph_spectrum
}
```
`graph_id` is computed by the server via the kernel's own `sha3_256` (`event_log.rs:30`) over the
canonicalized edge set — this is what lets the client detect "same graph ⇒ same layout" and is the anchor
for the byte-identity assertion.

### 3.3 `ActivityDelta` — high frequency (coalesced up to render rate)
```
ActivityDelta {
  epoch:     u64,          // MUST equal the client's current layout epoch, else buffered (never mis-applied)
  t_logical: u64,          // ordering key (actor_seq-derived) — the SINGLE scheduling authority (§4.3, J2)
  signals: [ Signal {
      node:        u32,        // index into the epoch's NodeLayout
      event_id:    [u8; 32],   // == MeshEvent::event_id → idempotent dedup (event_log.rs:148)
      signal_type: u8,         // SignalType — Phase-0: always Recall(2)
      kind:        u8,         // Kind (FE-10 vocab)
      energy:      f32,        // PPR salience → glow amplitude + audio grain amplitude
      edge_path:   [u32]?,     // optional traveling-signal node path (pulse animation)
  } ],
  salience: [(u32, f32)]?,     // optional sparse PPR update, coalesced
}
```
**Idempotency (J1):** because `event_id` is the `event_log` content-id, a dropped / duplicated / reordered
delta is a **structural no-op** on the client — the viz inherits `AppendOutcome::Duplicate`'s guarantee
(`event_log.rs:222-227`) for free. **Epoch pinning (J1):** a delta whose `epoch ≠ current layout epoch` is
**buffered, never applied to the wrong layout**; a delta referencing a node absent from the current epoch
is held/spawned at neighbours' centroid, never dropped.

### 3.4 Reserved-but-unbuilt slots (MESH / NODE) — proof they are additive
- **MESH later** = a new *server-side* graph-builder over `hydra::topology_adjacency(nodes, base_edges)`
  emitting `Tier::Mesh` keyframes; nodes = hubs, a "signal" = a `SignedFrame`/`BreachAlert`. Transport
  upgrades to the **signed** mesh path (`SignedFrame` + proto-cap, ML-DSA per-frame — affordable at MESH's
  low frequency, J8). **No `LayoutKeyframe`/`ActivityDelta` field changes** — only `tier=0` and a new
  builder.
- **NODE later** = a new builder over one `SpineRecord`'s ego-graph + its `event_log` sub-chain, emitting
  `Tier::Node`. NODE drill-down must **redact PII/money server-side** (spine is `payload_hash`-only, but
  `retrieval/memory_store.rs` holds real content — §5).
- Both are pure server additions behind the identical wire. The `tier` byte existing now is the entire cost
  of that future-proofing.

---

## 4. HUB-tier server / client data flow (Phase-0)

### 4.1 The hybrid split (R-LM §3.1)
- **SERVER (native kernel, GPU-less Hetzner VPS) computes STATE:** (1) positions via
  `spectral_embedding::coords_3d` over the fixture adjacency (§2); (2) health via `graph_spectrum`
  (`spectral.rs:263`); (3) activity via `personalized_pagerank` / `Ppr::rank` from `SEED`
  (`csr.rs:228` / `ppr.rs:42`). Positions and activity are **genuine backend numeric state**, not a render
  artifact.
- **CLIENT composites & animates** — it **invents no state**, only eases old→new *streamed* positions
  (FE-08 ζ=1, §6). This is the F-7 invariant.

### 4.2 Server sequence (Phase-0, over the fixture)
1. On subscribe (after the capability check, §5): build the undirected fixture adjacency from
   `diffusion::WIKI_EDGES`, compute `coords_3d` + `graph_spectrum`, assign `epoch = 1`,
   `graph_id = sha3_256(canonical edges)`, `lod = 0`, map each node label prefix → `Zone`, and emit **one
   `LayoutKeyframe`** (`tier = Hub`).
2. **Replay the `Recall` wave:** run PPR from `SEED`; for each node with `score > 0`, emit a `Signal`
   (`signal_type = Recall`, `energy = score`) inside `ActivityDelta { epoch:1, t_logical:k }`, coalesced.
   The `{5,6,12,16}` nodes have score exactly 0 → **no signal emitted for them → they stay dark.** This is
   the server side of Phase-8 acceptance #2, sourced directly from the `diffusion.rs` truth.
3. **Query seed (acceptance #3):** for a query, run the `living_knowledge::recall_at_k` path
   (`living_knowledge.rs:134`) → the top-5 doc indices → emit `Recall` signals lighting exactly those
   nodes. The relevant node is guaranteed present by `kernel_bm25_recall_at_5_is_one_point_zero`.

### 4.3 Ordering authority — the J2 discipline (LOAD-BEARING)
Both the visual pulse and the audio grain (§6) schedule off the **one kernel-validated monotonic
sequence** keyed by `t_logical` (derived from `actor_seq`) — **never raw wire-arrival order.** Phase-8
**consumes** the ordering authority established in Phase 6 (event stream) and Phase 7 (audio); it does not
invent its own clock. A single small presentation-lookahead buffer (~80–150 ms) feeds both renderers so
the glow peak and the grain land on the same perceived instant. This is cheap to design in now and
expensive to retrofit (J2), which is why it is stated as a hard constraint on this phase, not an
optimization.

### 4.4 Transport (Phase-0)
HUB is same-hub / local (client and server are the *same* hub) → a **lightweight unsigned local delta
protocol** over the localhost/in-process boundary. Per-frame ML-DSA signing is infeasible and wrong-layer
for a 60 fps intra-hub stream (J8); the trust boundary is the **subscribe-time capability check** (§5), not
per-frame signatures. Signed transport is a MESH-only concern (Phase 10).

---

## 5. Access-control design — proto-cap capability, fail-closed (J7)

### 5.1 The capability
Gate the stream behind a proto-cap token:
```
Capability {
  resource: Resource::LivingMemoryView,   // new resource variant
  scope:    Scope,                          // Phase-0: Owner only (default = minimum scope)
  expiry:   <timestamp>,                    // SHORT-LIVED (see 5.3)
  // ML-DSA-signed; nonce/replay + expiry fields per M12 proto-cap
}
```
`Resource::LivingMemoryView` is the one new capability variant Phase-0 introduces. Phase-0 issues **owner
scope only**; the scope enum is designed to carry finer roles later (coarse `up/down/partitioned` health
for low scope vs full `graph_spectrum` detail at owner scope) without a protocol change.

### 5.2 Enforcement — verify twice, fail-closed
- **On subscribe:** verify ML-DSA signature + scope + expiry. If any check fails → **explicit `Err`
  stream reject.**
- **At session boundary:** re-verify via the `EventStore::get` seam (`event_log.rs:162`, the G4/G5 durable
  re-verify hook).
- **Fail-closed = explicit reject, NEVER a silently-empty stream** — mirroring the `living_knowledge`
  house rule "*any error returns Err — retrieval never silently degrades to empty results*". This is
  Phase-8 acceptance #5, and it is a deliberately-designed-out class of bug: an empty stream that *looks
  like* a well-behaved "no data" response but is actually an unauthorized viewer being silently denied is
  a fail-open-masquerading-as-fail-closed security defect. The reject must be a typed `Err`, distinguishable
  from an authorized-but-empty graph.

### 5.3 The RevocationSet gap — FLAGGED, not solved here
🔴 **Known real gap (R-LM F-4, J7):** MESH-REAL-PLAN records *"revocation НЕ існує (лише expiry)"* — the
`RevocationSet` (M12) is **specced but not built**; only expiry is checked today. A viz capability that is
granted then revoked **keeps streaming until it expires.** Phase-0 does **not** solve this. Mitigations
that Phase-0 DOES ship:
1. Issue viz capabilities **short-lived** (small expiry window), so the un-revocable window is bounded.
2. **Re-verify at every session boundary** (5.2) — the shortest natural revocation boundary available
   today.
**Named follow-up:** wire the `RevocationSet` check the moment the set exists — this is a **Phase-10**
line-item (roadmap §4, Phase 10 "RevocationSet wiring"), not in scope here. This is called out explicitly
so it is never mistaken for solved.

### 5.4 PII boundary (reserved for NODE)
HUB streams the spine graph, which is **`payload_hash`-only → PII-free** (`spine.rs`). NODE-tier
drill-down (Phase 10) will expose `retrieval/memory_store.rs` content and MUST redact PII/money payloads
server-side via the existing `apps/api/.../anonymizer` boundary. Phase-0 ships no NODE tier, so no PII
crosses the wire — but the redaction obligation is recorded against the reserved NODE slot.

---

## 6. Client rendering — reuse Phase-3 bloom + RW-04 ParticlePool, build no parallel renderer

**Hard rule:** Phase-8 adds **zero new rendering machinery** beyond what Phases 2–4 already ship. It is a
new *consumer* of existing engine parts, not a second renderer.

### 6.1 The ONE required engine extension — `pos_z`
`ParticlePool` (`engine/src/widget_store.rs:68`) is 2-D today (`pos_x`, `pos_y`). Phase-8 needs one scoped
delta to **RW-04**: add `pos_z: Vec<f32>` alongside `pos_x`/`pos_y`, and widen the zero-copy staging in
`engine/src/bridge.rs:70` from `[pos_x*N][pos_y*N][vel_x*N][vel_y*N]` to include the `pos_z` lane. This is
the single structural change; everything else is reuse. (Roadmap Phase 3 already lists RW-04 "extended to
`pos_z`" — Phase-8 is where that extension is *consumed*.)

### 6.2 What is reused verbatim
- **Somata (neuron bodies):** RW-04 instanced emissive billboard quads, additive `SrcAlpha,One`, HDR
  emissive color > 1.0, size `= 2 + life·6·energy`. Requires RW-04's blue-hardwire fix (widen meta → full
  RGBA) — Phase-8 *depends on* that Phase-3 fix. Color from `zone` (T2 palette) × `energy`.
- **Synapses (edges):** FE-05 SDF capsule/fat-line tubes with analytic `fwidth` AA (no MSAA); a traveling
  pulse = a moving `smoothstep` window along the edge driven by `Signal.edge_path`.
- **Selective bloom pass:** the **Phase-3 bloom pass** (threshold bright emissive → multi-mip gaussian blur
  → composite → AgX/ACES tone-map → dark fog). Phase-8 **does not build bloom** — it composites through the
  pass Phase-3 already stood up (roadmap Phase 3 "net-new selective bloom pass"). Philosophically consistent:
  a gaussian blur *is* the heat kernel `e^{−tL}`, the same operator family.
- **Motion:** FE-08 per-property critically-damped **ζ=1** spring eases each node old→new streamed
  position — monotone, no overshoot, so the client provably converges to server state and never invents a
  position (F-7). FE-03 fixed-dt (`DT=0.02`) accumulator; FE-14 lazy-render-on-settle (dormant rAF).
- **Fallback:** WebGPU-primary, **WebGL2 fallback via naga** (FE-16), degrade-to-static-DOM (RW-11).
- **Brand tint (F-3):** the server streams brand-**neutral** state; the client applies the ambient Sea
  tint from its **locally-hydrated** token cascade. The viz marks read only T2/T3 tokens, never the 5 T1
  brand inputs — the wire never carries tenant brand identity.

### 6.3 Audio — ONE grain per activation (consumes Phase-7)
Phase-8 ships **one signal type (`Recall`) and one audio grain per node activation**, rendered by
**Phase-7's sonification renderer** (the third wasm artifact / AudioWorklet). Phase-8 consumes it off the
**same `ActivityDelta` stream** and the **same `t_logical` ordering authority** (§4.3, J2) — it does not
open a second audio path. Money is never a sonic parameter (FE-09 red-line extends into the audio crate).
**grain count == lit-node count** is the A/V-sync falsifier (acceptance #4): because both the glow and the
grain are driven off one `Signal` per lit node, the counts must match exactly.

---

## 7. Acceptance criteria — falsifiable checklist

Phase-5 and Phase-8 criteria are separated. Each binds to a named, already-existing kernel test or a
new test that reproduces an existing pattern.

### Phase 5 — spectral-coords primitive (roadmap Phase 5 done-tests)
- **P5-1 — Tangle-free layout.** On the 20-node fixture, `coords_3d` separates the `{5,6,12,16}`
  disconnected component cleanly along φ₂ (distinct eigenspace); assert the four unrelated nodes are
  spatially separated from the reachable cluster. **RED case:** a naive Fruchterman-Reingold force-directed
  layout produces a local-minimum tangle (crossed edges, nondeterministic). Spectral is tangle-free by
  construction.
- **P5-2 — Byte-identical positions.** `coords_3d(adj)` run twice ⇒ bitwise-identical `Vec<[f64;3]>` AND
  identical `{:.17e}` full-precision serialization — **the exact structure of
  `diffusion::green_ppr_byte_identical_two_runs` (`diffusion.rs:191-203`)**, achieved via the sign +
  ordering canonicalization and fixed solver seed (§2.4). This is the determinism gate the repo's
  VERIFIED-BY-MATH rule requires.
- **P5-3 — One action = one field impulse.** The embedding is one operator over `L = D − A` (reusing
  `spectral::laplacian`), not per-component feedback code. **RED case:** per-component layout code.

### Phase 8 — Living-Memory Viz Phase-0, HUB tier (roadmap Phase 8 done-tests)
- **P8-1 — Same graph ⇒ byte-identical positions.** End-to-end: identical fixture ⇒ identical
  `LayoutKeyframe.nodes[].pos` (server side inherits P5-2; wire narrows f64→f32 deterministically). Assert
  reproduction across two subscribe cycles; `graph_id` identical.
- **P8-2 — Activity matches kernel truth exactly (the 4 dark nodes).** The lit-node set = nodes with
  PPR `> 0` = exactly `diffusion::reachable_from(SEED)`; the **4 unreachable nodes `{5,6,12,16}` render
  DARK (PPR score exactly 0).** Bound to `green_relatedness_ranking_correct`
  (`diffusion.rs:220-261`, the `assert_eq!(scores[u], 0.0)` at `:228-234` and `reachable ⇔ score>0` at
  `:252-260`) and the `UNRELATED` const (`:166`). No signal is emitted for these nodes → they never light.
- **P8-3 — Query seeding lights the top-5 containing the relevant record.** Seeding a query lights exactly
  the top-5 nodes, which contain the relevant node. Bound to
  `kernel_bm25_recall_at_5_is_one_point_zero` (`living_knowledge.rs:286-322`, `mean recall@5 == 1.0`) via
  the `living_knowledge::recall_at_k` seam (`:134-136`).
- **P8-4 — grain count == lit-node count (A/V sync).** Each lit node fires exactly one audio grain (§6.3);
  assert `grains_fired == lit_nodes`. This is the A/V-sync falsifier and follows structurally from one
  `Signal` per lit node driving both renderers off one `t_logical`.
- **P8-5 — No owner capability ⇒ explicit stream REJECT (Err), never empty.** A subscribe without a valid
  `Capability{LivingMemoryView, Owner}` returns a typed `Err`, distinguishable from an authorized-but-empty
  stream (§5.2). RED case: a silently-empty stream (fail-open masquerading as fail-closed).

### Explicitly out of scope for Phase-0 (avoid gold-plating)
MESH & NODE UIs; cross-hub `SignedFrame` transport; live coherence `|ψ₁±ψ₂|²` interference (Tier-2, gated);
AR/native targets (DZ-12); the large-graph **Lanczos** eigensolver path (R-LM F-9 — dense path suffices for
n=20); full multi-signal-type sonification (only `Recall` ships); **`RevocationSet` wiring** (§5.3, Phase
10). All are protocol-reachable extensions behind the unchanged `tier`/`epoch`/`graph_id` wire.

---

## 8. Dependency & sequencing summary
- **Phase 5** depends on **Phase 2** (RW-01 `field-math` eigensolver vendor — the eigenvector source) and
  **Phase 4** (field-dynamics guards). Delivers `spectral_embedding::{coords_2d,coords_3d}`.
- **Phase 8** depends on **Phase 5** (positions) and **Phase 7** (sonification renderer + the ordering
  authority it consumes). Delivers the HUB-tier server builder + client cluster over the `diffusion.rs`
  fixture, with the 3-tier wire complete-but-HUB-only.
- The single most dangerous joint (J2) is honored by consuming Phase 6/7's one ordering authority; the
  single most security-relevant finding (J7) is honored by fail-closed subscribe-time + session-boundary
  verification, with the `RevocationSet` gap flagged as a Phase-10 follow-up, not solved here.

*End blueprint. Planning only — no product code, CI config, or canon edited. Extends R-LM / FE-12 / RW-01 /
RW-04 / FE-08/14/16 / the Phase-3 bloom pass without re-litigating their decided content.*
