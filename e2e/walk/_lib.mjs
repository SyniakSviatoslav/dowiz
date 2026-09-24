// Live-walk helpers: one browser, capped processes, every console error,
// failed request and CSP violation collected per screen.
import { chromium } from 'playwright';
import fs from 'node:fs';

export const HOST = process.env.HOST || 'https://sushi-durres.dowiz.org';
export const OUT = process.env.OUT || '/tmp/claude-0/-root/9cc00e2e-a227-4b36-a0f5-13ae2c40b743/scratchpad/walk';
fs.mkdirSync(OUT, { recursive: true });
export const creds = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8')
  .split('\n').filter(l => l.startsWith('export ')).map(l => { const s = l.slice(7); const i = s.indexOf('='); return [s.slice(0, i), s.slice(i + 1).replace(/^['"]|['"]$/g, '')]; }));
export const STATE = `${OUT}/state.json`;
export const st = () => { try { return JSON.parse(fs.readFileSync(STATE, 'utf8')); } catch { return {}; } };
export const save = patch => fs.writeFileSync(STATE, JSON.stringify({ ...st(), ...patch }, null, 1));

export const log = [];
export const say = (ok, name, detail = '') => {
  const l = `${ok === null ? 'info' : ok ? 'ok  ' : 'FAIL'} ${name}${detail ? ' :: ' + detail : ''}`;
  log.push(l); console.log(l);
};
export const issues = [];
export const cfNoise = { n: 0 };
export function watch(p, tag) {
  p.on('pageerror', e => { issues.push(`${tag} pageerror ${e.message.slice(0, 200)}`); console.log(`  !! ${tag} pageerror ${e.message.slice(0, 200)}`); });
  p.on('console', m => {
    if (m.type() !== 'error' && !/Content Security Policy|Refused to/.test(m.text())) return;
    if (/WebGL|GPU/i.test(m.text())) return;
    // Cloudflare's injected bot script (not the product): its inline <script> and the
    // report-only policy it brings are counted, not listed (memory: dowiz-landing-2026-09-20).
    if (/policy is report-only|Executing inline script violates/.test(m.text())) { cfNoise.n++; return; }
    issues.push(`${tag} console ${m.text().slice(0, 240)}`); console.log(`  !! ${tag} console ${m.text().slice(0, 240)}`);
  });
  p.on('response', r => {
    if (r.status() >= 400) { const s = `${tag} http ${r.status()} ${r.request().method()} ${r.url().replace(HOST, '').slice(0, 120)}`; issues.push(s); console.log('  !! ' + s); }
  });
  p.on('requestfailed', r => { const s = `${tag} reqfail ${r.failure()?.errorText} ${r.url().replace(HOST, '').slice(0, 120)}`; issues.push(s); console.log('  !! ' + s); });
  p.addInitScript(() => {
    window.__csp = [];
    document.addEventListener('securitypolicyviolation', e => { if (e.disposition === 'report') { window.__cspReport = (window.__cspReport || 0) + 1; return; } window.__csp.push(`${e.violatedDirective} ${e.blockedURI} ${e.sourceFile}:${e.lineNumber}`); });
  });
}
export async function csp(p, tag) {
  const v = await p.evaluate(() => window.__csp || []).catch(() => []);
  for (const x of v.filter(x => !/^script-src-elem inline /.test(x))) { issues.push(`${tag} CSP ${x}`); console.log(`  !! ${tag} CSP ${x}`); }
  return v;
}

export function procs() { return fs.readdirSync('/proc').filter(n => /^\d+$/.test(n)).length; }
export async function browser() {
  const n = procs();
  if (n > 24) { console.log(`process count ${n} > 24: refusing to launch (lane card: API-only until it drops)`); process.exit(3); }
  return chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-zygote', '--single-process', '--renderer-process-limit=1', '--disable-gpu', '--disable-3d-apis', '--disable-extensions'] });
}
// Cloudflare's "Worker exceeded resource limits" 503 flapped on ~15% of requests (menu, both
// logins, till tips) during the 2026-09-24 walk. A read-back is retried (bounded: 3 tries) so a
// flap does not hide the step behind it -- but every one is printed and counted, never swallowed.
export const flaps = [];
export async function api(path, { method = 'GET', body, token, headers = {} } = {}) {
  for (let i = 0; ; i++) {
    const r = await fetch(`${HOST}${path}`, { method, headers: { 'content-type': 'application/json', ...(token ? { authorization: 'Bearer ' + token } : {}), ...headers }, body: body == null ? undefined : JSON.stringify(body) });
    const t = await r.text();
    const limit = r.status === 503 && /exceeded resource limits/.test(t);
    if (limit) { flaps.push(`${method} ${path.split('?')[0]}`); console.log(`  !! 503 Worker exceeded resource limits: ${method} ${path.split('?')[0]} (try ${i + 1})`); }
    // A write that 503'd may or may not have landed; only reads and logins are retried.
    if (limit && i < 2 && (method === 'GET' || /\/login$/.test(path))) continue;
    let b; try { b = JSON.parse(t); } catch { b = t; }
    return { status: r.status, body: b };
  }
}
export async function ownerToken() {
  const r = await api('/api/auth/login', { method: 'POST', body: { email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD } });
  return r.body.access_token;
}
export async function staffToken() {
  const r = await api('/api/staff/login', { method: 'POST', body: { email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD } });
  return r.body.jwt;
}
export function finish() {
  fs.appendFileSync(`${OUT}/log.txt`, log.join('\n') + '\n' + issues.map(i => 'ISSUE ' + i).join('\n') + '\n');
  console.log(`\n${issues.length} issues (+${cfNoise.n} Cloudflare-injected script/report-only lines not listed)`);
  if (flaps.length) console.log(`${flaps.length} API calls answered 503 "Worker exceeded resource limits": ${[...new Set(flaps)].join(', ')}`);
}

// ── the per-role suite (guest/owner/waiter/kitchen/courier .mjs) ────────────
// One line per step: PASS / FAIL / INFO, with the API read-back as evidence.
// `end()` exits with the number of FAILs, so a phase is a test.
export const SUITE = `${OUT}/suite.json`;
export const sst = () => { try { return JSON.parse(fs.readFileSync(SUITE, 'utf8')); } catch { return {}; } };
export const ssave = patch => { const cur = sst(); for (const [k, v] of Object.entries(patch)) cur[k] = v; fs.writeFileSync(SUITE, JSON.stringify(cur, null, 1)); };
let fails = 0;
export function step(ok, name, evidence = '') {
  const ev = typeof evidence === 'string' ? evidence : JSON.stringify(evidence);
  const l = `${ok === null ? 'INFO' : ok ? 'PASS' : 'FAIL'} ${name}${ev ? ' :: ' + ev.slice(0, 700) : ''}`;
  if (ok === false) fails++;
  log.push(l); console.log(l);
  return ok;
}
export async function end(b) {
  try { await b?.close(); } catch {}
  finish();
  console.log(`${fails} FAIL`);
  process.exit(fails);
}
export const LOC = process.env.LOC || 'sushi-durres';
export const wait = (p, ms) => p.waitForTimeout(ms);
/// Owner console sign-in; answers the page.
export async function consoleIn(ctx, tag = 'console') {
  const o = await ctx.newPage(); watch(o, tag);
  await o.goto(`${HOST}/admin/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await o.waitForSelector('#e', { timeout: 40000 });
  await o.fill('#e', creds.OWNER_EMAIL); await o.fill('#p', creds.OWNER_PASSWORD);
  // The sign-in is tapped again while the platform answers 503 "Worker exceeded resource limits"
  // (about half of API calls on 2026-09-24); each 503 is still listed by watch().
  for (let i = 0; i < 6; i++) {
    await o.click('#go').catch(() => {});
    if (await o.waitForSelector('#nav:not([hidden])', { timeout: 15000 }).then(() => true).catch(() => false)) break;
    console.log(`  !! ${tag}: sign-in did not land (try ${i + 1})`);
  }
  await o.waitForTimeout(2500);
  return o;
}
/// Open a More tile's sheet by its key; answers the sheet's text.
export async function tile(o, key, ms = 3500) {
  await o.click('#nav [data-tab="more"]'); await o.waitForTimeout(1500);
  await o.click(`[data-open="${key}"]`); await o.waitForTimeout(ms);
  return o.$eval('#sheetIn', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
}
export async function closeSheet(o) { await o.keyboard.press('Escape'); await o.waitForTimeout(700); }
/// Room app sign-in with any staff credentials (or claim with a code).
export async function roomIn(ctx, email, password, code = null, tag = 'room') {
  const p = await ctx.newPage(); watch(p, tag);
  let answer = null;
  p.on('response', async r => { if (/\/api\/staff\/(login|claim)$/.test(r.url())) { answer = { status: r.status(), body: (await r.text().catch(() => '')).slice(0, 200).replace(/"(jwt|token|access_token|refresh_token)":"[^"]+"/g, '"$1":"…"') }; } });
  await p.goto(`${HOST}/room/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await p.waitForSelector('form[data-form="login"]', { timeout: 40000 });
  if (code) { await p.click('[data-act="claimToggle"]'); await p.waitForTimeout(400); }
  await p.fill('input[name="email"]', email);
  if (code) await p.fill('input[name="code"]', code);
  await p.fill('input[name="password"]', password);
  await p.click('form[data-form="login"] button[type="submit"]');
  for (let i = 0; i < 30 && !answer; i++) await p.waitForTimeout(500);
  await p.waitForTimeout(2500);
  return { p, answer };
}
/// Storefront, ready to tap: the install/consent sheets dismissed.
export async function storeIn(ctx, path = '/', tag = 'store') {
  const c = await ctx.newPage(); watch(c, tag);
  await c.goto(`${HOST}${path}`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await c.waitForSelector('.card', { timeout: 60000 });
  await c.waitForTimeout(3500);
  await dismiss(c);
  return c;
}
/// Close what the storefront opens by itself on a first visit -- never a sheet a link opened.
export async function dismiss(c, keep = ['bookMine', 'track', 'book']) {
  for (let i = 0; i < 6; i++) {
    const n = await c.evaluate(() => document.getElementById('sheet')?.dataset.name || '');
    if (!n || keep.includes(n)) return;
    const l = await c.$('#insLater'); if (l) await l.click().catch(() => {}); else await c.evaluate(() => document.getElementById('scrim')?.click());
    await c.waitForTimeout(600);
  }
}
/// Staff (room) token for any staff email/password.
export async function staffLogin(email, password) {
  const r = await api('/api/staff/login', { method: 'POST', body: { email, password } });
  return { status: r.status, jwt: r.body?.jwt, body: r.body };
}
/// An owner-side read of one order, by id, from the owner's list.
export async function ownerOrder(tok, id) {
  const r = await api(`/api/owner/orders?location_id=${LOC}`, { token: tok });
  const l = Array.isArray(r.body) ? r.body : (r.body.orders || []);
  return l.find(x => x.id === id) || null;
}

/// A screen that says "HTTP 503 ... Try again" (the platform's resource-limit flap) is retried by
/// tapping its own retry button, at most `n` times; answers how many taps it took.
export async function heal503(p, n = 5) {
  let i = 0;
  for (; i < n; i++) {
    const txt = await p.evaluate(() => document.body.innerText).catch(() => '');
    if (!/HTTP 503/.test(txt)) return i;
    console.log(`  !! screen shows HTTP 503, tapping retry (${i + 1})`);
    const b = await p.$('#retry, [data-act="retry"], button:has-text("Provo"), button:has-text("Try again"), button:has-text("Спробувати")');
    if (b) await b.click().catch(() => {}); else await p.reload({ waitUntil: 'domcontentloaded' }).catch(() => {});
    await p.waitForTimeout(3500);
  }
  return i;
}
