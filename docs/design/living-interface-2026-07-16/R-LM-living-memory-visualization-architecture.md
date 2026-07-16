# R-LM — Living-Memory Visualization Architecture (grounded design, not implementation)

> Status: **research/design v1 (2026-07-16)**. Part of the *living-interface* arc. Scope: a
> real-time 3-D visualization of a hub's own living-memory / event substrate, rendered as a glowing
> neuron/synapse cluster with sound. This document is DESIGN ONLY — no code was written or edited.
>
> Companion / anchor docs (read together):
> - `docs/design/ARCHITECTURE.md` — canon (M4–M12, SCOPE RULE, V2, F14/F47/F48).
> - `docs/design/physics-ui-capture-blueprint.md` — the ONE-Laplacian-`L` unification thesis.
> - `docs/design/field-ui-engine/BLUEPRINTS-FIELD-UI.md` — FE-01…FE-17 GPU-engine machinery.
> - `docs/design/rust-engine-rewrite/BLUEPRINTS-RUST-ENGINE-REWRITE.md` — RW-01/04/05/09/10/11.
> - `docs/design/dowiz-interfaces/RESEARCH-CONSPECT.md` — Sea & Sheet, TOKEN 3 TIERS.
> - `docs/design/mesh-real/MESH-REAL-PLAN.md` — SignedFrame / proto-cap / pull-anti-entropy sync.
> - `docs/design/living-interface-2026-07-16/EXTERNAL-RESEARCH-gpu-neural-field-sonification.md`
>   — third-party GPU neural-field/sonification research (filtered through house rules; JS-framework
>   recommendation NOT adopted — see §4).
> - Kernel source: `kernel/src/{living_knowledge,spine,spectral,csr,event_log,hydra}.rs`,
>   `kernel/src/retrieval/{mod,ppr,diffusion,recall}.rs`, `kernel/src/wasm.rs`.

---

## 0. One-sentence thesis

The living-memory visualization is **not a new renderer** — it is **another consumer of the ONE
graph-Laplacian operator `L`** that `physics-ui-capture-blueprint.md §1` already unifies across
memory-recall / decay / UI-layout / UI-blur: the hub's memory is a graph, its **positions** come from
the Laplacian's spectral embedding (`L=D−A` eigenvectors), its **activity** from the Laplacian's
diffusion (`csr::personalized_pagerank`), its **health** from the Laplacian's spectrum
(`spectral::graph_spectrum`), and the client only *draws* that server-computed field as glowing
particles + sound. The 3 zoom tiers (MESH / HUB / NODE) are the **same primitive at three graph
scales**, not three renderers.

This realizes the already-anchored canon items **F14** ("*Hub uses webgl render of its own
topology … possible (E23 feature-gated)*") and **F47** ("*Demo = wasm physics/math render of a
delivery … NOW: demo; FUT: GPU after unlock*"), over the **F48** per-hub graph ("*each Hydra head
keeps its OWN graph (BD+spectral+history), syncs opportunistically … NO central authority*").

---

## 1. Current-state grounding — what the living-memory data structures ACTUALLY are today

Everything below is verified against live kernel source (not the stale Repowise index).

### 1.1 The event substrate — `kernel/src/event_log.rs`
- A **`MeshEvent { prev:[u8;32], actor_pubkey:[u8;32], actor_seq:u64, payload:Vec<u8> }`** is the atomic
  "signal". Its **content-id `event_id = sha3_256(prev ‖ actor_pubkey ‖ actor_seq ‖ payload)`** is the
  idempotency key (`event_log.rs:146-155`). The `sha3_256` is a dependency-free FIPS-202 Keccak
  (`event_log.rs:30-125`).
- The log is a **hash chain**: `append()` binds a zeroed `prev` to the current tip
  (`event_log.rs:257-273`). Re-appending identical content is a **`Duplicate` structural no-op** — not a
  timeout dedup (`AppendOutcome`, `event_log.rs:220-228`). **This idempotency is the consistency
  substrate the viz stream rides (see §5, F-1).**
- Commit is **fail-closed through the kernel Law**: `commit_after_decide` runs `decide` BEFORE persist;
  rejection persists nothing (`event_log.rs:300-319`). `commit_after_decide_drift_gate`
  (`event_log.rs:347-375`) additionally runs `spectral::classify_drift` on the organism adjacency and
  **rejects an `Unstable` (ρ>1) mutation pre-persist** unless `intervention` lifts safeties (operator
  directive §3). A committed payload is e.g. an order transition `b"Pending->Confirmed"`
  (`event_log.rs:505`).
- Stores are behind the **`EventStore` trait** (`event_log.rs:162-183`): `MemEventStore` (offline
  stand-in), `PgEventStore` (pgrust, node binary), `FileEventStore` (hydra durable std-only). `get()`
  exists specifically "*for durable session-boundary re-verify, G4/G5*".

### 1.2 The knowledge graph nodes — `kernel/src/spine.rs`
- **`SpineRecord { id:String, kind:RecordKind, payload_hash:[u8;32], prev_hash:[u8;32],
  record_hash:[u8;32] }`** (`spine.rs:62-74`). **Three kinds: `Memory | Identity | Intent`**
  (`spine.rs:26-34`) — these are the natural **node classes** of the neuron cluster.
- The spine is an **append-only, tamper-evident hash chain** with `verify_chain()` (`spine.rs:199-211`)
  and `query(kind)` (`spine.rs:184-186`). **Privacy-relevant: the spine stores only `payload_hash`,
  never the payload** — the spine graph itself is PII-free (matters for §5, F-4). Actual record content
  lives in `retrieval/memory_store.rs`.

### 1.3 The relatedness graph + diffusion — `kernel/src/csr.rs`, `kernel/src/retrieval/`
- **`Csr { row_ptr, col_idx, val }`** — a single-contiguous CSR graph (`csr.rs:45-54`), `from_edges`
  (directed), `row_normalize` → row-stochastic Â (`csr.rs:79-152`).
- **`Csr::personalized_pagerank(seed, alpha, iters)`** — synchronous Jacobi PPR, **fixed K, fixed
  summation order, bit-reproducible** (`csr.rs:228-264`). This IS the recall/activation diffusion.
- **`Csr::laplacian_spmv(x, out, kind)`** with `LaplacianKind::{Unnormalized, Normalized, RandomWalk}`
  (`csr.rs:307-359`) — **this is the exact `y = L·x` per-frame operator that
  `physics-ui-capture-blueprint.md §1` unifies memory-recall + UI-layout + UI-blur around.** It already
  exists, is allocation-free in the hot loop, and conserves mass (`L·1 = 0`).
- **`recall_at_k` / `precision_at_k`** in-kernel scorers with deterministic tie-break (`csr.rs:387-427`).
- The concrete **HUB neuron-graph fixture already in the kernel**: `retrieval/diffusion.rs` — a **frozen
  20-node / 41-directed-edge wikilink graph** (`diffusion.rs:22,33-100`), nodes labelled from the L0
  corpus (`fixtures::FIXTURE`), `SEED=0` ("MEMORY.md"), `ALPHA=0.15`, `K=20`. Nodes `{5,6,12,16}` sit in
  a **separate component → PPR score exactly 0** (the clean "unrelated" baseline). `retrieval/ppr.rs`
  is the bit-reproducible power-iteration engine mirroring `markov.rs`.
- **"recall@5 = 1.0" concretely**: `retrieval/recall.rs` + `living_knowledge.rs:287-322` — over a
  **12-doc kernel fixture corpus** and a **12-query hand-verified oracle** (each query relevant to
  exactly one doc index), the std-only **BM25 + trigram fusion** puts every relevant doc in the top-5,
  certified by `csr::recall_at_k == 1.0`. `living_knowledge::recall_at_k` delegates to this
  kernel-owned path (no JS, no network) (`living_knowledge.rs:134-142`).

### 1.4 The spectral signature — `kernel/src/spectral.rs`
- **`graph_spectrum(adj) -> GraphSpectrum { spectral_radius ρ, slem |λ₂|, spectral_gap, fiedler λ₂(L),
  energy Σ|λ|, drift }`** (`spectral.rs:254-282`) — the "vectorless signature" of any graph, single
  eigenvalue pass. `classify_drift -> {Damped | Resonant | Unstable}` by ρ vs the unit circle
  (`spectral.rs:315-335`). **`fiedler = algebraic_connectivity` returns 0 ⟺ the graph is disconnected**
  (`spectral.rs:302-311`, tests `spectral.rs:474-493`) — **this is the exact server-side signal for
  "mesh partitioned" (§5, F-2).**
- ⚠️ **GAP (load-bearing for layout):** `spectral.rs` computes eigen*values* only (charpoly / Householder,
  `spectral.rs:113-214`). There is **no eigen*vector* / spectral-embedding / coordinate function** in the
  kernel today. A Laplacian eigenvector→3-D-coords helper is the **one net-new kernel primitive** the
  layout needs — already scoped by **FE-12** ("*Hall spectral embedding L=D−W → (λ₂,φ₂),(λ₃,φ₃) via
  bebop2 `field.rs` eigensolver (add `coords_2d` helper)*") and vendored by **RW-01** (`field-math`
  vendors bebop2 `field.rs`, which per `physics-ui-capture §2` already holds the eigenmode spectrum +
  `propagate_spectral`). Not invented — pointed-to.

### 1.5 The hub-as-organism + mesh topology — `kernel/src/hydra.rs`
- **`Hydra<S>`** wraps an `EventLog<S>` plus its **local topology** (`nodes`, `base_edges:Vec<TopoEdge>`)
  and an `OrganismState::{Live | Locked}` (`hydra.rs:75-141`). `topology_adjacency(nodes, edges)` builds
  the n×n adjacency (`hydra.rs:41-49`) — **this is the MESH-tier graph source.**
- `commit()` is the single closed-loop surface: integrity-check → **drift gate (`candidate_drift`,
  scores the mutation against the live spectral baseline)** → `decide` (`hydra.rs:185-215`).
  `boot_verify()` asserts ρ<1 on the base topology (`hydra.rs:223-235`).
- **`BreachAlert { node_id, group_size }`** (`hydra.rs:87-119`) is broadcast to the whole opted-in
  consensus hub, **ML-DSA-signed via mesh transport**, self-witnessed into the WORM log
  (`raise_breach_alarm`, `hydra.rs:257+`) — a distinct MESH-tier alarm signal that "cannot be forged,
  hidden or masked".

### 1.6 The already-exposed compute surface — `kernel/src/wasm.rs`
The spectral + FSM engine is **already exposed to the wasm/JSON boundary**: `spectral_eigenvalues_js`,
`spectral_radius_js`, `spectral_gap_js`, `spectral_algebraic_connectivity_js`,
`spectral_classify_drift_js`, `spectral_flat_js`, `fsm_graph_report_js`, `harmonic_centrality_js`,
plus `place_order_js` / `apply_event_js` / `channel_ledger_js` / `boot_verify_fsm_js`
(`wasm.rs`, verified). For the **hybrid split** the server runs the kernel **natively** (not wasm) and
streams; this wasm surface is the fallback/parity path and proof that the numbers are already
computable at the boundary.

---

## 2. The 3-TIER data model (MESH / HUB / NODE) — architect now, ship HUB first

**Core design decision: all three tiers are the SAME rendering primitive over a DIFFERENT graph.** One
`Graph → Laplacian L → { spectral-embedding = positions, PPR-diffusion = activity, graph_spectrum =
health }` pipeline; the tiers differ only in *which graph* seeds `L` and *what a node means*. This is
the strongest possible form of the operator's "the interface is a literal rendering of the backend":
zooming does not switch renderers, it re-seeds the operator.

| Tier | Nodes = | Edges = | Graph source (kernel) | A "signal" = | Canon anchor |
|---|---|---|---|---|---|
| **MESH** | hubs (Hydra heads) | mesh links | `hydra::topology_adjacency(nodes, base_edges)` | a `SignedFrame` gossip/sync frame; a `BreachAlert`; a hub join/leave | F48, M7, M4 |
| **HUB** | memory records (`SpineRecord` Memory/Identity/Intent) + the running-binary zones they belong to | relatedness edges (`retrieval` wikilink/co-citation graph) | `Csr` over `retrieval` spine + `diffusion::WIKI_EDGES` | a `MeshEvent` commit; a `decide()` Law firing; a `personalized_pagerank` recall wave; a `classify_drift` health change | F14, F47 |
| **NODE** | ONE record exploded: its `payload_hash`, its event-chain (`event_log` `prev`-linked `MeshEvent`s), its 1–2-hop neighbours | the record's own dendrites (edges to related records) + its firing history (the hash chain) | ego-graph of one `SpineRecord` + its `event_log` sub-chain | one committed intent's full lifecycle; `verify_chain` healthy/tampered | (detail inspector) |

### 2.1 Zones (the operator's "divided by containers/binaries and their functional purpose")
The HUB cluster is spatially partitioned into **zones = functional subsystems**, mapped to the real
running binaries/modules:

- **INTAKE / ORDER zone** = `order_machine.rs` + `cart.rs` + `intake.rs` — the FSM lifecycle an order
  flows through. **A full order lifecycle (Pending→…→Delivered) is a signal that traverses this zone
  and ends in a distinct "completion bloom" + resolving sound** (§4.4).
- **MEMORY zone** = `retrieval/` (spine, memory_store, bm25, trigram, diffusion, recall) — where recall
  diffusion waves spread (PPR).
- **EVENT / LEDGER zone** = `event_log.rs` + `spine.rs` — the append-only hash chain; committed intents
  accrete here as a visibly growing spine.
- **MESH zone** = `hydra.rs` + mesh transport (SignedFrame, proto-cap) — where cross-hub signals travel
  and `BreachAlert`s alarm.
- **SPECTRAL / HEALTH field** = `spectral.rs` — not a spatial zone but a **global tint** over the whole
  cluster from `classify_drift`: `Damped` = cool/stable, `Resonant` = amber pulsing, `Unstable` =
  red-alarm.
- Coarse binary zones (for the MESH/ops view): kernel (native), wgpu engine (client), bebop2 mesh node,
  pgrust (event-store durability), **deep-clean cron** (a periodic sweep that demotes stale memory nodes
  — rendered as a "tide going out": low-salience nodes dim, never delete, matching the memory
  "demote-never-delete / TTL=demote" rule from project memory).

### 2.2 Signal taxonomy (each has BOTH a visual marker AND a sound)
A "signal/event/state-change" is not one thing — it is **differentiated by type**, each with its own
marker + sound (per the operator's ask). Grounded in real kernel events:

| `signal_type` | Kernel origin | Visual marker | Sound (see §4.4) |
|---|---|---|---|
| `Event` | a `MeshEvent` committed to `event_log` | a new lit node + a glyph on the growing spine | a discrete grain (pitch by zone) |
| `Decide` | a `decide()` Law evaluation / drift-gate pass | a synapse flash between intent-node and the ORDER zone | a short "click"/attack |
| `Recall` | a `personalized_pagerank` diffusion from a seed | a wave of activation spreading over MEMORY-zone edges | a swelling drone (density ∝ mass) |
| `Gossip` | a `SignedFrame` sync/pull frame (cross-hub) | a pulse traveling a MESH edge | a spatialized (`PannerNode`) blip at the peer's position |
| `Drift` | a `classify_drift` class change | the global health tint shifts | a filter sweep / detune |
| `CycleComplete` | an order reaching a terminal FSM state | a bright collapse/bloom at the ORDER zone | a resolving chord |

**Propagation & interaction** (operator's "propagation depends on signal movement and interaction with
other signals"): a signal enters at a node, `decide()` fires (synapse), the commit grows the spine, a
`Recall` diffusion may light related nodes, and a cross-hub intent emits a `Gossip` frame down a MESH
edge. When two diffusion waves overlap, the render composes them via the **coherence `|ψ₁±ψ₂|²`
interference** already specced as `physics-ui-capture §1` Tier-2 (gated) — this is the literal
"interaction of signals".

---

## 3. Server / client protocol — what is computed where, wire format, LOD

### 3.1 The split (per the operator's HYBRID ruling)
- **SERVER (GPU-less Hetzner VPS — V2: kernel GPU offline/port-only) computes STATE:**
  1. **Layout** — spectral embedding of the tier's graph Laplacian → 3-D positions (the FE-12 /
     RW-01 `field-math` eigenvector helper, §1.4). *Expensive, low-cadence.*
  2. **Activity** — `personalized_pagerank` salience + the ordered `event_log` signal feed. *Cheap,
     high-cadence.*
  3. **Health** — `graph_spectrum` (ρ, |λ₂|, fiedler, energy, drift). *Cheap.*
- **CLIENT composites & animates** (see §4): particle render, glow/bloom, camera, event-driven pulses,
  audio synthesis. **The client invents no state** — it only interpolates presentation toward
  server-authoritative positions/activity (§5, F-7).

This is a stronger "interface = rendering of the backend" than pure-client-compute: **positions and
activity are genuine backend numeric state** (deterministic kernel output), not a rendering artifact.

### 3.2 Which layout algorithm — spectral embedding, NOT GraphWaGu force-directed
The external research offers two options: GraphWaGu-style GPU force-directed layout, or a
spectral/eigenmode approach. **Choose the spectral embedding** because it *is* the dowiz Laplacian
thesis: the same `L=D−A` that already drives recall/decay/blur (`csr::laplacian_spmv`,
`physics-ui-capture §1`) provides the positions (its low eigenvectors φ₂,φ₃,φ₄ → x,y,z; **FE-12**).
Benefits that force-directed lacks: **deterministic** (fixed seed → byte-identical layout, satisfying
the repo's VERIFIED-BY-MATH rule and matching PPR's bit-reproducibility), **no random init / no
iteration-count nondeterminism**, and it reuses one operator. Force-directed **stress-majorization
(FE-07 SMACOF)** is kept but only as the **incremental warm-start relaxation** for single-node topology
changes (§5, F-2) — spectral for cold layout, SMACOF for local repair. GraphWaGu's ceilings
(100k nodes / 2M edges @ ≥10 fps) inform the LOD budget (§3.4), not the algorithm choice.

### 3.3 Wire format — dual epoch-versioned streams
**Do NOT send full state every frame.** Two streams, decoupled by cadence, joined by a monotonic
`epoch`:

**(A) `LayoutKeyframe`** — low frequency (~0.1–1 Hz, or on topology change):
```
LayoutKeyframe {
  tier:      u8,          // Mesh | Hub | Node
  epoch:     u64,         // monotonic; activity deltas pin to this
  graph_id:  [u8;32],     // content-addressed hash of the (node,edge) set — detects graph identity
  lod:       u8,          // which decimation level this keyframe represents
  nodes:  [ NodeLayout { id:u32, pos:[f32;3], zone:u8, node_kind:u8, radius:f32 } ],
  edges:  [ EdgeLayout { src:u32, dst:u32, weight:f32 } ],   // decimated per LOD
  spectral: { rho:f32, slem:f32, fiedler:f32, energy:f32, drift:u8 },   // graph_spectrum
}
```
`node_kind` = `RecordKind` (Memory/Identity/Intent) at HUB, hub-type at MESH. `spectral` ships the
`graph_spectrum` signature so the client can render health without recomputing.

**(B) `ActivityDelta`** — high frequency (coalesced up to render rate):
```
ActivityDelta {
  epoch:    u64,          // MUST equal the client's current layout epoch
  t_logical:u64,          // ordering (actor_seq-derived) for presentation scheduling
  signals: [ Signal {
     node:        u32,         // index into the epoch's layout
     event_id:    [u8;32],     // == event_log content-id → idempotent dedup (F-1)
     signal_type: u8,          // Event|Decide|Recall|Gossip|Drift|CycleComplete
     kind:        u8,          // order|error|notification|success|loading (FE-10 vocab)
     energy:      f32,         // intensity → glow + audio amplitude
     edge_path:   [u32]?,      // a traveling signal's node path (for pulse animation)
  }],
  salience:  [(u32,f32)]?,     // OPTIONAL sparse PPR salience update, coalesced
}
```
**Key property:** `signal.event_id` is the `event_log` content-id, so the client **dedups exactly as
`MemEventStore::contains` does** — a dropped/duplicated/reordered delta is a structural no-op. Missing
`actor_seq` gaps are detectable → request backfill (mirrors mesh pull-anti-entropy).

**Transport split (grounded in M10 "inter-hub protocol defined; intra-hub anarchy"):**
- **MESH-tier** frames cross hub boundaries → ride the **signed mesh transport**: `SignedFrame` +
  `proto-cap` capability, **ML-DSA-signed** (M4 self-signing edges, M12 proto-cap), **opportunistic**
  per F48. Low frequency, so per-frame signing is affordable. Reuse verbatim (MESH-REAL-PLAN: "*Reuse
  UNMODIFIED … bebop2 {SignedFrame, Capability}*").
- **HUB / NODE tiers** are same-hub, local (client and server are the *same* hub) → a **lightweight
  unsigned local delta protocol** over the localhost/in-process boundary. The trust boundary is the
  **subscribe-time capability check** (§5, F-4), not per-frame signatures — signing every 60 fps frame
  with ML-DSA is infeasible and unnecessary intra-hub (M5: "intra-hub = hub's own business").

### 3.4 LOD / decimation (when the hub graph is too large to stream/render in full)
- **Level 0 (full):** all nodes+edges. Feasible while `n` ≲ a few thousand. **Real ceiling:** the kernel
  eigen-engine is O(n³) Householder for `n≤32`, O(n⁴) Faddeev-LeVerrier for `n>32` (`spectral.rs:195-214`)
  — dense full-spectrum is only viable for small graphs. For larger graphs the layout must use the
  **iterative Lanczos / power-iteration** path FE-12 names ("*Lanczos for large*") or **spectral
  coarsening**, computing only the few low eigenvectors needed for 3 coordinates.
- **Levels 1..k:** **spectral coarsening** — contract low-Fiedler edges into super-nodes (the Fiedler
  vector gives a principled partition); OR **top-K-by-PPR-salience**. Either way positions are computed
  **once on the full/coarsened graph and the visible subset is projected** — LOD changes *which* nodes
  are drawn, never *where* they are (so LOD transitions don't jump).
- **Stability (critical):** node visibility uses **hysteresis** (reuse FE-14's K=3 hysteresis): a node
  enters the visible set at `salience > θ_high`, leaves at `< θ_low` (`θ_low < θ_high`) → no
  pop-in/pop-out flicker across the K boundary (§5, F-6).
- Client picks LOD from viewport + device budget (FE-16 30 fps floor; external-research ceilings: sim
  10⁴–10⁵, render 10⁵–10⁶ particles). **Decouple sim from render**: one server "neuron" can drive many
  client trail particles (external research §Scaling).

---

## 4. Client rendering — built on FE-01…17 + wgpu, on the Laplacian thesis

Stack ruling (house rule, NOT the external research's Three.js/TSL default): **`wgpu` + WGSL is the sole
graphics dependency** (`physics-ui-capture §3`, RW-04/05/10), **WebGPU-primary + WebGL2 fallback via
naga** (FE-16, RW-04 "*WebGL2 cross-compile*"), **degrade to static DOM** when neither is present
(RW-11). Reuse the external research's *math and pipeline shape* (Izhikevich, spatial hashing, bloom,
AudioWorklet+Faust), reject its JS-framework recommendation.

### 4.1 Reuse the existing FE machinery (don't re-design)
- **Positions store:** `store::ParticlePool` SoA ring, **FE-02 / RW-04** (`pos_x/y`, `vel_x/y`, `life`,
  `max_life`, `color[f32;4]`, MAX=4096). *Extension needed:* widen `pos` to 3-D (`pos_z`) — the pool is
  2-D today; flag as a scoped delta to RW-04.
- **WASM↔GPU handoff:** **FE-01 / RW-05** zero-copy — Rust writes flat instance data into linear memory,
  JS wraps `Float32Array(memory.buffer, ptr, len)` and does **one `queue.writeBuffer`, zero JSON, zero
  parse** in the frame loop (Appendix-B invariant #4: "*Ніколи JSON у frame-loop*"). The `<10`-export
  `shell` crate (RW-05) is the single `wasm-bindgen` boundary.
- **Somata (neuron bodies):** instanced emissive billboard quads — **RW-04** ("*WGSL: point sprite →
  instanced billboard quad … additive SrcAlpha,One*"), size `= 2 + life·6·energy`, **HDR emissive
  color >1.0** (external research §2). Colour from `zone` (T2 palette) × `energy`. **RW-04's
  blue-hardwire bug must be fixed** (widen meta → full RGBA) — this feature *depends on* that fix.
- **Synapses (edges):** SDF capsule/fat-line tubes (**FE-05** `sdRoundBox`/`sdSegment` + analytic
  `fwidth` AA, no MSAA); a **traveling signal pulse** = a moving `smoothstep` window of emissive
  intensity along the edge (UV offset with time), driven by `signal.edge_path` (external research §2).
- **Motion / layout easing:** **FE-08** per-property **critically-damped ζ=1 spring** eases each node
  from its old→new streamed position — monotone, **no overshoot** → client presentation provably
  converges to server state and never diverges (§5, F-7). **FE-03** fixed-dt accumulator (`DT=0.02`),
  **FE-14** lazy-render-on-settle (dormant rAF when nothing pending → battery).
- **Feedback vocab:** **FE-10** Green's-function `U(x,t)=∫∫G·S` — each signal = a source impulse; the
  event→source vocab (tap/success/order/error/loading; existing `order/delivered/dispatch_failed`
  bursts) maps directly onto the `signal.kind` field. Particles become **field tracers advected by U̇**.
- **Text (NODE tier):** **FE-06** MSDF via cosmic-text (Latin/Cyrillic/icons) for the record inspector.

### 4.2 Net-new client pieces (small, additive)
- **Selective bloom pass** — the "cinematic neuron glow" is NOT in FE-* today (FE has additive blending
  only, no bloom). Add a screen-space post pass: threshold bright emissive → multi-mip gaussian blur →
  composite → AgX/ACES tone-map → dark fog (external research §2). **Grounded reuse:** a gaussian blur
  *is* the heat kernel `e^{−tL}` (`physics-ui-capture §1`, "blur ≡ heat eq") — bloom is the same
  operator family, here a screen-space image blur, so it is philosophically consistent, just a new
  post-pass.
- **3-D camera + drill-down** (see §4.3).
- **Audio subsystem** (see §4.4).

### 4.3 The tier transition (zoom)
Same primitive → the zoom is **camera dolly + graph swap**, animated as a Sea-layer **dive /
sheet-rise** (Sea & Sheet `DZ-03`): the parent graph recedes/fades, the child graph rises. MESH→HUB =
subscribe to the target hub's HUB graph; HUB→NODE = subscribe to the record's ego-graph. Because the
primitive is scale-invariant, the transition reads as *continuously zooming into a neuron until its
internal structure resolves* — a hub is a neuron in the mesh; a record is a neuron in the hub
(fractal self-similarity). No renderer switch; only a new `LayoutKeyframe`.

### 4.4 Sound (net-new subsystem — design)
Per **RW-09**, "Audio" is an enumerated **irreducible-JS membrane** shim — so the **`AudioWorklet`
plumbing is thin JS**, but the **DSP is WASM** (Faust→WASM, or Rust→wasm, external research §5). The
audio is driven by the **same server `ActivityDelta` stream** as the visuals (not a client compute
readback), which **eliminates the external research's #1 sync risk** (async `mapAsync` readback
latency) — audio and video share one ordered source. Mapping (external research §5):
- `signal_type`/`kind` → grain/note selection; **zone → pitch class** (pentatonic to avoid dissonance)
  and **`PannerNode` 3-D position** matching the neuron's location.
- `energy` → amplitude/density; `Recall` wave density → drone; `Drift` class → global filter/detune;
  `CycleComplete` → a resolving chord.
- 🔴 **RED-LINE (extension of FE-09 money-never-tweens):** a `CycleComplete` for an order sonifies the
  **event of completion, NEVER the monetary magnitude** — money must not become a pitch/amplitude
  channel any more than it may tween visually (FE-09/FE-17, Sea & Sheet "money→SHEET NEVER MOVES"). The
  money value, if shown, is a **static integer on the Sheet layer** (§5, F-3).

---

## 5. FRICTION / JOINT MAP  ← most-valued output

Every joint below is grounded in a specific code path or canon anchor, with the concrete failure and a
concrete resolution. Ordered by consequence.

### F-1 — Staleness: server layout vs client render desync (operator's joint **a**) — THE load-bearing one
**The joint.** The server computes two things at wildly different cost: **layout** (spectral eigensolve
— O(n³)–O(n⁴), `spectral.rs:195-214`, slow) and **activity** (event feed + PPR — cheap, fast). If both
ride one stream at one cadence, layout latency throttles activity, or a fast graph mutation outruns the
layout the client is drawing on.
**Failure if ignored.** Activity deltas reference node indices that no longer mean what the client's
layout thinks (index reuse after a mutation) → signals light the wrong neurons; or the client stalls
waiting for a keyframe.
**Consistency model (resolution).** **Dual epoch-versioned streams (§3.3):** slow `LayoutKeyframe`
(epoch-stamped, `graph_id` content-addressed) + fast `ActivityDelta` **pinned to a layout epoch**.
Rules: (1) an activity delta whose `epoch` ≠ the client's current layout epoch is **buffered, never
applied to the wrong layout**; (2) a delta referencing a node absent from the current epoch (added since
the last keyframe) is **held in a pending buffer or spawned at its neighbours' centroid — never
dropped**; (3) **idempotent dedup by `event_id`** (the `event_log` content-id, `event_log.rs:146-155`)
makes reorders/duplicates structural no-ops — *the viz inherits the log's own idempotency for free*; (4)
`actor_seq` gaps are detectable → pull-anti-entropy backfill (MESH-REAL sync). **Model = eventual
consistency with monotonic epochs; the event substrate's existing content-addressing IS the
correctness guarantee.** This joint dictates the entire protocol shape.

### F-2 — Topology change mid-render (operator's joint **b**)
**HUB sub-case.** The spectral embedding is **global** — adding one record perturbs *all* eigenvectors
→ a naïve re-layout makes **every neuron jump** (jarring).
**Resolution.** Split cold vs incremental: **spectral embedding only for cold-start / large topology
change**; for a single node add/remove, **pin existing nodes and run one warm-started
stress-majorization (FE-07 SMACOF) relaxation** seeded at the new node's neighbours' centroid — SMACOF
is monotone-non-increasing (Lyapunov, FE-07) so existing nodes drift minimally. **Additional built-in
guard:** the **drift gate already rejects `Unstable` (ρ>1) mutations pre-persist**
(`event_log.rs:347-375`, `hydra.rs` `candidate_drift`) — so in the default regime the client **never has
to render a divergent graph**; the topology it sees is always Damped/Resonant (bounded). The same
`classify_drift` that gates commits bounds what the layout must absorb.
**MESH sub-case.** A hub joins/leaves (M7 healing, "*no leader election required*"). Per **F48** the
view is the *viewing hub's own* picture, synced opportunistically — a hub drop is **not** a global
re-layout; the departing node's activity **fades to 0 (demote-never-delete)**. A **partition** (F15,
"both sides think they're root, merge via HRW") is renderable *directly* because the server's
`graph_spectrum` already carries the signal: **`fiedler λ₂(L) → 0 ⟺ disconnected`**
(`spectral.rs:302-311`) — the client tints/splits the cluster when the streamed `fiedler` crosses ~0.
No special partition protocol needed; the spectral signature already encodes it.

### F-3 — The state/render brand boundary (operator's joint **c**)
**The joint.** Where does design-token / branding data cross the server→client line?
**Resolution (Sea & Sheet, definitive).** The viz is a **Sea artifact — "dowiz-owned brand-TINTED never
brand-authored"** (RESEARCH-CONSPECT LAYER A). Therefore: **STATE crosses the wire; STYLE stays
client-local.** The server streams **brand-neutral** state (positions, activity, connectivity, spectral
signature, signal `kind` tags). The **brand tint is applied client-side** from the client's
**locally-hydrated token cascade** — T2 `--sea-tint = brand-primary`, the `--spectral` edge re-derived
per brand (TOKEN 3 TIERS). Sea & Sheet is explicit: **"server = sync target, NEVER render dependency"**
and theme data is "*hydrated boot*". The server must **not** inject brand color.
**Why violating this bites:** (1) injecting brand couples the wire to tenant identity (leaks which brand
is active — privacy) and forces per-frame style churn; (2) a **MESH** view of N hubs would have to
reconcile N brands — but per F48 the viewing hub renders *others* in **its own** neutral/spectral
palette, not each peer's brand. So the rule is architectural, not cosmetic. **Money exception:** the
`kind` tag crosses as a semantic enum (client maps `kind→color` via its local FE-04/FE-10 vocab); a
money value, if surfaced at NODE tier, is a **static integer on the Sheet layer** (FE-09/FE-17), never a
particle magnitude or a wire-animated field value.

### F-4 — Per-role access control, fail-closed at the server (operator's joint **d**)
**Decided:** access is per-role operator-configurable (M9: "*access controls are per-hub configurable,
not global*"). Enforcement must be **server-side, fail-closed — never client-side hiding.**
**Resolution.** Gate the stream behind a **`proto-cap` `Capability{Resource::LivingMemoryView, scope}`**
token (M12: ML-DSA-signed, fail-closed, nonce-replay, expiry, RevocationSet, per-agent scope; F19:
"*Hub rejects a frame with unknown capability scope — fail-closed*"). The server **verifies signature +
scope + expiry on subscribe and re-verifies at session boundary** (the G4/G5 re-verify the
`EventStore::get` seam exists for). **Fail-closed = explicit reject, not an empty stream** (mirrors
`living_knowledge.rs` "*fail-closed: any … error returns Err — retrieval never silently degrades to
empty results*").
**Concrete leak vectors handled:** (1) **NODE-tier PII** — the *spine* is PII-free (only `payload_hash`,
`spine.rs`), but `retrieval/memory_store.rs` holds real content, so NODE drill-down must apply the same
scope and **redact PII/money payloads server-side** (via the existing `apps/api/.../anonymizer`
boundary) before streaming. (2) **Signature leakage** — even the `graph_spectrum` reveals how active a
hub is; a low-scope viewer gets only coarse per-hub health (up/down/partitioned), full spectral detail
only at owner scope. **Default = minimum scope.**
🔴 **REAL EXISTING GAP (flagged, not solved here):** MESH-REAL-PLAN records "*revocation НЕ існує (лише
expiry)*" — the `RevocationSet` is specced (M12) but **not built**; only expiry is checked. A per-role
viz capability that is granted then revoked **keeps streaming until it expires**. **Mitigation until
revocation lands:** issue viz capabilities **short-lived** and **re-verify at every session boundary
(G5)**; wire the `RevocationSet` check the moment it exists. This is the single most security-relevant
finding.

### F-5 — Audio-visual sync + presentation scheduling (net-new joint)
**The joint.** A visual pulse travels a synapse over ~hundreds of ms; an audio grain wants to fire on
event receipt. Misaligned → the "boom" precedes the "flash."
**Resolution.** Because audio and video share **one ordered server stream** (not a client GPU readback),
they are synced at the source — this already beats the external research's async-readback risk. Residual
fix: the client schedules **both** the visual pulse peak and the audio grain off the **same
`t_logical`** + a small fixed **presentation lookahead buffer** (external research §5 "sonify every N
frames"), so both peak together. Money magnitude is never a sonic parameter (F-3 / §4.4 red-line).

### F-6 — LOD decimation flicker (net-new joint)
**The joint.** Top-K-by-activity decimation makes the visible node set flicker as scores cross the K
boundary each frame.
**Resolution.** **Hysteresis** (reuse FE-14 K=3): enter at `θ_high`, leave at `θ_low`. Positions
computed once on the full/coarsened graph and projected (§3.4) so LOD level changes never move a node.
Decimation is temporally coherent by construction.

### F-7 — Client presentation vs server state divergence (net-new joint)
**The joint.** If the client integrator invents motion, it can drift from server-authoritative state.
**Resolution (clean invariant).** **Server = state, client = presentation.** The client integrator only
eases old→new *streamed* positions via **FE-08 ζ=1 critical damping (monotone, no overshoot)** — it can
never overshoot or invent a position the server didn't send. FE-16's scalar==SIMD bit-identical rule +
FE-03 fixed-dt keep even the presentation reproducible. State always originates in the deterministic
kernel (PPR bit-reproducible `csr.rs:494-508`, spectral fixed-seed `spectral.rs:141-186`).

### F-8 — Over-signing the local stream (net-new joint)
**The joint.** Applying the MESH `SignedFrame` + ML-DSA path to the 60 fps HUB/NODE stream is
infeasible (per-frame post-quantum signatures) and wrong-layer.
**Resolution.** Protocol split per M10 (§3.3): **sign only cross-hub MESH frames** (low-frequency,
opportunistic, F48); HUB/NODE is intra-hub local, trust established once at subscribe-time (F-4), no
per-frame signing (M5 "intra-hub = hub's own business").

### F-9 — The layout eigensolver ceiling (net-new joint, honest limit)
**The joint.** The kernel's dense eigen-engine is O(n³)–O(n⁴) (`spectral.rs`), so full-spectrum layout
does not scale past small graphs; and **no eigenvector primitive exists yet** (§1.4).
**Resolution / honest status.** Phase-0 targets small graphs (the 20-node fixture / early live spine) so
the dense path suffices. Beyond that, layout needs the **Lanczos / iterative low-eigenvector path
(FE-12)** and the **`field-math` vendored eigensolver (RW-01, bebop2 `field.rs`)** — a real,
already-scoped engineering line-item, not a hidden cost. Flag it explicitly rather than assume it.

---

## 6. Phase-0 scope recommendation (smallest real, falsifiable slice)

**Goal:** prove the HUB-level view works end-to-end, with the 3-tier model present in the *data/protocol
layer* from day one (per operator), shipping only the HUB UI.

**Scope (build):**
- **Tier:** HUB only, **current/local hub**, **read-only**, **owner-scope only** (F-4 fail-closed).
- **Data source:** the **already-in-kernel `retrieval/diffusion.rs` fixture** — 20 nodes, 41 edges,
  deterministic, hand-verified, `{5,6,12,16}` unreachable. (Then swap to the live retrieval spine +
  `event_log` behind the same wire.) Using the fixture first makes every claim falsifiable against
  existing tests.
- **Server computes & streams:** (1) spectral-embedding positions — **the one net-new kernel primitive:
  a Laplacian eigenvector→3-D-coords helper (FE-12 / RW-01)**; (2) `graph_spectrum` signature (exists);
  (3) `personalized_pagerank` activity from a seed (exists, `csr.rs`/`ppr.rs`). Emits one
  `LayoutKeyframe` + `ActivityDelta`s over the **local unsigned delta protocol** (§3.3B).
- **Client renders:** 3-D SoA particle cluster (extend RW-04 pool to `pos_z`) at streamed positions,
  emissive billboards coloured by zone (node-label prefix → zone), glow from PPR salience + **one net-new
  selective bloom pass**, **one signal type** (`Recall` diffusion wave replayed from the seed) with a
  matching sound (one AudioWorklet grain per node activation). WebGPU-primary, WebGL2 fallback (FE-16),
  degrade-to-DOM (RW-11). Client-local brand tint from hydrated tokens (F-3).

**Falsifiable proof (VERIFIED-BY-MATH — the repo's ship rule):**
1. **Layout determinism:** same graph ⇒ byte-identical positions (fixed spectral seed) — assert
   reproduction across two runs (parallels `diffusion.rs` `green_ppr_byte_identical_two_runs`).
2. **Activity ⟺ kernel truth:** the lit-node set at step k = nodes with PPR > 0 = exactly
   `diffusion::reachable_from(SEED)`; the **4 unreachable nodes `{5,6,12,16}` must render DARK**
   (score exactly 0, `diffusion.rs:36-37,228-234`) — a directly falsifiable visual assertion bound to an
   existing test.
3. **recall@5 rendered:** seeding a query lights the top-5, containing the relevant node
   (ties to `recall_at_k == 1.0`, `living_knowledge.rs:287-322`).
4. **A/V sync:** each lit node fires exactly one grain → grain count == lit-node count.
5. **Fail-closed access:** no owner capability ⇒ explicit stream reject (Err), not empty.

**3-tier readiness (no UI, but wired):** the `LayoutKeyframe`/`ActivityDelta` format carries
`tier` + `epoch` + `graph_id` from day one. Adding MESH later = a new server graph-builder over
`hydra::topology_adjacency` (+ SignedFrame transport); adding NODE later = a new builder over one
`SpineRecord`'s ego-graph + `event_log` sub-chain. **No wire-format change** — only new server-side
graph sources. The protocol is 3-tier from the first commit; only the HUB builder ships in Phase-0.

**Explicit non-goals for Phase-0 (avoid gold-plating):** MESH & NODE UIs; cross-hub SignedFrame
transport; live coherence `|ψ₁±ψ₂|²` interference (Tier-2, gated); AR/native targets (DZ-12); the large-
graph Lanczos path (F-9); full multi-signal-type sonification. All are protocol-reachable extensions.

---

### Source ledger (precise)
Kernel (verified live): `event_log.rs` (MeshEvent/hash-chain/drift-gate/EventStore), `spine.rs`
(SpineRecord/RecordKind/verify_chain), `csr.rs` (laplacian_spmv/personalized_pagerank/recall_at_k),
`spectral.rs` (graph_spectrum/classify_drift/algebraic_connectivity; eigen*values* only — no embedding),
`hydra.rs` (topology_adjacency/candidate_drift/BreachAlert/OrganismState), `retrieval/{mod,ppr,diffusion,
recall}.rs` (20-node/41-edge fixture, PPR, recall@5=1.0), `wasm.rs` (spectral_*_js/fsm_graph_report_js).
Canon: ARCHITECTURE.md M4/M5/M6/M7/M9/M10/M11/M12, SCOPE RULE, V2, F14/F15/F19/F40/F47/F48, E52/E54.
Design corpus: physics-ui-capture-blueprint §1–§3 (ONE-L unification, wgpu sole dep, blur≡heat-kernel);
field-ui BLUEPRINTS FE-01/02/03/04/05/06/07/08/09/10/12/14/16/17 + Appendix-B invariants #2/#4/#7;
rust-engine-rewrite RW-01/04/05/09/10/11; dowiz-interfaces RESEARCH-CONSPECT (Sea & Sheet LAYER A/B,
SEAM=spectral, TOKEN 3 TIERS, "server=sync-target-never-render-dependency"); DZ-03. Mesh: MESH-REAL-PLAN
(SignedFrame/proto-cap/pull-anti-entropy; "revocation НЕ існує лише expiry"). External research
(operator-supplied, filtered): Izhikevich model, spatial hashing, selective bloom + AgX + fog,
GraphWaGu ceilings, AudioWorklet+Faust-WASM, async-readback sync caveat, particle/sim scaling numbers.
```
