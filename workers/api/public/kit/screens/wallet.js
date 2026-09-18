// My Wallet 1:9757 and Add Money 1:9857 (light 1:20201, 1:20311).
//
// A 327x128 r8 balance card, then either the transaction list or a grid of
// preset amounts with a field under it.
//
// THE BALANCE IS A REPLAY, NOT A NUMBER. `dowiz_kernel::ledger_account` is an
// account-scoped double-entry journal in which every transaction sums to exactly
// zero, and a wallet's balance is the sum of its postings — there is no balance
// column anywhere and there must never be one.
//
// What is still missing is the PAYMENT RAIL. A top-up needs a `providerRef`
// from whatever actually took the money; without one the server refuses, because
// a posting with nothing behind it would be money this system invented. The
// screen says exactly that instead of pretending the button worked.

import { icon, esc } from '/kit/app.js';
import { topBar, ctaBar } from '/kit/parts.js';
import { wallet as api, requestId, formatMoney } from '/kit/data.js';

const BALANCE = '$ 2400.00';
const PRESETS = [100, 200, 500, 1000, 2000, 3000, 4000, 5000];

const TX = [
  { icon: 'scooter-20', label: 'Order #FD785462', when: 'April 12 · 07:30 PM', amount: '-$48.00' },
  { icon: 'wallet',     label: 'Top Up',          when: 'April 11 · 02:15 PM', amount: '+$500.00', in: true },
  { icon: 'ticket',     label: 'Coupon refund',   when: 'April 09 · 06:02 PM', amount: '+$8.00', in: true },
  { icon: 'scooter-20', label: 'Order #FD568974', when: 'April 08 · 08:44 PM', amount: '-$16.00' },
];

const state = { preset: null, amount: '' };

const balanceCard = (withButton, shown, conserved) => `
  <div class="k-balance">
    <span><span class="k-balance-k">Wallet Balance</span>
      <span class="k-balance-v">${esc(shown)}</span>
      ${conserved === false ? `<span class="k-in-err">Журнал не сходиться до нуля —
        показане число не можна захистити.</span>` : ''}</span>
    ${withButton ? `<button class="k-balance-btn" type="button" data-go="add-money"
        aria-label="Поповнити">${icon('wallet')}</button>` : ''}
  </div>`;

// The signed-in customer. Until the kit carries a session this is the venue's
// demo account, and the screen says whose balance it is showing.
const USER = 'demo-customer';

export async function render(params, routeName = 'my-wallet'){
  const live = await api.balance(USER);
  const shown = live && !live.error && live.balanceMinor != null
    ? formatMoney(live.balanceMinor, live.currency)
    : (live && !live.error && live.balanceMinor == null ? 'ще не використовувався' : BALANCE);
  const conserved = live && !live.error ? live.conserved : null;
  if (routeName === 'add-money'){
    return `
    ${topBar('Add Money')}
    <div class="wrap k-page">
      ${balanceCard(false, shown, conserved)}
      <div class="k-block">
        <div class="k-amounts" role="group" aria-label="Сума">
          ${PRESETS.map(n => `
            <button class="k-amount" type="button" data-preset="${n}"
                    aria-pressed="${n === state.preset}">+ $${n}</button>`).join('')}
        </div>
      </div>
      <div class="k-in k-block">
        <label for="amt">Enter Amount</label>
        <span class="k-prefix"><span>$</span>
          <input id="amt" type="text" inputmode="decimal" placeholder="Enter Amount"
                 value="${esc(state.amount)}"></span>
        <span class="k-in-err" id="amtErr" hidden></span>
      </div>
      <p class="k-page-p" id="said" role="status"></p>
    </div>
    ${ctaBar('Add Money')}`;
  }

  return `
  ${topBar('My Wallet')}
  <div class="wrap k-page">
    ${balanceCard(true, shown, conserved)}
    <div class="k-block">
      <h2 class="k-block-h">Transactions</h2>
      <div class="k-box">
        ${TX.map(t => `
          <div class="k-tx">
            <span class="k-row-ic">${icon(t.icon)}</span>
            <span class="k-choice-body">
              <span class="k-choice-t">${esc(t.label)}</span>
              <span class="k-choice-s">${esc(t.when)}</span>
            </span>
            <span class="k-tx-amt${t.in ? ' is-in' : ''}">${esc(t.amount)}</span>
          </div>`).join('')}
      </div>
    </div>
  </div>
  ${ctaBar('Add Money', { to: 'add-money' })}`;
}

export function bind(root){
  root.addEventListener('click', e => {
    const p = e.target.closest('[data-preset]');
    if (p){
      state.preset = Number(p.dataset.preset);
      state.amount = String(state.preset);
      root.querySelector('#amt').value = state.amount;
      for (const b of root.querySelectorAll('[data-preset]'))
        b.setAttribute('aria-pressed', String(Number(b.dataset.preset) === state.preset));
      return;
    }

    const cta = e.target.closest('[data-cta]');
    if (!cta) return;
    const input = root.querySelector('#amt');
    if (!input) return;
    const err = root.querySelector('#amtErr');
    const value = input.value.trim();
    // Money is integer minor units everywhere in dowiz; a comma, a stray sign
    // or three decimal places is a refusal, not a rounding.
    const ok = /^\d+([.,]\d{1,2})?$/.test(value) && Number(value.replace(',', '.')) > 0;
    err.textContent = ok ? '' : 'Введіть суму, наприклад 250 або 250.00';
    err.hidden = ok;
    if (!ok) return;
    topUp(root, value);
  });

  root.addEventListener('input', e => {
    if (e.target.id !== 'amt') return;
    state.amount = e.target.value;
    state.preset = null;
    for (const b of root.querySelectorAll('[data-preset]')) b.setAttribute('aria-pressed', 'false');
  });
}

async function topUp(root, value){
  const said = root.querySelector('#said');
  const minor = Math.round(Number(value.replace(',', '.')) * 100);
  said.textContent = 'Надсилаємо…';

  const answer = await api.topUp({
    user: USER,
    amountMinor: minor,
    currency: 'ALL',
    // Deliberately empty: there is no payment rail wired yet, and the server
    // refuses a top-up with nothing behind it. The refusal IS the feature —
    // a posting without a payment would be money this system invented.
    providerRef: '',
    requestId: requestId(),
  });

  if (!answer){
    said.textContent = 'Хост не називає заклад, тож поповнювати нічого.';
    return;
  }
  said.textContent = answer.error
    ? answer.error
    : `Поповнено на ${formatMoney(minor, 'ALL')} (${answer.id}).`;
}
