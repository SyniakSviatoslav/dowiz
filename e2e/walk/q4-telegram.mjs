// Q4 TELEGRAM SCREEN on the QA hub -- it opens; the connect step renders and answers a missing token in words;
// with a group present the matrix renders, a tapped cell is saved (a reload shows it); "test" with no bot says so.
// No real bot token exists for QA and none is invented: the group is the LEGACY chat key (a placeholder chat id
// on the QA hub), which the hub reads as a group exactly as it reads a venue that predates groups.
//   HOST=https://qa-durres.dowiz.org LOC=qa-durres OUT=<dir> slot.sh qa node e2e/walk/q4-telegram.mjs
import { HOST, OUT, LOC, browser, csp, issues, step, end, api, creds } from './_lib.mjs';
import { ownerConsole } from './_qa.mjs';

if (!/qa-durres/.test(HOST)) { console.log('Q4 writes: QA hub only'); process.exit(2); }
const shot = n => `${OUT}/q4-${n}.png`;
const QA_CHAT = '-1009999999999';
const since = n => issues.slice(n).filter(i => !/http 503/.test(i));
const toastOf = async p => { await p.waitForTimeout(900); return p.evaluate(() => [...document.querySelectorAll('.toast, #toast, [role="status"]')].map(e => e.innerText.trim()).filter(Boolean).join(' | ')); };

async function openTg(o){
  await o.click('#nav [data-tab="more"]'); await o.waitForTimeout(1200);
  await o.click('[data-open="notifications"]'); await o.waitForTimeout(2000);
  await o.click('[data-go="telegram"]'); await o.waitForTimeout(3000);
  return o.$eval('#sheetIn', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
}

let b = null;
try {
  const tok = (await api('/api/auth/login', { method: 'POST', body: { email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD, location_id: LOC } })).body.access_token;
  const q = `?location_id=${LOC}`;
  // Start from no group, so the first screen is the one a new venue sees.
  let st = await api(`/api/owner/telegram${q}`, { token: tok });
  for (const g of st.body.groups || []) await api(`/api/owner/telegram/unlink${q}`, { method: 'POST', token: tok, body: { id: g.id } });
  await api('/api/owner/settings', { method: 'POST', token: tok, body: { key: 'notify.telegram.chat', value: '' } });
  st = await api(`/api/owner/telegram${q}`, { token: tok });
  step(st.status === 200 && (st.body.groups || []).length === 0, 'API: the QA hub starts with no Telegram group', `${st.status} groups=${(st.body.groups || []).length} bot=${JSON.stringify(st.body.bot)} tokenSet=${st.body.tokenSet} events=${(st.body.events || []).length}`);

  b = await browser();
  const ctx = await b.newContext({ viewport: { width: 390, height: 844 }, serviceWorkers: 'block' });
  let n0 = issues.length;
  const o = await ownerConsole(ctx, 'q4');
  let txt = await openTg(o);
  await o.screenshot({ path: shot('1-empty'), fullPage: true });
  const hasToken = await o.$('#tgToken').then(Boolean), addDisabled = await o.$eval('#tgAdd', e => e.disabled).catch(() => null);
  step(hasToken && addDisabled === true && !/tg_[a-zA-Z]+|tgev_/.test(txt), 'the screen opens: the bot step with a token field; "add group" waits for the bot', `add disabled=${addDisabled} :: ${txt.slice(0, 220)}`);
  step(since(n0).length === 0, 'opening it: no console error, no failed request', since(n0).join(' | '));

  // Connect with nothing typed: the words, not a status code.
  await o.click('#tgConnect'); const said = await toastOf(o);
  await o.screenshot({ path: shot('2-connect-empty') });
  step(/BotFather|token/i.test(said) && !/HTTP \d|undefined/.test(said), 'connect with no token says what to do', said);

  // A group, the way a venue that predates groups has one.
  const set = await api('/api/owner/settings', { method: 'POST', token: tok, body: { key: 'notify.telegram.chat', value: QA_CHAT } });
  step(set.status === 200, 'API: QA placeholder chat set (legacy key)', `${set.status}`);
  await o.keyboard.press('Escape'); await o.waitForTimeout(600);
  n0 = issues.length;
  txt = await openTg(o);
  const cells = await o.$$eval('[data-cell]', els => els.map(e => ({ cell: e.dataset.cell, mode: e.dataset.mode, disabled: e.disabled })));
  await o.screenshot({ path: shot('3-matrix'), fullPage: true });
  step(cells.length > 0 && await o.$('#tgMatrix').then(Boolean), 'the matrix renders one row per hub event', `${cells.length} cells; first ${JSON.stringify(cells.slice(0, 3))}`);
  step(!/tg_[a-zA-Z]+|tgev_[a-z]|tg_area_/.test(txt), 'every label is words, not a key', (txt.match(/tg_[a-zA-Z_]+|tgev_[a-z_.]+/g) || []).slice(0, 8).join(','));

  // Tap one live cell; the hub saves; a reload shows it saved.
  const live = cells.find(c => !c.disabled);
  if (live) {
    await o.click(`[data-cell="${live.cell}"]`); await o.waitForTimeout(2500);
    const after = await o.$eval(`[data-cell="${live.cell}"]`, e => e.dataset.mode).catch(() => null);
    step(after && after !== live.mode, 'a tapped cell changes mode on the screen', `${live.cell}: ${live.mode} -> ${after}`);
    await o.reload({ waitUntil: 'domcontentloaded' }); await o.waitForSelector('#nav:not([hidden])', { timeout: 40000 }); await o.waitForTimeout(2500);
    await openTg(o);
    const again = await o.$eval(`[data-cell="${live.cell}"]`, e => e.dataset.mode).catch(() => null);
    const api2 = await api(`/api/owner/telegram${q}`, { token: tok });
    const [gid, ev] = live.cell.split('/');
    const held = (api2.body.groups || []).find(g => g.id === gid)?.subs?.[ev];
    step(again === after && held === after, 'after a reload the cell is still saved (screen and API agree)', `screen=${again} api=${held}`);
    await o.screenshot({ path: shot('4-saved'), fullPage: true });
    // test send, no bot
    n0 = issues.length;
    await o.click(`[data-gtest="${gid}"]`); const t2 = await toastOf(o);
    await o.screenshot({ path: shot('5-test') });
    step(t2.length > 0 && !/HTTP \d|undefined|null/.test(t2), 'test with no bot says so in words', `${t2} ${since(n0).join(' | ')}`);
  } else step(false, 'no live cell to tap', JSON.stringify(cells.slice(0, 5)));
  await csp(o, 'q4');
} catch (e) {
  step(false, 'Q4 threw', e.stack?.slice(0, 400));
}
await end(b);
