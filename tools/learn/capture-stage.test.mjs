// node --test tools/learn/capture-stage.test.mjs -- the recorder's staged state, without a browser.
import test from 'node:test';
import assert from 'node:assert/strict';
import { STAGES, COURIER, ROOM, stageFor, stagedAnswer, stateAfter, merge, courierOrder, round, speechShim, outboxShim } from './capture-stage.mjs';
import { lessonFile, loadLesson } from './capture-lib.mjs';

test('stageFor: the staged lessons have a stage, every other lesson none', () => {
  for (const id of ['C1', 'C2', 'C3', 'C4', 'C5', 'W2', 'W5', 'G1', 'G3']) assert.ok(stageFor(id), id);
  for (const id of ['O1a', 'W3', 'G2', 'Z99']) assert.equal(stageFor(id), null, id);
});

test('every stage names a lesson that exists, states it defines, and steps the lesson has', () => {
  for (const [id, st] of Object.entries(STAGES)) {
    const f = lessonFile(id);
    assert.ok(f, `${id}: no lesson file`);
    const { lesson } = loadLesson(f);
    if (st.answers) {
      assert.ok(st.answers[st.start], `${id}: start state ${st.start} is not defined`);
      for (const [n, s] of Object.entries(st.after || {})) {
        assert.ok(st.answers[s], `${id}: state ${s} after step ${n} is not defined`);
        assert.ok(Number(n) >= 1 && Number(n) <= lesson.steps.length, `${id}: step ${n} is not a step of the lesson`);
      }
    } else assert.ok(st.patch || st.local || st.speech, `${id}: a stage with no answers must patch, seed or hear something`);
    for (const p of Object.keys(st.writes || {})) assert.match(p, /^\/api\//, `${id}: a staged write is an /api/ path`);
    assert.ok(!st.writes || st.answers, `${id}: only a lesson whose screens are staged answers its writes`);
  }
});

test('stagedAnswer: a staged read answers from the current state; an unstaged path goes on to the venue', () => {
  const st = STAGES.C2;
  const off = stagedAnswer(st, 'offShift', 'GET', '/api/courier/tasks');
  assert.equal(off.status, 200);
  assert.equal(off.body.onShift, false);
  assert.deepEqual(off.body.mine, []);
  const pick = stagedAnswer(st, 'pickList', 'GET', '/api/courier/tasks');
  assert.equal(pick.body.onShift, true);
  assert.equal(pick.body.available.length, 2);
  assert.equal(stagedAnswer(st, 'pickList', 'GET', '/api/courier/position'), null);
  assert.equal(stagedAnswer(null, 'x', 'GET', '/api/courier/tasks'), null);
});

test('stagedAnswer: an offer counts down from when it was first served', () => {
  const a = stagedAnswer(STAGES.C2, 'offer', 'GET', '/api/courier/tasks', { since: 1000 });
  assert.equal(a.body.mine[0].offerEndsMs, 121000);
  assert.equal(stagedAnswer(STAGES.C2, 'offer', 'GET', '/api/courier/tasks', { since: 5000 }).body.mine[0].offerEndsMs, 125000);
});

test('stagedAnswer: a staged lesson answers its own writes in the browser; a patched one does not', () => {
  const w = stagedAnswer(STAGES.C2, 'offShift', 'POST', '/api/courier/shift');
  assert.deepEqual(w, { status: 200, body: { ok: true, staged: true } });
  assert.equal(stagedAnswer(STAGES.C2, 'offShift', 'OPTIONS', '/api/courier/shift'), null);
  assert.equal(stagedAnswer(STAGES.G3b, null, 'POST', '/api/public/locations/qa-durres/orders'), null);
});

test('stagedAnswer: a named write answers what the app reads; an offline one is never carried', () => {
  const v = stagedAnswer(STAGES.C6, 'waiting', 'POST', '/api/voice');
  assert.equal(v.status, 200);
  assert.equal(v.body.needsConfirmation, true);
  assert.ok(v.body.token && v.body.readback);
  assert.deepEqual(stagedAnswer(STAGES.C6, 'waiting', 'POST', '/api/courier/shift'), { status: 200, body: { ok: true, staged: true } });
  const pick = STAGES.C5.outbox[0].route;
  assert.deepEqual(stagedAnswer(STAGES.C5, 'active', 'POST', pick), { status: 0, offline: true });
  // the waiting tap is for the order on screen, with the tag courier/app.js's queuedFor reads
  assert.equal(STAGES.C5.outbox[0].tag, 'pickup:' + COURIER.active['/api/courier/tasks'].mine[0].id);
  assert.ok(pick.endsWith('/' + COURIER.active['/api/courier/tasks'].mine[0].id + '/pickup'));
});

test('speechShim: the page recogniser hears the phrase once, final, then ends', async () => {
  const saved = globalThis.window;
  globalThis.window = {};
  try {
    speechShim('two QA Salmon roll');
    const r = new window.webkitSpeechRecognition(), got = [];
    let ended = 0;
    r.onresult = ev => got.push(ev.results[ev.results.length - 1]);
    r.onend = () => { ended++; };
    r.start();
    await new Promise(res => setTimeout(res, 800));
    assert.equal(got.length, 1);
    assert.equal(got[0][0].transcript, 'two QA Salmon roll');
    assert.equal(got[0].isFinal, true);
    assert.equal(ended, 1);
    assert.equal(window.SpeechRecognition, window.webkitSpeechRecognition);
  } finally { globalThis.window = saved; }
  assert.equal(STAGES.G1.speech, 'two QA Salmon roll');
});

test('outboxShim: without IndexedDB it does nothing and does not throw', () => {
  assert.equal(typeof globalThis.indexedDB, 'undefined');
  assert.doesNotThrow(() => outboxShim([{ route: '/api/x', tag: 'pickup:x' }]));
  assert.doesNotThrow(() => outboxShim({ db: 'dowiz.room.outbox', rows: [{ route: '/api/x', tag: 'pay:x' }] }));
});

test('opened into the page: the shim fills the database it is given, the courier\'s by default', () => {
  const opened = [], saved = globalThis.indexedDB;
  globalThis.indexedDB = { open: name => { opened.push(name); return {}; } };
  try {
    outboxShim([{ route: '/api/x', tag: 'a' }]);
    outboxShim(STAGES.W1.outbox);
  } finally { if (saved === undefined) delete globalThis.indexedDB; else globalThis.indexedDB = saved; }
  assert.deepEqual(opened, ['dowiz.outbox', 'dowiz.room.outbox']);
});

test('W1 signs in on camera: signed out, the sign-in answered with the recorder\'s own session, one pay waiting', () => {
  const st = STAGES.W1;
  assert.equal(st.signedOut, true);
  const session = { jwt: 'j', staff: { locationId: 'qa-durres' } };
  for (const p of ['/api/staff/login', '/api/staff/claim'])
    assert.deepEqual(stagedAnswer(st, 'table', 'POST', p, { session }), { status: 200, body: session });
  assert.deepEqual(stagedAnswer(st, 'table', 'POST', '/api/staff/login', {}).body, { ok: true, staged: true });
  const pay = st.outbox.rows[0];
  assert.equal(pay.tag, 'pay:' + ROOM.table['/api/staff/room'].sittings[0].rounds[0].id);
  assert.deepEqual(stagedAnswer(st, 'table', 'POST', pay.route), { status: 0, offline: true });
});

test('the room: table 4 holds two rounds, so its tap opens the sitting (one round opens straight into the sheet)', () => {
  assert.equal(ROOM.table['/api/staff/room'].sittings[0].rounds.length, 2);
  assert.equal(ROOM.guest['/api/staff/room'].sittings[0].rounds.length, 1);
  const fl = stagedAnswer(STAGES.W4, 'table', 'GET', '/api/staff/floor').body;
  const tables = fl.zones[0].tables;
  assert.ok(tables.some(t => t.state === 'dirty' && t.sitting_id), 'a dirty table W4 can tap');
  assert.ok(fl.unplaced.some(u => u.state === 'dirty' && u.sitting_id), 'a dirty table on no plan');
  const z = JSON.parse(STAGES.W10.local.dw_room_till);
  assert.equal(z.ans.kind, 'till.closed');
  assert.ok(z.ans.expected && z.ans.over_short, 'a closed drawer carries its Z');
});

test('stagedAnswer: a patch merges into the venue answer, only on its own path', () => {
  const p = stagedAnswer(STAGES.G3, null, 'GET', '/api/public/locations/qa-durres/menu');
  assert.equal(p.status, 0);
  assert.equal(p.patch.location.pickup, true);
  assert.equal(stagedAnswer(STAGES.G3, null, 'GET', '/api/public/locations/other/menu'), null);
});

test('stateAfter: the state moves on only at the steps the stage names', () => {
  const st = STAGES.C2;
  assert.equal(stateAfter(st, 1, 'offShift'), 'pickList');
  assert.equal(stateAfter(st, 2, 'pickList'), 'pickList');
  assert.equal(stateAfter(st, 3, 'pickList'), 'offer');
  assert.equal(stateAfter(null, 1, 'x'), 'x');
  assert.equal(stateAfter(STAGES.G3, 1, null), null);
});

test('merge: objects deep, arrays and scalars replaced, the real answer kept elsewhere', () => {
  const real = { location: { pickup: false, name: 'QA', payments: { cash: true, card: false, crypto: [] } }, categories: [1, 2] };
  const m = merge(real, { location: { pickup: true, payments: { card: true } } });
  assert.deepEqual(m, { location: { pickup: true, name: 'QA', payments: { cash: true, card: true, crypto: [] } }, categories: [1, 2] });
  assert.deepEqual(merge({ a: [1, 2] }, { a: [3] }), { a: [3] });
  assert.equal(real.location.pickup, false, 'the real answer is not mutated');
  assert.deepEqual(merge(null, { a: 1 }), { a: 1 });
});

test('fixtures: the shapes the apps read (courier tasks, room sittings)', () => {
  for (const [name, s] of Object.entries(COURIER)) {
    const t = typeof s['/api/courier/tasks'] === 'function' ? s['/api/courier/tasks']({ since: 0 }) : s['/api/courier/tasks'];
    assert.ok(Array.isArray(t.mine) && Array.isArray(t.available) && typeof t.onShift === 'boolean', name);
    for (const o of [...t.mine, ...t.available]) {
      assert.ok(o.id && Number.isInteger(o.total) && o.address.line && ['READY', 'IN_DELIVERY'].includes(o.status), `${name} ${o.id}`);
    }
  }
  assert.equal(COURIER.onTheWay['/api/courier/tasks'].mine[0].payment, 'cash', 'C4 needs cash at the door');
  assert.equal(COURIER.nextRun['/api/courier/tasks'].mine[0].status, 'IN_DELIVERY', 'C4 refusal needs a run on the road');
  for (const [name, s] of Object.entries(ROOM)) {
    for (const sit of s['/api/staff/room'].sittings) {
      assert.ok(sit.sitting_id && sit.table && sit.rounds.length, name);
      for (const r of sit.rounds) {
        assert.equal(r.subtotal, r.items.reduce((a, i) => a + i.quantity * i.unit_price, 0), `${name} ${r.id}: subtotal is the lines' sum`);
        assert.equal(r.total, r.subtotal - r.discount + r.tip);
      }
    }
  }
  assert.ok(ROOM.guest['/api/staff/room'].sittings[0].rounds.some(r => r.placed_by === 'guest' && r.status === 'PENDING'), 'W6 needs a guest round');
  assert.equal(courierOrder({ total: 5 }).total, 5);
  assert.equal(round({ seq: 9 }).seq, 9);
});

test('pick: a stage taps only options that exist, on the steps that offer them (W7: the wallet, the euro)', async () => {
  const { METHODS, TILL_CURRENCIES } = await import('../../workers/api/public/room/logic.js');
  const { lessonFile, loadLesson } = await import('./capture-lib.mjs');
  const { lesson } = loadLesson(lessonFile('W7'));
  const anchor = n => lesson.steps.find(s => s.n === n).anchor;
  assert.equal(anchor(3), 'pay.method'); assert.ok(METHODS.includes(STAGES.W7.pick[3]));
  assert.equal(anchor(4), 'pay.currency'); assert.ok(TILL_CURRENCIES.includes(STAGES.W7.pick[4]));
});

test('a sign-in or refresh the stage does not name goes to the venue; one it names is answered', () => {
  assert.equal(stagedAnswer(STAGES.O13, 'list', 'POST', '/api/auth/refresh'), null);
  assert.equal(stagedAnswer(STAGES.W2, 'table', 'POST', '/api/staff/login'), null);
  assert.deepEqual(stagedAnswer(STAGES.O13, 'list', 'PUT', '/api/owner/customers/c-ana/record'), { status: 200, body: { ok: true, staged: true } });
  const owner = { access_token: 'a', user: { locationId: 'qa-durres' } };
  assert.deepEqual(stagedAnswer(STAGES.O16a, 'in', 'POST', '/api/auth/login', { session: owner }), { status: 200, body: owner });
  assert.equal(STAGES.O16a.signedOut, true);
});

test('choose: a stage sets only a select the lesson has, to a value it offers, before the step it shows (O14b)', async () => {
  const { chooseFor } = await import('./capture-stage.mjs');
  const { readFileSync } = await import('node:fs');
  const src = readFileSync(new URL('../../workers/api/public/admin/campaigns.js', import.meta.url), 'utf8');
  const segs = JSON.parse(src.match(/SEGMENTS\s*=\s*(\[[^\]]*\])/)[1].replace(/'/g, '"'));
  const { lesson } = loadLesson(lessonFile('O14b'));
  const anchor = n => lesson.steps.find(s => s.n === n).anchor;
  assert.equal(anchor(7), 'campaigns.days'); assert.equal(chooseFor(STAGES.O14b, 7)[1], 'not_seen_since');
  assert.equal(anchor(8), 'campaigns.tag'); assert.equal(chooseFor(STAGES.O14b, 8)[1], 'tag');
  for (const [n, [sel, v]] of Object.entries(STAGES.O14b.choose)) {
    assert.ok(segs.includes(v), `step ${n}: ${v} is not a segment`);
    assert.ok(lesson.steps.some(s => s.action.selector === sel), `step ${n}: ${sel} is no step of the lesson`);
  }
  assert.equal(chooseFor(STAGES.O14b, 1), null); assert.equal(chooseFor(STAGES.W7, 3), null); assert.equal(chooseFor(null, 1), null);
});

test('the owner stages hold the state each lesson shows (O14a-O19)', async () => {
  const ans = (id, path, state = STAGES[id].start) => stagedAnswer(STAGES[id], state, 'GET', path).body;
  // O14a: a running code and a scheduled one, so the flip and the delete are on screen
  assert.equal(ans('O14a', '/api/owner/promotions').promotions.length, 2);
  // O14b: the saved campaign is the one its detail reads, and the preview has someone new to send to
  const saved = stagedAnswer(STAGES.O14b, 'list', 'POST', '/api/owner/campaigns').body.campaign;
  assert.equal(ans('O14b', `/api/owner/campaigns/${saved.id}`).campaign.id, saved.id);
  const pv = stagedAnswer(STAGES.O14b, 'list', 'POST', `/api/owner/campaigns/${saved.id}/preview`).body.preview;
  assert.ok(pv.count - pv.already > 0, 'the send button unlocks only with someone new');
  assert.ok(ans('O14b', '/api/owner/campaigns').tags.length > 0, 'the tag select has options');
  // O14c: a draft, so approve and reject are drawn
  assert.ok(ans('O14c', '/api/owner/posts').posts.some(p => p.state === 'draft'));
  // O16d: every thread the list shows reads back under the path its peer makes (no '+' to encode)
  const th = ans('O16d', '/api/owner/inbox').threads[0];
  assert.equal(encodeURIComponent(th.peer), th.peer);
  assert.ok(ans('O16d', `/api/owner/inbox/${th.peer}`).messages.length > 0);
  // O16e: not accepted in the current version (the tick and Accept are drawn); Accept answers accepted
  const d = ans('O16e', '/api/owner/dpa');
  assert.equal(d.current, false);
  const a = stagedAnswer(STAGES.O16e, 'old', 'POST', '/api/owner/dpa/accept').body;
  assert.equal(a.current, true); assert.equal(a.accepted.version, d.version);
  // O16f: a connected bot with one group (its Test button carries notify.test), and a print queue
  assert.ok(ans('O16f', '/api/owner/telegram').bot && ans('O16f', '/api/owner/telegram').groups.length === 1);
  assert.ok(ans('O16f', '/api/owner/print/jobs').jobs.length > 0);
  // O16h / O16j: a key in each list, so its row and its revoke are drawn
  assert.equal(ans('O16h', '/api/owner/apikeys').keys.length, 1);
  assert.equal(ans('O16j', '/api/owner/mcp/keys').keys.length, 1);
  // O19: the heard words come back as a rejection, the one verb that draws the reason field
  const { needsReason } = await import('../../workers/api/public/admin/voice-plan.js');
  const v = stagedAnswer(STAGES.O19, 'on', 'POST', '/api/voice').body;
  assert.ok(v.needsConfirmation && needsReason(v.verb) && STAGES.O19.speech);
  // an unstaged read still goes to the venue
  assert.equal(stagedAnswer(STAGES.O16f, 'on', 'GET', '/api/owner/settings'), null);
});
