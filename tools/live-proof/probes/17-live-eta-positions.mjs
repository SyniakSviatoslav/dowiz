// Row 17: while the courier carries an order, the position the phone posts
// reaches the customer: GET /api/order/:id with the customer's own token
// carries the courier's point.
import { courierRun } from './_courier.mjs';
export default async function (ctx) {
  const { lib, check, must, note } = ctx;
  const r = await courierRun(ctx, async ({ o, C }) => {
    const p = await C('/api/courier/position', { lat: 41.3189, lon: 19.4561, accuracy_m: 12, order_id: o.id });
    must(p.status === 200, `position ${p.status} ${p.text.slice(0, 100)}`);
    const g = await lib.api(`/api/order/${encodeURIComponent(o.id)}`, { token: o.token });
    must(g.status === 200, `customer read ${g.status}`);
    check('response_schema', g.body);
    const s = JSON.stringify(g.body);
    must(/41\.318|41318/.test(s) && /19\.456|19456/.test(s), `the customer's read does not carry the courier's point: ${s.slice(0, 200)}`);
    return 'seen';
  });
  note(`posted 41.3189,19.4561 during IN_DELIVERY; the customer's token read it back; then refused -> ${r.st}`);
}
