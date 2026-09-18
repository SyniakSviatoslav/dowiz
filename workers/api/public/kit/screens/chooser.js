// Choosers — Delivery Address 1:4389, Payment Methods 1:4589, Order Type
// 1:4459, Delivery Type 1:4516, PickUp Time 1:3655, Manage Address 1:9629.
// Light set: 1:14789, 1:14992, 1:14860, 1:14918, 1:14056, 1:20070.
//
// Six frames of "pick one of these, then continue". They differ in whether
// the options are grouped under headings and whether a row carries a ring icon
// or a bare glyph, so those are properties of the chooser rather than two
// modules that will disagree about what a selected row looks like.
//
// A radio group is a radio group: `role="radiogroup"` with `aria-checked` on
// the rows, so the arrow keys work and a screen reader says which is chosen.

import { icon, esc, go } from '/kit/app.js';
import { topBar, ctaBar } from '/kit/parts.js';
import * as Me from '/kit/me.js';

const CHOOSERS = {
  'delivery-address': {
    title: 'Delivery Address',
    cta: 'Continue',
    ring: true,
    addNew: { label: 'Add New Shipping Address', to: 'add-address' },
    groups: [{
      options: [
        { id: 'home',   icon: 'pin-24', title: 'Home',
          sub: '245 Madison Ave, New York, NY 10016, USA' },
        { id: 'office', icon: 'pin-24', title: 'Office',
          sub: '780 Broadway, New York, NY 10003, USA' },
        { id: 'parent', icon: 'pin-24', title: 'Parent’s House',
          sub: '210 E 34th St, New York, NY 10016, USA' },
        { id: 'friend', icon: 'pin-24', title: 'Friend’s House',
          sub: '400 W 42nd St, New York, NY 10036, USA' },
      ],
    }],
    chosen: 'home',
  },

  'order-type': {
    title: 'Order Type',
    cta: 'Continue',
    ring: true,
    groups: [{ options: [
      { id: 'pickup',   icon: 'box',     title: 'Pickup',
        sub: '15–20 mins · Collect from restaurant' },
      { id: 'delivery', icon: 'scooter', title: 'Delivery',
        sub: '30–45 mins · To your address' },
    ] }],
    chosen: 'delivery',
    then: 'cart',
  },

  'delivery-type': {
    title: 'Delivery Type',
    cta: 'Continue',
    ring: true,
    groups: [{ options: [
      { id: 'express',  icon: 'scooter', title: 'Express Delivery',
        sub: 'Delivery in 15–25 mins', right: '$18' },
      { id: 'standard', icon: 'scooter', title: 'Standard Delivery',
        sub: 'Delivery in 30–45 mins', right: '$12' },
      { id: 'eco',      icon: 'scooter', title: 'Eco Saver Delivery',
        sub: 'Delivery in 45–60+ mins', right: '$06' },
    ] }],
    chosen: 'standard',
    then: 'cart',
  },

  'pickup-time': {
    title: 'PickUp Time',
    cta: 'Continue',
    ring: true,
    groups: [{ head: 'Choose Pickup Time', options: [
      { id: 'asap', icon: 'clock-bold', title: 'As soon as possible',
        sub: 'Ready in 20–30 mins' },
      { id: 't1930', icon: 'clock-bold', title: '12 April 2026 07:30 PM', sub: 'Today' },
      { id: 't2000', icon: 'clock-bold', title: '12 April 2026 08:00 PM', sub: 'Today' },
      { id: 't2030', icon: 'clock-bold', title: '12 April 2026 08:30 PM', sub: 'Today' },
    ] }],
    chosen: 'asap',
    then: 'cart?mode=pickup',
  },

  'manage-address': {
    title: 'Manage Address',
    cta: 'Done',
    ring: true,
    addNew: { label: 'Add New Shipping Address', to: 'add-address' },
    groups: [{ options: [
      { id: 'home',   icon: 'pin-24', title: 'Home',
        sub: '245 Madison Ave, New York, NY 10016, USA' },
      { id: 'office', icon: 'pin-24', title: 'Office',
        sub: '780 Broadway, New York, NY 10003, USA' },
      { id: 'parent', icon: 'pin-24', title: 'Parent’s House',
        sub: '210 E 34th St, New York, NY 10016, USA' },
      { id: 'friend', icon: 'pin-24', title: 'Friend’s House',
        sub: '400 W 42nd St, New York, NY 10036, USA' },
    ] }],
    chosen: 'home',
    then: 'profile',
  },

  'payment-methods': {
    title: 'Payment Methods',
    cta: 'Confirm Payment',
    flat: true,
    groups: [
      { head: 'Cash',   options: [{ id: 'cash',   icon: 'money',  title: 'Cash' }] },
      { head: 'Wallet', options: [{ id: 'wallet', icon: 'wallet', title: 'Wallet' }] },
      { head: 'Credit & Debit Card',
        options: [{ id: 'card', icon: 'card', title: 'Add Card', to: 'add-card' }] },
      { head: 'More Payment Options', options: [
        { id: 'paypal', icon: 'brand-paypal', title: 'Paypal' },
        { id: 'apple',  img: '/kit/icon/brand-apple.png', title: 'Apple Pay' },
        { id: 'gpay',   icon: 'brand-gpay',  title: 'Google Pay' },
      ] },
    ],
    chosen: 'wallet',
  },
};

// The chosen option per chooser, so going back to one remembers the answer.
const picked = new Map();

const glyph = o => o.img
  ? `<img src="${esc(o.img)}" alt="" width="23" height="24">`
  : icon(o.icon);

const optionRow = (c, o, chosen) => {
  // A row with a destination is a link, not a choice: Payment Methods' "Add
  // Card" opens a form and cannot be the selected payment method.
  const isLink = !!o.to;
  return `
  <button class="k-opt" type="button" data-opt="${esc(o.id)}"
          ${isLink ? `data-go="${esc(o.to)}"` : `role="radio" aria-checked="${o.id === chosen}"`}>
    <span class="${c.ring ? 'k-opt-ring' : 'k-opt-ic'}">${glyph(o)}</span>
    <span class="k-opt-body">
      <span class="k-opt-t">${esc(o.title)}</span>
      ${o.sub ? `<span class="k-opt-s">${esc(o.sub)}</span>` : ''}
    </span>
    ${o.right ? `<span class="k-opt-right">${esc(o.right)}</span>` : ''}
    ${isLink
      ? `<span class="k-opt-chev">${icon('arrow-down')}</span>`
      : `<span class="k-opt-radio">${icon(o.id === chosen ? 'radio-on' : 'radio-off')}</span>`}
  </button>`;
};

/// THE CUSTOMER'S OWN ADDRESSES, above the frame's.
///
/// `Add New Shipping Address` opened a form that validated the address and then
/// dropped it, so this list could only ever be the four New York examples the
/// Figma frame was drawn with -- and a customer in Durrës had to order to
/// `245 Madison Ave`. Saved addresses now come first, and the one checkout will
/// actually send is the one shown as selected, so the screen cannot disagree
/// with the order.
function withMine(c, routeName){
  if (routeName !== 'delivery-address') return c;
  const mine = Me.addresses();
  if (!mine.length) return c;
  const options = mine.map(a => ({ id: a.id, icon: 'pin-24', title: a.label, sub: a.line }));
  const [first, ...rest] = c.groups;
  return { ...c, groups: [{ ...first, options: [...options, ...first.options] }, ...rest] };
}

/// Which row is selected when the screen opens: what was chosen LAST TIME, found
/// by the address itself rather than by a row's position, because the saved list
/// changes and position does not survive it.
function current(c, routeName){
  if (routeName !== 'delivery-address') return null;
  const now = Me.chosen();
  if (!now) return null;
  for (const g of c.groups) for (const o of g.options) if (o.sub === now.line) return o.id;
  return null;
}

export function render(params, routeName = 'delivery-address'){
  const c = withMine(CHOOSERS[routeName] || CHOOSERS['delivery-address'], routeName);
  const chosen = picked.get(routeName) ?? current(c, routeName) ?? c.chosen;
  return `
  ${topBar(c.title)}
  <div class="wrap" data-chooser="${esc(routeName)}" role="radiogroup"
       aria-label="${esc(c.title)}">
    ${c.groups.map(g => `
      <div class="k-block">
        ${g.head ? `<h2 class="k-block-h">${esc(g.head)}</h2>` : ''}
        <div class="k-opts${c.flat ? ' is-flat' : ''}">
          ${g.options.map(o => optionRow(c, o, chosen)).join('')}
        </div>
      </div>`).join('')}

    ${c.addNew ? `
      <button class="k-addnew" type="button" data-go="${esc(c.addNew.to)}">
        ${icon('add-20')}${esc(c.addNew.label)}
      </button>` : ''}
  </div>
  ${ctaBar(c.cta)}`;
}

export function bind(root){
  const host = root.querySelector('[data-chooser]');
  const routeName = host.dataset.chooser;

  const choose = row => {
    picked.set(routeName, row.dataset.opt);
    for (const b of host.querySelectorAll('[role="radio"]')){
      const on = b === row;
      b.setAttribute('aria-checked', String(on));
      b.querySelector('.k-opt-radio').innerHTML = icon(on ? 'radio-on' : 'radio-off');
    }
    remember(routeName, row);
  };

  /// AN ADDRESS THAT IS CHOSEN HAS TO BE KEPT SOMEWHERE.
  ///
  /// Choosing one used to move a radio dot and nothing else, so checkout had no
  /// address to send and the order could not be placed at all. It is written to
  /// the device -- this is the customer's own list, not the hub's -- under the
  /// key the summary screen reads.
  function remember(which, row){
    if (which !== 'delivery-address') return;
    const label = row.querySelector('.k-opt-t')?.textContent?.trim() || '';
    const line = row.querySelector('.k-opt-s')?.textContent?.trim() || '';
    if (!line) return;
    // A saved address keeps its note -- the floor and the landmark the customer
    // typed for the courier. Picking it must not quietly drop them, which is
    // what reading the row's two lines back out of the DOM would do.
    const saved = Me.addresses().find(a => a.line === line);
    Me.setChosen(saved || { label, line, note: null });
  }

  root.addEventListener('click', e => {
    const row = e.target.closest('[role="radio"]');
    if (row) choose(row);
  });

  // Arrow keys move the selection, which is what a radio group does and what a
  // row of buttons does not do on its own.
  root.addEventListener('keydown', e => {
    if (!['ArrowDown', 'ArrowUp', 'ArrowLeft', 'ArrowRight'].includes(e.key)) return;
    const rows = [...host.querySelectorAll('[role="radio"]')];
    const at = rows.indexOf(e.target.closest('[role="radio"]'));
    if (at < 0) return;
    e.preventDefault();
    const step = (e.key === 'ArrowDown' || e.key === 'ArrowRight') ? 1 : -1;
    const next = rows[(at + step + rows.length) % rows.length];
    next.focus();
    choose(next);
  });

  root.querySelector('.k-cta')?.addEventListener('click', () => {
    const c = CHOOSERS[routeName];
    go(c?.then || (routeName === 'payment-methods' ? 'payment-successful' : 'cart'));
  });
}
