// FEATURE glue-orders (LV-LOC-B, S2b; contract ../contracts/feature-glue-orders.json).
//
// A kitchen tablet's offline taps on a TEST order are sent as one tail and rebased by the venue's
// own deciders: every tap merged or held, the held one on the decision strip, the same tail twice
// adopted once, a tail with a foreign key refused whole. MAIN RUNS THIS AFTER THE DEPLOY:
//
//   node tools/live-proof/probes/feature-glue-orders.mjs
//
// The owner's token signs (staff_at takes an owner token for Cap::Advance). The TEST order is
// ended and the decision closed in a finally.
import fs from 'node:fs';
import path from 'node:path';
import { place, close } from './_order.mjs';

const HERE = path.dirname(new URL(import.meta.url).pathname);
const CONTRACT = JSON.parse(fs.readFileSync(path.join(HERE, '../contracts/feature-glue-orders.json'), 'utf8'));

export default async function (ctx) {
  const { lib, run, check, must, note } = ctx;
  const token = await lib.owner();
  const tab = `lp${String(run).replace(/[^A-Za-z0-9]/g, '').slice(-12)}`;
  const glue = tail => lib.api('/api/staff/orders/glue', { method: 'POST', token, body: { location_id: lib.LOC, ctx: tab, tail } });
  let o, did = null;
  try {
    o = await place(ctx);
    const at = Date.now();
    const op = (oid, extra) => JSON.stringify({ op: extra ? 'advance' : 'ack', oid: `${at}.${oid}`, order: o.id, ...(extra || {}), at: at + oid });
    const tail = [op(0), op(1, { from: 'PENDING', next: 'CONFIRMED' }), op(2, { from: 'CONFIRMED', next: 'PREPARING' }), op(3, { from: 'READY', next: 'PICKED_UP' })];
    for (const l of tail) check('op_schema', JSON.parse(l));
    check('request_schema', { location_id: lib.LOC, ctx: tab, tail });
    const r = await glue(tail);
    must(r.status === 200, `glue ${r.status} ${r.text.slice(0, 160)}`);
    check('response_schema', r.body);
    const b = r.body;
    must(b.sent === 4 && b.sent === b.merged + b.held.length, `the law: ${JSON.stringify(b)}`);
    must(b.merged === 3 && b.applied === 3 && b.held.length === 1 && b.held[0].pos === 3, `3 merged, the stale bump held: ${JSON.stringify(b)}`);
    const w = await lib.ownerOrder(o.id);
    must(w?.status === 'PREPARING' && w?.kitchen?.seen, `the owner's order: ${w?.status} seen=${JSON.stringify(w?.kitchen?.seen || null)}`);
    did = `o-${tab}-${at}.3`;
    const d = await lib.api(`/api/staff/stock/decisions?location_id=${lib.LOC}`, { token });
    must(d.status === 200 && (d.body?.open || []).some(x => x.did === did && x.kind === 'held' && x.order === o.id), `the held tap is not on the strip: ${d.text.slice(0, 200)}`);
    const again = await glue(tail);
    must(again.status === 200 && again.body.applied === 0 && again.body.notes === 0 && again.body.merged === 3, `the same tail twice: ${again.text.slice(0, 160)}`);
    const spy = [JSON.stringify({ ...JSON.parse(op(9, { from: 'PREPARING', next: 'READY' })), phone: '+355690000019' })];
    const bad = await glue(spy);
    must(bad.status === 422 && /phone/.test(bad.text), `a foreign key: ${bad.status} ${bad.text.slice(0, 120)}`);
    must((await lib.ownerOrder(o.id))?.status === 'PREPARING', 'the refused tail moved the order');
    note(`order ${o.id}: 4 taps -> 3 merged + 1 held (${b.held[0].why.slice(0, 60)}); replay applied 0; phone key 422`);
  } finally {
    if (did) await lib.api('/api/staff/stock/decisions/close', { method: 'POST', token, body: { location_id: lib.LOC, did } });
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
