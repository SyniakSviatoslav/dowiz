// RoutingProvider — the road-routing seam (Dependency-Inversion).
//
// No calling code knows whether the route came from ORS-free, a self-hosted ORS/
// OSRM in `fra`, or the pure-math haversine fallback. Switching is one env line
// (ROUTING_PROVIDER), never a rewrite. The real implementations land in G1; G0
// only freezes the contract and ships a NotImplemented stub so the seam compiles.
//
// Routing is NON-CRITICAL: every implementation degrades silently to a straight
// polyline + haversine ETA on provider failure, so a routing outage never blocks
// a delivery (see docs/adr/ADR-GEO-SEAMS.md, Circuit Breaker matrix).

export type LatLng = { lat: number; lng: number };

export type RoutingProviderKind = 'ors' | 'self' | 'haversine';

export interface RouteResult {
  /** Road geometry, ordered from→to. Used for drawing and progress-along-route. */
  polyline: LatLng[];
  distance_m: number;
  /** Baseline travel time at calculation time. ETA is recomputed locally per ping. */
  duration_s: number;
  /** Provenance for logs/metrics only — never surfaced to the UI (ETA is advisory). */
  provider: RoutingProviderKind;
}

export interface RoutingProvider {
  /**
   * Compute one road route for a delivery leg. Called per-leg (≈once per delivery),
   * never per-ping. Implementations MUST resolve (never reject) by falling back to
   * a haversine straight-line result on any provider error/timeout/rate-limit.
   */
  route(from: LatLng, to: LatLng): Promise<RouteResult>;
  // matrix(...) — intentionally omitted (courier assignment; see planned-features).
}

/**
 * G0 placeholder. Throws until G1 wires the ORS + haversine-fallback implementation.
 * Never register this in a runtime path.
 */
export class NotImplementedRoutingProvider implements RoutingProvider {
  async route(_from: LatLng, _to: LatLng): Promise<RouteResult> {
    throw new Error('NotImplemented: RoutingProvider.route is wired in G1');
  }
}
