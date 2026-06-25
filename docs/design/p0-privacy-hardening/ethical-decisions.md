# Ethical / Human Decisions — P0 privacy hardening

> STOP-ETHICS gate. Counsel found **zero blocking ETHICAL-STOP** (counsel-opinion.md
> R1 + RE-EXAMINE R2: GO). The items below are owner/product rulings the Architect
> escalated at RESOLVE. Decided by the owner.
> Owner: SyniakSviatoslav (sviatoslavsyniak@gmail.com). Date: 2026-06-21.

## HD-1 — Idle on-shift courier map dots
**Ruling: PRIVACY-MAX (Counsel recommendation).** No position is stored while a courier
is idle / not on an `accepted`/`picked_up` delivery. On-shift-but-idle couriers are
**invisible on the owner live map** until they accept a delivery. Accepted operational
loss: owner cannot see idle couriers' rough location. Rationale: makes the P0-1 win whole
(tracking begins at the courier's consent act, never in free time); worker dignity > the
marginal owner convenience of an idle dot. The existing idle-visibility code
(`courier-events.ts:155-164`, `fetchLatestPosition`) is removed/neutralized deliberately,
not silently.

## HD-2 — Telegram owner-alert default detail level
**Ruling: `area` best-effort (Counsel recommendation).** Default body = order# + item
count/total + coarse area (district/street, **no house number**), degrading honestly to
"order# + total + authenticated deep-link" when the free-text address cannot be safely
stripped. Full address/phone live **only behind the authenticated owner-app link**.
`full`-in-body is per-owner opt-in (accepted risk R2/NR-4; canary = `full` opt-in rate).
Rationale: never write customer home addresses into permanent Telegram history by default;
honest degradation beats a lying "area" label.

## R11 — Owner dashboard live customer/item search
**Ruling: BUILD server-side authenticated search (escalated default overridden).** P0-3
removes customer name / item names from the realtime bus payload, so the current
client-side `.filter()` live search would miss in-flight orders until reload. Instead of
accepting that gap, **add a debounced owner-authenticated order-search endpoint** (RLS +
JWT scoped) so live search keeps working without putting PII back on the bus. This moves
R11 from accept-risk into P0-3 implementation scope. Owner: Product/Backend.

## Courier as data subject of own movement data (Counsel open question)
**DEFER (confirmed, not a blocker).** This batch *removes* the acute harm (off-work
tracking) and F-1 ships the first half of the transparency (courier-facing boundary
notice). A full courier-facing data-access/retention notice parallel to `checkout.privacy.*`
is a future change, bounded by the existing 24h purge — recorded, not built here.
