import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import rateLimit from '@fastify/rate-limit';
import { clientIp, normalizeIp } from '../src/lib/client-ip.js';

// #9 (security-hardening 2026-07) — proof that the shared clientIp() resolver keys every
// IP-keyed rate limiter on the REAL client IP (Fly-Client-IP), never the client-injectable
// X-Forwarded-For, and never the Fly-edge socket (request.ip) that collapses all clients
// into one bucket.

async function buildApp(keyGenerator: (req: any) => string) {
  const app = Fastify();
  // max:1 so the SECOND hit on the SAME bucket is a 429, the FIRST hit on a NEW bucket is 200.
  await app.register(rateLimit, { max: 1, timeWindow: '1 minute', keyGenerator });
  app.get('/ping', async () => ({ ok: true }));
  return app;
}

// ── RED: the pre-fix behavior (global limiter had no keyGenerator → defaults to request.ip).
// Behind the Fly edge, request.ip is the edge socket, IDENTICAL for every real client, so two
// distinct clients collapse into ONE bucket → the 2nd distinct client is falsely 429'd.
test('RED (pre-fix): request.ip keying collapses distinct clients into ONE bucket', async () => {
  const app = await buildApp((req: any) => req.ip); // <-- the old/default key
  try {
    // inject() gives every request the same remoteAddress (127.0.0.1) → same request.ip,
    // exactly like the Fly edge socket in prod.
    const a = await app.inject({ method: 'GET', url: '/ping', headers: { 'fly-client-ip': '1.1.1.1' } });
    const b = await app.inject({ method: 'GET', url: '/ping', headers: { 'fly-client-ip': '2.2.2.2' } });
    assert.equal(a.statusCode, 200, 'first client ok');
    assert.equal(b.statusCode, 429, 'DISTINCT client wrongly throttled — the bug clientIp() fixes');
  } finally {
    await app.close();
  }
});

// ── GREEN: with the clientIp() resolver, distinct real IPs get distinct buckets.
test('GREEN: distinct Fly-Client-IP → distinct rate-limit buckets', async () => {
  const app = await buildApp((req: any) => clientIp(req));
  try {
    const a = await app.inject({ method: 'GET', url: '/ping', headers: { 'fly-client-ip': '1.1.1.1' } });
    const b = await app.inject({ method: 'GET', url: '/ping', headers: { 'fly-client-ip': '2.2.2.2' } });
    assert.equal(a.statusCode, 200, 'client 1.1.1.1 ok');
    assert.equal(b.statusCode, 200, 'client 2.2.2.2 has its OWN bucket, not throttled');
  } finally {
    await app.close();
  }
});

test('GREEN: same Fly-Client-IP → SAME bucket (second hit 429)', async () => {
  const app = await buildApp((req: any) => clientIp(req));
  try {
    const a = await app.inject({ method: 'GET', url: '/ping', headers: { 'fly-client-ip': '3.3.3.3' } });
    const b = await app.inject({ method: 'GET', url: '/ping', headers: { 'fly-client-ip': '3.3.3.3' } });
    assert.equal(a.statusCode, 200);
    assert.equal(b.statusCode, 429, 'repeat client correctly throttled');
  } finally {
    await app.close();
  }
});

// ── XFF is NEVER trusted: rotating X-Forwarded-For must NOT let an attacker escape the bucket.
test('GREEN: spoofed X-Forwarded-For does NOT change the bucket (XFF ignored)', async () => {
  const app = await buildApp((req: any) => clientIp(req));
  try {
    const a = await app.inject({
      method: 'GET', url: '/ping',
      headers: { 'fly-client-ip': '4.4.4.4', 'x-forwarded-for': '10.0.0.1' },
    });
    // Attacker rotates XFF to try to get a fresh bucket — same Fly-Client-IP → still throttled.
    const b = await app.inject({
      method: 'GET', url: '/ping',
      headers: { 'fly-client-ip': '4.4.4.4', 'x-forwarded-for': '10.0.0.99' },
    });
    assert.equal(a.statusCode, 200);
    assert.equal(b.statusCode, 429, 'XFF rotation must NOT create a new bucket — brute-force evasion blocked');
  } finally {
    await app.close();
  }
});

// ── IPv6 normalization: a client seen as bare IPv4 and IPv4-mapped-IPv6 shares one bucket;
// casing does not fragment.
test('GREEN: IPv6 normalization — ::ffff: prefix and casing share one bucket', async () => {
  const app = await buildApp((req: any) => clientIp(req));
  try {
    const a = await app.inject({ method: 'GET', url: '/ping', headers: { 'fly-client-ip': '5.5.5.5' } });
    const b = await app.inject({ method: 'GET', url: '/ping', headers: { 'fly-client-ip': '::ffff:5.5.5.5' } });
    assert.equal(a.statusCode, 200);
    assert.equal(b.statusCode, 429, 'IPv4-mapped-IPv6 must collapse to the same bucket as bare IPv4');
  } finally {
    await app.close();
  }

  // casing / zone-id / brackets normalize to one canonical key
  assert.equal(normalizeIp('2001:DB8::1'), '2001:db8::1', 'lowercase');
  assert.equal(normalizeIp('[2001:db8::1]'), '2001:db8::1', 'brackets stripped');
  assert.equal(normalizeIp('fe80::1%eth0'), 'fe80::1', 'zone id stripped');
  assert.equal(normalizeIp('::ffff:127.0.0.1'), '127.0.0.1', 'v4-mapped collapsed');
});

// ── Header-absent fail-safe. Non-prod → deterministic request.ip (normalized). Prod → fails
// CLOSED to a single shared bucket, and NEVER falls to the client-controllable XFF.
test('GREEN: header-absent fail-safe — prod → shared bucket, never XFF', async () => {
  const prev = process.env.NODE_ENV;
  try {
    process.env.NODE_ENV = 'production';
    const key = clientIp({ headers: { 'x-forwarded-for': '9.9.9.9' }, ip: '7.7.7.7', log: { warn() {} } });
    assert.equal(key, 'shared:no-fly-ip', 'prod + no Fly-Client-IP → shared bucket (NOT the XFF, NOT request.ip)');

    process.env.NODE_ENV = 'test';
    const key2 = clientIp({ headers: { 'x-forwarded-for': '9.9.9.9' }, ip: '::ffff:7.7.7.7' });
    assert.equal(key2, '7.7.7.7', 'non-prod degrades to normalized request.ip, never the XFF');
  } finally {
    process.env.NODE_ENV = prev;
  }
});
