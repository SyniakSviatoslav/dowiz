import Redis from 'ioredis';
import {
  createRoutingProvider,
  haversineMeters,
  type LatLng,
  type RouteResult,
  type RoutingProvider,
} from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';

// Per-leg routing service. Wraps a RoutingProvider with:
//   • a short-TTL, in-process, NON-AUTHORITATIVE cache (kill → re-fetch; never the
//     source of truth — the authoritative RouteResult is pushed to order:{id} and
//     lives in delivery state),
//   • re-route geometry so a live position that strays > threshold from the polyline
//     triggers exactly one new route() (never per-ping).
//
// One route() per delivery is achieved by calling getLegRoute() at the single
// assignment / picked_up transition; the cache only dedupes identical legs across
// deliveries to spare the provider's rate budget.

const ROUTE_CACHE_TTL_MS = 10 * 60 * 1000;
const DEFAULT_REROUTE_THRESHOLD_M = 300;

/** ~4 decimals ≈ 11 m: near-identical legs share a cache entry. */
function roundCoord(n: number): number {
  return Math.round(n * 10_000) / 10_000;
}
function legKey(from: LatLng, to: LatLng): string {
  return `${roundCoord(from.lat)},${roundCoord(from.lng)}|${roundCoord(to.lat)},${roundCoord(to.lng)}`;
}

interface CacheEntry {
  result: RouteResult;
  expiresAt: number;
}

export class RoutingService {
  private readonly cache = new Map<string, CacheEntry>();

  constructor(
    private readonly provider: RoutingProvider,
    private readonly now: () => number = Date.now,
    private readonly ttlMs: number = ROUTE_CACHE_TTL_MS,
  ) {}

  /** Cache-first leg route. Identical legs within the TTL never hit the provider. */
  async getLegRoute(from: LatLng, to: LatLng): Promise<RouteResult> {
    const key = legKey(from, to);
    const hit = this.cache.get(key);
    if (hit && hit.expiresAt > this.now()) {
      return hit.result;
    }
    const result = await this.provider.route(from, to);
    this.cache.set(key, { result, expiresAt: this.now() + this.ttlMs });
    return result;
  }

  /** Test/ops visibility — current live cache size. */
  get cacheSize(): number {
    return this.cache.size;
  }
}

// ── Re-route geometry ──────────────────────────────────────────────────────────

/**
 * Shortest distance (m) from a point to a polyline. Uses a local equirectangular
 * projection — accurate at city scale, cheap enough to run per-ping for the
 * re-route check (no provider call).
 */
export function deviationMeters(polyline: LatLng[], pos: LatLng): number {
  if (polyline.length === 0) return Infinity;
  if (polyline.length === 1) return haversineMeters(polyline[0]!, pos);

  const latRad = (pos.lat * Math.PI) / 180;
  const mPerDegLat = 111_320;
  const mPerDegLng = 111_320 * Math.cos(latRad);
  const toXY = (p: LatLng) => ({ x: (p.lng - pos.lng) * mPerDegLng, y: (p.lat - pos.lat) * mPerDegLat });
  const P = { x: 0, y: 0 }; // pos at origin

  let min = Infinity;
  for (let i = 0; i < polyline.length - 1; i++) {
    const A = toXY(polyline[i]!);
    const B = toXY(polyline[i + 1]!);
    const dx = B.x - A.x;
    const dy = B.y - A.y;
    const lenSq = dx * dx + dy * dy;
    let t = lenSq === 0 ? 0 : ((P.x - A.x) * dx + (P.y - A.y) * dy) / lenSq;
    t = Math.max(0, Math.min(1, t));
    const cx = A.x + t * dx;
    const cy = A.y + t * dy;
    const d = Math.hypot(P.x - cx, P.y - cy);
    if (d < min) min = d;
  }
  return min;
}

/** True only when the live position has strayed beyond the threshold → one re-route. */
export function shouldReroute(
  polyline: LatLng[],
  pos: LatLng,
  thresholdM: number = DEFAULT_REROUTE_THRESHOLD_M,
): boolean {
  return deviationMeters(polyline, pos) > thresholdM;
}

// ── Process-wide singletons + Redis-backed route state (N-safe) ─────────────────
//
// The RouteResult is authoritative state, not the in-process cache: it lives in
// Redis keyed by order so it survives a process kill and is shared across the N
// instances. The status endpoint reads it for reconnecting clients; the worker
// writes it once at picked_up (+ on a re-route).

let _service: RoutingService | null = null;
/** Lazy singleton — shares one provider + leg-cache across the process. */
export function getRoutingService(): RoutingService {
  if (!_service) _service = new RoutingService(createRoutingProvider(loadEnv()));
  return _service;
}

let _redis: Redis | null = null;
function routeRedis(): Redis {
  // Lazy: no connection at boot. Separate logical use from MessageBus (PG NOTIFY).
  if (!_redis) _redis = new Redis(loadEnv().REDIS_URL, { maxRetriesPerRequest: 2 });
  return _redis;
}

/** Test-only: close the lazy Redis connection so a test process can exit. */
export async function closeRouteRedis(): Promise<void> {
  if (_redis) { await _redis.quit().catch(() => {}); _redis = null; }
}

const routeKey = (orderId: string) => `route:${orderId}`;
const ROUTE_TTL_S = 2 * 60 * 60; // outlives a delivery; cleaned up by TTL

export async function saveRoute(orderId: string, r: RouteResult): Promise<void> {
  await routeRedis().set(routeKey(orderId), JSON.stringify(r), 'EX', ROUTE_TTL_S);
}

export async function loadRoute(orderId: string): Promise<RouteResult | null> {
  try {
    const v = await routeRedis().get(routeKey(orderId));
    return v ? (JSON.parse(v) as RouteResult) : null;
  } catch {
    return null; // routing is advisory — never let a Redis hiccup break the read
  }
}

/**
 * NX claim so exactly one of the N instances computes/publishes for a given key
 * within the window (PG NOTIFY fans out to every listener, so all instances see
 * the same picked-up / reroute trigger). Returns true iff this instance won.
 */
export async function claimOnce(key: string, ttlS: number): Promise<boolean> {
  try {
    const res = await routeRedis().set(`claim:${key}`, '1', 'EX', ttlS, 'NX');
    return res === 'OK';
  } catch {
    return false; // on Redis error, don't compute — fallback ETA still flows
  }
}
