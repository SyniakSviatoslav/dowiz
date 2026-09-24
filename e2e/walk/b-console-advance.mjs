// B: owner console -- advance the TEST round confirm -> preparing -> ready.
import { HOST, OUT, creds, say, watch, csp, browser, api, staffToken, st, finish } from './_lib.mjs';
const ID = process.env.ORDER || st().roomOrder;
const STEPS = (process.env.STEPS || 'confirm:CONFIRMED,preparing:PREPARING,ready:READY').split(',').map(s => s.split(':'));
const b = await browser();
try {
  const ctx = await b.newContext({ viewport: { width: 420, height: 900 }, serviceWorkers: 'block' });
  const o = await ctx.newPage(); watch(o, 'console');
  await o.goto(`${HOST}/admin/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await o.waitForSelector('#e', { timeout: 40000 });
  await o.fill('#e', creds.OWNER_EMAIL); await o.fill('#p', creds.OWNER_PASSWORD); await o.click('#go');
  await o.waitForSelector('#nav:not([hidden])', { timeout: 60000 }).catch(() => {});
  await o.waitForTimeout(3000);
  say(!(await o.$('#e')), 'console signs in');
  const seen = await o.waitForSelector(`[data-o="${ID}"]`, { timeout: 30000 }).then(() => true).catch(() => false);
  say(seen, 'dine-in TEST round is on the console', ID);
  await o.screenshot({ path: `${OUT}/b1-console.png` });
  const tok = await staffToken();
  const status = async () => { const r = await api(`/api/staff/room?location_id=sushi-durres`, { token: tok }); for (const s of r.body.sittings || []) for (const x of s.rounds || []) if (x.id === ID) return x.status; return '(not in room)'; };
  for (const [act, want] of STEPS) {
    const btn = await o.$(`[data-act="${act}"][data-o="${ID}"]`);
    if (!btn) { say(false, `console button ${act}`, 'not on screen'); break; }
    await btn.click(); await o.waitForTimeout(3500);
    const s = await status();
    say(s === want, `console ${act} -> ${want}`, `status=${s}`);
  }
  const row = await o.$eval(`[data-o="${ID}"]`, e => e.closest('li,article,.order,.row,div')?.innerText.replace(/\s+/g, ' ').slice(0, 200)).catch(() => '(row gone)');
  say(null, 'console row now', row);
  await o.screenshot({ path: `${OUT}/b2-console-after.png` });
  await csp(o, 'console');
} finally { await b.close(); finish(); }
