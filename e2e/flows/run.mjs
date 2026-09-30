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

const FLOWS = [['F1', 'customer', f1], ['F2', 'owner', f2], ['F3', 'courier', f3], ['F4', 'cleanup', f4]];
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
    const why = e instanceof Fail || e instanceof NoLaunch ? e.message : `threw ${String(e.stack || e).split('\n').slice(0, 2).join(' ')}`;
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
