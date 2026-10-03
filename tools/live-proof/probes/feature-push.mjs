// LIVE PROBE push (W-PUSH) on qa-durres -- main runs it AFTER the deploy and
// after `wrangler secret put VAPID_PRIVATE_KEY`; a lane never runs it against
// production.
//
//   node tools/live-proof/probes/feature-push.mjs      (FLOWS_HOST may name another qa- hub)
//
// A REAL RECEIVER, NOT A MOCK. The probe is its own browser: it speaks Mozilla
// autopush's WebSocket protocol (wss://push.services.mozilla.com, the service
// Firefox uses), registers channels restricted to the hub's VAPID key, and
// gets real `https://updates.push.services.mozilla.com/wpush/v2/...` endpoints.
// It holds the P-256 key and the auth secret of each "device", so what the hub
// sends is read back DECRYPTED (RFC 8291) here -- the same bytes a phone opens.
//
// 0. the decryptor opens the RFC 8291 §5 example (or nothing below means anything);
// 1. GET /api/push/key validates and is the key every channel is registered with;
// 2. the owner subscribes device A; /api/push/state says on;
// 3. a TEST order (LIVE-<run>) is placed: device A receives "New order" #<id>;
// 4. the customer (order token) subscribes device B; the owner confirms:
//    device B receives "Confirmed" with url /?order=<id>;
// 5. (when the QA hub courier exists) the courier subscribes device C, opens a
//    shift, is assigned the order: device C receives the assignment;
// 6. clean-up: the order is cancelled (device B hears it and is forgotten),
//    the shift closed, device A unsubscribed (state says off).
// Every push that does not arrive within WAIT_MS is a FAIL, never a skip.
import crypto from 'node:crypto';
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter, validate } from './stock-lib.mjs';
import { place, sweep } from './_order.mjs';

const C = contract('push');
const { step, schema, verdict } = reporter('feature-push');
const WAIT_MS = 120_000;
const AUTOPUSH = 'wss://push.services.mozilla.com/';
const b64u = b => Buffer.from(b).toString('base64url');
const unb64u = s => Buffer.from(s, 'base64url');

// ── RFC 8291 receiver ───────────────────────────────────────────────────────
function device(){
  const ecdh = crypto.createECDH('prime256v1');
  ecdh.generateKeys();
  return { ecdh, auth: crypto.randomBytes(16) };
}
function open(body, ecdh, auth){
  const salt = body.subarray(0, 16);
  const rs = body.readUInt32BE(16);
  const idlen = body[20];
  const asPublic = body.subarray(21, 21 + idlen);
  if (rs < 18 || idlen !== 65) throw new Error(`bad header rs=${rs} idlen=${idlen}`);
  const secret = ecdh.computeSecret(asPublic);
  const uaPublic = ecdh.getPublicKey();
  const keyInfo = Buffer.concat([Buffer.from('WebPush: info\0'), uaPublic, asPublic]);
  const ikm = Buffer.from(crypto.hkdfSync('sha256', secret, auth, keyInfo, 32));
  const cek = Buffer.from(crypto.hkdfSync('sha256', ikm, salt, Buffer.from('Content-Encoding: aes128gcm\0'), 16));
  const nonce = Buffer.from(crypto.hkdfSync('sha256', ikm, salt, Buffer.from('Content-Encoding: nonce\0'), 12));
  const ct = body.subarray(21 + idlen);
  const d = crypto.createDecipheriv('aes-128-gcm', cek, nonce);
  d.setAuthTag(ct.subarray(ct.length - 16));
  let plain = Buffer.concat([d.update(ct.subarray(0, ct.length - 16)), d.final()]);
  let end = plain.length; while (end > 0 && plain[end - 1] === 0) end--;
  if (plain[end - 1] !== 2) throw new Error('no 0x02 last-record delimiter');
  return plain.subarray(0, end - 1).toString('utf8');
}

// ── autopush: one WebSocket, a channel per device ───────────────────────────
function autopush(){
  const ws = new WebSocket(AUTOPUSH, 'push-notification');
  const inbox = new Map(); // channelID -> [{text}]
  const waiters = [];
  let replies = [];
  const send = m => ws.send(JSON.stringify(m));
  const next = type => new Promise((res, rej) => {
    const t = setTimeout(() => rej(new Error(`autopush: no ${type} in 20 s`)), 20_000);
    replies.push(m => { if (m.messageType !== type) return false; clearTimeout(t); res(m); return true; });
  });
  ws.onmessage = ev => {
    const m = JSON.parse(String(ev.data));
    if (m.messageType === 'notification') {
      send({ messageType: 'ack', updates: [{ channelID: m.channelID, version: m.version, code: 100 }] });
      const list = inbox.get(m.channelID) || [];
      list.push(m);
      inbox.set(m.channelID, list);
      for (const w of waiters.splice(0)) w();
      return;
    }
    if (m.messageType === 'ping') return send({ messageType: 'ping' });
    replies = replies.filter(f => !f(m));
  };
  const ready = new Promise((res, rej) => { ws.onopen = res; ws.onerror = e => rej(new Error(`autopush: ${e.message || 'socket error'}`)); });
  return {
    async hello(){ await ready; const h = next('hello'); send({ messageType: 'hello', use_webpush: true, uaid: '' }); const m = await h; if (m.status !== 200) throw new Error(`hello ${m.status}`); },
    async register(key){
      const channelID = crypto.randomUUID();
      const r = next('register');
      send({ messageType: 'register', channelID, key });
      const m = await r;
      if (m.status !== 200 || !m.pushEndpoint) throw new Error(`register ${m.status}`);
      return { channelID, endpoint: m.pushEndpoint };
    },
    /// The next message on `channelID` whose decrypted JSON satisfies `want`, or null at the deadline.
    async await(channelID, dev, want, ms = WAIT_MS){
      const end = Date.now() + ms;
      for (;;) {
        for (const m of inbox.get(channelID) || []) {
          if (m.seen) continue;
          m.seen = true;
          try {
            const msg = JSON.parse(open(unb64u(m.data || ''), dev.ecdh, dev.auth));
            if (want(msg)) return msg;
          } catch (e) { console.log(`  !! undecryptable message on ${channelID.slice(0, 8)}: ${e.message}`); }
        }
        const left = end - Date.now();
        if (left <= 0) return null;
        await new Promise(r => { const t = setTimeout(r, Math.min(left, 5000)); waiters.push(() => { clearTimeout(t); r(); }); });
      }
    },
    close(){ try { ws.close(); } catch {} },
  };
}

const subBody = (endpoint, dev, lang) => ({ endpoint, expirationTime: null, keys: { p256dh: b64u(dev.ecdh.getPublicKey()), auth: b64u(dev.auth) }, lang });
const must = (ok, msg) => { if (!ok) throw new Error(msg); };
const run = `LIVE-${lib.RUN}`;
let ap = null, order = null, ownerDev = null, courier = null;

try {
  // ── 0. the decryptor, on the RFC's own bytes ──
  const ua = crypto.createECDH('prime256v1');
  ua.setPrivateKey(unb64u('q1dXpw3UpT5VOmu_cf_v6ih07Aems3njxI-JWgLcM94'));
  const rfc = unb64u('DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPTpK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN');
  step('the decryptor opens RFC 8291 §5', open(rfc, ua, unb64u('BTBZMqHH6r4Tts7J_aSIgg')) === 'When I grow up, I want to be a watermelon');

  // ── 1. the key ──
  const k = await lib.api('/api/push/key');
  step('GET /api/push/key answers 200', k.status === 200, `${k.status} ${k.text}`);
  schema('the key validates against the contract', k.body, C.key_response_schema);
  const key = k.body?.key;
  must(typeof key === 'string' && unb64u(key).length === 65, 'no usable VAPID key');

  // ── a real push service ──
  ap = autopush();
  await ap.hello();
  step('autopush hello', true, AUTOPUSH);
  await sweep(lib);

  // ── 2. the owner's device ──
  ownerDev = { ...device(), ...(await ap.register(key)) };
  step('autopush gives a real endpoint', /^https:\/\/updates\.push\.services\.mozilla\.com\//.test(ownerDev.endpoint), ownerDev.endpoint.slice(0, 60));
  const tok = await lib.owner();
  const s1 = await lib.api('/api/push/subscribe', { method: 'POST', token: tok, body: subBody(ownerDev.endpoint, ownerDev, 'en') });
  step('the owner subscribes', s1.status === 200, `${s1.status} ${s1.text}`);
  schema('subscribe validates against the contract', s1.body, C.response_schema);
  const st = await lib.api('/api/push/state', { method: 'POST', token: tok, body: { endpoint: ownerDev.endpoint } });
  schema('state validates', st.body, C.state_response_schema);
  step('state says on', st.body?.on === true, st.text);

  // ── 3. a new order rings the owner's device ──
  order = await place({ lib, run, must });
  const short = order.id.slice(0, 8);
  const a = await ap.await(ownerDev.channelID, ownerDev, m => m.title === `Order #${short}`);
  step('the owner device RECEIVED and DECRYPTED "New order"', a?.body === 'New order', JSON.stringify(a));
  if (a) schema('the delivered message validates', a, C.message_schema);

  // ── 4. the customer's device hears its order move ──
  const custDev = { ...device(), ...(await ap.register(key)) };
  const s2 = await lib.api('/api/push/subscribe', { method: 'POST', token: order.token, body: subBody(custDev.endpoint, custDev, 'uk') });
  step('the customer subscribes with the order token', s2.status === 200, `${s2.status} ${s2.text}`);
  const c1 = await lib.own(`/api/owner/orders/${encodeURIComponent(order.id)}/action`, { action: 'confirm', location_id: lib.LOC });
  step('the owner confirms', c1.status === 200, `${c1.status} ${c1.text.slice(0, 120)}`);
  const b = await ap.await(custDev.channelID, custDev, m => m.title === `Замовлення #${short}`);
  step('the customer device RECEIVED and DECRYPTED "Підтверджено" (uk)', b?.body === 'Підтверджено' && b?.url === `/?order=${order.id}`, JSON.stringify(b));

  // ── 5. the courier (when the QA hub has one) ──
  const { phone, password } = lib.courierCreds();
  if (!phone || !password) step('courier assignment', 'NEEDS-KEY', 'no QA_HUB_COURIER_* in /root/.dowiz_owner');
  else {
    const lg = await lib.api('/api/courier/auth/login', { method: 'POST', body: { phone, password } });
    courier = { jwt: lg.body?.jwt, id: lg.body?.courier?.id };
    must(lg.status === 200 && courier.jwt && courier.id, `courier login ${lg.status}`);
    const cDev = { ...device(), ...(await ap.register(key)) };
    const s3 = await lib.api('/api/push/subscribe', { method: 'POST', token: courier.jwt, body: subBody(cDev.endpoint, cDev, 'sq') });
    step('the courier subscribes', s3.status === 200, `${s3.status} ${s3.text}`);
    const sh = await lib.api('/api/courier/shift', { method: 'POST', token: courier.jwt, body: { open: true } });
    step('the courier opens a shift', sh.status === 200, `${sh.status} ${sh.text.slice(0, 100)}`);
    const as = await lib.own(`/api/owner/orders/${encodeURIComponent(order.id)}/assign`, { location_id: lib.LOC, courier_id: courier.id });
    step('the owner assigns the order', as.status === 200, `${as.status} ${as.text.slice(0, 120)}`);
    const c = await ap.await(cDev.channelID, cDev, m => m.title === `Porosia #${short}`);
    step('the courier device RECEIVED and DECRYPTED the assignment (sq)', c?.body === 'Ju është caktuar një porosi', JSON.stringify(c));
    await lib.api('/api/push/unsubscribe', { method: 'POST', token: courier.jwt, body: { endpoint: cDev.endpoint } });
  }

  // ── 6. the end: the customer hears it, then is forgotten ──
  const x = await lib.own(`/api/owner/orders/${encodeURIComponent(order.id)}/action`, { action: 'cancel', location_id: lib.LOC });
  step('the TEST order is cancelled', x.status === 200, `${x.status} ${x.text.slice(0, 120)}`);
  const e = await ap.await(custDev.channelID, custDev, m => m.body === 'Скасовано');
  step('the customer device RECEIVED the last word', !!e, JSON.stringify(e));
  const gone = await lib.api('/api/push/state', { method: 'POST', token: order.token, body: { endpoint: custDev.endpoint } });
  step('the ended order\'s device is forgotten', gone.body?.on === false, gone.text);
  order = null;
} catch (e) {
  step('the probe ran to the end', false, e.stack || String(e));
} finally {
  try {
    if (order) await lib.own(`/api/owner/orders/${encodeURIComponent(order.id)}/action`, { action: 'cancel', location_id: lib.LOC });
    if (courier?.jwt) await lib.api('/api/courier/shift', { method: 'POST', token: courier.jwt, body: { open: false } });
    if (ownerDev) {
      const tok = await lib.owner();
      const u = await lib.api('/api/push/unsubscribe', { method: 'POST', token: tok, body: { endpoint: ownerDev.endpoint } });
      step('the owner device is unsubscribed', u.status === 200 && u.body?.on === false, u.text);
    }
    await sweep(lib);
  } catch (e) { step('clean-up', false, String(e)); }
  ap?.close();
}
verdict();
