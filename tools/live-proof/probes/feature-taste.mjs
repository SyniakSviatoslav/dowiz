// LIVE PROBE taste (W-TASTE, 2026-10-05) on qa-durres -- main runs it AFTER the deploy; a lane never
// runs it against production.
//
//   node tools/live-proof/probes/feature-taste.mjs      (FLOWS_HOST may name another qa- hub)
//
// 1. the DEPLOYED /store/taste-int.js carries the hub's half-life table and, imported, ranks the
//    shared fixture's first 50 guests exactly as the file says (with the deployed taste.js + sense.js);
// 2. the deployed storefront carries the open-meteo.com credit (sense-view.js) in four languages
//    (sense-words.js) and draws it under the strip (taste-device.js);
// 3. Suggest on qa-water answers feature-sense 1.1.0: `ai` provenance (null when AI is off),
//    source lexicon | lexicon+model, never the model's text, and writes nothing.
// The DO-internal `/fold/menu?block=taste` is not reachable from outside: in-memory proof only.
// A step that cannot run is a FAIL, never a skip.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter } from './stock-lib.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const C = contract('taste');
const S = contract('sense');
const { step, schema, verdict } = reporter('feature-taste');
const FIX = JSON.parse(fs.readFileSync(path.join(HERE, '../../../crates/dowiz-hub/fixtures/rank/strip.json'), 'utf8'));
const DISH = 'qa-water';

const asset = async p => {
  const r = await fetch(lib.HOST + p, { headers: { 'user-agent': lib.UA } });
  return { status: r.status, text: await r.text() };
};

try {
  // 1. the deployed device arithmetic, imported from what the phone downloads
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'taste-probe-'));
  for (const f of ['taste-int.js', 'taste.js', 'sense.js']) {
    const a = await asset(`/store/${f}`);
    if (!step(`/store/${f} is served`, a.status === 200, `${a.status}`)) throw new Error(`no ${f}`);
    fs.writeFileSync(path.join(dir, f), a.text);
  }
  const I = await import(pathToFileURL(path.join(dir, 'taste-int.js')).href);
  const T = await import(pathToFileURL(path.join(dir, 'taste.js')).href);
  schema('the deployed arithmetic matches its schema', { half: I.HALF, fade_60: I.fade(2000, 60), step: String(I.STEP) }, C.device_asset_schema);
  step('the deployed half-life table is the hub\'s', JSON.stringify(I.HALF) === JSON.stringify(FIX.half), JSON.stringify(I.HALF.slice(0, 4)));
  let same = 0;
  const cases = FIX.cases.slice(0, 50);
  for (const c of cases) {
    const got = T.scored(FIX.menus[c.menu], c.profile, c.day, c.opts).map(x => [x.id, x.why, x.guessed ? 1 : 0, x.s]);
    if (JSON.stringify(got) === JSON.stringify(c.want)) same++;
  }
  step(`the deployed strip ranks ${same}/${cases.length} fixture guests as the hub does`, same === cases.length);

  // 2. the weather credit (CC BY 4.0)
  const view = await asset('/store/sense-view.js');
  step('sense-view.js carries the open-meteo.com credit', view.status === 200 && view.text.includes('https://open-meteo.com/') && view.text.includes('weatherCredit'), `${view.status}`);
  const words = await asset('/lib/sense-words.js');
  step('the credit is worded in four languages', words.status === 200 && (words.text.match(/sx_weather:/g) || []).length === 4, `${words.status}`);
  const dev = await asset('/store/taste-device.js');
  step('the For-you strip draws it', dev.status === 200 && dev.text.includes('weatherCredit(momentKeys())'), `${dev.status}`);

  // 3. Suggest with the venue's model, held to the vocabulary
  const m0 = await lib.api(`/api/public/locations/${lib.LOC}/menu?fresh=1`);
  const s = await lib.own(`/api/owner/products/${DISH}/sense/suggest?location_id=${lib.LOC}`, { location_id: lib.LOC });
  schema('suggest matches feature-sense 1.1.0', s.body, S.suggest_response_schema);
  const ai = s.body?.ai;
  step(`suggest says where the draft came from (${s.body?.source}, ai ${ai ? ai.provider ?? 'tried, none answered' : 'off'})`,
    s.status === 200 && ['lexicon', 'lexicon+model'].includes(s.body?.source) && (ai === null || (typeof ai === 'object' && Array.isArray(ai.failed))), `${s.status} ${s.text}`);
  step('a model-added id is marked from=model', s.body?.source !== 'lexicon+model' || (s.body?.why || []).some(w => w.from === 'model'), JSON.stringify(s.body?.why));
  const m1 = await lib.api(`/api/public/locations/${lib.LOC}/menu?fresh=1`);
  step('suggest wrote nothing', s.body?.saved === false && m1.body?.location?.menuVersion === m0.body?.location?.menuVersion, `${m0.body?.location?.menuVersion} -> ${m1.body?.location?.menuVersion}`);
} catch (e) {
  step('the probe ran to the end', false, e.stack || String(e));
}
verdict();
