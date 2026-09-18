// Onboarding — Figma nodes 1:2090, 1:2157, 1:2231 (dark); 1:12503+ (light).
//
// Three frames that differ only in the illustration, the headline and which
// pager dot is lit, so they are ONE screen with the step in the route:
// `#/onboarding?step=2`. Three modules would be three copies of the same
// layout, and the kit's next two steps would drift from the first.

import { icon, esc, go } from '/kit/app.js';

const STEPS = [
  { head: ['Order Delicious ', 'Food Anytime, Anywhere'],
    body: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ' },
  { head: ['Track Your Order ', 'Live, Every Step'],
    body: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ' },
  { head: ['Pay Your Way, ', 'Fast and Secure'],
    body: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ' },
];

const clamp = n => Math.max(0, Math.min(STEPS.length - 1, n));

export function render(params){
  const i = clamp(Number(params?.get('step') || 1) - 1);
  const step = STEPS[i];
  const last = i === STEPS.length - 1;
  return `
  <!-- NOT \`k-screen\`. That class is the router's mount point and is
       \`display:contents\` -- an element with NO BOX. Wearing it here deleted
       this screen's box, and with it the \`overflow:hidden\` that keeps the
       sheet's curve inside the phone: the curve is 105% wide by design, it bled
       10px past the viewport, and the first screen a customer ever sees scrolled
       sideways. -->
  <div class="k-onb">
    <button class="k-onb-skip" type="button" data-go="signin">Skip</button>

    <div class="k-onb-art">
      <span class="k-onb-stars" aria-hidden="true">${icon('star-15')}${icon('star-16')}</span>
      <!-- The illustration is the file's own iPhone mock: a 241x497 bezel with
           the exported screen artwork inside it. -->
      <div class="k-device" role="img" aria-label="Ілюстрація кроку ${i + 1}">
        ${icon('onb-screen')}
      </div>
    </div>

    <div class="k-onb-sheet">
      ${icon('sheet-curve', 'k-onb-curve')}
      <h1 class="k-onb-h">${esc(step.head[0])}<em>${esc(step.head[1])}</em></h1>
      <p class="k-onb-p">${esc(step.body)}</p>
      <div class="k-onb-foot">
        <span class="k-onb-dots" aria-label="Крок ${i + 1} з ${STEPS.length}">
          ${STEPS.map((_, n) => n === i
            ? icon('pager-dot')
            : `<span class="k-dot"></span>`).join('')}
        </span>
        <button class="k-next" type="button" id="next"
                aria-label="${last ? 'Почати' : 'Далі'}">
          ${icon('next-ring')}${icon('next-arrow')}
        </button>
      </div>
    </div>
  </div>`;
}

export function bind(root){
  root.querySelector('#next')?.addEventListener('click', () => {
    const i = clamp(Number(new URLSearchParams(location.hash.split('?')[1]).get('step') || 1) - 1);
    if (i === STEPS.length - 1) return go('signin');
    location.hash = `#/onboarding?step=${i + 2}`;
  });
}
