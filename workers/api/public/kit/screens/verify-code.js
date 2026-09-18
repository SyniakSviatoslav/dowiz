// Verify Code — Figma node 1:2458 (dark) / 1:12867 (light).
//
// Title 24/500 lh29, a 12/400 line naming where the code went, then four
// 57x41 r12 boxes at 12px apart, and a resend line under them.

import { icon, esc, go } from '/kit/app.js';
import { topBar } from '/kit/parts.js';

const SENT_TO = 'example@email.com';
const LENGTH = 4;
const RESEND_SECONDS = 30;

export function render(){
  return `
  ${topBar('Verify Code')}
  <div class="wrap k-ask-copy">
    <h1 class="k-onb-h">Verify Code</h1>
    <p class="k-onb-p">Enter the verification code we sent to ${esc(SENT_TO)}</p>

    <div class="k-otp-row" role="group" aria-label="Код підтвердження">
      ${Array.from({ length: LENGTH }, (_, i) => `
        <input id="d${i}" type="text" inputmode="numeric" maxlength="1" autocomplete="one-time-code"
               aria-label="Цифра ${i + 1}">`).join('')}
    </div>

    <p class="k-resend">Не отримали код?
      <button type="button" id="resend" disabled>Надіслати ще раз (<span id="left">${
        RESEND_SECONDS}</span>)</button></p>

    <button class="k-submit" id="verify" type="button" disabled>Verify</button>
  </div>`;
}

export function bind(root){
  const boxes = [...root.querySelectorAll('.k-otp-row input')];
  const verify = root.querySelector('#verify');
  const resend = root.querySelector('#resend');
  const left = root.querySelector('#left');

  const code = () => boxes.map(b => b.value).join('');
  const sync = () => { verify.disabled = code().length !== LENGTH; };

  // One box at a time, and typing past the last one does not wrap round to the
  // first -- which is what makes a four-box code feel like one field.
  boxes.forEach((box, i) => {
    box.addEventListener('input', () => {
      box.value = box.value.replace(/\D/g, '').slice(0, 1);
      if (box.value && i < boxes.length - 1) boxes[i + 1].focus();
      sync();
    });
    box.addEventListener('keydown', e => {
      if (e.key === 'Backspace' && !box.value && i > 0){ boxes[i - 1].focus(); e.preventDefault(); }
      if (e.key === 'ArrowLeft' && i > 0){ boxes[i - 1].focus(); e.preventDefault(); }
      if (e.key === 'ArrowRight' && i < boxes.length - 1){ boxes[i + 1].focus(); e.preventDefault(); }
    });
    // A pasted code fills the row rather than landing entirely in one box.
    box.addEventListener('paste', e => {
      const text = (e.clipboardData?.getData('text') || '').replace(/\D/g, '');
      if (!text) return;
      e.preventDefault();
      boxes.forEach((b, n) => { b.value = text[n] || ''; });
      boxes[Math.min(text.length, boxes.length - 1)].focus();
      sync();
    });
  });

  verify.addEventListener('click', () => go('new-password'));

  // The countdown is real: a resend button that is always live invites people
  // to hammer it, and one that never comes back strands them.
  let n = RESEND_SECONDS;
  const tick = setInterval(() => {
    n -= 1;
    if (n > 0){ left.textContent = String(n); return; }
    clearInterval(tick);
    resend.disabled = false;
    resend.textContent = 'Надіслати ще раз';
  }, 1000);

  resend.addEventListener('click', () => {
    resend.disabled = true;
    n = RESEND_SECONDS;
    resend.textContent = `Надіслати ще раз (${n})`;
  });
}
