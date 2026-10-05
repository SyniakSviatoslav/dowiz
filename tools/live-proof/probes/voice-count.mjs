// LIVE PROBE voice.kitchen_count.v1 (W-VOICE, P14) on qa-durres -- main runs
// it after the deploy; a lane never runs it against production.
//
//   set -a; . /root/.dowiz_owner; set +a; node tools/live-proof/probes/voice-count.mjs
//   VOICE_COUNT_WRITE=1 node tools/live-proof/probes/voice-count.mjs   (also adds the ONE confirmed line)
//
// WHAT IT PROVES, through the real Worker, signing key and venue object:
//   1. A spoken count of a real supply of the QA shelf (the first one counted
//      in g; its name is read from GET /api/owner/stock) answers a PROPOSAL
//      that validates against the contract, in all four languages, with the
//      quantity in base units: "two kilo three hundred" = 2300 g.
//   2. "2,3 kg" and "2.3 kg" are the same 2300; "2,300" is a QUESTION.
//   3. A missing unit is a QUESTION (no token); an unknown supply is a QUESTION.
//   4. NOTHING IS WRITTEN by a proposal: the shelf's `sessions` are unchanged.
//   5. The token is required: {confirm: <token>} answers {action:'do', verb:
//      'count', args:{itemId, observed}}; a forged token is refused
//      (understood:false); a proposal's args never reach the count route
//      without it (the console only posts `args` from a confirmation).
//   6. (opt-in, VOICE_COUNT_WRITE=1) the confirmed line goes to the EXISTING
//      POST /api/owner/stock/count with a vc_<ms> session, answers 200, and
//      the shelf lists that session. It writes ONE count line of the QA venue
//      (the observed amount is the supply's own current on-hand, so the
//      count records no drift).
// Exit 0 = every step held; 1 = a step failed; 3 = NEEDS-KEY / NEEDS-DATA.
import { LOC, own, owner, reporter, contract, read, shelf, validate } from './stock-lib.mjs';

const C = contract('voice-count');
const S = C.response_schema;
const { step, verdict } = reporter('voice-count');
const as = (name, body, def) => { const bad = validate(body, S.$defs[def], S); return step(name, bad.length === 0, bad.slice(0, 6).join('; ')); };
const voice = (transcript, lang) => own('/api/voice', { transcript, confidence: 0.95, is_final: true, lang });

try {
  step('the QA owner signs in', !!(await owner()));
  const sh = await shelf(own, LOC);
  step('GET /api/owner/stock answers', sh.status === 200, `${sh.status}`);
  const nm = s => typeof s.name === 'string' ? s.name : (s.name?.en || '');
  const g = sh.supplies.map(s => ({ ...s, name: nm(s) })).filter(s => (s.unit || 'g') === 'g' && /^[\p{L} ]{3,}$/u.test(s.name));
  const sup = g.find(s => g.filter(x => String(x.name).toLowerCase().split(' ')[0] === String(s.name).toLowerCase().split(' ')[0]).length === 1);
  if (!sup) {
    step('a supply counted in g with a one-word-unique name exists on the QA shelf', 'NEEDS-KEY', 'NEEDS-DATA: add one supply in g to the QA venue');
  } else {
    const name = String(sup.name).toLowerCase();
    const sessions = JSON.stringify(sh.body?.sessions || []);

    const said = [
      ['en', `count ${name} two kilo three hundred`], ['en', `count ${name} 2,3 kg`], ['en', `count ${name} 2.3 kg`],
      ['uk', `рахую ${name} два кіло триста`], ['ru', `считаю ${name} два кило триста`], ['sq', `numero ${name} dy kile e treqind`],
    ];
    let last = null;
    for (const [lang, t] of said) {
      const r = await voice(t, lang);
      step(`"${t}" answers 200`, r.status === 200, `${r.status} ${r.text?.slice(0, 120)}`);
      as(`"${t}" is a proposal (voice.kitchen_count.v1)`, r.body, 'proposal');
      step(`"${t}" is 2300 g of ${sup.id}`, r.body?.observed === 2300 && r.body?.itemId === sup.id, JSON.stringify({ observed: r.body?.observed, itemId: r.body?.itemId, readback: r.body?.readback }));
      if (r.body?.token) last = r.body;
    }

    for (const [why, t] of [['no unit', `count ${name} 2`], ['"2,300" (2.3 or 2300?)', `count ${name} 2,300 kg`], ['an unknown supply', 'count zzqqxx 2 kg']]) {
      const r = await voice(t, 'en');
      as(`${why} is a QUESTION, never a guess`, r.body, 'question');
      step(`${why} carries no token`, !r.body?.token, JSON.stringify(r.body).slice(0, 160));
    }

    const after = await shelf(own, LOC);
    step('a proposal writes nothing (count sessions unchanged)', JSON.stringify(after.body?.sessions || []) === sessions);

    const forged = await own('/api/voice', { confirm: `${last?.token || 'x'}x`, lang: 'en' });
    step('a forged token is refused', forged.status === 200 && forged.body?.understood === false, JSON.stringify(forged.body).slice(0, 120));
    const ok = await own('/api/voice', { confirm: last?.token, lang: 'en' });
    as('the signed token confirms {itemId, observed}', ok.body, 'confirmed');
    step('the confirmation carries what was proposed', ok.body?.args?.itemId === sup.id && ok.body?.args?.observed === 2300, JSON.stringify(ok.body?.args));

    if (process.env.VOICE_COUNT_WRITE === '1') {
      const onHand = Number.isInteger(sup.onHand) ? Math.max(0, sup.onHand) : Number.isInteger(sup.qty) ? Math.max(0, sup.qty) : null;
      if (onHand === null) step('the supply shows its on-hand (to count without drift)', 'NEEDS-KEY', JSON.stringify(sup).slice(0, 160));
      else {
        const session = `vc_${Date.now()}`;
        const body = { session, lines: [{ item: sup.id, observed: onHand }] };
        const bad = validate(body, C.count_route.body_schema);
        step('the count body validates', bad.length === 0, bad.join('; '));
        const w = await own(`/api/owner/stock/count?location_id=${LOC}`, body);
        step('POST /api/owner/stock/count adds the line', w.status === 200, `${w.status} ${w.text?.slice(0, 160)}`);
        const s2 = await read(own, `/api/owner/stock?location_id=${LOC}`);
        step('the shelf lists the vc_ session', JSON.stringify(s2.body?.sessions || []).includes(session), JSON.stringify(s2.body?.sessions || []).slice(0, 200));
      }
    }
  }
} catch (e) {
  step('the probe ran to the end', false, e.stack?.slice(0, 300));
} finally {
  verdict();
}
