// Every gate, one exit code. `node e2e/kit-regression/run.mjs`
//
// HOST      which hub to point at (default https://dubin-sushi.dowiz.org)
// RETRIES   how many times an edge transient is retried (default 2)
// ONLY      render|domains|mobile|pwa|interact, to run one of them
import { run as render } from './render.mjs';
import { run as domains } from './domains.mjs';
import { run as mobile } from './mobile.mjs';
import { run as pwa } from './pwa.mjs';
import { run as interact } from './interact.mjs';
import { run as outbox } from './outbox.mjs';

const GATES = [
  ['render',   'render gate — real Chromium at 375x812',        () => render(null)],
  ['mobile',   'mobile gate — 320/390/412 with touch',          mobile],
  ['interact', 'interaction gate — every control does something', interact],
  ['pwa',      'installability gate — manifest, worker, offline', pwa],
  ['domains',  'domain contracts — the kernel, over HTTP',       domains],
  // The only gate here that does NOT touch HOST: it serves `public/` from disk
  // against a counting stub, because half of what it proves is a refusal being
  // replayed and that must never be replayed at a live venue.
  ['outbox',   'offline writes — a tap survives a dead network',  outbox],
];

const only = process.env.ONLY;
let failed = 0;
for (const [name, title, fn] of GATES) {
  if (only && only !== name) continue;
  console.log(`\n══ ${title}\n`);
  failed += await fn();
}

console.log(failed ? `\nTOTAL: ${failed} failure(s)` : '\nTOTAL: everything passes');
process.exit(failed ? 1 : 0);
