// Cross-role courier-geo validation against dev (dowiz.fly.dev).
// Setup via /api/dev/* (mock-auth + create-assignment), then drive the courier
// map UI + prove the live geo broadcast reaches the customer over WS.
import { chromium, request as pwRequest } from '@playwright/test';

const BASE = 'https://dowiz.fly.dev';
const SECRET = process.env.DEV_AUTH_SECRET;
const REST = { lat: 41.3275, lng: 19.8187 };   // courier start (Tirana center)
const CUST = { lat: 41.3300, lng: 19.8200 };    // delivery pin
const log = [];
const P = (s) => { log.push(s); console.log(s); };

const lerp = (a, b, t) => a + (b - a) * t;
const step = (t) => ({ latitude: lerp(REST.lat, CUST.lat, t), longitude: lerp(REST.lng, CUST.lng, t) });

const api = await pwRequest.newContext({ baseURL: BASE, extraHTTPHeaders: { 'x-dev-auth-secret': SECRET } });

// ── 1. SETUP ──────────────────────────────────────────────────────────────
const ownerAuth = await (await api.post('/api/dev/mock-auth', { data: { role: 'owner', locationSlug: 'demo' } })).json();
const ownerToken = ownerAuth.access_token, locationId = ownerAuth.activeLocationId;
P(`[setup] owner token=${!!ownerToken} location=${locationId}`);

const H = (tok) => ({ Authorization: `Bearer ${tok}` });
const cat = await (await api.post('/api/owner/menu/categories', { data: { name: `Geo-Cat-${Date.now()}` }, headers: H(ownerToken) })).json();
const prod = await (await api.post('/api/owner/menu/products', { data: { name: `Geo-Prod-${Date.now()}`, price: 500, available: true, categoryId: cat.id, stockCount: 50 }, headers: H(ownerToken) })).json();
P(`[setup] product=${prod.id}`);

const courierAuth = await (await api.post('/api/dev/mock-auth', { data: { role: 'courier', locationId } })).json();
const courierToken = courierAuth.access_token, courierId = courierAuth.userId;
P(`[setup] courier token=${!!courierToken} id=${courierId}`);

const orderRes = await api.post('/api/orders', { data: {
  locationId, type: 'delivery', items: [{ product_id: prod.id, quantity: 1 }],
  customer: { phone: `+3556000${String(Date.now()).slice(-6)}`, name: 'Geo Test' },
  delivery: { pin: CUST, address_text: 'Rruga Test, Tirana' },
  payment: { method: 'cash' }, idempotency_key: crypto.randomUUID(),
} });
const order = await orderRes.json();
P(`[setup] order=${order.id} (${orderRes.status()})`);
await api.post(`/api/owner/locations/${locationId}/orders/${order.id}/confirm`, { headers: H(ownerToken) });
const asgn = await (await api.post('/api/dev/create-assignment', { data: { orderId: order.id, courierId, locationId } })).json();
P(`[setup] assignment=${asgn.assignmentId} (status=assigned)`);
// Advance lifecycle so the CUSTOMER broadcast activates: assigned→accepted→picked_up
// (courier-events.ts only broadcasts order.courier_updated for accepted/picked_up/delivered;
//  ETA computes at picked_up). This is the real lifecycle gate, not a workaround.
const accRes = await api.post(`/api/courier/assignments/${asgn.assignmentId}/accept`, { headers: H(courierToken) });
P(`[setup] accept -> ${accRes.status()}`);
const puRes = await api.post(`/api/courier/assignments/${asgn.assignmentId}/picked-up`, { headers: H(courierToken) });
P(`[setup] picked-up -> ${puRes.status()}`);

const result = { courierMap: 'UNKNOWN', adminMap: 'UNKNOWN', customerWS: 'UNKNOWN', geoBroadcast: 'UNKNOWN' };
const browser = await chromium.launch();

// ── 2. COURIER MAP UI ───────────────────────────────────────────────────────
try {
  const ctx = await browser.newContext({ permissions: ['geolocation'], geolocation: { latitude: REST.lat, longitude: REST.lng } });
  await ctx.addInitScript(([t]) => localStorage.setItem('dos_access_token', t), [courierToken]);
  const page = await ctx.newPage();
  const errs = []; page.on('console', m => m.type() === 'error' && errs.push(m.text().slice(0, 70)));
  await page.goto(`${BASE}/courier/delivery/${asgn.assignmentId}`, { waitUntil: 'domcontentloaded', timeout: 25000 });
  await page.waitForTimeout(5000);
  const map = await page.locator('.maplibregl-map, canvas.maplibregl-canvas').count();
  const markers = await page.locator('.maplibregl-marker').count();
  const landed = new URL(page.url()).pathname;
  P(`[courier] landed=${landed} maplibreMap=${map} markers=${markers} consoleErrs=${errs.length}`);
  result.courierMap = (map > 0) ? 'GO' : (landed.includes('login') ? 'NO-GO(auth)' : 'NO-GO(no-map)');
  await ctx.close();
} catch (e) { result.courierMap = 'ERR ' + String(e.message).split('\n')[0].slice(0, 60); }

// ── 3. LIVE WS broadcast (the core): one authed socket subscribes to BOTH the owner
//      channel (location:{id}:couriers) and the customer channel (order:{orderId});
//      real pings drive courier movement → assert both channels receive live updates.
try {
  const wsUrl = `wss://${new URL(BASE).host}/ws?token=${ownerToken}`;
  const ownerRoom = `location:${locationId}:couriers`;
  const orderRoom = `order:${order.id}`;
  const got = { posUpd: [], courierUpd: [] };
  const ws = new WebSocket(wsUrl);
  let opened = false;
  ws.onopen = () => { opened = true; };
  ws.onmessage = (e) => {
    try {
      const d = JSON.parse(e.data); const inner = d.data || d; const t = inner.type;
      if (t === 'auth_success') {
        ws.send(JSON.stringify({ type: 'subscribe', room: ownerRoom }));
        ws.send(JSON.stringify({ type: 'subscribe', room: orderRoom }));
      }
      if (t === 'courier.position_updated') got.posUpd.push(inner.payload || {});
      if (t === 'order.courier_updated') got.courierUpd.push(inner.payload || {});
    } catch {}
  };
  // wait for socket open + subscribe
  for (let i = 0; i < 20 && !opened; i++) await new Promise(r => setTimeout(r, 250));
  await new Promise(r => setTimeout(r, 1500));
  let pingOK = 0; const pingCodes = [];
  for (let i = 0; i <= 3; i++) {
    const s = step(i / 3);
    const r = await api.post('/api/courier/shifts/ping', { data: { lat: s.latitude, lng: s.longitude, accuracy_meters: 8 }, headers: H(courierToken) });
    pingCodes.push(r.status()); if (r.ok()) pingOK++;
    if (i < 3) await new Promise(r => setTimeout(r, 12000));
  }
  await new Promise(r => setTimeout(r, 2500));
  try { ws.close(); } catch {}
  const posPts = new Set(got.posUpd.map(p => `${p.position?.lat ?? p.lat},${p.position?.lng ?? p.lng}`)).size;
  const ordPts = new Set(got.courierUpd.map(p => `${p.position?.lat ?? p.lat},${p.position?.lng ?? p.lng}`)).size;
  const eta = got.courierUpd.at(-1)?.etaSeconds ?? got.courierUpd.at(-1)?.eta ?? 'n/a';
  P(`[ws] opened=${opened} pings=${pingCodes.join(',')}`);
  P(`[ws] OWNER courier.position_updated=${got.posUpd.length} (distinct=${posPts})  CUSTOMER order.courier_updated=${got.courierUpd.length} (distinct=${ordPts}) ETA=${eta}`);
  result.geoBroadcast = pingOK >= 4 ? 'GO' : 'PARTIAL(ping ' + pingOK + ')';
  result.adminLiveWS = got.posUpd.length >= 1 ? 'GO' : 'NO-GO(no courier.position_updated)';
  result.customerLiveWS = got.courierUpd.length >= 1 ? 'GO (eta=' + eta + ')' : 'NO-GO(no order.courier_updated)';
} catch (e) { result.adminLiveWS = result.customerLiveWS = 'ERR ' + String(e.message).split('\n')[0].slice(0, 60); }

// ── 3b. CUSTOMER tracking UI (correct route /s/demo/order/:id; needs customer session) ─
try {
  const ctx = await browser.newContext();
  const page = await ctx.newPage();
  const cerr = []; page.on('console', m => m.type() === 'error' && cerr.push(m.text().slice(0, 60)));
  await page.goto(`${BASE}/s/demo/order/${order.id}`, { waitUntil: 'domcontentloaded', timeout: 25000 });
  await page.waitForTimeout(3500);
  const landed = new URL(page.url()).pathname;
  const body = (await page.locator('body').innerText()).replace(/\s+/g, ' ').slice(0, 70);
  const map = await page.locator('.maplibregl-map').count();
  P(`[customer-ui] landed=${landed} map=${map} body="${body}" errs=${cerr.length}`);
  result.customerUI = map > 0 ? 'GO' : 'NEEDS-CUSTOMER-SESSION (order behind verifyAuth)';
  await ctx.close();
} catch (e) { result.customerUI = 'ERR ' + String(e.message).split('\n')[0].slice(0, 60); }

// ── 4. ADMIN MAP UI ──────────────────────────────────────────────────────────
try {
  const ctx = await browser.newContext();
  await ctx.addInitScript(([t]) => localStorage.setItem('dos_access_token', t), [ownerToken]);
  const page = await ctx.newPage();
  const errs = []; page.on('console', m => m.type() === 'error' && errs.push(m.text().slice(0, 70)));
  await page.goto(`${BASE}/admin/couriers`, { waitUntil: 'domcontentloaded', timeout: 25000 });
  await page.waitForTimeout(5000);
  const map = await page.locator('.maplibregl-map, canvas.maplibregl-canvas').count();
  const markers = await page.locator('.maplibregl-marker').count();
  const landed = new URL(page.url()).pathname;
  P(`[admin] landed=${landed} maplibreMap=${map} markers=${markers} consoleErrs=${errs.length}`);
  result.adminMap = (map > 0) ? 'GO' : (landed.includes('login') ? 'NO-GO(auth)' : 'NO-GO(no-map)');
  await ctx.close();
} catch (e) { result.adminMap = 'ERR ' + String(e.message).split('\n')[0].slice(0, 60); }

await browser.close(); await api.dispose();
P('\n=== VERDICT ===');
for (const [k, v] of Object.entries(result)) P(`  ${k}: ${v}`);
