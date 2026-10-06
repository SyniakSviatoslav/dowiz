// LIVE PROBE taste2 (W-TASTE2, 2026-10-06) on qa-durres -- main runs it AFTER the deploy; a lane never
// runs it against production.
//
//   node tools/live-proof/probes/feature-taste2.mjs      (FLOWS_HOST may name another qa- hub)
//
// 1. AI default: /api/owner/integrations .ai matches its schema; the QA venue's switch is reported as
//    stored (on unless its owner turned it off); Suggest on qa-water answers 200 with a lexicon draft
//    whether or not a model answered (fail soft), and writes nothing.
// 2. Server "For you": a TEST order with a phone -> GET /api/order/:id/taste/for-you validates; never a
//    number or a price in it; no link -> 401; after the one-tap withdraw -> state off, no items; an
//    order with no phone -> 404.
// 3. The deployed builder carries the open-meteo.com credit beside the weather filter.
// 4. S7: /api/owner/health .sheaf validates (acts:false); Suggest's .recipe validates and, while the
//    venue has no recipes, says no-recipes with no draft; the deployed health sheet loads the row.
// Every TEST order is ended. A step that cannot run is a FAIL, never a skip.
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter } from './stock-lib.mjs';
import { place, close, sweep } from './_order.mjs';

const C = contract('taste2');
const { step, schema, verdict } = reporter('feature-taste2');
const run = `LIVE-${lib.RUN}`;
const must = (ok, why) => { if (!ok) throw new Error(why); };
const tail = Number.parseInt(lib.RUN.replace(/\D/g, '').slice(-2) || '0', 10) % 30;
const phone = `+3556900008${String(tail).padStart(2, '0').slice(-2)}`;
const DISH = 'qa-water';
const asset = async p => { const r = await fetch(lib.HOST + p, { headers: { 'user-agent': lib.UA } }); return { status: r.status, text: await r.text() }; };
const orders = [];

try {
  await sweep(lib);

  // 1. AI on by default, fail soft
  const ig = await lib.own(`/api/owner/integrations?location_id=${lib.LOC}`);
  schema('integrations .ai matches its schema', ig.body?.ai, C.ai_status_schema);
  const st = await lib.own(`/api/owner/ai?location_id=${lib.LOC}`);
  step(`the AI switch is reported as stored (enabled=${st.body?.enabled})`, st.status === 200 && typeof st.body?.enabled === 'boolean' && st.body.enabled === ig.body?.ai?.enabled, `${st.status} ${st.text}`);
  const m0 = await lib.api(`/api/public/locations/${lib.LOC}/menu?fresh=1`);
  const s = await lib.own(`/api/owner/products/${DISH}/sense/suggest?location_id=${lib.LOC}`, { location_id: lib.LOC });
  step(`Suggest answers 200 whatever the model did (source ${s.body?.source}, ai ${s.body?.ai?.provider ?? 'none'})`, s.status === 200 && ['lexicon', 'lexicon+model'].includes(s.body?.source), `${s.status} ${s.text}`);
  const m1 = await lib.api(`/api/public/locations/${lib.LOC}/menu?fresh=1`);
  step('Suggest wrote nothing', s.body?.saved === false && m1.body?.location?.menuVersion === m0.body?.location?.menuVersion);

  // 4b. the recipe draft (S7b)
  schema('Suggest .recipe matches its schema', s.body?.recipe, C.recipe_schema);
  step(`recipe state ${s.body?.recipe?.state}: a draft only when drafted`, s.body?.recipe && (s.body.recipe.state === 'drafted') === !!s.body.recipe.draft, JSON.stringify(s.body?.recipe));

  // 2. server For you
  const o = await place({ lib, run, must }, {}, phone); orders.push(o.id);
  const fy = await lib.api(`/api/order/${encodeURIComponent(o.id)}/taste/for-you`, { token: o.token });
  schema('for-you matches its schema', fy.body, C.for_you_response_schema);
  step(`for-you answers (${fy.body?.state}, ${fy.body?.items?.length ?? '-'} dishes) with no number in it`, fy.status === 200 && !/\d{3,}/.test(JSON.stringify(fy.body?.items || [])), `${fy.status} ${fy.text}`);
  const anon = await lib.api(`/api/order/${encodeURIComponent(o.id)}/taste/for-you`);
  step('no link, no answer', anon.status === 401, `${anon.status}`);
  const w = await lib.api(`/api/order/${encodeURIComponent(o.id)}/taste/withdraw`, { method: 'POST', token: o.token });
  step('the guest objects in one tap', w.status === 200 && w.body?.objected === true, `${w.status} ${w.text}`);
  const off = await lib.api(`/api/order/${encodeURIComponent(o.id)}/taste/for-you`, { token: o.token });
  step('after the objection: state off, nothing shown', off.status === 200 && off.body?.state === 'off' && (off.body?.items || []).length === 0, off.text);
  const nophone = await place({ lib, run, must }, { fulfilment: { kind: 'pickup', note: null } }, ''); orders.push(nophone.id);
  const np = await lib.api(`/api/order/${encodeURIComponent(nophone.id)}/taste/for-you`, { token: nophone.token });
  step('an order with no phone has no profile: 404', np.status === 404, `${np.status} ${np.text}`);
  const venueJs = await asset('/store/taste-venue.js');
  step('the deployed order page mounts it', venueJs.status === 200 && venueJs.text.includes('/taste/for-you') && venueJs.text.includes('forYouOrder'), `${venueJs.status}`);

  // 3. the weather credit in the builder
  const b = await asset('/admin/sense-builder.js');
  step('the segment builder draws the open-meteo.com credit', b.status === 200 && b.text.includes('${weatherSourceLine()}'), `${b.status}`);
  const v = await asset('/store/sense-view.js');
  step('the credit links open-meteo.com', v.status === 200 && v.text.includes('https://open-meteo.com/') && v.text.includes('weatherSourceLine'), `${v.status}`);

  // 4a. the radius on health (S7a)
  const h = await lib.own(`/api/owner/health?location_id=${lib.LOC}`);
  schema('health .sheaf matches its schema', h.body?.sheaf, C.health_sheaf_schema);
  step(`radius: ${h.body?.sheaf?.taste?.compared} compared, max ${h.body?.sheaf?.taste?.maxPm} per mille, acts false`, h.status === 200 && h.body?.sheaf?.taste?.acts === false, `${h.status}`);
  const more = await asset('/admin/more.js');
  step('the deployed health sheet loads the radius row', more.status === 200 && more.text.includes('/admin/sheaf-health.js'), `${more.status}`);
} catch (e) {
  step('the probe ran to the end', false, e.stack || String(e));
} finally {
  for (const id of orders) { try { await close({ lib, run, must }, id); } catch { /* sweep() catches it next run */ } }
}
verdict();
