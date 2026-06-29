import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createCourierRelayGuard, type RelayMember } from '../src/lib/courier-relay-guard.js';
import type { AuthzVerdict } from '../src/lib/courier-room-authz.js';

// Flush pending microtasks (the async revalidate resolves on the microtask queue).
const tick = () => new Promise<void>((r) => setImmediate(r));

function member(sub: string, role = 'courier'): RelayMember & { sent: string[]; readyState: number } {
  const sent: string[] = [];
  const m: any = {
    sent,
    readyState: 1,
    user: { role, sub, activeLocationId: 'loc-1' },
    ws: {
      get readyState() { return m.readyState; },
      send: (d: string) => sent.push(d),
    },
  };
  return m;
}

// A clock + a programmable tri-state check + an evict spy.
function harness(verdict: AuthzVerdict | (() => AuthzVerdict), o: Parameters<typeof createCourierRelayGuard>[0] extends infer T ? Partial<any> : never = {}) {
  let t = 1_000_000;
  const checks: { orderId: string; sub: string }[] = [];
  const evicted: { orderId: string; sub: string; reason: string }[] = [];
  const guard = createCourierRelayGuard({
    now: () => t,
    ttlMs: 10_000,
    ceilingMs: 60_000,
    maxUnavail: 120,
    check: async (orderId, sub) => { checks.push({ orderId, sub }); return typeof verdict === 'function' ? verdict() : verdict; },
    evict: (orderId, mem, reason) => evicted.push({ orderId, sub: mem.user.sub, reason }),
    ...o,
  });
  return { guard, checks, evicted, advance: (ms: number) => { t += ms; }, at: () => t };
}

const ORD = 'ord-1';

test('cold frame is WITHHELD and triggers one re-read; after ALLOW the next frame relays from cache', async () => {
  const { guard, checks } = harness('ALLOW');
  const m = member('cA');
  assert.equal(guard.relay(ORD, m, 'f1'), 'withheld');
  assert.deepEqual(m.sent, [], 'never relay-then-revalidate: the cold frame is withheld');
  await tick();
  assert.equal(checks.length, 1);
  assert.equal(guard.relay(ORD, m, 'f2'), 'relayed');
  assert.deepEqual(m.sent, ['f2']);
});

test('within TTL, subsequent frames relay from cache with NO extra db read', async () => {
  const { guard, checks, advance } = harness('ALLOW');
  const m = member('cA');
  guard.relay(ORD, m, 'f1'); await tick(); // prime
  guard.relay(ORD, m, 'f2');
  advance(9_999);
  guard.relay(ORD, m, 'f3');
  assert.deepEqual(m.sent, ['f2', 'f3']);
  assert.equal(checks.length, 1, 'one read primed the cache for the whole TTL window');
});

test('absolute TTL, NO refresh-on-access: after ttl the frame is withheld and re-read', async () => {
  const { guard, checks, advance } = harness('ALLOW');
  const m = member('cA');
  guard.relay(ORD, m, 'f1'); await tick();
  guard.relay(ORD, m, 'f2');             // within TTL, relayed from cache, does NOT refresh expiry
  advance(10_000);                        // ttl boundary crossed
  assert.equal(guard.relay(ORD, m, 'f3'), 'withheld');
  assert.deepEqual(m.sent, ['f2'], 'the TTL-boundary frame is withheld, never relayed to an unconfirmed member');
  await tick();
  assert.equal(checks.length, 2, 'a second read fires only after expiry, not on every access');
});

test('DENY (real revocation) → evict + binding_revoked, frame never relayed', async () => {
  const { guard, evicted } = harness('DENY');
  const m = member('cA');
  assert.equal(guard.relay(ORD, m, 'f1'), 'withheld');
  await tick();
  assert.deepEqual(m.sent, []);
  assert.equal(evicted.length, 1);
  assert.deepEqual(evicted[0], { orderId: ORD, sub: 'cA', reason: 'binding_revoked' });
});

test('reassigned colleague mid-stream: cached ALLOW expires → re-read DENY → evicted (the C1 leak closed)', async () => {
  let v: AuthzVerdict = 'ALLOW';
  const { guard, evicted, advance } = harness(() => v);
  const m = member('cA');
  guard.relay(ORD, m, 'f1'); await tick();   // bound
  guard.relay(ORD, m, 'f2');                  // relayed
  v = 'DENY';                                 // owner reassigns the order to another courier
  advance(10_000);                            // cached ALLOW expires
  guard.relay(ORD, m, 'f3'); await tick();    // re-read sees the revocation
  assert.deepEqual(m.sent, ['f2'], 'no frame leaks after revocation');
  assert.equal(evicted.length, 1);
});

test('UNAVAILABLE: withheld, NOT evicted on first occurrences; evicted at the ~60s wall ceiling', async () => {
  const { guard, evicted, advance } = harness('UNAVAILABLE');
  const m = member('cA');
  guard.relay(ORD, m, 'f1'); await tick();    // first UNAVAILABLE — withhold, do not evict
  assert.equal(evicted.length, 0, 'a single DB blip must not bounce a legitimate courier');
  guard.relay(ORD, m, 'f2'); await tick();
  assert.equal(evicted.length, 0);
  advance(60_000);                            // wall reached
  guard.relay(ORD, m, 'f3'); await tick();    // ceiling fires from in-memory state (no successful DB read)
  assert.equal(evicted.length, 1);
  assert.deepEqual(m.sent, [], 'nothing relayed throughout the outage');
});

test('UNAVAILABLE secondary count is set above the wall frame-rate so the WALL fires first at 1Hz', async () => {
  // maxUnavail must be high enough that 1Hz frames over <60s never trip the count before the wall.
  const { guard, evicted, advance } = harness('UNAVAILABLE', { maxUnavail: 120 });
  const m = member('cA');
  // first read at t0 (elapsed 0), then 59 more at 1s..59s elapsed — count reaches 60, well under 120.
  for (let i = 0; i < 60; i++) { guard.relay(ORD, m, `f${i}`); await tick(); advance(1_000); }
  assert.equal(evicted.length, 0, 'at 1Hz, 60 UNAVAILABLE reads across 0..59s must NOT evict before the 60s wall');
  // t is now 60s past the first UNAVAILABLE → the wall fires (count=61 still < 120, so the WALL is dominant).
  guard.relay(ORD, m, 'last'); await tick();
  assert.equal(evicted.length, 1);
});

test('a fresh ALLOW resets the UNAVAILABLE ceiling (DB recovered)', async () => {
  let v: AuthzVerdict = 'UNAVAILABLE';
  const { guard, evicted, advance } = harness(() => v);
  const m = member('cA');
  guard.relay(ORD, m, 'f1'); await tick();    // unavailable, firstAt recorded
  advance(30_000);
  v = 'ALLOW';
  guard.relay(ORD, m, 'f2'); await tick();    // recovery — resets ceiling state, primes cache
  v = 'UNAVAILABLE';
  advance(40_000);                            // 40s since recovery, but the OLD firstAt was cleared
  guard.relay(ORD, m, 'f3'); await tick();    // fresh firstAt — well under the wall
  assert.equal(evicted.length, 0, 'ceiling is measured from the LATEST unavailable streak, not the first ever');
});

test('non-courier members relay directly with no db read (owner/customer admission is authoritative)', async () => {
  const { guard, checks } = harness('DENY');
  const owner = member('o1', 'owner');
  const cust = member('u1', 'customer');
  assert.equal(guard.relay(ORD, owner, 'f1'), 'relayed');
  assert.equal(guard.relay(ORD, cust, 'f2'), 'relayed');
  assert.deepEqual(owner.sent, ['f1']);
  assert.deepEqual(cust.sent, ['f2']);
  await tick();
  assert.equal(checks.length, 0, 'no binding read for non-courier members');
});

test('courier on a non-order room (orderId=null, e.g. courier:<self>) relays directly with no db read', async () => {
  const { guard, checks } = harness('DENY');
  const m = member('cA');
  assert.equal(guard.relay(null, m, 'f1'), 'relayed');
  assert.deepEqual(m.sent, ['f1']);
  await tick();
  assert.equal(checks.length, 0, 'self-scoped courier room has no order binding to revalidate');
});

test('a closed socket is skipped (no send, no db read)', async () => {
  const { guard, checks } = harness('ALLOW');
  const m = member('cA');
  m.readyState = 3; // CLOSED
  assert.equal(guard.relay(ORD, m, 'f1'), 'skipped');
  assert.deepEqual(m.sent, []);
  await tick();
  assert.equal(checks.length, 0);
});

test('concurrent frames for the same (order,courier) trigger only ONE in-flight re-read', async () => {
  const { guard, checks } = harness('ALLOW');
  const m = member('cA');
  guard.relay(ORD, m, 'f1');
  guard.relay(ORD, m, 'f2');
  guard.relay(ORD, m, 'f3');
  await tick();
  assert.equal(checks.length, 1, 'inflight dedup collapses a burst into one db read');
});

test('LRU bound caps the ALLOW cache', async () => {
  const { guard } = harness('ALLOW', { maxEntries: 3 });
  for (const sub of ['a', 'b', 'c', 'd', 'e']) {
    const m = member(sub);
    guard.relay(ORD, m, 'f'); await tick();
    guard.relay(ORD, m, 'f'); // prime cache entry
  }
  assert.ok(guard._stats().allow <= 3, `allow cache stays bounded: ${guard._stats().allow}`);
});
