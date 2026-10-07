// THE CANVAS SURFACES' BROWSER GATES (Wave CV row CV7): dom-count, first-frame, frame-cpu, no
// canvas doubling, context-loss. One headless Chromium harness, one check per run:
//
//   node tools/gates/canvas.mjs <dom|frame|ctxloss> [--canvas-dir DIR] [--out FILE]
//
// It serves workers/api/public on 127.0.0.1 (with /room/canvas/ taken from --canvas-dir when
// given -- the .prove.sh scripts point it at a broken copy) and answers /api/* from a fixture:
// a signed-in counter-manager (caps advance + take_orders) and 30 tickets + 6 tables. That makes
// these gates the FAST layer (in-memory fixtures, software raster on the box); the live proof on
// qa-durres is tools/live-proof/probes/feature-canvas-room.mjs, run by main after deploy.
//
// THRESHOLDS (docs/research/2026-10-06-canvas-rust-webgl-ui.md §5.3; the card's CV7 subset):
//   dom      body has exactly 1 element at rest (signed out AND signed in); focusing a field with
//            EditContext forced off (?noec=1) adds exactly 1 transient element and blur removes it
//   frame    first frame <= 300 ms from navigation at dpr 2 (median of 3 fresh contexts); 120 wheel frames p99 CPU <= 8 ms;
//            backing store == innerWidth x dpr after 5 resizes and 30 frames (no doubling);
//            something is drawn (pixels that are not the page colour)
//   ctxloss  the backing store is wiped and the context's state reset, then 'contextrestored'
//            fires: the frame hash AND the pixel hash equal the frame before. Canvas2D only:
//            a real GPU loss cannot be triggered on a 2D context, and the WebGL path is not
//            built (this box rasterises nothing on WebGL -- memory webgl-renders-nothing-on-the-box)
// Exit 1 on a failed threshold, 2 when the harness itself could not run (never a pass).
import { createServer } from 'node:http';
import { readFile, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { extname, join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const PUB = join(ROOT, 'workers/api/public');
const args = process.argv.slice(2);
const check = args[0];
const opt = k => { const i = args.indexOf(k); return i > 0 ? args[i + 1] : null; };
const CANVAS = resolve(opt('--canvas-dir') || join(PUB, 'room/canvas'));
const OUT = opt('--out');
const DPR = Number(opt('--dpr') || 2);

function playwright() {
  for (const base of [ROOT, '/root/dowiz']) {
    try { return createRequire(join(base, 'package.json'))('playwright'); } catch {}
  }
  console.log('canvas: playwright is not installed (looked in this tree and /root/dowiz) -- NOT a pass'); process.exit(2);
}

const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.wasm': 'application/wasm', '.css': 'text/css', '.json': 'application/json' };
function fixture(now) {
  const orders = [];
  const st = ['PENDING', 'CONFIRMED', 'PREPARING', 'READY'];
  for (let i = 0; i < 30; i++) {
    orders.push({ id: `ord_fixture_${String(i).padStart(4, '0')}`, status: st[i % 4], created_at_ms: now - i * 90_000,
      fulfilment: i % 3 ? { kind: 'dine_in', table: String(1 + (i % 9)) } : { kind: i % 2 ? 'delivery' : 'pickup', note: 'Pa qepë' },
      kitchen: { seen: i % 2 === 0 },
      items: [{ name: 'Salmon nigiri', quantity: 2, station: 'sushi' }, { name: 'Supë miso', quantity: 1, note: 'pa qepë' }, { name: 'Limonadë', quantity: 1, station: 'bar' }] });
  }
  const sittings = [];
  for (let i = 0; i < 6; i++) sittings.push({ sitting_id: `sit_${i}`, table: String(i + 1), rounds: [{ id: `r${i}`, status: 'PREPARING', total: 1500 + i * 100, payment_status: 'unpaid' }] });
  return { orders, sittings };
}

function serve() {
  const now = Date.now();
  const fx = fixture(now);
  const srv = createServer(async (q, res) => {
    const path = decodeURIComponent(q.url.split('?')[0]);
    if (path.startsWith('/api/')) {
      const body = path === '/api/staff/kitchen' ? { orders: fx.orders, generation: 1, full: true }
        : path === '/api/staff/room' ? { sittings: fx.sittings } : { ok: true };
      res.writeHead(200, { 'content-type': 'application/json' }); res.end(JSON.stringify(body)); return;
    }
    const rel = path.endsWith('/') ? path + 'index.html' : path;
    const file = rel.startsWith('/room/canvas/') ? join(CANVAS, rel.slice('/room/canvas/'.length)) : join(PUB, rel);
    try { const b = await readFile(file); res.writeHead(200, { 'content-type': TYPES[extname(file)] || 'application/octet-stream' }); res.end(b); }
    catch { res.writeHead(404); res.end(); }
  });
  return new Promise(r => srv.listen(0, '127.0.0.1', () => r(srv)));
}

const SESSION = JSON.stringify({ jwt: 'h.eyJyb2xlIjoic3RhZmYifQ.s', staff: { id: 's1', locationId: 'qa', role: 'counter-manager', caps: 'advance,take_orders', expiresMs: Date.now() + 864e5 } });

async function page(browser, base, { signed, qs = '' }) {
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: DPR, isMobile: true, hasTouch: true });
  if (signed) await ctx.addInitScript(s => { try { localStorage.setItem('dw_room_session', s); localStorage.setItem('dw_room_lang', 'sq'); } catch {} }, SESSION);
  else await ctx.addInitScript(() => { try { localStorage.removeItem('dw_room_session'); localStorage.setItem('dw_room_lang', 'sq'); } catch {} });
  const p = await ctx.newPage();
  const logs = [];
  p.on('pageerror', e => logs.push('ERR ' + e.message));
  p.on('console', m => { if (m.type() === 'error') logs.push('console.error ' + m.text().slice(0, 160)); });
  await p.goto(`${base}/room/canvas/${qs}`, { waitUntil: 'load' });
  const ok = await p.evaluate(() => window.__ready);
  if (!ok) throw new Error('board did not start: ' + (await p.evaluate(() => window.__err)) + ' ' + logs.join(' | '));
  if (signed) await p.waitForFunction(() => window.__canvas.stats()[8] > 0, null, { timeout: 15000 });
  return { p, ctx, logs };
}

const count = p => p.evaluate(() => ({ n: document.body.querySelectorAll('*').length, tags: [...document.body.querySelectorAll('*')].map(e => e.tagName).join(',') }));
const tap = (p, name) => p.evaluate(n => { const r = window.__canvas.tourRect(n); if (!r) return false;
  const c = document.querySelector('canvas'), x = r.x + r.w / 2, y = r.y + r.h / 2;
  for (const t of ['pointerdown', 'pointerup']) c.dispatchEvent(new PointerEvent(t, { clientX: x, clientY: y, bubbles: true, pointerId: 1, buttons: t === 'pointerdown' ? 1 : 0 }));
  return true; }, name);

async function dom(b, base) {
  const res = {};
  const a = await page(b, base, { signed: false, qs: '?noec=1' });
  res.signedOutRest = await count(a.p);
  res.tappedEmail = await tap(a.p, 'login.email');
  res.focused = await count(a.p);
  await tap(a.p, 'hud.sync');      // a tap that is not a field: the keyboard closes
  await a.p.waitForTimeout(50);
  res.blurred = await count(a.p);
  res.editContext = await a.p.evaluate(() => 'EditContext' in window);
  await a.ctx.close();
  if (res.editContext) {
    const e = await page(b, base, { signed: false });
    await tap(e.p, 'login.email');
    res.focusedEditContext = await count(e.p);
    await e.ctx.close();
  }
  const s = await page(b, base, { signed: true });
  res.signedInRest = await count(s.p);
  res.logs = [...a.logs, ...s.logs];
  await s.ctx.close();
  res.pass = res.signedOutRest.n === 1 && res.signedInRest.n === 1 && res.focused.n === 2 && /INPUT/.test(res.focused.tags)
    && res.blurred.n === 1 && (!res.editContext || res.focusedEditContext.n === 1);
  return res;
}

async function frame(b, base) {
  // First frame: the MEDIAN of three fresh contexts (each a cold page cache), measured from
  // navigation start -- one run on this shared box swung 244..364 ms with other lanes compiling.
  const navs = [];
  for (let i = 0; i < 2; i++) {
    const w = await page(b, base, { signed: true });
    navs.push(await w.p.evaluate(() => window.__canvas.t0 + window.__canvas.t_first_frame_ms));
    await w.ctx.close();
  }
  const s = await page(b, base, { signed: true });
  const r = await s.p.evaluate(async () => {
    const C = window.__canvas, c = document.querySelector('canvas');
    const first = { fromNav: C.t0 + C.t_first_frame_ms, fromLoader: C.t_first_frame_ms, instantiate: C.t_instantiate_ms };
    const raf = () => new Promise(r => requestAnimationFrame(r));
    const sz = () => ({ width: c.width, height: c.height, want: Math.round(innerWidth * devicePixelRatio), wantH: Math.round(innerHeight * devicePixelRatio), clientWidth: c.clientWidth, innerWidth });
    const off = z => z.width !== z.want || z.height !== z.wantH;
    // A doubling store stops the run at once: a doubled canvas re-rasterised 30 more times took
    // the prove 9 minutes on the box (S/canvas1-frame-prove.out, 2026-10-07).
    for (let i = 0; i < 5; i++) { dispatchEvent(new Event('resize')); await raf(); if (off(sz())) return { first, size: { ...sz(), afterResizes: i + 1 }, doubled: true }; }
    for (let i = 0; i < 30; i++) { C.ask(); await raf(); }
    const size = sz();
    C.frames.length = 0;
    for (let i = 0; i < 120; i++) { c.dispatchEvent(new WheelEvent('wheel', { deltaY: 40, bubbles: true, cancelable: true })); await raf(); }
    const f = C.frames.slice().sort((a, b) => a - b), q = x => f[Math.min(f.length - 1, Math.floor(x * f.length))];
    const x = c.getContext('2d').getImageData(0, 0, c.width, c.height).data;
    let drawn = 0; for (let i = 0; i < x.length; i += 4) if (x[i] !== x[0] || x[i + 1] !== x[1] || x[i + 2] !== x[2]) drawn++;
    return { first, size, wheel: { n: f.length, p50: q(0.5), p90: q(0.9), p99: q(0.99), max: f[f.length - 1] }, drawn, pixels: x.length / 4, stats: C.stats() };
  });
  await s.ctx.close();
  r.logs = s.logs;
  navs.push(r.first.fromNav); navs.sort((x, y) => x - y);
  r.first.navs = navs; r.first.median = navs[1];
  r.pass = !r.doubled && r.first.median <= 300 && r.wheel.n >= 100 && r.wheel.p99 <= 8 && r.size.width === r.size.want && r.size.height === r.size.wantH
    && r.size.clientWidth === r.size.innerWidth && r.drawn > r.pixels / 20 && r.stats[7] === 0;
  return r;
}

async function ctxloss(b, base) {
  const s = await page(b, base, { signed: true });
  const r = await s.p.evaluate(async () => {
    const C = window.__canvas, c = document.querySelector('canvas'), g = c.getContext('2d');
    const pix = () => { const d = g.getImageData(0, 0, c.width, c.height).data; let h = 0x811c9dc5; for (let i = 0; i < d.length; i += 7) { h ^= d[i]; h = Math.imul(h, 0x01000193) >>> 0; } return h; };
    C.redraw();
    const before = { hash: C.hash(), pix: pix() };
    c.width = c.width;            // what a loss leaves: a blank store, a reset context
    const wiped = { pix: pix(), transform: g.getTransform().a };
    c.dispatchEvent(new Event('contextrestored'));
    const after = { hash: C.hash(), pix: pix(), restored: C.restored || 0 };
    return { before, wiped, after };
  });
  await s.ctx.close();
  r.logs = s.logs;
  r.pass = r.wiped.pix !== r.before.pix && r.after.restored === 1 && r.after.hash === r.before.hash && r.after.pix === r.before.pix;
  return r;
}

const CHECKS = { dom, frame, ctxloss };
if (!CHECKS[check]) { console.log('usage: canvas.mjs <dom|frame|ctxloss> [--canvas-dir DIR] [--out FILE] [--dpr N]'); process.exit(2); }
if (!existsSync(join(CANVAS, 'board.wasm'))) { console.log(`canvas: no board.wasm in ${CANVAS} -- build it (crates/dowiz-canvas/build.sh); NOT a pass`); process.exit(2); }
const { chromium } = playwright();
const srv = await serve();
let browser, res;
try {
  // Fewer OS processes (no zygotes, GPU thread in the browser process, one renderer): on this box
  // Android's phantom-process killer SIGKILLs the whole gate when the box crosses 32 processes
  // (memory box-sigkill-phantom), and a default Chromium is ~8 of them.
  browser = await chromium.launch({ headless: true, args: ['--no-sandbox', '--no-zygote', '--in-process-gpu', '--renderer-process-limit=1'] });
  res = await CHECKS[check](browser, `http://127.0.0.1:${srv.address().port}`);
} catch (e) {
  console.log(`canvas ${check}: harness failed -- ${String(e.message || e).split('\n')[0]} -- NOT a pass`);
  await browser?.close(); srv.close(); process.exit(2);
}
res.chromium = browser.version(); res.dpr = DPR; res.check = check;
await browser.close(); srv.close();
const line = JSON.stringify(res);
if (OUT) await writeFile(OUT, line + '\n');
console.log(line);
console.log(`canvas ${check}: ${res.pass ? 'PASS' : 'FAIL'}`);
process.exit(res.pass ? 0 : 1);
