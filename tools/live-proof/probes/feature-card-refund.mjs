// LIVE PROBE card-refund (W-REFUND) on qa-durres -- main runs it AFTER the
// deploy, with Stripe TEST keys on the Worker; a lane never runs it.
//
//   node tools/live-proof/probes/feature-card-refund.mjs    (FLOWS_HOST may name another qa- hub)
//
// STRIPE TEST MODE ONLY. The probe holds QA_STRIPE_TEST_SECRET_KEY (the same
// test account as the Worker's STRIPE_SECRET_KEY) to do what a customer's
// browser does -- confirm the PaymentIntent with the test card pm_card_visa --
// and to read Stripe's own records back. A key that is not `sk_test_` is
// REFUSED before a single request. No key: every step that needs it is
// NEEDS-KEY (rc 3), never a pass.
//
// 1. a card order (the QA water) on the qa hub: the answer carries the PI;
//    Stripe holds it with amount = total x 100 for ALL (Stripe's units);
// 2. the PI is confirmed with pm_card_visa; the signed webhook marks the
//    order paid with amount_received = the order's total, in lek;
// 3. the owner confirms it, then refunds HALF to the card: the order is
//    REFUNDING with one `queued` attempt carrying the key;
// 4. the venue's drain sends it: the attempt gets Stripe's refund id; that
//    refund, read at Stripe, validates against the contract, carries the key
//    and the venue in its metadata and the half in Stripe's units;
// 5. Stripe's refund webhook marks it succeeded: still REFUNDING;
// 6. the REST: succeeded, and the order is COMPENSATED_REFUND by Stripe's word;
// 7. read back at Stripe: the charge's amount_refunded is the whole amount;
//    the guest's own order read (its token) shows both attempts succeeded.
// Every wait is bounded (WAIT_MS) and a timeout is a FAIL.
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter, validate } from './stock-lib.mjs';
import { place } from './_order.mjs';

const C = contract('card-refund');
const { step, schema, verdict } = reporter('feature-card-refund');
const WAIT_MS = 120_000;
const SK = process.env.QA_STRIPE_TEST_SECRET_KEY || lib.creds.QA_STRIPE_TEST_SECRET_KEY || '';
const STRIPE = 'https://api.stripe.com/v1';
const must = (ok, why) => { if (!ok) { step(why, false); verdict(); } };

async function stripe(path, form){
  const r = await fetch(STRIPE + path, {
    method: form ? 'POST' : 'GET',
    headers: { authorization: `Bearer ${SK}`, 'stripe-version': '2024-06-20', ...(form ? { 'content-type': 'application/x-www-form-urlencoded' } : {}) },
    body: form ? new URLSearchParams(form).toString() : undefined,
  });
  return { status: r.status, body: await r.json().catch(() => null) };
}

async function until(what, read, ok){
  const end = Date.now() + WAIT_MS;
  let last;
  while (Date.now() < end) {
    last = await read();
    if (ok(last)) return last;
    await lib.sleep(4000);
  }
  step(`${what} within ${WAIT_MS / 1000} s`, false, JSON.stringify(last)?.slice(0, 300));
  verdict();
}

const order = id => lib.ownerOrder(id);
const attempts = o => o?.refund?.card?.attempts || [];

if (!SK) {
  step('QA_STRIPE_TEST_SECRET_KEY present', 'NEEDS-KEY', 'add it to /root/.dowiz_owner (the test account of the Worker key)');
  verdict();
}
must(SK.startsWith('sk_test_'), 'the probe key is a Stripe TEST key (sk_test_) -- refused otherwise');

// 1. the card order
const run = lib.RUN;
const p = await place({ lib, run, must }, { payment: 'card', fulfilment: { kind: 'pickup' } });
if (p.body.payment_error) {
  step(`the hub created a PaymentIntent (${p.body.payment_error})`, 'NEEDS-KEY', 'STRIPE_SECRET_KEY / STRIPE_PUBLISHABLE_KEY on the Worker');
  verdict();
}
const pi = p.body.payment_intent;
step('the order answer carries the PaymentIntent', typeof pi === 'string' && pi.startsWith('pi_'), pi);
const total = (await order(p.id))?.total;
const held = await stripe(`/payment_intents/${pi}`);
step('Stripe holds the intent for the order total in Stripe units (ALL x 100)', held.body?.amount === total * 100, `${held.body?.amount} vs ${total}`);

// 2. paid with the test card; the webhook records it in lek
const conf = await stripe(`/payment_intents/${pi}/confirm`, { payment_method: 'pm_card_visa', return_url: 'https://example.com/return' });
step('pm_card_visa confirms the intent', conf.body?.status === 'succeeded', `${conf.status} ${conf.body?.status || conf.body?.error?.message}`);
const paid = await until('the payment webhook marks the order paid', () => order(p.id), o => o?.payment_status === 'paid');
step('amount_received is the total in lek', paid.amount_received === total, `${paid.amount_received} vs ${total}`);
const c = await lib.own(`/api/owner/orders/${encodeURIComponent(p.id)}/action`, { action: 'confirm', location_id: lib.LOC });
must(c.status === 200, `confirming: ${c.status}`);

// 3. half to the card
const owner = await lib.owner();
const half = Math.floor(total / 2);
const r1 = await lib.api(`/api/staff/orders/${encodeURIComponent(p.id)}/refund`, { method: 'POST', token: owner,
  headers: { 'idempotency-key': `${run}-card-1` }, body: { location_id: lib.LOC, reason: 'customer_request', note: 'live-proof card refund', card_amount: half } });
step('the refund route takes the half', r1.status === 200, `${r1.status} ${r1.text.slice(0, 160)}`);
const o1 = await order(p.id);
schema('refund.card validates (card_record_schema)', o1?.refund?.card, C.card_record_schema);
step('REFUNDING with one queued attempt of the half', o1?.status === 'REFUNDING' && attempts(o1)[0]?.amount === half, JSON.stringify(attempts(o1)));

// 4. the drain sends it
const sent1 = await until('the drain sends the first card refund', () => order(p.id), o => !!attempts(o)[0]?.id);
const re1 = await stripe(`/refunds/${attempts(sent1)[0].id}`);
schema('the Stripe refund validates (stripe_refund_response_schema)', re1.body, C.stripe_refund_response_schema);
step('Stripe refunded the half in Stripe units', re1.body?.amount === half * 100, `${re1.body?.amount}`);
step('the refund carries the attempt key and the venue', re1.body?.metadata?.key === attempts(sent1)[0].key && re1.body?.metadata?.venue === lib.LOC, JSON.stringify(re1.body?.metadata));

// 5. Stripe's webhook: succeeded, still REFUNDING
const ok1 = await until('the refund webhook marks the half succeeded', () => order(p.id), o => attempts(o)[0]?.status === 'succeeded');
step('half back is not the card part: still REFUNDING', ok1.status === 'REFUNDING', ok1.status);
const over = await lib.api(`/api/staff/orders/${encodeURIComponent(p.id)}/refund`, { method: 'POST', token: owner,
  headers: { 'idempotency-key': `${run}-card-over` }, body: { location_id: lib.LOC, card_amount: total - half + 1 } });
step('one lek more than is left is refused (400)', over.status === 400, `${over.status} ${over.text.slice(0, 120)}`);

// 6. the rest
const r2 = await lib.api(`/api/staff/orders/${encodeURIComponent(p.id)}/refund`, { method: 'POST', token: owner,
  headers: { 'idempotency-key': `${run}-card-2` }, body: { location_id: lib.LOC, card_amount: total - half } });
step('the refund route takes the rest', r2.status === 200, `${r2.status} ${r2.text.slice(0, 160)}`);
const done = await until('Stripe\'s word ends the order', () => order(p.id), o => o?.status === 'COMPENSATED_REFUND');
step('ended by Stripe, both attempts succeeded', done.refund?.returned?.by === 'stripe' && attempts(done).every(a => a.status === 'succeeded'), JSON.stringify(attempts(done)));
step('two different keys', attempts(done)[0].key !== attempts(done)[1].key);

// 7. read back
const ch = await stripe(`/charges?payment_intent=${pi}`);
step('the charge is refunded in full at Stripe', ch.body?.data?.[0]?.amount_refunded === total * 100, `${ch.body?.data?.[0]?.amount_refunded}`);
const guest = await lib.api(`/api/order/${encodeURIComponent(p.id)}`, { token: p.token });
const back = attempts(guest.body).filter(a => a.status === 'succeeded').reduce((s, a) => s + a.amount, 0);
step('the guest reads the whole amount refunded', guest.status === 200 && back === total, `${guest.status} ${back}`);
const bad = validate({ location_id: lib.LOC, card_amount: half }, C.request_schema);
step('the request body this probe sent validates', bad.length === 0, bad.join('; '));

verdict();
