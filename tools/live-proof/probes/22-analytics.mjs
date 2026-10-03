// Row 22: the owner's numbers move by exactly one order. Read the analytics
// and the kitchen analytics; take a TEST order (one QA Water, its price read
// from the public menu) to READY; both reads grow by one order and by that
// price; the refund that ends it takes the revenue back out.
import { place, close } from './_order.mjs';
export default async function (ctx) {
  const { lib, check, must, note } = ctx;
  const price = lib.dishes(await lib.menu()).find(d => d.id === 'qa-water')?.price;
  must(Number.isInteger(price), 'QA Water has no integer price on the public menu');
  const read = async () => {
    const a = await lib.own(`/api/owner/analytics?location_id=${lib.LOC}`);
    const k = await lib.own(`/api/owner/analytics/kitchen?location_id=${lib.LOC}`);
    must(a.status === 200 && k.status === 200, `analytics ${a.status}, kitchen ${k.status}`);
    check('response_schema', a.body);
    return { orders: a.body.orders, revenue: a.body.revenue, kOrders: k.body.totals?.orders, kRevenue: k.body.totals?.revenue };
  };
  const r0 = await read();
  const o = await place(ctx);
  let r1, r2;
  try {
    for (const a of ['confirm', 'preparing', 'ready']) {
      const x = await lib.own(`/api/owner/orders/${o.id}/action`, { action: a, location_id: lib.LOC });
      must(x.status === 200, `${a} ${x.status} ${x.text.slice(0, 100)}`);
    }
    r1 = await read();
  } finally { await close(ctx, o.id); }
  r2 = await read();
  const d = (k) => r1[k] - r0[k];
  must(d('orders') === 1 && d('revenue') === price, `analytics moved by ${d('orders')} orders / ${d('revenue')} lek, wanted 1 / ${price}`);
  must(d('kOrders') === 1 && d('kRevenue') === price, `kitchen analytics moved by ${d('kOrders')} / ${d('kRevenue')}, wanted 1 / ${price}`);
  must(r2.revenue === r0.revenue, `after the refund revenue is ${r2.revenue}, was ${r0.revenue}`);
  note(`READY: +1 order +${price} lek on both reads (${r0.orders}->${r1.orders}); refund: revenue back to ${r2.revenue}`);
}
