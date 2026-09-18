// Mobile audit — is this usable with a thumb, on the smallest phone people have.
//
// The render gate proves a screen draws. This one asks whether it WORKS on a
// phone, and it checks the things that only bite on a phone:
//
//   * an input smaller than 16px makes iOS Safari ZOOM THE WHOLE PAGE on focus
//     and never zoom back. It is the single most common mobile defect in a
//     desktop-built form and it is invisible on a desktop browser.
//   * a tap target under 44x44 (Apple HIG) / 48x48 (Material) is a control
//     people miss. Ours are measured against 44.
//   * 320px is still a real width — an iPhone SE in landscape-locked apps, a
//     Galaxy Fold's cover screen, and any phone at 200% text zoom. A layout
//     that only works at 375 is a layout that breaks for somebody.
//   * text under 12px is not read, it is squinted at.
//   * a fixed bar that covers the last row of a list makes that row
//     unreachable, and nobody ever scrolls further to find out.
//
// Every number here is a published guideline, not a preference:
//   Apple HIG "Layout": 44x44pt minimum tap target.
//   Material 3 "Accessibility": 48x48dp, 24dp minimum spacing.
//   WCAG 2.2 SC 2.5.8 (Target Size, Minimum): 24x24 CSS px absolute floor.
//   iOS Safari: zooms on focus when the field's computed font-size < 16px.

import { chromium, devices } from 'playwright';

const HOST = process.env.HOST || 'https://dubin-sushi.dowiz.org';

/** The floors. A control below the first is a defect; below the second is severe. */
export const TAP_MIN = 44;   // Apple HIG
export const TAP_FLOOR = 24; // WCAG 2.2 SC 2.5.8 absolute minimum
export const INPUT_MIN_FONT = 16;
export const TEXT_MIN = 12;

/** The phones. The narrow one is the point: 320 is where layouts break. */
const PROFILES = [
  { name: 'iPhone SE (320)', viewport: { width: 320, height: 568 }, mobile: true },
  { name: 'iPhone 14 (390)', viewport: { width: 390, height: 844 }, mobile: true },
  { name: 'Pixel 7 (412)', viewport: { width: 412, height: 915 }, mobile: true },
];

const UA = 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 ' +
           '(KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1';

async function auditPage(page, url) {
  await page.goto(url, { waitUntil: 'networkidle', timeout: 45_000 });
  await page.waitForTimeout(900);

  // A HIT TEST ONLY WORKS ON WHAT IS ON SCREEN. The first version measured
  // everything after scrolling to the bottom, so every control above the fold
  // hit-tested as zero and the audit reported working 44px targets as 15px.
  // The page is therefore sampled at three scroll positions and the findings
  // merged — each control is measured while it is actually visible.
  const at = async fraction => {
    await page.evaluate(f => {
      const d = document.documentElement;
      window.scrollTo(0, (d.scrollHeight - d.clientHeight) * f);
    }, fraction);
    await page.waitForTimeout(250);
    return measure(page);
  };
  const passes = [await at(0), await at(0.5), await at(1)];

  // Merge: a control is a defect if it was too small EVERY time it was seen.
  // Seen once and fine is fine; never seen is not a finding.
  const seen = new Map();
  for (const p of passes) {
    for (const c of p.small) {
      const prev = seen.get(c.el);
      if (!prev || c.w * c.h > prev.w * prev.h) seen.set(c.el, c);
    }
    for (const c of p.measured) {
      const prev = seen.get(c.el);
      if (prev && c.w * c.h > prev.w * prev.h) seen.delete(c.el);
    }
  }
  const last = passes[passes.length - 1];
  const small = [...seen.values()];
  return {
    ...last,
    small,
    tiny: small.filter(c => c.w < TAP_FLOOR || c.h < TAP_FLOOR),
  };
}

function measure(page) {
  return page.evaluate(({ TAP_MIN, TAP_FLOOR, INPUT_MIN_FONT, TEXT_MIN }) => {
    const label = el => {
      const cls = String(el.className || '').split(' ').filter(Boolean)[0];
      return `${el.tagName.toLowerCase()}${cls ? '.' + cls : ''}` +
             (el.id ? '#' + el.id : '');
    };
    const visible = el => {
      const r = el.getBoundingClientRect();
      const s = getComputedStyle(el);
      return r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none'
             && s.opacity !== '0';
    };

    // ── Tap targets ──
    // A control's target is its own box, grown by any padding on a parent that
    // exists only to give it room. Measured as drawn: what a thumb can hit.
    const controls = [...document.querySelectorAll(
      'button, a[href], input:not([type=hidden]), select, textarea, [role="button"], ' +
      '[role="radio"], [role="tab"], [tabindex]:not([tabindex="-1"])')]
      .filter(visible);
    // ── The target is measured by HIT-TESTING, not by the layout box ──
    //
    // A control's hit area is often bigger than its painted box: a transparent
    // `::after` extension is the standard way to give a 15px-tall word a 44px
    // target without moving anything. `getBoundingClientRect` cannot see that,
    // so the first version of this audit reported every extended control as
    // still too small. Asking `elementFromPoint` at the corners of a 44x44 box
    // centred on the control measures what a thumb actually gets.
    //
    // An input wrapped in a <label> is tapped by tapping the label, so the
    // label counts as the control.
    const owns = (el, hit) => !!hit && (hit === el || el.contains(hit) || hit.contains(el)
      || (el.closest('label') && el.closest('label').contains(hit)));
    const hitSpan = el => {
      const r = el.getBoundingClientRect();
      const cx = r.left + r.width / 2, cy = r.top + r.height / 2;
      const reach = (dx, dy) => {
        let lo = 0, hi = TAP_MIN;            // how far the target extends
        while (hi - lo > 1) {
          const mid = (lo + hi) / 2;
          const x = cx + dx * mid, y = cy + dy * mid;
          const inside = x >= 0 && y >= 0 && x <= window.innerWidth && y <= window.innerHeight
            && owns(el, document.elementFromPoint(x, y));
          if (inside) lo = mid; else hi = mid;
        }
        return lo;
      };
      return { w: Math.round(reach(-1, 0) + reach(1, 0)), h: Math.round(reach(0, -1) + reach(0, 1)) };
    };
    const small = controls
      .filter(el => {
        const r = el.getBoundingClientRect();
        return r.bottom > 0 && r.top < window.innerHeight;   // hit-testable
      })
      .map(el => {
      const box = el.getBoundingClientRect();
      const hit = hitSpan(el);
      return {
        el: label(el),
        // Whichever is larger: the painted box or what the hit test reached.
        w: Math.max(Math.round(box.width), hit.w),
        h: Math.max(Math.round(box.height), hit.h),
      };
    }).filter(c => c.w < TAP_MIN || c.h < TAP_MIN);
    const tiny = small.filter(c => c.w < TAP_FLOOR || c.h < TAP_FLOOR);

    // ── iOS zoom-on-focus ──
    const zoomers = [...document.querySelectorAll('input, select, textarea')]
      .filter(visible)
      .map(el => ({ el: label(el), size: parseFloat(getComputedStyle(el).fontSize) }))
      .filter(f => f.size < INPUT_MIN_FONT);

    // ── Unreadable text ──
    const tooSmall = [...document.querySelectorAll('body *')]
      .filter(el => el.children.length === 0 && (el.textContent || '').trim().length > 2)
      .filter(visible)
      .map(el => ({ el: label(el), size: parseFloat(getComputedStyle(el).fontSize),
                    text: (el.textContent || '').trim() }))
      // A SHORT LABEL IS NOT BODY COPY. "25% OFF" on a high-contrast pill is
      // read at a glance, not read through, and 11px is conventional for one.
      // A sentence at 11px is not. The rule is written down rather than being a
      // list of exempt class names, which would rot the first time a badge is
      // renamed.
      .filter(t => (t.text.length > 16 ? t.size < TEXT_MIN : t.size < TEXT_MIN - 1))
      .map(t => ({ ...t, text: t.text.slice(0, 24) }));

    // ── Sideways scroll ──
    const doc = document.documentElement;
    // Only when the PAGE actually scrolls sideways. A child that sticks out of
    // a clipped parent has a layout box past the edge and is not visible past
    // it — reporting those made a correct offer card look like a defect.
    const pageScrolls = doc.scrollWidth > doc.clientWidth + 1;
    const overflowing = !pageScrolls ? [] : [...document.querySelectorAll('body *')]
      .filter(visible)
      .filter(el => el.getBoundingClientRect().right > window.innerWidth + 1)
      .filter(el => {
        // Clipped by an ancestor is not overflow.
        let p = el.parentElement;
        while (p && p !== document.body) {
          if (getComputedStyle(p).overflow !== 'visible') return false;
          p = p.parentElement;
        }
        return true;
      })
      // A rail that scrolls horizontally ON PURPOSE is not an overflow; the
      // page scrolling sideways is. Only count elements that push the PAGE.
      .filter(el => {
        let p = el.parentElement;
        while (p && p !== document.body) {
          const s = getComputedStyle(p);
          if (s.overflowX === 'auto' || s.overflowX === 'scroll') return false;
          p = p.parentElement;
        }
        return true;
      })
      .slice(0, 5)
      .map(el => `${label(el)}@${Math.round(el.getBoundingClientRect().right)}px`);

    // ── Content under a fixed bar ──
    const bars = [...document.querySelectorAll('.k-nav, .k-bar, .k-composer')].filter(visible);
    const covered = [];
    for (const bar of bars) {
      const b = bar.getBoundingClientRect();
      for (const el of controls) {
        if (bar.contains(el)) continue;
        const r = el.getBoundingClientRect();
        // Only what is on screen right now: an element further down the page is
        // reachable by scrolling.
        if (r.bottom <= 0 || r.top >= window.innerHeight) continue;
        const overlap = Math.min(r.bottom, b.bottom) - Math.max(r.top, b.top);
        if (overlap > Math.min(r.height, b.height) / 2) covered.push(label(el));
      }
    }

    // ── Mobile hygiene on the document itself ──
    const bodyStyle = getComputedStyle(document.body);
    const meta = document.querySelector('meta[name=viewport]')?.getAttribute('content') || '';

    return {
      controls: controls.length,
      // Controls that were ON SCREEN for this pass and met the floor. Used to
      // clear a finding raised while the same control was off screen.
      measured: controls
        .filter(el => {
          const r = el.getBoundingClientRect();
          return r.bottom > 0 && r.top < window.innerHeight;
        })
        .map(el => {
          const box = el.getBoundingClientRect();
          const hit = hitSpan(el);
          return { el: label(el),
                   w: Math.max(Math.round(box.width), hit.w),
                   h: Math.max(Math.round(box.height), hit.h) };
        })
        .filter(c => c.w >= TAP_MIN && c.h >= TAP_MIN),
      small, tiny, zoomers, tooSmall,
      overflowing,
      scrollW: doc.scrollWidth,
      clientW: doc.clientWidth,
      covered: [...new Set(covered)],
      viewportMeta: meta,
      // A page that can be zoomed is an accessibility requirement; blocking it
      // is a WCAG 1.4.4 failure, so this is checked in the direction people
      // usually get wrong.
      blocksZoom: /user-scalable\s*=\s*(no|0)/i.test(meta) || /maximum-scale\s*=\s*1(\.0)?\b/i.test(meta),
      hasSafeArea: /viewport-fit\s*=\s*cover/i.test(meta),
      tapHighlight: bodyStyle.webkitTapHighlightColor || '',
      overscroll: bodyStyle.overscrollBehaviorY || bodyStyle.overscrollBehavior || '',
      textSizeAdjust: getComputedStyle(document.documentElement).webkitTextSizeAdjust || '',
    };
  }, { TAP_MIN, TAP_FLOOR, INPUT_MIN_FONT, TEXT_MIN });
}

/// EVERY SCREEN THE ROUTER KNOWS.
///
/// Read from `app.js`'s own `SCREENS` table, so a screen added to the kit is
/// audited without anybody remembering to add it here twice.
export async function allRoutes(){
  const src = await (await fetch(`${HOST}/kit/app.js`)).text();
  const block = src.split('const SCREENS = {')[1].split('};')[0];
  return [...new Set([...block.matchAll(/^\s*'?([a-z-]+)'?:/gm)].map(m => m[1]))].sort();
}

export async function run(routes) {
  // `run.mjs` calls every gate with no arguments, so this one received
  // `undefined` and died on `for (const route of routes)` before it had audited
  // a single screen -- and the suite went on to report a total that did not
  // include it. A gate that crashes is not a gate that passes: the route list is
  // discovered here when nobody hands one in.
  routes = routes?.length ? routes : await allRoutes();
  const browser = await chromium.launch({
    args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu'],
  });

  // The findings, collapsed across screens: the same undersized control on
  // forty screens is ONE defect, not forty.
  const byDefect = new Map();
  const note = (kind, key, where) => {
    const id = `${kind}|${key}`;
    if (!byDefect.has(id)) byDefect.set(id, { kind, key, where: new Set() });
    byDefect.get(id).where.add(where);
  };

  for (const profile of PROFILES) {
    const ctx = await browser.newContext({
      viewport: profile.viewport, deviceScaleFactor: 2,
      isMobile: true, hasTouch: true, userAgent: UA,
    });
    const page = await ctx.newPage();

    for (const route of routes) {
      let a;
      try {
        a = await auditPage(page, `${HOST}/kit/#/${route}`);
      } catch (e) {
        note('navigation', e.message.slice(0, 60), `${profile.name} ${route}`);
        continue;
      }
      const at = `${profile.name} ${route}`;
      for (const c of a.zoomers) note('ios-zoom-on-focus', `${c.el} @${c.size}px`, at);
      for (const c of a.tiny) note('tap-target-below-24', `${c.el} ${c.w}x${c.h}`, at);
      for (const c of a.small) note('tap-target-below-44', `${c.el} ${c.w}x${c.h}`, at);
      for (const t of a.tooSmall) note('text-below-12', `${t.el} @${t.size}px`, at);
      for (const o of a.overflowing) note('page-scrolls-sideways', o, at);
      for (const c of a.covered) note('covered-by-fixed-bar', c, at);
      if (a.blocksZoom) note('blocks-pinch-zoom', a.viewportMeta, at);
      if (!a.hasSafeArea) note('no-viewport-fit-cover', a.viewportMeta, at);
    }
    await ctx.close();
  }
  await browser.close();

  const order = ['navigation', 'ios-zoom-on-focus', 'page-scrolls-sideways',
                 'covered-by-fixed-bar', 'tap-target-below-24', 'blocks-pinch-zoom',
                 'tap-target-below-44', 'text-below-12', 'no-viewport-fit-cover'];
  const found = [...byDefect.values()]
    .sort((a, b) => order.indexOf(a.kind) - order.indexOf(b.kind));

  for (const f of found) {
    const where = [...f.where];
    console.log(`${f.kind.padEnd(24)} ${f.key}`);
    console.log(`  on ${where.length} screen/profile pair(s), e.g. ${where.slice(0, 2).join(', ')}`);
  }
  console.log(found.length
    ? `\n${found.length} distinct mobile defect(s)`
    : '\nno mobile defects');
  return found.length;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const arg = process.argv.slice(2).filter(Boolean);
  process.exit(await run(arg) ? 1 : 0);
}
