// Coupon — Figma node 1:7557 (dark) / 1:17981 (light).
//
// Each coupon is a torn ticket: a 61x142 gradient stub carrying the discount
// sideways with four 16px notches punched along the seam, then a 267 body with
// the code, what it does, what is still needed to unlock it, and when it ends.
//
// The stub gradients are the file's own, read with design/spec.py 1:7557.

import { icon, esc } from '/kit/app.js';
import { topBar } from '/kit/parts.js';

const COUPONS = [
  { code: 'FOODIE30', stub: '20% FREE', off: '30% OFF',
    desc: 'Enjoy 30% OFF on All Orders',
    unlock: 'Add items worth $12 more to unlock', ends: 'Ends in 1 day',
    grad: 'linear-gradient(160deg,#EA4F16,#E85B27,#FF8558,#FF9E7A,#DE5E2E,#D03800)' },
  { code: 'SNACK25', stub: '20% FREE', off: '25% OFF',
    desc: 'Get 25% OFF on Snacks',
    unlock: 'Add items worth $22 more to unlock', ends: 'Ends in 2 day',
    grad: 'linear-gradient(160deg,#499F48,#5EB35A,#73CF6F,#59B057,#479B44)' },
  { code: 'QUICKEATS', stub: '15% FREE', off: '15% OFF',
    desc: 'Flat 15% OFF on Fast Food',
    unlock: 'Add items worth $32 more to unlock', ends: 'Ends in 4 day',
    grad: 'linear-gradient(160deg,#55489F,#695AB3,#8D6FCF,#6657B0,#53449B)' },
];

const ticket = c => `
  <button class="k-ticket" type="button" data-coupon="${esc(c.code)}">
    <span class="k-ticket-stub" data-grad="${esc(c.grad)}"><span>${esc(c.off)}</span></span>
    <span class="k-ticket-body">
      <span class="k-ticket-code">${esc(c.code)}</span>
      <span class="k-ticket-desc">${esc(c.desc)}</span>
      <span class="k-ticket-rule"></span>
      <span class="k-ticket-hint">${esc(c.unlock)}</span>
      <span class="k-ticket-foot">${esc(c.ends)}${icon('ellipse3293')}
        <span class="k-tcs">T&amp;Cs Apply</span></span>
    </span>
  </button>`;

export function render(){
  return `
  ${topBar('Coupon')}
  <div class="wrap">${COUPONS.map(ticket).join('')}</div>`;
}

export function bind(root){
  // Each stub's gradient belongs to its coupon, so it is written through CSSOM
  // rather than becoming a class per coupon -- `style-src 'self'` forbids the
  // attribute, and a class per coupon does not survive a fourth coupon.
  for (const el of root.querySelectorAll('[data-grad]')){
    el.style.background = el.dataset.grad;
    el.removeAttribute('data-grad');
  }

  root.addEventListener('click', e => {
    const t = e.target.closest('[data-coupon]');
    if (!t) return;
    // Applying a coupon is the cart's business and the server's answer; this
    // screen hands the code over and says it did.
    try { sessionStorage.setItem('dw_kit_coupon', t.dataset.coupon); } catch { /* private */ }
    location.hash = '#/cart';
  });
}
