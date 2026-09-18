// Help Center — FAQ 1:10096 and Contact Us 1:10183 (light 1:20564, 1:20653).
//
// One frame with two tabs: FAQ is a search, a row of 35px category pills and a
// list of questions that open; Contact Us is a list of channels. So it is one
// screen with the tab in the route.

import { icon, esc } from '/kit/app.js';
import { topBar, menuList } from '/kit/parts.js';
import { menu, venue } from '/kit/data.js';

const TABS = [{ id: 'faq', name: 'FAQ' }, { id: 'contact', name: 'Contact Us' }];
const CATEGORIES = ['All', 'Services', 'General', 'Account'];

const FAQ = [
  { cat: 'General',  q: 'How do I place an order?',
    a: 'Pick a restaurant, add dishes to the cart and confirm the address and the payment method.' },
  { cat: 'Services', q: 'How long does delivery take?',
    a: 'The estimate shown on the restaurant is the venue’s own, and the courier updates it live.' },
  { cat: 'Account',  q: 'How do I change my address?',
    a: 'Profile → Manage Address. A new address can also be added during checkout.' },
  { cat: 'General',  q: 'Can I cancel an order?',
    a: 'Until the restaurant starts preparing it. After that the order is already being cooked.' },
  { cat: 'Account',  q: 'How do I delete my account?',
    a: 'Settings → Delete Account. Orders already placed are kept for the venue’s records.' },
];

// CONTACT US IS THE VENUE'S OWN CHANNELS, not the frame's six logos. The design
// draws Customer Service, Website, WhatsApp, Facebook, Instagram and X; dowiz
// knows a venue's telephone number and its hub, and nothing else. Six rows for
// six channels that a tap could not reach is what the interaction gate reported
// here — `DEAD button.k-row` for every one of them.
//
// So a channel is drawn when the venue has it, as a link the phone can act on,
// and the screen says plainly that the rest are not published rather than
// showing a logo behind a control that goes nowhere.
const channelsOf = v => {
  const list = [];
  if (v?.phone){
    const dial = `tel:${String(v.phone).replace(/[^+\d]/g, '')}`;
    list.push({ icon: 'notification-bing', label: 'Customer Service',
                value: v.phone, href: dial });
  }
  list.push({ icon: 'linear-map-location-compass', label: 'Website',
              value: location.hostname, href: '/', external: false });
  // A message to the venue is a dowiz thread, not a third party's app.
  list.push({ icon: 'linear-messages-conversation-chat-round-dots',
              label: 'Написати в чат', to: 'chat' });
  return list;
};

const state = { tab: 'faq', cat: 'All', open: new Set() };

const rows = () => FAQ.filter(f => state.cat === 'All' || f.cat === state.cat);

const faqRows = () => rows().map((f, i) => `
      <div class="k-faq-row${state.open.has(f.q) ? ' is-open' : ''}">
        <button class="k-faq-q" type="button" data-faq="${esc(f.q)}"
                aria-expanded="${state.open.has(f.q)}" aria-controls="a${i}">
          <span>${esc(f.q)}</span><span class="k-faq-mark">${icon('arrow-down')}</span>
        </button>
        <p class="k-faq-a" id="a${i}" ${state.open.has(f.q) ? '' : 'hidden'}>${esc(f.a)}</p>
      </div>`).join('');

const faqPane = () => `
  <label class="k-field k-field-flat">
    ${icon('search')}
    <input id="q" type="search" placeholder="Search" aria-label="Пошук у довідці">
  </label>
  <div class="k-pills k-block" role="group" aria-label="Категорії">
    ${CATEGORIES.map(c => `
      <button class="k-pill" type="button" data-cat="${esc(c)}"
              aria-pressed="${c === state.cat}">${esc(c)}</button>`).join('')}
  </div>
  <div class="k-faq" id="faq">${faqRows()}</div>`;

let PLACE = null;

const contactPane = () => `
  ${menuList(channelsOf(PLACE))}
  <p class="k-page-p">${PLACE?.name ? esc(PLACE.name) : 'Заклад'} не опублікував
    інших каналів. Соцмережі з’являться тут, щойно власник додасть їх у консолі.</p>`;

export async function render(params, routeName = 'help-faq'){
  state.tab = routeName === 'help-contact' ? 'contact' : 'faq';
  PLACE = venue(await menu('uk'));
  return `
  ${topBar('Help Center')}
  <div class="wrap">
    <div class="k-tabs k-tabs-wide" role="tablist">
      ${TABS.map(t => `
        <button type="button" role="tab" data-tab="${esc(t.id)}"
                aria-selected="${t.id === state.tab}">${esc(t.name)}</button>`).join('')}
    </div>
    <div id="pane" class="k-block" role="tabpanel">
      ${state.tab === 'faq' ? faqPane() : contactPane()}
    </div>
  </div>`;
}

export function bind(root){
  const repaint = () => {
    root.querySelector('#pane').innerHTML = state.tab === 'faq' ? faqPane() : contactPane();
  };

  root.addEventListener('click', e => {
    const tab = e.target.closest('[data-tab]');
    if (tab){
      state.tab = tab.dataset.tab;
      for (const b of root.querySelectorAll('[role="tab"]'))
        b.setAttribute('aria-selected', String(b.dataset.tab === state.tab));
      history.replaceState(null, '', `#/${state.tab === 'faq' ? 'help-faq' : 'help-contact'}`);
      return repaint();
    }
    // A CONTROL SURVIVES ITS OWN TAP. Both of these used to redraw the whole
    // pane, which threw away the very button that was pressed: the keyboard
    // lost its place, and `aria-pressed`/`aria-expanded` were reported by the
    // interaction gate as STATE STUCK, because the node it had just tapped no
    // longer existed to have changed. Only what the tap actually means is
    // rewritten.
    const cat = e.target.closest('[data-cat]');
    if (cat){
      state.cat = cat.dataset.cat;
      for (const b of root.querySelectorAll('[data-cat]'))
        b.setAttribute('aria-pressed', String(b.dataset.cat === state.cat));
      root.querySelector('#faq').innerHTML = faqRows();
      return;
    }

    const q = e.target.closest('[data-faq]');
    if (q){
      const key = q.dataset.faq;
      const open = !state.open.has(key);
      open ? state.open.add(key) : state.open.delete(key);
      q.setAttribute('aria-expanded', String(open));
      q.closest('.k-faq-row')?.classList.toggle('is-open', open);
      const answer = root.querySelector(`#${q.getAttribute('aria-controls')}`);
      if (answer) answer.hidden = !open;
    }
  });

  root.addEventListener('input', e => {
    if (e.target.id !== 'q') return;
    const needle = e.target.value.trim().toLowerCase();
    for (const row of root.querySelectorAll('.k-faq-row'))
      row.hidden = !!needle && !row.textContent.toLowerCase().includes(needle);
  });
}
