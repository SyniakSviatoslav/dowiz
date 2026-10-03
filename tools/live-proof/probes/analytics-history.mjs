// LIVE PROBE (W-HIST P2, P3): the daily sales cube, the owner's analytics v2
// and the menu-engineering matrix, on the QA hub ONLY.
//
//   set -a; . /root/.dowiz_owner; set +a; node tools/live-proof/probes/analytics-history.mjs
//
// Main runs this after a deploy. It never touches a real venue: the host is
// fixed to qa-durres.dowiz.org and the rotation switch is refused by the hub
// anywhere else (`handler::qa_only`).
//
// WHAT IT PROVES, through the real Worker, Durable Object and storage:
//   1. GET /api/owner/analytics?v=2&days=90 validates against analytics.owner.v2.
//   2. POST /api/owner/analytics/history {rotateAsOf: now + 31 days} moves every
//      finished QA order out of the hot log into an archive AND into the cube.
//   3. The same GET afterwards gives THE SAME NUMBERS for every past day: the
//      days now come from the cube, not the hot log (byDay[].archived > 0).
//   4. GET ?verify=1 refolds the newest archived day from its archive and says
//      equal: true (byte for byte).
//   5. GET ?trace=<day> lists records whose `took` adds up to that day's revenue.
//   6. GET /api/owner/analytics/kitchen?v=2&days=62 carries `menu` (analytics.kitchen.v2):
//      every dish is a quadrant, or `costUnknown` with no cost at all.
//   7. A second POST /history adds nothing (a catch-up is idempotent).
// Exit 0 = PROVEN; 1 = FAILED (the line says which); 2 = NEEDS-KEY / NEEDS-DATA
// (a skip is never a pass).
//
// THE ROTATION IS A REAL WRITE: QA orders older than "now + 31 days" minus 30
// days -- every finished one -- leave the hot log for an archive. That is the
// QA hub's purpose; nothing is deleted and every number is still reachable.
import fs from 'node:fs';

const HOST = 'https://qa-durres.dowiz.org';
const UA = 'Mozilla/5.0 (dowiz live-proof analytics-history)';
const DAY = 86400000;
const say = (k, m) => console.log(`${k} ${m}`);
const fail = m => { say('FAILED', m); process.exit(1); };
const need = m => { say('NEEDS', m); process.exit(2); };

// The QA hub's owner: QA_OWNER_* when set, else the OWNER_* of /root/.dowiz_owner (main checks which hub it names).
const email = process.env.QA_OWNER_EMAIL || process.env.OWNER_EMAIL, password = process.env.QA_OWNER_PASSWORD || process.env.OWNER_PASSWORD;
if (!email || !password) need('KEY: QA_OWNER_EMAIL / QA_OWNER_PASSWORD (or OWNER_*) are not set');

const contract = n => JSON.parse(fs.readFileSync(new URL(`../contracts/${n}.json`, import.meta.url), 'utf8'));

/// The JSON Schema subset the contracts use: type, required, properties,
/// items, enum, minItems/maxItems, const. Returns the first violation.
function check(schema, v, at = '$'){
  if (!schema) return null;
  if (schema.const !== undefined && v !== schema.const) return `${at}: expected ${JSON.stringify(schema.const)}, got ${JSON.stringify(v)}`;
  if (schema.enum && !schema.enum.includes(v)) return `${at}: ${JSON.stringify(v)} not in ${JSON.stringify(schema.enum)}`;
  const types = [].concat(schema.type || []);
  const kind = v === null ? 'null' : Array.isArray(v) ? 'array' : Number.isInteger(v) ? 'integer' : typeof v;
  if (types.length && !types.some(t => t === kind || (t === 'number' && kind === 'integer'))) return `${at}: type ${kind}, wanted ${types.join('|')}`;
  if (kind === 'object') {
    for (const r of schema.required || []) if (!(r in v)) return `${at}: missing ${r}`;
    for (const [k, s] of Object.entries(schema.properties || {})) if (k in v) { const e = check(s, v[k], `${at}.${k}`); if (e) return e; }
  }
  if (kind === 'array') {
    if (schema.minItems != null && v.length < schema.minItems) return `${at}: ${v.length} items < ${schema.minItems}`;
    if (schema.maxItems != null && v.length > schema.maxItems) return `${at}: ${v.length} items > ${schema.maxItems}`;
    for (let i = 0; i < v.length; i++) { const e = check(schema.items, v[i], `${at}[${i}]`); if (e) return e; }
  }
  return null;
}

let token = null;
async function call(method, path, body){
  const t0 = Date.now();
  const r = await fetch(HOST + path, { method, headers: { 'user-agent': UA, 'content-type': 'application/json', ...(token ? { authorization: 'Bearer ' + token } : {}) },
    body: body == null ? undefined : JSON.stringify(body) });
  const text = await r.text();
  let json = null; try { json = JSON.parse(text); } catch {}
  return { status: r.status, json, text, ms: Date.now() - t0 };
}

const login = await call('POST', '/api/auth/login', { email, password });
if (login.status !== 200 || !login.json?.access_token) fail(`login ${login.status}: ${login.text.slice(0, 120)}`);
token = login.json.access_token;

const owner = contract('feature-analytics-owner-v2'), cubeC = contract('feature-analytics-cube'), menuC = contract('feature-menu-matrix');

// 1. the pane, before
const before = await call('GET', '/api/owner/analytics?v=2&days=90');
if (before.status !== 200) fail(`analytics ${before.status}: ${before.text.slice(0, 160)}`);
const e1 = check(owner.response_schema, before.json); if (e1) fail(`analytics.owner.v2 schema: ${e1}`);
say('OK', `analytics days=90 in ${before.ms} ms: orders=${before.json.orders} revenue=${before.json.revenue} archivedDays=${before.json.history.archivedDays}`);
const year = await call('GET', '/api/owner/analytics?v=2&days=365');
if (year.status !== 200 || year.json.byDay.length !== 365) fail(`a year: ${year.status} ${year.json?.byDay?.length}`);
say('OK', `analytics days=365 in ${year.ms} ms (wall clock, network included)`);

// 2. rotate as of 31 days from now, then fold
const rot = await call('POST', '/api/owner/analytics/history', { rotateAsOf: Date.now() + 31 * DAY });
if (rot.status !== 200) fail(`history ${rot.status}: ${rot.text.slice(0, 160)}`);
const e2 = check(cubeC.response_schema, rot.json); if (e2) fail(`analytics.cube schema: ${e2}`);
say('OK', `rotated=${JSON.stringify(rot.json.rotated?.rotated ?? null)} moved=${rot.json.rotated?.ordersMoved ?? 0} folded=${rot.json.folded} added=${rot.json.added.length}`);

// 3. the same numbers, now from the cube
const after = await call('GET', '/api/owner/analytics?v=2&days=90');
if (after.status !== 200) fail(`analytics after ${after.status}`);
const today = after.json.to;
for (let i = 0; i < before.json.byDay.length; i++) {
  const a = before.json.byDay[i], b = after.json.byDay[i];
  if (a.day === today) continue; // today may have moved on its own
  if (a.orders !== b.orders || a.revenue !== b.revenue) fail(`day ${a.day} changed across the rotation: ${a.orders}/${a.revenue} -> ${b.orders}/${b.revenue}`);
}
const archived = after.json.byDay.filter(d => d.archived > 0);
if (!archived.length) need('DATA: no finished QA order to archive -- place and finish one, then re-run');
say('OK', `${archived.length} day(s) now read from the cube, every past day unchanged`);

// 4. verify the newest archived day
const v = await call('GET', '/api/owner/analytics?v=2&verify=1');
if (v.status !== 200 || v.json.archived?.equal !== true) fail(`verify ${v.status}: ${v.text.slice(0, 200)}`);
say('OK', `verify ${v.json.day}: stored == refolded from ${v.json.archived.src.join(',')}`);

// 5. trace a day: the records add up to the day's number
const d = archived[archived.length - 1];
const tr = await call('GET', `/api/owner/analytics?v=2&trace=${d.day}`);
if (tr.status !== 200) fail(`trace ${tr.status}`);
const sum = (tr.json.records || []).reduce((s, r) => s + (r.took || 0), 0);
if (sum !== d.revenue) fail(`trace ${d.day}: records add to ${sum}, the pane says ${d.revenue}`);
if (JSON.stringify(tr.json).match(/"phone"|"contact"|"address"/)) fail('trace carries customer data');
say('OK', `trace ${d.day}: ${tr.json.records.length} record(s) add up to ${sum}`);

// 6. the matrix
const k = await call('GET', '/api/owner/analytics/kitchen?v=2&days=62');
if (k.status !== 200) fail(`kitchen ${k.status}`);
const e6 = check(menuC.response_schema, k.json); if (e6) fail(`menu schema: ${e6}`);
for (const x of k.json.menu.dishes) if (x.quadrant == null && (!x.costUnknown || 'cogs' in x)) fail(`dish ${x.id}: no quadrant but a cost`);
say('OK', `menu: ${k.json.menu.count} dish(es), ${k.json.menu.unknown} cost unknown, history.archivedDays=${k.json.history.archivedDays}`);

// 7. idempotent
const again = await call('POST', '/api/owner/analytics/history', {});
if (again.status !== 200 || again.json.added.length !== 0) fail(`second catch-up added ${again.json?.added?.length}`);
say('PROVEN', 'analytics-history: cube, v2 pane, verify, trace and matrix on qa-durres');
