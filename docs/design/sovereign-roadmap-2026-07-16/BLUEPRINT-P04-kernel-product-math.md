# BLUEPRINT P04 — KERNEL PRODUCT-MATH PRIMITIVES (2026-07-16)

> **Phase 4 of 19** (R2-MERGED-PHASE-ROADMAP §2). Anchors: **F45** (route Dijkstra/A*),
> **E61** (kernel math-first — substantively BUILT, confirmed here as the exemplar).
> Depends on: — (Wave 0). Parallel-safe with Phases 1, 2, 3, 5.
> **This is THE single implementation of the M7 / E32 / E33 / F45 / F46 graph-math.**
> R2 §1 "Major merges" is explicit: the math primitives land **once, here**, and are consumed
> by Phase 9 (mesh heal), Phase 13 (product route F45 + partition-tolerant delivery F46), and
> Phase 16/17 (UI/demo). They do **not** get their own copies. Build one zero-dependency library.
>
> Planning document only — **no kernel code is written or edited by this blueprint.**

---

## 1. Current-state evidence (file:line)

**Router — absent from the kernel, present stranded in a legacy crate.**
- `kernel/src/geo.rs` computes no route. It has haversine (`:15`), bearing (`:30`, 0=N clockwise),
  EMA (`:39`), polyline projection `progress_along_route` (`:70`), ETA `eta_seconds` (`:153`),
  and even-odd `point_in_polygon` (`:200`). It projects a position onto an **already-given**
  polyline; it never computes one.
- A real, working, zero-external-dep router exists at
  **`/root/bebop-repo/crates/bebop/src/cost_estimate.rs`**: `route()` (`:209`) is a binary-heap
  Dijkstra with an admissible Euclidean A* heuristic `h()` (`:245`, measuring `i→dst`), CH
  shortcut preprocessing `build_shortcuts()` (`:157`), BFS reachability guard `reachable()`
  (`:123`), weighted-adjacency builder `weighted_adj()` (`:79`), and the `hybrid_route()`
  pipeline (`:305`). Its only crate coupling is `serde` derive on `EdgeCost` (`:56`) and the
  bebop-local types `Node2D{id,x,y,red_line}` / `ConnEdge{from,to,kind,weight}` and
  `field_physics::adjacency`. It is `Vec<Vec<usize>>`-adjacency based, **not** CSR.

**DSU / MST — absent everywhere** (dowiz + bebop2, grep zero for dijkstra/DSU/MST/Kruskal).
The parity-swap target exists: `kernel/src/cgraph.rs:171-205` `c_components()` is a stack-based
flood-fill of the **bidirected** subgraph, restricted to `present` nodes, each component sorted
ascending, components emitted in ascending-minimum-node order (the `0..n` first-unseen scan).
Regression fixtures already exist: `chain_graph()` → 3 singletons
(`green_dag_c_components_are_singletons`, `:558`) and `m_graph()` bow-arc → `{0,1},{2}`
(`green_bow_arc_groups_confounded_pair`, `:569`).

**CSR container — present and deterministic.** `kernel/src/csr.rs` `Csr{row_ptr,col_idx,val}`
with `from_edges(n, &[(usize,usize,f64)])` (`:79`, directed; undirected = pass both directions),
`to_adjacency()` (`:190`). This is the natural road-graph container; the router must consume it.

**Six GS geo functions — specified, not built.** `storey_height_m`, `floor_slice_height_m`,
`arrow_screen_rotation_deg`, `angular_diff_deg`, `in_field_of_view`, `los_clear` — signatures
and math fixed in `GAUSSIAN-SPLATTING-…-SYNTHESIS-2026-07-16.md §2.6`, acceptance list in §4 P0.1.
Not re-derived here.

**`compose` — CORRECTION to the inherited gap.** The roadmap/R1-D gap "`field_frame::compose`
not wasm-exported, physics can't reach a canvas" is **stale/mis-scoped**. It is true only of
`kernel/src/wasm.rs` (the kernel cannot depend on the engine — the dependency runs
engine→kernel). Against the **full tree**, `compose` (`engine/src/field_frame.rs:189`) is
**already exported**: `dowiz-wasm/src/lib.rs:57` `compose_field()` and the stateful `FieldSim`
(`:64`, constructor/`step`/`frame`), each with a **GREEN host-level determinism test**
(`wasm_compose_deterministic` `:204`, `wasm_fieldsim_deterministic` `:288` — both
`assert_eq!` bit-identical frames). `dowiz-wasm` depends on both engine and kernel and pins
`wasm-bindgen =0.2.95` to the offline CLI. The residual gap is narrower than "export compose":
(a) actual wasm-bindgen glue generation + a **real browser-canvas** smoke (documented ceiling,
`wasm/src/lib.rs:172`), and (b) the new router/DSU/geo functions are exported **nowhere** yet.

---

## 2. Router port plan (`cost_estimate.rs` → kernel)

**Module location.** New `kernel/src/router.rs`, sibling to `csr.rs`/`geo.rs`, registered in
`kernel/src/lib.rs`. Pure `std`, zero external deps — drop the `serde` derive on the cost struct
(the kernel gates serde behind the `wasm` feature; the router core must compile without it).
Drop the bebop `Node2D`/`ConnEdge`/`field_physics` couplings entirely.

**Interface with `csr.rs`.** The graph is a `Csr` built via `Csr::from_edges` from the ingestion
port (below). The router traverses CSR rows directly — no `Vec<Vec<usize>>` rebuild: for node `u`,
neighbours are `col_idx[row_ptr[u]..row_ptr[u+1]]` with weights `val[..]` (mirrors
`csr.rs:178`). This removes `weighted_adj()`/`adjacency()` and their allocations.

**Node coordinates for the heuristic.** Road-graph nodes carry `(lat,lng)`. The port passes a
parallel `coords: &[(f64,f64)]` (index-aligned with CSR rows). **Admissibility contract (the one
load-bearing adaptation):** `cost_estimate.rs:245` uses raw Euclidean `√(dx²+dy²)` over `x/y`.
For a geo graph with **metre** edge weights, the heuristic must be a *lower bound in the same
units* — use `geo::haversine_meters(coords[i], coords[dst])` (great-circle ≤ any road path).
A raw degree-space Euclidean number would be inadmissible and silently return sub-optimal paths.

**Signatures (proposed):**
```rust
pub struct RoadGraph { pub csr: Csr, pub coords: Vec<(f64, f64)> }   // coords[i] = (lat,lng) of node i

/// Dijkstra/A* over CSR. `heuristic=false` ⇒ pure Dijkstra (h≡0); `true` ⇒ haversine A*.
/// Returns (node path src→dst, total weight) or None if unreachable. Deterministic
/// tie-break by node id via the (Prio, usize) heap ordering (cost_estimate.rs:244).
pub fn route(g: &RoadGraph, src: usize, dst: usize, heuristic: bool) -> Option<(Vec<usize>, f64)>;

/// Convenience alias: A* with the haversine lower bound.
pub fn shortest_path(g: &RoadGraph, src: usize, dst: usize) -> Option<(Vec<usize>, f64)>;

/// Optional CH acceleration (port of build_shortcuts:157). Off by default — CH legitimately
/// collapses intermediate hops, so it is used only where exact node-sequence is not asserted.
pub fn build_shortcuts(g: &RoadGraph) -> Vec<Shortcut>;
```
The `Prio(f64)` `total_cmp` wrapper (`cost_estimate.rs:35`) ports verbatim — it is the correct
NaN-safe heap ordering and must not be reinvented.

**Road-graph ingestion port.** A thin `pub fn road_graph_from_ways(nodes: &[(f64,f64)],
ways: &[(usize,usize,f64)]) -> RoadGraph` that: keys OSM node ids → dense indices, emits both
directions per undirected way (per `csr.rs:73` undirected convention), floors each weight at
`1e-6` (A*/Eikonal need `W>0`, `cost_estimate.rs:88`), and calls `Csr::from_edges`. OSM parsing
itself is a downstream (Phase 13) concern; this port takes already-extracted `(node,node,cost)`
triples so the kernel stays I/O-free and offline-clean.

**Done-test note (must be honoured by the implementer):** on the 10-node oracle, assert **total
cost** equals the hand-computed shortest cost and endpoints equal `src`/`dst`; assert the exact
node **sequence** only with CH **disabled** (shortcuts collapse hops by design —
`cost_estimate.rs` test at `:396` already documents this). The admissibility check is
`route(g,s,d,true) == route(g,s,d,false)` in both path cost and (CH-off) sequence.

---

## 3. DSU / MST primitive + `cgraph` parity-swap

**New `kernel/src/dsu.rs`** (pure `std`, zero deps):
```rust
pub struct Dsu { parent: Vec<usize>, rank: Vec<usize> }
impl Dsu {
    pub fn new(n: usize) -> Self;
    pub fn find(&mut self, x: usize) -> usize;          // path compression
    pub fn union(&mut self, a: usize, b: usize) -> bool; // union by rank; false if already joined
    pub fn connected(&mut self, a: usize, b: usize) -> bool;
    /// Canonical partition: members ascending WITHIN a set; sets ordered by ascending
    /// minimum member. This ordering is the byte-parity contract with cgraph (§below).
    pub fn components(&mut self, present: &[bool]) -> Vec<Vec<usize>>;
}

/// Kruskal MST over undirected weighted edges. Deterministic tie-break: sort by
/// (weight, min(u,v), max(u,v)) ascending. Returns chosen edges + total weight.
/// Used for the mesh gossip/overlay spanning tree (M7). Consumes Csr or a triple list.
pub fn kruskal_mst(n: usize, edges: &[(usize, usize, f64)]) -> (Vec<(usize, usize, f64)>, f64);
```

**Parity-swap plan for `cgraph::c_components`.** Add `c_components_dsu()` that: for each present
node `i`, unions every bidirected arc `(i,j)` with `j>i` (exactly `cgraph.rs:174-181`); then emits
`Dsu::components(&self.present)`. The **byte-parity contract** is subtle and non-negotiable:
the existing flood-fill emits components ordered by ascending-minimum-node (because the `0..n`
first-unseen scan reaches each component at its smallest present member), members sorted
ascending (`cgraph.rs:201`). `Dsu::components` must reproduce **exactly** that ordering — hence
the "sets ordered by ascending minimum member" rule above. Only nodes with `present[i]` appear.

Swap procedure (regression-safe): (1) land `c_components_dsu` alongside the original; (2) add a
parity test `assert_eq!(g.c_components(), g.c_components_dsu())` across the existing fixtures
(`chain_graph`, `m_graph`, and the malformed/edge fixtures at `:558-629`); (3) once byte-identical,
make `c_components()` **delegate** to the DSU implementation (single call-site change), keeping
every existing cgraph test (`:558`, `:569`, d-sep tests `:583`/`:592`) GREEN unchanged. If any
fixture diverges, the DSU ordering is wrong — fix the primitive, never the fixture.

---

## 4. The six GS geo functions (verbatim per GS §2.6 / §4 P0.1)

Added to `kernel/src/geo.rs` in its existing pure/deterministic style (reuse `DEG2RAD`,
`bearing_deg`, `point_in_polygon`). Angles in degrees, 0=N clockwise (matching `bearing_deg`).

- `storey_height_m(height_m: Option<f64>, levels: Option<u32>) -> f64` — measured `height/levels`
  when **both** present and `levels>0`; else generic **3.0 m** storey. **Never fabricates a level
  count** (returns a per-storey height only). Edge: `levels=Some(0)` → 3.0 m (no divide-by-zero).
- `floor_slice_height_m(floor: u32, storey_h: f64) -> f64` = `(floor as f64 + 0.5) * storey_h` —
  mid-storey plane so the cut passes through windows/doors (the `+0.5`).
- `arrow_screen_rotation_deg(facing_deg: f64, view_rotation_deg: f64) -> f64` =
  `angular_diff`-normalised `(facing - view_rotation).rem_euclid(360)`. On a **north-up** slice
  (`view_rotation=0`) this is `facing` unchanged — one global frame; v1 must **not** rotate the
  view to straighten facades (one missed subtraction and the arrow lies).
- `angular_diff_deg(a: f64, b: f64) -> f64` — smallest unsigned separation, seam-correct:
  `let d = (a-b).rem_euclid(360.0); d.min(360.0-d)`. Oracle: `angular_diff_deg(350,10)==20`.
- `in_field_of_view(facing_deg: f64, target_bearing_deg: f64, fov_deg: f64) -> bool` =
  `angular_diff_deg(facing, target) <= fov/2.0`. Default `fov=120` (±60°); optional ±30° "direct".
  Correctness across the **0°/360° seam** falls out of `angular_diff_deg`.
- `los_clear(a: (f64,f64), b: (f64,f64), footprints: &[Vec<(f64,f64)>]) -> bool` — coarse **2D**
  segment-vs-footprint-edge intersection; `true` iff the a→b segment crosses no footprint edge.
  **Stated limits stay loud** (GS §2.6): ignores height (false-positive over low walls,
  false-negative across courtyards) and reads empty footprint data as "all clear" ⇒ drives a
  **soft advisory hint only**, never a hard visibility claim. Composition: `point_in_polygon`
  (`geo.rs:200`) chooses building-vs-open-space branch; `bearing_deg(self,other)` feeds the FOV.

**GS P0.1 acceptance (verbatim, must pass):** (1) pin-drop works everywhere, no external-data
dependency; (2) OSM footprint+levels → correct floor-selector range + north-up slice at
`floor_slice_height_m`, verifiable against the raw tag; (3) no footprint → graceful open-space
degrade, **no crash, no fabricated floor**; (4) arrow bearing matches `bearing_deg` to **<1°**;
(5) `in_field_of_view` correct across the 0/360 seam; (6) `los_clear` **false** across a known
rectangular footprint, **true** routing around it.

---

## 5. wasm-export plan

**`compose` — already done; formalise + prove browser reach.** No new export code for compose:
`dowiz-wasm::compose_field` (`wasm/src/lib.rs:57`) and `FieldSim` (`:64`) exist with GREEN host
determinism (`:204`, `:288`). Phase-4 work here is (a) confirm the wasm32 build links and the
determinism `assert_eq!` holds, and (b) close the documented ceiling (`:172`): generate the
wasm-bindgen glue with the pinned `=0.2.95` CLI and add ONE browser smoke
(`canvas.putImageData(compose_field(...))` renders + a second identical run is byte-equal). The
"bit-identical frames across two runs" done-test is the existing host test promoted to the
wasm/browser boundary.

**New pure-kernel functions → `kernel/src/wasm.rs`.** Follow the established `geo_*_js` pattern
(`wasm.rs:470-607`): a private `_logic` fn returning `Result<String,String>` + a thin
`#[wasm_bindgen] *_js` wrapper mapping to `JsValue`, arguments as JSON strings / flat numeric
arrays. Add:
- `route_js(nodes_json, ways_json, src, dst, heuristic) -> String` (JSON `{path:[..], cost}`),
  built on `router::road_graph_from_ways` + `router::route`.
- `mst_js(n, edges_json) -> String` (JSON chosen-edge list + total weight) via `dsu::kruskal_mst`.
- Six `geo_*_js` wrappers for the §4 functions (footprints/polylines as JSON per
  `parse_polyline`, `wasm.rs:507`). `los_clear` takes a JSON array of footprint rings.

`c_components` needs no new export (internal causal-ID primitive); the DSU swap is transparent to
its callers. Everything above stays behind the kernel `wasm` feature (`kernel/Cargo.toml:23`) so
the default offline build pulls no wasm-bindgen/serde.

---

## 6. Acceptance criteria (numbered checklist)

1. **Route oracle.** On a hand-built 10-node graph, `router::shortest_path` returns total cost ==
   the hand-computed shortest-path cost; endpoints == src/dst.
2. **A* admissibility.** `route(g,s,d,heuristic=true)` == `route(g,s,d,heuristic=false)` in path
   cost, and in exact node sequence with CH disabled (haversine heuristic is admissible).
3. **DSU parity.** `c_components_dsu()` is **byte-identical** (`assert_eq!`) to the existing
   `c_components()` across all `cgraph.rs` fixtures; after delegation, every existing cgraph test
   stays GREEN.
4. **MST.** `kruskal_mst` returns the known-minimum spanning tree on a hand oracle with the
   documented deterministic tie-break; total weight matches.
5. **GS P0.1 six-item list** (§4) passes verbatim: pin-drop everywhere / floor-slice range vs raw
   OSM tag / open-space degrade with no crash and no fabricated floor / arrow bearing vs
   `bearing_deg` <1° / FOV across the 0°/360° seam / `los_clear` false across a rectangle, true
   around it.
6. **`compose` determinism + reach.** `compose_field`/`FieldSim` produce bit-identical frames
   across two independent runs (host `assert_eq!` GREEN today); wasm32 build links and a browser
   `putImageData` smoke renders the same bytes twice.
7. **Zero-dep discipline.** Default `cargo build`/`test` dependency graph is byte-identical to
   today (router/dsu/geo are pure `std`; no new crate enters any default Cargo.lock). This is the
   E61 exemplar invariant.
8. **Single-library check.** No duplicate router/DSU/MST/geo implementation exists in bebop2 or
   elsewhere in dowiz (grep) — Phases 9/13/16/17 import **this** module.

---

## 7. What this unblocks (downstream dependency)

This phase is Wave-0 and gates a large slice of the critical path. Building the library once, here,
is the whole point — R2 §1 merge note and §3 adjacency list make the consumers explicit:

- **Phase 9 (Confidential, self-healing wire).** M7/E31-E33 heal layer — partition detect,
  shortest-path re-route, MST overlay spanning tree — consumes `router::route` + `dsu`/`kruskal_mst`
  directly ("a dropped node is routed around via recomputed shortest path + spanning tree").
  Phase 9 hard-depends on Phase 4 (adjacency: `P9 ← P3, P4`).
- **Phase 13 (Delivery on protocol).** F45 product routing feeds `progress_along_route`/ETA
  unchanged; F46 partition-tolerant delivery uses the same DSU/MST. `P13 ← P4, P7, P9, P10`.
- **Phase 16 (Product UI rebuild).** The address-picker v1 ships the six geo functions + OSM
  floor-slice; UI physics consumes `compose_field`/`FieldSim`. `P16 ← P4, P13`.
- **Phase 17 (Demo, splat tiers, GPU-unlock).** The scripted wasm delivery demo runs a courier
  along a **Phase-4-computed** route with the field-physics `compose` render; the GS splat P1 tier
  layers on top of the same six geo functions. `P17 ← P11, P16`.

Any of these re-implementing route/DSU/MST/geo would violate the "one implementation" mandate and
the E61 zero-dep exemplar. Phase 4 is small (2 anchors) precisely because its value is
**consolidation**: one library, built exactly once, proven by the checklist above.

---

## 8 — Planning-protocol completion appendix (2026-07-17, decorrelated pass)

> Independent verifier pass. Re-checked every cited file:line against the live `feat/harness-llm-backend`
> checkout and re-derived §7's load-bearing sequencing claim by finding (or failing to find) a real
> file-level dependency edge, per this task's instructions.

### (i) Citation-verification results

**All dowiz-side citations re-verified accurate, none stale — nothing has landed yet.** `kernel/src/router.rs`
and `kernel/src/dsu.rs` do not exist; this remains a pure planning document with zero implementation
drift since authorship. Spot-checked and confirmed at the cited lines: `kernel/src/geo.rs`
(`haversine_meters:15`, `bearing_deg:30`, `ema_next:39`, `progress_along_route:70`, `eta_seconds:153`,
`point_in_polygon:200`); `kernel/src/cgraph.rs` (`c_components:171`; test fixtures `chain_graph`/`m_graph`
and the two `green_*` regression tests exist within a line or two of the cited `:558`/`:569`);
`kernel/src/csr.rs` (`from_edges:79`, `to_adjacency:190`); `kernel/Cargo.toml`'s `wasm` feature line;
`/root/bebop-repo/crates/bebop/src/cost_estimate.rs` (`route`, the A* heuristic, `build_shortcuts`,
`reachable`, `weighted_adj`, `hybrid_route` all present, function bodies matching the description).
`engine/src/field_frame.rs:189` (`compose`) and the wasm-export citations all confirmed byte-accurate,
including `wasm_compose_deterministic`/`wasm_fieldsim_deterministic` at the exact cited lines and the
`innovate:` ceiling comment at `:172`.

**One precision correction, harmless but literal.** §5 cites `dowiz-wasm/src/lib.rs`. `dowiz-wasm` is
the **crate name** (`wasm/Cargo.toml:2`); the **path** is `wasm/src/lib.rs`, not `dowiz-wasm/src/lib.rs`
— a reader following the citation as a filesystem path will not find that directory. All the specific
line numbers cited under that name (57, 64, 172, 204, 288) are otherwise exactly right.

### (ii) DECART

**No DECART owed.** Every new module this blueprint designs (`router.rs`, `dsu.rs`, the six geo
functions) is explicitly pure-`std`, zero-external-dependency, and acceptance criterion #7 makes that
an falsifiable invariant ("default `cargo build`/`test` dependency graph is byte-identical to today").
No new crate, service, or vendor choice is introduced anywhere in this document.

### (iii) 2-question doubt audit

**Q1 — least confident about (concrete):**
1. I did not re-derive the exact byte offsets of `cost_estimate.rs:35` (`Prio` `total_cmp` wrapper) —
   the line I read at that offset was a `#[derive(...)]` attribute, not the wrapper struct itself; the
   wrapper is nearby but I did not pin its exact line, so §2's citation may be off by a few lines the
   way the SELF-CRITIQUE pass already found for this same file (R1-A cited "238-290", R2/P04 cited
   "205-290" for the same function — a pre-existing, documented drift this pass did not re-resolve).
2. I did not build or dry-run any part of the router/DSU port to confirm the admissibility contract
   (haversine as a valid Euclidean-in-degree-space substitute lower bound) actually holds under real
   OSM edge weights — the argument in §2 is sound on paper (great-circle ≤ any road path in metres) but
   untested against a real graph.
3. I did not verify the DSU byte-parity claim (§3, "`Dsu::components` must reproduce exactly [cgraph's]
   ordering") against an actual DSU implementation, because none exists — this is a design constraint,
   not yet a checked one, and union-by-rank's natural output order is NOT ascending-by-construction, so
   the "sets ordered by ascending minimum member" requirement will need real post-processing the
   blueprint names but does not show is cheap.
4. I did not check whether the GS geo functions' cited spec document
   (`GAUSSIAN-SPLATTING-…-SYNTHESIS-2026-07-16.md §2.6`) still matches §4's verbatim restatement — I
   trusted the "not re-derived here" framing at face value.
5. I did not check whether `wasm.rs:470-607` (`geo_*_js` pattern) or `wasm.rs:507` (`parse_polyline`)
   still sit at those exact lines — high-churn file per this repo's own hotspot data is `apps/*`, not
   `kernel/src/wasm.rs`, so risk is lower, but I did not spot-check it this pass.
6. The zero-dep discipline (criterion #7) assumes `cargo vendor`/offline builds stay feasible — I did
   not re-check whether Phase 1's vendoring decision (a real open item in that blueprint) affects this
   phase's own "byte-identical Cargo.lock" claim if vendoring changes how deps resolve.

**Q2 — biggest thing this plan might be missing:** the phase's own §7 marquee claim — "math primitives
land ONCE here, consumed by Phase 9" — is **not just under-derived, it is confirmed underivable as
written** (see Anu below). The blueprint is otherwise careful and self-contained for the dowiz-only
half of its scope (router/DSU/geo/wasm, all real file citations, all zero-dep), but its own stated
*reason for existing at Wave-0 priority* — "consolidation before Phase 9/13 need it" — rests on a
cross-repo consumption path that has no dependency edge today and a canon law (M6) standing against
creating one casually. A reader could build everything in §2-§5 correctly and still not have
delivered what §7 promises Phase 9 will get.

### (iv) Anu & Ananke check

**Anu — the load-bearing sequencing claim does not survive re-derivation.** §7 asserts "Phase 9 hard-
depends on Phase 4 (adjacency: P9 ← P3, P4)" and "[Phase 9] consumes `router::route` + `dsu`/`kruskal_mst`
directly." I checked this the way the task asks: `BLUEPRINT-P09-confidential-self-healing-wire.md`
plans its self-healing/mesh-heal work inside **`bebop-repo`**'s `proto-wire`/`mesh-node`/`bebop2-core`
crates (confirmed by its own text: "Phase 9 touches `proto-wire` + a new heal module"). This blueprint's
router/DSU land in **`/root/dowiz/kernel`** — a **different git repository**. There is no Cargo
dependency edge between them today, and M6 (canon: zero-dep protocol boundary) stands against creating
one casually. This is not a new finding — `SELF-CRITIQUE-2Q-DOUBT-AUDIT.md` §1.2 already confirmed the
identical problem for the master roadmap's merge table ("CONFIRMED — the merge's marquee de-duplication
is not proven buildable as written") — but P04 §7 restates the un-derivable claim as settled fact
("R2 §1 'Major merges' is explicit... They do not get their own copies") without carrying that doubt
forward into its own text. Per Anu, this decision should be downgraded from "hard-depends" to: **the
consumption path requires an unresolved cross-repo decision (vendor the router/DSU into bebop2, publish
a shared crate, or duplicate with a documented parity test) that neither this blueprint nor P09 makes**
— a DECART-shaped decision this phase's own scope does not currently own.

**Ananke.** What survives on structure alone: the acceptance criteria (§6, 8 items, each a real
command/assertion) and the zero-dep invariant (#7) are genuinely self-checking — a future implementer
cannot silently violate them without a failing `cargo build`/`test` diff. What does NOT survive on
structure alone: the Phase-9/13/16/17 "consumption" story in §7 is pure prose with no artifact that
would fail if the cross-repo edge never gets built — nothing here creates a test, a DECART placeholder,
or even a TODO in the P09 blueprint that would force the question to be asked again before Phase 9's
implementer discovers, mid-build, that the library they were told to import lives in a repo they cannot
depend on. The cheapest structural fix (not built here, flagged for whoever picks up Phase 9): P09's own
blueprint should carry an explicit, named open item — "router/DSU source repo: dowiz kernel vs vendored
copy vs shared crate — unresolved, blocks the 'consumes Phase 4' claim" — so the gap is a checked
precondition, not a silent assumption a builder inherits by trusting this document's confident tone.
