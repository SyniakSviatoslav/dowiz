// Row 47: the console's copy catches up by delta. Read generation g; a new
// order; ?since=g answers it as a change with a generation above g
// (owner.rs orders, lib/replica.js).
import { place, reject } from './_order.mjs';
export default async function (ctx) {
  const { lib, check, must, note } = ctx;
  const full = await lib.own(`/api/owner/orders?location_id=${lib.LOC}`);
  must(full.status === 200, `owner orders ${full.status}`);
  const g0 = await lib.own(`/api/owner/orders?location_id=${lib.LOC}&since=0`);
  const g = g0.body?.generation;
  must(Number.isInteger(g), `no integer generation in the since=0 answer: ${g0.text.slice(0, 100)}`);
  const o = await place(ctx);
  try {
    const d = await lib.own(`/api/owner/orders?location_id=${lib.LOC}&since=${g}`);
    must(d.status === 200, `since=${g}: ${d.status}`);
    must(!d.body?.full, `since=${g} fell back to the full list (${d.text.slice(0, 80)}): the delta path was not exercised`);
    check('response_schema', d.body);
    const hit = JSON.stringify(d.body.changes).includes(o.id);
    must(hit && d.body.generation > g, `since=${g}: generation ${d.body.generation}, order ${o.id} ${hit ? 'present' : 'ABSENT'} in ${d.body.changes.length} changes`);
    note(`g=${g}; after one order since=${g} -> generation ${d.body.generation}, ${d.body.changes.length} change(s) naming ${o.id}`);
  } finally { await reject(ctx, o.id); }
}
