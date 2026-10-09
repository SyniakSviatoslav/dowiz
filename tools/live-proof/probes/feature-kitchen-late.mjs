// FEATURE kitchen-late (W-CV2P row 1; contract ../contracts/feature-kitchen-late.json).
//
// The console's kitchen board (/admin/, tab Kitchen) colours a ticket by the venue's own
// notify.order.late_min -- amber at half of it, red at it -- the same rule as the canvas board
// (room/canvas/feed.js ages()). Unset, 0 or nonsense keeps 10/20.
// MAIN RUNS THIS AFTER THE DEPLOY (written by the lane, never run against production by it):
//
//   node tools/live-proof/probes/feature-kitchen-late.mjs
//
// 1. the owner reads the settings (GET /api/owner/settings) and remembers late_min;
// 2. sets it to 8 through the console's own write (POST /api/owner/settings {key, value});
// 3. places a TEST order, opens /admin/ signed in as the owner, taps the Kitchen tab, and moves
//    the PAGE's clock 5 minutes on (Playwright clock): the TEST ticket is kds-age-warn (5 >= 8/2),
//    where 10/20 would leave it ok; and the page itself read /api/owner/settings;
// 4. finally: late_min goes back to what it was, the order is ended, the browser closed.
import fs from 'node:fs';
import path from 'node:path';
import { place, close } from './_order.mjs';

const HERE = path.dirname(new URL(import.meta.url).pathname);
const CONTRACT = JSON.parse(fs.readFileSync(path.join(HERE, '../contracts/feature-kitchen-late.json'), 'utf8'));
const KEY = 'notify.order.late_min';

export default async function (ctx) {
  const { lib, check, must, note } = ctx;
  const { launch, until, shot } = await import('../../../e2e/flows/page.mjs');
  const token = await lib.owner();
  const q = `?location_id=${encodeURIComponent(lib.LOC)}`;
  const read = await lib.api(`/api/owner/settings${q}`, { token });
  must(read.status === 200, `GET /api/owner/settings ${read.status}`);
  check('response_schema', read.body);
  const was = read.body?.values?.[KEY] ?? '';
  let o = null, browser = null;
  try {
    const body = { key: KEY, value: '8' };
    check('request_schema', body);
    const set = await lib.api(`/api/owner/settings${q}`, { method: 'POST', token, body });
    must(set.status === 200, `POST /api/owner/settings ${set.status} ${String(set.text || '').slice(0, 140)}`);
    const back = await lib.api(`/api/owner/settings${q}`, { token });
    must(back.body?.values?.[KEY] === '8', `the setting reads back ${JSON.stringify(back.body?.values?.[KEY])}`);
    o = await place(ctx);
    browser = await launch();
    const context = await browser.newContext({ viewport: { width: 1280, height: 900 }, userAgent: lib.UA });
    await context.addInitScript(([t, loc]) => { try { sessionStorage.setItem('dw_at', t); localStorage.setItem('dw_loc', loc); } catch {} }, [token, lib.LOC]);
    const page = await context.newPage();
    await page.clock.install({ time: Date.now() });
    const uncaught = [];
    page.on('pageerror', e => uncaught.push(String(e.message).split('\n')[0].slice(0, 160)));
    const res = await page.goto(`${lib.HOST}/admin/`, { waitUntil: 'load' });
    must(res && res.status() === 200, `GET /admin/ answered ${res && res.status()}`);
    must(await until(page, () => !!document.querySelector('#nav [data-tab="kitchen"]'), null, 30000), `no Kitchen tab: ${uncaught.join(' | ')}`);
    const asked = page.waitForRequest(r => r.method() === 'GET' && r.url().includes('/api/owner/settings'), { timeout: 20000 }).catch(() => null);
    await page.click('#nav [data-tab="kitchen"]');
    const req = await asked;
    must(req, 'the kitchen board never read /api/owner/settings');
    const sel = `article[data-o="${o.id}"]`;
    must(await until(page, s => !!document.querySelector(s), sel, 30000), `the TEST ticket ${o.id} is not on the board`);
    await page.clock.fastForward('05:10');
    // The board re-draws on its 30 s tick; the settings answer re-draws it too.
    const age = await (async () => {
      for (let k = 0; k < 20; k++) {
        const c = await page.evaluate(s => (document.querySelector(s)?.className.match(/kds-age-(\w+)/) || [])[1] || '', sel);
        if (c === 'warn' || c === 'late') return c;
        await page.clock.fastForward('00:31'); await page.waitForTimeout(200);
      }
      return await page.evaluate(s => (document.querySelector(s)?.className.match(/kds-age-(\w+)/) || [])[1] || '', sel);
    })();
    check('page_schema', { settingsRead: !!req, ageClass: age, uncaught: uncaught.length });
    await shot(page, 'kitchen-late');
    note(`/admin/ kitchen: late_min 8 -> TEST ticket ${o.id} is kds-age-${age} after ~5 min (10/20 would be ok)`);
  } finally {
    await browser?.close().catch(() => {});
    const r = await lib.api(`/api/owner/settings${q}`, { method: 'POST', token, body: { key: KEY, value: String(was) } });
    note(`late_min restored to ${JSON.stringify(was)}: ${r.status}`);
    if (o) await close(ctx, o.id);
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
