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
import { esc, money, METHODS, TILL_CURRENCIES, owed, parseMinor, minorToInput, ratePpm, convertPpm, maxAmountFor, quoteOf, settles } from './logic.js';
import { pick } from './sheet.js';

/// The form's working state, per round, so a redraw keeps what was typed.
function form(c, round) {
  const S = c.S;
  if (!S.payForm || S.payForm.id !== round.id) {
    S.payForm = { id: round.id, currency: S.currency, method: 'cash', amount: minorToInput(owed(round), S.currency), rate: '' };
  }
  return S.payForm;
}

export function renderPay(c, round) {
  const { S, t } = c;
  // The bill's currency is the venue's, read with the menu. Without it every
  // amount below would be parsed on a guessed number of decimals.
  if (!S.currency) return `<div class="bar"><button class="btn" data-act="back">← ${esc(t('back'))}</button></div><p class="muted">${esc(t('menuFailed'))}</p>`;
  const loc = c.locale(), order = S.currency, f = form(c, round), due = owed(round);
  const foreign = f.currency !== order;
  const ppm = foreign ? ratePpm(f.rate, order, f.currency) : null;
  const amt = parseMinor(f.amount, f.currency);
  const preview = foreign && ppm && amt ? convertPpm(amt, ppm) : null;
  const { base, quote } = quoteOf(order, f.currency);
  const curs = [...new Set([order, ...TILL_CURRENCIES])];
  const seg = (name, vals, cur, word) => `<div class="seg" role="radiogroup" aria-label="${esc(t(name))}">${vals.map(v =>
    `<button type="button" class="seg-b${v === cur ? ' on' : ''}" role="radio" aria-checked="${v === cur}" data-act="${name}" data-v="${v}">${esc(word(v))}</button>`).join('')}</div>`;
  const note = S.payNote;
  return `
    <div class="bar"><button class="btn" data-act="back">← ${esc(t('back'))}</button></div>
    <h2>${esc(t('take'))} · ${esc(t('table'))} ${esc(c.tableOf(round) || '—')}</h2>
    <p class="big">${round.payment_status === 'paid' ? esc(t('paidInFull')) : `${esc(t('owed'))}: ${money(due, order, loc)}`}</p>
    ${note ? `<p class="state ok" role="status">${note}</p>` : ''}
    ${round.payment_status === 'paid' ? '' : `<form data-form="pay" novalidate>
      <label>${esc(t('currency'))}</label>${seg('currency', curs, f.currency, v => v)}
      <label>${esc(t('method'))}</label>${seg('method', METHODS, f.method, v => t('method_' + v))}
      ${foreign ? `<label class="rate">${esc(t('rate'))}<span class="rate-row">1 ${esc(base)} =
        <input name="rate" data-in="rate" inputmode="decimal" autocomplete="off" value="${esc(f.rate)}" required> ${esc(quote)}</span></label>` : ''}
      <label>${esc(t('amount'))} (${esc(f.currency)})
        <input name="amount" data-in="amount" inputmode="decimal" autocomplete="off" value="${esc(f.amount)}" required></label>
      ${foreign ? `<p class="muted">${preview != null ? `≈ ${money(preview, order, loc)} ${esc(t('offTheBill'))}` : esc(t('rateNeeded'))}
        ${ppm ? `<button type="button" class="btn" data-act="fill">${esc(t('fillOwed'))}</button>` : ''}</p>` : ''}
      <button class="cta" type="submit">${esc(t('takeN').replace('{a}', amt ? money(amt, f.currency, loc) : '—'))}</button>
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
    const body = { location_id: S.loc, amount, method: f.method };
    if (f.currency !== order) {
      const ppm = ratePpm(f.rate, order, f.currency);
      if (!ppm) return c.toast(t('badRate'));
      body.currency = f.currency;
      body.rate_ppm = ppm;
    }
    const btn = root.querySelector('button[type="submit"]');
    if (btn) btn.disabled = true;
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
/// server's, read off the payment it recorded.
export function paidNote(c, data, cur, amount) {
  const { t } = c, loc = c.locale();
  const ps = data?.order?.payments;
  const last = Array.isArray(ps) && ps.length ? ps[ps.length - 1] : null;
  const off = last ? settles(last) : null;
  const tail = data?.order?.payment_status === 'paid' ? ` ${esc(t('paidInFull'))}` : '';
  return `${esc(t('taken'))}: ${money(amount, cur, loc)}${off != null ? ` → ${money(off, c.S.currency, loc)} ${esc(t('offTheBill'))}` : ''}.${tail}`;
}
