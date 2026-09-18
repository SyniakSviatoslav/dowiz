// dowiz — the Food Delivery kit, shell and router.
//
// The kit is 87 screens. They are separate modules under kit/screens/ and are
// loaded on demand, so opening Home does not download Chat: a phone on a
// restaurant's wifi is the target, not a laptop.
//
// Routing is by hash. The Worker serves this directory as static assets and
// never sees the path after `#`, so a deep link works without a route table
// on the edge, and a reload lands where the customer was.

import { mountSprite, icon } from '/kit/icons.js';
import { paintPlates, paintBasketBadge } from '/kit/parts.js';
import { register as registerWorker } from '/kit/install.js';

export { icon };

// The screen registry. A screen is `() => import(...)` so nothing is fetched
// until it is asked for; the value is the module's `render(params)`.
const SCREENS = {
  home:            () => import('/kit/screens/home.js'),
  onboarding:      () => import('/kit/screens/onboarding.js'),
  splash:          () => import('/kit/screens/splash.js'),
  welcome:         () => import('/kit/screens/splash.js'),
  'verify-code':   () => import('/kit/screens/verify-code.js'),
  'your-profile':     () => import('/kit/screens/forms.js'),
  'add-address':      () => import('/kit/screens/forms.js'),
  'password-manager': () => import('/kit/screens/forms.js'),
  'add-card':         () => import('/kit/screens/forms.js'),
  'profile-complete': () => import('/kit/screens/forms.js'),
  'help-faq':      () => import('/kit/screens/help.js'),
  'help-contact':  () => import('/kit/screens/help.js'),
  'privacy-policy':() => import('/kit/screens/privacy.js'),
  'my-wallet':     () => import('/kit/screens/wallet.js'),
  'add-money':     () => import('/kit/screens/wallet.js'),
  'invite-friends':() => import('/kit/screens/invite.js'),
  review:          () => import('/kit/screens/review.js'),
  explore:         () => import('/kit/screens/map.js'),
  'qr-entry':      () => import('/kit/screens/pass.js'),
  'remove-from-cart': () => import('/kit/screens/confirm.js'),
  logout:             () => import('/kit/screens/confirm.js'),
  'cancel-booking':   () => import('/kit/screens/confirm.js'),
  arrived:         () => import('/kit/screens/pass.js'),
  gallery:         () => import('/kit/screens/gallery.js'),
  'restaurant-video': () => import('/kit/screens/gallery.js'),
  'track-live':    () => import('/kit/screens/map.js'),
  'get-direction': () => import('/kit/screens/map.js'),
  'leave-review':  () => import('/kit/screens/review.js'),
  'rate-delivery': () => import('/kit/screens/review.js'),
  location:            () => import('/kit/screens/permission.js'),
  'manual-location':   () => import('/kit/screens/permission.js'),
  'notification-access': () => import('/kit/screens/permission.js'),
  signin:          () => import('/kit/screens/signin.js'),
  'create-account':() => import('/kit/screens/signin.js'),
  'new-password':  () => import('/kit/screens/signin.js'),
  cart:            () => import('/kit/screens/cart.js'),
  'track-order':   () => import('/kit/screens/track-order.js'),
  'my-orders':     () => import('/kit/screens/my-orders.js'),
  search:          () => import('/kit/screens/search.js'),
  profile:         () => import('/kit/screens/profile.js'),
  settings:        () => import('/kit/screens/settings.js'),
  'delivery-address': () => import('/kit/screens/chooser.js'),
  'payment-methods':  () => import('/kit/screens/chooser.js'),
  'order-type':       () => import('/kit/screens/chooser.js'),
  'delivery-type':    () => import('/kit/screens/chooser.js'),
  'pickup-time':      () => import('/kit/screens/chooser.js'),
  'manage-address':   () => import('/kit/screens/chooser.js'),
  'e-receipt':     () => import('/kit/screens/e-receipt.js'),
  'review-summary':  () => import('/kit/screens/review-summary.js'),
  'book-a-table':    () => import('/kit/screens/booking.js'),
  chat:            () => import('/kit/screens/chat.js'),
  'chat-detail':   () => import('/kit/screens/chat.js'),
  'voice-call':    () => import('/kit/screens/chat.js'),
  'my-booking':      () => import('/kit/screens/booking.js'),
  'booking-summary': () => import('/kit/screens/review-summary.js'),
  notification:    () => import('/kit/screens/notification.js'),
  coupon:          () => import('/kit/screens/coupon.js'),
  filter:          () => import('/kit/screens/filter.js'),
  'popular-dishes':      () => import('/kit/screens/listing.js'),
  'popular-restaurants': () => import('/kit/screens/listing.js'),
  'exclusive-offers':    () => import('/kit/screens/listing.js'),
  category:              () => import('/kit/screens/listing.js'),
  'my-favourites':       () => import('/kit/screens/listing.js'),
  'payment-successful':    () => import('/kit/screens/done.js'),
  'reservation-confirmed': () => import('/kit/screens/done.js'),
  'top-up-successful':     () => import('/kit/screens/done.js'),
  'item-details':  () => import('/kit/screens/item-details.js'),
  compare:         () => import('/kit/screens/compare.js'),
  'restaurant-menu':   () => import('/kit/screens/restaurant-menu.js'),
  'restaurant-about':  () => import('/kit/screens/restaurant-menu.js'),
  'restaurant-gallery':() => import('/kit/screens/restaurant-menu.js'),
  'restaurant-review': () => import('/kit/screens/restaurant-menu.js'),
};

const app = () => document.getElementById('app');

// EVERY SCREEN GETS A NEW NODE TO BIND TO.
//
// A screen's `bind` attaches its listeners to the element it is handed. That
// element used to be `#app` itself, which never changes — so the listeners never
// went away. Leave a screen and come back and its handler is attached TWICE, and
// a toggle fires twice and lands back where it started: the control does exactly
// nothing, from the user's side, and there is no error anywhere to explain it.
// Measured on the filter screen — first visit works, second visit is inert — and
// it is the same mechanism behind a heart that would not stay filled.
//
// It was also worse than double: a screen left behind kept listening on every
// LATER screen, so home's handler was still running while the customer was in
// the cart.
//
// `#app` stays, because it is the live region and CSS keys off it. The screen is
// mounted in a child that is replaced wholesale, and replacing a node takes its
// listeners with it.
const mount = () => {
  const host = document.createElement('div');
  host.className = 'k-screen';
  app().replaceChildren(host);
  return host;
};

function parseHash(){
  const raw = location.hash.replace(/^#\/?/, '');
  const [name, query] = raw.split('?');
  return { name: name || 'home', params: new URLSearchParams(query || '') };
}

// THE LAST NAVIGATION WINS.
//
// A screen is fetched on demand, so `route()` is asynchronous and two taps in
// quick succession start two of them. Nothing made the slower one stand down:
// tap Cart and then immediately Profile, and if Cart's module arrives second it
// paints over Profile — the address bar says one screen and the app shows
// another. Measured, not imagined: driving two hash changes back to back
// reproduced it every time, and it is why a screen's controls could be detached
// from the document a moment after they were drawn.
//
// The counter is taken before the await and checked after it. A render whose
// ticket is no longer current does nothing at all.
let navigation = 0;

async function route(){
  const mine = ++navigation;
  const { name, params } = parseHash();
  // Cleared first: while the next screen is being fetched the app is between
  // screens, and saying it is still on the last one is worse than saying nothing.
  delete app().dataset.route;
  const load = SCREENS[name];
  if (!load){
    mount().innerHTML = notFound(name);
    return;
  }
  // A screen that fails to load is a broken deploy, not a dead end: say which
  // one, because "nothing rendered" is the report that cost a day.
  let mod;
  try { mod = await load(); }
  catch (e){ if (mine === navigation) mount().innerHTML = failed(name, e); return; }
  if (mine !== navigation) return;              // a newer navigation overtook us
  const markup = await mod.render(params, name);
  if (mine !== navigation) return;              // ...and again, render can await
  const screen = mount();
  screen.innerHTML = markup;
  paintPlates(screen);
  paintFavourites(screen);
  paintBasketBadge(screen);
  mod.bind?.(screen, params);
  // The screen that is actually mounted, named on the element that holds it.
  // A screen module is fetched on demand, so between the hash changing and the
  // markup arriving the app still shows the PREVIOUS screen — and anything
  // watching from outside, a test or a stylesheet, had no way to tell the two
  // apart. `data-route` is set last, after the markup and the bindings, so its
  // presence means the screen is finished rather than merely started.
  app().dataset.route = name;
  scrollTo(0, 0);
}

// ── Favourites ───────────────────────────────────────────────────────────────
// The heart is drawn by `heroButtons` and by every card, on a dozen screens, so
// it is handled ONCE here rather than a dozen times. It was drawn on all of them
// and wired on one, which is how a control ends up looking like a control and
// doing nothing.
//
// It persists, because a heart that forgets on reload is decoration. The key is
// the card's own item id where there is one, and the route otherwise — a venue's
// hero heart favourites the venue.
const FAVS = 'dowiz.favourites';
const readFavs = () => {
  try { return new Set(JSON.parse(localStorage.getItem(FAVS) || '[]')); }
  catch { return new Set(); }                 // private window, or someone's junk
};
const writeFavs = set => {
  try { localStorage.setItem(FAVS, JSON.stringify([...set])); } catch { /* fine */ }
};
const favKeyOf = el => el.dataset.fav
  || el.closest('[data-item]')?.dataset.item
  || el.closest('[data-venue]')?.dataset.venue
  || parseHash().name;

/** Paint every heart on the screen from what was saved. */
export function paintFavourites(root = app()){
  const saved = readFavs();
  for (const h of root.querySelectorAll('.k-heart,[data-fav],.k-hero-btn[aria-pressed]'))
    h.setAttribute('aria-pressed', String(saved.has(favKeyOf(h))));
}

// ── One line of feedback ─────────────────────────────────────────────────────
// Built with the DOM, not innerHTML with a style attribute: `style-src 'self'`
// forbids the latter, and this is the one thing that has to appear over any
// screen.
let toastSeq = 0;

export function toast(text){
  document.querySelector('.k-toast')?.remove();
  const el = document.createElement('div');
  el.className = 'k-toast';
  el.setAttribute('role', 'status');
  // EACH ANSWER IS A NEW ANSWER, even when it says the same words. Replacing a
  // toast with an identical one leaves the document byte-for-byte unchanged, so
  // a second tap on "copy the order number" was indistinguishable from a tap
  // that did nothing -- to the interaction gate, which reported the control as
  // DEAD, and to a screen reader, which re-announces `role="status"` only when
  // the node changes. The counter is what makes the repeat visible to both.
  el.dataset.seq = String(++toastSeq);
  el.textContent = text;
  document.body.append(el);
  setTimeout(() => el.remove(), 2600);
  return el;
}

// Navigation is delegated once, at the root, so a screen only writes markup:
// `data-go` goes to a route, `data-back` goes back. A screen that wants to do
// something else on a tap handles the event first and stops it.
document.addEventListener('click', e => {
  const back = e.target.closest('[data-back]');
  if (back){ history.length > 1 ? history.back() : go('home'); return; }

  // A heart the screen did not claim for itself.
  const heart = e.target.closest('.k-heart,[data-fav],.k-hero-btn[aria-pressed]');
  if (heart){
    const key = favKeyOf(heart);
    const saved = readFavs();
    const on = !saved.has(key);
    on ? saved.add(key) : saved.delete(key);
    writeFavs(saved);
    heart.setAttribute('aria-pressed', String(on));
    return;
  }

  // Sharing. `navigator.share` is the real thing on a phone; a desktop and a
  // headless browser have no share sheet, so the link goes to the clipboard and
  // the app SAYS which of the two happened rather than failing silently.
  const share = e.target.closest('[data-share]');
  if (share){
    const data = { title: document.title, url: location.href };
    if (navigator.share){
      navigator.share(data).catch(() => { /* the person cancelled; nothing to say */ });
      return;
    }
    navigator.clipboard?.writeText(location.href)
      .then(() => toast('Посилання скопійоване'))
      .catch(() => toast(location.href));
    if (!navigator.clipboard) toast(location.href);
    return;
  }

  const nav = e.target.closest('[data-go]');
  if (nav) go(nav.dataset.go);
});

const notFound = name => `<div class="wrap k-note">
  <h1 class="t-title">Немає такого екрана</h1>
  <p class="t-body muted2">${esc(name)}</p></div>`;

const failed = (name, e) => `<div class="wrap k-note">
  <h1 class="t-title">Екран не завантажився</h1>
  <p class="t-body muted2">${esc(name)} — ${esc(String(e && e.message || e))}</p></div>`;

export const esc = s => String(s ?? '').replace(/[&<>"']/g, c =>
  ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

// `go` takes a route, optionally with its own query: `item-details?id=item-01`.
// Screens name their destination in markup, and a destination that cannot carry
// an id is a destination that opens the wrong record.
export const go = name => { location.hash = '#/' + name; };

mountSprite();
registerWorker();
addEventListener('hashchange', route);
route();
