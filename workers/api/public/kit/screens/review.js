// Reviews — Review 1:5065, Leave Review 1:5209 and 1:7820, Rate Delivery
// Partner 1:7488. Light set: 1:15471, 1:15616, 1:18243, 1:17911.
//
// Reading reviews and writing one are two shapes, and "rate the courier" is the
// writing one with a person at the top instead of a dish. So: one module, three
// modes named by the route.
//
// dowiz DOES NOT SCORE PEOPLE. `DECISIONS.md` makes trust a signed capability
// and a CI job fails the build if a `courier_score`/`rating`/`reputation`
// identifier appears in the kernel. So Rate Delivery Partner is drawn exactly as
// the kit draws it and says plainly, on submit, that the score is not kept
// against the courier.

import { icon, esc } from '/kit/app.js';
import { topBar, plate, orderLine, ctaBar } from '/kit/parts.js';

const SUMMARY = { score: 4.8, count: '1.2K Reviews',
                  bars: [[5, 78], [4, 14], [3, 5], [2, 2], [1, 1]] };

const CHIPS = ['Filter', 'Verified', 'Latest', 'Detailed Reviews'];

const REVIEWS = [
  { who: 'Leslie Alexander', when: '1 day ago', stars: 5,
    body: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt' },
  { who: 'Marvin McKinney', when: '3 days ago', stars: 4,
    body: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt' },
  { who: 'Jenny Wilson', when: '1 week ago', stars: 5,
    body: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt' },
];

const ITEM = { name: 'ItaliaCrisp Pizza', kind: 'Pizza', variant: '8’ - Small', price: '$12.00' };
const PARTNER = { name: 'Charlotte Taylor', role: 'Delivery Partner', rating: 4.8,
                  order: '#FD785462' };

const state = { stars: 0, chip: 'Latest' };

const readPane = () => `
  <div class="k-score"><b>${SUMMARY.score}</b><span>(${esc(SUMMARY.count)})</span></div>
  <div class="k-bars">
    ${SUMMARY.bars.map(([n, pct]) => `
      <div class="k-bar-row">${n}${icon('star2')}
        <span class="k-bar-track"><span class="k-bar-fill" data-pct="${pct}"></span></span>
      </div>`).join('')}
  </div>

  <label class="k-field k-field-flat">
    ${icon('search')}
    <input id="q" type="search" placeholder="Search in reviews" aria-label="Пошук у відгуках">
  </label>
  <div class="k-chips" role="group" aria-label="Фільтр відгуків">
    ${CHIPS.map(c => `
      <button class="k-chip${c === state.chip ? ' on' : ''}" type="button" data-chip="${esc(c)}"
              aria-pressed="${c === state.chip}">${esc(c)}</button>`).join('')}
  </div>

  <div id="reviews">
    ${REVIEWS.map(r => `
      <article class="k-review">
        <div class="k-review-head">
          <span class="k-review-img">${plate(r.who)}</span>
          <span class="k-review-body">
            <span class="k-review-n">${esc(r.who)}</span>
            <span class="k-review-w">${esc(r.when)}</span>
          </span>
          <span class="k-rate">${icon('star2')}${r.stars}.0</span>
        </div>
        <p class="k-review-b">${esc(r.body)}</p>
      </article>`).join('')}
  </div>`;

const writePane = partner => `
  ${partner ? `
    <div class="k-partner">
      <span class="k-partner-img">${plate(PARTNER.name)}</span>
      <span>
        <span class="k-partner-n">${esc(PARTNER.name)}</span>
        <span class="k-partner-r">${esc(PARTNER.role)} · ${PARTNER.rating} · ${
          esc(PARTNER.order)}</span>
      </span>
    </div>`
  : orderLine(ITEM)}

  <h2 class="k-rate-head">${partner
    ? `How was Delivery Experience with ${esc(PARTNER.name.split(' ')[0])}?`
    : 'How was your food experience?'}</h2>
  <p class="k-rate-sub">Your overall rating</p>
  <div class="k-stars-pick" role="radiogroup" aria-label="Оцінка">
    ${[1,2,3,4,5].map(n => `
      <button type="button" role="radio" data-star="${n}" aria-checked="${n === state.stars}"
              aria-label="${n} з 5">${icon('star2')}</button>`).join('')}
  </div>

  <div class="k-in k-block">
    <label for="body">Add detailed review</label>
    <input id="body" type="text" placeholder="Enter here">
    <span class="k-in-err" id="err" hidden></span>
  </div>
  <button class="k-photo-add" type="button" id="addPhoto">${icon('add-20')}Add photo</button>
  <p class="k-page-p" id="said" role="status"></p>`;

export function render(params, routeName = 'review'){
  const partner = routeName === 'rate-delivery';
  const write = partner || routeName === 'leave-review';
  return `
  ${topBar(partner ? 'Rate Delivery Partner' : write ? 'Leave Review' : 'Review')}
  <div class="wrap k-page" data-review="${esc(routeName)}">
    ${write ? writePane(partner) : readPane()}
  </div>
  ${write ? ctaBar('Submit') : ''}`;
}

export function bind(root){
  const mode = root.querySelector('[data-review]').dataset.review;

  // The distribution bars are percentages, so they are CSSOM writes.
  for (const el of root.querySelectorAll('.k-bar-fill[data-pct]')){
    el.style.width = el.dataset.pct + '%';
    el.removeAttribute('data-pct');
  }

  const paintStars = () => {
    for (const b of root.querySelectorAll('[data-star]')){
      const on = Number(b.dataset.star) <= state.stars;
      b.classList.toggle('is-on', on);
      b.setAttribute('aria-checked', String(Number(b.dataset.star) === state.stars));
    }
  };

  root.addEventListener('click', e => {
    const star = e.target.closest('[data-star]');
    if (star){ state.stars = Number(star.dataset.star); return paintStars(); }

    const chip = e.target.closest('[data-chip]');
    if (chip){
      state.chip = chip.dataset.chip;
      for (const b of root.querySelectorAll('[data-chip]')){
        const on = b === chip;
        b.setAttribute('aria-pressed', String(on));
        b.classList.toggle('on', on);
      }
      return;
    }

    if (e.target.closest('#addPhoto')){
      root.querySelector('#said').textContent = 'Завантаження фото ще не підключене.';
      return;
    }

    if (e.target.closest('[data-cta]')){
      const err = root.querySelector('#err');
      if (!state.stars){
        err.textContent = 'Оберіть оцінку';
        err.hidden = false;
        return;
      }
      err.hidden = true;
      root.querySelector('#said').textContent = mode === 'rate-delivery'
        ? 'Дякуємо. Оцінка йде закладу як відгук про замовлення — dowiz не веде рейтингів кур’єрів: '
          + 'довіра тут це підписана здатність, а не бал.'
        : 'Дякуємо! Відгук надіслано закладу.';
    }
  });

  root.addEventListener('keydown', e => {
    if (!e.target.closest('[data-star]')) return;
    if (e.key === 'ArrowRight' || e.key === 'ArrowUp'){
      state.stars = Math.min(5, state.stars + 1); paintStars(); e.preventDefault();
    }
    if (e.key === 'ArrowLeft' || e.key === 'ArrowDown'){
      state.stars = Math.max(1, state.stars - 1); paintStars(); e.preventDefault();
    }
  });

  root.addEventListener('input', e => {
    if (e.target.id !== 'q') return;
    const needle = e.target.value.trim().toLowerCase();
    for (const r of root.querySelectorAll('.k-review'))
      r.hidden = !!needle && !r.textContent.toLowerCase().includes(needle);
  });
}
