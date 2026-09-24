// Privacy Policy — Figma node 1:10282 (dark) / 1:20754 (light).
//
// The same 15/500 heading over a 12/400 paragraph as the Figma frame. The text
// is the venue's REAL notice, which the Worker renders at `/privacy` from the
// personal-data registry and the venue's own settings (P8 of
// BLUEPRINT-GDPR-AND-MCP-2026-09-24), in sq/en/uk. This screen points there
// rather than copying it, so there is one notice and it cannot go stale here.

import { esc } from '/kit/app.js';
import { topBar } from '/kit/parts.js';

const LEAD = 'The venue is responsible for your data. Its privacy notice says what is kept about you, ' +
  'why, for how long, who receives it, and how to see, correct or erase it. Requests are answered within 30 days.';

const LANGS = [['sq', 'Shqip'], ['en', 'English'], ['uk', 'Українська']];

export function render(){
  return `
  ${topBar('Privacy Policy')}
  <div class="wrap k-page">
    <section class="k-legal">
      <h2>${esc('Privacy notice')}</h2>
      <p>${esc(LEAD)}</p>
      ${LANGS.map(([l, name]) => `<p><a href="/privacy?lang=${l}" target="_blank" rel="noopener">${esc(name)}</a></p>`).join('')}
    </section>
  </div>`;
}
