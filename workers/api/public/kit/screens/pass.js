// QR Code – Restaurant Entry 1:5637 and You Have Arrived! 1:8436
// (light 1:16048, 1:18865).
//
// A white pass card with two notches bitten out of its sides, the code above a
// divider and the reservation's facts below.
//
// THE CODE IS ISSUED, NOT DRAWN. `GET …/reservations/:id/pass` mints one with
// `dowiz_kernel::pass`: a 16-byte SHAKE256 tag over the venue, the reservation,
// the slot and the party size, checked by that venue's own scanner.
//
// A post-quantum signature is NOT used here and the reason is measured, not
// preferred: ML-DSA-65 is 3309 bytes and the largest QR holds 2953. It does not
// fit at any density. The venue is both issuer and verifier, so a keyed tag is
// the right primitive; the PQ chain protects the KEY's delivery instead.
//
// Without a reservation id in the route the screen falls back to the design's
// sample image and says so — a code that cannot be scanned must never look like
// one that can.

import { icon, esc } from '/kit/app.js';
import { topBar, ctaBar } from '/kit/parts.js';
import { reservations } from '/kit/data.js';

const PASS = {
  id: 'RSV8475',
  name: 'Jennifer Aaker',
  date: 'April 15, 2026',
  time: '07:30 PM',
};

export function render(params, routeName = 'qr-entry'){
  if (routeName === 'arrived'){
    return `
    <div class="k-done">
      <span class="k-done-badge" aria-hidden="true">${icon('verify-badge')}</span>
      <h1>You Have Arrived!</h1>
      <p>Please scan your Restaurant E-Ticket QR Code on the scanner machine.</p>
    </div>
    ${ctaBar('Scan QR', { to: 'qr-entry' })}`;
  }

  return `
  ${topBar('QR Code')}
  <div class="wrap k-page" data-pass>
    <div class="k-pass">
      <div class="k-pass-code">
        <img src="/kit/img/qr-sample.png" width="153" height="153" id="qrArt"
             alt="Зразок QR-коду з макета">
        <p class="k-pass-code-text" id="code" hidden></p>
        <p class="k-pass-note" id="passNote">Зразок із макета. Справжній код видає
          хаб закладу — цей не відскануєш.</p>
      </div>
      <div class="k-pass-rule"></div>
      <div class="k-pass-facts">
        <span><span class="k-pass-k">Reservation ID</span>
          <span class="k-pass-v">${esc(PASS.id)}</span></span>
        <span><span class="k-pass-k">Name</span>
          <span class="k-pass-v">${esc(PASS.name)}</span></span>
        <span><span class="k-pass-k">Date</span>
          <span class="k-pass-v">${esc(PASS.date)}</span></span>
        <span><span class="k-pass-k">Time</span>
          <span class="k-pass-v">${esc(PASS.time)}</span></span>
      </div>
    </div>
  </div>
  ${ctaBar('Navigate to Restaurant', { to: 'get-direction' })}`;
}

export async function bind(root, params){
  const id = params?.get('id');
  if (!id) return;                       // no booking named: the sample stands

  const note = root.querySelector('#passNote');
  const out = root.querySelector('#code');
  note.textContent = 'Отримуємо код…';

  const answer = await reservations.pass(id);
  if (!answer){
    note.textContent = 'Хост не називає заклад, тож код видавати нікому.';
    return;
  }
  if (answer.error){
    // The server says why — "a pass is issued for a confirmed booking; this one
    // is REQUESTED" is an answer somebody can act on.
    note.textContent = answer.error;
    return;
  }

  // The code is shown as text as well as encoded: a scanner reads the code, and
  // a person at a counter can read it out.
  root.querySelector('#qrArt').hidden = true;
  out.hidden = false;
  out.textContent = answer.code.replace(/(.{8})/g, '$1 ').trim();
  note.textContent = 'Код видано хабом закладу. Дійсний біля часу броні.';
}
