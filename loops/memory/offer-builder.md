# Пам'ять петлі · offer-builder

Callable loop: 1 Google-Maps location → a ready-to-review warm-outreach OFFER PACKET.
`node scripts/offer-builder.mjs "<maps query>" --slug <demo-slug> [--base <url>]` → writes
`loops/offers/<slug>-offer.md` (gitignored — holds contact info) + prints it.

## What it produces (6 sections)
1. **Target** — venue name/address/rating/hours/Maps-url/website (Maps scrape, same recipe as demo-builder).
2. **Demo** — the /s/<slug> link + a live-check (`/public/locations/<slug>/menu` 200). The demo IS the
   personalization (research: personalization beyond first-name ~2× reply rate; the "could only be sent to
   this one person" hook). ⚠️ build the demo first (demo-builder) or the link is dead.
3. **Owner DM channels (PUBLIC)** — WhatsApp = `wa.me/<digits>` from the Maps phone (the reliable channel);
   Instagram/Facebook from the venue website (best-effort; flagged when not public). Owner personal name is
   usually NOT reliable from OSINT — keep to the venue name.
4. **Communication strategy** — observable signals + the Albanian warm-list playbook (below).
5. **Albanian DM draft** — 5-block scaffold, slot-filled; ⚠️ NATIVE REVIEW required before sending.
6. **Send checklist.**

## ETHICS (baked in, not optional)
PUBLIC BUSINESS data only. NO covert individual psychological profiling (manipulation; backfires on a
high-trust/honor market). NO auto-send — the packet is PREPARED for a human to review + send. Warm list =
prior consent; still one soft follow-up MAX, honour opt-outs. Mirrors the repo's outreach posture
(preview-by-default, Art-14 notice = a separate human act; "no unconsented outreach out of a bulk run").

## Albanian warm-list playbook (operator, 2026-07-01)
- WARM (already said "yes" to testing) → ONE message, demo FIRST, call as a soft P.S. (not a hook-question —
  you already have permission). COLD volume later → ask first (SPER), demo second (a lone link from a
  stranger trips the scam reflex). This loop targets WARM.
- 5 short lines, one role each: recognition → bridge ("I made something") → demo (THEIR menu) → value in
  their terms ("porositë drejt te ti, pa komision") → soft two-way CTA ("nëse jo, thjesht më thuaj").
- AVOID: falas/kursim/mundësi (post-1997 pyramid scam trigger); any competence hit ("ti humbet / gabim" =
  face-slap on an honor culture); English/Italian default; call-pressure; religion (any axis).
- Name EXACT with ë/ç. Personalise ≥ name + one real fact per recipient — never identical copy across the 50.
- Demo MUST open on a phone, no login — one broken first click on this low-trust market = closed forever.
- Humanizing: OSS humanizers are English detector-bypass (useless here) — use anti-slop phrasing + a NATIVE
  pass. See [[demo-builder-loop-2026-07-01]], [[storefront-venue-data-maps-scrape-2026-07-01]].

## Уроки
- 2026-07-01 BUILD. OSINT reality: automated owner-personal data is THIN — venue websites are often generic
  templates (artepasta.al had placeholder emails, no socials). The dependable channel is phone→WhatsApp +
  the demo; don't fabricate an Instagram handle or an owner name. Honesty > a full-looking packet.
