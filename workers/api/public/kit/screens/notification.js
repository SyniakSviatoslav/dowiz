// Notification — Figma node 1:7208 (dark) / 1:17627 (light).
//
// Spec read from design/spec.py 1:7208: title 16/500 with a 58x35 orange "2 NEW"
// pill beside it; sections TODAY at 24,124 and YESTERDAY at 24,502, each with a
// 14/400 muted heading and an orange "Mark all as read"; rows are a 56px surface
// circle, a 16/500 title, a 12/400 body 255 wide and the age at the right.

// A NOTIFICATION IS A POINTER, not a paragraph. Every row names the screen it
// is about and opens it, because "tapped it and nothing happened" is exactly
// what a notification that only marks itself read feels like. Marking read and
// opening are the same tap: the row's own handler records the read and lets the
// event reach the router's `data-go`.
//
// "Mark all as read" is disabled once that day has nothing unread, rather than
// staying tappable and doing nothing.

import { icon, esc } from '/kit/app.js';
import { topBar } from '/kit/parts.js';

const NOTES = [
  { day: 'TODAY', id: 'n1', icon: 'scooter-20', unread: true, age: '1h',
    to: 'track-order', title: 'Order On the Way',
    body: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor' },
  { day: 'TODAY', id: 'n2', icon: 'ticket', unread: true, age: '3h',
    to: 'coupon', title: 'New Coupon Unlocked',
    body: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor' },
  { day: 'TODAY', id: 'n3', icon: 'box-tick', unread: false, age: '6h',
    to: 'my-orders', title: 'Order Delivered',
    body: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor' },
  { day: 'YESTERDAY', id: 'n4', icon: 'reserve-20', unread: false, age: '1d',
    to: 'my-booking', title: 'Table Booked Successfully!',
    body: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor' },
  { day: 'YESTERDAY', id: 'n5', icon: 'wallet', unread: false, age: '1d',
    to: 'my-wallet', title: 'Wallet Topped Up',
    body: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor' },
];

const state = { read: new Set(NOTES.filter(n => !n.unread).map(n => n.id)) };

const unreadCount = () => NOTES.filter(n => !state.read.has(n.id)).length;

const row = n => `
  <button class="k-note-row${state.read.has(n.id) ? '' : ' is-unread'}" type="button"
          data-note="${esc(n.id)}" data-go="${esc(n.to)}">
    <span class="k-note-ring">${icon(n.icon)}</span>
    <span class="k-note-body">
      <span class="k-note-head">
        <span class="k-note-t">${esc(n.title)}</span>
        <span class="k-note-age">${esc(n.age)}</span>
      </span>
      <span class="k-note-b">${esc(n.body)}</span>
    </span>
  </button>`;

const unreadOn = day => NOTES.some(n => n.day === day && !state.read.has(n.id));

const section = day => `
  <div class="k-note-sec">
    <h2>${esc(day)}</h2>
    <button class="k-seeall" type="button" data-readall="${esc(day)}"
            ${unreadOn(day) ? '' : 'disabled'}>Mark all as read</button>
  </div>
  <div class="k-notes">${NOTES.filter(n => n.day === day).map(row).join('')}</div>`;

const panel = () => ['TODAY', 'YESTERDAY'].map(section).join('');

export function render(){
  return `
  ${topBar('Notification')}
  <div class="wrap">
    <p class="k-block-h" id="head">Notification
      ${unreadCount() ? `<span class="k-count">${unreadCount()} NEW</span>` : ''}</p>
    <div id="notes">${panel()}</div>
  </div>`;
}

export function bind(root){
  const repaint = () => {
    root.querySelector('#notes').innerHTML = panel();
    const n = unreadCount();
    root.querySelector('#head').innerHTML =
      `Notification ${n ? `<span class="k-count">${n} NEW</span>` : ''}`;
  };

  root.addEventListener('click', e => {
    const all = e.target.closest('[data-readall]');
    if (all){
      for (const n of NOTES) if (n.day === all.dataset.readall) state.read.add(n.id);
      return repaint();
    }
    const one = e.target.closest('[data-note]');
    if (one){
      state.read.add(one.dataset.note);
      // The router's `data-go` handler runs after this one and opens the screen
      // the row is about, so there is nothing left to repaint. Repainting first
      // would hand that handler a node this screen had already discarded.
      if (!one.dataset.go) repaint();
    }
  });
}
