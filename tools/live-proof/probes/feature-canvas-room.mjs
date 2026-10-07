// FEATURE canvas-room (W-CANVAS1, Wave CV row CV1; contract ../contracts/feature-canvas-room.json).
//
// The room/kitchen board on ONE canvas, on the live QA hub: a TEST order appears on it, the
// body holds exactly one element, and a real mouse click on that ticket's Accept button moves
// the order to CONFIRMED through the console's own route. The order is ended whatever happens.
// MAIN RUNS THIS AFTER THE DEPLOY (written by the lane, never run against production by it):
//
//   node tools/live-proof/probes/feature-canvas-room.mjs
//
// The owner's token signs the room session (staff_at takes an owner for Cap::Advance and
// Cap::TakeOrders), as feature-offline-sale does. One Chromium, closed in a finally.
import fs from 'node:fs';
import path from 'node:path';
import { place, close } from './_order.mjs';

const HERE = path.dirname(new URL(import.meta.url).pathname);
const CONTRACT = JSON.parse(fs.readFileSync(path.join(HERE, '../contracts/feature-canvas-room.json'), 'utf8'));

export default async function (ctx) {
  const { lib, check, must, note } = ctx;
  const { launch, until, go, shot } = await import('../../../e2e/flows/page.mjs');
  const token = await lib.owner();
  const o = await place(ctx);
  const browser = await launch();
  try {
    const context = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, userAgent: lib.UA });
    const session = { jwt: token, staff: { id: 'owner', locationId: lib.LOC, role: 'owner', caps: 'advance,take_orders', expiresMs: Date.now() + 3600_000 } };
    await context.addInitScript(s => { try { localStorage.setItem('dw_room_session', s); } catch {} }, JSON.stringify(session));
    const page = await context.newPage();
    const uncaught = [];
    page.on('pageerror', e => uncaught.push(String(e.message).split('\n')[0].slice(0, 160)));
    const wasm = page.waitForResponse(r => r.url().endsWith('/room/canvas/board.wasm'), { timeout: 30000 }).catch(() => null);
    const res = await page.goto(`${lib.HOST}/room/canvas/`, { waitUntil: 'load' });
    must(res && res.status() === 200, `GET /room/canvas/ answered ${res && res.status()}`);
    const w = await wasm;
    must(await until(page, () => window.__ready, null, 30000), `the board never started: ${await page.evaluate(() => window.__err || '')} ${uncaught.join(' | ')}`);
    must(await until(page, id => !!window.__canvas && window.__canvas.stats()[8] > 0 && window.__canvas.ex && true, o.id, 30000), 'the board read no tickets');
    // The TEST order is PENDING: tab New. Scroll until its Accept button is on screen.
    let r = null;
    for (let k = 0; k < 40 && !r; k++) {
      r = await page.evaluate(id => { window.__canvas.redraw(); return window.__canvas.bumpRect(id); }, o.id);
      if (!r) { await page.mouse.move(195, 600); await page.mouse.wheel(0, 300); await page.waitForTimeout(100); }
    }
    must(r, `the TEST order ${o.id} has no Accept button on the board`);
    const facts = await page.evaluate(() => ({ bodyElements: document.body.querySelectorAll('*').length, tickets: window.__canvas.stats()[8], tooSmall: window.__canvas.stats()[7], transient: window.__canvas.transient }));
    check('page_schema', { ...facts, csp: res.headers()['content-security-policy'] || '', wasmType: (w && w.headers()['content-type']) || '' });
    await shot(page, 'canvas-room-before');
    const sent = page.waitForRequest(q => q.method() === 'POST' && q.url().includes(`/api/owner/orders/${encodeURIComponent(o.id)}/action`), { timeout: 20000 }).catch(() => null);
    await page.mouse.click(r.x + r.w / 2, r.y + r.h / 2);
    const req = await sent;
    must(req, 'a click on Accept sent no action request');
    check('request_schema', JSON.parse(req.postData() || '{}'));
    const moved = await (async () => {
      for (let k = 0; k < 20; k++) { if ((await lib.ownerOrder(o.id))?.status === 'CONFIRMED') return true; await lib.sleep(500); }
      return false;
    })();
    must(moved, `the order did not move to CONFIRMED (now ${(await lib.ownerOrder(o.id))?.status})`);
    const board = await lib.api(`/api/staff/kitchen?location_id=${lib.LOC}`, { token });
    must(board.status === 200, `GET /api/staff/kitchen ${board.status}`);
    check('response_schema', board.body);
    const mine = (board.body.orders || []).find(x => x.id === o.id);
    check('readback_schema', mine || {});
    must(await until(page, () => document.body.querySelectorAll('*').length === 1, null, 5000), 'after the tap the body holds more than the canvas');
    await shot(page, 'canvas-room-after');
    must(!uncaught.length, `uncaught on the page: ${uncaught.join(' | ')}`);
    note(`board ${lib.HOST}/room/canvas/: body=1, ${facts.tickets} tickets; Accept on ${o.id} -> CONFIRMED (owner order and /api/staff/kitchen)`);
  } finally {
    await browser.close().catch(() => {});
    await close(ctx, o.id);
  }
}

// RUN DIRECTLY: one JSON line in run.mjs's shape, exit 0 only on LIVE-PROVEN.
if (import.meta.url === `file://${process.argv[1]}`) {
  process.env.FLOWS_HOST ||= process.env.LIVE_HOST || 'https://qa-durres.dowiz.org';
  const lib = await import('../../../e2e/flows/lib.mjs');
  const { validate } = await import('../lib/schema.mjs');
  class Fail extends Error {}
  const line = { row: CONTRACT.id, name: CONTRACT.name, status: 'FAILED', evidence: [], at: new Date().toISOString(), contract: CONTRACT.contract_version };
  const v = await lib.api('/api/version');
  line.build = v.body?.commit || `unknown (${v.status})`;
  let checked = 0; const bad = [];
  const ctx = { lib, run: lib.RUN, Fail, note: s => line.evidence.push(s),
    check(name, value) { checked++; const e = validate(value, CONTRACT[name]); if (e.length) bad.push(`${name}: ${e.slice(0, 4).join('; ')}`); return value; },
    must(ok, why) { if (!ok) throw new Fail(why); } };
  try {
    await (await import(import.meta.url)).default(ctx);
    line.status = bad.length || !checked ? 'FAILED' : 'LIVE-PROVEN';
  } catch (e) { line.evidence.push(`${e instanceof Fail ? '' : 'CRASH '}${e.message}`); }
  if (bad.length) line.evidence.push(`SCHEMA ${bad.join(' | ')}`);
  console.log(JSON.stringify(line));
  process.exit(line.status === 'LIVE-PROVEN' ? 0 : 1);
}
