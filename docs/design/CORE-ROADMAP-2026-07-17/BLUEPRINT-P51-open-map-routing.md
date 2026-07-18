# BLUEPRINT P51 — Open map + routing: OSM vector data, field-rendered routes, pin-drop, live tracking (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §10). Component:
> **DELIVERY**. Operator ask (2026-07-18, verbatim): "потрібна - openstreet map з можливістю
> ставити піни та трекати маршрут - або краще і цікавіше фізичний рендер маршруту/карти на
> основі супутникових даних - не платних і не vendor lock-in." — OpenStreetMap with pin-drop +
> route tracking, OR (better) a physics-render of the route/map from satellite data; hard
> constraints: **non-paid, non-vendor-lock-in**. §1 answers the satellite-vs-vector question
> with a falsifiable verdict; the design that follows delivers the physics-render — of OSM
> vector data, through the existing field engine — because the satellite leg fails on
> measurable resolution physics, not on taste. Structural template:
> `BLUEPRINT-P-A-kernel-primitives.md` (numbering mirrored); sibling precedent:
> `BLUEPRINT-P38-webgpu-render-engine.md`.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads. The single most load-bearing finding:
**the kernel already contains a zero-dependency routing engine** — this phase feeds and renders
it, it does not build one.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| In-kernel router EXISTS: "CSR-native Dijkstra / A* shortest path + Contraction-Hierarchy shortcuts + OSM road-graph ingestion" (P04 product-math, ported from bebop `cost_estimate.rs` with couplings dropped) | `kernel/src/router.rs:1-14` (module doc); registered `kernel/src/lib.rs:181` | **VERIFIED — P51 consumes, never re-derives** |
| `RoadGraph { csr, coords }` — CSR adjacency + index-aligned `(lat,lng)` | `kernel/src/router.rs:44-47` | VERIFIED |
| `route()` Dijkstra/A* (haversine admissible heuristic, deterministic tie-break by node id), `shortest_path` alias, `build_shortcuts` CH (off by default) | `kernel/src/router.rs:71-78` (doc+sig), `:163-164`, `:168-171` | VERIFIED |
| `road_graph_from_ways(nodes, ways)` takes **clean triples**; its own doc: "OSM parsing itself is a downstream (Phase 13) concern; this takes clean triples so the kernel stays I/O-free" — floors weights at `1e-6` (W > 0), emits both directions | `kernel/src/router.rs:221-228` | **VERIFIED — P51 IS that named downstream concern** |
| Route kinematics already complete: `haversine_meters`, `lerp_lat_lng`, `bearing_deg`, `ema_next`, `RouteProgress`, `polyline_length_meters`, `progress_along_route` (full polyline projection + snap), `eta_seconds`, `is_out_of_order`, `ARRIVE_THRESHOLD_M = 150.0`, `SNAP_THRESHOLD_M = 500.0`, `should_snap`, `is_arriving`, `point_in_polygon` | `kernel/src/geo.rs:15,25,30,39,45,56,70,153,171,179,181,187,194,200` | VERIFIED |
| Splatting-arc UX geometry also in geo.rs (storey/floor-slice/FOV/LOS) — Stage-2 substrate, untouched by P51 | `kernel/src/geo.rs:230,239,262,274` | VERIFIED |
| Full n-D Kalman filter, fail-closed on singular S, innovation/surprise surfaced; **2-D constant-velocity model already tested** (`F=[[1,1],[0,1]]`, observe position, velocity converges) | `kernel/src/kalman.rs:149-289` (filter), `:212-250` (update), `:392-420` (`kalman_2d_constant_velocity`) | VERIFIED — the tracking estimator is a configuration of this, not new math |
| `ema_next` is the proven scalar steady-state special case of the KF | `kernel/src/kalman.rs:1-25` (doc), `:357-389` (equivalence test) | VERIFIED |
| Graph substrate: `Csr::from_edges` (out-of-range endpoints dropped), `spmv`, `laplacian_spmv(LaplacianKind)`, `personalized_pagerank` | `kernel/src/csr.rs:79,268,536,330` | VERIFIED |
| Incidence/Laplacian operators (E1) | `kernel/src/incidence.rs:76,88,107` (`grad`/`div`/`laplacian`) | VERIFIED |
| Engine scene already has the road-render primitive: `SdfShape::LineSegment` ("thin line primitive, distance to nearest point on segment") + `Scene::add/render_frame/render_to_bridge` | `engine/src/scene.rs:43` (variant), `:71,88,122,168` | **VERIFIED — a road polyline is N LineSegments in the EXISTING scene type** |
| `compose(scene, eq, w, h, steps) -> Vec<u8>` — scene renders a **source field**, the field integrator diffuses it, bit-deterministic (P38 §0's oracle) | `engine/src/field_frame.rs:218-226` | VERIFIED — a route added as scene source ⇒ its glow IS the diffusion field, zero new render math |
| wasm surface has **ZERO** geo/route exports (P04's `route_js` DoD line never landed): grep `route\|geo\|road` over `wasm/src/lib.rs` → 0 hits | `wasm/src/lib.rs` (exports at `:57,78,96,112,153-168` only) | **VERIFIED gap — P51 closes it per P38 G7 ptr/len conventions** |
| No OSM/PBF parser anywhere in product code (repo grep, node_modules excluded) | grep this pass | VERIFIED — extraction tool is genuinely new |
| `serde_json` is in-tree and cached (kernel `wasm`/`pq` features pull it; engine deliberately dep-free) | `kernel/Cargo.toml:24,42`; `engine/Cargo.toml:15-21` | VERIFIED — a JSON-consuming extractor tool needs **no network unlock** |
| P49 (roadmap §11) owns customer identity/notifications and its DoD-4 names "a live tracking view renders real geo state (Kalman/EMA output) through P38a's pipelines" | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:1207-1242` | VERIFIED — overlap resolved in §2 (P51 supplies, P49 consumes) |
| P38 G3 SDF instanced pipeline + G2 particles + FE-06 MSDF text are the render substrate; GPU legs gated on O18a | `BLUEPRINT-P38-webgpu-render-engine.md` §1-§3 | VERIFIED — P51 renders through P38's pipelines, never beside them |
| Splatting arc verdict (prior art, binding): address-picker data source = courier photos NOT satellite ("legally blocked by tile-ToS, wrong input shape for multi-view GS, economically backwards"); Stage-1 = 2D pin via geo.rs; explicit-rejections list §5 | memory `gaussian-splatting-address-picker-arc-2026-07-16`; `docs/design/GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-2026-07-16.md` §2.1, §2.6, §5 | VERIFIED — §1.2 confirms it from an independent angle for THIS use case; nothing re-litigated |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Research verdicts (2026-07-18 web pass; every claim cited) — the operator's question answered

### 1.1 Free, open, non-vendor-locked geo-data — current status

**Vector data + services (all genuinely free/self-hostable):**

| Source | License / policy (2026) | Fit |
|---|---|---|
| OpenStreetMap raw data | ODbL 1.0. Attribution "© OpenStreetMap contributors" required, legible, corner-of-map for browsable maps (may collapse; must stay findable) — [osmfoundation.org/wiki/Licence/Attribution_Guidelines](https://osmfoundation.org/wiki/Licence/Attribution_Guidelines). Rendered maps = **Produced Works** (license as you like + attribution); an imported road graph = **derivative database** → share-alike on that layer if publicly used; own order/courier tables stay independent as a **Collective Database** provided OSM and non-OSM geometry are never mixed for the same feature type — [Produced Work guideline](https://osmfoundation.org/wiki/Licence/Community_Guidelines/Produced_Work_-_Guideline), [Collective Database guideline](https://osmfoundation.org/wiki/Licence/Community_Guidelines/Collective_Database_Guideline_Guideline) | **THE data source.** Vector road graph — effectively resolution-unbounded for street rendering (geometry, not pixels) |
| tile.openstreetmap.org | Best-effort, no SLA, "commercial services… access may be withdrawn at any point" — [operations.osmfoundation.org/policies/tiles](https://operations.osmfoundation.org/policies/tiles/) | **Rejected for production** (also: any tile server violates F12 offline canon) |
| Overpass API | Public instance soft-cap ~10k queries / 1 GB per day; self-host planet 200-300 GB disk, regional extracts cheap — [wiki: Overpass_API](https://wiki.openstreetmap.org/wiki/Overpass_API) | One-time per-venue extract within policy = fine; NOT a runtime dependency |
| Nominatim (geocoding) | Public: hard 1 req/s, **no autocomplete**, no bulk — [usage policy](https://operations.osmfoundation.org/policies/nominatim/). Self-host planet ≥1 TB disk / 128 GB RAM (country extracts far smaller); GPL-3.0 — [nominatim.org install docs](https://nominatim.org/release-docs/latest/admin/Installation/) | Deferred — Wave-0 is pin-first, not address-string-first (§2). Photon (Apache-2.0, autocomplete, ≥64 GB RAM planet — [github.com/komoot/photon](https://github.com/komoot/photon)) is the named self-host option if text search ever becomes a phase |
| Vector-tile tooling (Planetiler Apache-2.0, tilemaker FTWPL, PMTiles/Protomaps BSD+ODbL, Shortbread schema CC0) | All open, active 2026 — [Planetiler](https://github.com/onthegomap/planetiler), [tilemaker](https://github.com/systemed/tilemaker), [docs.protomaps.com](https://docs.protomaps.com/), [shortbread-tiles.org](https://shortbread-tiles.org/) | **Not needed.** Tiles solve planet-scale streaming; a delivery venue needs one compact regional pack (§3 MapPack). Named escape hatch only |

**Routing engines — honest comparison (the DECART table, §1.3 verdict):**

| Engine | License / language / status (2026) | Self-host shape | Map-matching | Verdict for THIS architecture |
|---|---|---|---|---|
| **kernel `router.rs`** (already landed) | in-repo, zero-dep Rust, tested | in-process, no server at all | polyline snap via `geo.rs` today; graph HMM = named future | **REUSE — wins by substrate.** City-venue graphs (10⁴-10⁵ nodes) are milliseconds for A*; runs identically native + WASM ⇒ F12 offline + mesh-node-local by construction |
| OSRM | BSD-2, C++; **revived** — v6.0.0 Dec 2025, monthly releases through v26.7.3 Jul 2026 — [github.com/Project-OSRM/osrm-backend](https://github.com/Project-OSRM/osrm-backend) | separate C++ server; planet car profile: ~415 GiB RAM extract / ~123 GiB serving; country cuts modest — [wiki: Disk-and-Memory-Requirements](https://github.com/Project-OSRM/osrm-backend/wiki/Disk-and-Memory-Requirements) | `/match` HMM, confidence-scored | Rejected Wave-0: an out-of-substrate C++ service reintroduces a server dependency the F12 canon just removed. Named fallback for planet-scale matrix work (doesn't exist here) |
| Valhalla | MIT, C++, active (v3.8.2 Jul 2026) — [github.com/valhalla/valhalla](https://github.com/valhalla/valhalla) | tiled, low-memory, dynamic runtime edge costing | Meili `trace_route`/`trace_attributes`, per-edge attribution | Rejected Wave-0, same substrate reason. **The named fallback if dynamic costing (live traffic) ever becomes a requirement** — the honest boundary where in-kernel static weights stop being enough |
| GraphHopper | core + map-matching module Apache-2.0, Java; commercial = their hosted API (v11.0 Oct 2025) — [github.com/graphhopper/graphhopper](https://github.com/graphhopper/graphhopper) | JVM service | Apache-2.0 module | Rejected: JVM substrate, open-core gravity toward the hosted API |
| Rust crates: `fast_paths` (CH, MIT/Apache-2.0, quiet since v1.0 2024), `rust_road_router` (KIT CCH research, BSD-3) | — [github.com/easbar/fast_paths](https://github.com/easbar/fast_paths), [github.com/kit-algo/rust_road_router](https://github.com/kit-algo/rust_road_router) | libraries | no | Noted: `router.rs::build_shortcuts` already gives CH-lite in-repo; CCH is the research pointer if weight-update-at-CH-speed is ever needed |

**Satellite / aerial imagery — the operator's second branch, checked not assumed:**

| Source | Resolution / license (2026) | Street-render fitness |
|---|---|---|
| Copernicus Sentinel-2 | 10 m/px visible bands, 5-day revisit; free full open, attribution "Contains modified Copernicus Sentinel data [Year]" — [Sentinel legal notice](https://sentinel.esa.int/documents/247904/690755/Sentinel_Data_Legal_Notice), [dataspace.copernicus.eu](https://dataspace.copernicus.eu/data-collections/copernicus-sentinel-missions/sentinel-2) | **A 10 m road = 1 pixel; a building = a 2×3-px blob.** Legally perfect, physically unusable at street zoom |
| Landsat 8/9 | 30 m (15 m pan), US public domain — [USGS](https://www.usgs.gov/faqs/are-landsat-data-cloud-still-considered-be-within-public-domain) | Coarser still |
| New free missions 2024-26 | NISAR = free but **radar**; Umbra open data = 25 cm but SAR + ~1k fixed sites CC-BY — [earthdata](https://www.earthdata.nasa.gov/news/now-that-nisar-launched-heres-what-you-can-expect-from-the-data), [open-data directory](https://spacefromspace.com/blog/the-open-data-directory-list-of-open-satellite-data-2026/) | **No free sub-meter optical mission exists** |
| Maxar/Vantor, Airbus Pléiades Neo, Planet SkySat | 30-50 cm, all paid; Maxar Open Data = **CC BY-NC, disaster-activation only** (sole carve-out: OSM tracing) — [maxar.com/open-data](https://www.maxar.com/open-data), [AWS registry](https://registry.opendata.aws/maxar-open-data/) | Paid or non-commercial ⇒ excluded by the operator's hard constraint |
| Government orthophotos (US NAIP 60 cm public domain; NL 7.5 cm CC-BY; FR/DK/…) | country patchwork, heterogeneous — [USGS NAIP](https://www.usgs.gov/centers/eros/science/usgs-eros-archive-aerial-photography-national-agriculture-imagery-program-naip) | No global free sub-meter mosaic exists |
| Google/Esri/Bing imagery tiles | ToS forbid offline use, bulk caching, derivative extraction — [GMP ToS](https://cloud.google.com/maps-platform/terms), [Esri export cap](https://www.arcgis.com/home/item.html?id=226d23f076da478bba4589e7eae95952), [Bing ToU](https://www.bingmapsportal.com/terms) | Look free, are vendor-locked — exactly what the operator excluded |
| OpenAerialMap | CC-BY commons, genuinely open — [openaerialmap.org/about](https://openaerialmap.org/about/) | Coverage sparse/event-driven; not a product substrate |

### 1.2 The falsifiable verdict on satellite-based route rendering

**REJECTED for Wave-0, on resolution physics + licensing economics — and the rejection is
falsifiable:** it flips if and only if a free, redistributable, commercially-usable, global
(or at least venue-coverage-guaranteed) **optical sub-meter** source appears. As of 2026-07
none exists (§1.1 third table — every row checked, none qualifies). A courier navigating a
route needs street/lane/building-outline detail; free global imagery tops out at 10 m/px where
a street is one pixel. OSM's road graph is **vector** — its render resolution is unbounded, its
cost is zero, its license is satisfiable by an attribution string and a share-alike on a pack
we are happy to publish (§4 M7).

**Reconciliation with prior art (not contradicted, extended):** the gaussian-splatting arc
rejected satellite for the *address-picker 3D-reconstruction bootstrap* on tile-ToS, input
shape, and economics (its §2.1). This pass re-ran the question for the *route/map render* use
case with fresh 2026 sources and reached the same NO from an independent angle — resolution
physics (10 m/px vs street-level need) plus the confirmed absence of any free sub-meter optical
feed. Two use cases, two independent argument chains, one conclusion — the same
triple-convergence pattern that arc itself flagged as strong signal. One honest nuance the
prior arc didn't need: Sentinel-2 at 10 m **is** legally and physically fine as a *city-scale
ambient backdrop texture* (a color wash under the vector map at far-out zoom). That is recorded
as an explicitly-deferred decorative option (§2 anti-scope), not scope.

### 1.3 The map-render approach (research question 3, answered honestly)

Three candidate architectures, judged:

1. **Bolted-on web map widget (Leaflet/MapLibre + tile server).** Rejected: violates the
   zero-visible-DOM canon (P38 §1), reintroduces a runtime tile-server dependency (F12), and
   duplicates a render stack beside `compose()`. This is the default industry answer and the
   wrong one for this codebase.
2. **Road network AS spectral/Laplacian layout** ("the graph lays itself out through the field
   engine"). Rejected — and this rejection matters because it *sounds* like the most
   field-native answer: a spectral embedding (φ₂/φ₃) preserves topology, **not geography**. A
   navigation map whose street positions are eigenvector coordinates instead of geographic
   coordinates is actively harmful to a courier. The geometry is already known exactly (OSM
   node coords, `RoadGraph.coords`); recomputing a worse layout from the Laplacian would be
   ritual math (the Anu/Ananke discipline forbids exactly this). The road graph's Laplacian
   remains genuinely useful for *dynamics on the graph* — ETA/isochrone diffusion fields via
   `Csr::laplacian_spmv` — which is a named future unit with no Wave-0 consumer, not scope.
3. **OSM vector geometry as Scene content of the existing field renderer** — **CHOSEN.** Roads
   become `SdfShape::LineSegment` strokes in the `Scene` the engine already renders
   (`scene.rs:43`); the planned route is added as a *source term*, so its glow is literally the
   diffusion field `compose()` already integrates (`field_frame.rs:218`) — the "physics-render
   of the route" the operator asked for, with zero new render math; the courier marker is a
   field point riding P38 G2's particle pipeline; order-state overlay (Sea) composes over the
   same frame because it *is* the same frame. One renderer, one scene, one operator —
   map, route, marker, and product field are layers of a single `compose()`/P38 pass, not a
   map widget under an overlay. This is simultaneously the honest answer AND the interesting
   one: it wins not by being clever but by the substrate already being shaped for it.

---

## 2. Scope — what P51 owns vs deliberately does NOT

**P51 owns (build items §4):**

| Item | Content |
|---|---|
| M1 | `tools/map-pack` extractor: Overpass-JSON venue extract → deterministic MapPack artifact (roads + building outlines + delivery-zone polygon) |
| M2 | `kernel/src/mappack.rs`: pure `&[u8]` → typed pack parse (fail-closed) → `road_graph_from_ways` |
| M3 | Pin-drop: `nearest_road_node` + snapped `GeoPin` + zone check — the DELIVERY address-selection gap (splatting Stage-1 supplier) |
| M4 | Route plan + render: `shortest_path` → route polyline → Scene layers (roads, route-as-source, marker) through `compose()`/P38 |
| M5 | Courier live tracking estimator: 2-D CV Kalman (kalman.rs config) + route snap + ETA + off-route re-route |
| M6 | Surfaces + wire: courier route display and customer live-track view as two consumers of one `TrackFrame`; wasm ptr/len exports; position event on the P34/P37 wire |
| M7 | ODbL compliance: rendered attribution, MapPack published under ODbL, collective-database invariant |

**P51 explicitly does NOT own:**

- **NO paid mapping/geocoding/imagery API — hard operator constraint, not a preference.** No
  Google/Mapbox/HERE even as fallback; a diff introducing one is a scope violation regardless
  of test state. The fallback for "no map data" is the honest degraded state (§5.1), never a
  vendor.
- **NOT turn-by-turn voice navigation** — AGENT/voice territory (DZ-10 stays Phase-9b deferred
  per P38 §1); P51 renders the route, it does not speak it.
- **NOT text-address geocoding/autocomplete** — Wave-0 is pin-first (the operator asked for
  pins). Self-hosted Photon/Nominatim is the named future unit if a text-search phase is ever
  opened (§1.1).
- **NOT the 3D address-picker Stage-2** — the gaussian-splatting arc owns it; P51's pin-drop is
  exactly its Stage-1 ("2D pin — geo.rs haversine/point_in_polygon — already buildable",
  synthesis §2.0/§2.6). No satellite re-litigation (§1.2).
- **NOT the customer identity/notification wrapper** — P49 owns it. **Overlap note (honest,
  P-A §1 pattern):** P49 DoD-4 ("live tracking view renders Kalman/EMA through P38a") is
  *supplied* by P51 M5+M6 — one implementation, two consumers; P49's blueprint must cite §4
  M5/M6 instead of re-specifying, and P49 keeps identity/notification scope untouched.
- **NOT the `Order` address field.** The same-day DELIVERY MVP audit
  (`docs/design/DELIVERY-MVP-FEATURE-COMPLETENESS-AUDIT-2026-07-18.md` §"Navigation/routing")
  hands P51 one seam: the kernel `Order` carries no address today (P13/P16-flagged). P51's
  `GeoPin` (§4.3) is the *value* that field will hold; adding the field to the order
  intent/fold is P13/P16's diff. Sequencing: M3 lands `GeoPin` + the zone gate now
  (self-contained, fixture-tested); M4-M6's end-to-end order-addressed tests go
  `#[ignore = "P13-address-field"]` until that seam closes — ignored-not-deleted, same
  honesty convention as P38's O18a markers.
- **NOT tile servers, not planet scale** — one venue = one MapPack (§3 budget). PMTiles et al.
  are the recorded escape hatch if a many-venues aggregation surface ever needs planet
  streaming (§1.1).
- **NOT live-traffic dynamic costing** — static metric weights Wave-0; Valhalla self-host is
  the pre-named fallback at that boundary (§1.1) so the decision is already made honestly if
  the need materializes.
- **NOT satellite texture decoration** — the Sentinel-2 ambient-backdrop option (§1.2) is
  deferred until an operator asks for it; recorded so it isn't re-researched.

---

## 3. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── kernel/src/mappack.rs — NEW module (pure parse; NO I/O — kernel law) ────
/// MapPack v1: little-endian, content-addressed regional map artifact.
///   header:  magic "DWMP" (4) · version u16 · flags u16 · fnv1a64 of payload (8)
///   payload: n_nodes u32 · nodes [i32 lat_micro7, i32 lng_micro7] × n
///            n_ways u32  · ways  [u32 a, u32 b, f32 len_m] × n
///            n_buildings u32 · building outlines as index-runs into nodes
///            zone: n_pts u32 · [i32,i32] × n   (delivery-zone polygon)
/// Coordinates are i32 units of 1e-7 degree (OSM native precision, ~11 mm) —
/// integer in the artifact ⇒ bit-stable across platforms; f64 only after load.
pub const MAPPACK_MAGIC: [u8; 4] = *b"DWMP";
pub const MAPPACK_VERSION: u16 = 1;
pub const MAPPACK_MAX_BYTES: usize = 8 * 1024 * 1024; // venue budget; §5.2 scaling axis
pub const MICRO7: f64 = 1e-7;                          // i32 → degrees

pub struct MapPack { pub nodes: Vec<(f64, f64)>, pub ways: Vec<(usize, usize, f64)>,
                     pub buildings: Vec<Vec<usize>>, pub zone: Vec<(f64, f64)> }
/// Typed refusal — NEVER a partial pack. Wrong magic/version/hash/truncation/
/// out-of-range index each name themselves.
pub enum MapPackError { BadMagic, BadVersion(u16), BadHash, Truncated, BadIndex, TooLarge }
pub fn parse_mappack(bytes: &[u8]) -> Result<MapPack, MapPackError>;
/// MapPack → RoadGraph via the EXISTING kernel ingestion (router.rs:228).
pub fn road_graph(pack: &MapPack) -> crate::router::RoadGraph;

// ── kernel/src/router.rs — ONE new fn beside route() ────────────────────────
/// Nearest graph node to (lat,lng) by haversine. Linear scan (§5.2 axis:
/// grid-bucket index at ≥1e5 nodes). Returns (node, distance_m).
pub fn nearest_road_node(g: &RoadGraph, lat: f64, lng: f64) -> Option<(usize, f64)>;

// ── kernel/src/track.rs — NEW module (estimator = kalman.rs configuration) ──
/// Local-meter frame around the venue anchor (equirectangular — the SAME
/// city-scale approximation geo.rs:78-81 already uses). State [x, y, vx, vy].
pub const TRACK_GPS_SIGMA_M: f64 = 15.0;   // R = diag(σ²) — consumer-GPS class
pub const TRACK_ACCEL_SIGMA: f64 = 2.5;    // m/s² — Q white-accel model
pub const TRACK_MAX_SPEED_MPS: f64 = 42.0; // sanity gate (~150 km/h) — reject, don't clamp
pub const OFFROUTE_K: u8 = 3;              // consecutive off-route samples before re-route

pub struct CourierTrack { /* KalmanFilter (4-state CV) + anchor + last_ts */ }
pub struct TrackSample { pub lat: f64, pub lng: f64, pub ts: i64 }
/// Event-shaped output (standard item 3: tests assert on these sequences).
pub enum TrackEvent { Updated { est: (f64, f64), v_mps: f64, eta_s: f64, remaining_m: f64 },
                      Snapped { seg: usize }, OffRoute { count: u8 },
                      RerouteNeeded { from_node: usize }, Arriving, Rejected(SampleReject) }
pub enum SampleReject { OutOfOrder, NonFinite, Teleport }
impl CourierTrack {
    pub fn new(anchor: (f64, f64), t0: i64, x0: (f64, f64)) -> Self;
    pub fn ingest(&mut self, s: TrackSample, route: &[(f64, f64)]) -> Vec<TrackEvent>;
}

// ── engine/src/map_layer.rs — NEW module (Scene assembly; render math = zero) ──
pub const ROAD_STROKE_M: f64 = 4.0;        // world-space half-width of a road stroke
pub const ROUTE_SOURCE_GAIN: f32 = 1.0;    // route line's field source amplitude
/// MapPack + route + marker → SdfShape lists for the EXISTING Scene (scene.rs:71).
/// Roads/buildings = passive geometry layer; route = source-term layer (its glow
/// is compose()'s diffusion); marker = particle seed for P38 G2.
pub fn map_scene(pack_roads: &[Vec<(f64, f64)>], route: &[(f64, f64)],
                 marker: Option<(f64, f64)>, viewport: &Viewport) -> Scene;
pub struct Viewport { pub center: (f64, f64), pub scale_m_per_unit: f64 }

// ── attribution (M7) — ONE authority, rendered on every map view ────────────
pub const OSM_ATTRIBUTION: &str = "© OpenStreetMap contributors";

// ── wasm/src/lib.rs — exports per P38 G7 ptr/len conventions ────────────────
// map_frame_ptr/len (scene-composed RGBA), route_json (path+cost — closes the
// P04 route_js gap, §0), track_ingest_js (sample in → events out)
```

Rejected alternatives (DECART one-liners): **f64 coords in the artifact** — rejected:
`to_bits` platform identity is what the integer form gives for free (P-A A5's canonical-NaN
lesson). **A new polyline SDF variant now** — rejected: N `LineSegment`s reuse the tested
variant; a fused `Polyline` SDF is the named perf step if §7's bench flags segment count.
**HMM map-matching Wave-0** — rejected: snap-to-planned-route + re-route covers the courier
flow; full-graph HMM (OSRM `/match` semantics) is the named future unit with the §1.1 citations
already gathered.

---

## 4. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

### 4.1 M1 — `tools/map-pack` extractor (out-of-kernel; serde_json already in-tree)

New bin crate `tools/map-pack`: input = an Overpass JSON export for the venue region (one-time
query, within the public-instance policy §1.1, cached as a committed fixture; self-hosted
Overpass named for scale), filters `highway=*` ways for the road graph, `building=*` outlines,
and the operator-drawn delivery-zone polygon; emits the §3 MapPack bytes. Way length =
`polyline_length_meters` over member nodes (the kernel fn is the single length authority —
the tool depends on the kernel crate, not a reimplementation). **Determinism law:** same input
JSON ⇒ byte-identical pack (node order = sorted OSM id; FNV-1a payload hash — same hash family
as `spectral_cache.rs`). RED→GREEN: `mappack_deterministic_bytes` (two runs, `assert_eq!`
bytes) against a committed small-town fixture. **Adversarial:** malformed JSON → typed error,
never a partial pack; a way referencing a missing node id → way dropped + counted in the
tool's stderr report (mirror of `Csr::from_edges`'s drop semantics, `csr.rs:79`), pack still
valid; coordinates outside ±90/±180 → refused (`BadIndex` class, not clamped).

### 4.2 M2 — `mappack.rs` pure parse + graph build (fail-closed)

Kernel-side `parse_mappack` per §3: verify magic → version → declared-size ≤
`MAPPACK_MAX_BYTES` → FNV-1a payload hash → bounds-check every index; any failure = typed
`MapPackError`, state untouched (the loader is Self-Termination-leg only, §5.4). `road_graph()`
converts i32 micro-degrees → f64 once and calls the existing `road_graph_from_ways`
(`router.rs:228`) — zero new graph code. RED→GREEN: round-trip test (M1 fixture → parse →
counts + spot coordinates match the JSON source). **Adversarial (designed to break):**
(i) single bit-flip anywhere in payload ⇒ `BadHash` (teeth: flip, assert Err, restore);
(ii) truncation at every 1/16 boundary ⇒ `Truncated`, never a panic (structured fuzz loop);
(iii) a crafted pack whose way indices exceed `n_nodes` ⇒ `BadIndex` from the parser — the
kernel's own drop-semantics in `Csr::from_edges` is deliberately NOT relied on as the only
guard (defense stated in §5.1); (iv) `MAPPACK_MAX_BYTES + 1` declared size ⇒ `TooLarge`
before any allocation.

### 4.3 M3 — pin-drop (the DELIVERY address-selection gap; splatting Stage-1 supplier)

`nearest_road_node` (§3) + the pin flow: customer drops `(lat,lng)` → zone check
`point_in_polygon(pt, pack.zone)` (`geo.rs:200`) → snap candidate = nearest node; if
`distance_m > SNAP_THRESHOLD_M` (`geo.rs:181` — the existing constant, reused not re-invented)
the pin keeps its raw position and the plan marks an unroutable tail (courier walks the last
stretch — honest, not fabricated). Output `GeoPin { raw, snapped_node, tail_m, in_zone }`.
RED→GREEN: fixture pins on the M1 town — on-street pin snaps to the expected node; park pin
keeps raw + `tail_m > 0`. **Adversarial:** pin outside the delivery zone ⇒ typed refusal (the
order intent never reaches `decide` with an out-of-zone address — event-sequence asserted);
pin exactly on the zone boundary (ray-cast edge case) pinned both sides; NaN/non-finite input
⇒ refused before any math.

### 4.4 M4 — route plan + field-rendered map layers

Route: `shortest_path(g, src_node, dst_node)` (`router.rs:164`) → node path → `(lat,lng)`
polyline via `RoadGraph.coords`. Render: `map_scene` (§3) builds three layers into ONE `Scene`
— (a) roads + building outlines as `LineSegment` strokes (passive geometry), (b) the route
polyline as **source-term** segments (`ROUTE_SOURCE_GAIN`) so `compose()`'s diffusion step
produces the route glow — the physics-render, by construction not by shader — and (c) the
marker as a particle seed. Viewport = equirectangular world→scene transform (same approx as
`geo.rs:78-81`; one authority comment cross-links them). RED→GREEN: (i)
`route_scene_deterministic_frame` — `compose()` over a fixture map+route is byte-identical
across two runs (extends the existing oracle discipline, `field_frame.rs:346` pattern);
(ii) `route_glow_localized` — frame pixels within k px of the route polyline carry ≥ the glow
intensity of pixels far from it (the diffusion actually renders the route). **Adversarial:**
unreachable destination (`route` → `None`) ⇒ typed `NoRoute`, surface renders the two pins +
straight `lerp_lat_lng` dashed hint labeled as no-road-path — a fabricated road route is
unrepresentable because only `route()`'s output ever reaches the route layer (§5.1);
disconnected-component fixture proves it; a route crossing the viewport edge clips, never
panics (clip test at all four edges).

### 4.5 M5 — courier live tracking (kalman.rs configuration + geo snap + re-route)

`CourierTrack::ingest` per sample: (1) gates — `is_out_of_order(last_ts, ts)` (`geo.rs:171`)
⇒ `Rejected(OutOfOrder)`; non-finite ⇒ `Rejected(NonFinite)`; implied speed >
`TRACK_MAX_SPEED_MPS` ⇒ `Rejected(Teleport)` (reject-don't-clamp: a clamped teleport poisons
the covariance silently); (2) KF predict (dt from timestamps, F per §3 CV model — the shape
`kalman.rs:392-420` already proves) + update; (3) snap: `progress_along_route(route, est)`
(`geo.rs:70`) + `should_snap` (`geo.rs:187`) ⇒ `Snapped`/`OffRoute` events; `OffRoute` count
reaching `OFFROUTE_K` ⇒ `RerouteNeeded { from_node: nearest_road_node(est) }` — the consumer
recomputes `shortest_path` from there (re-route is a *new M4 plan*, not special logic);
(4) `eta_seconds(remaining_m, total_m, baseline_s)` (`geo.rs:153`) + `is_arriving` with
`ARRIVE_THRESHOLD_M` ⇒ `Arriving` fires exactly once per approach. RED→GREEN
(event-sequence form, standard item 3): a synthetic drive along the fixture route asserts the
ordered event stream `[Updated×n, Snapped, …, Arriving]` — not just final state; a
deliberately-detoured trace asserts `[…, OffRoute×3, RerouteNeeded]`. **Adversarial:** GPS
noise burst (σ=50 m for 5 samples) must NOT emit `OffRoute` (the KF innovation absorbs it —
this is the test that fails if someone replaces the filter with raw-sample snapping);
duplicate timestamp ⇒ exactly one rejection, filter state unchanged (bit-compare covariance);
singular-S path returns the kalman.rs fail-closed arm (`kalman.rs:212` contract) — forced via
a degenerate R fixture.

### 4.6 M6 — surfaces + wire (courier display, customer view, wasm exports, position event)

One `TrackFrame { est, v_mps, eta_s, remaining_m, route_version }` derived from M5 events;
**two consumers, one implementation**: the courier surface (route + own marker + ETA — Sea
grammar, P38b DZ-08) and the customer live-track view (marker + ETA — supplies P49 DoD-4,
§2 overlap note). Wasm: ptr/len exports per §3 (P38 G7 conventions — frame-scoped views,
re-derive every frame; the detached-buffer rule is P38 §3.7's, cited not re-proven). Wire:
`CourierPositionUpdated { order_scope, lat_micro7: i32, lng_micro7: i32, ts }` — ≤ 32-byte
payload at 0.2-0.5 Hz (§5.3 mesh budget) riding P34/P37's existing event path; **privacy
invariant:** the event carries an order scope and is emitted ONLY between assignment-accept
and delivery-complete fold states — a position event outside an active assignment is
unrepresentable at the emit site, asserted by test (courier position is personal data; this
is the P50-auditable boundary). RED→GREEN: an end-to-end test drives M5 with a synthetic
trace and asserts both surfaces read the SAME `TrackFrame` bytes (no fork); customer-view
test consumes over the wire path. **Adversarial:** position event emitted in a
non-active-assignment state ⇒ the test that must stay RED-impossible (compile/emit-site
guard); wire consumer receives out-of-order position events ⇒ view monotonicity holds
(stale event dropped by ts — same `is_out_of_order` authority).

### 4.7 M7 — ODbL compliance as build items, not prose

(a) `OSM_ATTRIBUTION` rendered on every map-bearing view — via FE-06 MSDF text once P38 G3
lands; **until then** via the a11y-mirror DOM layer (the one legitimate DOM surface, P38
§3.6), so attribution is never gated on the GPU unlock; test: the a11y tree contains the
attribution node whenever a map frame is composed. (b) MapPack artifacts are pure-OSM-derived
⇒ published under **ODbL** alongside the product (share-alike honored by policy, §1.1) — the
repo carries `docs/design/CORE-ROADMAP-2026-07-17/` a MAPPACK-LICENSE note + the pack format
doc; the P50 legal audit gains a named row (this blueprint pre-answers it). (c)
**Collective-database invariant:** no proprietary geometry (venue positions, customer pins,
courier traces) is ever written INTO a MapPack — packs are OSM-only by construction (the M1
tool has no input channel for product data; stated as the structural guard). Adversarial:
a test greps the M1 tool's input surface for any product-data path — the guard is that none
exists to call.

---

## 5. Cross-cutting design obligations (items 6, 8, 9, 11-16)

### 5.1 Hazard-safety as math (item 6)

Reachability arguments, not prose: **corrupt map data cannot corrupt anything** — the parse
is total over `&[u8]` with typed refusal (M2's fuzz corpus makes "panic on hostile pack" a
tested-unreachable state), and a parsed pack is read-only input to routing/render — no path
from MapPack bytes to order/money state exists (the order intent carries a `GeoPin`, whose
zone gate is kernel-side). **Fabricated routes are unrepresentable:** the route layer's only
producer is `route()`'s `Some` arm (M4); the no-route state renders as an honestly-labeled
straight hint. **Estimator divergence is gated:** every sample passes reject-don't-clamp
gates before touching the filter; the KF's own singular-S arm fails closed
(`kalman.rs:220-223`). **Privacy:** position events are emit-site-scoped to active
assignments (M6) — the unsafe state (tracking outside a delivery) is structurally absent, not
policied. Money is untouched by this entire phase (ETA is seconds — `geo.rs:5`'s own law).

### 5.2 Schemas & scaling axes (item 8)

MapPack: sized for a venue delivery zone — ~10⁴-10⁵ road nodes ≈ 0.5-2 MiB, budget 8 MiB;
axis = nodes/pack; break points: `nearest_road_node` linear scan → grid buckets at ≥10⁵
nodes; plain Dijkstra/A* → `build_shortcuts` CH (already in-repo, off by default) when §7's
bench exceeds budget; pack > 8 MiB → per-district pack tiling (format field `flags` reserves
the bit). `CourierTrack`: O(1) state per active courier, axis = concurrent couriers, no break
point in sight (a 4-state KF is microseconds). Scene: axis = SDF segment count/viewport —
§7's bench guards it; fused-polyline SDF is the named step (§3 rejected-alternatives).

### 5.3 Isolation (item 11), mesh awareness (item 12), living memory (item 15)

Isolation: map data is a content-addressed static asset; the extractor tool is
authoring-time only (P-A's eqc-rs precedent — a tool defect reaches the product only through
a committed, hash-verified, test-gated artifact). Renderer stays a state consumer (P38 §4.3's
bulkhead, inherited). Mesh: MapPacks are **fetched as static assets** over P37's HTTP surface
or pre-bundled — NOT gossiped (a MiB-class blob has no business in the SyncFrame path);
position events are the only mesh-borne payload: ≤ 32 B at ≤ 0.5 Hz per active courier,
stated budget. Living memory: packs are content-addressed (recall by hash — the same
content-not-location principle as the spectral cache); superseded packs demote, never
in-place mutate (a route computed on pack `h1` names `h1` — `route_version` in `TrackFrame`).

### 5.4 Rollback / self-healing vocabulary (item 13, used precisely)

**Self-Termination leg claimed:** typed `MapPackError`/`SampleReject`/`NoRoute` refusals;
emit-site privacy scoping; reject-don't-clamp gates. **Self-Healing leg claimed narrowly:**
re-route (M5) is genuine error-correction — the plan regenerates from current truth when
reality diverges — claimed for the route plan only, not for state. **Snapshot-Re-entry: NOT
claimed** (frames and plans are derived; recovery = recompute from kernel state + pack, which
is re-derivation). Mechanical rollback: every module is additive (`mappack.rs`, `track.rs`,
`map_layer.rs`, one fn in `router.rs`, tool crate) — deletion restores today's tree.

### 5.5 Linux discipline (item 9) + tensor/spectral/eqc (item 16)

Verdicts per the adoption framework: **ALREADY-EQUIVALENT** — one length authority
(`polyline_length_meters` shared by tool and kernel), one snap-threshold authority (geo.rs
constants reused); **REINFORCES** — stable on-disk format with magic/version/hash
(kernel-module discipline for data); **EXTENDS** — the artifact-determinism law (same input ⇒
byte-identical pack) as a new gate class for data tooling; **GAP** honestly named — no
Overpass self-host exists; Wave-0 depends on one-time public-instance extracts within policy
(acceptable at venue cadence; self-host is the named step at fleet scale). Item 16: spectral
machinery is deliberately NOT decoratively invoked (§1.3's rejection of spectral layout);
`laplacian_spmv` isochrone fields are named-future-only. eqc-rs: haversine needs `asin`
(P-A A1 adds it — sequencing note: if A1 has landed, the M1 tool's length math may be
eqc-generated; if not, the hand-written kernel fn remains the single authority — either way
one authority, no fork).

---

## 6. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| M1 | no tool; determinism test absent | `mappack_deterministic_bytes` on the town fixture; drop-report on dangling refs | determinism test |
| M2 | `parse_mappack` absent; bit-flip corpus RED by construction | round-trip green; bit-flip ⇒ `BadHash`; truncation fuzz panic-free; oversize refused | fuzz corpus + hash-teeth test (ledger row) |
| M3 | no `nearest_road_node`; pin tests absent | on-street snap, park tail, out-of-zone refusal, boundary pins pinned | zone-refusal test |
| M4 | no scene layers; `route_scene_deterministic_frame` RED vs stub | byte-identical frame; glow-localized; `NoRoute` honest-fallback fixture | frame oracle + no-fabricated-route test (ledger row) |
| M5 | `CourierTrack` absent; event-sequence tests RED | drive/detour sequences exact; noise-burst no-false-offroute; teleport/out-of-order rejected bit-stable | event-sequence + noise-burst tests (ledger row) |
| M6 | zero wasm geo exports (§0); no position event | both surfaces read one `TrackFrame`; wire round-trip; emit-site privacy test | privacy-scope test (ledger row) |
| M7 | no attribution anywhere | attribution node present whenever a map frame composes; MAPPACK-LICENSE note committed; P50 audit row named | attribution presence test |

Not-done clauses: any paid-API import = NOT done regardless of green totals (§2 hard
constraint); attribution gated behind the GPU unlock = NOT done (the a11y-mirror path exists
now); a clamped (rather than rejected) teleport sample = NOT done.

---

## 7. Benchmark plan (item 10) — existing harness, four benches, zero new infrastructure

Criterion harness + `bench_track`/`native-trackers` baseline discipline (P-A §6, verified
there): add `router/shortest_path_city_10k` (synthetic 10⁴-node grid+diagonals graph —
target < 10 ms, the "in-kernel router suffices at venue scale" claim made falsifiable; CH
shortcuts engage only if this regresses past budget), `router/nearest_node_50k` (< 1 ms
linear-scan headroom check for the §5.2 break point), `mappack/parse_1mib` (< 50 ms load
budget), `map_layer/scene_2k_segments` (CPU compose cost of a real viewport — must fit P38
§6's frame split; this is the bench that triggers the fused-polyline step if RED). All added
RED-commit-first so baselines auto-seed; results to `BENCH_HISTORY.md`, never prose
estimates. Telemetry: route-query latency + pack-parse counters ride the existing
native-trackers hooks (P-H's lane), so a data-growth regression surfaces without review.

---

## 8. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.3 + §11 (P37/P38/P49 seams; §12
entry) · `BLUEPRINT-P38-webgpu-render-engine.md` (render substrate; G2/G3/G7 conventions
consumed) · `sovereign-roadmap-2026-07-16/BLUEPRINT-P04-kernel-product-math.md` (router
provenance; its unlanded `route_js` line closed here) ·
`BLUEPRINT-P47-P50-gap-closing-phases.md` (P49 overlap; P50 audit row) ·
`GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-2026-07-16.md` §2.0/§2.6/§5 (Stage-1 supplied;
rejections honored) · `docs/regressions/REGRESSION-LEDGER.md` (four rows named in §6) ·
`HERMETIC-ARCHITECTURE-PRINCIPLES.md` (§9). Memory:
`gaussian-splatting-address-picker-arc-2026-07-16` (prior-art reconciliation, §1.2) ·
`physics-ui-capture-quantum-math-arc-2026-07-14` (ONE Laplacian/field operator — honored by
rendering through compose, and by refusing decorative spectral layout) ·
`rust-native-bare-metal-decision-2026-07-14` (DECART tables §1; Rust-default, older =
adapters) · `anu-ananke-strict-discipline-feedback-2026-07-17` (style; the §1.3-2 rejection) ·
`verified-by-math-2026-07-07` · `never-bypass-human-gates-2026-06-29` (no red-lines touched —
none needed). Supersedes: nothing — additive; feeds P49 DoD-4 and closes P04's wasm line.

---

## 9. Hermetic principles honored (item 20 — load-bearing only)

- **P1 MENTALISM** (spec is source): the MapPack format §3 precedes the tool; the artifact is
  derived, hash-pinned, regenerable from source data.
- **P2 CORRESPONDENCE** (one concept, one primitive): one router (`router.rs`), one length
  authority, one snap threshold, one attribution constant, one `TrackFrame` for both
  surfaces, one renderer for map+route+product field.
- **P6 CAUSE-AND-EFFECT** (determinism as law): byte-identical packs, byte-identical composed
  frames, event-sequence tests, integer coordinates in the artifact — every determinism claim
  carries a falsifier (§4, §6).
- **P7 GENDER** (paired verification, no self-certification): the tool's output is refereed
  by the kernel's independent parse + round-trip against source JSON; the estimator is
  refereed by geo.rs's independent snap math; the render is refereed by the compose oracle;
  legal claims are explicitly NOT self-certified (M7 defers judgment rows to P50/operator).

(P3/P4/P5 not load-bearing here; not claimed decoratively.)

---

## 10. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites; the router.rs finding; the wasm-gap finding) |
| 2 DoD | §6 |
| 3 spec/event-driven TDD | §3 spec-first; §4 RED-first; §4.5/§4.6 event-sequence assertions |
| 4 predefined types/consts | §3 |
| 5 adversarial/breaking tests | §4.1-4.7 (bit-flip, truncation fuzz, boundary pins, disconnected graph, noise burst, teleport, privacy emit-site) |
| 6 hazard-safety as math | §5.1 |
| 7 links docs/memory | §8 |
| 8 scaling axes | §5.2 (each with named break point) |
| 9 Linux discipline | §5.5 (all four verdict classes incl. an honest GAP) |
| 10 benchmarks+telemetry | §7 |
| 11 isolation/bulkhead | §5.3 |
| 12 mesh awareness | §5.3 (asset-vs-gossip split; 32 B / 0.5 Hz budget) |
| 13 rollback/self-heal vocabulary | §5.4 (two legs claimed precisely, one refused) |
| 14 error-propagation gates | §6 (named ledger rows), §5.1 (typed refusal classes) |
| 15 living memory | §5.3 (content-addressed packs, demote-never-mutate) |
| 16 tensor/spectral + eqc reuse | §5.5 (spectral honestly NOT invoked; eqc sequencing note) |
| 17 regression ledger | §6 (four rows) |
| 18 agent-executable instructions | §11 |
| 19 reuse-first | §0/§1.3 (router/geo/kalman/scene/compose all reused; three rejected alternatives in §3; engines rejected with cited comparison §1.1) |
| 20 Hermetic citations | §9 |

---

## 11. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Order below is the dependency order; T1-T5 are buildable today with zero network (serde_json
is cached, §0); nothing in this phase waits on O18a except the MSDF leg of T7 (which has a
stated non-GPU fallback).

1. **T1 (M2 first — the format is the contract).** Create `kernel/src/mappack.rs` per §3
   (types verbatim), register `pub mod mappack;` in `kernel/src/lib.rs` (alphabetical, near
   `markov`). Write the RED tests first: hand-build a 4-node/3-way pack in bytes, round-trip;
   bit-flip teeth; truncation fuzz loop; oversize refusal. Acceptance:
   `cargo test -p dowiz-kernel mappack` green.
2. **T2 (M1).** New bin crate `tools/map-pack` (serde_json + path-dep on kernel). Commit a
   small-town Overpass-JSON fixture (query: `highway=*` ways + `building=*` in a ~1 km bbox;
   record the query string in the tool's README). Implement extract → §3 bytes; run twice,
   `mappack_deterministic_bytes` asserts identity; dangling-ref and bad-coord adversarial
   fixtures. Acceptance: tool test green; T1's parser accepts the tool's output (round-trip).
3. **T3 (M3).** Add `nearest_road_node` to `kernel/src/router.rs` (§3 signature, beside
   `shortest_path` at `:164`); pin flow tests per §4.3 using T2's fixture pack (reuse
   `SNAP_THRESHOLD_M`/`point_in_polygon` — do NOT define new thresholds). Acceptance:
   `cargo test -p dowiz-kernel router` green including boundary pins.
4. **T4 (M5).** Create `kernel/src/track.rs` per §3 (KF config per the CV model
   `kalman.rs:392-420` proves; local-meter frame per `geo.rs:78-81`'s approximation).
   Event-sequence tests per §4.5 — write the drive/detour/noise-burst/teleport fixtures
   BEFORE the impl (RED), then implement. Acceptance: `cargo test -p dowiz-kernel track`
   green; covariance bit-stability on rejected samples asserted.
5. **T5 (M4 CPU).** Create `engine/src/map_layer.rs` per §3; `route_scene_deterministic_frame`
   + `route_glow_localized` + `NoRoute` fallback + viewport-clip tests per §4.4 (compose
   oracle pattern from `engine/src/field_frame.rs:346`). Add the four §7 benches
   RED-commit-first. Acceptance: `cargo test -p engine map_layer` green; baselines seeded.
6. **T6 (M6).** Wasm exports per §3 (follow P38 G7 ptr/len + frame-scoped-view rules
   exactly); `TrackFrame` single-source test; `CourierPositionUpdated` event + emit-site
   privacy guard on the P34/P37 path (coordinate with those phases' owners — the event rides
   their wire, it does not fork one). Acceptance: wasm tests green; privacy test green.
7. **T7 (M7).** Attribution via the a11y-mirror path NOW (P38 §3.6's module; add the
   attribution node + presence test); MSDF render swap-in when P38 G3 lands (leave a named
   TODO keyed "P38-G3", not a silent gap). Commit the MAPPACK-LICENSE note (ODbL, §4.7b) and
   add the P50 audit row + the four §6 ledger rows to `docs/regressions/REGRESSION-LEDGER.md`.
   Acceptance: attribution test green; ledger rows present.
