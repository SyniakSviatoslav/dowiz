// Gallery 1:4366 and Restaurant Video 1:4321 (light 1:14766, 1:14721).
//
// The gallery is a grid of the venue's photographs; the video frame is one
// portrait player with a scrubber reading "01 : 15 / 01:59".
//
// THE GRID IS THE VENUE'S OWN PHOTOGRAPHS. Every product the owner console has
// a picture for is a cell, and the cell opens that dish — a photograph in a
// food app is a way into what is in it, not decoration.
//
// A venue that has uploaded none still gets the frame's eight-cell grid, but as
// PLAIN TILES rather than buttons, and the screen says what is missing. Eight
// tappable blanks that lead nowhere is what the interaction gate reported here,
// eight times: `DEAD button.k-shot`. A control is a promise, so a tile with
// nothing behind it must not be drawn as one.
//
// The video frame has no source at all: dowiz has no video field, and the
// player says what it is waiting for instead of showing a black rectangle.

import { icon, esc } from '/kit/app.js';
import { topBar, plate } from '/kit/parts.js';
import { menu, dishes } from '/kit/data.js';

// The frame's own eight cells, kept as the shape of an empty gallery.
const SHOTS = ['Interior', 'Counter', 'Terrace', 'Chef', 'Sushi bar', 'Street',
               'Table', 'Evening'];

const state = { playing: false, at: 75, length: 119 };

const clock = s => `${String(Math.floor(s / 60)).padStart(2, '0')} : ${
  String(Math.floor(s % 60)).padStart(2, '0')}`;

export async function render(params, routeName = 'gallery'){
  if (routeName === 'restaurant-video'){
    return `
    ${topBar('Restaurant Video')}
    <div class="wrap k-page">
      <div class="k-player">
        ${plate('The Savory Spot video')}
        <button class="k-player-play" type="button" id="play"
                aria-label="Відтворити">${icon('next-arrow')}</button>
        <div class="k-player-bar">
          <span id="at">${clock(state.at)}</span>
          <span class="k-player-track"><span class="k-player-fill" id="fill"
            data-pct="${Math.round(state.at / state.length * 100)}"></span></span>
          <span>${clock(state.length)}</span>
        </div>
      </div>
      <p class="k-page-p" id="said" role="status">Відео закладу ще не підключене:
        у dowiz немає поля для нього, тільки фотографії з консолі власника.</p>
    </div>`;
  }

  const shots = (dishes(await menu('uk'), 60) || []).filter(d => d.imageUrl);

  return `
  ${topBar('Gallery')}
  <div class="wrap k-page">
    <div class="k-shots">
      ${shots.length
        ? shots.map(d => `
        <button class="k-shot" type="button" aria-label="${esc(d.name)}"
                data-go="item-details?id=${encodeURIComponent(d.id)}">
          <img src="${esc(d.imageUrl)}" alt="${esc(d.name)}" loading="lazy">
        </button>`).join('')
        : SHOTS.map(s => `
        <div class="k-shot is-empty" role="presentation">${plate(s)}</div>`).join('')}
    </div>
    ${shots.length ? '' : `<p class="k-page-p">Заклад ще не завантажив фотографій.
      Вони з’являються тут, щойно власник додасть їх у консолі.</p>`}
  </div>`;
}

export function bind(root){
  const fill = root.querySelector('#fill');
  if (fill){
    fill.style.width = fill.dataset.pct + '%';
    fill.removeAttribute('data-pct');
  }

  root.querySelector('#play')?.addEventListener('click', () => {
    root.querySelector('#said').textContent =
      'Нема чого відтворювати: джерела відео ще немає.';
  });
}
