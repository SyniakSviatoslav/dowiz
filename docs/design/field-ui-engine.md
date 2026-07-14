# field-ui-engine — Delivery field-simulation under the canonical Rust/WASM kernel

> Scope: the **delivery field-simulation** = courier-marker kinematics (lerp + bearing +
> snap + progress + ETA over a live ping stream) and **delivery-zone** point-in-polygon
> check, rendered on a map. This is a *narrow* sub-problem of the broader "field drives
> the whole UI" vision (see `docs/design/field-ui-engine/RESEARCH-CONSPECT.md`, the
> bebop2 spectral/fluid substrate). This doc is the grounded port plan: move the
> deterministic geo/route math into the kernel and expose it on the wasm surface so the
> Astro frontend never re-implements float physics in TS.
>
> Standing invariants honored: build DOWN from the first real order, not UP from protocol;
> gates are falsifiable (RED→GREEN), not calendar dates; the **kernel (Rust/WASM) is the
> canonical core** and the TS app is the legacy oracle.

---

## 1. Audit — what exists vs what is kernel-authoritative

### 1.1 Kernel side (authoritative, present)

`kernel/src/geo.rs` (RW-06) is a 1:1 pure port from `geo-anim.ts` + `delivery-zone.ts`,
zero DOM, RED→GREEN tested. It already owns:

| Kernel fn | TS oracle fn | Status |
|---|---|---|
| `haversine_meters` | `haversineMeters` | ✅ ported, tested |
| `lerp_lat_lng` | `lerpLatLng` | ✅ ported, tested |
| `bearing_deg` | `bearingDeg` | ✅ ported, tested |
| `ema_next` | `emaNext` | ✅ ported, tested |
| `should_snap` | `shouldSnap` | ⚠️ signature differs (see 2.1) |
| `is_arriving` | `isArriving` | ⚠️ signature differs (see 2.1) |
| `point_in_polygon` | `delivery-zone.ts` ray-cast | ✅ ported, tested |
| `progress_along_route(t)` | `progressAlongRoute(polyline,pos)` | ❌ **wrong shape** — kernel is a bare `t.clamp(0,1)`; TS does full polyline projection |
| — | `polylineLengthMeters` | ❌ **missing** |
| — | `isOutOfOrder` | ❌ **missing** (out-of-order ping guard) |
| — | `etaSeconds(rem,total,baseline)` | ⚠️ kernel `eta_seconds(dist,speed)` is a different signature |

**Conclusion:** the kernel is the *intended* authority for this math, and ~70% is already
ported and tested. The remaining 30% (polyline projection, polyline length, out-of-order
guard, ETA-by-route-pacing) is the real work, plus a wasm surface.

### 1.2 Frontend side (legacy oracle, NOT canonical)

- `packages/ui/dist/lib/geo-anim.{js,d.ts}` — **compiled only**; the `src/lib/geo-anim.ts`
  source is **no longer in-tree** (only `dist/` artifacts remain). The oracle source that
  `geo.rs` was ported from is gone; `geo.rs` is now the de-facto reference.
- `packages/ui/dist/hooks/use-courier-marker.js` — the rAF glue. It imports
  `bearingDeg, isOutOfOrder, lerpLatLng, shouldSnap` from `geo-anim.js` and runs the
  per-frame tween (lerp only, no forward extrapolation), drops out-of-order pings, snaps
  on first-fix/large-jump, rotates the icon by bearing, pauses on `document.hidden`.
  **This hook is the only consumer that still depends on TS geo math.**
- `attic/apps-api/tests/geo-anim-g2.test.ts` — the only surviving parity fixture
  (G2: haversine/lerp/bearing/ema/progress/polyline/eta/snap/arriving). It is the
  acceptance oracle for the port — promote it to the kernel test suite.
- `apps/web` — legacy React SPA (per `RESEARCH-CONSPECT.md`); the canonical `web/`
  (Astro/Svelte) exists only as `dist/` builds in this tree, no source.

### 1.3 wasm side (the gap)

`kernel/src/wasm.rs` exports JSON-RPC control-plane fns only:
`place_order_js`, `apply_event_js`, `channel_ledger_js`, `reduce_anomalies_js`,
`estimate_order_total_js`, `fsm_graph_report_js`. **No geo/route function is exposed.**
Because the kernel's geo math is unreachable from JS, the frontend is forced to keep a
parallel TS copy (`geo-anim.js`) — the exact drift the kernel was meant to eliminate.

---

## 2. Port plan — complete the kernel geo math, then expose it

### 2.1 Close the kernel gaps (no new modules; extend `geo.rs`)

Add the missing pure functions to `kernel/src/geo.rs` (mirroring `geo-anim.d.ts`):

- `polyline_length_meters(poly: &[(f64,f64)]) -> f64` — sum of haversine segments.
- `progress_along_route(poly: &[(f64,f64)], pos: (f64,f64)) -> RouteProgress`
  where `RouteProgress { remaining_m: f64, snapped: (f64,f64), segment_index: usize }`.
  Local equirectangular projection (city-scale), project `pos` onto each segment, pick the
  closest, return the snapped point + remaining metres to the end.
- `is_out_of_order(last_ts: Option<i64>, ts: i64) -> bool` — strictly-older rejection.
- `eta_seconds(remaining_m, total_m, baseline_s) -> f64` — pace by route average speed
  `total_m/baseline_s`, fall back to ~5 m/s urban speed when `baseline_s<=0`.
- Reconcile signatures so the kernel matches the oracle:
  - `should_snap(prev: Option<(f64,f64)>, next: (f64,f64), threshold_m: f64) -> bool`
    (kernel currently takes `(distance_m, threshold_m)` — change to lat/lng pair, or add an
    overload; keep `haversine_meters` as the distance primitive).
  - `is_arriving(remaining_m: f64, threshold_m: f64) -> bool` — already 2-arg; default
    `threshold_m = 150.0` to match `ARRIVE_THRESHOLD_M`.

Do **not** port the rAF glue (`use-courier-marker`) into Rust. The rAF loop is browser
bound (requestAnimationFrame / visibilitychange) and stays a thin TS hook — it should call
the kernel for *every math step* but own the loop.

### 2.2 Add a geo surface to `wasm.rs`

Follow the existing thin-wrapper pattern (`*_logic` pure fn + `#[wasm_bindgen] *_js`). All
numeric in/out, plain JSON strings (control-plane style, fine at ping frequency — not
60 fps field sim). Add:

- `geo_haversine_js(a_lat,a_lng,b_lat,b_lng) -> f64`
- `geo_lerp_js(a_lat,a_lng,b_lat,b_lng,t) -> {lat,lng}`
- `geo_bearing_js(a_lat,a_lng,b_lat,b_lng) -> f64`
- `geo_progress_js(poly_json, pos_lat, pos_lng) -> {remaining_m, snapped:{lat,lng}, segment_index}`
- `geo_eta_js(remaining_m,total_m,baseline_s) -> f64`
- `geo_should_snap_js(prev_json, next_json, threshold_m) -> bool`
- `geo_is_arriving_js(remaining_m, threshold_m) -> bool`
- `geo_point_in_polygon_js(pt_lat,pt_lng, polygon_json) -> bool`
- `geo_is_out_of_order_js(last_ts, ts) -> bool`

Wire them through the same `Result<String,JsValue>` / `Result<...,JsValue>` boundary the
module already uses; no new serde shapes beyond a small `PolylineIn`/`PosOut`. Keep the
logic fns host-testable (the existing `mod tests` proves the pattern).

---

## 3. wasm boundary contract — pure function of kernel state

**Principle: the field animation is a pure function of (ping stream, route polyline,
zone polygon).** Given the same inputs it must reproduce the same marker path, bearing,
ETA, and zone membership bit-for-bit. No wall-clock, no RNG, no `Date.now()` inside the
kernel — the TS hook passes `recorded_at` timestamps and the current `t` (ping elapsed
fraction) in. This makes the animation **replayable and Verifiable-by-Math**: record the
ping stream once, replay it through the kernel, assert the marker trace equals the fixture.

Cross-boundary shapes (all JSON/number, no structs leak):

| Direction | Shape |
|---|---|
| hook → kernel (per ping) | `geo_progress_js(poly_json, posLat, posLng)` + `geo_eta_js(...)` + `geo_bearing_js(from,to)` |
| hook → kernel (per frame) | `t = (now - start)/pingIntervalMs` computed in TS, fed to `geo_lerp_js(from,to,t)` |
| hook → kernel (zone) | `geo_point_in_polygon_js(ptLat,ptLng, polygonJson)` |
| kernel → hook | `{lat,lng}` + `bearing: f64` + `remaining_m: f64` + `eta_s: f64` + `arriving: bool` |

The hook keeps only:
1. a `requestAnimationFrame` loop and `document.hidden` pause/resume,
2. `from`/`to`/`start` refs and the `last_ts` out-of-order guard,
3. calling the kernel for `lerpLatLng`/`bearingDeg`/`shouldSnap`/`progressAlongRoute`/
   `etaSeconds` instead of `geo-anim.js`.

So the TS dependency on `geo-anim` drops to zero and `packages/ui/dist/lib/geo-anim.*` can
be deleted after the hook is repointed. The marker trace is then deteterministic by
construction — replay the recorded ping array through the kernel and you get the identical
trace, which is the falsifiable gate (§5).

---

## 4. Dependency ordering — keep bebop2 protocol code parked

**`bebop-repo/` and `rust-core` (the bebop2 `field_build`/`sinc` spectral/heat-kernel
core) are NOT present in this workspace.** They were referenced in the task brief but do
not exist in `/root/dowiz` here (verified: `find . -name 'field_build'` returns nothing;
no `bebop-repo` dir; no `rust-core` crate). The broader vision in
`RESEARCH-CONSPECT.md` (field_physics damped-wave, spectral layout, GPU/wgpu) is a
**separate research substrate** and is explicitly out of scope for this doc.

Operator invariant: **bebop protocol work stays parked until dowiz carries it.** Therefore:

- dowiz's delivery field-sim reuses the **kernel `geo.rs` math only** — haversine/lerp/
  bearing/progress/ETA/zone. These are plain kinematics, not spectral fields.
- We do **not** pull `rust-core::field_build` (CSR adjacency → L=D−A → heat-kernel) or
  `sinc` into the product. The courier marker is a 2-point lerp + polyline projection, not
  a Laplacian eigenproblem. Importing the bebop2 field engine here would violate the
  "build down from the first real order, keep protocol work parked" rule and add a
  research core the product does not need.
- If, later, dowiz carries a genuine spectral need (e.g. graph layout of delivery zones),
  *then* the dependency is opened and `field_build` is reconsidered — behind its own gate.

---

## 5. Acceptance gates (falsifiable, RED→GREEN)

1. **G-geo-port** — `kernel/src/geo.rs` gains `polyline_length_meters`,
   `progress_along_route(poly,pos)`, `is_out_of_order`, `eta_seconds(rem,total,base)`;
   RED test fails before, GREEN after.
2. **G-wasm** — `wasm.rs` exposes the 9 geo fns above; host tests assert JSON round-trip
   and numeric equality vs the oracle values.
3. **G-parity** — promote `attic/apps-api/tests/geo-anim-g2.test.ts` expectations into
   `kernel/src/geo.rs` `#[cfg(test)]` (haversine 1°≈111195 m, bearing cardinals, lerp
   clamp, ema seed, progress midpoint/remaining monotonic, eta pacing 1000 m→200 s,
   snap 33 m vs 1.1 km, arriving 150 m). Kernel output == TS-oracle within tolerance.
4. **G-replay** — a recorded ping stream (fixture) replayed through the wasm geo fns
   yields a byte-identical marker trace + ETA + zone membership; serves as the
   Verified-by-Math evidence and a regression lock.
5. **G-detach** — `use-courier-marker` no longer imports from `geo-anim`;
   `packages/ui/dist/lib/geo-anim.*` removable; `shouldSnap`/`isArriving` semantics
   unchanged (150 m arrive threshold, 500 m snap default preserved).

---

## 6. What was NOT done (constraints)

- No Rust source was modified (doc only). The port in §2 is the proposed change set, not
  applied.
- No git/commit.
- bebop2 `field_build`/`sinc` left parked (absent from tree; out of scope per §4).
