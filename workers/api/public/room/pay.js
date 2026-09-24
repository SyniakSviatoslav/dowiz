// TAKE A PAYMENT on a round (`POST /api/staff/orders/:id/pay`, pay.rs).
//
// THE BILL IS IN THE ORDER'S CURRENCY and stays there (`command/pay/fx.rs`).
// A guest may hand over the other one; then the waiter types the rate AS THE
// BOARD BY THE TILL SAYS IT ("1 EUR = 97.50 ALL") and this screen turns it
// into `rate_ppm` exactly (`logic.ratePpm`, integers only). The preview is a
// preview; what the payment took off the bill is the SERVER's
// `amount_in_order_currency`, shown when it answers.
//
// A SPLIT is several payments: the screen stays open after one, with what is
// still owed as the next default, until the server says "paid".
import { money, METHODS, TILL_CURRENCIES, owed, parseMinor, minorToInput, ratePpm, convertPpm, maxAmountFor, quoteOf, settles, tipMinor, walletOk, walletTipOk, walletField } from './logic.js';
import { pick } from './sheet.js';
import { ui, k, backBar, pickChips, amt } from './parts.js';

/// The form's working state, per round, so a redraw keeps what was typed.
function form(c, round) {
  const S = c.S;
  if (!S.payForm || S.payForm.id !== round.id) {
    S.payForm = { id: round.id, currency: S.currency, method: 'cash', amount: minorToInput(owed(round), S.currency), rate: '', tip: '', wallet: '' };
  }
  return S.payForm;
}

/// A decimal amount field that keeps the binder's `data-in` and the form's `name`.
const num = (name, tour, label, value, o = {}) => ui.field({ id: 'pay-' + name, name, label, value, inputmode: 'decimal', autocomplete: 'off',
  controlCls: 'money', ...o, attrs: { data: { in: name, tour } } });

export function renderPay(c, round) {
  const { S, t } = c;
  // The bill's currency is the venue's, read with the menu. Without it every
  // amount below would be parsed on a guessed number of decimals.
  if (!S.currency) return `${backBar()}${ui.emptyState({ icon: 'plug-connected-x', title: k('menuFailed'), alert: true })}`;
  const loc = c.locale(), order = S.currency, f = form(c, round), due = owed(round);
  const foreign = f.currency !== order;
  const ppm = foreign ? ratePpm(f.rate, order, f.currency) : null;
  const amount = parseMinor(f.amount, f.currency);
  const preview = foreign && ppm && amount ? convertPpm(amount, ppm) : null;
  const { base, quote } = quoteOf(order, f.currency);
  const curs = [...new Set([order, ...TILL_CURRENCIES])];
  const note = S.payNote;
  const paid = round.payment_status === 'paid';
  const head = paid ? ui.alert({ tone: 'success', label: k('paidInFull') })
    : ui.stat({ label: k('owed'), value: amt(money(due, order, loc), { size: 'xl', strong: true }), emphasis: true, attrs: { data: { tour: 'pay.owed' } } });
  return `
    ${backBar()}
    <h2>${ui.esc(t('take'))} · ${ui.esc(t('table'))} ${ui.esc(c.tableOf(round) || '—')}</h2>
    ${head}
    ${note ? ui.alert({ tone: 'success', label: note }) : ''}
    ${paid ? '' : `<form data-form="pay" class="card-form" novalidate>
      <p class="ui-label">${ui.esc(t('currency'))}</p>${pickChips('currency', t('currency'), curs, f.currency, v => v, 'pay.currency')}
      <p class="ui-label">${ui.esc(t('method'))}</p>${pickChips('method', t('method'), METHODS, f.method, v => t('method_' + v), 'pay.method')}
      ${foreign ? num('rate', 'pay.rate', `${t('rate')}: 1 ${base} = … ${quote}`, f.rate, { required: true }) : ''}
      ${num('amount', 'pay.amount', `${t('amount')} (${f.currency})`, f.amount, { required: true })}
      ${num('tip', 'pay.tip', `${t('tip')} (${order})`, f.tip, { placeholder: '0' })}
      ${f.method === 'wallet' ? ui.field({ id: 'pay-wallet', name: 'wallet', label: k('walletCode'), value: f.wallet, autocomplete: 'off', required: true,
        attrs: { data: { in: 'wallet', tour: 'pay.wallet' } } }) : ''}
      ${foreign ? `<div class="preview">${ui.para(preview != null ? `≈ ${money(preview, order, loc)} ${t('offTheBill')}` : t('rateNeeded'), { hint: true })}
        ${ppm ? ui.button({ icon: 'coins', label: k('fillOwed'), attrs: { data: { act: 'fill', tour: 'pay.fill' } } }) : ''}</div>` : ''}
      ${ui.button({ type: 'submit', variant: 'primary', size: 'lg', block: true, icon: 'cash',
        label: t('takeN').replace('{a}', amount ? money(amount, f.currency, loc) : '—'), attrs: { data: { tour: 'pay.submit' } } })}
    </form>`}`;
}

export function bindPay(c, root, round) {
  const { S, t } = c;
  root.onclick = ev => {
    const b = ev.target.closest('[data-act]');
    if (!b) return;
    const f = form(c, round), act = b.dataset.act;
    if (act === 'back') { S.view = 'round'; return c.render(); }
    if (act === 'currency') {
      f.currency = b.dataset.v;
      f.amount = f.currency === S.currency ? minorToInput(owed(round), S.currency) : '';
      return c.render();
    }
    if (act === 'method') { f.method = b.dataset.v; return c.render(); }
    if (act === 'fill') {
      const ppm = ratePpm(f.rate, S.currency, f.currency);
      if (ppm) { f.amount = minorToInput(maxAmountFor(owed(round), ppm), f.currency); c.render(); }
    }
  };
  // The preview follows the fields without stealing focus from them.
  root.oninput = ev => {
    const k = ev.target.dataset.in;
    if (!k) return;
    form(c, round)[k] = ev.target.value;
    const pos = ev.target.selectionStart;
    c.render();
    const i = root.querySelector(`[data-in="${k}"]`);
    if (i) { i.focus(); try { i.setSelectionRange(pos, pos); } catch {} }
  };
  root.onsubmit = async ev => {
    ev.preventDefault();
    const f = form(c, round), order = S.currency;
    const amount = parseMinor(f.amount, f.currency);
    if (!amount || amount < 1) return c.toast(t('badAmount'));
    const tip = tipMinor(f.tip, order);
    if (tip == null) return c.toast(t('badTip'));
    if (!walletOk(f.method, f.wallet)) return c.toast(t('needWallet'));
    if (!walletTipOk(f.method, tip)) return c.toast(t('walletNoTip'));
    const body = { location_id: S.loc, amount, method: f.method };
    // D7: THE VERSION THIS SCREEN SHOWED, as an amendment quotes it. A second
    // waiter's split, or this tap re-sent under a fresh key, meets a round
    // that moved on and is refused instead of taking the same money twice.
    if (Number.isInteger(round.seq)) body.base_seq = round.seq;
    // THE TIP is in the bill's currency and raises the round's total by
    // itself (command/pay.rs `PayIn::tip`); `amount` stays the bill's share.
    if (tip > 0) body.tip = tip;
    if (f.method === 'wallet') Object.assign(body, walletField(f.wallet));
    if (f.currency !== order) {
      const ppm = ratePpm(f.rate, order, f.currency);
      if (!ppm) return c.toast(t('badRate'));
      body.currency = f.currency;
      body.rate_ppm = ppm;
    }
    const btn = root.querySelector('button[type="submit"]');
    ui.setBusy(btn, t('loading'));
    try {
      const r = await c.write(`/staff/orders/${encodeURIComponent(round.id)}/pay`, body, 'pay:' + round.id);
      if (!r.landed) { c.toast(t(r.queued ? 'queuedSaved' : r.reason === 'full' ? 'queueFull' : 'queueNoStore')); S.view = 'round'; return c.render(); }
      S.payNote = paidNote(c, r.data, f.currency, amount);
      if (r.data?.order) Object.assign(round, pick(r.data.order), { seq: r.data.seq });
      S.payForm = null;
      c.reload();
    } catch (e) {
      c.toast(e.message || t('error'));
    }
    c.render();
  };
}

/// "€20.00 taken: 1 950 L off the bill." -- the second figure is the
/// server's, read off the payment it recorded. PLAIN TEXT: the screen draws it
/// through `ui.alert`, which escapes it once.
export function paidNote(c, data, cur, amount) {
  const { t } = c, loc = c.locale();
  const ps = data?.order?.payments;
  const last = Array.isArray(ps) && ps.length ? ps[ps.length - 1] : null;
  const off = last ? settles(last) : null;
  const tail = data?.order?.payment_status === 'paid' ? ` ${t('paidInFull')}` : '';
  return `${t('taken')}: ${money(amount, cur, loc)}${off != null ? ` → ${money(off, c.S.currency, loc)} ${t('offTheBill')}` : ''}.${tail}`;
}
