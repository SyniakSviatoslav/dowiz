// LIVE PROBE sms (W-SMS) on qa-durres -- main runs it AFTER the deploy; a lane
// never runs it against production.
//
//   node tools/live-proof/probes/feature-sms.mjs      (FLOWS_HOST may name another qa- hub)
//
// A REAL PHONE, NOT A MOCK. The SMS leaves from the venue's own Android phone
// running SMSGate (capcom6/android-sms-gateway) in Cloud mode; the probe reads
// the GATEWAY'S OWN record of each message (GET /3rdparty/v1/messages/{id})
// to see the phone report it Sent/Delivered. Without the phone configured on
// the venue every delivery step says NEEDS-KEY -- never a pass.
//
// Needs in /root/.dowiz_owner:
//   QA_SMS_TEST_PHONE      a phone that receives the test texts (E.164, +355...)
//   QA_SMSGATE_USER/_PASSWORD  the same login the venue's SMS screen holds
//                          (only to READ the gateway's message state)
//
// 1. the owner card and the public box validate against the contract;
// 2. NEEDS-KEY unless the venue's gateway is on and complete;
// 3. the Test button answers ok; the gateway's record reaches Sent/Delivered;
// 4. a TEST order (LIVE-<run>) with the SMS box ticked to QA_SMS_TEST_PHONE is
//    confirmed: health.sent rises within WAIT_MS;
// 5. a STOP for that number, then the order is cancelled: the cancellation is
//    NOT sent (health.stopped rises, health.sent does not);
// 6. clean-up: every TEST order closed (sweep).
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter } from './stock-lib.mjs';
import { place, sweep } from './_order.mjs';

const C = contract('sms');
const { step, schema, verdict } = reporter('feature-sms');
const WAIT_MS = 180_000;
const GATE = 'https://api.sms-gate.app/3rdparty/v1/messages';
const run = `LIVE-${lib.RUN}`;
const q = `?location_id=${encodeURIComponent(lib.LOC)}`;
const must = (ok, msg) => { if (!ok) throw new Error(msg); };
const sleep = ms => new Promise(r => setTimeout(r, ms));

async function card(){
  const r = await lib.own(`/api/owner/sms${q}`);
  must(r.status === 200, `GET /api/owner/sms ${r.status} ${r.text}`);
  return r.body;
}
/// Wait until `pred(card)` holds; answers the last card.
async function until(pred){
  const end = Date.now() + WAIT_MS;
  let c = await card();
  while (!pred(c) && Date.now() < end) { await sleep(10_000); c = await card(); }
  return c;
}
/// The gateway's own state of one message, until Sent/Delivered/Failed or the wait ends.
async function gateState(id){
  const { QA_SMSGATE_USER: u, QA_SMSGATE_PASSWORD: p } = lib.creds;
  const auth = 'Basic ' + Buffer.from(`${u}:${p}`).toString('base64');
  const end = Date.now() + WAIT_MS;
  let last = null;
  while (Date.now() < end) {
    const r = await fetch(`${GATE}/${encodeURIComponent(id)}`, { headers: { authorization: auth } });
    last = r.status === 200 ? (await r.json()).state : `HTTP ${r.status}`;
    if (['Sent', 'Delivered', 'Failed'].includes(last)) return last;
    await sleep(10_000);
  }
  return last;
}

let order = null;
try {
  const c0 = await card();
  schema('the owner card validates against the contract', c0, C.card_schema);
  const b = await lib.api(`/api/public/locations/${encodeURIComponent(lib.LOC)}/sms`);
  step('GET the public SMS box answers 200', b.status === 200, `${b.status} ${b.text}`);
  schema('the box validates against the contract', b.body, C.box_response_schema);
  const ready = c0.config?.on && !c0.config?.missing;
  step('the box is shown exactly when the venue can text', b.body?.on === !!ready, `box.on=${b.body?.on} config.on=${c0.config?.on} missing=${c0.config?.missing}`);

  const phone = lib.creds.QA_SMS_TEST_PHONE;
  if (!ready || !phone) {
    step('the venue\'s own SMS gateway', 'NEEDS-KEY', 'install SMSGate (github.com/capcom6/android-sms-gateway) on an Android phone with a SIM, switch Cloud server on, tap Online, copy its username/password into Settings -> SMS on the QA venue, switch texts on; put QA_SMS_TEST_PHONE (+ QA_SMSGATE_USER/_PASSWORD) in /root/.dowiz_owner');
  } else {
    const t = await lib.own(`/api/owner/sms/test${q}`, { phone });
    schema('the Test answer validates', t.body, C.test_response_schema);
    step('the Test button: the gateway accepted the text', t.status === 200 && t.body?.ok === true, t.text);
    const canRead = lib.creds.QA_SMSGATE_USER && lib.creds.QA_SMSGATE_PASSWORD;
    if (!canRead) step('the phone reports the test text Sent/Delivered', 'NEEDS-KEY', 'QA_SMSGATE_USER/_PASSWORD not in /root/.dowiz_owner');
    else if (t.body?.message_id) {
      const st = await gateState(t.body.message_id);
      step('the phone reports the test text Sent/Delivered (gateway record)', st === 'Sent' || st === 'Delivered', `state=${st}`);
    }

    const wording = (b.body.wordings || []).find(w => w.lang === 'en')?.id;
    must(wording, 'no English SMS sentence in the box');
    await sweep(lib);
    const before = (await card()).health;
    order = await place({ lib, run, must }, { sms: { order_status: true, wording } }, phone);
    step('a TEST order with the SMS box ticked is placed', !!order.id, order.id);
    const cf = await lib.own(`/api/owner/orders/${encodeURIComponent(order.id)}/action`, { action: 'confirm', location_id: lib.LOC });
    step('the owner confirms', cf.status === 200, `${cf.status} ${cf.text.slice(0, 120)}`);
    const c1 = await until(c => (c.health?.sent || 0) > (before.sent || 0));
    step('the CONFIRMED text left through the venue gateway (health.sent rose)', (c1.health?.sent || 0) > (before.sent || 0), JSON.stringify(c1.health));
    schema('the card after a send validates', c1, C.card_schema);

    const sp = await lib.own(`/api/owner/sms/stop${q}`, { phone });
    step('the owner files the customer\'s STOP', sp.status === 200 && sp.body?.ok === true, sp.text);
    const x = await lib.own(`/api/owner/orders/${encodeURIComponent(order.id)}/action`, { action: 'cancel', location_id: lib.LOC });
    step('the TEST order is cancelled', x.status === 200, `${x.status} ${x.text.slice(0, 120)}`);
    const c2 = await until(c => (c.health?.stopped || 0) > (c1.health?.stopped || 0));
    step('the cancellation was NOT texted after STOP', (c2.health?.stopped || 0) > (c1.health?.stopped || 0) && c2.health?.sent === c1.health?.sent, JSON.stringify(c2.health));
    order = null;
  }
} catch (e) {
  step('the probe ran to the end', false, e.stack || String(e));
} finally {
  try {
    if (order) await lib.own(`/api/owner/orders/${encodeURIComponent(order.id)}/action`, { action: 'cancel', location_id: lib.LOC });
    await sweep(lib);
  } catch (e) { step('clean-up', false, String(e)); }
}
verdict();
