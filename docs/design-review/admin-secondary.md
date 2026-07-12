# Admin secondary — design review

Reviewer: senior product designer. Scope: 10 screenshots of dowiz admin secondary screens (Analytics, CRM, Promotions, Supplies, Activation) across desktop + mobile. Soft-UI dark design system. The rolled-back paper/Nomadic cream skin is ignored; critique targets structure/UX/layout only.

> **Screenshot/label mismatches found (flag for the lead):**
> - `admin-analytics-d.png` does **not** show analytics — it renders the **owner login / "session expired"** screen.
> - `admin-analytics-m.png` shows the analytics **error state** ("Analitika e padisponueshme"), not a populated dashboard.
> Neither analytics screenshot shows a healthy data view, so the data-viz critique below is necessarily limited to the error/auth states captured. Recommend re-capturing both with live data.

---

### Admin Analytics — desktop (1280w) — ACTUALLY: Owner login / session-expired
- **Reads as:** Centered login card on a near-black canvas with a session-expired banner; not analytics at all.
- **Suggestions:**
  - Mislabeled capture → re-shoot the real `/admin/analytics` desktop view. This is the highest priority; the analytics desktop layout is unreviewed.
  - The session-expired banner uses olive/khaki-on-dark with a thin border — contrast of the body text against that muted green is borderline. Promote it to a clear semantic warning token (amber bg tint + readable foreground ≥ 4.5:1) and left-align the icon+text rather than center-wrapping two lines.
  - The decorative amber tab/underline floating above the card has no label and reads as an orphan artifact → remove it or attach it to a real element (logo lockup).
  - Card is a lone island in a vast empty viewport. Add a faint product wordmark/illustration or a one-line value prop so the page does not feel broken on wide screens.
  - "Hyr" primary and "Vazhdo me Telegram" secondary are both full-width and similar weight; the blue Telegram button visually out-competes the amber primary. Make Telegram the visibly secondary action (outline/ghost) so the email path stays primary.
  - The SQ/EN/UA language switch sits awkwardly between the primary CTA and the OR divider, interrupting the form flow → move it to the top-right corner or footer.

### Admin Analytics — mobile (390w) — ACTUALLY: error state
- **Reads as:** Full error card — warning triangle, "Analitika e padisponueshme", reassurance copy, "Provo përsëri" retry; rest of screen is empty black.
- **Suggestions:**
  - Mislabeled/limited capture → also re-shoot the populated mobile analytics. Cannot assess KPI cards, charts, or density from an error screen.
  - The error card floats high with a huge dead void below → either vertically center the error block in the viewport or add a skeleton/secondary content so the screen does not look half-loaded.
  - The reassurance line ("Porositë tuaja janë të sigurta — provoni sërish") is good UX — keep it, but render it at higher contrast; the muted gray is below AA on this background.
  - "Provo përsëri" is the only action and is a low-emphasis filled chip → make it a clear primary button and add a subtle spinner-on-tap so the retry feels responsive.
  - Consider a secondary escape ("Kthehu te Paneli") so a persistent analytics failure does not trap the user on a dead screen.
  - The error triangle icon is thin/gray and nearly invisible → use the semantic warning color and slightly larger weight so the state reads instantly.

### Admin CRM / Customers — desktop (1280w)
- **Reads as:** Clean customer table — Klient / Telefoni / Porositë / LTV / Last order / "Zbulo" action; left nav, 50 customers, search + sort + Export CSV. Genuinely the strongest screen of the set.
- **Suggestions:**
  - Row data is overwhelmingly test fixtures ("E2E Stepper", "RaceTest", "QA", "LC D invalid") → this is a capture/seed issue, but it also means the screen has never been pressure-tested with real names; verify long names, missing phones, and 0-order customers render gracefully.
  - **LTV column** "7600 ALL" wraps to two lines (number / currency) in a narrow column → widen the column or render currency inline ("7600 ALL" on one line, right-aligned with tabular-nums) so values are scannable and comparable.
  - "Porositë" count is right-aligned far from its header and reads as floating digits → tighten alignment, and consider a subtle bar/sparkline or bold weight so high-value customers pop.
  - Every row has an identical amber "Zbulo" pill — heavy repeated color draws the eye away from the data → demote to a ghost/row-hover action or a chevron; reserve solid amber for one primary CTA per view.
  - "Më shumë porosi" sort dropdown and Export CSV are visually equal-weight to search → group sort+filter controls and let the search field dominate as the primary entry point.
  - Add column affordances: make headers sortable (caret on active sort), and the disclosure chevron at row start implies expand — confirm it works and add hover state, or remove it.
  - Phone numbers are masked (`+3***67`) — good for privacy; add a tooltip/explanation of why, and ensure LTV/last-order are also consistent in masking policy.

### Admin CRM — mobile (390w)
- **Reads as:** Table collapsed into stacked cards — name, masked phone, order count + LTV, relative time, "Zbulo" pill top-right. Reasonable responsive transform.
- **Suggestions:**
  - The search field truncates its own placeholder to "Kër" because it shares one row with the sort dropdown + Export CSV → stack search full-width on its own row, and move sort/export into a filter sheet or icon row beneath.
  - LTV ("7600 ALL") is the most important number per card but sits inline mid-card with equal weight to the order count → make LTV the visual anchor (larger/bolder, right-aligned) since CRM value ranking is the job here.
  - The amber "Zbulo" pill in every card top-right repeats the desktop over-emphasis problem → make the whole card tappable to open the customer, and drop the redundant pill (or shrink to a chevron).
  - Cards are tall with generous internal padding → only ~4 customers fit per screen; tighten vertical rhythm so more of the 50 customers are scannable without heavy scrolling.
  - Icons (briefcase, clock) are low-contrast gray → either commit to them with adequate contrast or drop them; right now they add noise without aiding scan.
  - Add a sticky result count / active-sort label so the user knows they are sorted by "Më shumë porosi" while scrolling.

### Admin Promotions — desktop (1280w)
- **Reads as:** Clean empty state — ticket glyph, "Asnjë promocion", helper copy, primary "Krijo Promocion"; a second identical CTA sits top-right.
- **Suggestions:**
  - Two identical "Krijo Promocion" buttons (top-right + center) in an empty state is redundant → in the empty state, keep only the centered CTA; show the top-right one once promotions exist (list view).
  - The empty-state card is short and floats in the top third of a tall viewport with a large dead zone below → either grow the card, vertically center it, or add example/template promo cards ("Try: 10% off first order") to teach the feature and reduce blank-canvas friction.
  - The ticket glyph is thin gray and barely visible → increase weight/size and consider a brand-tinted illustration so the empty state feels intentional, not unstyled.
  - Helper copy is centered and good, but the heading "Asnjë promocion" is a dead-end statement → reframe to an action-forward line ("Create your first promo to drive repeat orders") to motivate the click.
  - Consider seeding 2–3 promo type starting points (percentage off, free delivery, BOGO) as selectable cards — turns an empty state into an onboarding moment.

### Admin Promotions — mobile (390w)
- **Reads as:** Full-width amber "Krijo Promocion" CTA at top, then an **error** state below ("Promocionet nuk u ngarkuan. Provo përsëri") — inconsistent with the desktop empty state.
- **Suggestions:**
  - Desktop shows *empty* and mobile shows *error* for the same screen → reconcile: this is likely a load failure on mobile capture. Confirm the data layer and re-capture; the two breakpoints must agree on state.
  - With a load error, the top "Krijo Promocion" CTA is misleading — creating a promo while the list failed to load is risky → disable or de-emphasize the primary CTA until the list loads, surfacing the retry as the primary action instead.
  - The error block floats mid-screen with huge void below (same pattern as analytics mobile) → standardize a single reusable centered error component across all admin screens (icon + message + retry + optional back).
  - Retry is a small ghost chip → promote to a clear button; add loading feedback on tap.
  - Error icon (red circle-exclamation) is the right semantic but small → scale up slightly for instant recognition.

### Admin Supplies / Inventory — desktop (1280w)
- **Reads as:** Dense ingredient list with type badges (ING/VEGL/SALCA), category, unit, kcal, allergen tags, and per-row edit/delete; search + sort + category filter chips; info banner; "Shto Furnizim" primary. Information-rich and the most functional screen here.
- **Suggestions:**
  - The info banner ("Përbërësit e shtuar këtu shfaqen në redaktorin e recetave…") is persistent and consumes a full row → make it dismissible (remember per-user) so it does not permanently cost vertical space for experienced operators.
  - Type badges (ING / VEGL / SALCA) are low-contrast gray uppercase and easy to miss → either color-code by type (matching the filter chips) or drop them, since the category text ("Vegetables", "Utensils") already conveys most of the meaning — right now badge + category is partly redundant.
  - The "pa konfirmim" (unconfirmed) status is the most operationally important signal but rendered as a small teal chip mid-row → elevate it: a left border accent or a dedicated status column so operators can scan what needs confirming.
  - Allergen tags ("Qumësht", "Soje", "Gluten") appear on a second line under some rows, making row heights uneven and the list rhythm jumpy → give allergens a consistent slot (own column or chip rail) so every row is the same height and scannable.
  - Row leading icons are near-identical generic glyphs in low contrast → either make them meaningful per type (bottle for sauce already differs — extend that) or remove to reduce noise.
  - Edit/delete icons are tiny and far right with no labels/hover affordance → enlarge tap targets, add hover background, and guard delete with confirmation (allergen/recipe data is destructive to lose).
  - Sort button is an unlabeled icon next to "Të gjitha" filter chips → label it or give it a tooltip; current state of sort is invisible.

### Admin Supplies — mobile (390w)
- **Reads as:** Stacked supply cards with same badges/allergens, full-width "Shto Furnizim" CTA, info banner, search + sort + horizontally-scrolling filter chips. A faithful responsive collapse.
- **Suggestions:**
  - The info banner eats a large block above the (already scrollable) list on a small screen → make it collapsible/dismissible; on mobile this cost is even higher than desktop.
  - Filter chips scroll horizontally and are partially cut ("Përbë…") with no scroll affordance → add a fade/gradient edge or a filter-sheet button so users know more categories exist off-screen.
  - "pa konfirmim" + allergen tags stack into a 2-line tag pile under each item, making cards uneven and tall → cap to one tag row with "+2" overflow, and surface confirm-status as a card-level accent (border/dot) instead of a mid-card chip.
  - Search and sort share a row; search is fine here but the sort icon is unlabeled → same fix as desktop (label/tooltip).
  - Edit/delete icons sit inline top-right of each card with small targets → ensure ≥44px tap targets; consider a swipe action or overflow menu so destructive delete is not a one-tap mistake.
  - Only ~4–5 items fit per screen due to tall cards → tighten padding and allergen rows to raise density; inventory lists are scanning-heavy tasks.

### Admin Activation / Go-live — desktop (split: checklist left + storefront preview right)
- **Reads as:** Strong "Dyqani juaj është online" checklist with green check steps, an optional "make a test order" step, and a primary "Shiko dyqanin online" CTA — paired with a live storefront preview on the right. The most polished, confidence-building screen in the set.
- **Suggestions:**
  - The "Lidhni njoftimet" (Telegram) step uses a neutral gray minus/incomplete icon while others are green-checked → make incomplete steps clearly actionable (chevron + "Set up" affordance + slightly warmer state) so the one remaining task stands out rather than reading as disabled.
  - "Ofroni marrjen në vend" shows an "AKTIV" tag in green text — good — but it competes with the green checkmarks; differentiate status tags (pill background) from completion checks (icon) so the two signals are not confusable.
  - The split layout is effective but the storefront preview has no frame/label → add a "Pamja paraprake" header and a device frame so it reads as a preview, not a second live panel the user might try to edit.
  - "REKOMANDOHET (OPSIONALE)" section header is tiny low-contrast gray → it is an important distinction (required vs optional steps); give it slightly more weight and spacing so the grouping is obvious.
  - Primary CTA "Shiko dyqanin online" is great; consider showing it sticky/fixed at the bottom of the checklist column so it is always reachable as the list grows.
  - The preview shows test/placeholder products with generic fork-knife image glyphs and a long "Test-Cat-1782148864348" category → again a seed issue, but it undermines the "your store is live" confidence message; ensure the activation preview pulls the owner's real (or cleaned demo) menu.

### Admin Activation — mobile (390w)
- **Reads as:** Tabbed "Lista e hapave / Pamja paraprake" with the same checklist, optional step, and a prominent fixed green "Shiko dyqanin online" CTA. Well-adapted; tabs are the right call for the split.
- **Suggestions:**
  - The "Lista e hapave / Pamja paraprake" tabs are text-only with a thin underline on the active tab → strengthen the active-state contrast (bolder weight + clearer underline) and ensure the inactive tab is obviously tappable, not just dimmed text.
  - The incomplete "Lidhni njoftimet" step (gray minus) needs an explicit tap affordance on mobile (chevron is present — good) but the visual difference from completed steps is subtle → add a faint "to-do" background tint so the single remaining action is unmissable.
  - The green CTA is fixed at the bottom and slightly overlaps the optional "Bëni një porosi provë" card → add bottom padding/scroll inset so the last list item is never hidden behind the CTA.
  - "REKOMANDOHET (OPSIONALE)" header is very low-contrast → same fix as desktop; on mobile it is even harder to read.
  - "AKTIV" status tag is green text inline with step copy → on the narrow column it crowds the description; give it a pill and right-align so it does not fight the body text.
  - Bottom nav (rocket / grid / clipboard / cutlery / "Më shumë") icons are low-contrast and unlabeled except the last → label all or none for consistency, and ensure the active section is clearly highlighted.

---

## Cross-screen themes
1. **Over-use of solid amber.** Repeated amber pills ("Zbulo" on every CRM row, duplicate "Krijo Promocion") dilute the "one primary action per view" principle. Reserve solid amber for a single CTA; demote repeated row actions to ghost/hover/chevron.
2. **Inconsistent and low-emphasis empty/error/loading states.** Analytics (error), Promotions (empty on desktop, error on mobile — disagreeing), and the floating mid-screen error blocks need a single shared component: centered, semantic icon, readable message, clear retry, optional back/escape.
3. **Low-contrast secondary text and icons throughout.** Muted gray badges (ING/VEGL/SALCA), section headers ("REKOMANDOHET (OPSIONALE)"), leading row icons, and helper/reassurance copy frequently fall below AA. Raise foreground contrast on all secondary text and status icons.
4. **Density and uneven row/card heights.** Allergen/status tags wrap to extra lines (Supplies) and LTV currency wraps (CRM), creating jumpy rhythm and low information density — especially on mobile where only ~4 items fit. Standardize a status/allergen slot and use tabular-nums for money.
5. **Mobile control crowding.** Search + sort + export crammed into one row truncates the search placeholder ("Kër"/"Përbë…") on CRM and Supplies. Give search its own full-width row; move sort/filter/export into an icon row or filter sheet.

## Top 3 highest-impact fixes
1. **Re-capture the two analytics screens with live data** (desktop shows login, mobile shows error). The analytics layout — the core of this audit — is currently unreviewed; everything else is secondary to seeing the real dashboard.
2. **Ship one shared empty/error/loading component** and reconcile state disagreements (Promotions empty-vs-error across breakpoints; floating mid-screen errors): centered block, semantic icon at proper contrast, prominent retry, optional escape — applied uniformly across Analytics/Promotions/Supplies.
3. **Tame amber and fix mobile control rows:** demote repeated "Zbulo"/duplicate CTAs to non-solid, make whole rows/cards tappable, and give search its own full-width row on CRM + Supplies mobile so placeholders stop truncating. This single pass raises both hierarchy clarity and mobile usability across four screens.
