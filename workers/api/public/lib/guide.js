// dowiz guide -- the first-run tour and the per-control hints, ONE module for
// every surface.
//
// WHY ONE TABLE. A surface hands this module a single declarative table of
// { at, title, body } rows keyed by name. The tour is an ORDERED LIST OF KEYS
// into that table, and a hint is a row that names an element -- so a control's
// explanation lives in exactly one place. The copy the tour reads out is the
// copy the "?" beside the control opens, and neither can drift from the other,
// which is the rule this codebase has about two lists of the same thing.
//
// WHY IT INJECTS ITS OWN <link>. The index.html files are not this module's to
// edit, and the CSP is `style-src 'self'` -- an inline <style> would be dropped
// without a word. A same-origin <link> is what the policy allows, so the module
// adds one, once, before it draws anything.
//
// WHAT IT REUSES. Toasts: the surface passes its own toast() and the guide
// speaks through it, so there is one notice mechanism on the page rather than
// two. Icons: the `.ti` masks in /lib/icons.css. Measurements: T2 tokens,
// read from the computed style so the ring's inset and the card's gutter are
// the same --space-* every other control uses.
//
// PERSISTENCE. localStorage `dw_guide_<key>` = { state, step }:
//   (absent)   never seen           -> autoStart() opens the tour at step 0
//   'open'     reloaded mid-tour    -> resumes where it was; nobody dismissed it
//   'paused'   "завершити пізніше"  -> stays closed; the help button carries a
//                                      dot and reopens at the saved step
//   'done'     finished or skipped  -> stays closed; help reopens from step 0
//
// A STEP WHOSE ELEMENT IS NOT ON SCREEN IS SKIPPED, in the direction of
// travel. The console's find bar only exists once the queue has loaded and the
// courier's microphone only exists where the browser has a recogniser; a tour
// that pointed at nothing would be worse than one step shorter.

const CSS_HREF = '/lib/guide.css';
const ICON = name => `<i class="ti ti-${name}" aria-hidden="true"></i>`;
const calm = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
const clamp = (v, lo, hi) => Math.min(Math.max(v, lo), hi);

function q(sel){ try { return sel ? document.querySelector(sel) : null; } catch { return null; } }
// display:none, a [hidden] ancestor, a detached node -- all have no client rects.
const shown = el => Boolean(el && el.isConnected && el.getClientRects().length);

// A token read once, as a number of px. The guide positions with the same
// spacing scale the page is laid out on rather than a private set of magic numbers.
const tokCache = {};
function tok(name, fallback){
  if (!(name in tokCache)) {
    const v = parseFloat(getComputedStyle(document.documentElement).getPropertyValue(name));
    tokCache[name] = Number.isFinite(v) ? v : fallback;
  }
  return tokCache[name];
}

function ensureCss(){
  if (document.querySelector(`link[href="${CSS_HREF}"]`)) return;
  const l = document.createElement('link');
  l.rel = 'stylesheet'; l.href = CSS_HREF;
  document.head.appendChild(l);
}

function box(el, left, top, width, height){
  el.style.left = `${Math.round(left)}px`; el.style.top = `${Math.round(top)}px`;
  el.style.width = `${Math.round(width)}px`; el.style.height = `${Math.round(height)}px`;
}

/// Build a guide for one surface.
///
///   key    -- storage key suffix ('owner', 'courier')
///   help   -- { name: { at, title, body, hint, place } }
///             at:    selector of the element the row explains (optional for a
///                    tour-only row such as a welcome)
///             body:  a string, or a function returning one -- read at show
///                    time, so a row can describe live state
///             hint:  false to keep the row out of the "?" affordances
///             place: 'after' | 'append' -- where the "?" goes; the default puts
///                    it after a control and inside a container
///   tour   -- ordered list of keys, or { key, ...overrides } objects; an
///             override may carry `at` (a wider anchor than the hint's) and
///             `before` (a function that brings the element on screen)
///   mount  -- the help entry: { into, before, className, text, when, tour }
///   toast  -- the surface's own toast(message)
///   onEnd  -- called with 'done' | 'paused' when the tour closes
/// The guide's own words. A caller passes its language's; these are the defaults.
const WORDS = { step: (i, n) => `Крок ${i} з ${n}`, skip: 'Пропустити', later: 'Завершити пізніше', back: 'Назад', next: 'Далі', done: 'Готово',
  paused: 'Тур збережено — «Довідка» продовжить з цього місця', skipped: 'Тур пропущено — його завжди можна відкрити через «Довідка»',
  finished: 'Готово. Маленькі «?» біля елементів пояснюють кожен окремо', unfinished: 'Тур не завершено — «Довідка» продовжить з того ж місця',
  what: 'Що це', close: 'Закрити', help: 'Довідка' };
export function createGuide({ key, help = {}, tour = [], mount = null, toast = () => {}, onEnd = null, words = {} } = {}){
  const W = { ...WORDS, ...words };
  const SK = `dw_guide_${key}`;
  const read = () => { try { const v = JSON.parse(localStorage.getItem(SK) || 'null'); return v && typeof v === 'object' ? v : null; } catch { return null; } };
  const write = v => { try { localStorage.setItem(SK, JSON.stringify(v)); } catch {} };

  const steps = tour.map(s => {
    const o = typeof s === 'string' ? { key: s } : s;
    return { ...(help[o.key] || {}), ...o };
  });

  let ring = null, card = null, pop = null;
  let cur = -1, open = false, lastFocus = null, popFor = null;
  let mo = null, raf = 0;

  function build(){
    if (ring) return;
    ensureCss();
    ring = document.createElement('div'); ring.className = 'gd-ring'; ring.hidden = true;
    card = document.createElement('div'); card.className = 'gd-card'; card.hidden = true;
    card.setAttribute('role', 'dialog'); card.tabIndex = -1;
    pop = document.createElement('div'); pop.className = 'gd-pop'; pop.hidden = true;
    pop.id = `gd-pop-${key}`; pop.setAttribute('role', 'dialog');
    document.body.append(ring, card, pop);
  }

  const text = row => typeof row.body === 'function' ? String(row.body() ?? '') : String(row.body ?? '');

  // ── the tour ──────────────────────────────────────────────────────────────
  async function show(i, dir = 1){
    build();
    if (i >= steps.length) return end('done');
    if (i < 0) return;
    const s = steps[i];
    try { await s.before?.(); } catch {}
    const target = s.at ? q(s.at) : null;
    // `soft`: a lesson step explains a control that may be on another screen
    // (the cash field exists only at the door). It is shown in the middle
    // instead of skipped, so a lesson opened while standing still is read
    // through rather than finishing at once with nothing seen.
    if (s.at && !shown(target) && !s.soft) return show(i + dir, dir);
    cur = i; open = true;
    write({ state: 'open', step: i });
    renderCard(s, i);
    ring.hidden = false; card.hidden = false;
    if (target) target.scrollIntoView({ block: 'center', inline: 'nearest', behavior: calm() ? 'auto' : 'smooth' });
    place();
    card.querySelector('[data-gd-act="next"]').focus({ preventScroll: true });
  }

  function renderCard(s, i){
    const n = steps.length, last = i + 1 === n;
    card.innerHTML = `
      <button type="button" class="gd-x" aria-label="${W.later}">${ICON('x')}</button>
      <div class="gd-n">${W.step(i + 1, n)}</div>
      <h2 id="gd-title-${key}"></h2>
      <p></p>
      <div class="gd-acts">
        ${last ? '' : `<button type="button" class="gd-btn gd-quiet" data-gd-act="skip">${W.skip}</button>`}
        <button type="button" class="gd-btn gd-quiet" data-gd-act="later">${W.later}</button>
        <span class="gd-sp"></span>
        ${i > 0 ? `<button type="button" class="gd-btn" data-gd-act="back">${W.back}</button>` : ''}
        <button type="button" class="gd-btn gd-pri" data-gd-act="next">${last ? W.done : W.next}</button>
      </div>`;
    card.setAttribute('aria-labelledby', `gd-title-${key}`);
    card.querySelector('h2').textContent = s.title || '';
    card.querySelector('p').textContent = text(s);
    // Behaviour hangs off data-gd-act, not off a class: a class is a promise
    // of a rule, and these four carry none of their own.
    const ACT = { skip: () => end('skip'), later: () => end('paused'),
                  back: () => show(cur - 1, -1), next: () => show(cur + 1, 1) };
    card.querySelector('.gd-x').onclick = () => end('paused');
    card.querySelectorAll('[data-gd-act]').forEach(b => { b.onclick = ACT[b.dataset.gdAct]; });
  }

  function place(){
    if (!open) return;
    const s = steps[cur];
    const t = s.at ? q(s.at) : null;
    const vw = innerWidth, vh = innerHeight;
    const m = tok('--space-4', 16), g = tok('--space-3', 12), p = tok('--space-2', 8);
    const cw = card.offsetWidth, ch = card.offsetHeight;
    let left, top;
    if (shown(t)) {
      const r = t.getBoundingClientRect();
      ring.classList.remove('gd-none');
      box(ring, r.left - p, r.top - p, r.width + 2 * p, r.height + 2 * p);
      left = clamp(r.left, m, Math.max(m, vw - cw - m));
      if (r.bottom + g + ch <= vh - m) top = r.bottom + g;            // below
      else if (r.top - g - ch >= m) top = r.top - g - ch;              // above
      else top = Math.max(m, vh - ch - m);                              // neither fits: pin to the bottom
    } else {
      // No target (a welcome), or the target left the screen mid-tour: the
      // scrim stays, the card sits in the middle.
      ring.classList.add('gd-none');
      box(ring, vw / 2, vh / 2, 0, 0);
      left = Math.max(m, (vw - cw) / 2); top = Math.max(m, (vh - ch) / 2);
    }
    card.style.left = `${Math.round(left)}px`; card.style.top = `${Math.round(top)}px`;
  }

  function end(how){
    if (!open) return;
    open = false; ring.hidden = true; card.hidden = true;
    const state = how === 'paused' ? 'paused' : 'done';
    write({ state, step: state === 'paused' ? cur : 0 });
    syncHelp();
    if (how === 'paused') toast(W.paused);
    else if (how === 'skip') toast(W.skipped);
    else toast(W.finished);
    try { lastFocus?.focus?.({ preventScroll: true }); } catch {}
    onEnd?.(state);
  }

  function start(i = 0){
    build();
    if (!open) lastFocus = document.activeElement;
    show(i, 1);
  }

  // First visit opens the tour. A paused one is not reopened by itself -- the
  // owner said "later", and a tour that comes back on every load is a nag --
  // but it is mentioned once per tab so "later" stays findable.
  function autoStart(){
    const v = read();
    if (!v) return start(0);
    if (v.state === 'open') return start(v.step || 0);
    if (v.state === 'paused') {
      syncHelp();
      try {
        if (!sessionStorage.getItem(`${SK}_said`)) {
          sessionStorage.setItem(`${SK}_said`, '1');
          toast(W.unfinished);
        }
      } catch {}
    }
  }

  // ── the help entry ────────────────────────────────────────────────────────
  function mountHelp(){
    if (!mount) return;
    const into = q(mount.into); if (!into) return;
    if (into.querySelector('.gd-help')) return;
    if (mount.when && !mount.when()) return;
    const b = document.createElement('button');
    b.type = 'button';
    b.className = 'gd-help' + (mount.className ? ` ${mount.className}` : '');
    if (mount.tour) b.dataset.tour = mount.tour;   // the learning anchor (`data-tour`)
    b.setAttribute('aria-label', W.help); b.title = W.help;
    b.innerHTML = ICON('help-circle');
    if (mount.text) { const s = document.createElement('span'); s.textContent = mount.text; b.appendChild(s); }
    b.onclick = () => {
      if (open) return end('paused');
      const v = read();
      start(v?.state === 'paused' ? (v.step || 0) : 0);
    };
    const before = mount.before ? into.querySelector(mount.before) : null;
    before ? into.insertBefore(b, before) : into.appendChild(b);
    syncHelp();
  }
  function syncHelp(){
    const paused = read()?.state === 'paused';
    document.querySelectorAll('.gd-help').forEach(b => b.classList.toggle('gd-resume', paused));
  }

  // ── the "?" beside a control ──────────────────────────────────────────────
  // Idempotent: a target is marked when its "?" is placed, and a re-render that
  // replaces the target replaces the mark with it, so the observer below can
  // call this after every DOM change and only ever add what is missing.
  function mountHints(){
    build();
    for (const [name, row] of Object.entries(help)) {
      if (!row.at || row.hint === false) continue;
      const el = q(row.at);
      if (!el || el.dataset.gdHinted) continue;
      el.dataset.gdHinted = name;
      const b = document.createElement('button');
      b.type = 'button'; b.className = 'gd-hint'; b.dataset.gd = name;
      b.setAttribute('aria-label', `${W.what}: ${row.title || name}`);
      b.setAttribute('aria-expanded', 'false');
      b.setAttribute('aria-controls', pop.id);
      b.innerHTML = ICON('help-circle');
      b.onclick = e => { e.stopPropagation(); popFor === b ? closePop() : openPop(b, row); };
      placeHint(el, b, row.place);
    }
  }
  // A control gets its "?" after it (a button cannot contain a button, and a
  // field's "?" belongs beside the field, not in it); a container gets it
  // inside. An input wrapped in a <label> is placed after the label, so the
  // label's own click target is not split.
  function placeHint(el, b, place){
    const control = /^(BUTTON|INPUT|SELECT|TEXTAREA|A|LABEL|IMG|CANVAS|IFRAME)$/.test(el.tagName);
    if (place === 'append' || (!place && !control)) { el.appendChild(b); return; }
    const host = (el.tagName === 'INPUT' || el.tagName === 'SELECT') && el.closest('label') || el;
    host.after(b);
  }

  function openPop(btn, row){
    if (popFor) popFor.setAttribute('aria-expanded', 'false');
    popFor = btn;
    btn.setAttribute('aria-expanded', 'true');
    pop.innerHTML = `<button type="button" class="gd-x" aria-label="${W.close}">${ICON('x')}</button><b id="${pop.id}-t"></b><p></p>`;
    pop.setAttribute('aria-labelledby', `${pop.id}-t`);
    pop.querySelector('b').textContent = row.title || '';
    pop.querySelector('p').textContent = text(row);
    pop.querySelector('.gd-x').onclick = closePop;
    pop.hidden = false;
    placePop();
  }
  function placePop(){
    if (!popFor) return;
    if (!shown(popFor)) return closePop();          // its control was re-rendered away
    const r = popFor.getBoundingClientRect();
    const m = tok('--space-4', 16), g = tok('--space-2', 8);
    const w = pop.offsetWidth, h = pop.offsetHeight, vw = innerWidth, vh = innerHeight;
    const left = clamp(r.left + r.width / 2 - w / 2, m, Math.max(m, vw - w - m));
    const top = r.bottom + g + h <= vh - m ? r.bottom + g : Math.max(m, r.top - g - h);
    pop.style.left = `${Math.round(left)}px`; pop.style.top = `${Math.round(top)}px`;
  }
  function closePop(){
    if (!popFor) return;
    const b = popFor; popFor = null;
    b.setAttribute('aria-expanded', 'false');
    const hadFocus = pop.contains(document.activeElement);
    pop.hidden = true;
    if (hadFocus && shown(b)) b.focus({ preventScroll: true });
  }

  // ── events ────────────────────────────────────────────────────────────────
  function onKey(e){
    if (e.key === 'Escape' && popFor) { e.preventDefault(); return closePop(); }
    if (!open) return;
    if (e.key === 'Escape') { e.preventDefault(); return end('paused'); }
    if (e.key === 'ArrowRight') { e.preventDefault(); return show(cur + 1, 1); }
    if (e.key === 'ArrowLeft') { e.preventDefault(); return show(cur - 1, -1); }
    if (e.key === 'Tab') {
      // Focus stays in the card. The page beneath is still clickable -- that is
      // deliberate -- but a keyboard user tabbing out of a dialog is lost.
      const f = [...card.querySelectorAll('button')];
      if (!f.length) return;
      const first = f[0], last = f[f.length - 1], a = document.activeElement;
      if (!card.contains(a)) { first.focus(); e.preventDefault(); }
      else if (e.shiftKey && a === first) { last.focus(); e.preventDefault(); }
      else if (!e.shiftKey && a === last) { first.focus(); e.preventDefault(); }
    }
  }
  function onDown(e){
    if (!popFor) return;
    if (pop.contains(e.target) || popFor.contains(e.target)) return;
    closePop();
  }
  // Two rhythms. A DOM change may have re-rendered a pane, so the affordances
  // are re-mounted; a scroll or a resize moved things, so only the positions
  // are redone -- a scroll must not walk the table on every frame.
  let dirty = false;
  function schedule(remount){
    dirty = dirty || Boolean(remount);
    if (raf) return;
    raf = requestAnimationFrame(() => {
      raf = 0;
      if (dirty) { dirty = false; mountHelp(); mountHints(); }
      if (open) place();
      placePop();
    });
  }
  const remount = () => schedule(true);
  const reflow = () => schedule(false);

  /// Wire the guide to the page: draws the help entry and the "?" affordances
  /// now, and again after every DOM change, so a re-rendered pane keeps them.
  function init(){
    build();
    if (!mo) {
      mo = new MutationObserver(remount);
      mo.observe(document.body, { childList: true, subtree: true });
      addEventListener('resize', reflow, { passive: true });
      addEventListener('scroll', reflow, { capture: true, passive: true });
      document.addEventListener('keydown', onKey);
      document.addEventListener('pointerdown', onDown);
    }
    schedule(true);
  }

  /// Open the tour at the step whose key is `name` (a lesson deep link names a
  /// step, not an index); an unknown name opens at step 0.
  const startAt = name => start(Math.max(0, steps.findIndex(s => s.key === name)));

  return { init, autoStart, start, startAt, mountHints, mountHelp,
           pause: () => end('paused'), get open(){ return open; } };
}
