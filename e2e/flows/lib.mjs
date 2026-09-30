// THE FLOWS GATE'S PLUMBING: the one venue it may touch, the credentials, the
// API with the owner's and the courier's tokens, and the state a run carries
// from flow to flow (so F4 can close whatever F1..F3 opened, even when they
// failed half way).
//
// qa-durres ONLY. The host is fixed; FLOWS_HOST may name another `qa-` venue
// and nothing else, because every flow here places orders and edits a dish.
import fs from 'node:fs';
import os from 'node:os';
import dns from 'node:dns';
// Measured 2026-09-30: undici picked Cloudflare's IPv6 address and timed out
// six connects in a row; IPv4 first.
dns.setDefaultResultOrder('ipv4first');
// Box network: TCP connect to Cloudflare takes 4-11 s and undici gives up at
// 10 s; a connect that timed out never reached the server, so it is retried.
import '../kit-regression/_retry-fetch.mjs';

export const HOST = process.env.FLOWS_HOST || 'https://qa-durres.dowiz.org';
export const LOC = new URL(HOST).hostname.split('.')[0];
if (!/^qa-/.test(LOC)) { console.log(`flows: FAIL refusing to run against ${LOC}: the flows write orders, qa-* venues only`); process.exit(2); }

/// Every order and object this run creates carries RUN in its name, which is
/// what F4's final sweep looks for.
export const PREFIX = 'FLOWS-';
export const RUN = process.env.FLOWS_RUN || `${PREFIX}${Date.now().toString(36)}`;
export const OUT = process.env.FLOWS_OUT || `${os.tmpdir()}/dowiz-flows/${RUN}`;
fs.mkdirSync(OUT, { recursive: true });

/// `/root/.dowiz_owner` holds `export K=V` lines. Values are read into this
/// process and never printed.
export const creds = Object.fromEntries(fs.readFileSync(process.env.DOWIZ_OWNER_FILE || '/root/.dowiz_owner', 'utf8')
  .split('\n').filter(l => l.startsWith('export ')).map(l => {
    const s = l.slice(7); const i = s.indexOf('=');
    return [s.slice(0, i), s.slice(i + 1).replace(/^['"]|['"]$/g, '')];
  }));

/// A flow's failure: the first broken assertion ends the flow, with its reason.
export class Fail extends Error {}

/// State shared by the flows of one run, mirrored to disk after every write.
///
/// WHAT MUST BE PUT BACK OUTLIVES THE RUN. `restore` (the venue's pickup
/// switch, the dish's name, the courier's shift, as FOUND) lives in a file per
/// venue that only a successful F4 empties: measured 2026-09-30, a run whose
/// F4 died on the network left pickup on, and the next run recorded "on" as
/// the state it found.
export const PENDING = process.env.FLOWS_PENDING || `${os.tmpdir()}/dowiz-flows/pending-${LOC}.json`;
const readPending = () => { try { return JSON.parse(fs.readFileSync(PENDING, 'utf8')); } catch { return {}; } };
export const S = { run: RUN, orders: [], restore: readPending() };
export const save = () => {
  fs.writeFileSync(`${OUT}/state.json`, JSON.stringify(S, null, 1));
  fs.writeFileSync(PENDING, JSON.stringify(S.restore));
};
/// F4, once everything it had to put back is back.
export const restored = () => { S.restore = {}; save(); };

const redact = t => String(t).replace(/"(jwt|token|access_token|refresh_token|password)":"[^"]+"/g, '"$1":"…"');

/// One HTTP call. Cloudflare's "Worker exceeded resource limits" 503 is
/// retried for READS only (a write that 503'd may have landed), and every one
/// is printed -- never swallowed.
export async function api(path, { method = 'GET', body, token, headers = {} } = {}) {
  for (let i = 0; ; i++) {
    const r = await fetch(`${HOST}${path}`, {
      method,
      headers: { 'content-type': 'application/json', 'user-agent': UA, ...(token ? { authorization: `Bearer ${token}` } : {}), ...headers },
      body: body == null ? undefined : JSON.stringify(body),
    });
    const t = await r.text();
    if (r.status === 503 && method === 'GET' && i < 2) { console.log(`  !! api 503 ${method} ${path.split('?')[0]} (try ${i + 1}): ${t.slice(0, 80)}`); continue; }
    let b; try { b = JSON.parse(t); } catch { b = t; }
    return { status: r.status, body: b, text: redact(t).slice(0, 300) };
  }
}

// Cloudflare's bot rules answer node's default UA with 403 1010 (memory:
// dowiz-hub-owner-credentials); every call sends a browser's.
export const UA = 'Mozilla/5.0 (Linux; Android 14; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Mobile Safari/537.36';

let ownerTok = null;
/// The owner's token for THIS venue (the owner has several; login names it).
export async function owner() {
  if (ownerTok) return ownerTok;
  const r = await api('/api/auth/login', { method: 'POST', body: { email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD, location_id: LOC } });
  if (!r.body?.access_token) throw new Fail(`owner API login ${r.status}: ${r.text.slice(0, 120)}`);
  return (ownerTok = r.body.access_token);
}
export const own = async (path, body, method = body === undefined ? 'GET' : 'POST') =>
  api(path, { method, body, token: await owner() });

/// The courier credential that belongs to THIS venue. QA_COURIER_* is a
/// sushi-durres courier (memory: courier-login-venue-from-host); the qa hub's
/// is QA_HUB_COURIER_*, made by W-VIDEO on 2026-09-27.
export const courierCreds = () => ({ phone: creds.QA_HUB_COURIER_PHONE, password: creds.QA_HUB_COURIER_PASSWORD });

/// An owner-side read of every order of the venue.
export async function ownerOrders() {
  const r = await own(`/api/owner/orders?location_id=${LOC}`);
  if (r.status !== 200) throw new Fail(`GET /api/owner/orders ${r.status}: ${r.text.slice(0, 120)}`);
  return Array.isArray(r.body) ? r.body : (r.body.orders || []);
}
export async function ownerOrder(id) { return (await ownerOrders()).find(o => o.id === id) || null; }

/// The status the API holds for an order: the customer's link first (what the
/// status page itself reads), the owner's list otherwise.
export async function statusOf(id) {
  const o = S.orders.find(x => x.id === id);
  if (o?.token) {
    const r = await api(`/api/order/${encodeURIComponent(id)}`, { token: o.token });
    if (r.status === 200 && r.body?.status) return r.body.status;
  }
  return (await ownerOrder(id))?.status;
}

/// The venue's public menu, fresh (the edge cache answers stale for ~30 s).
export async function menu() {
  const r = await api(`/api/public/locations/${LOC}/menu?fresh=1`);
  if (r.status !== 200) throw new Fail(`public menu ${r.status}: ${r.text.slice(0, 120)}`);
  return r.body;
}
export const dishes = m => (m.categories || []).flatMap(c => c.products || []);

/// Remember an order this run opened, so F4 closes it whatever happens next.
export function opened(id, token, by) { S.orders.push({ id, token: token || null, by }); save(); }

export const TERMINAL = new Set(['DELIVERED', 'CANCELLED', 'REJECTED', 'PICKED_UP', 'COMPENSATED_REFUND']);
export const sleep = ms => new Promise(r => setTimeout(r, ms));
