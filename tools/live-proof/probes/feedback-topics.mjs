// LIVE PROBE analytics.feedback_topics.v1 (W-VOICE, P16b) on qa-durres --
// main runs it after the deploy; a lane never runs it against production.
//
//   set -a; . /root/.dowiz_owner; set +a; node tools/live-proof/probes/feedback-topics.mjs
//
// WHAT IT PROVES, through the real storefront, order machine, feedback route
// and the venue object's kitchen numbers:
//   1. ONE TEST pickup order of QA Water is placed through the storefront
//      route and driven to PICKED_UP (the feedback route takes terminal orders).
//   2. The customer's own token leaves ONE note that says "cold" and holds a
//      phone number and a name: "QA <run>: the water was cold, call Arben
//      +355690000019" (POST /api/order/:id/feedback; the owner's token is
//      refused there by design, so the probe uses the order's access_token).
//   3. GET /api/owner/analytics/kitchen?from=today&to=today validates against
//      the contract; a row (this week, qa-water, cold) exists with count >= 1.
//   4. NO PERSON, NO ORDER: no row carries a key outside week/dish/name/topic/
//      count/examples (additionalProperties:false), the order id appears
//      nowhere in `topics`, and no example phrase holds the phone's digits,
//      any digit, or the name "Arben".
// Exit 0 = every step held; 1 = a step failed; 3 = NEEDS-KEY.
// BUDGET: ~8 requests, one order. It is a TEST order of the QA venue only.
import { LOC, RUN, api, own, owner, reporter, contract, read } from './stock-lib.mjs';

const C = contract('feedback-topics');
const { step, schema, verdict } = reporter('feedback-topics');
const PHONE = '+355690000019';

try {
  step('the QA owner signs in', !!(await owner()));
  const body = { items: [{ product_id: 'qa-water', modifier_ids: [], quantity: 1 }], contact: { name: `${RUN} topics`, phone: PHONE },
    fulfilment: { kind: 'pickup' }, payment: 'cash', locale: 'en' };
  const o = await api(`/api/public/locations/${LOC}/orders`, { method: 'POST', body });
  const id = o.body?.id, tok = o.body?.access_token;
  step('a TEST pickup order is placed', o.status === 200 && !!id && !!tok, `${o.status} ${o.text?.slice(0, 120)}`);
  for (const a of ['confirm', 'preparing', 'ready', 'collected']) {
    const x = await own(`/api/owner/orders/${id}/action`, { action: a, location_id: LOC });
    step(`the order goes ${a}`, x.status === 200, `${x.status} ${x.text?.slice(0, 100)}`);
  }
  const note = `QA ${RUN}: the water was cold, call Arben ${PHONE}`;
  const fb = await api(`/api/order/${id}/feedback`, { method: 'POST', body: { text: note }, token: tok });
  step('the customer leaves one note', fb.status === 200, `${fb.status} ${fb.text?.slice(0, 100)}`);

  const today = new Date().toISOString().slice(0, 10);
  const k = await read(own, `/api/owner/analytics/kitchen?location_id=${LOC}&from=${today}&to=${today}`);
  step('GET /api/owner/analytics/kitchen answers 200', k.status === 200, `${k.status} ${k.text?.slice(0, 120)}`);
  schema('the topics validate against analytics.feedback_topics.v1', k.body, C.response_schema);
  const rows = k.body?.topics || [];
  const cold = rows.find(r => r.dish === 'qa-water' && r.topic === 'cold');
  step('a (week, qa-water, cold) row counts the note', !!cold && cold.count >= 1, JSON.stringify(cold || rows.slice(0, 3)));
  const all = JSON.stringify(rows);
  step('the order id appears nowhere in the topics', !!id && !all.includes(id));
  const ex = rows.flatMap(r => r.examples || []);
  step('no example phrase holds a digit (the phone is gone)', ex.every(e => !/[0-9]/.test(e)) && !all.includes('690000019'), JSON.stringify(ex.slice(0, 4)));
  step('no example phrase holds the name', !all.includes('Arben'), JSON.stringify(ex.slice(0, 4)));
  step('the example still says what was wrong', !cold || cold.examples.some(e => /cold/i.test(e)), JSON.stringify(cold?.examples));
} catch (e) {
  step('the probe ran to the end', false, e.stack?.slice(0, 300));
} finally {
  verdict();
}
