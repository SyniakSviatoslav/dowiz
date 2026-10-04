// LIVE PROBE bag-qr (W-QR) on qa-durres -- main runs it AFTER the deploy; a lane
// never runs it against production.
//
//   node tools/live-proof/probes/feature-bag-qr.mjs      (FLOWS_HOST may name another qa- hub)
//
// The real Worker, the venue's real object, the real order log. No external
// service is involved (the QR is drawn inside the Worker), so nothing here is
// NEEDS-KEY except the owner login in /root/.dowiz_owner.
//
// 1. the owner card validates against the contract; its link is the venue's
//    own host with ?src=bag&c=<campaign>, and the QR route answers an SVG;
// 2. a 1-lek fixed offer is set; the public offer validates and sets no cookie;
// 3. a TEST order with src=bag from a fresh phone is GRANTED the offer, records
//    the referral, and keeps every line at its menu price;
// 4. a second TEST order from the SAME phone is placed at full price with
//    welcome_refused=used;
// 5. the card's bag orders rose by two;
// 6. clean-up: both orders rejected, the owner's original offer restored.
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter, validate } from './stock-lib.mjs';
import { place, reject, sweep } from './_order.mjs';

const C = contract('bag-qr');
const { step, schema, verdict } = reporter('feature-bag-qr');
const run = `LIVE-${lib.RUN}`;
const CAMPAIGN = 'probe';
const q = (c = '') => `?location_id=${encodeURIComponent(lib.LOC)}${c ? '&c=' + c : ''}`;
const must = (ok, msg) => { if (!ok) throw new Error(msg); };
/// A phone no earlier run used: the welcome is once per phone.
const PHONE = '+35569' + String(Date.now()).slice(-7);

async function card(){
  const r = await lib.own(`/api/owner/bag${q(CAMPAIGN)}`);
  must(r.status === 200, `GET /api/owner/bag ${r.status} ${r.text}`);
  return r.body;
}
/// The public answer's variant, picked by its own words (the validator has no oneOf).
const variant = b => C.public_response_schema.oneOf.find(s => s.properties.on.const === b?.on && (!b?.on || s.properties.kind?.const === b.kind));

let original = null;
const placed = [];
try {
  const c0 = await card();
  original = { offer: c0.offer || { kind: 'off' }, commission_pct: c0.commission_pct == null ? '' : String(c0.commission_pct) };
  schema('the owner card validates against the contract', c0, C.card_schema);
  if (c0.offer) schema('its offer validates', c0.offer, C.card_schema.$defs.offer.oneOf.find(s => s.properties.kind.const === c0.offer.kind) || {});
  const u = new URL(c0.url);
  step('the printed link is the venue\'s own host, root path, src=bag&c=<campaign> only',
    u.host === new URL(lib.HOST).host && u.pathname === '/' && [...u.searchParams.keys()].join() === 'src,c' && u.searchParams.get('c') === CAMPAIGN, c0.url);
  const svg = await fetch(`${lib.HOST}/api/owner/bag/qr.svg${q(CAMPAIGN)}`, { headers: { authorization: 'Bearer ' + await lib.owner() } });
  const svgText = await svg.text();
  step('the QR route answers an SVG, the same the card draws', svg.status === 200 && (svg.headers.get('content-type') || '').startsWith('image/svg+xml') && svgText === c0.svg, `${svg.status} ${svg.headers.get('content-type')}`);

  const set = await lib.own(`/api/owner/bag${q()}`, { offer: { kind: 'fixed', value: 1, min: 0 }, commission_pct: '' });
  must(validate({ offer: { kind: 'fixed', value: 1, min: 0 }, commission_pct: '' }, C.edit_request_schema).length === 0, 'the probe\'s own request breaks the contract');
  step('the owner sets a 1-lek welcome', set.status === 200 && set.body?.ok === true, set.text);
  const pub = await fetch(`${lib.HOST}/api/public/locations/${encodeURIComponent(lib.LOC)}/welcome`);
  const pb = await pub.json();
  schema('the public offer validates', pb, variant(pb) || { not: {} });
  step('the public offer is the one just set', pb.on === true && pb.kind === 'fixed' && pb.value === 1, JSON.stringify(pb));
  step('reading it sets no cookie', pub.headers.get('set-cookie') === null, String(pub.headers.get('set-cookie')));

  await sweep(lib);
  const before = (await card()).stats.orders;
  const src = { src: { src: 'bag', c: CAMPAIGN } };
  must(validate(src.src, C.order_field_schema).length === 0, 'the probe\'s own src breaks the contract');
  const a = await place({ lib, run, must }, src, PHONE); placed.push(a.id);
  schema('the first order\'s bag fields validate', { referral: a.body.referral, welcome: a.body.welcome }, C.order_record_schema);
  step('the first order from this phone is GRANTED the welcome', a.body.welcome?.discount === 1 && a.body.welcome?.c === CAMPAIGN, JSON.stringify(a.body.welcome));
  step('it records where it came from', a.body.referral?.src === 'bag' && a.body.referral?.c === CAMPAIGN, JSON.stringify(a.body.referral));
  const menu = await lib.menu();
  const price = Object.fromEntries(lib.dishes(menu).map(d => [d.id, d.price]));
  step('every line keeps its menu price', (a.body.items || []).every(l => l.unit_price === price[l.product_id]), JSON.stringify(a.body.items));
  const b = await place({ lib, run, must }, src, PHONE); placed.push(b.id);
  step('a SECOND order from the same phone is placed at full price, refused politely',
    !b.body.welcome && b.body.welcome_refused === 'used' && b.body.total === a.body.total + 1, `${b.body.welcome_refused} total ${b.body.total} vs ${a.body.total}`);
  const c1 = await card();
  schema('the card after two bag orders validates', c1, C.card_schema);
  step('the card counts both bag orders', c1.stats.orders === before + 2, `${before} -> ${c1.stats.orders}`);
  step('scans are reported as not counted (null), never invented', c1.stats.scans === null, String(c1.stats.scans));
} catch (e) {
  step('the probe ran to the end', false, e.stack || String(e));
} finally {
  try {
    for (const id of placed) await reject({ lib, must }, id);
    if (original) await lib.own(`/api/owner/bag${q()}`, original);
    await sweep(lib);
  } catch (e) { step('clean-up', false, String(e)); }
}
verdict();
