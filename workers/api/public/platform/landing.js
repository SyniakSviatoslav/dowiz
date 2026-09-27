// dowiz.org — the front door's motion and its languages.
//
// GSAP, ScrollTrigger, SplitText and Lenis are vendored under /lib/vendor and
// loaded as classic scripts before this module (script-src 'self'). This file
// only reads their globals. Every animation has a still frame: with reduced
// motion, or without the libraries, the page is complete and readable.

import { T } from './landing-words.js';
import { MEDIA_LANGS, pickLang as pickFirst } from '../lib/langs.js';

const gsap = window.gsap;
const ScrollTrigger = window.ScrollTrigger;
const SplitText = window.SplitText;
const Lenis = window.Lenis;
const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => Array.from(r.querySelectorAll(s));
const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
const finePointer = matchMedia('(pointer: fine)').matches;
const DESK = '(min-width: 56rem)';
const motion = !!gsap && !reduced;

// ── the words: platform/landing-words.js, one table per language ──────────

// SplitText instances and the statement's word spans are rebuilt on a language
// change, so the registry is the only place that knows about them.
const splits = [];
let statementTween = null;

function applyLang(lang) {
  const t = T[lang] || T.uk;
  document.documentElement.lang = lang;
  document.title = t.title;
  $$('[data-i18n]').forEach(el => { const k = el.dataset.i18n; if (t[k] != null) el.textContent = t[k]; });
  $$('[data-i18n-html]').forEach(el => { const k = el.dataset.i18nHtml; if (t[k] != null) el.innerHTML = t[k]; });
  $$('[data-i18n-attr]').forEach(el => {
    const [attr, k] = el.dataset.i18nAttr.split(':');
    if (t[k] != null) el.setAttribute(attr, t[k]);
  });
  $$('.lang').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.lang === lang)));
  try { localStorage.setItem('dowiz-lang', lang); } catch { /* private window */ }
  document.dispatchEvent(new CustomEvent('dowiz:lang', { detail: lang }));
}

function pickLang() {
  let stored = null;
  try { stored = localStorage.getItem('dowiz-lang'); } catch { /* private window */ }
  // The visitor's choice, else the first of the browser's languages we speak, else Ukrainian.
  return pickFirst([stored], navigator.languages || [navigator.language], 'uk');
}

// ── the statement: one span per word so scroll can read it ────────────────
function buildStatement() {
  const p = $('#statementP');
  if (!p) return [];
  const text = p.textContent.trim();
  p.textContent = '';
  const words = text.split(/\s+/);
  const spans = words.map((w, i) => {
    const s = document.createElement('span');
    s.className = 'word';
    s.textContent = w;
    p.appendChild(s);
    if (i < words.length - 1) p.appendChild(document.createTextNode(' '));
    return s;
  });
  return spans;
}

// ── still frame for reduced motion or a missing library ───────────────────
function stillFrame() {
  const loader = $('#loader');
  if (loader) loader.classList.add('is-hidden');
  $$('.step').forEach(s => s.classList.add('active'));
  $$('.panel').forEach(p => p.classList.add('is-on'));
  $$('.statement .word').forEach(w => { w.style.opacity = '1'; });
  const b = $('#screenB'); if (b) b.style.opacity = '0';
  setupFilm();
}

// ── motion ─────────────────────────────────────────────────────────────────
function revealLines(el, opts = {}) {
  const inst = SplitText.create(el, {
    type: 'lines', mask: 'lines', linesClass: 'line', autoSplit: true,
    onSplit(self) {
      return gsap.from(self.lines, {
        yPercent: 110, duration: 1.1, stagger: 0.085, ease: 'expo.out',
        ...(opts.scroll ? { scrollTrigger: { trigger: el, start: 'top 85%', once: true } } : {}),
        delay: opts.delay || 0,
      });
    },
  });
  splits.push(inst);
  return inst;
}

function setupStatement() {
  if (statementTween) { statementTween.scrollTrigger && statementTween.scrollTrigger.kill(); statementTween.kill(); }
  const words = buildStatement();
  statementTween = gsap.to(words, {
    opacity: 1, stagger: 0.06, ease: 'none',
    scrollTrigger: { trigger: '#statement', start: 'top 70%', end: 'bottom 55%', scrub: true },
  });
}

function setupScrollScenes() {
  const mm = gsap.matchMedia();

  // hero: the zero drifts as you leave it
  gsap.to('#zero', {
    yPercent: 45, rotate: -6, ease: 'none',
    scrollTrigger: { trigger: '#hero', start: 'top top', end: 'bottom top', scrub: true },
  });

  // the bar hides on the way down and returns on the way up
  const top = $('#top');
  ScrollTrigger.create({
    start: 0, end: 'max',
    onUpdate: self => top.classList.toggle('is-hidden', self.direction === 1 && self.scroll() > 140),
  });

  // headings rise into view, once
  $$('[data-reveal]:not([data-reveal="hero"])').forEach(el => revealLines(el, { scroll: true }));

  // product: the phone answers the step in view
  const screenA = $('#screenA'), screenB = $('#screenB'), chip = $('#chip');
  const show = which => {
    gsap.to(screenB, { opacity: which === 'a' ? 0 : 1, duration: 0.7, ease: 'power2.inOut' });
    gsap.to(chip, { y: which === 'c' ? 0 : '120%', opacity: which === 'c' ? 1 : 0, duration: 0.6, ease: 'expo.out' });
  };
  $$('.step').forEach(step => {
    ScrollTrigger.create({
      trigger: step, start: 'top 62%', end: 'bottom 38%',
      onToggle: self => {
        if (!self.isActive) return;
        $$('.step').forEach(s => s.classList.toggle('active', s === step));
        show(step.dataset.screen);
      },
    });
  });

  // engine: one horizontal run on a wide screen, a plain stack on a narrow one
  mm.add(DESK, () => {
    const track = $('#track');
    const dist = () => Math.max(0, track.scrollWidth - window.innerWidth);
    const run = gsap.to(track, {
      x: () => -dist(), ease: 'none',
      scrollTrigger: {
        trigger: '#engine', start: 'top top', end: () => '+=' + dist(), pin: true, scrub: 0.8,
        anticipatePin: 1, invalidateOnRefresh: true,
      },
    });
    $$('.panel').forEach(panel => ScrollTrigger.create({
      trigger: panel, containerAnimation: run, start: 'left 85%', once: true,
      onEnter: () => panel.classList.add('is-on'),
    }));
    return () => { $$('.panel').forEach(p => p.classList.remove('is-on')); };
  });
  mm.add('(max-width: 55.99rem)', () => {
    $$('.panel').forEach(panel => ScrollTrigger.create({
      trigger: panel, start: 'top 80%', once: true, onEnter: () => panel.classList.add('is-on'),
    }));
  });

  // the twenty rows arrive as a list: one stagger, capped, once
  const rows = $$('.adv-item');
  if (rows.length) {
    gsap.from(rows, {
      y: 28, opacity: 0, duration: 0.9, ease: 'expo.out', stagger: { each: 0.05, from: 'start' },
      scrollTrigger: { trigger: '#advList', start: 'top 82%', once: true },
    });
  }

  // the film: plays muted while on screen, sound behind its button
  setupFilm();

  // the statement reads itself
  setupStatement();

  // the numbers count when they arrive
  $$('[data-count]').forEach(el => {
    const to = Number(el.dataset.count), pre = el.dataset.prefix || '', suf = el.dataset.suffix || '';
    const from = to === 0 ? 35 : 0;
    const o = { v: from };
    ScrollTrigger.create({
      trigger: el, start: 'top 85%', once: true,
      onEnter: () => gsap.to(o, {
        v: to, duration: 1.6, ease: 'power3.out',
        onUpdate: () => { const n = Math.round(o.v); el.textContent = (n === 0 ? '' : pre || '−') + n + suf; },
        onComplete: () => { el.textContent = (to === 0 ? '' : pre) + to + suf; },
      }),
    });
  });

  // the wordmark rises out of the ground
  gsap.fromTo('#wordmark', { yPercent: 24 }, {
    yPercent: 0, ease: 'none',
    scrollTrigger: { trigger: '#wordmark', start: 'top bottom', end: 'bottom bottom', scrub: true },
  });

  // the marquee stops when nobody can see it
  const marq = $('#marq');
  if (marq && 'IntersectionObserver' in window) {
    new IntersectionObserver(([e]) => marq.classList.toggle('still', !e.isIntersecting)).observe(marq);
  }
}

function setupPointer() {
  if (!finePointer) return;
  const cursor = $('#cursor');
  const xTo = gsap.quickTo(cursor, 'x', { duration: 0.22, ease: 'power3' });
  const yTo = gsap.quickTo(cursor, 'y', { duration: 0.22, ease: 'power3' });
  window.addEventListener('pointermove', e => { cursor.classList.add('is-on'); xTo(e.clientX); yTo(e.clientY); }, { passive: true });
  document.addEventListener('pointerover', e => { if (e.target.closest('a,button')) cursor.classList.add('hover'); });
  document.addEventListener('pointerout', e => { if (e.target.closest('a,button')) cursor.classList.remove('hover'); });

  $$('[data-magnet]').forEach(btn => {
    const label = btn.querySelector('span');
    btn.addEventListener('pointermove', e => {
      const r = btn.getBoundingClientRect();
      const dx = (e.clientX - (r.left + r.width / 2)) / r.width;
      const dy = (e.clientY - (r.top + r.height / 2)) / r.height;
      gsap.to(btn, { x: dx * 18, y: dy * 14, duration: 0.4, ease: 'power3.out' });
      if (label) gsap.to(label, { x: dx * 6, y: dy * 4, duration: 0.4, ease: 'power3.out' });
    });
    btn.addEventListener('pointerleave', () => {
      gsap.to(btn, { x: 0, y: 0, duration: 0.7, ease: 'elastic.out(1, 0.4)' });
      if (label) gsap.to(label, { x: 0, y: 0, duration: 0.7, ease: 'elastic.out(1, 0.4)' });
    });
  });
}

function setupSmoothScroll() {
  if (!Lenis) return null;
  const lenis = new Lenis({ lerp: 0.09, smoothWheel: true });
  lenis.on('scroll', ScrollTrigger.update);
  gsap.ticker.add(t => lenis.raf(t * 1000));
  gsap.ticker.lagSmoothing(0);
  $$('a[href^="#"]').forEach(a => a.addEventListener('click', e => {
    const target = $(a.getAttribute('href'));
    if (!target) return;
    e.preventDefault();
    lenis.scrollTo(target, { offset: -24, duration: 1.4 });
  }));
  return lenis;
}

// The preloader counts the fee down to nothing, then the curtain lifts and the
// hero rises. On a second visit in the same session it is skipped: the point
// has been made.
function intro(lenis) {
  const loader = $('#loader');
  const n = $('#loaderN');
  let seen = false;
  try { seen = sessionStorage.getItem('dowiz-intro') === '1'; sessionStorage.setItem('dowiz-intro', '1'); } catch { /* private window */ }

  const hero = () => {
    revealLines('#h1', { delay: 0.05 });
    gsap.from('#zero', { scale: 0.6, rotate: -10, opacity: 0, duration: 1.4, ease: 'expo.out', delay: 0.25 });
    gsap.from('.hero .lede, .hero .cta', { y: 24, opacity: 0, duration: 1, stagger: 0.12, ease: 'expo.out', delay: 0.45 });
  };

  if (seen) { loader.classList.add('is-hidden'); hero(); return; }

  if (lenis) lenis.stop();
  const o = { v: 35 };
  const tl = gsap.timeline({
    onComplete: () => { loader.classList.add('is-hidden'); if (lenis) lenis.start(); ScrollTrigger.refresh(); },
  });
  tl.to(o, { v: 0, duration: 1.3, ease: 'power2.inOut', onUpdate: () => { const v = Math.round(o.v); n.textContent = v === 0 ? '0%' : '−' + v + '%'; } })
    .call(() => n.classList.add('hot'))
    .to(n, { scale: 1.06, duration: 0.25, ease: 'power2.out' })
    .to(loader, { yPercent: -100, duration: 0.9, ease: 'power4.inOut' }, '+=0.2')
    .call(hero, null, '<0.35');
}

function setupLangButtons() {
  $$('.lang').forEach(b => b.addEventListener('click', () => {
    const lang = b.dataset.lang;
    if (motion) {
      splits.forEach(s => s.revert());
      splits.length = 0;
    }
    applyLang(lang);
    if (motion) {
      revealLines('#h1');
      $$('[data-reveal]:not([data-reveal="hero"])').forEach(el => revealLines(el, { scroll: true }));
      setupStatement();
      ScrollTrigger.refresh();
    }
  }));
}

// ── the film ───────────────────────────────────────────────────────────────
// The source follows the language (one file per language), the poster shows
// before anything loads, autoplay is muted and only while the phone is on
// screen; the button turns the sound on and restarts from the top.
// The poster follows it too: the frame carries a burned-in subtitle, so a
// Ukrainian page must not open on an English still.
/// The films exist in MEDIA_LANGS only (no Russian cut): a ru page plays the English film.
function filmLang() { const lang = document.documentElement.lang; return MEDIA_LANGS.includes(lang) ? lang : 'en'; }
function filmSrc(v) { return v.dataset.src.replace('{lang}', filmLang()); }
function filmPoster(v) { return v.dataset.poster.replace('{lang}', filmLang()); }
function setupFilm() {
  const v = $('#promo'), btn = $('#filmSound');
  if (!v) return;
  const load = () => { const src = filmSrc(v); if (v.getAttribute('src') !== src) { v.setAttribute('src', src); v.load(); } };
  const repost = () => { const poster = filmPoster(v); if (v.getAttribute('poster') !== poster) v.setAttribute('poster', poster); };
  repost();
  let onScreen = false;
  if ('IntersectionObserver' in window) {
    new IntersectionObserver(([e]) => {
      onScreen = e.isIntersecting;
      if (onScreen) { load(); v.play().catch(() => {}); } else { v.pause(); }
    }, { threshold: 0.35 }).observe(v);
  } else { load(); }
  if (btn) btn.addEventListener('click', () => {
    const on = v.muted;
    v.muted = !on;
    btn.setAttribute('aria-pressed', String(on));
    const t = T[document.documentElement.lang] || T.uk; btn.querySelector('span').textContent = on ? t.filmMute : t.filmSound;
    if (on) { v.currentTime = 0; v.play().catch(() => {}); }
  });
  document.addEventListener('dowiz:lang', () => { repost(); if (onScreen) { load(); v.play().catch(() => {}); } else { v.removeAttribute('src'); } });
}

// ── the waiting list ───────────────────────────────────────────────────────
// One POST, three answers. The row is written server-side before any mail
// goes out, so a mail failure is never the visitor's problem.
function setupJoin() {
  const form = $('#join');
  if (!form) return;
  const email = $('#joinEmail'), go = $('#joinGo'), said = $('#joinSaid');
  const t = () => T[document.documentElement.lang] || T.uk;
  const tell = (msg, kind) => { said.textContent = msg; said.className = 'join-said ' + (kind || ''); };
  form.addEventListener('submit', async e => {
    e.preventDefault();
    const addr = email.value.trim().toLowerCase();
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/.test(addr)) { tell(t().joinInvalid, 'bad'); email.focus(); return; }
    go.disabled = true;
    const label = go.querySelector('span'); const was = label.textContent; label.textContent = t().joinSending;
    try {
      const r = await fetch('/api/waitlist', {
        method: 'POST', headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ email: addr, venue: '', lang: document.documentElement.lang }),
      });
      if (!r.ok) throw new Error('HTTP ' + r.status);
      tell(t().joinOk.replace('{email}', addr), 'ok');
      form.classList.add('is-sent');
    } catch {
      tell(t().joinBad, 'bad');
      go.disabled = false;
    } finally {
      label.textContent = was;
    }
  });
}

// ── boot ───────────────────────────────────────────────────────────────────
applyLang(pickLang());
setupLangButtons();
setupJoin();

if (!motion) {
  buildStatement();
  stillFrame();
} else {
  gsap.registerPlugin(ScrollTrigger, SplitText);
  document.fonts.ready.then(() => {
    const lenis = setupSmoothScroll();
    setupScrollScenes();
    setupPointer();
    intro(lenis);
  });
}
