// Q3 THE ASSISTANT on the QA hub, as the owner and as the kitchen: a question; a status; three proposals
// ("received 2 kg ...", "86 <dish>", "<order> ready") each confirmed with one tap and checked through the API.
//   HOST=https://qa-durres.dowiz.org LOC=qa-durres OUT=<dir> slot.sh qa node e2e/walk/q3-assistant.mjs
import { HOST, OUT, LOC, browser, consoleIn, csp, issues, step, end, api, sst } from './_lib.mjs';
import { ownerConsole, PHONE, qaOwnerToken, placeOrder, ownerOrder, stockOf, dishOf, ask, confirmLast } from './_qa.mjs';

if (!/qa-durres/.test(HOST)) { console.log('Q3 writes: QA hub only'); process.exit(2); }
const S = sst();
const K = S.qaKitchen;
const shot = n => `${OUT}/q3-${n}.png`;
async function again(fn, n = 6){ let r; for (let i = 0; i < n; i++) { r = await fn(); if (r && r.status !== 503) return r; } return r; }
let tok = null;
const ownerTok = async () => { for (let i = 0; i < 6 && !tok; i++) tok = await qaOwnerToken(); return tok; };
const UKR = /[Ѐ-ӿ]/;

/// One person's round: `who` is 'owner' or 'kitchen'; `p` their signed-in console page.
async function round(p, who, { supply, grams, dish }){
  // A question.
  let a = await ask(p, who === 'owner' ? 'which dish sold best this week?' : 'what should we prep first?', 9000);
  await p.screenshot({ path: shot(`${who}-1-question`) });
  step(a.said.length > 0 && !/asNotUnderstood|undefined|HTTP \d{3}/.test(a.said), `${who}: a question gets an answer in words, not a status code`, a.said.slice(0, 300));
  step(!UKR.test(a.said), `${who}: the answer is in the reader's language (the console is in English)`, a.said.slice(0, 200));
  // A status.
  a = await ask(p, 'how many orders are open?');
  step(/\d/.test(a.said), `${who}: "how many orders are open?" answers with numbers`, a.said.slice(0, 200));

  // Received 2 kg of a supply.
  const before = (await stockOf(await ownerTok(), supply))?.onHand;
  a = await ask(p, `received ${grams / 1000} kg ${supply.replace('qa-', 'QA ')}`);
  step(a.yes != null, `${who}: "received ${grams / 1000} kg" is proposed with a read-back`, a.said.slice(0, 200));
  if (a.yes != null) {
    const c = await confirmLast(p, a.yes);
    const after = (await stockOf(await ownerTok(), supply))?.onHand;
    await p.screenshot({ path: shot(`${who}-2-received`) });
    step(after - before === grams, `${who}: one tap, and the shelf holds ${grams} g more (API)`, `${before} -> ${after} state="${c.state}" toast="${c.toast}"`);
  }

  // 86 a dish, then put it back.
  const name = dish.replace('qa-', 'QA ').replace(/-/g, ' ');
  a = await ask(p, `86 ${name}`);
  step(a.yes != null, `${who}: "86 ${name}" is proposed`, a.said.slice(0, 200));
  if (a.yes != null) {
    const c = await confirmLast(p, a.yes);
    const d = await dishOf(dish);
    step(d?.available === false, `${who}: one tap, and the dish is off sale (API)`, `available=${d?.available} state="${c.state}"`);
    a = await ask(p, `${name} back on sale`);
    if (a.yes != null) await confirmLast(p, a.yes);
    const d2 = await dishOf(dish);
    step(d2?.available === true, `${who}: "${name} back on sale" puts it back (API)`, `available=${d2?.available} ${a.said.slice(0, 120)}`);
  }

  // Move an order: a fresh order, confirmed and cooked through the assistant, by the number the board shows.
  const o = await again(() => placeOrder(who));
  step(o.status === 200, `${who}: a fresh order to move`, `${o.status} ${o.id}`);
  if (o.id) {
    const shown = o.id.slice(-4).toUpperCase();
    for (const [line, want] of [[`confirm ${shown}`, 'CONFIRMED'], [`preparing ${shown}`, 'PREPARING'], [`move order ${shown} to ready`, 'READY']]) {
      a = await ask(p, line);
      if (a.yes == null) { step(false, `${who}: "${line}" (the number the board shows) is proposed`, a.said.slice(0, 200)); continue; }
      const c = await confirmLast(p, a.yes);
      const st = (await ownerOrder(await ownerTok(), o.id))?.status;
      step(st === want, `${who}: "${line}" + one tap -> ${want} (API)`, `${st} state="${c.state}"`);
    }
    await p.screenshot({ path: shot(`${who}-3-orders`) });
  }
}

let b = null;
try {
  b = await browser();
  const ctx = await b.newContext(PHONE);
  // The owner, in English so a stray Ukrainian line is visible.
  await ctx.addInitScript(() => { try { localStorage.setItem('dw_admin_lang', 'en'); } catch {} });
  const o = await ownerConsole(ctx, 'q3-owner');
  await round(o, 'owner', { supply: 'qa-salmon', grams: 2000, dish: 'qa-tea' });
  await csp(o, 'q3-owner');
  // The kitchen: the session the fixed sign-in form stores (see q2-kitchen.mjs).
  const k = await ctx.newPage();
  const sl = await again(() => api('/api/staff/login', { method: 'POST', body: { email: K.email, password: K.password } }));
  await k.goto(`${HOST}/admin/`, { waitUntil: 'domcontentloaded' });
  await k.evaluate(([t, l]) => { sessionStorage.setItem('dw_at', t); localStorage.setItem('dw_loc', l); }, [sl.body.jwt, sl.body.staff.locationId]);
  await k.reload({ waitUntil: 'domcontentloaded' }); await k.waitForSelector('#nav:not([hidden])', { timeout: 30000 }); await k.waitForTimeout(3000);
  await round(k, 'kitchen', { supply: 'qa-rice', grams: 1000, dish: 'qa-water' });
  await csp(k, 'q3-kitchen');
} catch (e) {
  step(false, 'Q3 threw', e.stack?.slice(0, 400));
}
await end(b);
