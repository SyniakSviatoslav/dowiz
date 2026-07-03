# Admin core — design review

Reviewer: senior product designer. Scope: 10 screens (5 surfaces × desktop/mobile) of the dowiz admin. The experimental cream/paper skin is ignored where it appears; critiques target structure, hierarchy, and UX.

---

### Admin Orders / Dashboard (desktop)
- **Reads as:** A dense live-orders board fronted by a 5-stat strip, but the emotional signal (every visible order is "Refuzuar" / overdue by 1000+ min) is buried under neutral styling.
- **Suggestions:**
  - The five KPI tiles all use the same weight and the live color logic is inverted — "Gati: 0" is green while "Në pritje: 0" is orange. Reserve color for *attention*: orange/red only when a number needs action (overdue, waiting), muted gray for zero/idle. Right now color reads as decoration, not status.
  - "Vonesë! (1028 min)" is the single most important data point on each card yet it sits as plain red text mid-card. Promote it to a pill/badge anchored top-right next to the status, and format human-readable ("17h vonesë" not "1028 min").
  - The onboarding welcome banner ("Mirë se vini në Panel") consumes the prime top slot on a screen with 17 live orders. Make it dismissible-once and collapse permanently after first session — it competes with operational data every load.
  - Filter chips ("Të Gjitha / Në Pritje / Konfirmuar...") and the Live/Historiku toggle and the sort button are three separate control clusters on one row — visually noisy. Group the segmented Live/Historiku with the status chips, and move "Eksporto CSV" + the slug-copy field into an overflow/utility area; they are not per-order workflow.
  - Order cards repeat the label column ("Klienti / Telefoni / deri / Artikujt") on every card — high redundancy. Tighten to a label-light layout (icon + value) or a compact table view toggle for operators triaging volume.
  - "Pa OTP" and "Rep: 75" badges are unexplained micro-jargon. Add tooltips or spell them out; an operator scanning fast shouldn't have to decode them.

### Admin Orders (mobile)
- **Reads as:** A faithful vertical reflow, but the top 60% of the first screen is chrome (header + lang switcher + banner + KPI grid + filters) before a single order appears.
- **Suggestions:**
  - Five KPIs wrap into a 3+2 grid that pushes the first order card entirely below the fold. Collapse KPIs into a single horizontal scroll strip or a 2-row compact summary so at least one order is visible on load — operators open this to see orders, not stats.
  - The filter chips overflow horizontally with the last chip ("Duke U Përgatit...") clipped mid-word; add a fade/scroll affordance and ensure no chip truncates its own label.
  - "Eksporto CSV" gets a full-width prominent button on mobile where it's rarely used on a phone; demote it. Reserve prominent mobile real estate for the primary action (filtering/refreshing live orders).
  - The dismiss "×" on the welcome banner is a tiny top-right tap target near the edge — enlarge to a 44px hit area and pull it in from the corner.
  - The card label column ("Klienti / Telefoni") wastes ~40% of narrow-screen width on labels. Drop to value-first lines with small inline icons to reclaim horizontal space for the actual customer/phone values.

### Admin Menu Manager (desktop)
- **Reads as:** A clean product grid, but the page top is a stack of full-width banner-buttons that bury the actual product management.
- **Suggestions:**
  - "Shëno kuzhinën si të zënë" and "Oraret e disponueshmërisë" are two full-width bars stacked above the toolbar — they read as content, not controls, and push products down. Make the kitchen-busy toggle a compact switch in the toolbar (with the busy state surfacing a persistent thin banner only when active), and collapse availability hours into a settings panel.
  - Every product card shows a gray image-placeholder icon (no photos uploaded). With 50 products this reads as broken. Either de-emphasize the thumbnail (smaller, neutral) until images exist, or add an inline "Add photo" affordance on the placeholder so the empty state becomes an action.
  - The "I disponueshëm" toggle + label repeats on all 50 cards and is the only per-card control besides edit/delete. The edit (pencil) and delete (trash) icons sit detached bottom-right with no labels — add tooltips, and consider hover-reveal to reduce per-card visual noise across a 50-item grid.
  - Category chips overflow with the last one ("Test-Cat-178214886...") truncated and a dangling "M..." chip clipped at the right edge — clean up test-category naming display and add a scroll/overflow control.
  - The "+" add-category button sits orphaned next to a "Kategori e re..." input and the filter funnel — three different add/filter affordances crowd the toolbar. Consolidate: one clear "Add product" primary action is currently missing from the visible toolbar entirely (only "Add category" is present). Add an unmistakable primary "Shto produkt" CTA.
  - The "42" badge and "Soje" (allergen) tag on the Pita card appear without legend — clarify what the number represents (recipe lines? stock?).

### Admin Menu Manager (mobile)
- **Reads as:** Same banner-heavy top; by the time you scroll past kitchen-busy + hours + search + sort/filter + category input + chips, only one product card is visible.
- **Suggestions:**
  - Six stacked control rows precede the first product. Collapse the kitchen-busy bar and availability-hours into a single "Menu settings" expandable, and move sort/filter into a single compact control, so products appear within the first viewport.
  - Product cards are nearly empty (placeholder image + name + price + toggle) yet take ~140px each — tighten card height on mobile so 3–4 products fit per screen for faster scanning.
  - Edit/delete icons sit bottom-right of each card at small size; on mobile these need 44px tap targets and clearer separation so a thumb doesn't hit delete instead of edit.
  - "Importo PDF" is centered as a lone text-link under the title — easy to miss and ambiguous as an affordance. Style it as a clear secondary button or move it into the menu-settings area.
  - The "+" add button next to "Kategori e re..." is the only visible add control; a primary "Add product" CTA (e.g., a FAB) is missing on the surface where it's most needed.

### Admin Settings (desktop)
- **Reads as:** A clean, well-spaced settings form grouped into cards — the most conventional and successful of the surfaces.
- **Suggestions:**
  - The form is a single tall scroll (Store details → Delivery config → map → presumably hours/notifications below). For a multi-section settings page, add a left/top section nav or anchor tabs so operators jump to "Notifications" without scrolling past the map.
  - There is no visible Save affordance in the viewport. Settings forms need an explicit, sticky save bar (or per-section save) with a clear dirty/saved state — relying on a save button far below the fold is a data-loss risk.
  - Field labels ("Tarifa e dorëzimit (ALL)", "Porosia Minimale (ALL)") are good, but currency-typed numeric fields should show the unit as an input adornment ("200 ALL") and validate numeric input, not just a bare number field.
  - The map for delivery zone has zoom controls but no visible radius handle or "current radius: X km" readout in this crop — surface the actual zone value as text alongside the map so it's editable/confirmable without manipulating the map.
  - Section headers ("Detajet e Dyqanit", "Konfigurimi i Dorëzimit") are low-contrast gray on dark — bump weight/contrast so the form's structure is scannable.

### Admin Settings (mobile)
- **Reads as:** Clean single-column reflow of the form — works well, the strongest mobile surface here.
- **Suggestions:**
  - Same missing-save concern, amplified on mobile: add a sticky bottom save bar (above the tab bar) with saved/dirty state so changes aren't lost on a long scroll.
  - The delivery-zone map is squeezed to a thin sliver at the bottom of the visible area; on mobile give it a usable minimum height (or a "tap to edit zone" expand) since pinch-zooming a sliver map is poor ergonomics.
  - Inputs are appropriately full-width and tappable — good. Ensure the numeric fields trigger the numeric keyboard (inputmode="numeric") for tariff/minimum-order.
  - Section header contrast is even weaker at this size — strengthen so "Detajet e Dyqanit" / "Konfigurimi i Dorëzimit" clearly delineate groups.

### Admin Branding (desktop)
- **Reads as:** A strong three-pane concept — auto-generate + manual controls on the left, live phone preview on the right — but the preview exposes a real contrast bug.
- **Suggestions:**
  - The live preview shows the chosen palette producing near-invisible text: "200 ALL" red price on a pale pink card, and pale headings on white — a real WCAG failure the branding tool is actively generating. Add a contrast check that warns when text/background pairs fall below AA, and auto-derive a safe text color rather than letting users ship dark-on-dark / pale-on-pale.
  - The three color fields (Primare / Sfondit / Tekstit) use hex inputs with a swatch but no color picker is evident — add a clickable swatch that opens a picker, and show a live mini-preview of each token in context, not just the hex.
  - "Gjenero automatikisht nga brandi juaj" (auto-generate from URL) is a powerful entry point but visually equal to the manual color section — make it the clear primary path (it's the magic feature) with manual colors as the "fine-tune" fallback below.
  - The logo block has both "Choose File" and an "ose URL e Logos" (or logo URL) input — two ways to set a logo with unclear precedence. Clarify which wins, and show the current logo preview prominently.
  - The two URL readouts at the bottom (SSR / SPA preview links) are developer-facing and low-value to a restaurant owner — demote or hide behind an "advanced/share" disclosure.
  - The left nav text is noticeably lower-contrast than on other screens (the branding preview seems to have dimmed the chrome) — verify the admin chrome isn't inheriting the tenant's draft palette.

### Admin Branding (mobile)
- **Reads as:** Clean vertical reflow of the controls, but the live preview (the whole point of branding) is pushed far below the fold and not visible at all on first screen.
- **Suggestions:**
  - With no preview visible, a mobile user changes hex values blind. Add a small sticky/inline preview chip (a single branded card sample) near the color inputs, or a "Preview" toggle, so changes are seen without hunting for the phone mock.
  - The auto-generate card is well-sized and the "Gjenero" CTA is a strong full-width primary — good. Keep this as the hero action.
  - Color swatch + hex rows are clean and tappable; make the swatch itself open a native color picker for thumb-friendly editing instead of forcing hex entry.
  - Same contrast-guardrail need as desktop: warn inline when the chosen text/background combo is illegible before it ships to the live storefront.

### Admin Couriers (desktop)
- **Reads as:** A long flat list where every single courier is an identical "Korrier në pritje / Pa telefon ende / 0 dërgesa / Online" row — reads as seed/test data and exposes a weak empty-ish state.
- **Suggestions:**
  - "53 online" yet every row says "Pa telefon ende" (no phone) and "0 dërgesa" — these are clearly unprovisioned placeholder couriers. Add a real empty/onboarding state ("No active couriers yet — invite your first courier") and filter or visually separate provisioned vs. pending couriers instead of a wall of identical rows.
  - Every avatar is a generic "?" and every name is identical — there's zero scannability. Once real, show initials/photo and name; for pending, label the row clearly as "Pending invite" with the invite action inline, not a fake "Online" status.
  - "Online" appears in green as a dropdown on every row but pairs with "Pa telefon ende" — a courier with no phone can't be meaningfully online. Reconcile the status model; don't show "Online" for unprovisioned couriers.
  - The "0 dërgesa" metric and "Online" dropdown are visually competing on the right edge with unclear hierarchy — make deliveries-count secondary text and the status control a clear pill-style select.
  - Long identical lists need pagination or virtualization plus a count ("Showing 53") and sort/filter (by status, by deliveries) — searching is the only affordance currently and it can't disambiguate identical names.

### Admin Couriers (mobile)
- **Reads as:** Same identical-row list, slightly better proportioned, but "0 dërgesa" appears dropped from rows so each row is just name + "Pa telefon ende" + Online.
- **Suggestions:**
  - The deliveries count visible on desktop seems absent on mobile rows — keep a consistent, compact metric (e.g., a small "0 ✓" chip) so mobile isn't a reduced-information dead end.
  - Rows are tappable-sized but the only action is the "Online" status dropdown on the right edge — far from the thumb and small. Move primary courier actions into the row tap (open detail) and keep status as a clearly-tappable pill.
  - Same fundamental empty-state problem: a phone screen full of "Korrier në pritje / Pa telefon ende" looks broken. Lead with an onboarding/empty state and an "Invite courier" CTA when no real couriers exist.
  - "+ Shto Postier" is a strong primary at the top — good; ensure it's the obvious next step when the list is all-placeholder.

---

## Cross-screen themes
1. **Top-of-page chrome buries the operational content.** Welcome banners, full-width toggle bars (kitchen-busy, availability hours), and large KPI grids push the actual work (orders, products) below the fold — worst on mobile. Demote secondary controls and lead with primary content.
2. **Color is decorative, not semantic.** KPI tiles, the courier "Online" status, and the branding preview all use color inconsistently or in ways that contradict meaning (green zeros, inverted attention colors, illegible pale-on-pale). Establish a strict status-color system: color = attention/state, gray = idle.
3. **Placeholder / seed data exposes weak empty states.** Couriers (53 identical "?" rows) and Menu (50 image placeholders) both look broken because there's no designed empty/onboarding state — only a populated state filled with test data.
4. **Per-row/per-card action affordances are weak.** Edit/delete icons and status dropdowns sit detached, small, unlabeled, and edge-aligned (poor thumb reach on mobile). Needs labels/tooltips, 44px targets, and consistent placement.
5. **Missing or unconfirmed save/primary-CTA states.** Settings shows no visible save bar; Menu lacks an obvious "Add product" primary CTA; mobile branding hides its preview. Primary actions and their saved/dirty feedback need to be unmistakable and reachable.

## Top 3 highest-impact fixes
1. **Fix the branding contrast bug + add an AA guardrail** (Branding) — the tool is actively generating illegible storefronts (pale price on pale card). This ships broken customer-facing pages; it's the single most damaging issue here.
2. **Lead operational screens with content, not chrome** (Orders + Menu, esp. mobile) — collapse welcome banner / kitchen-busy / availability bars / oversized KPIs so the first order or product is visible on load. Highest daily-use impact.
3. **Design real empty/onboarding states for Couriers and Menu** — replace the wall of identical placeholder rows / blank thumbnails with a designed empty state + clear invite/add CTA, and add a status model that doesn't mark phone-less couriers "Online".
