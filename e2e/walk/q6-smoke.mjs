// Q6 SMOKE -- the storefront, the waiter room, the courier app and the console's sign-in open on a phone with no console error and
// no failed request; a wrong password on each sign-in is answered in the hub's words. Read-only.
//   HOST=https://qa-durres.dowiz.org OUT=<dir> slot.sh qa node e2e/walk/q6-smoke.mjs
import { HOST, OUT, browser, watch, csp, issues, step, end } from './_lib.mjs';
import { PHONE, storeReady } from './_qa.mjs';

const shot = n => `${OUT}/q6-${n}.png`;
const since = n => issues.slice(n).filter(i => !/http 503|status of 503/.test(i));
let b = null;
try {
  b = await browser();
  const ctx = await b.newContext(PHONE);
  let n0 = issues.length;
  const { c, taps } = await storeReady(ctx, 'q6-store');
  const cards = await c.$$eval('.card', els => els.length);
  await c.screenshot({ path: shot('store') });
  await csp(c, 'q6-store');
  step(cards >= 5 && since(n0).length === 0, 'storefront opens with the menu, clean', `${cards} cards; ${taps} retry tap(s) after a platform 503; ${since(n0).join(' | ')}`);

  for (const [app, path, form] of [['room', '/room/', 'form[data-form="login"]'], ['courier', '/courier/', '#loginForm'], ['admin', '/admin/', '#e']]) {
    n0 = issues.length;
    const p = await ctx.newPage(); watch(p, `q6-${app}`);
    await p.goto(`${HOST}${path}`, { waitUntil: 'domcontentloaded', timeout: 90000 });
    const ok = await p.waitForSelector(form, { timeout: 40000 }).then(() => true).catch(() => false);
    await p.waitForTimeout(2500);
    await p.screenshot({ path: shot(`${app}-open`) });
    await csp(p, `q6-${app}`);
    step(ok && since(n0).length === 0, `${app} app opens to its sign-in, clean`, since(n0).join(' | '));
    // A wrong password: the hub says "invalid credentials" in plain text.
    n0 = issues.length;
    if (app === 'room') {
      await p.fill('input[name="email"]', 'qa-nobody@example.com'); await p.fill('input[name="password"]', 'wrong-password-1');
      await p.click('form[data-form="login"] button[type="submit"]');
    } else if (app === 'admin') {
      await p.fill('#e', 'qa-nobody@example.com'); await p.fill('#p', 'wrong-password-1');
      await p.click('#go');
    } else {
      await p.fill('#em', '+355690000001'); await p.fill('#pw', 'wrong-password-1');
      await p.click('#go');
    }
    // A toast leaves after a few seconds: the page is read every half second for eight.
    let said = '';
    for (let i = 0; i < 16; i++) { await p.waitForTimeout(500); said = await p.evaluate(() => document.body.innerText.replace(/\s+/g, ' ')); if (/credentials|JSON|HTTP \d/.test(said)) break; }
    await p.screenshot({ path: shot(`${app}-wrong-password`) });
    const words = /invalid credentials|not staff|Kredenciale|невір/i.test(said);
    step(words && !/is not valid JSON|Unexpected token/.test(said), `${app}: a wrong password is answered in the hub's words`, (said.match(/.{0,60}(JSON|credentials|HTTP \d+).{0,40}/) || [said.slice(0, 160)])[0]);
  }
} catch (e) {
  step(false, 'Q6 threw', e.stack?.slice(0, 400));
}
await end(b);
