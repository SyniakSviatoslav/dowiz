// Live multi-courier real-time check against a deployed env.
// Drives 3 couriers delivering at once, pinging distinct GPS positions, and
// verifies the owner dashboard receives ALL of them (real UUID-keyed positions),
// that positions persist (/couriers/live), and that the saved route is
// retrievable. Cross-customer isolation is covered by the decorrelated worker
// unit test (apps/api/tests/courier-multi-delivery.test.ts).
//
// Usage: BASE=https://dowiz-staging.fly.dev SECRET=stg-e2e-secret node scripts/live-multi-courier-check.mjs
import WebSocket from 'ws';

const BASE = process.env.BASE || 'https://dowiz-staging.fly.dev';
const SECRET = process.env.SECRET || 'stg-e2e-secret';
const H = { 'content-type': 'application/json', 'x-dev-auth-secret': SECRET };
const results = [];
const ok = (name, cond, extra = '') => { results.push({ name, pass: !!cond, extra }); console.log(`${cond ? 'PASS' : 'FAIL'}: ${name}${extra ? ' — ' + extra : ''}`); };

const j = async (res) => { try { return await res.json(); } catch { return null; } };
const post = (path, body, auth) => fetch(`${BASE}${path}`, { method: 'POST', headers: auth ? { ...H, authorization: `Bearer ${auth}` } : H, body: JSON.stringify(body) });
const get = (path, auth) => fetch(`${BASE}${path}`, { headers: auth ? { ...H, authorization: `Bearer ${auth}` } : H });

const main = async () => {
  // 1. Owner + location
  const owner = await j(await post('/api/dev/mock-auth', {}));
  const LOC = owner?.activeLocationId;
  ok('mock-auth owner + location', owner?.access_token && LOC, LOC);
  if (!LOC) return finish();

  const settings = await j(await get('/api/owner/settings', owner.access_token));
  const baseLat = Number.isFinite(settings?.lat) ? settings.lat : 41.331;
  const baseLng = Number.isFinite(settings?.lng) ? settings.lng : 19.817;

  // 2. Three couriers
  const couriers = [];
  for (let i = 0; i < 3; i++) {
    const c = await j(await post('/api/dev/mock-auth', { role: 'courier' }));
    couriers.push({ courierId: c.userId, token: c.access_token, pos: { lat: +(baseLat + 0.001 * (i + 1)).toFixed(6), lng: +(baseLng + 0.001 * (i + 1)).toFixed(6) } });
  }
  ok('minted 3 courier tokens', couriers.every(c => c.courierId && c.token));

  // 3. Three orders to assign (reuse existing demo orders)
  const orders = await j(await get('/api/owner/orders', owner.access_token));
  const orderIds = (Array.isArray(orders) ? orders : []).slice(0, 3).map(o => o.id);
  ok('found >=3 existing orders to assign', orderIds.length >= 3, `${orderIds.length} available`);

  // 4. Seed assignment (creates courier row + available shift + assignment)
  for (let i = 0; i < couriers.length && i < orderIds.length; i++) {
    await post('/api/dev/create-assignment', { orderId: orderIds[i], courierId: couriers[i].courierId, locationId: LOC });
  }

  // 5. Owner subscribes to the live couriers room
  const received = new Map(); // courierId -> position
  const ws = new WebSocket(`${BASE.replace('https', 'wss')}/ws?token=${owner.access_token}`);
  await new Promise((res, rej) => { ws.on('open', res); ws.on('error', rej); });
  await new Promise((res) => {
    ws.on('message', (d) => {
      const m = JSON.parse(d.toString());
      if (m.type === 'auth_success') { ws.send(JSON.stringify({ type: 'subscribe', room: `location:${LOC}:couriers` })); }
      if (m.type === 'subscribed') res();
      const env = m.data;
      if (env?.type === 'courier.position_updated' && env.payload?.position) {
        received.set(env.payload.courierId, env.payload.position);
      }
    });
  });
  ok('owner subscribed to its location:couriers room (H1 allows own location)', true);

  // 6. All couriers ping distinct positions concurrently
  const pings = await Promise.all(couriers.map(c => post('/api/courier/shifts/ping', { lat: c.pos.lat, lng: c.pos.lng }, c.token)));
  const pingCodes = pings.map(p => p.status);
  ok('all courier pings accepted (200)', pingCodes.every(s => s === 200), `codes=${pingCodes.join(',')}`);

  // 7. Wait for fan-out, then assert the owner saw every courier with its own position
  await new Promise(r => setTimeout(r, 8000));
  const seenAll = couriers.every(c => received.has(c.courierId));
  ok('owner dashboard received a position for ALL 3 couriers', seenAll, `${received.size} distinct couriers`);
  const positionsMatch = couriers.every(c => {
    const got = received.get(c.courierId);
    return got && Math.abs(got.lat - c.pos.lat) < 1e-4 && Math.abs(got.lng - c.pos.lng) < 1e-4;
  });
  ok('each courier reported its OWN distinct position', positionsMatch);
  const distinct = new Set([...received.values()].map(p => `${p.lat},${p.lng}`));
  ok('positions are distinct per courier (no collision)', distinct.size === received.size, `${distinct.size} unique`);
  ws.close();

  // 8. Persistence: /couriers/live shows the couriers with last-known positions
  const live = await j(await get(`/api/owner/locations/${LOC}/couriers/live`, owner.access_token));
  const livePosCount = (live?.couriers || []).filter(c => c.position).length;
  ok('GET /couriers/live returns couriers with persisted positions', livePosCount >= 3, `${livePosCount} with position`);

  // 9. Saved route is retrievable for an assigned order
  const route = await j(await get(`/api/owner/locations/${LOC}/orders/${orderIds[0]}/route`, owner.access_token));
  ok('GET order route returns the saved breadcrumb trail', Array.isArray(route?.points) && route.points.length >= 1, `${route?.points?.length ?? 0} points`);

  finish();
};

const finish = () => {
  const failed = results.filter(r => !r.pass);
  console.log(`\n${results.length - failed.length}/${results.length} checks passed`);
  process.exit(failed.length ? 1 : 0);
};

main().catch(e => { console.error('ERROR', e?.message); process.exit(1); });
