# DeliveryOS rebuild — Astro 5 + Svelte 5 storefront shell (Phase A spike)

Lane R2 of the complete-rebuild program (`docs/design/rebuild-plan/REBUILD-MAP.md` §3 Phase A).
Self-contained package: **not** part of the root pnpm workspace (`pnpm-workspace.yaml` globs
`apps/*` / `packages/*` / `tools/*` / `spikes/*` only — `rebuild/web` matches none of them, same
precedent as `rebuild/` the Rust Cargo workspace already established).

## STATUS: scaffold complete, dependency install BLOCKED — needs one human approval

Every source file in this tree (Astro pages/layouts/static components, Svelte 5 islands, i18n
messages, hand-derived OpenAPI types, the typed API client) is written and ready. **The one thing
this lane could not do itself is create `package.json` and run `npm install`** — this worktree's
governance hooks hard-block it:

- `.claude/hooks/protect-paths.sh` blocks any `Write`/`Edit` tool call to a path ending in
  `package.json` (exit 2, "requires manual approval") — this fired the first time I tried.
- `.claude/hooks/guard-bash.sh` independently blocks `npm install <pkg>` / `pnpm add` / `yarn add`
  as "dependency mutations... New deps go through the council/human."
- I confirmed `astro`/`svelte`/`@inlang/paraglide-js`/`openapi-typescript` are not already present
  anywhere in this environment's `node_modules` to reuse without a fresh install.
- I deliberately did **not** route around this via `npx <pkg>` (still executes unreviewed
  third-party code), `npm create astro@latest` (same), or a Bash heredoc into `package.json`
  (technically not pattern-matched by `guard-bash.sh`'s mirror list, which is very likely a gap in
  that list rather than an intended allowance — worth a follow-up ticket, not something to exploit).
  Ran this through the `doubt-escalation` skill; its own text confirms `guard-bash.sh` is
  "complementary to `protect-paths.sh` — it never weakens that hard block," i.e. this is a genuine
  human-gate, not an ambiguity a stronger model or council could resolve on my behalf.

### Unblock recipe (one approval, then fully mechanical)

1. A human (or a permission-elevated session) reviews `rebuild/web/package.json.pending` — the
   exact intended manifest, pinned to the major versions the task specified (Astro 5, Svelte 5,
   Paraglide-JS 2), verified compatible via `npm view <pkg>@<version> peerDependencies` during this
   session (`@astrojs/svelte@7.2.5` is the newest 7.x line that still declares `astro: ^5.0.0` —
   the 8.x/9.x lines require Astro 6/7).
2. `mv package.json.pending package.json`
3. `cd rebuild/web && npm install` (generates `package-lock.json` — not in either hook's protected
   list, only the root `pnpm-lock.yaml` is guarded)
4. `npm run gen:messages` — compiles `messages/{sq,en,uk}.json` via the real
   `@inlang/paraglide-js` compiler into `src/paraglide/` (see "i18n" below — replaces
   `src/lib/paraglide-stub.ts` + `src/lib/locale-data/*`, which are hand-written stand-ins with the
   **same call shape** Paraglide 2 generates, so swapping is an import-path change only)
5. `npx openapi-typescript ../../docs/design/rebuild-plan/openapi-contracts/openapi-s1-storefront-read.yaml -o src/lib/api-types.d.ts`
   — regenerates the real typed client from the S1 YAML; diff against the current hand-transcribed
   file (should be near-identical; any drift is a real contract nuance to double check)
6. `npm run build` && `npm run check` — paste tail output per the Mandatory Proof Rule
7. `du -sh dist/client/_astro/*` (or `ls -la` per chunk) for the real per-route bundle-size table

## What's built (file tree)

```
rebuild/web/
├── .gitignore
├── package.json.pending        ← rename to package.json once approved (§ above)
├── astro.config.mjs            ← output:'server' (SSR, no build-time fetches), @astrojs/svelte,
│                                  PUBLIC_API_BASE_URL env schema (no hardcoded host)
├── tsconfig.json
├── project.inlang/settings.json  ← Paraglide 2 project config, baseLocale sq, locales sq/en/uk
├── messages/{sq,en,uk}.json     ← 10 sample keys, values copied verbatim from
│                                  packages/ui/src/lib/i18n-catalog.ts (dotted keys → underscored,
│                                  Paraglide message keys must be valid JS identifiers)
└── src/
    ├── env.d.ts
    ├── lib/
    │   ├── api-types.d.ts       ← hand-transcribed from the S1 YAML (openapi-typescript stand-in,
    │   │                          see file header for the exact regen command)
    │   ├── api-client.ts        ← typed fetch wrapper: getPublicMenu / getPublicLocationInfo /
    │   │                          getPublicTheme, base URL from env, zero hardcoded hosts
    │   ├── cart-store.svelte.ts ← shared Svelte-5-runes cart singleton (module $state), consumed
    │   │                          by MenuBrowser + CartButton islands (parity: CartProvider.tsx) —
    │   │                          note the `.svelte.ts` extension: runes only compile in `.svelte`
    │   │                          or `.svelte.js/.ts` files, a plain `.ts` module fails silently
    │   ├── paraglide-stub.ts    ← hand-written stand-in for `src/paraglide/messages.js`
    │   │                          (identical call shape: `cart_title(locale?)`) — see "i18n" below
    │   └── locale-data/{en,sq,uk}.ts  ← message tables backing the stub
    ├── styles/tokens.css        ← placeholder --brand-* vars only (no design-system port — that's
    │                              Phase-B scope per REBUILD-MAP inventory 11 §3.3)
    ├── components/
    │   ├── VenueHeader.astro    ← static SSR: name, open/closed/busy chip, rating, address
    │   ├── CategoryChips.astro ← static <a href="#id"> anchors (zero-JS scroll works natively;
    │   │                          MenuBrowser progressively enhances with smooth-scroll + spy)
    │   ├── ProductCard.astro   ← static SSR product card with data-* attrs for event delegation
    │   ├── StorefrontFooter.astro
    │   └── islands/
    │       ├── MenuBrowser.svelte     ← client:load — scroll-spy (IntersectionObserver, parity:
    │       │                            MenuPage.tsx:516-533), search filter, add-to-cart delegation
    │       ├── CartButton.svelte      ← client:idle — sticky cart bar (parity: ClientLayout.tsx
    │       │                            StickyActionBar)
    │       └── LanguageSwitcher.svelte ← client:idle — wired to the Paraglide stand-in
    ├── layouts/StorefrontLayout.astro ← header chrome + SERVER-SIDE theme CSS-var injection
    │                                     (kills the theme-flash the current React app has)
    └── pages/s/[slug].astro    ← the S1 SSR page: parallel fetch menu+info+theme per request,
                                    404 on unknown location, empty-menu / fetch-error states,
                                    renders SSR grid + mounts the 3 islands
```

## Island architecture vs. REBUILD-MAP's 27-island roster

This skeleton builds **3 of the 27** islands (inventory 11 §7.1 rows 1/2/4: MenuBrowser,
CartCheckout-lite as CartButton, StorefrontShellControls as LanguageSwitcher) — the storefront-read
(S1) surface only, matching Phase A scope. CartButton here is a read-display stand-in for the full
🔴 CartCheckout island (cart sheet + checkout sheet + OTP modal) — money-adjacent checkout logic is
explicitly S5/Phase-B, out of scope for a read-only spike.

Hydration directives match the roster: MenuBrowser `client:load` (needs to be interactive
immediately — scroll-spy + search + add-to-cart), CartButton and LanguageSwitcher `client:idle`
(non-critical, hydrate once idle). The roster's "client:idle, eager-upgraded to client:load on
first add-to-cart" nuance for CartCheckout is noted in `CartButton.svelte`'s header comment as a
Phase-B follow-up — Astro doesn't have a first-class "upgrade hydration at runtime" primitive today.

## i18n: Paraglide-JS 2 spike

**Messages** (`messages/{sq,en,uk}.json`, 10 keys) are copied **verbatim** from
`packages/ui/src/lib/i18n-catalog.ts`: `client.menu`, `cart.title`, `cart.empty`, `cart.total`,
`cart.checkout`, `cart.clear`, `cart.increase`, `cart.decrease`, `checkout.title`,
`client.closed_title` → renamed to `client_menu`, `cart_title`, etc. (dots aren't legal in Paraglide
message keys — a compiler constraint, not a stylistic choice; REBUILD-MAP inventory 11 §5.2 already
flags this for the 13 dynamic-key families).

**Overhead measurement — could not run the real compiler (blocker above).** As a substitute, I
hand-built a proxy bundle matching Paraglide-JS 2's **publicly documented** compiled-message shape
(one small function per message + the runtime's `getLocale`/`setLocale`/`isLocale` surface — see
paraglidejs.com "Compiled Bundle" docs) for these exact 10 messages × 3 locales, and gzip'd it for
real:

```
$ wc -c paraglide-proxy-bundle.js && gzip -9 -c paraglide-proxy-bundle.js | wc -c
4731  (raw)
1567  (gzip -9)
```

**≈1.53 kB gz for 10 messages + runtime — well inside the ≤8 kB gz budget** (REBUILD-MAP §3 Phase A
gate / inventory 11 §5.2 fallback trigger "storefront island i18n overhead measured > ~8 kB gz").
Caveat: this is a **hand-modeled proxy**, not `astro build` output — real per-message output may
carry a few hundred more bytes of compiler boilerplate (import wiring, tree-shaking artifacts), but
even 3-4x this number stays under budget. At 1,445 real catalog keys the picture doesn't change:
Paraglide's whole value proposition is **per-message** tree-shaking (REBUILD-MAP inventory 11 §5.2
point 2) — MenuBrowser only pulls in the ~10-20 keys it actually calls, not the 61.4 kB gz whole
catalog (C7). Re-run the measurement for real once the compiler is installed (recipe step 4 above)
before treating this as final.

## Parity notes vs. `apps/web/src/pages/client/MenuPage.tsx` + `ClientLayout.tsx`

| Current (React) | This skeleton | Gap / follow-up |
|---|---|---|
| Category chips = jump-nav anchors, IntersectionObserver scroll-spy (MenuPage.tsx:493-533) | `CategoryChips.astro` (static `<a href="#id">`) + `MenuBrowser.svelte` (IntersectionObserver, smooth-scroll) | 1:1 — same anchor-not-filter model |
| ProductCard: image/price/prep/allergen chips/taste/chefPick/sold-out | `ProductCard.astro`: image/name/description/price/prep only | allergen/taste/chefPick/compare — Phase-B (flags default OFF today anyway per inventory 11 §6.3) |
| Hero: image→video→StylizedMap fallback chain + OSRM client ETA | Not built | explicitly deferred — client-only enhancement, not structural |
| Cart: CartProvider context, sticky bar, cart sheet, free-delivery nudge | `cart-store.ts` (module runes singleton) + `CartButton.svelte` (bar only, no sheet) | cart sheet + free-delivery nudge + checkout — 🔴 S5/Phase-B, out of scope for S1 read spike |
| ThemeProvider: client-side derivePalette, flash-on-load | `StorefrontLayout.astro`: SSR-rendered `<html style="--brand-*">` | **improvement, not a gap** — kills the theme-flash bug noted in memory `client-theme-palette-rootcause-2026-06-21` |
| Bot vs. human SSR split (`ssr.ts`/`spa-shell.ts`) | ONE Astro SSR render for everyone | matches REBUILD-MAP inventory 11 §4.1 "bot/human split disappears" — by design |
| JSON-LD / OG / hreflang / robots / sitemap | Not built | explicitly out of this lane's scope (§4.1 says Astro component, Lane C decision on robots/sitemap ownership) |
| `is_preview` shadow-tenant handling (never-orderable, generic OG) | Not built | 🔴 privacy invariant (P6-2/P6-3) — flagging for a dedicated pass before this ships, not silently droppable |

## Traceability (FE route/page/component → target artifact → status)

See the full row-by-row table in this lane's final report to the lead (task instruction: don't
edit `traceability.csv` directly). Summary: 1 of 27 routes (`/s/[slug]`) scaffolded; 3 of 27
islands scaffolded (MenuBrowser, CartButton, LanguageSwitcher); i18n decision spiked and measured
(proxy); OpenAPI client hand-derived pending real codegen. Build/check/real-bundle-size proof is
**blocked** pending the package.json approval above.
