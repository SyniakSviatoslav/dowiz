// THE TILL, for the Counter-Manager (`Cap::OpenTill`): open with a float per
// currency, cash in and out with a reason, a BLIND count, and the close.
// `POST /api/staff/till/{open,count,close,pay_in,pay_out}` (till.rs).
//
// The tips beside it are READ (`GET /api/staff/till/tips`, the period of
// the drawer this phone knows). There is no till READ route, so this screen shows the last answer THIS
// phone received, stored only as `visible()` -- the projection that cannot
// carry `expected` before a close. Opening on a second phone shows
// "not known here" until that phone does something to the drawer.
import { esc, TILL_CURRENCIES, parseMinor } from './logic.js';
import { renderTill, visible, tipsQuery, renderTips } from './till-view.js';
import { safeGet, safeSet } from '../store/storage.js';

const KEY = 'dw_room_till';

export function lastTill(loc) {
  try { const v = JSON.parse(safeGet(KEY) || 'null'); return v && v.loc === loc ? v.ans : null; } catch { return null; }
}
export function keep(loc, ans) { safeSet(KEY, JSON.stringify({ loc, ans: visible(ans) })); }

const pileInputs = (t, name) => TILL_CURRENCIES.map(c =>
  `<label>${esc(c)}<input name="${name}_${c}" inputmode="decimal" autocomplete="off" placeholder="0"></label>`).join('');

export function renderTillScreen(c) {
  const { S, t } = c;
  const ans = S.till;
  const open = ans && ans.open;
  const curOpts = TILL_CURRENCIES.map(x => `<option value="${x}">${x}</option>`).join('');
  return `
    <div class="bar"><button class="btn" data-act="back">← ${esc(t('back'))}</button></div>
    <h2>${esc(t('till'))}</h2>
    <section class="till-state" aria-live="polite">${renderTill(ans, t, c.locale())}</section>
    ${tipsQuery(ans, S.loc) ? `<section class="card-form till-tips" data-tips aria-live="polite"><h3>${esc(t('tipsTitle'))}</h3></section>` : ''}
    ${open ? '' : `<form class="card-form" data-form="open"><h3>${esc(t('openTill'))}</h3>
      <p class="muted">${esc(t('floatHint'))}</p><div class="piles">${pileInputs(t, 'f')}</div>
      <button class="cta" type="submit">${esc(t('openTill'))}</button></form>`}
    <form class="card-form" data-form="move"><h3>${esc(t('cashMove'))}</h3>
      <div class="seg" role="radiogroup" aria-label="${esc(t('cashMove'))}">
        <label class="seg-b"><input type="radio" name="dir" value="pay_in" checked> ${esc(t('payIn'))}</label>
        <label class="seg-b"><input type="radio" name="dir" value="pay_out"> ${esc(t('payOut'))}</label></div>
      <div class="piles"><label>${esc(t('currency'))}<select name="currency">${curOpts}</select></label>
        <label>${esc(t('amount'))}<input name="amount" inputmode="decimal" autocomplete="off" required></label></div>
      <label>${esc(t('reasonText'))}<input name="reason" maxlength="140" required autocomplete="off"></label>
      <button class="btn" type="submit">${esc(t('send'))}</button></form>
    <form class="card-form" data-form="count"><h3>${esc(t('count'))}</h3>
      <p class="muted">${esc(t('countHint'))}</p><div class="piles">${pileInputs(t, 'o')}</div>
      <button class="btn" type="submit">${esc(t('saveCount'))}</button></form>
    <form class="card-form" data-form="close"><h3>${esc(t('closeTill'))}</h3>
      <p class="muted">${esc(t('closeHint'))}</p>
      <label class="check"><input type="checkbox" name="sure" required> ${esc(t('closeSure'))}</label>
      <button class="cta danger" type="submit">${esc(t('closeTill'))}</button></form>`;
}

/// A per-currency map from the pile inputs. Empty fields are left out; a
/// field that is not an amount makes the whole map null.
export function piles(get, prefix) {
  const m = {};
  for (const c of TILL_CURRENCIES) {
    const raw = String(get(`${prefix}_${c}`) ?? '').trim();
    if (!raw) continue;
    const n = parseMinor(raw, c);
    if (n == null) return null;
    m[c] = n;
  }
  return m;
}

export function bindTill(c, root) {
  const { S, t } = c;
  loadTips(c, root);
  root.onclick = ev => {
    const b = ev.target.closest('[data-act]');
    if (b && b.dataset.act === 'back') { S.view = 'room'; c.render(); }
  };
  root.onsubmit = async ev => {
    ev.preventDefault();
    const f = ev.target, kind = f.dataset.form, get = n => f.elements[n]?.value;
    let path, body = { location_id: S.loc };
    if (kind === 'open') {
      const float = piles(get, 'f');
      if (!float) return c.toast(t('badAmount'));
      path = 'open'; body.float = float;
    } else if (kind === 'count') {
      const observed = piles(get, 'o');
      if (!observed || !Object.keys(observed).length) return c.toast(t('badAmount'));
      // A pile left empty was counted as nothing, not skipped: the close
      // compares every currency the drawer holds.
      for (const cur of TILL_CURRENCIES) if (!(cur in observed)) observed[cur] = 0;
      path = 'count'; body.observed = observed;
    } else if (kind === 'move') {
      const currency = get('currency'), amount = parseMinor(get('amount'), currency), reason = String(get('reason') || '').trim();
      if (!amount) return c.toast(t('badAmount'));
      if (!reason) return c.toast(t('needReason'));
      path = f.elements.dir.value === 'pay_out' ? 'pay_out' : 'pay_in';
      Object.assign(body, { currency, amount, reason });
    } else if (kind === 'close') {
      if (!f.elements.sure.checked) return;
      path = 'close';
    } else return;
    const btn = f.querySelector('button[type="submit"]');
    if (btn) btn.disabled = true;
    try {
      const r = await c.write('/staff/till/' + path, body, 'till:' + path);
      if (r.landed) { S.till = visible(r.data); keep(S.loc, r.data); c.toast(t('saved')); }
      else c.toast(t(r.queued ? 'queuedSaved' : r.reason === 'full' ? 'queueFull' : 'queueNoStore'));
    } catch (e) {
      c.toast(e.message || t('error'));
    }
    c.render();
  };
}

/// The tip record for the period on screen, read from the server each time the
/// till is drawn. A failed read says so; it never draws a zero.
async function loadTips(c, root) {
  const { S, t } = c;
  const q = tipsQuery(S.till, S.loc), el = root.querySelector('[data-tips]');
  if (!q || !el) return;
  try { el.innerHTML = renderTips(await c.api(q), t, c.locale()); }
  catch (e) { el.innerHTML = `<h3>${esc(t('tipsTitle'))}</h3><p class="muted">${esc(t('tipsFailed'))} ${esc(e.message || '')}</p>`; }
}
