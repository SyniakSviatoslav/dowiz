// The storefront's motion, in node: a language switch or a filter measures
// every node it animates BEFORE it starts the first animation. A measure read
// after an animation or a style write makes the browser lay the page out again,
// once per node: a switch of language froze a phone for 8 s on a menu.
// `node --test workers/api/public/store/motion.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';

// motion.js imports '/store/ui.js' by absolute path; this answers it.
register('data:text/javascript,' + encodeURIComponent(`
  const F = { '/store/ui.js': 'export const $$ = (...a) => globalThis.__mfake.$$(...a); export const reduced = () => false;' };
  export async function resolve(spec, ctx, next){
    return F[spec] ? { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true } : next(spec, ctx);
  }`));

/// A page whose layout goes stale on every write; a measure of a stale layout counts as one forced layout.
function page(){
  const p = { dirty: false, forced: 0, reads: 0, animations: 0 };
  p.write = () => { p.dirty = true; };
  p.rect = (top = 10) => { p.reads++; if (p.dirty) { p.forced++; p.dirty = false; } return { top, bottom: top + 20, left: 0, width: 100, height: 20 }; };
  p.node = (top, extra = {}) => {
    let text = 'old', hidden = false;
    const style = new Proxy({}, { set(o, k, v){ p.write(); o[k] = v; return true; } });
    return Object.assign({
      isConnected: true, style,
      get textContent(){ return text; }, set textContent(v){ text = v; p.write(); },
      get hidden(){ return hidden; }, set hidden(v){ hidden = v; p.write(); },
      classList: { add: p.write, remove: p.write },
      getBoundingClientRect: () => p.rect(top),
      animate(){ p.animations++; p.write(); return { finished: Promise.resolve(), cancel(){} }; },
    }, extra);
  };
  return p;
}

globalThis.innerHeight = 800;
const { relabel, morph } = await import('./motion.js');

test('relabel measures every changed node before the first animation starts', async () => {
  const p = page();
  const nodes = Array.from({ length: 40 }, (_, i) => p.node(10 + i * 15));
  globalThis.document = { querySelectorAll: () => nodes };
  await relabel(() => { for (const n of nodes) n.textContent = 'new'; });
  assert.equal(p.animations, 40, 'every visible changed node turns over');
  assert.ok(p.forced <= 1, `the page was laid out ${p.forced} times for one switch; want at most 1`);
});

test('morph measures every card after the change before the first animation starts', async () => {
  const p = page();
  const host = { getBoundingClientRect: () => p.rect(0) };
  const cards = Array.from({ length: 30 }, (_, i) => p.node(10 + i * 25, { parentElement: host }));
  globalThis.__mfake = { $$: () => cards };
  // A third leave, the rest move up: what a category filter does.
  await morph(() => { cards.forEach((c, i) => { if (i % 3 === 0) c.hidden = true; }); cards.forEach((c, i) => { c.getBoundingClientRect = () => p.rect(5 + i * 20); }); });
  assert.ok(p.animations >= 20, `cards animated: ${p.animations}`);
  assert.ok(p.forced <= 1, `the page was laid out ${p.forced} times for one filter; want at most 1`);
});
