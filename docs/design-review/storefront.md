# Storefront / customer — design review

Reviewer: senior product designer. Surface: dowiz public ordering storefront (Dubin & Sushi tenant). Brand = crimson/rose soft-UI. The experimental paper/Nomadic skin is ignored; critique targets underlying structure, UX, and conversion. Date: 2026-06-25.

---

### Storefront menu — desktop
- **Reads as:** A pale, low-contrast menu where the single visible item (Cola) floats alone in a vast pink void — feels empty and unfinished, not appetizing.
- **Suggestions:**
  - **No food imagery → kill the generic fork/knife placeholder.** Every card shows the same crimson cutlery glyph on a pink wash. For a food storefront this is the #1 conversion killer. Either require/encourage owner photos, or design a far more attractive text-first card (large item name, short description, price as the hero) so the absence of a photo doesn't read as "broken."
  - **One card per row wastes the viewport.** Desktop shows a single ~360px card in a 1280px frame. Use a responsive grid (2–3 columns) so users see 6–9 items above the fold and can scan, instead of scrolling category-by-category through near-empty rows.
  - **The `+` add button is mis-weighted and ambiguous.** A solid crimson circle bottom-right competes visually with the price and gives no label. Add a clear "Add" affordance (icon + text or an accessible aria-label) and make price the dominant element; right now "200 ALL" and the button fight for the same corner.
  - **The filter/sort toolbar is cryptic.** Pills read `⇅`, `↑$`, `↓$`, `A-Z`, `SOJE` with no labels and an unclear active state. Label them ("Price ↑", "Price ↓", "A–Z") and show which sort is active. "SOJE" appears to be an untranslated/placeholder token — fix it.
  - **Category tab bar overflows silently.** Tabs run off-screen ("Fut…") with no scroll affordance or "more" indicator. Add a gradient fade + chevron, or wrap, so users know more categories exist.
  - **Hero contrast fails.** "Dubin & Sushi" white serif over a light pink-to-grey gradient is low-contrast (likely <3:1). Darken the overlay or add a scrim behind the title; the "Hapur" (Open) status pill is good — keep it but make it more legible.
  - **No persistent cart entry.** There's no visible cart/basket affordance in the header on desktop. Add a sticky cart button (with item count + running total) so users always know how to check out.

### Storefront menu — mobile
- **Reads as:** Same emptiness as desktop but the single-column layout suits mobile better; still photo-less and lacking a persistent path to cart.
- **Suggestions:**
  - **Add a sticky bottom cart bar.** On mobile, once an item is added there must be a thumb-reachable "View cart · N items · total" bar pinned to the bottom. This is the single highest-leverage mobile conversion fix.
  - **Cards are too tall for too little content.** The Cola card is ~240px for a name + price. Tighten vertical padding so 2–3 items fit per screen; the giant empty placeholder image is the main offender (see imagery note above).
  - **`+` button overlaps the item name on the Pizzas row.** "Margherita" / "Pepperoni" cards show the crimson `+` colliding with the title at the bottom edge. Fix the card footer layout so price + add button sit in a dedicated row, never overlapping text.
  - **Header is crowded.** Theme toggle, currency (L), language (SQ/EN) all jammed top-right. Consider collapsing currency/language into a single settings/⋯ menu so the header can host the cart button.
  - **Sort pills need touch-target + label work.** At mobile size the icon-only pills are <44px and unlabeled — fails tap-target guidance and is unguessable. Label and enlarge.

### Product detail modal — desktop
- **Reads as:** A small centered card that looks more like a confirmation toast than a product page; almost no information to justify opening it.
- **Suggestions:**
  - **The modal adds nothing over the card.** It repeats name, the placeholder glyph, and price. Give it a job: real photo, a description, allergens/nutrition, and modifier groups. If an item genuinely has no options (like Cola), consider letting the `+` add directly from the card and skipping the modal entirely.
  - **"SHIJE" section is empty/placeholder.** It shows a label and four faint glyphs with no content. Either populate it (flavor/variant modifiers with prices) or hide the section when empty — never render an empty options block.
  - **Primary CTA is good but isolate the math.** "Shto në Shportë … 200 ALL" is strong and crimson — keep it. But show how quantity changes the total live (qty 2 → 400 ALL on the button) so the price feels responsive.
  - **Quantity stepper styling is timid.** The `– 1 +` control is low-contrast grey. Make the stepper buttons clearly tappable and align the control's visual weight with its importance.
  - **Modal is undersized for desktop.** A ~280px-wide dialog on a 1280px screen wastes the medium. Use a two-pane layout (image left, details + CTA right) once real content exists.

### Product detail modal — mobile
- **Reads as:** Better proportioned than desktop (full-width sheet, sticky bottom CTA) but still mostly empty placeholder.
- **Suggestions:**
  - **Sticky bottom add-to-cart is correct — keep it.** "Shto në Shportë · 200 ALL" pinned to the bottom is the right pattern; ensure it stays above the iOS home indicator and remains tappable with the keyboard open.
  - **Huge placeholder image eats the sheet.** ~40% of the viewport is the empty pink glyph block. Shrink it dramatically when there's no real photo and promote description/options into that space.
  - **Close affordance placement.** The `×` floats over the image top-right; ensure it has a solid hit area and contrast against varied photos (it's barely visible on the light wash now).
  - **Empty "SHIJE" block again.** Same fix as desktop — hide empty modifier sections.
  - **Quantity + CTA stacking.** Stepper sits directly above the CTA; good. Make the stepper full-width-ish and high-contrast for thumb use.

### Checkout — desktop
- **Reads as:** Clean, conventional, single-column checkout with a clear sticky order CTA — the most polished screen, but with friction and trust gaps.
- **Suggestions:**
  - **Sticky "Porosit • 1050 ALL" bar is excellent — keep it.** Total + action pinned to the bottom is the right conversion move. Add a tiny order summary (N items, delivery fee, subtotal) accessible from it so users trust the 1050 figure.
  - **No order summary on this screen.** The user is asked for name/phone/address but can't see what they're buying or the price breakdown (subtotal, delivery fee, total). Add a collapsible/side order summary; opaque totals erode trust at the highest-intent moment.
  - **Form is single-column and wide.** Inputs span the full ~520px card; name/phone could sit on one row to shorten the form. Constrain input max-width for readability.
  - **"Foto e hyrjes" (entrance photo) is a clever delivery aid** — but as a button with helper text it's easy to miss its value. Reassure on privacy (who sees it, when it's deleted) since this is PII/an image upload.
  - **Dërgo / Merr (delivery/pickup) toggle is subtle.** The segmented control's active state (grey "Dërgo") is low-contrast. Make the selected segment clearly filled with brand color; this choice changes fee + address requirements, so it must be obvious.
  - **Map placeholder is unstyled.** A raw blue/grey map tile with a floating "Adresa e dorëzimit" pill looks unfinished. Add a clear "set your pin" instruction, a search/autocomplete field, and a confirmed-address readout.
  - **No progress/steps cue.** "Përfundimi" (Checkout) gives no sense of how many steps remain. A lightweight "Contact → Address → Pay" indicator reduces abandonment anxiety.

### Checkout with validation errors
- **Reads as:** Native browser validation bubble ("Please fill out this field.") on an empty name — functional but jarring and untranslated.
- **Suggestions:**
  - **Replace native browser validation with branded inline errors.** The default OS tooltip is English-only (the whole UI is Albanian), inconsistent with the design system, and disappears on click. Use inline field-level messages in the active language, with red border + helper text (the red ring on the field is already there — pair it with persistent text).
  - **Validate on blur + on submit, not just submit.** Show the error under the field the moment the user leaves it empty, and scroll to / focus the first invalid field on submit.
  - **CTA total changed to 400 ALL here vs 1050 on the other shot** — confirm the total reflects the live cart and isn't stale; either way, reinforce why the button is disabled/blocked when fields are invalid (e.g., subtle "complete required fields" state).
  - **Don't block submit silently.** If the user taps "Porosit" with errors, give a single summary toast ("2 fields need attention") in addition to inline markers.
  - **Phone format hint is good** ("+355 6X XXX XXXX") — extend the same pattern-mask helper to all constrained fields.

### Cart drawer
- **Reads as:** A clean bottom-sheet cart with item, qty stepper, total, and a strong "Porosit" CTA — solid, minimal, but missing reassurance and upsell.
- **Suggestions:**
  - **Add per-line and subtotal clarity.** It shows "Cola 200 ALL … Totali 200 ALL" but no delivery fee, min-order, or fees line. Show the breakdown so "Totali" is trustworthy, and surface any minimum-order threshold ("Add 300 ALL more for delivery").
  - **Remove affordance is implicit.** Decrementing to 0 is the only way to remove a line; add an explicit remove/trash action and a swipe-to-delete on mobile.
  - **Empty-cart state isn't shown but must be designed.** Ensure the drawer has a friendly empty state ("Your cart is empty — browse the menu") with a CTA back to items, not a blank sheet.
  - **CTA could carry the total.** Match the checkout pattern: "Porosit • 200 ALL" on the button itself reduces a glance and reinforces commitment.
  - **Quantity steppers are low-contrast grey circles.** Same as the modal — increase contrast and tap-target size; the active crimson is reserved only for the CTA, leaving controls feeling disabled.
  - **Drawer scrim is fine; add a sticky header.** Keep "Shporta" + close pinned if the list grows, so the close and title don't scroll away.

### Order tracking — not-found / error state
- **Reads as:** A stark dark-green 404 with a map-search glyph and a single "back home" button — visually distinct (off-brand dark theme) and a dead end for a customer expecting their order status.
- **Suggestions:**
  - **This is the wrong tone for a tracking failure.** A generic "404 / Faqja nuk u gjet" tells a customer nothing about *their order*. Distinguish "order not found" from "page not found": explain ("We couldn't find an order with that link") and offer recovery — re-enter order number, contact the restaurant, or view recent orders.
  - **Off-brand dark theme breaks trust.** Every other screen is light crimson/rose soft-UI; this hard dark-green + amber page looks like a different app and signals "something is broken." Bring it into the storefront's brand (or at least a branded header) so users know they're still in the right place.
  - **Single CTA is a dead end.** "Kthehu në ballina" (back to home) abandons the tracking intent. Add primary recovery actions: "Contact restaurant" (phone/WhatsApp), "Track a different order," and only then "Back to menu."
  - **Give the empty state a reason.** Most tracking-link failures are typos/expired links. Provide an input to paste the order code and retry inline rather than forcing a full restart.
  - **Accessibility:** amber-on-dark "404" and the button may pass, but verify contrast; ensure the map glyph has alt text and the heading is a real `<h1>` for screen readers.

---

## Cross-screen themes
1. **No food photography anywhere** — every card and modal falls back to the same crimson cutlery glyph. This is the dominant visual and the biggest conversion drag; the design must either drive owners to upload photos or make photo-less cards genuinely attractive.
2. **Price/total opacity** — no order summary at checkout, no fee/min-order breakdown in cart, total varies between shots. Customers commit without seeing the math.
3. **Low-contrast / under-weighted controls** — qty steppers, sort pills, segmented toggles, and hero text are all pale grey on pale pink; only the CTA carries brand color, leaving everything else feeling disabled or unfinished.
4. **Unlabeled / untranslated tokens** — icon-only sort pills, "SOJE", "Test-Cat-1782…" category, and English native validation in an Albanian UI. Polish + i18n gaps.
5. **Inconsistent/missing system states** — empty options blocks render, the tracking error is off-brand and unhelpful, and there's no persistent cart entry on the menu.

## Top 3 highest-impact (conversion) fixes
1. **Make the menu sellable without photos:** redesign item cards (text-first hierarchy, description, price as hero, clear labeled "Add" button) and switch desktop to a 2–3 col grid — and add a persistent/sticky cart with count + total on the menu (sticky bottom bar on mobile).
2. **Add a transparent order summary at checkout and in the cart:** itemized subtotal + delivery fee + min-order + total, surfaced from the sticky "Porosit • total" CTA, so users trust the number before paying.
3. **Replace native browser validation with branded, localized, on-blur inline errors** and stop the silent submit-block — reduce the highest-intent abandonment at the form.
