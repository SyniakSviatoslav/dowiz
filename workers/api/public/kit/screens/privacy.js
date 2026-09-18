// Privacy Policy — Figma node 1:10282 (dark) / 1:20754 (light).
//
// Two 15/500 orange headings with 12/400 paragraphs under each. The kit's copy
// is lorem; a venue's real policy replaces the strings and nothing else.

import { esc } from '/kit/app.js';
import { topBar } from '/kit/parts.js';

const LOREM_LONG = 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod ' +
  'tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud ' +
  'exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.';
const LOREM_SHORT = 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod ' +
  'tempor incididunt ut labore et.';

const SECTIONS = [
  { head: 'Cancelation Policy', paras: [LOREM_LONG, LOREM_SHORT] },
  { head: 'Terms & Condition',  paras: [LOREM_LONG, LOREM_LONG, LOREM_SHORT,
                                        LOREM_LONG, LOREM_LONG, LOREM_SHORT, LOREM_LONG] },
];

export function render(){
  return `
  ${topBar('Privacy Policy')}
  <div class="wrap k-page">
    ${SECTIONS.map(s => `
      <section class="k-legal">
        <h2>${esc(s.head)}</h2>
        ${s.paras.map(p => `<p>${esc(p)}</p>`).join('')}
      </section>`).join('')}
  </div>`;
}
