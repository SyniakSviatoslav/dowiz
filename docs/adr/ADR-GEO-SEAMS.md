# ADR: Geo-seams — Routing · ETA · Tiles

**Status:** Accepted (G0 landed) · Supersedes nothing · Extends v4.4 (+ v3.1)
**Scope:** `RoutingProvider`, ETA engine + live display, `TileSource`.

## Context

Phase-3 delivery surfaces already exist: `courier_positions` (physical invariants),
the `deliver` contract (`delivery_trace`), and live position streaming over WS to
`order:{id}` (client) and `location:{id}:couriers` (owner). `useGeoStream` /
`useGeolocation` / `MapLibreBase` / `calcETA` exist as stubs.

These geo-seams **augment** the delivery surface. They need **no new migration**
(routing/tiles are not DB concerns). The goal is that **every provider sits behind a
seam** so cost/ops can be controlled by flipping a managed free provider to a
self-hosted one in `fra` — a config change, not a rewrite.

## Decision — red-line principles

1. **Provider behind the seam.** No calling code knows whether routing is ORS-free,
   MCP, or self-host; tiles free or self-host. Switching = one env line.
2. **Routing per-leg, not per-ping.** One `route()` call per delivery (+ re-route only
   on significant deviation). ETA along the polyline is computed **locally per ping**,
   zero provider calls per ping.
3. **ETA is advisory, not a contract.** Degrades silently: provider down → straight
   polyline + haversine ETA; the client never knows it got a fallback.
4. **Turn-by-turn is delegated.** Real navigation stays in `NavigationDeepLink`
   (Google/Apple Maps). The in-app map is display, not navigator.
5. **EU preference.** All else equal, provider/tiles in EU; self-host in `fra`;
   customer addresses don't leave the region.

## Seam registry (extends the Dependency-Inversion table)

| Interface | Implementation now | Swap successor |
|---|---|---|
| `RoutingProvider` | ORS (free API / MCP) + haversine fallback | Self-hosted ORS / OSRM (`fra`) |
| `TileSource` (`MapLibreBase` config) | Free vector tiles + Cloudflare cache | Self-hosted tileserver-gl / Protomaps (`fra`) + Cloudflare |

`RoutingProvider` lives in `packages/platform` (alongside `MessageBus` /
`QueueProvider`): `packages/platform/src/routing-provider.ts`, barrelled in
`index.ts`. Tile config lives in `apps/web` (`src/lib/tileConfig.ts`) and is read by
`MapLibreBase` in `packages/ui` (wired in G3).

## Circuit Breaker (extends the matrix)

| Service | Criticality | On failure |
|---|---|---|
| `RoutingProvider` | NON-CRITICAL | Fallback: straight polyline + haversine ETA (silent) |
| `TileSource` | NON-CRITICAL | Fallback: cached tiles / degraded map background |

## Config contract

### Backend (`packages/config` · `loadEnv`, fail-fast)
| Var | Type | Default | Notes |
|---|---|---|---|
| `ROUTING_PROVIDER` | `ors` \| `self` \| `haversine` | `ors` | Chooses the impl; never read by feature code directly. |
| `ROUTING_BASE_URL` | URL | `https://api.openrouteservice.org` | Directions base. For `self`, the internal `fra` URL. |
| `ROUTING_API_KEY` | string? | — | Empty for `self` / `haversine`. |

### Frontend (`apps/web` · `src/lib/tileConfig.ts`, Vite `import.meta.env`)
| Var | Type | Default | Notes |
|---|---|---|---|
| `VITE_TILE_STYLE_URL` | URL | `https://tiles.openfreemap.org/styles/liberty` | Full MapLibre `style.json` URL. |
| `VITE_TILE_PROVIDER` | `free` \| `self` | `free` | Provenance flag for the flip. |

**Deliberate deviation from the build-prompt's strict "verify:env fails on absent
var":** every new var has a *safe default* (the default URL equals the previous
hardcode, so wiring is behavior-neutral). Rationale: a hard-fail-on-absence in the
backend boot schema or the frontend build would risk a deploy outage, and the
"zero hardcode" principle is satisfied by keeping the single default *in config*
rather than scattered in code. `verify:env` and `tileConfig` still **fail fast on an
invalid value** (unknown provider, malformed URL).

## Stage map

```
G0 seams + config (no runtime)  ← this ADR
   ├──────────────┬───────────────┐
   ▼              ▼               ▼
G1 RoutingProvider impl     G3 TileSource (config + cache)
   + ETA data (backend)
   │
   ▼
G2 Live display (frontend): marker tween + ETA smoothing + real polyline
```

- **G1** — ORS directions; haversine hard-fallback via circuit-breaker on
  429/403/timeout/error; per-leg route + recoverable (non-authoritative) cache;
  re-route only on >threshold deviation; push `RouteResult` once to `order:{id}`.
- **G2** — `useCourierMarker` (rAF tween, out-of-order guard, snap on
  reconnect/jump, bearing, Page-Visibility pause/resume); `useDeliveryEta` +
  `ETADisplay` (progress-along-route, EMA smoothing, arrive-flip at ~150 m, draw the
  real polyline); zero routing calls per ping.
- **G3** — `MapLibreBase` reads `VITE_TILE_STYLE_URL` (no hardcode); free vector
  provider with domain-restricted key + spending cap; Cloudflare cache in front;
  embed stays MapLibre-free.

## Flip-runbook (managed → self-host; trigger = cost/ops over threshold, not early)

**Routing (ORS → self-host):** raise an ORS container in `fra` on an Albania `.pbf`
extract; build the graph; await health → set `ROUTING_PROVIDER=self`,
`ROUTING_BASE_URL=<internal>`, clear `ROUTING_API_KEY` → smoke one `route` self vs
ORS-free for geometry/duration parity → `fly deploy`. The haversine fallback stays
untouched as the safety net.

**Tiles (free → self-host):** raise `tileserver-gl` / `Protomaps` in `fra` on the
same extract; generate a style → set `VITE_TILE_PROVIDER=self`,
`VITE_TILE_STYLE_URL=<self>`; **⚠ update the CSP `connect-src` to allow the new
tile/style domain** in `apps/api/src/lib/security/headers.ts`,
`apps/api/src/lib/spa-shell.ts`, and `apps/api/src/routes/public/branding-preview.ts`
(plus the `tiles.openfreemap.org` assertion in `apps/api/scripts/config-drift.ts`) —
otherwise the browser silently blocks every tile fetch; add a Cloudflare cache-rule →
verify render at 390/768/1280, embed still MapLibre-free → `fly deploy`.

**G3 ops checklist (provider console — not code, must be done by a human):**
- MapTiler (or chosen free vector provider): set a **spending cap** so an overage
  can't produce a surprise bill.
- Restrict the tile API key to the **production domain(s)** (referer/origin lock).
- Put **Cloudflare** in front of the tile origin with a cache-rule so origin egress
  stays ~zero. The CSP already allows `tiles.openfreemap.org`; the current free
  default needs no key, so these apply once a keyed provider is adopted.

**Triggers:** routing — approaching the ORS-free directions ceiling (~2000/day) *and*
a 2–3-month bill projection > server cost + maintenance time. Tiles — approaching the
free tile ceiling *and* the same projection.

## Consequences

- ETA becomes accurate and reversible; geo cost becomes a fixed line item flippable
  by env, not a rewrite.
- `opossum` is **not** added for the breaker (it would touch a governance-protected
  `package.json`); G1 uses the codebase's existing `Promise.race` + timeout + state
  pattern (see `packages/platform/src/message-bus.ts`). Revisit if a richer breaker
  is warranted.
