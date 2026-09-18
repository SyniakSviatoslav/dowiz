// Interaction gate — every control does something, and says so.
//
// The render gate proves a screen PAINTS. That is a different question from
// whether it WORKS. A button that is beautifully styled and wired to nothing
// passes every check in render.mjs, and is exactly what a customer reports as
// "I tapped it and nothing happened".
//
// So this gate taps things. For every control on every screen it asks four
// questions, and each one is a defect class that has shipped in real apps:
//
//   DEAD        — tapping it changes nothing observable. Not the route, not the
//                 DOM, not an attribute, not the focus, not the scroll.
//   MUTE        — it has no hover and no focus ring, so a pointer and a
//                 keyboard both get no confirmation that it is a control.
//   NAMELESS    — it has no accessible name, so a screen reader announces
//                 "button" and a voice-control user cannot address it.
//   NOT A TOGGLE— it carries aria-pressed/expanded/selected/checked and tapping
//                 it twice does NOT come back to where it started, which means
//                 the state and the markup have diverged.
//
// And after every state change it re-checks what the screen renders, because a
// state change is where a template starts printing `undefined`:
//
//   PLACEHOLDER — the words `undefined`, `NaN`, `[object Object]` or `Infinity`
//                 are visible on the screen.
//   DOUBLE      — two elements now share an id, which is how a double render
//                 announces itself before anyone notices the duplicated row.
//   SIDEWAYS    — the state change made the phone scroll horizontally.
//
// A no-op is not always a defect, and the two legitimate cases are named rather
// than filtered by feel: a `data-go` pointing at the screen already open, and a
// control that is disabled. Both are skipped with a reason.
//
// ENGINES. Chromium is the baseline. WebKit is the one that matters most for
// this app — it is what every iPhone runs — and Firefox is the third rendering
// model. Any engine that is not installed is reported as skipped, never
// silently dropped.
//
//   node e2e/kit-regression/interact.mjs
//   ENGINES=chromium,webkit  ROUTES=cart,home  node e2e/kit-regression/interact.mjs

import * as pw from 'playwright';

const HOST = process.env.HOST || 'https://dubin-sushi.dowiz.org';
// WHICH VENUE THE GATE IS LOOKING AT.
//
// `data.js` reads the venue from the hostname, and `localhost` names none — so
// against a local stand every screen fell back to the DESIGNER'S content and
// the gate tapped Burger, Pizza and three invented dishes. It passed, and it
// was judging a menu no customer will ever see. `SLUG=<venue>` puts the venue
// back in the query the same way the app's own `?s=` override does.
const SLUG = process.env.SLUG || '';
const KIT = SLUG ? `/kit/?s=${encodeURIComponent(SLUG)}#/` : '/kit/#/';
const WANT = (process.env.ENGINES || 'chromium,webkit,firefox').split(',').map(s => s.trim());
const ONLY_ROUTES = process.env.ROUTES ? process.env.ROUTES.split(',').map(s => s.trim()) : null;
const PER_SCREEN = Number(process.env.PER_SCREEN ?? 0);   // 0 = every control

/** The kit's own route table, read from the deploy rather than duplicated. */
async function routes() {
  const src = await (await fetch(`${HOST}/kit/app.js`)).text();
  const block = src.split('const SCREENS = {')[1].split('};')[0];
  const all = [...new Set([...block.matchAll(/^\s*'?([a-z-]+)'?:/gm)].map(m => m[1]))].sort();
  return ONLY_ROUTES ? all.filter(r => ONLY_ROUTES.includes(r)) : all;
}

// ── What runs inside the page ────────────────────────────────────────────────
// One script, installed once per page, so a thousand taps do not mean a
// thousand round trips of function source.
const PROBE = `
window.__ix = (() => {
  const SEL = 'button, a[href], input:not([type=hidden]), select, textarea,' +
              ' [role="button"], [role="radio"], [role="tab"], [role="switch"],' +
              ' [tabindex]:not([tabindex="-1"])';

  // A label that points at nothing is its own defect, checked separately — but
  // it is NOT a control. Tapping it operates its input, and that input is in the
  // list above, so listing both made the label focus the input and then reported
  // the input's own turn as a control that does nothing.
  const orphanLabels = () => [...document.querySelectorAll('label[for]')]
    .filter(l => !document.getElementById(l.htmlFor))
    .map(l => 'label for="' + l.htmlFor + '"');

  const visible = el => {
    const r = el.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) return false;
    const s = getComputedStyle(el);
    return s.visibility !== 'hidden' && s.display !== 'none' && s.opacity !== '0'
        && !el.closest('[hidden]');
  };

  const name = el => {
    if (el.getAttribute('aria-label')) return el.getAttribute('aria-label').trim();
    const by = el.getAttribute('aria-labelledby');
    if (by) return by.split(/\\s+/).map(id => document.getElementById(id)?.textContent || '')
                     .join(' ').trim();
    const own = (el.innerText || el.value || '').trim();
    if (own) return own;
    if (el.title) return el.title.trim();
    const img = el.querySelector('img[alt]');
    if (img && img.alt.trim()) return img.alt.trim();
    if (el.tagName === 'INPUT' && el.placeholder) return el.placeholder.trim();
    return '';
  };

  const label = el => el.tagName.toLowerCase()
    + (el.id ? '#' + el.id : '')
    + (el.className && typeof el.className === 'string'
        ? '.' + el.className.trim().split(/\\s+/)[0] : '');

  // A cheap content address of the whole document: catches a class flip, an
  // attribute change, a text change and an inserted node alike.
  const digest = () => {
    const s = document.documentElement.outerHTML;
    let h = 5381;
    for (let i = 0; i < s.length; i++) h = ((h * 33) ^ s.charCodeAt(i)) >>> 0;
    return h + ':' + s.length;
  };

  // Where an element is, not what it is called. Two unclassed buttons are both
  // "button", so a focus move between them was invisible; this tells them apart.
  const nodePath = el => {
    const bits = [];
    for (let n = el; n && n.nodeType === 1 && n !== document.body; n = n.parentElement) {
      let k = 1;
      for (let p = n.previousElementSibling; p; p = p.previousElementSibling)
        if (p.tagName === n.tagName) k++;
      bits.unshift(n.tagName.toLowerCase() + ':' + k);
    }
    return bits.join('/');
  };

  const snap = () => ({
    hash: location.hash,
    path: location.pathname,
    digest: digest(),
    nodes: document.getElementsByTagName('*').length,
    open: document.querySelectorAll(
      '[role="dialog"]:not([hidden]), .k-sheet:not([hidden]), .k-scrim:not([hidden])').length,
    focus: document.activeElement ? nodePath(document.activeElement) : '',
    scrollY: Math.round(window.scrollY),
    histLen: history.length,
  });

  // The visible words a template should never print.
  const BAD = /(^|[\\s>(\\[:,])(undefined|NaN|\\[object Object\\]|Infinity|null)($|[\\s<)\\].,!?])/;
  const placeholders = () => {
    const out = [];
    const walk = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    for (let n = walk.nextNode(); n; n = walk.nextNode()) {
      const t = n.nodeValue;
      if (!t || !t.trim() || !BAD.test(t)) continue;
      if (n.parentElement?.closest('script, style, #kit-sprite')) continue;
      if (!visible(n.parentElement)) continue;
      out.push(t.trim().slice(0, 60));
    }
    return [...new Set(out)];
  };

  const duplicateIds = () => {
    const seen = new Map();
    for (const el of document.querySelectorAll('[id]'))
      seen.set(el.id, (seen.get(el.id) || 0) + 1);
    return [...seen].filter(([, n]) => n > 1).map(([id, n]) => id + ' x' + n);
  };

  const sideways = () => {
    const d = document.documentElement;
    return d.scrollWidth > d.clientWidth + 1
      ? [...document.querySelectorAll('body *')].filter(el => {
          const r = el.getBoundingClientRect();
          return r.width > 0 && (r.right > d.clientWidth + 1 || r.left < -1);
        }).slice(0, 3).map(label)
      : [];
  };

  const health = () => ({
    placeholders: placeholders(), duplicateIds: duplicateIds(), sideways: sideways(),
    orphanLabels: orphanLabels(),
  });

  const STATES = ['aria-pressed', 'aria-expanded', 'aria-selected', 'aria-checked'];

  // The style properties that can carry a hover or focus affordance. Compared
  // as strings, so a colour that merely resolves the same is the same.
  const FEEL = ['outlineStyle', 'outlineWidth', 'outlineColor', 'boxShadow',
                'backgroundColor', 'color', 'borderColor', 'opacity', 'transform',
                'textDecorationLine', 'filter', 'borderBottomColor'];
  const one = el => { const s = getComputedStyle(el); return FEEL.map(p => s[p]).join('|'); };

  // The ring is not always on the control. A text input inside a pill gets it on
  // the PILL, and a checkbox gets it on the box drawn next to the real input.
  // Measuring only the control itself reported both of those as having no focus
  // affordance while a person could plainly see one, so the reading covers the
  // control, its parent and the parent's first few children.
  const feel = el => {
    const bits = [one(el)];
    const p = el.parentElement;
    if (p) { bits.push(one(p));
             for (const c of [...p.children].slice(0, 6)) if (c !== el) bits.push(one(c)); }
    return bits.join('||');
  };

  // The control list is captured ONCE and addressed by index, so a control that
  // re-renders under a new object is still the same entry in the report.
  let list = [];
  const info = i => {
    const el = list[i];
    return el ? shape(el, i) : null;
  };
  const collect = () => {
    list = [...document.querySelectorAll(SEL)].filter(visible);
    return list.map((el, i) => shape(el, i));
  };
  const shape = (el, i) => ({
      i, label: label(el), name: name(el),
      disabled: el.disabled === true || el.getAttribute('aria-disabled') === 'true',
      goto: el.dataset ? (el.dataset.go || '') : '',
      back: el.dataset ? ('back' in el.dataset) : false,
      href: el.tagName === 'A' ? el.getAttribute('href') : '',
      handoff: el.dataset ? (el.dataset.handoff || '') : '',
      state: STATES.filter(a => el.hasAttribute(a)).map(a => a + '=' + el.getAttribute(a)),
      // A tab, a radio and a single-select filter chip all sit in a group where
      // exactly one is on. Tapping the one that is already on is CORRECT and
      // changes nothing — so the group has to be recognised rather than the
      // no-op excused by hand. A group is siblings carrying the same state
      // attribute with exactly one of them true.
      group: (() => {
        const a = STATES.find(x => el.hasAttribute(x));
        if (!a || !el.parentElement) return 0;
        const sibs = [...el.parentElement.querySelectorAll('[' + a + ']')];
        const on = sibs.filter(s2 => s2.getAttribute(a) === 'true').length;
        // At most one on: that is single-select. ZERO on is the same group
        // before anything has been chosen — a time slot, a payment method — and
        // treating it as a toggle demanded that choosing a slot un-choose it.
        return sibs.length > 1 && on <= 1 ? sibs.length : 0;
      })(),
      on: STATES.some(a => el.getAttribute(a) === 'true'),
      type: (el.getAttribute('type') || '').toLowerCase(),
      tag: el.tagName,
    });
  const at = i => list[i] || null;

  // A SCREEN RE-RENDERS. Writing innerHTML on a pane, or outerHTML on a card,
  // replaces the very elements a captured list is pointing at, and a detached
  // node answers getAttribute forever without ever changing — so every control
  // after the first re-render reported as a stuck state, and the label printed
  // came from one element while the attribute came from another. Nonsense like
  // an input#q carrying aria-checked, in the report, is what that looks like.
  //
  // So the list is rebuilt at each step and the control is described from the
  // rebuilt one. Indices can shift when a tap changes how many controls exist;
  // that can skip or repeat a control, which is visible in the output rather
  // than silently wrong.
  const describe = i => { collect(); const c = info(i); return c; };

  // AN INDEX IS NOT AN IDENTITY.
  //
  // The list is rebuilt at every step, and anything that appears or disappears
  // between two steps — a sheet, an error line, the install banner on its timer
  // — shifts every index after it. The consequence was not a crash: it was the
  // SAME control being described at step n and step n+1, so the second tap of it
  // did nothing and it was reported as dead. A working control, named as broken.
  //
  // So a control is addressed by a fingerprint: what it is, what it is called,
  // and which of the identical ones it is. The walker returns the first one not
  // yet visited, together with its index in the list it just built.
  const fingerprint = (el, i, all) => {
    const base = label(el) + '|' + name(el).slice(0, 40);
    let nth = 0;
    for (let k = 0; k < i; k++)
      if (label(all[k]) + '|' + name(all[k]).slice(0, 40) === base) nth++;
    return base + '|' + nth;
  };
  const next = visited => {
    collect();
    const seen = new Set(visited);
    for (let i = 0; i < list.length; i++) {
      const fp = fingerprint(list[i], i, list);
      if (!seen.has(fp)) return { ...shape(list[i], i), fp };
    }
    return null;
  };

  return {
    collect, describe, next, snap, health, feel,
    // A CONTROL IS TAPPED WHERE IT IS, NOT WHERE ITS BOX SAYS.
    // getBoundingClientRect is in viewport coordinates, and a screen with 48
    // dishes puts most of its hearts at y=2000. Clicking there clicks nothing,
    // and every one of them reported as a dead control — a whole screen of
    // false findings from one missing scroll. So the element is brought on
    // screen first and the box is read after.
    box: i => { const el = at(i); if (!el) return null;
                if (!visible(el)) return null;
                const r0 = el.getBoundingClientRect();
                if (r0.top < 8 || r0.bottom > innerHeight - 8
                    || r0.left < 8 || r0.right > innerWidth - 8) {
                  // inline as well as block: a heart on a card in a
                  // horizontally scrolling rail is off to the right, and
                  // scrolling only vertically leaves it outside the viewport,
                  // where elementFromPoint answers nothing at all.
                  el.scrollIntoView({ block: 'center', inline: 'center', behavior: 'instant' });
                }
                let r = el.getBoundingClientRect();
                if (r.bottom < 0 || r.top > innerHeight) return null;
                let x = r.left + r.width / 2, y = r.top + r.height / 2;
                // IN VIEW IS NOT THE SAME AS REACHABLE.
                // A control can sit inside the viewport and still be underneath
                // the sticky bar — the price slider does, at the top of the
                // filter screen — and scrolling a little reveals it. So a point
                // that resolves to something else is scrolled to and asked
                // again, and only a control that is STILL covered after that is
                // reported. Otherwise the gate reports the fixed bar as a
                // defect on every screen that has one.
                const reaches = () => {
                  const h = document.elementFromPoint(x, y);
                  return !!h && (h === el || el.contains(h) || h.contains(el)
                    || (el.closest('label') && el.closest('label').contains(h)));
                };
                if (!reaches()) {
                  el.scrollIntoView({ block: 'center', inline: 'center', behavior: 'instant' });
                  r = el.getBoundingClientRect();
                  x = r.left + r.width / 2; y = r.top + r.height / 2;
                }
                // WHO IS ACTUALLY AT THAT POINT. A tap that lands on something
                // else has not tested this control, and calling that a dead
                // control is the worst thing a gate can do — it sends someone
                // to fix code that works. So the point is resolved here and a
                // miss is reported as a miss, with the name of whatever was on
                // top of it.
                const hit = document.elementFromPoint(x, y);
                const owns = !!hit && (hit === el || el.contains(hit) || hit.contains(el)
                  || (el.closest('label') && el.closest('label').contains(hit)));
                return { x, y, owns, over: hit ? label(hit) : 'nothing' }; },
    baseline: i => { const el = at(i); return el ? feel(el) : ''; },
    focusFeel: i => { const el = at(i); if (!el) return ''; el.focus(); return feel(el); },
    blur: i => { const el = at(i); if (el) el.blur(); },
    nodePath: i => { const el = at(i); return el ? nodePath(el) : ''; },
    states: i => { const el = at(i); return el
      ? STATES.filter(a => el.hasAttribute(a)).map(a => a + '=' + el.getAttribute(a)) : []; },
    route: () => (location.hash.replace(/^#\\/?/, '').split('?')[0] || 'home'),
  };
})();
`;

const results = [];
const note = (engine, route, kind, detail) => results.push({ engine, route, kind, detail });

// EVERY SKIP IS PRINTED. A gate that quietly declines to judge a control is a
// gate that reports "no defects" about work it never did, so the hand-offs it
// chose not to tap are listed under the report rather than dropped.
const skipped = [];

// ── One screen, one engine ───────────────────────────────────────────────────
async function screen(ctx, engine, route) {
  const page = await ctx.newPage();
  const consoleErrors = [];
  page.on('console', m => {
    if (m.type() !== 'error') return;
    const t = m.text();
    // Cloudflare's edge-injected inline script is blocked by our own policy.
    // Ours ships no inline script at all; this is the edge's, and render.mjs
    // classifies it the same way.
    if (/Content Security Policy/.test(t) && /inline script/i.test(t)) return;
    consoleErrors.push(t);
  });
  page.on('pageerror', e => consoleErrors.push('pageerror: ' + e.message));

  // The install banner appears on a timer and is a dialog over every screen, so
  // it both perturbs the control list and covers what is at the bottom. It has
  // its own gate in pwa.mjs; here it is snoozed exactly the way a person
  // snoozing it does.
  await page.addInitScript(() => {
    try { localStorage.setItem('dowiz.install.snoozed', String(Date.now())); } catch {}
  });

  const open = async () => {
    // Home first, then the screen. Otherwise `data-back` has no previous entry
    // and falls through to its own home fallback, which on the home screen
    // itself changes nothing and reads as a dead button.
    await page.goto(`${HOST}${KIT}home`, { waitUntil: 'load' });
    await page.goto(`${HOST}${KIT}${route}`, { waitUntil: 'load' });
    // WAIT FOR THE SCREEN, NOT FOR ANY SCREEN.
    //
    // Screens are dynamically imported, so between the hash changing and the
    // markup arriving the app still holds the PREVIOUS screen. A wait for "the
    // app has children" was satisfied by that previous screen, so the gate
    // collected controls that were about to be replaced — and getComputedStyle
    // on a detached element returns EMPTY STRINGS for every property, so every
    // control on the screen measured as having no focus ring. Ten false
    // findings on one screen, all from a wait that was too weak.
    //
    // `#app[data-route]` is set by the shell after the markup AND the bindings,
    // so this waits for the screen to be finished.
    //
    // A FUNCTION, not a string: Playwright evaluates a string predicate through
    // the page's own eval, which `script-src 'self'` forbids.
    await page.waitForFunction(
      want => document.getElementById('app')?.dataset.route === want,
      route, { timeout: 20000 }).catch(() => {});
    await page.evaluate(PROBE);
    // A few screens fetch a menu after they mount, so the markup settles a
    // moment after data-route appears. Waited for by watching it stop changing
    // rather than by picking a number.
    await page.waitForFunction(() => {
      const a = document.getElementById('app');
      if (!a) return false;
      const n = a.innerHTML.length;
      if (window.__settle === n) return true;
      window.__settle = n;
      return false;
    }, null, { timeout: 8000, polling: 200 }).catch(() => {});
  };
  await open();

  const controls = await page.evaluate('window.__ix.collect()');
  const health0 = await page.evaluate('window.__ix.health()');
  for (const p of health0.placeholders) note(engine, route, 'PLACEHOLDER', `on load: ${p}`);
  for (const d of health0.duplicateIds) note(engine, route, 'DOUBLE', `on load: id ${d}`);
  for (const l of health0.orphanLabels || [])
    note(engine, route, 'ORPHAN LABEL', `${l} points at no element`);

  // ── Names and affordances: no clicking needed ─────────────────────────────
  // A keyboard press first, so Chromium and WebKit treat the programmatic focus
  // that follows as keyboard modality and apply :focus-visible. Without it a
  // focus ring that IS defined measures as absent.
  await page.keyboard.press('Tab').catch(() => {});
  // That Tab lands on the first control, so its baseline would be read while it
  // is focused and it would measure as having no ring. Chromium and WebKit keep
  // the keyboard modality after the blur, which is the part that was needed.
  await page.evaluate('document.activeElement && document.activeElement.blur()');
  for (const c of controls) {
    if (!c.name && !c.disabled) note(engine, route, 'NAMELESS', c.label);
    // A <label> cannot take focus, so it cannot show a focus ring. It is in the
    // list because tapping it operates its input — and that input is checked on
    // its own turn. Demanding a ring here reported three labels per form.
    if (c.tag === 'LABEL' || c.disabled) continue;
    const base = await page.evaluate(`window.__ix.baseline(${c.i})`);
    const focused = await page.evaluate(`window.__ix.focusFeel(${c.i})`);
    await page.evaluate(`window.__ix.blur(${c.i})`);
    if (process.env.IXDEBUG)
      console.log(`    [dbg] ${c.label} base=${String(base).slice(0, 70)} foc=${String(focused).slice(0, 70)}`);
    if (base && focused && base === focused)
      note(engine, route, 'MUTE', `${c.label} — no focus ring`);
  }

  // ── Taps ─────────────────────────────────────────────────────────────────
  const cap = PER_SCREEN || controls.length + 12;   // +12: a tap can reveal more
  const visited = [];
  for (let step = 0; step < cap; step++) {
    // The next control nobody has tapped yet, from a list rebuilt right now.
    const c = await page.evaluate(v => window.__ix.next(v), visited);
    if (!c) break;                            // every control has had its turn
    visited.push(c.fp);
    if (process.env.IXDEBUG) console.log(`    [step ${step}] fp=${c.fp} i=${c.i}`);
    if (c.disabled) continue;
    if (c.href && /^(https?:|mailto:|tel:|sms:)/.test(c.href)) continue;   // leaves the app
    // A CONTROL MAY HAND OFF TO THE PLATFORM, and then the browser — not the
    // page — is what answers. Opening the camera roll changes no DOM, so the
    // dead check cannot see it and would report every photo picker in the app.
    //
    // This is a skip with EVIDENCE, not a label anyone can paste onto a dead
    // button: `data-handoff="file"` is honoured only when the screen really
    // contains the `input[type=file]` such a button must be opening. A button
    // that claims the hand-off without one is still reported.
    if (c.handoff === 'file') {
      const hasPicker = await page.evaluate(
        `!!document.querySelector('input[type=file]')`).catch(() => false);
      if (hasPicker) { skipped.push(`${route}: ${c.label} — hands off to the file picker`); continue; }
      note(engine, route, 'FALSE HANDOFF',
        `${c.label} claims data-handoff="file" and the screen has no input[type=file]`);
      continue;
    }
    if (c.goto && c.goto === route) continue;                          // already here, by design
    if (c.group && c.on) continue;            // the selected tab of a group; a no-op is right

    const box = await page.evaluate(`window.__ix.box(${c.i})`);
    if (!box) continue;                       // a previous tap hid it; not its turn
    if (!box.owns) {
      note(engine, route, 'COVERED',
        `${c.label} — ${box.over} is on top of its centre` +
        (process.env.IXDEBUG ? ` @${Math.round(box.x)},${Math.round(box.y)}` : ''));
      continue;
    }

    // AFTER the scroll box() may have done, or the scroll itself reads as the
    // change that proves the control works.
    const before = await page.evaluate('window.__ix.snap()');
    const statesBefore = await page.evaluate(`window.__ix.states(${c.i})`);
    const pageUrlBefore = page.url();

    // Tap by coordinate, so what is measured is what a thumb reaches — a
    // control covered by a fixed bar fails here and not in a synthetic click.
    try {
      await page.mouse.click(box.x, box.y, { delay: 20 });
    } catch (e) { note(engine, route, 'UNTAPPABLE', `${c.label} — ${e.message}`); continue; }
    await page.waitForTimeout(220);           // transitions and a dynamic import

    const after = await page.evaluate('window.__ix.snap()').catch(() => null);
    if (!after) {
      // A CONTROL THAT LEAVES THE APP IS NOT A CRASH. `window.__ix` lives in the
      // document, so a full-page navigation — the Contact Us row that opens the
      // venue's own storefront at `/` — takes the probe with it, and the gate
      // read the missing probe as the page having died. The two are told apart
      // by the only evidence that survives: the browser's address bar. A URL
      // that moved is the strongest possible proof the control did something.
      const now = page.url();
      if (now !== pageUrlBefore) { await open(); await page.evaluate('window.__ix.collect()'); continue; }
      note(engine, route, 'CRASHED', c.label); await open(); continue;
    }

    // FOCUS ON THE THING YOU JUST TAPPED IS NOT AN EFFECT.
    //
    // A browser focuses a button when it is clicked. Counting that as "something
    // happened" would mean no button in the app could ever be reported as dead,
    // which is the one thing this gate exists to find. For a text field it is
    // the opposite: accepting focus IS what the control is for.
    const self = await page.evaluate(`window.__ix.nodePath(${c.i})`);
    const focusMoved = after.focus !== before.focus;
    const focusIsItself = after.focus === self;
    const takesFocus = c.tag === 'INPUT' || c.tag === 'TEXTAREA' || c.tag === 'SELECT';
    const moved = after.hash !== before.hash || after.path !== before.path
      || after.digest !== before.digest || after.open !== before.open
      || after.scrollY !== before.scrollY
      || (focusMoved && (takesFocus || !focusIsItself));
    if (!moved) {
      if (process.env.IXDEBUG)
        console.log(`    [dead] ${c.label} "${c.name.slice(0, 20)}" at ${JSON.stringify(box)}` +
          `\n      before ${JSON.stringify(before)}\n      after  ${JSON.stringify(after)}`);
      note(engine, route, 'DEAD', `${c.label}${c.name ? ` "${c.name.slice(0, 24)}"` : ''}`);
    }

    // A control that claims a state must return to it on a second tap — unless
    // it is one of a group, where selecting the selected one is correct.
    if (statesBefore.length && !c.group && after.hash === before.hash) {
      const mid = await page.evaluate(`window.__ix.states(${c.i})`);
      if (mid.join() === statesBefore.join())
        note(engine, route, 'STATE STUCK', `${c.label} ${statesBefore.join()} did not change`);
      else {
        const b2 = await page.evaluate(`window.__ix.box(${c.i})`);
        if (b2) {
          await page.mouse.click(b2.x, b2.y, { delay: 20 });
          await page.waitForTimeout(180);
          const back = await page.evaluate(`window.__ix.states(${c.i})`);
          if (back.join() !== statesBefore.join())
            note(engine, route, 'NOT A TOGGLE',
              `${c.label} ${statesBefore.join()} → ${mid.join()} → ${back.join()}`);
        }
      }
    }

    // After a state change, the screen still has to be a screen.
    // A group member that was not selected must become selected.
    if (c.group && !c.on && after.hash === before.hash) {
      const now2 = await page.evaluate(`window.__ix.states(${c.i})`);
      if (now2.join() === statesBefore.join())
        note(engine, route, 'STATE STUCK',
          `${c.label} ${statesBefore.join()} — one of ${c.group}, choosing it changed nothing`);
    }

    const h = await page.evaluate('window.__ix.health()').catch(() => null);
    if (h) {
      for (const p of h.placeholders)
        if (!health0.placeholders.includes(p))
          note(engine, route, 'PLACEHOLDER', `after ${c.label}: ${p}`);
      for (const d of h.duplicateIds)
        if (!health0.duplicateIds.includes(d))
          note(engine, route, 'DOUBLE', `after ${c.label}: id ${d}`);
      if (h.sideways.length && !health0.sideways.length)
        note(engine, route, 'SIDEWAYS', `after ${c.label}: ${h.sideways.join(', ')}`);
    }

    // Back to this screen if the tap left it, so each control is judged from
    // the same starting state rather than from the wreckage of the last one.
    const now = await page.evaluate('window.__ix.route()').catch(() => null);
    if (now !== route) {
      await open();
      // PROBE is reinstalled by open(), so its control list is empty until it
      // is rebuilt. Without this every control after the first navigation was
      // silently skipped, and a gate that skips quietly is a gate that lies.
      await page.evaluate('window.__ix.collect()');
    }
  }

  for (const e of [...new Set(consoleErrors)].slice(0, 2))
    note(engine, route, 'CONSOLE', e.slice(0, 160));

  await page.close();
  return controls.length;
}

// ── The run ──────────────────────────────────────────────────────────────────
export async function run() {
  const list = await routes();
  let ran = 0;

  // ENGINES ARE LAUNCHED ONE AT A TIME, immediately before their own pass.
  // Launching all three up front and then using them in turn is what made this
  // gate crash: the second browser sat idle for the length of the first
  // engine's pass — an hour over seventy screens — and was dead by its turn,
  // reported as `browser.newContext: Target page, context or browser has been
  // closed` AFTER the first engine's results had already been printed. A run
  // that dies between engines loses the report for the engine that passed.
  for (const name of WANT) {
    const type = pw[name];
    if (!type) { console.log(`SKIP  ${name} — not a playwright engine`); continue; }
    let browser;
    try {
      browser = await type.launch({
        args: name === 'chromium' ? ['--no-sandbox', '--disable-dev-shm-usage'] : undefined,
      });
    } catch (e) {
      // A missing engine is reported, never skipped silently: "it passes" must
      // never mean "it did not run".
      console.log(`SKIP  ${name} — not installed (${String(e.message).split('\n')[0]})`);
      note(name, '-', 'ENGINE MISSING', 'npx playwright install ' + name);
      continue;
    }
    ran++;
    const ctx = await browser.newContext({
      viewport: { width: 390, height: 844 },
      deviceScaleFactor: 2,
      hasTouch: true,
      isMobile: name === 'chromium' || name === 'webkit',   // firefox rejects isMobile
    });
    let controls = 0;
    console.log(`\n── ${name}`);
    for (const route of list) {
      const before = results.length;
      try { controls += await screen(ctx, name, route); }
      catch (e) { note(name, route, 'GATE ERROR', String(e.message).split('\n')[0]); }
      const found = results.length - before;
      console.log(`${found ? 'FAIL' : 'ok  '}  ${route}${found ? `  (${found})` : ''}`);
    }
    console.log(`${name}: ${controls} controls tapped across ${list.length} screens`);
    await ctx.close();
    await browser.close();
  }
  if (!ran) { console.log('no engine could be launched'); return 1; }

  // ── The report, grouped by defect rather than by screen ───────────────────
  if (skipped.length) {
    console.log(`\n${skipped.length} control(s) skipped, by name:`);
    for (const line of skipped) console.log(`  ${line}`);
  }
  if (!results.length) { console.log('\nno interaction defects'); return 0; }
  const byKind = new Map();
  for (const r of results) {
    const key = `${r.kind}\t${r.detail}`;
    if (!byKind.has(key)) byKind.set(key, []);
    byKind.get(key).push(`${r.engine}/${r.route}`);
  }
  console.log('');
  for (const [key, where] of [...byKind].sort()) {
    const [kind, detail] = key.split('\t');
    console.log(`${kind.padEnd(13)} ${detail}`);
    console.log(`  ${where.length} place(s): ${where.slice(0, 4).join(', ')}` +
                (where.length > 4 ? ` …` : ''));
  }
  console.log(`\n${byKind.size} distinct interaction defect(s)`);
  return byKind.size;
}

if (import.meta.url === `file://${process.argv[1]}`) process.exit(await run());
