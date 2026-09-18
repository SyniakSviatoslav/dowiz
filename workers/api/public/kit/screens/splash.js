// Splash 1:2070 and Welcome 1:884 (light set 1:12483, 1:11101).
//
// Splash is the logo alone at 114,341 — 148x128. Welcome puts a 209x420
// illustration panel above a 327 block at 24,530: a 24/600 headline, a 14/400
// line under it, a 327x48 call to action, and a way to sign in instead.

import { icon, esc } from '/kit/app.js';

const COPY = {
  splash: null,
  welcome: {
    head: 'Discover Delicious Food Delivered to Your Door',
    body: 'Explore a Variety of Delicious Foods, Pick Your Favorite Dishes',
    cta: 'Let’s Get Started',
    swap: 'Already have an account? ',
    link: 'Sign In',
  },
};

export function render(params, routeName = 'splash'){
  if (routeName === 'splash'){
    // The splash holds for a moment and then goes on by itself; a splash that
    // waits for a tap is a screen with nothing on it.
    return `<div class="k-splash" role="status" aria-label="dowiz">
      ${icon('logo-union')}
    </div>`;
  }

  const c = COPY.welcome;
  return `
  <div class="k-welcome">
    <div class="k-welcome-art" role="img" aria-label="Ілюстрація">
      ${icon('onb-screen')}
    </div>
    <div class="k-welcome-copy">
      <h1>${esc(c.head)}</h1>
      <p>${esc(c.body)}</p>
      <button class="k-submit" type="button" data-go="onboarding?step=1">${esc(c.cta)}</button>
      <p class="k-swap">${esc(c.swap)}<button type="button" data-go="signin">${esc(c.link)}</button></p>
    </div>
  </div>`;
}

export function bind(root, params){
  if (!root.querySelector('.k-splash')) return;
  const go = () => { location.hash = '#/welcome'; };
  const timer = setTimeout(go, 1400);
  // A splash nobody can skip is a splash that feels broken on a slow phone.
  root.addEventListener('click', () => { clearTimeout(timer); go(); }, { once: true });
}
