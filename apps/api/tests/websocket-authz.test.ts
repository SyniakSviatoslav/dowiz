import { test } from 'node:test';
import assert from 'node:assert/strict';

// websocket.ts pulls in @deliveryos/platform → @deliveryos/db, whose module top runs loadEnv().
// Seat the minimal required (non-optional, no-default) vars with FAKE values so the module LOADS
// (no pool connects at import — pools are lazy). We never touch fastify.db in these tests; the
// guard + verdict get injected/faked, so no live DB is needed.
const FAKE_URL = 'postgres://u:p@localhost:6543/db?sslmode=disable';
process.env.NODE_ENV ??= 'test';
process.env.APP_BASE_URL ??= 'http://localhost:3000';
process.env.DATABASE_URL_OPERATIONAL ??= FAKE_URL;
process.env.DATABASE_URL_SESSION ??= FAKE_URL;
process.env.DATABASE_URL_MIGRATIONS ??= FAKE_URL;
process.env.REDIS_URL ??= 'redis://localhost:6379';
process.env.JWT_PRIVATE_KEY ??= 'fake-priv';
process.env.JWT_PUBLIC_KEY ??= 'fake-pub';
process.env.JWT_KID ??= 'fake-kid';
process.env.GOOGLE_CLIENT_ID ??= 'fake-gid';
process.env.GOOGLE_CLIENT_SECRET ??= 'fake-gsecret';
process.env.VAPID_PUBLIC_KEY ??= 'fake-vapid-pub';
process.env.VAPID_PRIVATE_KEY ??= 'fake-vapid-priv';
process.env.IP_HASH_SALT ??= 'fake-salt';

const {
  createOwnerRelayGuard,
  ownerRoomVerdict,
  logTokenDeprecation,
  logAuthSuccess,
} = await import('../src/websocket.js');

type OwnerVerdict = 'ALLOW' | 'DENY' | 'UNAVAILABLE';

// Flush the microtask queue (the async revalidate resolves there).
const tick = () => new Promise<void>((r) => setImmediate(r));

function ownerMember(sub: string, userId?: string) {
  const sent: string[] = [];
  const m: any = {
    sent,
    readyState: 1, // OPEN
    user: { role: 'owner', sub, userId },
    ws: {
      get readyState() { return m.readyState; },
      send: (d: string) => sent.push(d),
    },
  };
  return m;
}

function guardHarness(verdict: OwnerVerdict | (() => OwnerVerdict)) {
  let t = 1_000_000;
  const checks: { room: string; ownerId: string }[] = [];
  const evicted: { room: string; ownerId: string; reason: string }[] = [];
  const guard = createOwnerRelayGuard({
    now: () => t,
    ttlMs: 10_000,
    check: async (room: string, ownerId: string) => {
      checks.push({ room, ownerId });
      return typeof verdict === 'function' ? verdict() : verdict;
    },
    evict: (room: string, mem: any, reason: string) => evicted.push({ room, ownerId: mem.user.userId ?? mem.user.sub, reason }),
  });
  return { guard, checks, evicted, advance: (ms: number) => { t += ms; } };
}

const ROOM = 'order:ord-1';

// ── #4 fan-out owner revocation (the load-bearing HIGH: eviction, not just subscribe) ──

test('#4 revoked owner (verdict DENY) is EVICTED from the fan-out and NO frame is relayed', async () => {
  const { guard, evicted } = guardHarness('DENY');
  const m = ownerMember('owner-A', 'owner-A');
  assert.equal(guard.relay(ROOM, m, 'f1'), 'withheld', 'cold frame is withheld, never relay-then-check');
  assert.deepEqual(m.sent, [], 'no frame reaches a revoked owner');
  await tick();
  assert.equal(evicted.length, 1, 'the revoked owner is evicted from the room');
  assert.deepEqual(evicted[0], { room: ROOM, ownerId: 'owner-A', reason: 'membership_revoked' });
});

test('#4 active owner (verdict ALLOW) receives frames from cache within the TTL window', async () => {
  const { guard, checks, evicted } = guardHarness('ALLOW');
  const m = ownerMember('owner-A', 'owner-A');
  assert.equal(guard.relay(ROOM, m, 'f1'), 'withheld'); // cold → prime
  await tick();
  assert.equal(guard.relay(ROOM, m, 'f2'), 'relayed');
  assert.equal(guard.relay(ROOM, m, 'f3'), 'relayed');
  assert.deepEqual(m.sent, ['f2', 'f3']);
  assert.equal(checks.length, 1, 'one read primes the cache for the whole TTL window');
  assert.equal(evicted.length, 0);
});

test('#4 mid-stream revocation: cached ALLOW expires → re-read DENY → owner evicted, no leak (≤TTL residual, OR-9)', async () => {
  let v: OwnerVerdict = 'ALLOW';
  const { guard, evicted, advance } = guardHarness(() => v);
  const m = ownerMember('owner-A', 'owner-A');
  guard.relay(ROOM, m, 'f1'); await tick(); // bound
  guard.relay(ROOM, m, 'f2');               // relayed from cache
  v = 'DENY';                               // membership.status flips to revoked
  advance(10_000);                          // cached ALLOW expires (the ≤TTL window)
  assert.equal(guard.relay(ROOM, m, 'f3'), 'withheld'); await tick();
  assert.deepEqual(m.sent, ['f2'], 'no frame leaks after revocation');
  assert.equal(evicted.length, 1);
});

test('#4 UNAVAILABLE (transient DB blip) withholds but does NOT evict a live owner', async () => {
  const { guard, evicted } = guardHarness('UNAVAILABLE');
  const m = ownerMember('owner-A', 'owner-A');
  assert.equal(guard.relay(ROOM, m, 'f1'), 'withheld'); await tick();
  assert.equal(evicted.length, 0, 'a single DB blip must not bounce a legitimate owner');
  assert.deepEqual(m.sent, []);
});

test('#4 a closed owner socket is skipped (no send, no read)', async () => {
  const { guard, checks } = guardHarness('ALLOW');
  const m = ownerMember('owner-A', 'owner-A');
  m.readyState = 3; // CLOSED
  assert.equal(guard.relay(ROOM, m, 'f1'), 'skipped');
  await tick();
  assert.equal(checks.length, 0);
});

test('#4 concurrent frames for the same (room,owner) trigger only ONE in-flight re-read', async () => {
  const { guard, checks } = guardHarness('ALLOW');
  const m = ownerMember('owner-A', 'owner-A');
  guard.relay(ROOM, m, 'f1');
  guard.relay(ROOM, m, 'f2');
  guard.relay(ROOM, m, 'f3');
  await tick();
  assert.equal(checks.length, 1, 'inflight dedup collapses a burst into one read');
});

test('#4 owner identity falls back to sub when userId is absent', async () => {
  const { guard, checks } = guardHarness('ALLOW');
  const m = ownerMember('sub-only'); // no userId
  guard.relay(ROOM, m, 'f1'); await tick();
  assert.equal(checks[0].ownerId, 'sub-only');
});

// ── subscribe-gate: status='active' required (ADR-0004) + tri-state verdict mapping ──

function fakeDb(rowCount: number | null, capture?: { sql?: string; params?: unknown[] }) {
  return {
    query: async (sql: string, params: unknown[]) => {
      if (capture) { capture.sql = sql; capture.params = params; }
      return { rowCount };
    },
  };
}

test('#4 subscribe-gate: order: verdict requires status = active (mirror the location: sibling)', async () => {
  const cap: { sql?: string } = {};
  await ownerRoomVerdict(fakeDb(1, cap), 'owner-A', 'order:ord-1');
  assert.match(cap.sql!, /status\s*=\s*'active'/, "order: room query must carry status = 'active' (was missing → revoked owner passed subscribe)");
  assert.match(cap.sql!, /JOIN\s+memberships/i);
});

test('#4 subscribe-gate: location: verdict also requires status = active', async () => {
  const cap: { sql?: string } = {};
  await ownerRoomVerdict(fakeDb(1, cap), 'owner-A', 'location:loc-1');
  assert.match(cap.sql!, /status\s*=\s*'active'/);
});

test('#4 verdict mapping: row→ALLOW, 0 rows→DENY, query throw→UNAVAILABLE', async () => {
  assert.equal(await ownerRoomVerdict(fakeDb(1), 'o', 'order:ord-1'), 'ALLOW');
  assert.equal(await ownerRoomVerdict(fakeDb(0), 'o', 'order:ord-1'), 'DENY', 'a clean 0-row read is a real negative → fail closed');
  const throwingDb = { query: async () => { throw new Error('pool exhausted'); } };
  assert.equal(await ownerRoomVerdict(throwingDb, 'o', 'order:ord-1'), 'UNAVAILABLE', 'a query failure is transient, not a negative');
});

test('#4 verdict: unknown room kind and empty userId → DENY with no query', async () => {
  let queried = false;
  const spyDb = { query: async () => { queried = true; return { rowCount: 1 }; } };
  assert.equal(await ownerRoomVerdict(spyDb, 'o', 'courier:x'), 'DENY');
  assert.equal(await ownerRoomVerdict(spyDb, '', 'order:ord-1'), 'DENY');
  assert.equal(queried, false);
});

// ── #5 JWT-in-URL: deprecation telemetry + auth-success log redaction ──

function captureConsole<T>(fn: () => T): { out: string[]; result: T } {
  const out: string[] = [];
  const origLog = console.log;
  const origWarn = console.warn;
  const sink = (...a: unknown[]) => { out.push(a.map(String).join(' ')); };
  console.log = sink as any;
  console.warn = sink as any;
  try {
    const result = fn();
    return { out, result };
  } finally {
    console.log = origLog;
    console.warn = origWarn;
  }
}

const RAW_TOKEN = 'eyJhbGciOiJSUzI1NiJ9.SECRET-JWT-BODY.sig';
const SUB = '00000000-0000-4000-8000-000000000abc';

test('#5 ?token= usage emits a deprecation log (drives usage → zero) with role, never the token or sub', () => {
  const { out } = captureConsole(() => logTokenDeprecation('owner', '1.2.3.4'));
  const line = out.join('\n');
  assert.match(line, /DEPRECATED.*\?token=/i, 'the deprecated ?token= path must be logged so usage can be driven to zero');
  assert.match(line, /owner/, 'role is recorded to target the client migration');
  assert.ok(!line.includes(RAW_TOKEN), 'the raw token value must never be logged');
  assert.ok(!line.includes(SUB), 'sub (identity) must not be in the deprecation log');
});

test('#5 auth-success log contains NO raw token and NO sub (the earlier-audit LOW)', () => {
  const url = captureConsole(() => logAuthSuccess('url', '1.2.3.4'));
  const msg = captureConsole(() => logAuthSuccess('message', '5.6.7.8'));
  for (const { out } of [url, msg]) {
    const line = out.join('\n');
    assert.ok(!line.includes(RAW_TOKEN), 'auth-success log must not contain the raw token');
    assert.ok(!line.includes(SUB), 'auth-success log must not contain sub');
    assert.match(line, /authenticated/i, 'still emits a success line (for ops), just redacted');
  }
});
