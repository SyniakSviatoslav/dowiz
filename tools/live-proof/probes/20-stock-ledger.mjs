// Row 20: an order moves its recipe's ingredients, exactly. QA Salmon roll =
// 80 g QA Salmon + 120 g QA Sushi rice (e2e/walk/qa-setup.mjs). Placing holds
// the recipe; at READY it is held or taken off the shelf, never lost; the
// refund that ends the order puts every gram back where it was.
import { place, close } from './_order.mjs';
const RECIPE = { 'qa-salmon': 80, 'qa-rice': 120 };
export default async function (ctx) {
  const { lib, check, must, note } = ctx;
  const shelf = async () => {
    const r = await lib.own(`/api/owner/stock?location_id=${lib.LOC}`);
    must(r.status === 200, `stock ${r.status}`);
    check('response_schema', r.body);
    return Object.fromEntries(Object.keys(RECIPE).map(id => {
      const s = (r.body.supplies || []).find(x => x.id === id);
      must(s, `the QA shelf has no ${id} (run e2e/walk/qa-setup.mjs)`);
      return [id, { onHand: s.onHand, reserved: s.reserved }];
    }));
  };
  const s0 = await shelf();
  const roll = { items: [{ product_id: 'qa-salmon-roll', modifier_ids: [], quantity: 1 }] };
  const o = await place(ctx, roll);
  let s1, s3;
  try {
    s1 = await shelf();
    for (const id in RECIPE) must(s1[id].reserved - s0[id].reserved === RECIPE[id], `placing: ${id} reserved ${s0[id].reserved}->${s1[id].reserved}, wanted +${RECIPE[id]}`);
    for (const a of ['confirm', 'preparing', 'ready']) {
      const x = await lib.own(`/api/owner/orders/${o.id}/action`, { action: a, location_id: lib.LOC });
      must(x.status === 200, `${a} ${x.status} ${x.text.slice(0, 100)}`);
    }
    const s2 = await shelf();
    for (const id in RECIPE) {
      const taken = s0[id].onHand - s2[id].onHand, held = s2[id].reserved - s0[id].reserved;
      must(taken === RECIPE[id] || held === RECIPE[id], `READY: ${id} taken ${taken}, held ${held}; the ${RECIPE[id]} g went nowhere`);
    }
  } finally { await close(ctx, o.id); }
  s3 = await shelf();
  const moved = Object.keys(RECIPE).map(id => `${id} onHand ${s0[id].onHand}->${s3[id].onHand} reserved ${s0[id].reserved}->${s3[id].reserved}`).join('; ');
  must(Object.keys(RECIPE).every(id => s3[id].reserved === s0[id].reserved && s0[id].onHand - s3[id].onHand === RECIPE[id]),
    `after the refund: ${moved}; wanted the reservation released and onHand down by exactly the recipe`);
  note(`placing holds 80 g salmon + 120 g rice; READY takes/holds them exactly; after the refund: ${moved}`);
}
