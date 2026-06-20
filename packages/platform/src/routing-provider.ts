// RoutingProvider — the road-routing seam (Dependency-Inversion).
//
// No calling code knows whether the route came from ORS-free, a self-hosted ORS/
// OSRM in `fra`, or the pure-math haversine fallback. Switching is one env line
// (ROUTING_PROVIDER), never a rewrite. See docs/adr/ADR-GEO-SEAMS.md.
//
// Routing is NON-CRITICAL: every implementation resolves (never rejects) by falling
// back to a straight haversine line on provider failure/timeout/rate-limit, so a
// routing outage never blocks a delivery. ETA is advisory, not a contract.

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
   * never per-ping. Resolves (never rejects) — falls back to haversine on any error.
   */
  route(from: LatLng, to: LatLng): Promise<RouteResult>;
  // matrix(...) — intentionally omitted (courier assignment; see planned-features).
}

// ── Tunables (defaults; overridable via factory config) ────────────────────────
export interface RoutingTunables {
  /** Multiplier turning straight-line distance into a road-ish estimate. */
  sinuosity: number;
  /** Average urban driving speed (km/h) for haversine ETA. */
  urbanSpeedKmh: number;
  /** Per-request timeout for the external provider. */
  timeoutMs: number;
  /** Consecutive failures before the breaker opens. */
  breakerFailureThreshold: number;
  /** How long the breaker stays open before a half-open trial. */
  breakerCooldownMs: number;
}

export const DEFAULT_TUNABLES: RoutingTunables = {
  sinuosity: 1.3,
  urbanSpeedKmh: 18,
  timeoutMs: 5000,
  breakerFailureThreshold: 3,
  breakerCooldownMs: 30_000,
};

// ── Pure geo math (self-contained; platform can't import apps/api) ──────────────
export function haversineMeters(a: LatLng, b: LatLng): number {
  const R = 6_371_000; // metres
  const dLat = ((b.lat - a.lat) * Math.PI) / 180;
  const dLng = ((b.lng - a.lng) * Math.PI) / 180;
  const s =
    Math.sin(dLat / 2) ** 2 +
    Math.cos((a.lat * Math.PI) / 180) * Math.cos((b.lat * Math.PI) / 180) * Math.sin(dLng / 2) ** 2;
  return 2 * R * Math.atan2(Math.sqrt(s), Math.sqrt(1 - s));
}

/** The hard fallback: straight polyline + haversine·sinuosity distance + speed-based ETA. */
export function haversineRoute(from: LatLng, to: LatLng, t: RoutingTunables = DEFAULT_TUNABLES): RouteResult {
  const straight = haversineMeters(from, to);
  const distance_m = Math.round(straight * t.sinuosity);
  const duration_s = Math.round(distance_m / ((t.urbanSpeedKmh * 1000) / 3600));
  return { polyline: [from, to], distance_m, duration_s, provider: 'haversine' };
}

// ── Minimal circuit breaker (existing Promise.race+timeout idiom; no opossum) ───
class CircuitBreaker {
  private failures = 0;
  private openedAt = 0;
  constructor(private threshold: number, private cooldownMs: number, private now: () => number = Date.now) {}

  /** True when the breaker is open (skip the external call entirely). */
  isOpen(): boolean {
    if (this.failures < this.threshold) return false;
    if (this.now() - this.openedAt >= this.cooldownMs) {
      // half-open: allow one trial, reset the window
      this.failures = this.threshold - 1;
      return false;
    }
    return true;
  }
  recordSuccess(): void {
    this.failures = 0;
    this.openedAt = 0;
  }
  recordFailure(): void {
    this.failures += 1;
    if (this.failures >= this.threshold) this.openedAt = this.now();
  }
}

export interface RoutingProviderConfig {
  provider: RoutingProviderKind;
  baseUrl: string;
  apiKey?: string;
  tunables?: Partial<RoutingTunables>;
  /** Injectable for tests; defaults to global fetch. */
  fetchImpl?: typeof fetch;
  /** Injectable clock for tests (breaker cooldown); defaults to Date.now. */
  now?: () => number;
  /** Structured warn sink (e.g. Pino). Defaults to console.warn. */
  warn?: (msg: string, meta?: Record<string, unknown>) => void;
}

/**
 * ORS-shaped provider (works for both managed ORS-free and a self-hosted ORS/OSRM —
 * same directions-geojson contract, different baseUrl + the `provider` tag). Falls
 * back to haversine on 429/403/timeout/any error, and short-circuits to haversine
 * while the breaker is open. Never rejects.
 */
export class OrsRoutingProvider implements RoutingProvider {
  private readonly t: RoutingTunables;
  private readonly breaker: CircuitBreaker;
  private readonly fetchImpl: typeof fetch;
  private readonly warn: (msg: string, meta?: Record<string, unknown>) => void;

  constructor(private readonly cfg: RoutingProviderConfig) {
    this.t = { ...DEFAULT_TUNABLES, ...(cfg.tunables ?? {}) };
    this.breaker = new CircuitBreaker(this.t.breakerFailureThreshold, this.t.breakerCooldownMs, cfg.now);
    this.fetchImpl = cfg.fetchImpl ?? globalThis.fetch;
    this.warn = cfg.warn ?? ((m, meta) => console.warn(`[routing] ${m}`, meta ?? ''));
  }

  async route(from: LatLng, to: LatLng): Promise<RouteResult> {
    const tag: RoutingProviderKind = this.cfg.provider === 'self' ? 'self' : 'ors';

    if (this.breaker.isOpen()) {
      return haversineRoute(from, to, this.t); // silent: client never knows
    }

    try {
      const url = `${this.cfg.baseUrl.replace(/\/$/, '')}/v2/directions/driving-car/geojson`;
      const res = await this.fetchImpl(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...(this.cfg.apiKey ? { Authorization: this.cfg.apiKey } : {}),
        },
        body: JSON.stringify({ coordinates: [[from.lng, from.lat], [to.lng, to.lat]] }),
        signal: AbortSignal.timeout(this.t.timeoutMs),
      });

      // Rate-limit hygiene: surface remaining budget; warn as we approach the ceiling.
      const remaining = Number(res.headers.get('x-ratelimit-remaining'));
      if (Number.isFinite(remaining) && remaining <= 50) {
        this.warn('approaching ORS rate-limit ceiling', { remaining });
      }

      if (!res.ok) {
        this.breaker.recordFailure();
        this.warn('ORS non-OK; falling back to haversine', { status: res.status });
        return haversineRoute(from, to, this.t);
      }

      const body: any = await res.json();
      const feature = body?.features?.[0];
      const coords: [number, number][] | undefined = feature?.geometry?.coordinates;
      const summary = feature?.properties?.summary;
      if (!Array.isArray(coords) || coords.length < 2 || !summary) {
        this.breaker.recordFailure();
        this.warn('ORS malformed response; falling back to haversine');
        return haversineRoute(from, to, this.t);
      }

      this.breaker.recordSuccess();
      return {
        polyline: coords.map(([lng, lat]) => ({ lat, lng })),
        distance_m: Math.round(summary.distance),
        duration_s: Math.round(summary.duration),
        provider: tag,
      };
    } catch (err) {
      // timeout / network / 429 thrown — all degrade silently.
      this.breaker.recordFailure();
      this.warn('ORS call failed; falling back to haversine', { err: String(err) });
      return haversineRoute(from, to, this.t);
    }
  }
}

/** Pure-math provider — always a straight haversine line. */
export class HaversineRoutingProvider implements RoutingProvider {
  private readonly t: RoutingTunables;
  constructor(tunables?: Partial<RoutingTunables>) {
    this.t = { ...DEFAULT_TUNABLES, ...(tunables ?? {}) };
  }
  async route(from: LatLng, to: LatLng): Promise<RouteResult> {
    return haversineRoute(from, to, this.t);
  }
}

/**
 * Factory — selects the implementation from env. The ONLY place the provider name is
 * read; feature code depends on the RoutingProvider interface alone.
 */
export function createRoutingProvider(
  env: { ROUTING_PROVIDER: RoutingProviderKind; ROUTING_BASE_URL: string; ROUTING_API_KEY?: string },
  overrides: Partial<RoutingProviderConfig> = {},
): RoutingProvider {
  if (env.ROUTING_PROVIDER === 'haversine') {
    return new HaversineRoutingProvider(overrides.tunables);
  }
  return new OrsRoutingProvider({
    provider: env.ROUTING_PROVIDER,
    baseUrl: env.ROUTING_BASE_URL,
    apiKey: env.ROUTING_API_KEY,
    ...overrides,
  });
}

/**
 * G0 placeholder kept for reference. Never register this in a runtime path —
 * use createRoutingProvider() instead.
 */
export class NotImplementedRoutingProvider implements RoutingProvider {
  async route(_from: LatLng, _to: LatLng): Promise<RouteResult> {
    throw new Error('NotImplemented: use createRoutingProvider() (G1)');
  }
}
