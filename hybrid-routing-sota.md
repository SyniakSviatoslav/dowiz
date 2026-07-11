# Hybrid Spatial + Topological + Cost-Aware Routing — SOTA Brief

*For a Rust/edge logistics engine architect. ENGLISH. Each non-trivial claim carries a source URL.*

---

## (1) Spatial pre-filtering: k-d tree / R-tree radius culling

**What it does:** Before any graph search, cull the candidate node set to a geometric neighborhood (e.g. all depots within radius R of a pickup, or the rectangle bounding the route corridor). This shrinks the graph that BFS/dijkstra ever touch.

**Rust crates + maturity:**

- **`rstar`** (georust org) — R\*-tree, n-dimensional, the de-facto spatial index in Rust geo ecosystem. Mature, actively maintained, 1.0+ line. Excellent for rectangle/circle window queries and kNN. Source: https://github.com/georust/rstar
- **`kiddo`** — modern, high-performance k-d tree (v4.x). A heavily optimized fork of the older `kdtree` crate, rewritten with const generics + SIMD friendliness. Best raw NN/window-query throughput in Rust. Source: https://crates.io/crates/kiddo
- **`kdtree`** — the original, older crate; still works but superseded by `kiddo` for performance. Source: https://crates.io/crates/kdtree
- **`petgraph`** — NOT a spatial index; it is the graph DS + algorithm lib (BFS, Dijkstra, connected-components). Mature, the standard Rust graph crate. Source: https://docs.rs/petgraph

**Typical latency (geometric radius culling):**

- A single k-d / R-tree window or kNN query over an in-memory index of millions of points lands in the **low-microsecond to tens-of-microseconds** range (≈1–50 µs) on modern hardware, because it is a cache-friendly tree descent with O(log N) comparisons plus a small linear scan of the leaf. k-d tree point/circle queries are routinely **sub-10 µs** for 10⁵–10⁶ points; `rstar` rectangle queries behave similarly. Practical confirmation: a robotics planner using `kiddo` for spatial NN in a Rust/SIMD inner loop reports per-query costs in the microsecond band (https://claytonwramsey.com/blog/captree/).
- Contrast: anything that touches disk, deserializes, or allocates per query jumps to **ms**. Keep the index resident in RAM and reuse the allocation → stay in µs.

**Verdict for layer 1:** cheap. µs-scale. Never the bottleneck if the index is resident.

---

## (2) BFS / connected-components: the "does a path exist" topological guard

**What it does:** Pure topology, cost-blind. Two uses:
- **Reachability / connectivity check** — is the destination even in the same connected component as the source? A BFS or union-find over components answers "yes/no path exists" in O(V+E).
- **Spatial-index → graph bridge** — after k-d/R-tree returns candidate nodes, run a BFS flood (or component lookup) to confirm the culled candidate is actually reachable through *real* edges, not just geometrically near.

**Cost:** O(V+E) traversal, typically **microseconds for local floods**, sub-ms even for whole-graph component scans at road-network scale. With petgraph's `Bfs` / `connected_components` this is well under 1 ms for the local neighborhoods you care about. Source: https://docs.rs/petgraph/latest/petgraph/visit/struct.Bfs.html

**Why it matters in the 3-layer pipeline:** It is a cheap gate that prevents you from spending the expensive cost-aware wavefront on an unreachable target. Put it *between* spatial culling and the cost search.

---

## (3) A* / Dijkstra with cost-aware edge weights

**What it does:** Single-source shortest path where edge weight `W_uv = f(latency, cost, risk)`. A* adds an admissible heuristic h(n) (e.g. great-circle distance / max-speed) to prune the frontier; Dijkstra is A* with h≡0. Both are exact for non-negative weights; Bellman-Ford if you need negative weights (rare in logistics).

**Rust:** `petgraph` gives `dijkstra`, `astar`, `bellman_ford` out of the box. Source: https://docs.rs/petgraph/latest/petgraph/algo/index.html

**Latency:** On a bare road graph (no CH), a point-to-point Dijkstra/A* at continental scale is **tens to hundreds of ms** — too slow for edge/online logistics. This is exactly why production engines add Contraction Hierarchies (see §5): CH drops A*/Dijkstra query to **1–10 ms**.

**Cost-aware weighting:** fold latency+cost+risk into one scalar (weighted sum or lexicographic). Note: once you have a single scalar non-negative weight, it is *just* shortest-path — the "cost-aware" part is entirely in how you compute W_uv, not in the search algorithm.

---

## (4) THE KEY QUESTION — is damped wavefront propagation == Dijkstra?

**Short answer: YES, under a precise correspondence — and it has established names.**

If you set each edge's local wave *speed* `F_uv = 1 / W_uv` (the wave slows on high-weight edges, speeds on low-weight edges), then the arrival-time field T(v) built by a **Dijkstra-style wavefront expansion** is *exactly* the shortest-path distance using weights W_uv. This is not a metaphor — it is the literal foundation of the **Fast Marching Method (FMM)**.

**What it is called:**
- **Fast Marching Method (FMM)** — the canonical name. Sethian's algorithm solving the **Eikonal equation** `|∇T(x)| = 1/F(x)` via a Dijkstra-like one-pass narrow-band expansion. Source: https://en.wikipedia.org/wiki/Fast_marching_method
- **Eikonal equation** — the continuous PDE whose viscosity solution is the arrival-time / distance field; FMM is its discrete solver. Source: https://pmc.ncbi.nlm.nih.gov/articles/PMC18495/ (Sethian & Vladimirsky, PNAS 2000 — "The Fast Marching Method is connected to … Dijkstra's method, which is an algorithm for computing smallest cost paths on a network.")
- **"Dijkstra-like methods for the Eikonal equation"** — Tsitsiklis (1995) proved the equivalence independently: a unit-speed trajectory minimizing ∫g(x)dt is solved by an O(n log n) Dijkstra-emulation on a discretized grid. Source: https://www.mit.edu/~jnt/dijkstra.html
- **Bellman-Ford on a potential field** — different framing: Bellman-Ford / value-iteration *is* the dynamic-programming fixed-point view of the same shortest-path problem (`T(v) = min_u { T(u) + W_uv }`). FMM is the *causal, one-pass* version of that update; Bellman-Ford is the *iterative label-correcting* version. So "Bellman-Ford on a potential field" is mathematically the same problem family, just a slower (iterative) solver.
- **Dijkstra-as-wavefront** — the plain-English description; accurate.

**Critical nuance (norm dependence):**
- On a **grid** with the **L∞ (max) norm**, FMM's update reduces *exactly* to Dijkstra. Source: https://github.com/Dmitrij91/FM_Eikonal ("For the infinity norm … equation (3) simplifies to a straightforward Dijkstra shortest path algorithm").
- On a **graph** with explicit edge weights W_uv, set speed `F_uv = 1/W_uv` and the FMM narrow-band expansion *is* Dijkstra on that weighted graph — same priority queue, same settle order, same distances. This is the "weighted-graph Eikonal" formulation (Desquesnes/Elmoataz/Lézoray 2012, implemented in FM_Eikonal).
- With Euclidean (L2) stencils on a fine grid you get a *smoother* geodesic distance but pay a small accuracy/complexity cost; for a discrete logistics graph you do **not** need L2 — use the graph weights directly.

**Production or academic?**
- The **method** is mature and production-grade in spirit, but production routing engines do **NOT** run FMM for point-to-point queries — they run Dijkstra/A* + Contraction Hierarchies (FMM on a road graph *is* Dijkstra, and CH beats naked Dijkstra). FMM/Eikonal is heavily used in production for **distance/arrival-time *fields* and reachability maps** (robotics, medical imaging segmentation, geodesic distance transforms, level-set methods), not for single-pair routing. So: academically proven since 1995, industrially used for *field* computation, redundant-with-Dijkstra for *pair* routing.

**Bottom line for your "cost-wave" layer:** Your "damped cost-wave" **is** Dijkstra with `W_uv = 1/F_uv`. Don't reinvent it as a PDE solver — implement it as A*/Dijkstra over petgraph with edge speeds `F_uv = 1/W_uv`. You get the wavefront semantics for free, with a battle-tested priority-queue implementation. If you want the full *field* (arrival time to every node), that's a multi-source Dijkstra / FMM — same code, different stop condition.

---

## (5) How real engines layer it — and where THEIR cost lives

**Pipeline pattern (OSRM / GraphHopper / Valhalla / Mapbox):**

1. **Spatial indexing** — not for routing queries per se; used for *snapping* the input coordinate to the nearest graph node (k-d/R-tree over node geometry). Cheap, µs.
2. **Preprocessing / speed-up hierarchy** — the dominant architectural investment:
   - **Contraction Hierarchies (CH):** precompute "shortcuts" by contracting low-importance nodes; query then walks mostly high-level edges. OSRM & GraphHopper primary path. Source: https://en.wikipedia.org/wiki/Contraction_hierarchies
   - **Multi-Level Dijkstra (MLD):** OSRM's newer default; partitions graph into cells, precomputes boundary distances. Better for flexible/real-time weights. Source: https://github.com/Project-OSRM/osrm-backend
   - **Valhalla:** bidirectional A* + hierarchy (Valhalla calls its preprocessor; historically lighter on CH than OSRM/GraphHopper — https://github.com/valhalla/valhalla/issues/1514).
   - **Mapbox:** forked/derived from OSRM lineage (CH/MLD-style).
3. **Query-time search:** bidirectional A* / CH-unpack over the hierarchy. Cheap at query time.

**Concrete latency (precompute vs query):**

- **OSRM CH query:** ~**5 ms** for a cross-continental (US) route. Preprocessing: **~6 hours** on a beefy machine for a continental extract. Source: https://news.ycombinator.com/item?id=12640162
- **CH query at road-network scale:** "order of milliseconds" (sub-10 ms) for continental networks. Source: https://publications.scss.tcd.ie/theses/diss/2013/TCD-SCSS-DISSERTATION-2013-047.pdf
- **GraphHopper:** CH ~**10× faster** than flexible (non-CH) routing; flexible routing was made "15× faster" but CH still wins ~10× on average. Source: https://www.graphhopper.com/blog/2017/08/14/flexible-routing-15-times-faster/
- **CH preprocessing:** sequential builds **22–41 min**; parallel best-effort **>300 s** for continental networks. Source: https://dl.acm.org/doi/full/10.1145/3721145.3725744
- **Field report:** "OSRM super fast, esp. short routes; GraphHopper nearly as fast as OSRM, can preprocess the world in 64 GB RAM." Source: https://news.ycombinator.com/item?id=17001422

**Where THEIR dominant latency cost is:** unambiguously **precompute (offline), not query (online).** Query is 1–10 ms; preprocessing is minutes-to-hours. The trade is deliberate: pay once offline so every query is ms.

---

## CRISP VERDICT — single biggest latency bottleneck in a **k-d + BFS + cost-wave** 3-layer pipeline

**The bottleneck is Layer 3: the cost-aware wavefront (A*/Dijkstra) over the *uncontracted* graph.**

Reasoning, ranked:
1. **Layer 1 (k-d/R-tree cull):** µs. Resident index, cache-friendly. Negligible.
2. **Layer 2 (BFS/connectivity):** sub-ms for local floods. Negligible as a *guard*.
3. **Layer 3 (cost-wave / Dijkstra on raw edges):** **tens–hundreds of ms** at logistics/road scale because nothing prunes the search — no hierarchy, no CH shortcuts, no bidirectional bound. This dominates by 2–4 orders of magnitude.

**Fix that removes the bottleneck:** add a **Contraction-Hierarchy-style preprocessing pass** (or at minimum bidirectional A* with an admissible heuristic) so the cost-wave only traverses high-level shortcuts. That is precisely what OSRM/GraphHopper/Valhalla do, and it is what converts Layer 3 from 100 ms → 5 ms. The spatial index and BFS gate are correctness/pruning helpers; they are not where time goes. **If you keep the cost-wave on the raw graph, that single layer is your latency ceiling — everything else is rounding error.**

**Recommended Rust stack:**
- Spatial cull: `rstar` (rect/circle windows) or `kiddo` (kNN/point, fastest).
- Topology gate: `petgraph` `Bfs` / `connected_components`.
- Cost-wave: `petgraph` `astar`/`dijkstra` with `W_uv = 1/F_uv` (this *is* your damped wavefront — don't write a PDE solver).
- Speed-up (the actual fix): implement CH preprocessing yourself, or adopt a CH-capable engine (OSRM/GraphHopper) and call it; no mature pure-Rust CH crate yet ships turn-by-turn at continental scale out of the box — `petgraph` + hand-rolled contraction is the common path for edge deployments.

---

### Source index
- rstar: https://github.com/georust/rstar
- kiddo: https://crates.io/crates/kiddo
- kdtree: https://crates.io/crates/kdtree
- petgraph: https://docs.rs/petgraph
- FMM (Wikipedia): https://en.wikipedia.org/wiki/Fast_marching_method
- Sethian & Vladimirsky PNAS 2000: https://pmc.ncbi.nlm.nih.gov/articles/PMC18495/
- Tsitsiklis "Dijkstra-like methods for the Eikonal equation": https://www.mit.edu/~jnt/dijkstra.html
- FM_Eikonal (graph FMM, L∞ == Dijkstra): https://github.com/Dmitrij91/FM_Eikonal
- Contraction hierarchies: https://en.wikipedia.org/wiki/Contraction_hierarchies
- OSRM backend (CH/MLD): https://github.com/Project-OSRM/osrm-backend
- OSRM CH latency/preproc (HN): https://news.ycombinator.com/item?id=12640162
- GraphHopper flexible vs CH: https://www.graphhopper.com/blog/2017/08/14/flexible-routing-15-times-faster/
- CH preprocessing time (ACM): https://dl.acm.org/doi/full/10.1145/3721145.3725744
- kiddo/Rust SIMD spatial perf: https://claytonwramsey.com/blog/captree/
