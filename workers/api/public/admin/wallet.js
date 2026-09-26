// A CUSTOMER'S WALLET, the venue's side (W-WIRE row 2).
//
// WHO MAY TOP UP: THE VENUE, AND ONLY THE VENUE (`wallet.rs::top_up`, the
// red-team reason): a top-up writes the ledger this system treats as money, so
// until a payment provider's webhook signs one, only the person who took the
// cash or the card slip at the counter may record it -- with that payment's
// reference. The kit no longer offers the customer a door that answers 403
// (`kit/screens/wallet.js`): it says where to pay.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { $, $$, esc, icon, t, api, post, busy, confirm, moneyEl, baseCurrency } from '/admin/core.js';
import { btn, field, rowBtn, rowDiv, empty, loading } from '/admin/parts.js';
import { decimalsOf } from '/lib/money.js';
import { q, venuePath, fail, open, paint } from '/admin/wire-core.js';
import { parseMinor, requestId } from '/admin/wire-logic.js';

const who = c => c.name || c.phone || c.key;

/// The customers, to pick whose wallet: a search and a list.
export async function mount(host){
  if (!host) return;
  host.innerHTML = loading(3);
  let d;
  try { d = await api('/owner/customers' + q()); } catch (e) { host.innerHTML = empty('alert-triangle', { title: String(e.message || e), alert: true }); return; }
  const all = d.customers || [];
  host.innerHTML = `${field({ id: 'w-find', key: 'w_findCustomer', type: 'search', autocomplete: 'off' })}<div class="rows" role="list" id="wList"></div>`;
  const draw = () => {
    const s = $('#w-find').value.trim().toLowerCase();
    const shown = all.filter(c => !s || String(who(c)).toLowerCase().includes(s)).slice(0, 50);
    $('#wList').innerHTML = shown.map((c, i) => rowBtn({ data: { i: all.indexOf(c) }, leading: icon('user'), title: String(who(c)), trailing: icon('chevron-right') })).join('')
      || empty('user', { key: 'w_noCustomers' });
    for (const b of $$('[data-i]', $('#wList'))) b.onclick = () => openWallet(all[Number(b.dataset.i)]);
  };
  $('#w-find').oninput = draw;
  draw();
  paint(host);
}

/// One customer's wallet: the balance, the statement, and the top-up.
export async function openWallet(c){
  const host = open('w_wallet', 'w_walletHint', 'wallet');
  host.innerHTML = loading(2);
  const user = encodeURIComponent(c.key);
  let bal, st;
  try {
    [bal, st] = await Promise.all([api(venuePath(`/wallet?user=${user}`)), api(venuePath(`/wallet/statement?user=${user}`))]);
  } catch (e) { return fail(e); }
  const cur = bal.currency || baseCurrency();
  const lines = (st.statement || []).map(r => rowDiv({ leading: icon(r.minor >= 0 ? 'plus' : 'minus'), title: r.memo || r.kind, sub: esc(r.kind), trailing: moneyEl(r.minor) }));
  host.innerHTML = `<p class="eyebrow">${esc(who(c))}</p>
    ${rowDiv({ leading: icon('wallet'), title: { t: 'w_balance' }, sub: bal.balanceMinor == null ? esc(t('w_neverUsed')) : '', trailing: moneyEl(bal.balanceMinor || 0) })}
    ${bal.conserved === false ? `<p class="muted small" data-t="w_notConserved"></p>` : ''}
    <p class="eyebrow mt-3" data-t="w_statement"></p>
    <div class="rows" role="list">${lines.join('') || empty('receipt', { key: 'w_noMoves' })}</div>
    <p class="eyebrow mt-3" data-t="w_topUp"></p><p class="muted small" data-t="w_topUpHint"></p>
    <div class="grid2">${field({ id: 'w-amt', key: 'w_amount', inputmode: 'decimal', autocomplete: 'off', hint: cur })}
      ${field({ id: 'w-ref', key: 'w_payRef', autocomplete: 'off', maxlength: 80, hintKey: 'w_payRefHint' })}</div>
    <div class="btn-row">${btn({ id: 'wBack', variant: 'ghost', icon: 'arrow-left', key: 'w_allCustomers' })}${btn({ id: 'wTop', variant: 'primary', icon: 'coin', key: 'w_record' })}</div>`;
  paint(host);
  $('#wBack').onclick = openWallets;
  $('#wTop').onclick = async () => {
    const minor = parseMinor($('#w-amt').value, decimalsOf(cur)), ref = $('#w-ref').value.trim();
    if (!minor) return fail(t('w_badAmount'));
    if (!ref) return fail(t('w_needRef'));
    const ok = await confirm(t('w_topUp'), `${who(c)}: +${$('#w-amt').value.trim()} ${cur}`);
    if (!ok) return openWallet(c);
    try {
      await busy(null, () => post(venuePath('/wallet/topup'), { user: c.key, amountMinor: minor, currency: cur, providerRef: ref, requestId: requestId('tu', Date.now(), c.key) }));
      openWallet(c);
    } catch (e) { fail(e); openWallet(c); }
  };
}

/// The screen as a sheet (the More tile).
export const openWallets = () => mount(open('w_wallets', 'w_walletsHint', 'wallets'));
