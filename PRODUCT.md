# Product

## Register

brand

> Scope note: the customer **storefront** (`/s/:slug` → menu, cart, checkout, live tracking) is `brand`-register — design *is* the experience. The owner **admin** (`/admin`) is `product`-register and is a separate track; switch register when working there.

## Users

- **Primary — diners.** Hungry consumers in Albania (and EN/UK speakers) ordering food delivery on a **phone**, arriving at a *specific restaurant's* branded storefront (often via a shared link or QR at the table). The job: see an appetizing menu, add a few items, and place an order in under a minute — mobile, often one-handed, sometimes on a flaky connection, sometimes older or not tech-savvy.
- **Secondary — restaurant owners/staff** use the `/admin` app to manage the menu and orders (product-register, separate track).

## Product Purpose

dowiz is a **multi-tenant, white-label food-delivery platform**. Each restaurant gets a branded ordering storefront at `/s/:slug` (menu → cart → checkout → live order tracking), themed by per-location brand tokens (`/public/locations/:id/theme.css`, `--brand-*`). The storefront's job: make the menu feel **appetizing and premium**, make ordering **effortless**, and carry **each restaurant's identity — not dowiz's**. Success = the storefront feels like the restaurant's own boutique site, the food looks worth ordering, and checkout completes without friction.

## Brand Personality

- **Three words: appetizing · crafted · confident.**
- Voice/tone: warm and hospitable, like a good restaurant — never corporate-utilitarian. Speaks the diner's language (sq/en/uk) in real food terms.
- Emotional goal: make someone **hungry**, and make ordering feel **effortless and trustworthy**. The storefront should feel hand-made *for that restaurant*, not stamped out by a platform.

## Anti-references

- **The current dowiz storefront's AI-slop** (the thing to kill): flat dark cards with 160px empty grey image placeholders, washed-out muted text, zero motion, no focus states, generic "Error loading menu" states, visible test-data clutter.
- **Utilitarian delivery-app density** — Uber Eats / DoorDash / Glovo: functional but soulless, every restaurant rendered identical.
- **Templated dark SaaS dashboards** — the default AI aesthetic.
- **"Cards everywhere"** lazy layouts; nested cards.

## Design Principles

1. **The food is the hero.** Lead with imagery and appetite; when a dish has no photo, design a *crafted, intentional* fallback — never a dead grey box. Show, don't tell.
2. **The restaurant's brand, not ours.** Every visual decision flows through per-tenant brand tokens; the default must look hand-made *and* re-skin gracefully. Identity-preservation over platform uniformity.
3. **Effortless to order, one-handed.** Mobile-first, fast, low cognitive load; the path from craving to confirmed order is short and obvious. Speed is a feature.
4. **Crafted, not generic.** Commit to design choices — real typographic hierarchy, intentional spacing rhythm, purposeful motion. Boutique restaurant, not delivery-app template.
5. **Trustworthy.** Clear prices, honest empty/error states, no dark patterns; accessible to every diner.

## Accessibility & Inclusion

Target **WCAG 2.2 AA**. Known needs: outdoor/mobile use (contrast matters), one-handed reach (≥44px targets, bottom-anchored primary actions), multilingual (sq/en/uk — including longer Albanian/Ukrainian strings), and older / less tech-savvy diners. Honor `prefers-reduced-motion`; visible `:focus-visible`; real form semantics (`autocomplete`/`inputmode`) for fast phone checkout.
