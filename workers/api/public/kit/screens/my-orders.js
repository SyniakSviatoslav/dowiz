// My Orders — Figma nodes 1:6468 (Active), 1:6599 (Completed), 1:6730
// (Cancelled); light set 1:16882, 1:17014, 1:17146.
//
// Three frames, one card. What changes between them is the status pill and the
// pair of actions under it, so those are a property of the ORDER's state rather
// than of the screen -- which is also what stops a cancelled order from
// offering "Track Order" the moment the list is filtered differently.
//
// The states are the kernel's (kernel/src/order_machine.rs), not this screen's:
// it groups them for display and never decides a transition.

import { icon, esc } from '/kit/app.js';
import { topBar, plate, paintPlates, navbar } from '/kit/parts.js';
import { formatMoney } from '/kit/data.js';
import * as Orders from '/kit/orders.js';

const TABS = [
  { id: 'active',    name: 'Active' },
  { id: 'completed', name: 'Completed' },
  { id: 'cancelled', name: 'Cancelled' },
];

// How a state is shown, and what can be done from it. A state missing from here
// is a state the screen would render blank, so the lookup throws rather than
// quietly dropping the row.
const STATES = {
  active: { label: 'Active Order', tone: 'is-active',
    actions: [{ label: 'Cancel', to: 'cancel-booking' },
              { label: 'Track Order', to: 'track-order', primary: true }] },
  completed: { label: 'Completed', tone: 'is-done',
    actions: [{ label: 'Leave Review', to: 'leave-review' },
              { label: 'Order Again', to: 'restaurant-menu', primary: true }] },
  cancelled: { label: 'Cancelled', tone: 'is-cancelled',
    actions: [{ label: 'View Receipt', to: 'e-receipt' },
              { label: 'Order Again', to: 'restaurant-menu', primary: true }] },
};

const ORDERS = [
  { id: '#FD785462', state: 'active', name: 'ItaliaCrisp Pizza', kind: 'Pizza',
    variant: '8’ - Small', qty: 1, rating: 4.8, price: '$12.00' },
  { id: '#FD785462', state: 'active', name: 'Mexican Tacos', kind: 'Tacos',
    variant: '2 Tacos', qty: 1, rating: 4.7, price: '$22.00' },
  { id: '#FD568974', state: 'active', name: 'Veggie Pasta', kind: 'Pasta',
    variant: '', qty: 1, rating: 4.8, price: '$16.00' },
  { id: '#FD441209', state: 'completed', name: 'Veg Burger', kind: 'Burger',
    variant: '', qty: 2, rating: 4.9, price: '$24.00' },
  { id: '#FD338771', state: 'completed', name: 'Smoky Ribs', kind: 'Grill',
    variant: 'Half rack', qty: 1, rating: 4.6, price: '$28.00' },
  { id: '#FD220145', state: 'cancelled', name: 'Italian Pizza', kind: 'Pizza',
    variant: '12’ - Large', qty: 1, rating: 4.8, price: '$16.00' },
];

const state = { tab: 'active' };

/// THE CUSTOMER'S OWN ORDERS, when there are any.
///
/// `ORDERS` above is the Figma frame's list and it was the ONLY list: a customer
/// who had ordered twice saw somebody else's pizza, tacos and pasta under
/// "Active", and their own two orders nowhere. What this device has placed comes
/// first; the frame's rows are what a design with no orders behind it shows.
///
/// A CANCELLED order is the kernel's `CANCELLED` or `REJECTED`, and `DELIVERED`
/// is completed. Everything else is still happening, which is what "Active"
/// means to the person waiting.
const TAB_OF = o => Orders.isOver(o.status) ? 'cancelled'
                  : o.status === 'DELIVERED' ? 'completed' : 'active';

function mine(){
  return Orders.all().map(o => {
    const first = o.lines?.[0];
    const money = c => o.currency ? formatMoney(c, o.currency) : '';
    return {
      id: '#' + String(o.id).slice(0, 8).toUpperCase(),
      real: o.id,
      state: TAB_OF(o),
      // An order of three dishes is one order. The card names the first and says
      // how many more, rather than drawing one card per dish as the frame does —
      // three cards carrying the same order id is how a customer comes to believe
      // they ordered three times.
      name: first?.name || 'Замовлення',
      kind: first?.kind || '',
      variant: o.lines?.length > 1 ? `+${o.lines.length - 1} ще` : (first?.variant || ''),
      qty: (o.lines || []).reduce((n, l) => n + l.qty, 0) || 1,
      rating: null,
      price: Number.isInteger(o.total) ? money(o.total)
           : money((o.lines || []).reduce((n, l) => n + l.cents * l.qty, 0)),
    };
  });
}

const orderCard = o => {
  const s = STATES[o.state];
  if (!s) throw new Error(`unknown order state: ${o.state}`);
  const parts = [o.kind, o.variant, `Qty. : ${o.qty}`].filter(Boolean);
  return `
  <article class="k-ocard" data-order="${esc(o.id)}">
    <div class="k-ocard-head">
      <span class="k-ocard-id"><span>Order ID :</span> ${esc(o.id)}</span>
      <span class="k-status ${s.tone}">${esc(s.label)}</span>
    </div>
    <div class="k-ocard-rule"></div>
    <div class="k-ocard-body">
      <div class="k-ocard-img">${plate(o.name)}</div>
      <div class="k-ocard-info">
        <span class="k-ocard-name">${esc(o.name)}</span>
        <span class="k-line-sub">${parts.map(esc).join(icon('ellipse3293'))}</span>
        ${o.rating ? `<span class="k-rate">${icon('star2')}${o.rating}</span>` : ''}
        <span class="k-ocard-price">${esc(o.price)}</span>
      </div>
    </div>
    <div class="k-ocard-acts">
      ${s.actions.map(a => `
        <button class="k-obtn${a.primary ? ' is-primary' : ''}" type="button"
                data-go="${esc(a.to === 'track-order' && o.real
                                ? `track-order?id=${encodeURIComponent(o.real)}` : a.to)}">${esc(a.label)}</button>`).join('')}
    </div>
  </article>`;
};

const list = () => {
  const own = mine();
  const rows = (own.length ? own : ORDERS).filter(o => o.state === state.tab);
  return rows.length
    ? rows.map(orderCard).join('')
    : `<div class="k-empty">${icon('clipboard-text')}
         <p class="t-title">Порожньо</p>
         <p class="t-body muted2">Тут з’являться замовлення зі статусом
           «${esc(TABS.find(t => t.id === state.tab).name)}».</p></div>`;
};

export function render(params){
  state.tab = TABS.some(t => t.id === params?.get('tab')) ? params.get('tab') : 'active';
  return `
  ${topBar('My Orders', `<button class="k-top-btn" type="button" data-go="search"
      aria-label="Пошук">${icon('search')}</button>`)}

  <div class="wrap">
    <div class="k-tabs k-tabs-wide" role="tablist">
      ${TABS.map(t => `
        <button type="button" role="tab" data-tab="${esc(t.id)}"
                aria-selected="${t.id === state.tab}">${esc(t.name)}</button>`).join('')}
    </div>
    <div id="orders" role="tabpanel" class="k-block">${list()}</div>
  </div>

  ${navbar('')}`;
}

export function bind(root){
  // ASK THE HUB WHAT THEY ARE DOING NOW.
  //
  // The list is drawn from what this device knows, which is the status each
  // order had when it was last seen — an order confirmed by the kitchen an hour
  // ago would still read "Awaiting confirmation" forever. The screen opens on
  // what is known and corrects itself when the answers arrive, so it is never
  // blank and never stale for long. A failed refresh keeps the last known
  // status: `orders.js` returns the stored entry rather than throwing.
  const known = Orders.all();
  if (known.length){
    Promise.all(known.map(o => Orders.refresh(o.id))).then(() => {
      const host = root.querySelector('#orders');
      if (!host || !root.isConnected) return;
      host.innerHTML = list();
      paintPlates(host);
    });
  }

  root.addEventListener('click', e => {
    const tab = e.target.closest('[data-tab]');
    if (!tab) return;
    state.tab = tab.dataset.tab;
    for (const b of root.querySelectorAll('[role="tab"]'))
      b.setAttribute('aria-selected', String(b.dataset.tab === state.tab));
    const host = root.querySelector('#orders');
    host.innerHTML = list();
    paintPlates(host);
    history.replaceState(null, '', `#/my-orders?tab=${state.tab}`);
  });
}
