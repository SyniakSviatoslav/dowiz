# Island cost breakdown + slimming options — `/s/[slug]`

Measurement only, no implementation. Same methodology as the pre-swap baseline:
`gzip -c dist/client/_astro/*.js | wc -c` per chunk, summed. Build command: `pnpm run build`
(which now runs the real `paraglide-js compile` via the new `prebuild` hook — see
`PARAGLIDE-SWAP.md`-equivalent notes in the Part 1 report). Astro/Vite default chunk splitting
(`manualChunks: undefined` in `astro.config.mjs`), so this is the natural split for the current
3-island graph, not a hand-tuned one.

## Headline

| | Value |
|---|---|
| Budget | 8,000 B gz |
| Before Part 1 (paraglide-stub) | 21,111 B gz |
| **After Part 1 (real Paraglide runtime)** | **21,612 B gz** (+501 B, +2.4%) |
| Over budget by | 13,612 B gz (2.7×) |

The real runtime is very slightly heavier than the hand-rolled stub (a dedicated compiled
`runtime.js` chunk instead of ~15 lines of stub code inlined nowhere), but the swap is not the
cost driver — see the floor finding below.

## Per-chunk table (after Part 1 rebuild)

| Chunk | Raw B | Gzip B | % of total gz | Category |
|---|---:|---:|---:|---|
| `template.ChFcpgPI.js` | 25,795 | 10,106 | 46.76% | Svelte runtime — core (signals/effects, template cloning) |
| `render.gz4d_5ko.js` | 6,087 | 2,804 | 12.97% | Svelte runtime — render/mount/hydrate helpers |
| `LanguageSwitcher.nSlX7pPz.js` | 5,308 | 2,674 | 12.37% | Per-island — LanguageSwitcher |
| `CartButton.Bz8uMHy2.js` | 3,281 | 1,611 | 7.45% | Per-island — CartButton (+ inlined `cart_title` message) |
| `MenuBrowser.BioKbbGL.js` | 3,132 | 1,577 | 7.30% | Per-island — MenuBrowser |
| `runtime.FLdgvEqT.js` | 1,972 | 972 | 4.50% | i18n — compiled Paraglide runtime (`getLocale`/`setLocale`/`locales`) |
| `client.svelte.D1AxwHxX.js` | 1,165 | 675 | 3.12% | Svelte runtime — `@astrojs/svelte` hydration bootstrap |
| `attributes.BRDNltmW.js` | 1,017 | 616 | 2.85% | Svelte runtime — attribute/class reactivity helpers |
| `cart-store.svelte.D6ydfNwv.js` | 847 | 468 | 2.17% | Store — shared cart singleton (`$state` runes) |
| `disclose-version.DsnmJJEf.js` | 66 | 109 | 0.50% | Svelte runtime — version-disclosure marker |
| **Total** | **48,670** | **21,612** | **100%** | |

Category rollup:

| Category | Gzip B | % of total |
|---|---:|---:|
| Svelte runtime (template + render + attributes + client.svelte + disclose-version) | 14,310 | 66.2% |
| Per-island code (CartButton + LanguageSwitcher + MenuBrowser) | 5,862 | 27.1% |
| i18n (Paraglide runtime chunk) | 972 | 4.5% |
| Stores (cart-store) | 468 | 2.2% |

### Dependency graph (who needs what)

Traced from each chunk's own `import` statements in `dist/client/_astro/*.js`:

- `template.ChFcpgPI.js` — no imports; the base chunk every other chunk here depends on.
- `render.gz4d_5ko.js` → imports `template.js`. Imported by **`client.svelte.js`** (Astro's own
  hydration bootstrap: `mount`/`hydrate` calls) **and** by `CartButton` and `LanguageSwitcher`.
- `attributes.BRDNltmW.js` → imports `template.js`. Imported by `LanguageSwitcher`
  (`class:active`) and `MenuBrowser` (attribute bindings).
- `client.svelte.js` → imports `template.js` + `render.js`. This is the `@astrojs/svelte` client
  entry that mounts **any** `client:*` Svelte component into its `<astro-island>` — one copy,
  shared by all three islands, not per-island.
- `disclose-version.js` → no imports; a side-effect-only marker imported by all three islands.
- `runtime.FLdgvEqT.js` (Paraglide) → no imports; imported by `CartButton` (`getLocale` for the
  inlined `cart_title` message) and `LanguageSwitcher` (`getLocale`/`setLocale`/`locales`).
- `cart-store.svelte.js` → imports `template.js`; imported by `CartButton` and `MenuBrowser`.

**Finding that drives every option below:** `render.js` is pulled in by Astro's own
`client.svelte.js` hydration bootstrap, not by island choice — it ships the moment **any** Svelte
component hydrates client-side, with or without CartButton or LanguageSwitcher. Combined with
`template.js`, `attributes.js`, `client.svelte.js` and `disclose-version.js`, the **Svelte-runtime
floor for hydrating even one Svelte island is ≈14.3 kB gz** — already 1.8× the entire 8 kB budget,
before any island's own code or i18n is counted.

## Slimming options (estimates, not implemented)

### (a) LanguageSwitcher → zero-JS (server-rendered links/form to a locale route)

Replace the `client:idle` Svelte island with plain server-rendered `<a>`/`<form>` elements in
`StorefrontLayout.astro` (no hydration at all).

- **Saves:** `LanguageSwitcher.nSlX7pPz.js`'s own chunk only — **2,674 B gz**. None of the shared
  Svelte-runtime chunks shrink (`render.js`/`attributes.js`/`client.svelte.js` all stay — both
  CartButton and MenuBrowser still need them), and `runtime.FLdgvEqT.js` (Paraglide) stays too,
  since CartButton still calls `cart_title()` client-side.
- **New total:** 21,612 − 2,674 = **18,938 B gz** (still 2.4× over budget).
- **UX change:** switching language becomes a full navigation instead of an instant DOM update.
  In practice this converges anyway — Paraglide's own `setLocale()` default is `reload: true`
  (a full page reload) precisely because SSR-rendered strings need a fresh render to reflect the
  new locale; only the current dark-build wiring (`{ reload: false }`, see Part 1) suppresses
  that. Real parity would need a locale route or query param the server reads.
- **Effort: Medium.** The current runtime strategy is `["globalVariable", "baseLocale"]` — no
  request-based detection at all. A zero-JS switcher needs the strategy widened back to include
  `"url"` or `"cookie"` plus an Astro middleware/route to read it per request — this is new
  server-side plumbing, not a pure deletion.

### (b) CartButton `client:idle` → `client:visible`, or a vanilla-JS micro-island

Two distinct sub-options bundled under "CartButton slimming":

- **`client:idle` → `client:visible` alone: saves 0 B gz.** This only changes *when* the existing
  `CartButton.Bz8uMHy2.js` chunk is fetched/hydrated (on-idle vs. on-intersection) — it ships the
  same bytes either way. Worth doing for time-to-interactive scheduling, not for the budget; don't
  count on it for the 8 kB target. (Also marginal here: the cart bar is `position:fixed;
  bottom:1rem`, so it's already within the initial viewport on most devices — `client:visible`
  may fire almost immediately anyway.)
- **Vanilla-JS micro-island rewrite (no Svelte compile for this one component):** replaces
  `CartButton.Bz8uMHy2.js` (1,611 B gz of Svelte-compiled output for one conditional bar + 3
  dynamic text nodes) with hand-written DOM code. Rough estimate: **~200–400 B gz** for the
  rewritten chunk, i.e. **≈1.2–1.4 kB gz saved** on the per-island line.
  Shared Svelte-runtime chunks (`template`/`render`/`attributes`/`client.svelte`/
  `disclose-version`) are **unaffected** — MenuBrowser (client:load, not in scope for either
  option) still needs every one of them. `cart-store.svelte.js` (468 B gz) also stays, since
  MenuBrowser still reads/writes it.
  - **New total (this sub-option alone):** ≈ 21,612 − 1,300 ≈ **20,300 B gz** — a ~6% cut, because
    the dominant cost is the shared Svelte runtime, not CartButton's own code.
  - **Bonus (combine with option a):** if CartButton also stops calling `cart_title()`
    client-side (label rendered server-side into a `data-*` attribute the vanilla script reads
    instead), and LanguageSwitcher goes zero-JS per (a), the entire `runtime.FLdgvEqT.js` Paraglide
    chunk (972 B gz) drops out of the client bundle too — i18n becomes 100% SSR-only. Combined
    (a) + vanilla (b) + this bonus ≈ 2,674 + 1,300 + 972 ≈ **4,946 B gz saved → ≈16,666 B gz**,
    still 2.1× over budget.
  - **Effort: Medium–High.** Hand-rolling the reactivity Svelte currently provides for free
    (subscribing to `cart-store`'s state without runes — plain pub/sub or a `CustomEvent`), plus
    re-testing the sticky-bar interaction manually since it exits the Svelte/Playwright-component
    testing path this repo otherwise uses uniformly.

### (c) Accept-and-revise the budget (if MenuBrowser stays a Svelte island)

MenuBrowser is `client:load` (must hydrate immediately — scroll-spy, search, add-to-cart) and is
the reason `client.svelte.js` (and therefore `render.js`) ship regardless of what happens to the
other two islands. Running the numbers for the floor if **both (a) and the vanilla version of
(b) are done** — i.e. MenuBrowser is the *only* remaining Svelte-hydrated component:

| Chunk | Gzip B | Still needed because |
|---|---:|---|
| `template.ChFcpgPI.js` | 10,106 | MenuBrowser imports it directly |
| `render.gz4d_5ko.js` | 2,804 | Astro's `client.svelte.js` bootstrap needs it for *any* hydrated Svelte component |
| `client.svelte.js` | 675 | Astro's hydration entry for any `client:*` Svelte island |
| `attributes.BRDNltmW.js` | 616 | MenuBrowser imports it for attribute bindings |
| `cart-store.svelte.js` | 468 | MenuBrowser's add-to-cart delegation reads/writes it |
| `disclose-version.js` | 109 | MenuBrowser imports it (Svelte marker) |
| MenuBrowser's own code | 1,577 | scroll-spy + search + add-to-cart logic |
| **Floor with MenuBrowser as the only Svelte island** | **16,355** | **still 2.0× the 8 kB budget** |

- **Finding:** the Svelte-runtime floor alone (14,310 B gz — before MenuBrowser's own code) already
  exceeds the entire budget. No amount of trimming CartButton or LanguageSwitcher closes this gap
  while MenuBrowser stays a hydrated Svelte component.
- **Two ways to actually hit 8 kB:**
  1. **Revise the budget** to reflect Svelte 5 islands' real floor — document ≈16–17 kB gz as the
     accepted number for this architecture and move on. **Effort: none (a decision, not code).**
  2. **Rewrite MenuBrowser itself in vanilla JS/web components**, dropping the Svelte dependency
     from the client bundle entirely (scroll-spy via `IntersectionObserver` directly, search via
     plain `input`/filter, add-to-cart via event delegation — all of which MenuBrowser's own
     header comments already describe doing manually, just not through Svelte's reactivity). This
     is the only path that removes the 14.3 kB floor, but it's a full rewrite of the largest and
     most interactive island, discards the Astro+Svelte-islands architecture decision for it
     specifically, and loses Svelte's declarative reactivity for the component that needs it most.
     **Effort: High** — architecture-level, not a slimming pass.

## Decision needed from the operator

None of (a)/(b) alone or combined reaches 8 kB (best case ≈16.7 kB gz, still 2.1× over). Closing
the gap requires either accepting a revised ≈16–17 kB gz budget for the current 3-Svelte-island
design, or rewriting MenuBrowser out of Svelte — a materially bigger decision than "trim the
islands" and one this measurement pass is flagging, not making.
