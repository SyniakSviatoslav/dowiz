// Shared real-client-IP resolver for ALL IP-keyed rate limiters + per-IP throttles
// (security-hardening 2026-07, finding #9). Single source of truth so the global flood
// limiter, the auth/OTP/login limiters that inherit it, the funnel/access-gate per-IP
// limiters, and the per-IP order throttle all key on the SAME real client IP.
//
// TRUST MODEL (OR-10, operator-confirmed edge-only ingress):
//   - Trust ONLY `Fly-Client-IP`. The Fly edge sets & overwrites it on every ingress path,
//     so it is NOT client-injectable.
//   - NEVER trust `X-Forwarded-For` — it is client-controllable; trusting it would let an
//     attacker rotate the header to fragment every rate-limit bucket → brute-force evasion
//     on money/auth. XFF is deliberately not consulted here at all.
//   - Header absent → fail SAFE: non-prod degrades to `request.ip` (the Fly socket, still
//     not client-controllable — deterministic for tests). Prod fails CLOSED to a single
//     shared bucket (`shared:no-fly-ip`) + a throttled re-warn (≤1/min) so a missing edge
//     header collapses everyone into one bucket rather than trusting a spoofable header.

const FLY_IP_HEADER = 'fly-client-ip';
let lastFlyMissingWarnAt = 0;

/**
 * Normalize an IP so casing / IPv4-mapped-IPv6 prefixes / zone ids / brackets do not
 * fragment a single client across multiple rate-limit buckets:
 *   - lowercase (IPv6 hex is case-insensitive: `2001:DB8::1` == `2001:db8::1`)
 *   - strip surrounding `[...]` brackets
 *   - strip an IPv6 zone id (`fe80::1%eth0` → `fe80::1`)
 *   - collapse IPv4-mapped IPv6 (`::ffff:1.2.3.4` → `1.2.3.4`) so the same client seen as
 *     v4 and as v4-mapped-v6 shares one bucket
 */
export function normalizeIp(raw: string): string {
  let ip = raw.trim().toLowerCase();
  if (ip.startsWith('[')) {
    const end = ip.indexOf(']');
    ip = end > 0 ? ip.slice(1, end) : ip.slice(1);
  }
  const zone = ip.indexOf('%');
  if (zone >= 0) ip = ip.slice(0, zone);
  // IPv4-mapped IPv6 → bare IPv4 (also handles the deprecated `::` compat form)
  const mapped = ip.match(/^::ffff:(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})$/);
  if (mapped && mapped[1]) ip = mapped[1];
  return ip;
}

/**
 * Real client IP for rate-limiting / throttling. See TRUST MODEL above.
 * Returns a stable bucket key (a normalized IP, or `shared:no-fly-ip` when failing closed).
 */
export function clientIp(request: any): string {
  const raw = request?.headers?.[FLY_IP_HEADER];
  const fly = Array.isArray(raw) ? raw[0] : raw;
  if (typeof fly === 'string' && fly.length > 0) return normalizeIp(fly);
  if (process.env.NODE_ENV !== 'production') {
    const ip = request?.ip;
    return typeof ip === 'string' && ip.length > 0 ? normalizeIp(ip) : 'unknown';
  }
  const now = Date.now();
  if (now - lastFlyMissingWarnAt > 60_000) {
    lastFlyMissingWarnAt = now;
    request?.log?.warn?.('[client-ip] Fly-Client-IP missing in production — rate-limit degraded to a shared bucket');
  }
  return 'shared:no-fly-ip';
}
