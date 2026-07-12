# G05 — Astro/Svelte FE parity (≈10%) — decision brief + execution blueprint

> **Date:** 2026-07-11 · **Gap owner doc:** audit `docs/research/2026-07-11-full-project-audit-dowiz-bebop.md`
> §5.1 bullet 3 + §8 ("FE ~10%") · **Status: RESEARCH + BLUEPRINT ONLY — nothing in this doc is
> implemented; no file outside this one was created or modified.**
> Every claim below is labeled VERIFIED (re-checked against the tree/git in this session) or
> CITED (external source).

---

## 1. Gap & evidence

**The gap.** Operator directive 2026-07-05 (`docs/design/rebuild-plan/astro-parity-matrix.md` header):
the target FE stack is **Astro 5 + Svelte 5 everywhere — React is interim-only**. Current state:

| Dimension | Target | Actual (VERIFIED 2026-07-11) |
|---|---|---|
| Islands | 27 (inventory 11 §7.1 roster) | **3** (`MenuBrowser`, `CartButton`, `LanguageSwitcher` — `rebuild/web/src/components/islands/`) |
| Routes | 27 addressable paths | **1** (`/s/[slug]`) |
| i18n keys migrated to Paraglide | 1,445 (now **1,515** — see §2.6) | **10** (0.7%) — `rebuild/web/messages/{sq,en,uk}.json` |
| Checkout / images / tracking | full parity matrix green | **absent entirely** (matrix rows D1–D24, E1–E16, B13–B24 all ❌) |
| Humans served by Astro | yes (post-parity) | **no** — `CUTOVER_ASTRO_UPSTREAM` unset; React SPA serves humans on staging + prod (`docs/ops/rebuild-cutover-h_t.json` S1 entry) |
| Blocking decision | — | **JS budget unresolved** since 2026-07-04 (`rebuild/web/ISLAND-BUDGET-OPTIONS.md`): measured 21.6 kB gz vs an "8 kB budget", Svelte floor 14.3 kB gz |

**Why it matters:** S1 (storefront-read) is the first cutover surface and its `readiness_ok` is
deliberately held FALSE until the Astro human page is parity-green (h_t frame: "S1 readiness_ok
ends FALSE until the Astro shell decision lands"). The unresolved budget decision therefore blocks
the FE program **and** the completion of the first cutover surface. Zero commits to `rebuild/web`
since 2026-07-05 (VERIFIED: `git log --oneline -- rebuild/web`).

**Precedent to respect:** on 2026-07-05 the Phase-A scaffold was accidentally served to humans and
had to be reverted (matrix header; reflection `2026-07-05-astro-scaffold-served-humans`, ledger #80
class). The matrix's rule — data-parity is NOT sufficient, agent-driven real-browser verification is
mandatory — is the governing verification method for everything below.

---

## 2. Research findings

### 2.1 THE HEADLINE FINDING: the "8 kB budget" is a requirements-drift artifact (VERIFIED)

The budget genealogy, traced through the tree:

1. **`REBUILD-MAP.md:64`** (Phase-A exit criteria, the program spine): *"Paraglide spike
   **(≤8 kB gz overhead check)**"* — the 8 kB number is the budget for **i18n overhead**, one line
   item of the spike, not for total page JS.
2. **`inventory/11-frontend-surface.md` §7.1** (the Lane-B FE authority doc): *"Storefront JS budget
   target **≤60–90 kB gz** total hydrated JS on `/s/[slug]` (excl. lazily-loaded maplibre on the
   order page)"* — restated in §5.2 ("consumes the entire ≤60–90 kB gz storefront JS budget").
   This is the only place a **total-JS** budget is defined, and it is 60–90 kB, not 8 kB.
3. **Commit `aa36ab0b`** (Astro build proof, memory `rebuild-decision-rust-astro-2026-07-04`):
   "FLAG-1: client JS 21.1kB gz **vs 8kB budget** (~2.6x)" — here the Paraglide-overhead check was
   first misapplied to total client JS.
4. **`ISLAND-BUDGET-OPTIONS.md`** then ran its entire (excellent) measurement pass against
   "Budget 8,000 B gz" with no source citation — inheriting the drift.

Measured against the correct budget: **the actual i18n overhead is 972 B gz** (compiled Paraglide
runtime chunk; per-message functions inline at <100 B each) — **8× under** its real 8 kB check —
and the **21.6 kB gz total is 2.8–4.2× UNDER the authoritative 60–90 kB budget**.

### 2.2 Context that settles the argument: the React oracle ships ~234 kB gz (VERIFIED)

`docs/design/rebuild-plan/03-frontend-build-bundle.md` §measured table — today's React storefront
critical path on `/s/:slug`: entry 4.1 + vendor 89.0 (react/react-dom/framer-motion) + shared
chunk w/ full 3-locale i18n catalog 85.0 + shared 13.4 + ClientRoutes 42.8 = **~234 kB gz JS**
(plus 66 kB gz CSS and an 820 kB icon woff2). The Astro scaffold at 21.6 kB gz is **~10.8× lighter
than the page it must reach parity with** — before any slimming.

### 2.3 The 14.3 kB Svelte floor is structural, not a build error (VERIFIED in-repo + CITED)

In-repo dependency-graph trace (`ISLAND-BUDGET-OPTIONS.md`, re-read this session): `template.js`
10.1 kB + `render.js` 2.8 kB + `attributes.js` 0.6 kB + Astro's `client.svelte.js` hydration
bootstrap 0.7 kB + `disclose-version.js` 0.1 kB ≈ **14.3 kB gz ships the moment ANY Svelte
component hydrates** — `render.js` is pulled by Astro's own bootstrap, not by island choice.

External corroboration (CITED): Svelte 5 deliberately trades a **larger base runtime** for much
smaller per-component output — measured base runtime ~5.9 kB min vs Svelte 4's ~2.3 kB, with
per-component cost dropping ~2.6 kB → ~1.3 kB
([geoffrich/component-size-benchmark](https://github.com/geoffrich/component-size-benchmark),
[khromov.se Svelte-5 bundle analysis](https://khromov.se/svelte-5-brings-up-to-50-bundle-size-decrease-for-existing-svelte-4-apps/),
[sveltejs/svelte discussion #11214](https://github.com/sveltejs/svelte/discussions/11214)).
Consequence: **the floor amortizes** — the in-repo per-island marginal cost is 0.5–2.7 kB gz
(measured table), so islands 4..27 get cheaper per unit, and Svelte 5 scales *better* than
Svelte 4 as the roster grows. An 8 kB total budget is unreachable with ANY Svelte 5 island;
it would force vanilla-JS for every interactive surface — i.e., silently revoke the operator's
own Astro+Svelte stack decision (memory `rebuild-decision-rust-astro-2026-07-04`).

### 2.4 Scaffold inventory & quality (VERIFIED)

`rebuild/web/` — committed clean (git status empty), **985 LOC** of source total:

| Piece | LOC | Quality assessment |
|---|---|---|
| `islands/MenuBrowser.svelte` (client:load) | 108 | Scroll-spy (IntersectionObserver, parity MenuPage.tsx:516-533), search filter, add-to-cart event delegation. **Note: it is mostly DOM manipulation over SSR HTML** (querySelectorAll), minimal Svelte reactivity — deliberate Phase-A thinness; the declarative payoff arrives with detail-sheet/modifiers (Phase B) |
| `islands/CartButton.svelte` (client:idle) | 49 | Sticky bar; reactive over shared store. **Defect: hardcodes `/100` minor-unit divisor + `toFixed(2)`** — ignores `currency.minor_unit`; React's `PriceDisplay` is the money-render authority (inventory §3.1). 🔴-adjacent display parity gap |
| `islands/LanguageSwitcher.svelte` (client:idle) | 55 | Wired to real compiled Paraglide runtime; `reload:false` in-memory only — SSR strings don't re-translate (matrix A6 ⚠️ confirmed accurate) |
| `lib/cart-store.svelte.ts` | 73 | **High quality**: integer-minor-units guard throws RangeError on float/negative (red-line #2, ledger #71) — a genuinely falsifiable money boundary |
| `lib/api-client.ts` + `api-types.d.ts` | 227 | Thin public-bypass wrapper, env-based base URL. **Latent bug found this audit: `getPublicTheme()` requests `${base}/api/public/theme/…` while base defaults to `/api` → `/api/api/public/theme/…` (Node registers `/api/public/theme/:slug`, `spa-proxy.ts:506`; Rust the same, `routes/theme.rs:26`). Menu/info paths are correct; theme is double-prefixed** — and `Promise.allSettled` + "theme failures degrade silently" in `[slug].astro` means SSR theming has plausibly NEVER worked against a real API and nothing went red. A textbook silent-degrade/VbM lesson |
| `pages/s/[slug].astro` + layout + 4 static components | 473 | Good SSR shape: parallel fetch, 404 parity, empty/error states, server-side theme CSS-var injection (kills React's theme-flash). **Hardcoded English strings** in error/empty states + search placeholder (i18n gap vs its own Paraglide spike) |

Build stack pinned and installed (`astro 5.18.2`, `svelte 5.56.4`, `@inlang/paraglide-js 2.20.2`,
`@astrojs/node 9.5.5`; node_modules present). Build proven green historically (`aa36ab0b`).
**No Playwright config exists in `rebuild/web`** — there is zero automated parity proof for the
Astro stack today.

### 2.5 Parity matrix classification by island-dependency (matrix + inventory cross-read)

The matrix (audit's "77-feature" count; its condensed rows span IDs A1–F3) classifies as:

- **Static / SSR-able — no hydration needed (~30 rows):** shell routes A1, theming A2–A5
  (derivePalette + fonts port to server-side render — the scaffold already proves the pattern),
  SSR locale render (A6 half), hero/reviews/state-chip B1–B3, chef's-pick + product card + R2
  photo chain + sold-out B13–B16, footer B29, venue-gate banners B30–B32 (render is SSR;
  *enforcement* needs the cart island to respect the flag), JSON-LD/OG/hreflang, `is_preview`
  shadow privacy (noindex — 🔴-adjacent), privacy page, 404. **This is the cheapest, highest-SEO
  half of the storefront and none of it pays the JS floor.**
- **Needs hydration, non-red-line (~28 rows):** category tabs/scroll-spy/search B4–B6 (✅ built),
  price sort B7, lenses/allergen B8–B9 (flags OFF), load/empty/error B10–B12, compare B17–B19
  (flag OFF), detail modal + modifiers B20–B24 (modifier min/max affects price → council review at
  port), add-to-cart UX B25–B28, currency A7, TMA/PWA/embed A8–A12, tracking stepper/map/SR/rating
  E-rows (WS island; totals display 🔴-adjacent).
- **Red-line 🔴 — council-before-port + operator-gated (all of D + parts of C/E):** checkout
  D1–D24 entirely (order POST, preflight/OTP, entry-photo PII, cash/tip, VAT display,
  idempotency), cart reprice/reconcile C1 (money), track-token exchange + totals E2/E13.
  The full FE red-line register is 23 rows (inventory §8) and extends into admin/courier islands.

### 2.6 React SPA drift since 07-05 — the gap widened slightly (VERIFIED)

`git log --since=2026-07-05 -- apps/web packages/ui` → 5 commits. Material to parity:

- **`330ff4ed` (07-07) + `77811204` (07-08): a NEW route** — cinematic landing at `/`
  (`LandingPage` + `HorizonDrift` + `CityPopRadio` + `BebopCharacter`, ~1,020 LOC + 444 CSS),
  replacing the inventory's `/ → Navigate /start` redirect row. **The 27-route census is now 28**;
  the landing is not in any parity matrix.
- **i18n catalog grew 1,445 → 1,515 keys** (re-measured: `grep -cE "^  '[^']+':"
  packages/ui/src/lib/i18n-catalog.ts` = 1,515; +70 landing keys ×3 locales).
- `reactAction.ts` lib (+179/+143 test) — infrastructure, not a user surface; its test file is
  also sitting **modified-uncommitted** in the working tree (audit §6.1).

No product-feature drift beyond the landing — checkout/menu/admin/courier untouched since 07-05.
Matrix staleness is bounded and enumerable (1 route + 70 keys + the already-flagged stale
`checkout-phone` selector in `client-path.visual.spec.ts:295,323`).

### 2.7 i18n plan state

Decision made and documented (inventory §5.2: **Paraglide-JS 2**, per-message tree-shaking,
13 dynamic-key families → explicit maps; converter + parity-gate re-target designed). Started:
**spike only** — 10 keys migrated verbatim, real compiler wired (`gen:messages` prebuild hook),
runtime measured at 972 B gz. Not started: the bulk converter, parity-gate re-target, dynamic-key
family maps, locale-aware routing (`url`/`cookie` strategy — currently `globalVariable` only, so
SSR strings never re-translate).

### 2.8 Dependency on G04 (Rust cutover) — FE parity is NOT blocked by it

The Astro FE consumes the HTTP contract through the same front-door that G04 flips; islands are
backend-agnostic by construction (`PUBLIC_API_BASE_URL` env). Per-surface reality:

| Island group | API surface | Rust state (h_t frame + this session) | Can build against Node today? |
|---|---|---|---|
| Storefront read (MenuBrowser etc.) | S1 public reads | 0-diff proven on staging 07-05 | **Yes** (identical contract) |
| Cart/Checkout 🔴 | S5 `POST /orders` + preflight/OTP | Rust create built dark; **preflight (E27) + `customer_track_grants` explicitly deferred** (h_t `remaining_before_prod`) | **Yes — and must** (Rust checkout path incomplete) |
| Tracking | S6 WS + customer status | WS opaque-passthrough built; track-grant deferred | Yes |
| Admin islands | S3/S5 owner routes | ~58 red-line keep-routes still Node | Yes (Node is the only complete impl) |
| Courier islands | S7 | partially ported | Yes |

**The dependency actually runs the other way:** S1's cutover `readiness_ok` is FALSE *waiting for
Astro parity*. Phase FE-1 below is the unblock for the first full-surface cutover, and the flip
mechanism already exists in prod code (`CUTOVER_ASTRO_UPSTREAM`, `packages/config/src/index.ts:66`,
wired at `apps/api/src/server.ts:441`) — serving Astro to humans is one env var once parity is
green, with the proven ~2.4 s rollback class.

**Honest caveat (audit §1 risk 4):** the h_t frame is 6 days stale and staging has been redeployed
from other lineages — re-probe staging flags before trusting any cutover state (audit rec #7).

---

## 3. Options & tradeoffs — the budget decision, front and center

The question as posed on 2026-07-04: *"revise the budget to ≈16–17 kB, or rewrite MenuBrowser in
vanilla JS"*. Research finding §2.1 reframes it: **the 8 kB number was never the total-JS budget** —
the authoritative Lane-B budget is 60–90 kB gz and the page is at 21.6 kB. The options, evaluated
with current evidence:

### Option A — Re-anchor the budget to the authoritative number, with a tight working target ⭐ RECOMMENDED

Ratify: **`/s/[slug]` critical-path JS ≤25 kB gz through Phase FE-1 (read parity), ≤35 kB gz once
the CartCheckout island lands, hard ceiling 60 kB gz** (the floor of the inventory's 60–90 band;
maplibre stays excluded/lazy per inventory §7.1). Enforced by a CI regression gate (procedure §3.1).

- **For (quantified):** 21.6 kB today leaves 3.4 kB headroom to finish read parity and ~13 kB for
  the entire checkout island — consistent with measured marginal island costs (1.5–2.7 kB each).
  10.8× lighter than the React page it replaces (§2.2). Preserves the operator's Svelte-everywhere
  stack decision. Effort ≈ 0 (a decision + one CI script).
- **Against:** concedes the aspirational 8 kB; the floor (14.3 kB) is paid on every storefront view.
  Mitigation: that floor is one-time-cached, HTTP/2-multiplexed, and ~6% of what ships today.

### Option B — Vanilla-JS islands (rewrite MenuBrowser out of Svelte)

The only path to ≤8 kB (ISLAND-BUDGET-OPTIONS option c-2: floor with MenuBrowser as sole Svelte
island = 16.4 kB; removing Svelte entirely ≈ 2–4 kB of hand-written DOM code).

- **For:** truly minimal JS; today's MenuBrowser is *already* mostly imperative DOM work (§2.4),
  so the immediate rewrite cost is genuinely low **for the current 108 LOC**.
- **Against (decisive):** (1) MenuBrowser's Phase-B growth — detail sheet, modifier min/max
  enforcement, media gallery, compare — is exactly where declarative reactivity prevents defects,
  and modifiers touch **price computation** (🔴-adjacent; hand-rolled reactivity in a money path
  is the wrong risk trade). (2) It quietly reverses the operator's stack decision for the largest
  storefront component and forks the codebase into two reactivity idioms ×24 remaining islands.
  (3) It exits the Svelte/Playwright component-testing path (ISLAND-BUDGET's own effort note:
  High, architecture-level). (4) The savings (~14 kB, one-time-cacheable) do not buy a measurable
  UX tier on the target market's devices vs the 234 kB status quo.

### Option C — Hybrid micro-optimizations (do regardless, under Option A)

From ISLAND-BUDGET options (a)+(b)-bonus, re-evaluated: **LanguageSwitcher → zero-JS server-rendered
locale links** is correct *independent of budget* — Paraglide's own `setLocale` default is
`reload:true` because SSR strings need a fresh render (§2.7); the current island is a stub that
only half-works. Saves 2,674 B + drops the Paraglide runtime chunk from the client (−972 B) if
CartButton reads its label from a `data-*` attr. CartButton-to-vanilla (−1.3 kB) is NOT worth the
bespoke-reactivity maintenance. Net: ≈18.0 kB gz after Phase FE-0, before Phase FE-1 growth.

### Option D — Switch island framework (Preact/solid) or drop hydration entirely

Rejected without further research: violates the 07-04 operator stack decision (Astro+**Svelte**),
resets the 3-island investment and the SvelteKit-extraction contingency (06 §2), and the public
size deltas (Preact ~4.5 kB base vs Svelte 5 ~6 kB min) don't justify a stack change.

### 3.1 The falsifiable measurement procedure (VbM — ships with whichever option is ratified)

Whole-app (valid while `/s/[slug]` is the only route; same methodology as ISLAND-BUDGET-OPTIONS):

```bash
cd /root/dowiz/rebuild/web && pnpm run build   # prebuild compiles Paraglide messages
total=0; for f in dist/client/_astro/*.js; do sz=$(gzip -c "$f" | wc -c); total=$((total+sz)); done
echo "client JS: ${total} B gz"   # RED if > threshold
```

Per-route (required from Phase FE-4 on, when >1 route exists — SSR output has no static HTML, so
measure what a rendered page actually references):

```bash
node dist/server/entry.mjs &   # or the staging URL
curl -s http://localhost:4321/s/demo \
  | grep -oE '(src|href)="/_astro/[^"]+\.js"' | grep -oE '/_astro/[^"]+\.js' | sort -u \
  | while read -r p; do gzip -c "dist/client${p}" | wc -c; done | paste -sd+ - | bc
```

(captures both `<script type="module" src>` island entries and `rel="modulepreload"` static-import
hrefs — the full transfer set for the route.)

**RED thresholds:** 25,000 B gz (`/s/[slug]`, Phase FE-1) · 35,000 B gz (post-CartCheckout) ·
60,000 B gz hard ceiling any storefront route. **Falsifiability proof (must be committed alongside
the gate):** a fixture build in which an island does `import * as m from '../paraglide/messages.js'`
(the full-catalog-import defect class, 61.4 kB gz) must turn the gate RED; reverting it returns
GREEN. A budget gate that cannot go red is a false-positive metric (VbM, CLAUDE.md).

---

## 4. Recommended execution blueprint (phased)

Sequencing principle: storefront read → cart → checkout(🔴) → tracking → misc → admin → courier;
every phase leaves Astro deployed **dark** until its explicit flip gate; React keeps serving humans
until Phase FE-8. Per-phase VbM = Playwright parity assertions vs the React oracle **on staging,
per matrix row** (agent-driven real browser per the matrix's verification directive; rate-limit ops
note: `--project=mobile --workers=1`). Effort unit = sessions (this project's unit).

### Phase FE-0 — Decision + harness + debt (1–2 sessions)

| # | Action | Gate | VbM proof | Effort |
|---|---|---|---|---|
| 0.1 | Ratify the budget (Option A + C numbers above); record in REBUILD-MAP + memory; supersede the 8 kB line with a pointer to its Paraglide-overhead origin | **OPERATOR-GATED (decision)** | n/a — a decision | 0.1 |
| 0.2 | Budget regression gate in CI (`rebuild/web` build + §3.1 script + thresholds file) | none (additive CI) | full-catalog-import fixture RED → revert GREEN | 0.3 |
| 0.3 | Playwright harness for Astro: config in `rebuild/web` (or `ASTRO_BASE_URL` project in root e2e); **fix the stale `checkout-phone` selector** (`client-path.visual.spec.ts:295,323` → `checkout-communication`/`checkout-comm-handle`) before any oracle use | none | one smoke spec green vs Astro dev + the SAME spec green vs React staging (dual-oracle wiring proven) | 0.4 |
| 0.4 | Fix scaffold defects found in §2.4: theme double-`/api` path; CartButton `/100` hardcode → `minor_unit`-aware format helper (port of PriceDisplay authority); hardcoded EN strings → Paraglide | none (dark) | **RED-first**: assertion "SSR HTML for a themed tenant contains `--brand-primary`" fails today, passes after fix (kills the silent-degrade class) | 0.3 |
| 0.5 | i18n bulk migration step: converter `i18n-catalog.ts` (1,515 keys) → `messages/{sq,en,uk}.json` (CI-run so both worlds stay in sync during the strangler window); re-target `scripts/i18n-parity.ts` at `messages/*.json`; 13 dynamic-key families → explicit typed maps | none | key-count equality assertion (catalog vs messages) + parity gate green + RED when one `uk` key is deleted | 0.6 |

### Phase FE-1 — Storefront read parity → S1 re-flip (4–6 sessions)

Scope: matrix groups A + B complete (islands: MenuBrowser grown to full B-group incl. detail
sheet + modifier display + hero/media chain + venue gates; StorefrontShellControls as zero-JS
locale + currency; InstallPrompt). Static: JSON-LD/OG/hreflang component, `is_preview` shadow
privacy (noindex + never-orderable — 🔴-adjacent, carry the E2E), derivePalette + font-allowlist
port server-side, R2 image resolution, per-tenant manifest. Includes the **NEW landing route** `/`
(Warm Cosmo-Noir, §2.6) or an explicit operator deferral of it (matrix delta row either way).

| Gate | **OPERATOR-GATED flip**: set `CUTOVER_ASTRO_UPSTREAM` on staging → soak → prod. Reversible in one env unset (2.4 s rollback class). Not money-red-line, but human-facing — the 07-05 incident precedent makes this an explicit human go |
|---|---|
| VbM | every A/B matrix row: selector renders + flow drives on Astro staging; visual net re-baselined (React baselines are NEEDS-REBASE by design, inventory §9.3); identical-failure-signature comparison vs React oracle for known-red staging specs; budget gate ≤25 kB; JSON-LD snapshot test; shadow-tenant noindex E2E |
| Unblocks | **G04/S1 `readiness_ok`** — first full-surface cutover becomes closeable |

### Phase FE-2 — Cart, C1–C7 (1–2 sessions)

CartCheckout island part 1: persisted cross-tab cart, reprice/reconcile-on-menu-version (🔴 money-
adjacent — port 1:1 with unit tests), drawer qty/remove/clear, empty state, free-delivery nudge,
checkout handoff stub. Gate: none (dark behind the flip already taken; checkout button can point at
the React checkout URL until FE-3). VbM: `client/cart.spec.ts` ports green; cart-store RangeError
RED test retained; budget ≤35 kB.

### Phase FE-3 — Checkout, D1–D24 — 🔴 RED-LINE, COUNCIL + OPERATOR-GATED (2–3 sessions + council)

Full council-before-port (per inventory §8 register): order POST w/ `idempotency_key`, preflight/
OTP modal, 6 messenger kinds, receiver, entry-photo PII upload, map-pin + contextual required
fields, cash/tip/VAT display, inline-never-toast money errors, call-restaurant fallback.
**Build against the Node API** (Rust S5 preflight E27 + track-grants are deferred — do not couple
to G04). Two standing bugs intersect here and need pre-decisions: (a) the live 3-kind 422
(`legacy.ts:48`) — carry-verbatim vs apply the operator-gated MessengerKind drafts; (b) the
**pre-existing staging checkout break** (flagged 07-04, never closed — audit §6.4) must be
root-caused first or the parity oracle is red on both sides. Gate: **OPERATOR go on the council
resolution + on serving Astro checkout to humans.** VbM: `flow-simpl-s1-sheet-checkout`,
`client-checkout-happy-path`, `flow-order-creation` green on Astro staging with a real staged
order; RED case: preflight `hard_block` renders review-cart state.

### Phase FE-4 — Tracking, E1–E16 (2 sessions)

OrderTracker island: WS live status (reconnect/backoff parity, drop URL-token auth per inventory
§6.2 — 🔴 council note), track-token exchange 🔴, stepper, honest ETA range, lazy maplibre, rating,
messages, SR announcer. VbM: `client/status*.spec.ts` + `flow-customer-track-link` green; per-route
budget gate live (§3.1 per-route mode).

### Phase FE-5 — Misc top-level (1–2 sessions)

`/start` OnboardingWizard 🔴 (TG claim), `/login` AuthLogin 🔴, `/auth/callback` 🔴, `/claim`
ClaimFlow 🔴 (fragment-token scrub), privacy (static + CI content-hash), 404. All auth islands are
red-line rows → council per inventory §8. Gate: operator go per auth surface.

### Phase FE-6 — Admin, 11 islands (5–7 sessions) — needs its own matrix first (harness task #12)

Order of build: ResponsiveDialog + primitive set FIRST (used by 19 consumers — inventory's
"build FIRST" note) → AdminShell → OrdersBoard 🔴 → MenuManager 🔴 (largest: 1,405+340+266+158 LOC
source) → Settings 🔴 → Couriers 🔴 → Branding → Analytics (converge both map stacks onto one
maplibre wrapper) → Promotions 🔴 → SupplyLibrary → CRM 🔴 → Activation 🔴. Budget looser behind
auth (no public floor concern). Gate: **admin parity matrix authored + green; operator flip.**

### Phase FE-7 — Courier, 6 islands (2–3 sessions) — courier matrix first

CourierTasks, Delivery 🔴 (cash-as-proof; **carry verbatim** the `cashCollected`-not-transmitted
behavior + the `atob`-JWT parse per the fix-vs-carry rule — both are flagged council candidates,
not silent fixes), Shift, Earnings 🔴/History, CourierLogin 🔴/InviteRedeem 🔴. Wake-lock = NEW
capability, flag-gated. Gate: courier matrix green; operator flip.

### Phase FE-8 — React decommission (0.5 session) — **OPERATOR-GATED (Phase-D class)**

All three matrices green + 48 h soak → delete `apps/web` from the build, retire the SPA-shell
serving path. Irreversible-ish (git-recoverable) — explicit operator sign-off, joins the REV-C10
Phase-D owner+date gate. VoiceControl island stays dark pending STOP-DESIGN-B regardless.

### Effort summary & G04 interplay

- **Total: ≈ 18–27 sessions** (FE-0 1–2 · FE-1 4–6 · FE-2 1–2 · FE-3 2–3+council · FE-4 2 ·
  FE-5 1–2 · FE-6 5–7 · FE-7 2–3 · FE-8 0.5). To the first meaningful milestone (S1 human re-flip):
  **5–8 sessions** (FE-0 + FE-1).
- **Does FE parity make sense to fund before the G04 cutover decision?** Split answer:
  **FE-0 yes, unconditionally** — it is ~free, kills the blocking decision, and adds the budget
  gate + parity harness that any future FE work needs. **FE-1 yes, IF the rebuild remains the
  program** — it is the only path to closing S1, is not blocked by any Rust state, and is fully
  reversible. **FE-2+ should wait** for (a) the staging cutover re-baseline (audit rec #7 — the
  h_t frame is 6 days stale) and (b) the arbiter doc (audit rec #2) ranking rebuild-cutover vs
  MVP-exit vs OSS vs bebop — funding 13+ sessions of admin/courier ports while the program spine
  is contested repeats the serial-pivot pattern the audit names as risk #2. Also on the record:
  the GDPR prod trio (audit rec #1) outranks all of G05 in harm terms and costs <1 session.

---

## 5. Risks & rollback

| Risk | Severity | Mitigation / rollback |
|---|---|---|
| Repeat of the 07-05 "scaffold served to humans" incident | High | Every flip operator-gated; Astro stays dark by default; flip = 1 env var, unset = rollback (2.4 s class, proven on S1 mechanism 07-05) |
| Parity oracle rot: React baselines stale, `checkout-phone` selector dead, staging checkout pre-broken, 429 rate-limit false-fails | High (silently invalidates all VbM proofs) | FE-0.3 fixes the selector; root-cause the checkout break BEFORE FE-3; `--workers=1` staging runs; re-baseline visual net per ported surface (NEEDS-REBASE is by design) |
| Silent-degrade masking (theme bug class — `allSettled` + no assertion = broken feature reads green) | Medium | FE-0.4 RED-first theme assertion; standing rule: every SSR degrade path gets one falsifiable "feature actually present" assertion |
| Budget creep as islands 4..27 land | Medium | CI gate (§3.1) with committed thresholds + RED fixture; threshold raises are operator decisions, not edits |
| Matrix staleness vs a still-moving React tree (landing route, +70 keys since 07-04) | Medium | FE-0.5 CI key-count equality; matrix delta row per React commit touching `apps/web` until FE-8; freeze-window during each phase's port |
| i18n dual-SSOT drift during the long strangler window | Medium | Converter runs in CI (both worlds sync); parity gate red on divergence |
| Operator attention elsewhere (bebop) — blueprint joins 4 competing programs | **Highest, non-technical** | This doc funds only FE-0 unconditionally; FE-1+ explicitly contingent on the arbiter doc (audit rec #2) |
| Svelte 5 / Astro major-version churn mid-program | Low | Versions pinned; `@astrojs/svelte` 7.x is the newest line for Astro 5 (README verified); upgrade decisions batch at phase boundaries |

Rollback posture overall: **every phase is routing-reversible** (React remains deployed and
serving until FE-8); no data or schema surface is touched by any FE phase; the only irreversible
step (FE-8 decommission) is last and operator-gated.

---

## 6. Operator decision points (in order of appearance)

1. **[NOW — the blocker] Budget ratification (FE-0.1).** Recommendation: **Option A + C** —
   re-anchor to the authoritative 60–90 kB Lane-B budget with working targets 25/35/60 kB gz and
   the §3.1 CI gate; adopt zero-JS LanguageSwitcher; reject the vanilla-MenuBrowser rewrite.
   Cost of deciding: zero code. Everything downstream is blocked on this signature.
2. **[FE-1 exit] S1 human re-flip** — set `CUTOVER_ASTRO_UPSTREAM` (staging, then prod) once the
   A/B matrix rows are green. Reversible; human-facing precedent (07-05) makes it an explicit go.
3. **[FE-1 scope] Landing page** — port the Warm Cosmo-Noir landing to Astro in FE-1, or defer
   `/` to a later phase (it is post-07-05 React work, absent from the matrix).
4. **[FE-3 entry] 🔴 Checkout council + go** — the money island. Includes the 3-kind-422
   carry-vs-fix call (MessengerKind drafts are already operator-gated queue item #5).
5. **[FE-5] 🔴 Auth-surface islands go** (login/claim/callback/invite — council per red-line register).
6. **[FE-6/7 entry] Admin + courier parity matrices** — authorize authoring them (harness task #12)
   before any admin/courier island is built.
7. **[FE-8] React decommission** — Phase-D-class sign-off (owner + date, REV-C10 discipline).
8. **[Standing] Funding order** — this blueprint recommends FE-0 now, FE-1 only if the arbiter doc
   (audit rec #2) confirms the rebuild as the program, FE-2+ after the staging re-baseline
   (audit rec #7). The GDPR prod trio (audit rec #1) should ship before any of it.

---

### Appendix — evidence quick-index (all VERIFIED this session unless CITED)

- Scaffold: `rebuild/web/` — 985 LOC, 3 islands (49/55/108 LOC), committed clean; deps installed;
  no Playwright config; 10/1,515 i18n keys migrated.
- Budget genealogy: `REBUILD-MAP.md:64` (8 kB = Paraglide overhead check) vs
  `inventory/11-frontend-surface.md` §7.1 + §5.2 (60–90 kB total budget) vs
  `ISLAND-BUDGET-OPTIONS.md` ("Budget 8,000 B gz", uncited) vs memory `aa36ab0b` FLAG-1.
- Measurements: 21,612 B gz total / 14,310 B Svelte-runtime floor / 972 B Paraglide runtime
  (`ISLAND-BUDGET-OPTIONS.md` per-chunk table); React storefront ~234 kB gz
  (`03-frontend-build-bundle.md`).
- Theme path bug: `rebuild/web/src/lib/api-client.ts` `getPublicTheme` (`${base}/api/public/theme`)
  vs `apps/api/src/routes/spa-proxy.ts:506` + `rebuild/crates/api/src/routes/theme.rs:26`.
- Flip mechanism: `packages/config/src/index.ts:66` (`CUTOVER_ASTRO_UPSTREAM`),
  `apps/api/src/server.ts:441`; S1 state: `docs/ops/rebuild-cutover-h_t.json`.
- React drift: commits `330ff4ed`, `77811204`; catalog 1,515 keys (re-measured).
- Svelte 5 size context (CITED): [geoffrich/component-size-benchmark](https://github.com/geoffrich/component-size-benchmark) ·
  [khromov.se](https://khromov.se/svelte-5-brings-up-to-50-bundle-size-decrease-for-existing-svelte-4-apps/) ·
  [sveltejs/svelte#11214](https://github.com/sveltejs/svelte/discussions/11214).

*Written 2026-07-11 by a read-only research session. The only file created is this blueprint;
working tree, branches, and worktrees left exactly as found.*
