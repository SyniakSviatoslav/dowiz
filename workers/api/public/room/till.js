// THE TILL, for the Counter-Manager (`Cap::OpenTill`): open with a float per
// currency, cash in and out with a reason, a BLIND count, and the close.
// `POST /api/staff/till/{open,count,close,pay_in,pay_out}` (till.rs).
//
// The tips beside it are READ (`GET /api/staff/till/tips`, the period of
// the drawer this phone knows, else the venue's day -- shown with or without a till). There is no till READ route, so this screen shows the last answer THIS
// phone received, stored only as `visible()` -- the projection that cannot
// carry `expected` before a close. Opening on a second phone shows
// "not known here" until that phone does something to the drawer.
import { TILL_CURRENCIES, parseMinor } from './logic.js';
import { ui, k, backBar } from './parts.js';
import { renderTill, visible, tipsQuery, renderTips } from './till-view.js';
import { safeGet, safeSet } from '../store/storage.js';

const KEY = 'dw_room_till';

export function lastTill(loc) {
  try { const v = JSON.parse(safeGet(KEY) || 'null'); return v && v.loc === loc ? v.ans : null; } catch { return null; }
}
export function keep(loc, ans) { safeSet(KEY, JSON.stringify({ loc, ans: visible(ans) })); }

const pileInputs = (name, tour) => TILL_CURRENCIES.map(c => ui.field({ id: `till-${name}-${c}`, name: `${name}_${c}`, label: c,
  inputmode: 'decimal', autocomplete: 'off', placeholder: '0', controlCls: 'money', attrs: { data: { tour } } })).join('');

/// A titled till form: heading, hint, fields, one submit.
const tillForm = (kind, tour, title, hint, body, submit) => `<form class="card-form" data-form="${kind}" data-tour="${tour}" novalidate>
  <h3>${ui.esc(title)}</h3>${hint ? ui.para(hint, { hint: true }) : ''}${body}${submit}</form>`;

/// The cash move's two choices: which way, which currency. Radio groups on
/// /lib/ui; the choice lives in `S.tillMove` (bindTill), not in a form input.
export function moveChoice(S) {
  if (!S.tillMove) S.tillMove = { dir: 'pay_in', currency: TILL_CURRENCIES[0] };
  return S.tillMove;
}

export function renderTillScreen(c) {
  const { S, t } = c;
  const ans = S.till;
  const open = ans && ans.open;
  const mv = moveChoice(S);
  return `
    ${backBar()}
    <h2>${ui.esc(t('till'))}</h2>
    <section class="till-state" aria-live="polite" data-tour="till.state">${renderTill(ans, t, c.locale())}</section>
    ${tipsQuery(ans, S.loc) ? `<section class="card-form till-tips" data-tips data-tour="till.tips" aria-live="polite"><h3>${ui.esc(t('tipsTitle'))}</h3>${ui.skeleton({ shape: 'line', count: 2, label: t('loading') })}</section>` : ''}
    ${open ? '' : tillForm('open', 'till.open', t('openTill'), k('floatHint'), `<div class="piles">${pileInputs('f', 'till.float')}</div>`,
      ui.button({ type: 'submit', variant: 'primary', size: 'lg', block: true, icon: 'cash', label: k('openTill') }))}
    ${tillForm('move', 'till.move', t('cashMove'), '', `
      ${ui.segmented({ id: 'tillDir', label: k('cashMove'), value: mv.dir, options: [
        { value: 'pay_in', label: k('payIn'), icon: 'plus' }, { value: 'pay_out', label: k('payOut'), icon: 'minus' }], attrs: { data: { tour: 'till.direction' } } })}
      <p class="ui-label">${ui.esc(t('currency'))}</p>
      ${ui.segmented({ id: 'tillCur', label: k('currency'), value: mv.currency, options: TILL_CURRENCIES.map(x => ({ value: x, label: x })), attrs: { data: { tour: 'till.currency' } } })}
      ${ui.field({ id: 'till-amount', name: 'amount', label: k('amount'), inputmode: 'decimal', autocomplete: 'off', required: true, controlCls: 'money', attrs: { data: { tour: 'till.amount' } } })}
      ${ui.field({ id: 'till-reason', name: 'reason', label: k('reasonText'), maxlength: 140, required: true, autocomplete: 'off', attrs: { data: { tour: 'till.reason' } } })}`,
      ui.button({ type: 'submit', icon: 'send', label: k('send') }))}
    ${tillForm('count', 'till.count', t('count'), k('countHint'), `<div class="piles">${pileInputs('o', 'till.counted')}</div>`,
      ui.button({ type: 'submit', icon: 'check', label: k('saveCount') }))}
    ${tillForm('close', 'till.close', t('closeTill'), k('closeHint'),
      `<label class="check"><input type="checkbox" name="sure" required data-tour="till.closeSure"> ${ui.esc(t('closeSure'))}</label>`,
      ui.button({ type: 'submit', variant: 'danger', block: true, icon: 'power', label: k('closeTill') }))}`;
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
  const mv = moveChoice(S);
  ui.bindSegmented(root.querySelector('#tillDir'), v => { mv.dir = v; });
  ui.bindSegmented(root.querySelector('#tillCur'), v => { mv.currency = v; });
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
      const currency = mv.currency, amount = parseMinor(get('amount'), currency), reason = String(get('reason') || '').trim();
      if (!amount) return c.toast(t('badAmount'));
      if (!reason) return c.toast(t('needReason'));
      path = mv.dir === 'pay_out' ? 'pay_out' : 'pay_in';
      Object.assign(body, { currency, amount, reason });
    } else if (kind === 'close') {
      if (!f.elements.sure.checked) return;
      path = 'close';
    } else return;
    const btn = f.querySelector('button[type="submit"]');
    ui.setBusy(btn, t('loading'));
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
  catch (e) { el.innerHTML = `<h3>${ui.esc(t('tipsTitle'))}</h3>${ui.emptyState({ icon: 'plug-connected-x', title: k('tipsFailed'), reason: e.message || '', alert: true })}`; }
}
