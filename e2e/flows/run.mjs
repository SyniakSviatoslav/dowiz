// THE FLOWS GATE: customer, owner, courier, cleanup -- one real browser at a
// time against qa-durres, every step asserted on the page AND through the API.
//
//   bash tools/gates/flows.sh                 (through the slot; what deploy calls)
//   node e2e/flows/run.mjs                    (direct; still ONE chromium at a time)
//   FLOWS_ONLY=F1,F4 node e2e/flows/run.mjs   (a subset -- F4 always runs)
//   FLOWS_LOCAL=<workers/api/public copy>     (serve the UI's .js/.css from a local tree)
//
// A FLOW IS OK OR IT FAILED; there is no third answer. A flow that did not run
// because an earlier one failed is a FAIL naming why, never a pass (memory
// gates-that-count-skips-as-passes). F4 runs whatever happened before it,
// because an order left open is litter in the venue's kitchen.
//
// Last line: `flows: F1 ok F2 ok F3 ok F4 ok (NN s)`, or `flows: FAIL F<n> ::
// <reason>` for the first failure with its screenshot. Exit 0 / 1, and 3 when
// Chromium could not launch at all.
import { HOST, RUN, OUT, S, save, Fail } from './lib.mjs';
import { NoLaunch } from './page.mjs';
import { f1 } from './f1-customer.mjs';
import { f2 } from './f2-owner.mjs';
import { f3 } from './f3-courier.mjs';
import { f4 } from './f4-cleanup.mjs';
import { f5, f6, f7, f8, f9 } from './f5-cycle.mjs';
import { f10 } from './f10-failures.mjs';
import fs from 'node:fs';

// F11 (stock + analytics) is owned by another lane: e2e/flows/f11-stock.mjs, if it exists.
// Its contract: `export default async function f11(ctx)`, where ctx is
//   { HOST, LOC, RUN, OUT, S,           // S.cycle.guests: [{key,how,name,phone,lang,ids,qty,id,token}], S.cycle.final
//     own(path, body?, method?),        // owner API call (owner token for LOC) -> {status, body, text}
//     api(path, {method, body, token}), // unauthenticated / token API call -> {status, body, text}
//     ownerOrders(), ownerOrder(id), menu() }
// It runs after F10 and before F4 (cleanup), and is skipped with an INFO line when the file is absent.
const F11_FILE = new URL('./f11-stock.mjs', import.meta.url);
const f11Present = fs.existsSync(F11_FILE);
const f11 = f11Present ? (await import(F11_FILE.href)).default : null;
const ctx11 = async () => ({ HOST, LOC, RUN, OUT, S, own, api, ownerOrders, ownerOrder, menu });
import { own, api, ownerOrders, ownerOrder, menu, LOC } from './lib.mjs';

const FLOWS = [['F1', 'customer', f1], ['F2', 'owner', f2], ['F3', 'courier', f3], ['F5', 'guests', f5], ['F6', 'kitchen', f6], ['F7', 'courier-cycle', f7], ['F8', 'in-venue', f8], ['F9', 'owner screens', f9], ['F10', 'failures', f10]];
// f11-stock.mjs as written exports `f11(log)`; the contract says `f11(ctx, log)`. The ctx handed over
// is ALSO callable as `log`, so either signature gets what it asks for.
// It also ANSWERS its verdict (0 ok, 1 a check failed, 2 could not run) instead of throwing: a non-zero
// answer is a FAIL here, never an ok over FAIL lines (memory gates-that-count-skips-as-passes).
if (f11) FLOWS.push(['F11', 'stock+analytics', async log => {
  const c = Object.assign((...a) => log(...a), await ctx11());
  const rc = await f11(c, log);
  if (typeof rc === 'number' && rc !== 0) throw new Fail(`f11-stock.mjs answered ${rc}${rc === 2 ? ' (could not run)' : ' (its FAIL lines above)'}`);
}]);
else console.log('F11: e2e/flows/f11-stock.mjs not present -- stock and analytics not run (INFO, not ok)');
// F12 (S2b, LV-LOC-B): the kitchen tablet's offline order taps, synced and rebased. Same verdict contract as F11.
const { default: f12 } = await import('./f12-offline-kitchen.mjs');
FLOWS.push(['F12', 'offline kitchen', async log => {
  const rc = await f12(await ctx11(), log);
  if (typeof rc === 'number' && rc !== 0) throw new Fail(`f12-offline-kitchen.mjs answered ${rc}${rc === 2 ? ' (could not run)' : ' (its FAIL lines above)'}`);
}]);
FLOWS.push(['F4', 'cleanup', f4]);
const only = (process.env.FLOWS_ONLY || '').split(',').filter(Boolean);
const t0 = Date.now();
const res = [];
let noLaunch = null;
console.log(`flows run ${RUN} against ${HOST}${process.env.FLOWS_LOCAL ? ` (UI from ${process.env.FLOWS_LOCAL})` : ''}; output ${OUT}`);
save();

for (const [id, name, fn] of FLOWS) {
  if (only.length && !only.includes(id) && id !== 'F4') { res.push({ id, ok: false, why: `not run (FLOWS_ONLY=${only.join(',')})` }); continue; }
  if (noLaunch && id !== 'F4') { res.push({ id, ok: false, why: `not run: ${noLaunch}` }); continue; }
  const t = Date.now();
  console.log(`\n── ${id} ${name} ──`);
  try {
    await fn(line => console.log(`  ok   ${id}: ${line}`));
    res.push({ id, ok: true });
    console.log(`  ${id} ok (${Math.round((Date.now() - t) / 1000)} s)`);
  } catch (e) {
    const why = e instanceof Fail || e instanceof NoLaunch ? e.message : `threw ${String(e.message || e).split('\n').map(l => l.trim()).filter(l => l && l !== 'Call log:').slice(0, 2).join(' :: ')} @ ${String(e.stack || '').split('\n').find(l => /e2e\/flows/.test(l))?.trim() || ''}`;
    if (e instanceof NoLaunch) noLaunch = e.message;
    res.push({ id, ok: false, why, shot: e.shot });
    console.log(`  FAIL ${id}: ${why}${e.shot ? `\n       screenshot: ${e.shot}` : ''}`);
  }
}
S.result = res; save();

const secs = Math.round((Date.now() - t0) / 1000);
const first = res.find(r => !r.ok);
console.log(`\nstate: ${OUT}/state.json`);
if (first) {
  console.log(`flows: ${res.map(r => `${r.id} ${r.ok ? 'ok' : 'FAIL'}`).join(' ')} (${secs} s)`);
  console.log(`flows: FAIL ${first.id} :: ${first.why}${first.shot ? ` [screenshot ${first.shot}]` : ''}`);
  process.exit(noLaunch ? 3 : 1);
}
console.log(`flows: ${res.map(r => `${r.id} ok`).join(' ')} (${secs} s)`);
process.exit(0);
