# dowiz program roadmap — 2026-07-03

Consolidates the operator's mega-vision into one sequenced program. Each track has a grounded
design doc (linked). Discipline: research/design → build with real proof → council for red-line →
ship. Money/RLS/auth stay council-gated; protect-paths (migrations/.github/package.json) stage for
the operator. Nothing here ships without its proof.

## ⭐ The convergent root (fix once, unblocks three tracks)
Three independent lanes (deliver-v2 council, payments council-prep, missing-features) converge on
ONE gap: **there is no central order-cancel primitive that records `refund_due`.** Today `refund_due`
is written ONLY in `deliveryCompletion.ts` (courier path). So every other cancel — grace/timeout
(deliver-v2), customer self-cancel, owner-cancel — silently keeps a paid customer's money, and also
misses the `ORDER_CANCELLED` event (orphaned dwell alerts). **Build a central `cancelOrder` primitive
(state-machine-owned edge + `ORDER_CANCELLED` publish + `refund_due` on paid orders) and every cancel
path routes through it.** This is the shared prerequisite for deliver-v2 shipping AND payments go-live.
Counsel pre-registered `STOP-REFUND-BEFORE-GRACE` on it. → the deliver-v2 council (task #34) is
resolving exactly this now.

## Track status + first increments
| Track | Plan doc | First increment (SAFE unless noted) | Gate |
|---|---|---|---|
| Bug-catching net | `bug-catching-net/plan.md` | `scripts/synthetic-probe.mjs` → /health → pages existing Telegram bot (closes the P0; ~1h, reuses Telegram rail) | SAFE now |
| Voice control | `voice-control-storefront/plan.md` | push-to-talk menu SEARCH (read-only) on `packages/voice` (~70% built) | SAFE, flag-dark |
| Missing features / friction | `missing-features-friction/plan.md` | **Reorder** (rehydrate cart from last order, device-local, FE + read-only) | SAFE now |
| Cinematic + per-vendor brand | `cinematic-brand-ux/plan.md` | ProductCard card→detail `layoutId` shared-element (reuses LazyMotion, flag-dark) | SAFE, visual-gated |
| Verified card+crypto payments | `payments-verified/council-prep.md` | (none — council + human sign-offs first) | 🔴 money red-line |

## Sequencing
**Wave 1 (SAFE, ship-now, no council):** synthetic probe + Telegram paging; reorder; fix the dead
promo at the till (`orders.ts:509` — wire owner promos into checkout — NOTE: money-adjacent, verify
it's discount-apply not payment → likely council-light); hide non-functional affordances (scheduled
"coming soon", un-redeemable promo); voice menu-search read-only (flag-dark); ProductCard shared-element
(flag-dark). Each with red→green + staging E2E.
**Wave 2 (council-gated):** the central `cancelOrder`+`refund_due` primitive (deliver-v2 council output,
then all cancel paths route through it); customer identity/address book (auth council — unblocks reorder-
server, order history, saved addresses, loyalty); CI bug-net prevention layer (protect-path .github).
**Wave 3 (red-line, full council + human):** verified card (per-tenant local-bank HPP, dowiz-as-conduit —
Stripe unavailable in Albania) + crypto verification (Plisio, the 3 CRITICALs, C2 refund, proof bar);
double-entry ledger when custodial card lands.

## Prod-deploy status (blocking everything)
The 275-commit merge is on `main` (`9e93c6a2`) but NOT deployed — CI validate correctly blocked it on
the deliver-v2 guardrail (order-state red-line). Prod safe on old image (`/livez` 200). The deliver-v2
council (task #34) fix → CI green → prod deploys 066..084 is the immediate unblock for shipping ANY of
the above. Do that first.

## Honest notes
- This is a multi-wave program, not one ship. Wave 1 is ~a week of SAFE builds; Waves 2-3 are
  council-gated and partly operator-gated (auth identity, payments, .github, migrations).
- Recurring subagent-init glitch hit several lanes; all were re-dispatched successfully.
- Everything money/RLS/auth honors the charter: council + human sign-off, dark-first, no autonomous
  enablement. The bug-net's external monitor + the payments proof-bar are the two highest-leverage
  reliability investments.
