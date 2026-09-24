// The room's stylesheet against its own markup. `node --test workers/api/public/room/css.test.mjs`.
// The live walk (2026-09-24) saw "0 queued" on every phone: `#outboxTag` is
// `class="tag" hidden`, and `.tag{display:inline-flex}` outranks the UA's
// `[hidden]{display:none}`. A class selector beats an attribute one only by
// order, and an author rule beats the UA's always, so `hidden` needs its own
// author rule with !important.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const here = new URL('.', import.meta.url);
const css = readFileSync(new URL('room.css', here), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const html = readFileSync(new URL('index.html', here), 'utf8');

/// Every class on an element the markup starts `hidden`.
function hiddenClasses() {
  const out = new Set();
  for (const m of html.matchAll(/<[a-z]+\b[^>]*\bhidden\b[^>]*>/g)) {
    const c = m[0].match(/class="([^"]*)"/);
    for (const k of (c ? c[1].split(/\s+/) : [])) if (k) out.add(k);
  }
  return out;
}

test('room: [hidden] is display:none !important', () => {
  assert.match(css, /(^|\})\s*\[hidden\]\s*\{\s*display\s*:\s*none\s*!important\s*;?\s*\}/);
});

test('room: the queued tag starts hidden, and a rule makes `hidden` beat its class', () => {
  // Since the /lib/ui migration the tag is a `ui-chip` (display:inline-flex in
  // ui.css). ui.css carries its own `[hidden]` rule for `ui-*` classes; the
  // room's `[hidden]` rule above covers the rest.
  assert.ok(hiddenClasses().has('ui-chip'), 'index.html starts #outboxTag hidden with class ui-chip');
  const ui = readFileSync(new URL('../lib/ui/ui.css', here), 'utf8');
  assert.match(ui, /\[class\^="ui-"\]\[hidden\][^{]*\{display:none!important\}/, 'ui.css: [hidden] wins over a ui-* class');
  assert.match(html, /href="\/lib\/ui\/ui\.css"/, 'index.html loads ui.css');
  assert.ok(html.indexOf('/lib/ui/ui.css') < html.indexOf('/room/room.css'), 'ui.css before the room sheet, so the room can tune it');
});
