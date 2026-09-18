// Invite Friends — Figma node 1:10323 (dark) / 1:20796 (light).
//
// A list of people with a 48px portrait, a 16/500 name, a 14/400 number and an
// outlined "Invite" that fills once tapped.
//
// INVITING IS THE PHONE'S JOB, NOT A DOWIZ DOMAIN. dowiz has no referral ledger
// and no way to send anyone a message on the customer's behalf, so the tap hands
// the invitation to the surface that CAN send it: the system share sheet, or the
// clipboard where there is none. The app says which of the two happened.
//
// The button then goes `disabled`, not `aria-pressed="true"`. `aria-pressed`
// promises a state that can be pressed BACK, and this one cannot: it announced
// a toggle to every screen reader and left the control tappable but dead.

import { esc } from '/kit/app.js';
import { topBar, plate } from '/kit/parts.js';

const PEOPLE = [
  { name: 'Elysia Snow',   phone: '(212) 555-0147' },
  { name: 'Elina Shaw',    phone: '(310) 555-0265' },
  { name: 'Arlette Hayes', phone: '(202) 555-0129' },
  { name: 'Skye Donovan',  phone: '(718) 555-0246' },
  { name: 'Marlowe Dean',  phone: '(617) 555-0152' },
  { name: 'Rowan Mercer',  phone: '(415) 555-0188' },
];

export function render(){
  return `
  ${topBar('Invite Friends')}
  <div class="wrap k-page">
    ${PEOPLE.map(p => `
      <div class="k-person">
        <span class="k-person-img">${plate(p.name)}</span>
        <span class="k-person-body">
          <span class="k-person-n">${esc(p.name)}</span>
          <span class="k-person-p">${esc(p.phone)}</span>
        </span>
        <button class="k-invite" type="button" data-invite="${esc(p.name)}"
                data-share>Invite</button>
      </div>`).join('')}
  </div>`;
}

export function bind(root){
  root.addEventListener('click', e => {
    const b = e.target.closest('[data-invite]');
    if (!b || b.disabled) return;
    // The share sheet itself is opened by the router's `data-share` handler,
    // which this event reaches next. All this screen owns is the record that
    // the invitation was handed over.
    b.textContent = 'Invited';
    b.disabled = true;
  });
}
