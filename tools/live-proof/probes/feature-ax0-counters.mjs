// LIVE PROBE ax0-counters (AX0-COUNTERS, plan §A0 + card W-AX0 (e)) on qa-durres -- main runs it
// AFTER the deploy; a lane never runs it against production. WRITTEN, NOT RUN by the lane.
//
//   node tools/live-proof/probes/feature-ax0-counters.mjs      (FLOWS_HOST may name another qa- hub)
//
// 1. GET /api/owner/health answers `counters`, valid against the contract;
// 2. two catch-ups (/api/owner/orders?since=0 and ?since=<far ahead>) move since_total by 2 and since_none
//    by 2 (neither is inside any window) -- unless the object WOKE between the reads, which
//    the window must then say (cause=wake, a later wokeAtMs): that is a pass with the reason printed;
//    a smaller count WITHOUT a new wake is a FAIL;
// 3. ?counters=flush answers flushed=true, and the next read is a `flush` window (or a later wake);
// 4. one catalogue write (a QA dish created, then deleted) moves cat_writes, cat_decoded_bytes,
//    journal_bytes, writes and proj_rows; cat_decode_us is printed with the clock, never judged
//    (on the platform the clock only advances on I/O, so a decode reads ~0 by construction).
// Every step that cannot run is a FAIL, never a skip.
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter } from './stock-lib.mjs';

const C = contract('ax0-counters');
const { step, schema, verdict } = reporter('feature-ax0-counters');
const run = `LIVE-${lib.RUN}`;
const must = (ok, why) => { if (!ok) throw new Error(why); };
const health = async (q = '') => {
  const r = await lib.own(`/api/owner/health?location_id=${lib.LOC}${q}`);
  must(r.status === 200 && r.body?.counters, `health answered ${r.status} ${r.text?.slice(0, 160)}`);
  must(!r.body.counters.error, `the object's counters are unreadable: ${r.body.counters.error}`);
  return r.body.counters;
};
// The same wake, or a later one the window names.
const woke = (a, b) => b.window.wokeAtMs > a.window.wokeAtMs && b.window.cause === 'wake';

let dish = null;
try {
  // 1. the shape
  const h0 = await lib.own(`/api/owner/health?location_id=${lib.LOC}`);
  step('GET /api/owner/health answers 200 with counters', h0.status === 200 && !!h0.body?.counters, `${h0.status}`);
  schema('health.counters matches the contract', h0.body, C.routes.health.response_schema);
  const a = h0.body.counters;
  console.log(`     window ${JSON.stringify(a.window)} wake ${JSON.stringify(a.wake)} clock ${a.clock}`);

  // 2. two catch-ups
  // Both reach `changes_since` (no status filter); since=0 is older than any window and a since far
  // AHEAD of the log is not "up to date" either -- both are "ask for the list".
  for (const since of [0, 9_000_000_000]) {
    const r = await lib.own(`/api/owner/orders?location_id=${lib.LOC}&since=${since}`);
    must(r.status === 200, `orders?since=${since} answered ${r.status}`);
  }
  const b = await health();
  if (woke(a, b)) {
    step('the object woke between the reads, and the window says so (cause=wake)', true, `wokeAtMs ${a.window.wokeAtMs} -> ${b.window.wokeAtMs}`);
  } else {
    step('two catch-ups move since_total by 2', b.since_total - a.since_total === 2, `${a.since_total} -> ${b.since_total}`);
    step('both are polls the window cannot answer (since_none +2)', b.since_none - a.since_none === 2, `${a.since_none} -> ${b.since_none}`);
  }

  // 3. the flush
  const f = await health('&counters=flush');
  step('?counters=flush answers flushed=true', f.window.flushed === true, JSON.stringify(f.window));
  const g = await health();
  step('the next window is a flush window (or a later wake)', g.window.cause === 'flush' || woke(f, g), JSON.stringify(g.window));

  // 4. a catalogue write's cost
  const m = await lib.menu();
  const cat = (m.categories || [])[0]?.id;
  must(cat, 'the qa menu has no category');
  const mk = await lib.own('/api/owner/products', { location_id: lib.LOC, category_id: cat, name: `${run} ax0`, price: 1000, available: false });
  step('a QA dish is created (one catalogue write)', mk.status === 200 && mk.body?.id, `${mk.status} ${mk.text}`);
  dish = mk.body?.id;
  const k = await health();
  if (woke(g, k)) {
    step('the object woke after the write; its cost is in a lost window (said, not hidden)', true, JSON.stringify(k.window));
  } else {
    step('cat_writes moved', k.cat_writes > g.cat_writes, `${g.cat_writes} -> ${k.cat_writes}`);
    step('the decodes are counted in bytes', k.cat_decoded_bytes > g.cat_decoded_bytes, `${g.cat_decoded_bytes} -> ${k.cat_decoded_bytes}`);
    step('the journal load is counted in bytes', k.journal_bytes > g.journal_bytes, `${g.journal_bytes} -> ${k.journal_bytes}`);
    step('the write stored at least one chunk', k.writes > g.writes && k.proj_rows > g.proj_rows, `writes ${g.writes} -> ${k.writes}, rows ${g.proj_rows} -> ${k.proj_rows}`);
    console.log(`     per this write: decode ${k.cat_decode_us - g.cat_decode_us} µs, journal ${k.journal_us - g.journal_us} µs (clock ${k.clock})`);
  }
} catch (e) {
  step('the probe ran to the end', false, e.message);
} finally {
  if (dish) {
    const d = await lib.own(`/api/owner/products/${encodeURIComponent(dish)}/delete`, { location_id: lib.LOC });
    step('the QA dish is deleted', d.status === 200, `${d.status} ${d.text}`);
  }
  verdict();
}
