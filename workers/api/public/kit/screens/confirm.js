// Confirm dialogs — Remove from Cart 1:3189, Logout 1:10620, Cancel Booking
// 1:7913 (light 1:13606, 1:20894, 1:18337).
//
// Each is a sheet over the screen it acts on. The kit draws them as separate
// frames because Figma has no "over" — here the sheet IS over the screen, which
// is what makes Cancel put the customer back where they were rather than on a
// blank page.
//
// The destructive action is on the right and filled, the quiet one cancels.
// Cancel Booking asks WHY first, because a cancellation with no reason is a
// cancellation the venue cannot learn anything from.

import { icon, esc } from '/kit/app.js';
import { orderLine } from '/kit/parts.js';

const LINE = { name: 'ItaliaCrisp Pizza', kind: 'Pizza', variant: '8’ - Small', price: '$12.00' };

const REASONS = [
  'Change in Plans',
  'Prefer home delivery instead',
  'Running late / cannot reach on time',
  'Booking made by mistake',
  'Other',
];

const DIALOGS = {
  'remove-from-cart': {
    under: 'cart',
    head: 'Remove from Cart?',
    line: LINE,
    no: 'Cancel', yes: 'Yes, Remove', then: 'cart',
  },
  logout: {
    under: 'profile',
    head: 'Logout',
    body: 'Are you sure you want to log out?',
    no: 'Cancel', yes: 'Yes, Logout', then: 'signin',
  },
  'cancel-booking': {
    under: 'my-booking',
    head: 'Cancel Booking',
    reasons: true,
    no: 'Keep it', yes: 'Cancel Order', then: 'my-orders',
  },
};

const state = { reason: REASONS[0], other: '' };

export function render(params, routeName = 'remove-from-cart'){
  const d = DIALOGS[routeName] || DIALOGS['remove-from-cart'];
  return `
  <div class="k-scrim" data-back></div>
  <div class="k-sheet k-confirm" role="dialog" aria-modal="true"
       aria-label="${esc(d.head)}" data-dialog="${esc(routeName)}">
    <div class="k-sheet-grab" aria-hidden="true"></div>
    <h2>${esc(d.head)}</h2>
    ${d.body ? `<p>${esc(d.body)}</p>` : ''}
    ${d.line ? orderLine(d.line) : ''}

    ${d.reasons ? `
      <div class="k-reasons" role="radiogroup" aria-label="Причина">
        ${REASONS.map(r => `
          <button class="k-reason" type="button" role="radio" data-reason="${esc(r)}"
                  aria-checked="${r === state.reason}">
            <span>${esc(r)}</span>
            <span class="k-reason-radio">${icon(r === state.reason ? 'radio-on' : 'radio-off')}</span>
          </button>`).join('')}
      </div>
      <div class="k-in" id="otherWrap" ${state.reason === 'Other' ? '' : 'hidden'}>
        <label for="other">Other</label>
        <input id="other" type="text" placeholder="Enter your Reason"
               value="${esc(state.other)}">
        <span class="k-in-err" id="otherErr" hidden></span>
      </div>` : ''}

    <div class="k-confirm-acts">
      <button class="k-confirm-no" type="button" data-back>${esc(d.no)}</button>
      <button class="k-confirm-yes" type="button" id="yes">${esc(d.yes)}</button>
    </div>
  </div>`;
}

export function bind(root){
  const host = root.querySelector('[data-dialog]');
  const d = DIALOGS[host.dataset.dialog];

  root.addEventListener('click', e => {
    const r = e.target.closest('[data-reason]');
    if (r){
      state.reason = r.dataset.reason;
      for (const b of root.querySelectorAll('[data-reason]')){
        const on = b === r;
        b.setAttribute('aria-checked', String(on));
        b.querySelector('.k-reason-radio').innerHTML = icon(on ? 'radio-on' : 'radio-off');
      }
      const wrap = root.querySelector('#otherWrap');
      if (wrap) wrap.hidden = state.reason !== 'Other';
      return;
    }

    if (!e.target.closest('#yes')) return;
    if (d.reasons && state.reason === 'Other'){
      const input = root.querySelector('#other');
      const err = root.querySelector('#otherErr');
      state.other = input.value.trim();
      if (!state.other){
        err.textContent = 'Напишіть причину';
        err.hidden = false;
        return;
      }
    }
    location.hash = '#/' + d.then;
  });

  // Escape cancels, as it does in every dialog that is one.
  root.addEventListener('keydown', e => {
    if (e.key !== 'Escape') return;
    history.length > 1 ? history.back() : (location.hash = '#/' + d.under);
  });

  root.addEventListener('input', e => {
    if (e.target.id === 'other') state.other = e.target.value;
  });
}
