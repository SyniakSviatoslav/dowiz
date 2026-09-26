// The courier's screens, as PURE functions: data in, markup out.
//
// This file is the reference migration onto /lib/ui. Everything a courier sees
// inside the sheet is built here from the design system's components; app.js
// keeps the state, the network and the event wiring, and assigns the result to
// `#app.innerHTML`. Nothing here reads the DOM, the clock or the network, so
// every screen is rendered and checked in node (screens.test.mjs).
//
// The ids are a CONTRACT: app.js binds them and the e2e journeys drive them
// (#em #pw #go #toClaim #cph #cod #cpw #cgo #toLogin #retry #openShift #askBox
// #askGo #answer #earn #hist #endShift #take #takeOffer #offerClock #pick #done
// #slide #slideFill #refused #got #confirm #back #rnote #rgo #pback #etaLine #learn).
//
// `data-tour` marks the controls the lessons point at (docs/learn/lessons/
// courier/*.yaml). Each is a LITERAL '<module>.<control>' string, never built
// from parts, so tools/gates/learn.sh and docs/learn/anchors-courier.txt find it.
//
// `ctx` is { t, money, lang, langs }: the surface's translator and its ONE money
// formatter (/lib/money.js). Relative imports so node can load this file too.
import * as ui from '../lib/ui/index.js';
import { mcpButton } from './mcp.js';

const k = key => ({ t: key });
const ref = id => ui.orderRef(id, 8); // `#0123` from `ord_0123...`: never the storage prefix
const block = { size: 'lg', block: true };

export function login(err, ctx){
  return `<form class="login" id="loginForm" novalidate>
    <div class="login-mark" aria-hidden="true"><span>d</span></div>
    ${ui.section({ title: k('loginTitle'), sub: k('loginLine'), cls: 'login-head' })}
    ${ui.field({ id: 'em', label: k('emailOrPhone'), autocomplete: 'username', inputmode: 'email', enterkeyhint: 'next' })}
    ${ui.field({ id: 'pw', label: k('password'), type: 'password', autocomplete: 'current-password', enterkeyhint: 'go' })}
    ${err ? ui.alert({ label: err }) : ''}
    ${ui.button({ id: 'go', type: 'submit', variant: 'primary', icon: 'login', label: k('signIn'), ...block })}
    ${ui.button({ id: 'toClaim', variant: 'ghost', icon: 'ticket', label: k('haveCode'), block: true })}
    <div class="langs">${ui.segmented({ id: 'langSeg', label: k('language'), value: ctx.lang,
      options: ctx.langs.map(l => ({ value: l, label: l.toUpperCase() })) })}</div>
  </form>`;
}

export function claim(err){
  return `<form class="login" id="claimForm" novalidate>
    ${ui.section({ title: k('claimTitle'), sub: k('claimHint') })}
    ${ui.field({ id: 'cph', label: k('yourPhone'), type: 'tel', inputmode: 'tel', autocomplete: 'tel', enterkeyhint: 'next' })}
    ${ui.field({ id: 'cod', label: k('code'), autocomplete: 'one-time-code', autocapitalize: 'characters', spellcheck: false, maxlength: 16, enterkeyhint: 'next' })}
    ${ui.field({ id: 'cpw', label: k('choosePassword'), type: 'password', autocomplete: 'new-password', minlength: 8, enterkeyhint: 'go', hint: k('passwordHint') })}
    ${err ? ui.alert({ label: err }) : ''}
    ${ui.button({ id: 'cgo', type: 'submit', variant: 'primary', icon: 'check', label: k('start'), ...block })}
    ${ui.button({ id: 'toLogin', variant: 'ghost', icon: 'arrow-left', label: k('havePassword'), block: true })}
  </form>`;
}

/// BEFORE anything is claimed about the shift: the shape of the answer, not a
/// spinner in a void, and never "you are offline" on no evidence.
export const loading = ctx => ui.skeleton({ shapes: ['title', 'block', 'block', 'button'], label: ctx.t('loading') });

export const failed = error => ui.emptyState({ icon: 'plug-connected-x', title: k('noLink'), reason: error || '', alert: true })
  + ui.button({ id: 'retry', variant: 'success', icon: 'refresh', label: k('retry'), ...block });

export const offShift = () => ui.emptyState({ icon: 'moon-stars', title: k('youAreOffline'), body: k('offlineHint') })
  + ui.button({ id: 'openShift', variant: 'success', icon: 'player-play', label: k('openShift'), ...block, attrs: { data: { tour: 'shift.open' } } })
  + learnButton() + mcpButton();

const endShift = () => ui.button({ id: 'endShift', variant: 'ghost', icon: 'power', label: k('endShift'), block: true, attrs: { data: { tour: 'shift.end' } } });
/// The lessons (C1..C6, /lib/learn.js). Only where the courier stands still --
/// offline or waiting -- like the help button: never beside a live run.
const learnButton = () => ui.button({ id: 'learn', variant: 'ghost', icon: 'info-circle', label: k('learn'), block: true, attrs: { data: { tour: 'panel.learn' } } });

/// On shift, nothing in hand, nothing free: the one state where the courier is
/// standing still, so the only one with a text field and the panels.
export function waiting(){
  return `${ui.emptyState({ icon: 'radar-2', title: k('noneFree'), body: k('noneFreeHint') })}
    ${ui.inputRow({ id: 'askBox', label: k('askPlaceholder'), placeholder: k('askPlaceholder'), enterkeyhint: 'send', attrs: { data: { tour: 'ask.box' } },
      action: ui.iconButton({ id: 'askGo', icon: 'send', ariaLabel: k('send'), attrs: { data: { tour: 'ask.go' } } }) })}
    <p id="answer" class="answer" aria-live="polite" hidden></p>
    <div class="row2">
      ${ui.button({ id: 'earn', variant: 'secondary', icon: 'coins', label: k('myShifts'), block: true, attrs: { data: { tour: 'panel.earnings' } } })}
      ${ui.button({ id: 'hist', variant: 'secondary', icon: 'history', label: k('history'), block: true, attrs: { data: { tour: 'panel.history' } } })}
    </div>
    ${learnButton()}
    ${mcpButton()}
    ${endShift()}`;
}

/// Rows select; ONE button takes. N "Take" buttons would be N main actions.
export function pickList(available, sel, queued, ctx){
  const chosen = available.find(o => o.id === sel) || available[0];
  const rows = available.map(o => ui.row({ select: true, pressed: o.id === chosen.id, data: { sel: o.id }, attrs: { data: { tour: 'pick.row' } },
    title: ref(o.id), sub: o.address?.line || '—', trailing: ui.amount(ctx.money(o.total), { strong: true }) }));
  const q = queued(chosen.id);
  return `${ui.section({ title: k('readyForPickup'), sub: `${available.length} ${ctx.t('pcs')} · ${ctx.t('pickOne')}` })}
    ${ui.list(rows, { label: ctx.t('readyForPickup') })}
    ${ui.button({ id: 'take', variant: 'primary', disabled: !!q, icon: q ? 'cloud-upload' : 'package',
      label: `${ctx.t(q ? 'queued' : 'take')} ${ref(chosen.id)}`, ...block, attrs: { data: { tour: 'pick.take' } } })}
    ${endShift()}`;
}

/// The head of every screen about ONE order: what to do, where, and the money.
export function orderHead(o, picked, eta, ctx){
  const cash = o.payment === 'cash' ? o.total : 0;
  return `${ui.section({ title: k(picked ? 'delivering' : 'pickUpOrder') })}
    <p class="sub">${ui.status({ label: k(picked ? 'onTheWay' : 'ready'), status: picked ? 'IN_DELIVERY' : 'READY', pulse: picked })}
      <span>${ui.esc(ref(o.id))} · ${ui.esc(o.items)} ${ui.esc(ctx.t('items'))}</span></p>
    <p class="eta" id="etaLine" data-tour="run.eta">${ui.esc(eta || '')}</p>
    <div class="addr" data-tour="run.address">${ui.icon('map-pin')}<span>${ui.esc(o.address?.line || '—')}</span></div>
    ${o.address?.note ? `<p class="note">${ui.esc(o.address.note)}</p>` : ''}
    <div class="meta">
      ${cash ? `<span class="cash">${ui.icon('cash')}${ui.amount(ctx.money(cash), { size: 'lg', strong: true })}</span>`
             : ui.badge({ icon: 'credit-card', label: k('paidOnline'), tone: 'success' })}
      ${o.contact?.phone ? `<a class="tel" href="tel:${ui.esc(o.contact.phone)}">${ui.icon('phone')}${ui.esc(o.contact.phone)}</a>` : ''}
    </div>`;
}

/// A run in hand. A tap this phone is still holding REPLACES the control and
/// leaves the status (the hub's answer) alone.
export function active(o, { waiting: held, eta }, ctx){
  const picked = o.status === 'IN_DELIVERY';
  const addr = o.address?.line || '';
  const control = held
    ? ui.emptyState({ icon: 'cloud-upload', title: k('queued'), body: k('queuedHint'), status: true })
    : picked
      ? `<div class="slide" id="slide" data-tour="run.slide">
           <div class="slide-fill" id="slideFill"></div>
           ${ui.button({ id: 'done', variant: 'success', size: 'lg', icon: 'circle-check', label: k('delivered'), ariaLabel: k('deliveredAria'), cls: 'slide-knob' })}
           <span class="slide-hint" aria-hidden="true">${ui.esc(ctx.t('swipe'))}</span>
         </div>`
      : ui.button({ id: 'pick', variant: 'primary', icon: 'package', label: k('pickedUp'), ...block, attrs: { data: { tour: 'run.picked' } } });
  return `${orderHead(o, picked, eta, ctx)}
    ${control}
    <div class="row2">
      ${addr ? ui.button({ href: `https://www.openstreetmap.org/search?query=${encodeURIComponent(addr)}`, target: '_blank',
        variant: 'secondary', icon: 'external-link', label: k('inMaps'), block: true, attrs: { data: { tour: 'run.maps' } } }) : ''}
      ${o.contact?.phone ? ui.button({ href: `tel:${o.contact.phone}`, variant: 'secondary', icon: 'phone', label: k('call'), block: true, attrs: { data: { tour: 'run.call' } } }) : ''}
    </div>
    ${picked && !held ? ui.button({ id: 'refused', variant: 'ghost', icon: 'x', label: k('refusedAtDoor'), block: true, attrs: { data: { tour: 'run.refused' } } }) : ''}`;
}

export function cash(o, eta, ctx){
  return `${orderHead(o, true, eta, ctx)}
    ${ui.field({ id: 'got', label: k('howMuchCash'), money: true, value: o.total, autocomplete: 'off', enterkeyhint: 'done', attrs: { data: { tour: 'cash.got' } } })}
    ${ui.button({ id: 'confirm', variant: 'success', icon: 'circle-check', label: k('confirm'), ...block, attrs: { data: { tour: 'cash.confirm' } } })}
    ${ui.button({ id: 'back', variant: 'ghost', icon: 'arrow-left', label: k('back'), block: true })}`;
}

export const NOTE_MAX = 280;
export function refused(o, eta, ctx){
  return `${orderHead(o, true, eta, ctx)}
    ${ui.para(k('confirmRefused'))}
    ${ui.field({ id: 'rnote', label: k('refusedNoteLabel'), rows: 3, maxlength: NOTE_MAX, autocomplete: 'off', placeholder: k('refusedNoteHint'), attrs: { data: { tour: 'refused.note' } } })}
    ${ui.button({ id: 'rgo', variant: 'danger', icon: 'x', label: k('refusedAtDoor'), ...block, attrs: { data: { tour: 'refused.submit' } } })}
    ${ui.button({ id: 'back', variant: 'ghost', icon: 'arrow-left', label: k('back'), block: true })}`;
}

/// An offer is a thing being handed to you, so it is a card; the active run
/// deliberately looks different. `left` is seconds, worked out by the caller.
export function offer(o, left, ctx){
  const clock = left === 0
    ? ui.esc(ctx.t('offerLapsed'))
    : `${ui.esc(ctx.t('timeLeft'))} <b id="offerClock">${ui.mmss(left)}</b>`;
  return `${ui.card({ tone: 'accent', cls: 'offer', attrs: { data: { tour: 'offer.card' } }, body: `
      ${ui.badge({ label: k('offered'), tone: 'accent', dot: true })}
      <b class="oid">${ui.esc(ref(o.id))}</b>
      ${ui.amount(ctx.money(o.total), { size: 'lg', strong: true })}
      <p class="note">${ui.esc(o.address?.line || '—')}</p>
      <p class="ui-hint" id="offerLeft" data-tour="offer.clock">${clock}</p>` })}
    ${ui.button({ id: 'takeOffer', variant: 'primary', icon: 'package', label: k('take'), ...block, attrs: { data: { tour: 'offer.take' } } })}
    ${endShift()}`;
}

/// A panel REPLACES the sheet's content and puts a back button on it, rather
/// than adding a second navigation model to a one-job app.
export const panel = (title, body) => ui.section({ title, back: { id: 'pback', label: k('back') } }) + body;
export const panelLoading = ctx => ui.skeleton({ shape: 'row', count: 3, label: ctx.t('loading') });
export const panelError = msg => ui.alert({ label: msg });

/// Tips are shown APART from the float: the cash is the venue's, the tips are
/// the courier's own, and one combined figure is the wrong number either way.
export function earnings(d, ctx){
  const line = (key, w) => ui.row({ title: k(key), trailing:
    `<b>${ui.esc(w.deliveries)}</b> · ${ui.amount(ctx.money(w.cash))}${w.tips ? ` · ${ui.amount(ctx.money(w.tips), { sign: '+', tone: 'success' })}` : ''}` });
  return `${ui.stat({ label: k('cashInHand'), value: ui.amount(ctx.money(d.cashInHand), { size: 'xl', strong: true }), emphasis: true })}
    ${d.expectedCash ? `<p class="ui-hint">${ui.esc(ctx.t('stillOnRoad'))}: ${ui.amount(ctx.money(d.expectedCash))}</p>` : ''}
    ${ui.list([line('today', d.today), line('days7', d.week), line('days30', d.month)], { inset: true })}
    ${ui.para(k('earningsHint'), { hint: true })}`;
}

/// `date(at)` is the caller's, in the reader's locale.
export function history(rows, date, ctx){
  if (!rows.length) return ui.emptyState({ icon: 'history', title: k('emptyHistory'), body: k('emptyHistoryHint') });
  return ui.list(rows.map(r => ui.row({ title: r.street || '—', sub: `${date(r.at || 0)} · ${r.status}`,
    trailing: ui.amount(ctx.money(r.cashCollected ?? r.total ?? 0)) })), { inset: true });
}
