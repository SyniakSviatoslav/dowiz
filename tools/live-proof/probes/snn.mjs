// LIVE PROBE snn.shadow (W-SNN) on qa-durres -- main runs it after the deploy; a lane never runs
// it against production.
//
//   set -a; . /root/.dowiz_owner; set +a; node tools/live-proof/probes/snn.mjs
//   SNN_WRITE=1 node tools/live-proof/probes/snn.mjs   (also flips the switch off and back)
//
// WHAT IT PROVES, through the real Worker and venue object:
//   1. GET /api/owner/snn answers the contract's shape (mode, model 20261006, counts per venue only).
//   2. /api/owner/health carries the same object under `snn`, equal field for field.
//   3. A mode outside shadow|on|off is a 400 by name (nothing is written).
//   4. (SNN_WRITE=1) POST {mode:'off'} answers {ok, mode:'off'}, GET reads 'off', the counts do not
//      move while off; POST {mode:'shadow'} restores the default and GET reads 'shadow'.
// What it does NOT prove: that a guest's order page is counted (that needs a QA order with a phone;
// the in-memory route test drives it). Exit 0 = every step held; 1 = a step failed; 3 = NEEDS-KEY.
import { LOC, own, owner, reporter, contract, read } from './stock-lib.mjs';

const C = contract('snn');
const { step, schema, verdict } = reporter('snn');

try {
  step('the QA owner signs in', !!(await owner()));
  const v = await read(own, `/api/owner/snn?location_id=${LOC}`);
  step('GET /api/owner/snn answers 200', v.status === 200, `${v.status} ${v.text?.slice(0, 160)}`);
  schema('GET /api/owner/snn matches snn.shadow 1.2.0', v.body, C.response_schema);
  step('the model is the shipped one', v.body?.model === C.model.id || v.body?.compared === 0, JSON.stringify(v.body));
  const h = await read(own, `/api/owner/health?location_id=${LOC}`);
  step('GET /api/owner/health answers 200', h.status === 200, `${h.status}`);
  schema('health.snn matches the same schema', h.body?.snn, C.response_schema);
  step('health.snn says the same mode', h.body?.snn?.mode === v.body?.mode, `${h.body?.snn?.mode} vs ${v.body?.mode}`);
  const bad = await own('/api/owner/snn', { mode: 'maybe' });
  step('an unknown mode is a 400 by name', bad.status === 400 && /shadow, on, off/.test(bad.text || ''), `${bad.status} ${bad.text?.slice(0, 160)}`);
  if (process.env.SNN_WRITE === '1') {
    const before = v.body?.compared;
    const off = await own('/api/owner/snn', { mode: 'off' });
    schema('POST {mode:off} matches the set schema', off.body, C.set_response_schema);
    const r1 = await read(own, `/api/owner/snn?location_id=${LOC}`);
    step('GET reads off, counts unchanged', r1.body?.mode === 'off' && r1.body?.compared === before, JSON.stringify(r1.body));
    const back = await own('/api/owner/snn', { mode: 'shadow' });
    step('POST {mode:shadow} restores the default', back.status === 200 && back.body?.mode === 'shadow', `${back.status}`);
    const r2 = await read(own, `/api/owner/snn?location_id=${LOC}`);
    step('GET reads shadow again', r2.body?.mode === 'shadow', JSON.stringify(r2.body));
  }
} catch (e) {
  step('the probe ran to the end', false, String(e?.message || e));
}
verdict();
