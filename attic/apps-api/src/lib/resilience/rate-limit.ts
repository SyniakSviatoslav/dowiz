// @ts-nocheck
import crypto from 'node:crypto';

interface TokenBucket {
  tokens: number;
  lastRefill: number;
}

const buckets = new Map<string, TokenBucket>();
const inflightCounts = new Map<string, number>();

export interface RateLimitOpts {
  perTenant: number;
  perTenantWindowMs: number;
  perIp: number;
  perIpWindowMs: number;
  inflightPerTenant: number;
}

export function checkRateLimit(key: string, max: number, windowMs: number): boolean {
  const now = Date.now();
  const bucket = buckets.get(key);
  if (!bucket) {
    buckets.set(key, { tokens: max - 1, lastRefill: now });
    return true;
  }
  const elapsed = now - bucket.lastRefill;
  const refill = Math.floor(elapsed / windowMs) * max;
  if (refill > 0) {
    bucket.tokens = Math.min(max, bucket.tokens + refill);
    bucket.lastRefill = now;
  }
  if (bucket.tokens > 0) {
    bucket.tokens--;
    return true;
  }
  return false;
}

export function checkTenantRateLimit(tenantId: string, route: string, opts: RateLimitOpts): boolean {
  return checkRateLimit(`tenant:${tenantId}:${route}`, opts.perTenant, opts.perTenantWindowMs);
}

export function checkIpRateLimit(ip: string, route: string, opts: RateLimitOpts): boolean {
  const ipHash = crypto.createHash('sha256').update(ip).digest('hex').slice(0, 16);
  return checkRateLimit(`ip:${ipHash}:${route}`, opts.perIp, opts.perIpWindowMs);
}

export function ipHash(ip: string): string {
  return crypto.createHash('sha256').update(ip).digest('hex').slice(0, 16);
}

export function acquireInflight(tenantId: string, max: number): boolean {
  const current = inflightCounts.get(tenantId) || 0;
  if (current >= max) return false;
  inflightCounts.set(tenantId, current + 1);
  return true;
}

export function releaseInflight(tenantId: string): void {
  const current = inflightCounts.get(tenantId) || 0;
  if (current <= 1) {
    inflightCounts.delete(tenantId);
  } else {
    inflightCounts.set(tenantId, current - 1);
  }
}

export function getInflightCount(tenantId: string): number {
  return inflightCounts.get(tenantId) || 0;
}

export function cleanupStaleBuckets(maxAgeMs = 3600000): void {
  const cutoff = Date.now() - maxAgeMs;
  for (const [key, bucket] of buckets) {
    if (bucket.lastRefill < cutoff) buckets.delete(key);
  }
}

const DEFAULT_OPTS: RateLimitOpts = {
  perTenant: 10,
  perTenantWindowMs: 60000,
  perIp: 20,
  perIpWindowMs: 60000,
  inflightPerTenant: 3,
};

export function getRateLimitOpts(overrides?: Partial<RateLimitOpts>): RateLimitOpts {
  return { ...DEFAULT_OPTS, ...overrides };
}

export const STRICT_OPTS: RateLimitOpts = {
  perTenant: 3,
  perTenantWindowMs: 60000,
  perIp: 5,
  perIpWindowMs: 60000,
  inflightPerTenant: 1,
};

export const ORDER_OPTS: RateLimitOpts = {
  perTenant: 5,
  perTenantWindowMs: 60000,
  perIp: 10,
  perIpWindowMs: 60000,
  inflightPerTenant: 2,
};

export const PROMO_OPTS: RateLimitOpts = {
  perTenant: 10,
  perTenantWindowMs: 60000,
  perIp: 5,
  perIpWindowMs: 60000,
  inflightPerTenant: 1,
};

export const AUTH_OPTS: RateLimitOpts = {
  perTenant: 5,
  perTenantWindowMs: 60000,
  perIp: 3,
  perIpWindowMs: 60000,
  inflightPerTenant: 1,
};

export function recordAbuse(kind: string, tenantId: string | null, ip: string, reason: string): void {
  const ipH = crypto.createHash('sha256').update(ip).digest('hex').slice(0, 16);
  const event = { ts: new Date().toISOString(), kind, tenantId: tenantId || 'global', ipHash: ipH, reason };
  console.warn('[ABUSE]', JSON.stringify(event));
}
