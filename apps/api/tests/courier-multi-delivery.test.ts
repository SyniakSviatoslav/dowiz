import test from 'node:test';
import assert from 'node:assert/strict';

// These modules chain to @deliveryos/db, which calls loadEnv() at import time → a static import
// crashes here with no env. Load them DYNAMICALLY after ensureEnv() (same pattern as
// orders-guards.test.ts). This is a pure unit test (bus + lookups stubbed) — no real DB/Redis.
let CourierEventsWorker: any;
let orderChannel: (id: string) => string;
let courierChannel: (id: string) => string;
function ensureEnv() {
  const d: Record<string, string> = {
    NODE_ENV: 'test',
    DATABASE_URL_OPERATIONAL: 'postgres://u:p@localhost:5432/db',
    DATABASE_URL_SESSION: 'postgres://u:p@localhost:5432/db',
    DATABASE_URL_MIGRATIONS: 'postgres://u:p@localhost:5432/db',
    REDIS_URL: 'redis://localhost:6379',
    JWT_PRIVATE_KEY: 'test', JWT_PUBLIC_KEY: 'test', JWT_KID: 'test',
    IP_HASH_SALT: 'test', APP_BASE_URL: 'http://localhost:3000',
    GOOGLE_CLIENT_ID: 'test', GOOGLE_CLIENT_SECRET: 'test',
    VAPID_PUBLIC_KEY: 'test', VAPID_PRIVATE_KEY: 'test',
  };
  for (const [k, v] of Object.entries(d)) if (!process.env[k]) process.env[k] = v;
}

// Concurrency proof: when 2-3 couriers are delivering AT THE SAME TIME, the
// position fan-out must be correct and isolated per role:
//   • Owner dashboard (location:<id>:couriers) sees EVERY on-shift courier move,
//     keyed by real courier UUID — including idle couriers with no active order.
//   • Each customer (order:<orderId>) sees ONLY their own courier — courier A's
//     position must never land in courier B's order room (no cross-order leak).
//   • An idle courier (no assignment) appears on the dashboard but produces NO
//     customer fan-out.
//
// Decorrelated from DB/crypto/routing: we stub the two data lookups and the
// message bus, exercising only handlePositionUpdated's routing logic — the exact
// code that decides "who sees which courier".

const LOC = 'loc-1';

// Per-courier fixtures: 3 active deliveries + 1 idle courier, distinct positions.
const COURIERS = {
  A: { order: 'order-A', pos: { lat: 41.11, lng: 19.11 }, dest: { lat: 41.20, lng: 19.20 } },
  B: { order: 'order-B', pos: { lat: 41.22, lng: 19.22 }, dest: { lat: 41.30, lng: 19.30 } },
  C: { order: 'order-C', pos: { lat: 41.33, lng: 19.33 }, dest: { lat: 41.40, lng: 19.40 } },
  D: { order: null,      pos: { lat: 41.44, lng: 19.44 }, dest: null }, // idle, on shift
};

function makeWorker() {
  const published: Array<{ channel: string; msg: any }> = [];
  const bus = { publish: async (channel: string, msg: any) => { published.push({ channel, msg }); } };
  const worker = new CourierEventsWorker({} as any, bus as any);

  // Stub the DB lookups: position is known for every on-shift courier; order
  // details exist only for couriers with an active assignment.
  (worker as any).fetchLatestPosition = async (courierId: string) => {
    const c = (COURIERS as any)[courierId];
    return c ? c.pos : null;
  };
  (worker as any).fetchCourierDetailsAndOrder = async (courierId: string) => {
    const c = (COURIERS as any)[courierId];
    if (!c || !c.order) return null; // idle courier → no customer fan-out
    return {
      orderId: c.order,
      courierName: `${courierId}***`,
      phoneMasked: '+*** *** 0000',
      position: c.pos,
      destination: c.dest,
      assignmentStatus: 'accepted',
    };
  };

  return { worker, published };
}

test('multi-courier concurrent position fan-out is correct and isolated', async (t) => {
  ensureEnv();
  ({ CourierEventsWorker } = await import('../src/workers/courier-events.js'));
  ({ orderChannel, courierChannel } = await import('../src/lib/registry.js'));
  const { worker, published } = makeWorker();

  // Fire all four couriers' position updates concurrently (real-time burst).
  await Promise.all(
    (['A', 'B', 'C', 'D'] as const).map((id) =>
      worker.handlePositionUpdated({ courierId: id, locationId: LOC, shiftId: `shift-${id}` }),
    ),
  );

  const dash = published.filter((p) => p.channel === courierChannel(LOC));
  const orderMsgs = published.filter((p) => p.channel.startsWith('order:'));

  await t.test('owner dashboard sees every on-shift courier (incl. idle), keyed by UUID', () => {
    const byCourier = new Map(dash.map((p) => [p.msg.payload.courierId, p.msg.payload.position]));
    for (const id of ['A', 'B', 'C', 'D'] as const) {
      assert.ok(byCourier.has(id), `dashboard missing courier ${id}`);
      assert.deepEqual(byCourier.get(id), (COURIERS as any)[id].pos, `dashboard position wrong for ${id}`);
      assert.equal(dash.every((p) => p.msg.type === 'courier.position_updated'), true);
    }
    assert.equal(byCourier.size, 4, 'expected exactly 4 distinct couriers on the dashboard');
  });

  await t.test('each customer sees ONLY their own courier (no cross-order leak)', () => {
    for (const id of ['A', 'B', 'C'] as const) {
      const room = orderChannel((COURIERS as any)[id].order);
      const forOrder = orderMsgs.filter((p) => p.channel === room);
      assert.equal(forOrder.length, 1, `order ${id} should get exactly one courier update`);
      const m = forOrder[0].msg;
      assert.equal(m.type, 'order.courier_updated');
      assert.equal(m.payload.orderId, (COURIERS as any)[id].order);
      assert.equal(m.payload.courierName, `${id}***`, `order ${id} got the wrong courier`);
      assert.deepEqual(m.payload.position, (COURIERS as any)[id].pos, `order ${id} got the wrong position`);
    }
  });

  await t.test('no order room ever receives a different courier’s position', () => {
    const allPositions = { A: COURIERS.A.pos, B: COURIERS.B.pos, C: COURIERS.C.pos, D: COURIERS.D.pos };
    for (const id of ['A', 'B', 'C'] as const) {
      const room = orderChannel((COURIERS as any)[id].order);
      const foreign = (['A', 'B', 'C', 'D'] as const).filter((x) => x !== id);
      for (const p of orderMsgs.filter((m) => m.channel === room)) {
        for (const f of foreignCheck(foreign, allPositions, p.msg.payload.position)) {
          assert.fail(`order ${id} room leaked courier ${f}'s position`);
        }
      }
    }
  });

  await t.test('idle courier D produces NO customer fan-out', () => {
    const leaked = orderMsgs.filter((p) => p.msg.payload.courierName === 'D***');
    assert.equal(leaked.length, 0, 'an idle courier must not push to any order room');
    assert.equal(orderMsgs.length, 3, 'exactly 3 active deliveries → 3 order updates');
  });
});

// Return the foreign courier ids whose position equals the observed one.
function foreignCheck(
  foreign: readonly string[],
  positions: Record<string, { lat: number; lng: number }>,
  observed: { lat: number; lng: number },
): string[] {
  return foreign.filter((f) => positions[f].lat === observed.lat && positions[f].lng === observed.lng);
}
