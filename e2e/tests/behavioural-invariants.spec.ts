import { test, expect } from '@playwright/test';

// SYSTEMIC GUARDRAIL (regression ledger #14) — the home for "text-gate-invisible" invariants.
//
// Council pattern-critic root R1: a recurring class of defects that are green to grep/
// typecheck/build/lint but WRONG at render time — dark-on-dark contrast (#6), a silently
// dropped CSS rule (#13), tenant-theme fall-through. Static rules can't see these; only a
// computed-outcome check on a real browser can. This suite asserts rendered INVARIANTS and is
// green on the current repo (the behaviours hold today) — it goes red when a change breaks one.
// Extend it (add `expect`s) as new outcome-invariants are discovered; never weaken it.
//
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test behavioural-invariants --project=desktop --reporter=list
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

// WCAG relative-luminance contrast, computed in the page over EFFECTIVE colours (walks
// ancestors for the first opaque background) — the same maths a human auditor would apply.
const CONTRAST_FN = `
(sel) => {
  const lum = (r,g,b) => { const f=c=>{c/=255;return c<=0.03928?c/12.92:Math.pow((c+0.055)/1.055,2.4);}; return 0.2126*f(r)+0.7152*f(g)+0.0722*f(b); };
  const parse = (s) => { const m=(s||'').match(/rgba?\\(([^)]+)\\)/); if(!m) return null; const p=m[1].split(',').map(x=>parseFloat(x)); return {r:p[0],g:p[1],b:p[2],a:p[3]===undefined?1:p[3]}; };
  // Walk to the first opaque solid bg. If a background-IMAGE/gradient appears first, contrast
  // is undefined by luminance (it needs a scrim, not a ratio) → signal skip.
  const effBg = (el) => { let n=el; while(n){ const s=getComputedStyle(n); if((s.backgroundImage||'none')!=='none') return {image:true}; const c=parse(s.backgroundColor); if(c&&c.a>0.5) return c; n=n.parentElement; } return {r:255,g:255,b:255,a:1}; };
  // Evaluate EVERY matching element, not just the first — a blind spot where only the first
  // card/button was ever contrast-checked while the rest could regress unseen.
  const evalEl = (el) => {
    const cs = getComputedStyle(el); const fg = parse(cs.color); const bg = effBg(el);
    if(!fg || bg.image) return { skip:true };
    const L1 = lum(fg.r,fg.g,fg.b), L2 = lum(bg.r,bg.g,bg.b);
    const ratio = (Math.max(L1,L2)+0.05)/(Math.min(L1,L2)+0.05);
    const px = parseFloat(cs.fontSize)||16; const bold = (parseInt(cs.fontWeight)||400)>=700;
    const large = px>=24 || (px>=18.66 && bold);
    return { ratio: Math.round(ratio*100)/100, large, text: (el.textContent||'').trim().slice(0,40) };
  };
  return Array.from(document.querySelectorAll(sel)).map(evalEl);
}`;

test('storefront renders readable text (WCAG-AA contrast — catches dark-on-dark / theme fall-through)', async ({ page }) => {
  await page.goto(`${BASE}/s/demo`);
  // Curated, always-present text surfaces (the essentials a customer must read).
  await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible({ timeout: 25000 });
  const targets = ['h1', '[data-testid="menu-item"]', 'button'];
  let presentSelectors = 0;
  let checkedSelectors = 0;
  for (const sel of targets) {
    const results = await page.evaluate(`(${CONTRAST_FN})(${JSON.stringify(sel)})`) as Array<{ ratio: number; large: boolean; text: string; skip?: boolean }>;
    if (!results || results.length === 0) continue; // selector absent from the DOM
    presentSelectors++;
    let checkedThisSel = 0;
    for (const res of results) {
      if (!res || res.skip) continue; // text-over-image/gradient (contrast undefined — separate scrim invariant)
      checkedThisSel++;
      const min = res.large ? 3 : 4.5;
      expect(res.ratio, `"${res.text}" (${sel}) contrast ${res.ratio}:1 must clear WCAG ${res.large ? 'AA-large 3:1' : 'AA 4.5:1'}`).toBeGreaterThanOrEqual(min);
    }
    if (checkedThisSel > 0) checkedSelectors++;
  }
  expect(presentSelectors, 'at least one curated text surface was present').toBeGreaterThan(0);
  // Sharpen the old `checked > 0`: it was satisfiable by a single non-skipped selector while the
  // other two fell through to skip (all over images/gradients). Require EVERY present selector to
  // contribute at least one real, solid-surface contrast check.
  expect(checkedSelectors, `every present curated selector yielded a solid-surface contrast check (${checkedSelectors}/${presentSelectors})`).toBe(presentSelectors);
});

test('the contrast invariant is sharp — solid dark-on-dark is flagged (red arm)', async ({ page }) => {
  // Inject the #6 shape: near-equal dark text on a dark solid surface. The gate MUST flag it.
  await page.setContent(`<div style="background:#152928"><p id="bad" style="color:#213433;font-size:16px">dark on dark</p></div>
    <div style="background:#ffffff"><p id="ok" style="color:#111;font-size:16px">good</p></div>`);
  const bad = (await page.evaluate(`(${CONTRAST_FN})("#bad")`) as Array<{ ratio: number }>)[0];
  const ok = (await page.evaluate(`(${CONTRAST_FN})("#ok")`) as Array<{ ratio: number }>)[0];
  expect(bad?.ratio, 'dark-on-dark solid text is below AA (gate bites)').toBeLessThan(4.5);
  expect(ok?.ratio, 'black-on-white clears AA (gate passes good contrast)').toBeGreaterThanOrEqual(4.5);
});

test('storefront paints a resolved brand surface (no empty/transparent token fall-through)', async ({ page }) => {
  await page.goto(`${BASE}/s/demo`);
  await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible({ timeout: 25000 });
  // The #13 class generalized: the body must paint a real, opaque colour — not an unresolved
  // var()/transparent (which is how a dropped/!defined token scope manifests as a blank shell).
  const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  expect(bg, 'body background resolves to a real colour').toMatch(/^rgb/);
  expect(bg, 'body background is not transparent (token did resolve)').not.toBe('rgba(0, 0, 0, 0)');
  // A token scope can resolve on <body> yet fall through on a child surface (the #13 shape in a
  // nested scope). The menu-item card's TEXT colour falling through to transparent is always a
  // bug (invisible text) — assert child surfaces resolve their tokens too, not just the body.
  const card = await page.evaluate(() => {
    const el = document.querySelector('[data-testid="menu-item"]');
    if (!el) return null;
    const cs = getComputedStyle(el);
    return { bg: cs.backgroundColor, color: cs.color };
  });
  expect(card, 'a menu-item card is present to validate its tokens').not.toBeNull();
  expect(card!.color, 'menu-item text colour resolved — not a token fall-through to transparent').not.toBe('rgba(0, 0, 0, 0)');
  expect(card!.bg, 'menu-item background resolves to a colour token').toMatch(/^rgb/);
});

test('a nonexistent storefront slug does NOT fall back to another tenant (no silent cross-tenant leak)', async ({ page }) => {
  // Negative control: every other test here only ever hits /s/demo, so a silent fallback (an
  // unknown slug quietly serving the demo tenant's menu) would be invisible to this suite. The
  // menu fetch 404s for an unknown slug (MenuPage sets notFound → renders a "not found" shell),
  // so ZERO menu-item cards must paint — a non-zero count means a real cross-tenant content leak.
  // TODO(staging): tighten to assert the not-found shell's own [data-testid] once one is added in
  //   apps/web/src/pages/client/MenuPage.tsx (the not-found block currently has no testid); and add
  //   a real 2nd-tenant case (bogus slug must not surface tenant B's data). Needs a live staging run.
  await page.goto(`${BASE}/s/no-such-tenant-xyz-${Date.now()}`);
  await page.waitForLoadState('networkidle');
  await expect(page.locator('[data-testid="menu-item"]')).toHaveCount(0);
});
