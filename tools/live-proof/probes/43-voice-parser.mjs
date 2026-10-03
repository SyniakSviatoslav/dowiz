// Row 43: the owner's voice line moves orders, and NOTHING IS DONE ON THE
// STRENGTH OF A TRANSCRIPT (crates/dowiz-hub/src/voice.rs): "confirm order
// <digits>" names a TEST order by the tail of its id; the answer proposes
// exactly that order with a confirmation token, and the order is untouched.
import { place, reject } from './_order.mjs';
export default async function (ctx) {
  const { lib, check, must, note } = ctx;
  const o = await place(ctx);
  try {
    const tail = o.id.replace(/-/g, '').slice(-4);
    const r = await lib.own(`/api/voice?location_id=${lib.LOC}`, { transcript: `confirm order ${tail}`, confidence: 0.95, is_final: true, lang: 'en' });
    must(r.status === 200, `voice ${r.status} ${r.text.slice(0, 120)}`);
    check('response_schema', r.body);
    must(r.body.orderId === o.id && r.body.verb === 'confirm', `proposed ${r.body.verb} on ${r.body.orderId}, not confirm on ${o.id}`);
    const still = await lib.statusOf(o.id) ?? (await lib.ownerOrder(o.id))?.status;
    must(still === 'PENDING', `the order is ${still} after a mere proposal`);
    note(`"confirm order ${tail}" -> proposes confirm on ${o.id.slice(0, 8)} ("${r.body.readback}"), order still PENDING`);
  } finally { await reject(ctx, o.id); }
}
