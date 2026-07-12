# Max-EV Adoption Synthesis — dowiz + bebop — 2026-07-11

> Synthesis of four independent research lenses run in parallel today, all under the operator's
> binding constraint: **zero-friction or vendors won't try**. Source reports in this directory:
> `2026-07-11-adoption-ev-onboarding.md` (funnel mechanics), `-market.md` (Durrës reality),
> `-pricing.md` (EV model + economics), `-bebop.md` (bebop adoption). Every load-bearing number is
> cited in its source lens with a VERIFIED / ESTIMATE / UNVERIFIED grade; this doc reconciles and
> ranks. No code, config, or git state was changed producing it.

---

## 1. The one-paragraph answer

The single highest-EV move is a **walk-in demo-claim in July** — stand in the venue, scan a QR that
shows the owner *their own restaurant already live*, mint the claim on the spot, and have them send
themselves a test order. It scores **+€1,155 / 90 days** (pricing lens), it is **the only channel
with real response probability** in a market that ranks last of 90 countries in generalized trust
(market lens), and it doubles as the 10–15-owner survey the whole program needs. It is currently
**blocked by one line** — `/claim` 404s everywhere (`apps/api/src/server.ts:858` SPA_ROUTES
omission). Everything else is either a rider on this move or a distraction. bebop adoption is a cheap
passive rider folded into an already-scheduled session, not a campaign.

## 2. Where all four lenses independently converged (highest confidence)

These weren't coordinated — four separate agents reached the same conclusions from different angles:

1. **Face-to-face + referral is the ONLY high-EV channel.** Market lens: Albania is last of 90
   countries in generalized trust (3%), 85.8% of food firms are 1–4-person family ops. Onboarding
   lens: ~half of SMBs never claim listings unaided → concierge, not passive CTA. Pricing lens:
   walk-in +€1,155 vs Instagram −€510 vs paid ads −€280. All three rank digital-cold at the bottom.
2. **"Complement Wolt," never "replace Wolt."** Market lens VERIFIED **9 of the 12 demo venues are
   already on Wolt Durrës** — so the pitch is "keep your regulars at 0% commission," and the local
   proof already exists: **InstaPorosi** (Durrës-local, QR→WhatsApp, 0%, 5 languages) validates the
   exact wedge. Pricing lens's honesty box says the same: shift regulars, don't claim to kill Wolt
   demand.
3. **Publish prices, charge nothing.** Pricing lens: free ≤100 orders/mo → flat 2,900 lek, price
   list public day one. The trial promise (no money asked) and a public price are not in tension —
   they're a trust signal in a low-trust market.
4. **The risk is non-execution, not rejection.** Pricing lens sensitivity: walk-ins stay
   EV-positive even at a **5% claim rate**, because the pre-committed N=10 RED is itself worth
   ~€1,800 in validation information (VOI dominates at zero venues). The failure mode is not "owners
   say no" — it's "the operator never does the July mornings."

## 3. The reconciled play (what to actually do)

**Timing (market lens, position taken): show NOW, close in September.** July is peak beach season;
approach owners in the **15:30–17:30 lull** with each venue's own demo QR card; formal onboarding
lands in September when 71% of seasonal staff is released and owners have attention. July walk-ins
ARE the survey.

**Prerequisite (blocks everything, ~hours):** un-break the hinge — `/claim` in SPA_ROUTES, rotate
the lost `PROVISION_OPS_SECRET`, restart the stopped prod worker. This is Wave-0/1 material from the
MASTER-EXECUTION-PLAN; the OG-card prod delivery rides the G01 merge vehicle.

**The ≤10-minute visit (onboarding lens, three wow-moments chained):**
- W1 — scan the QR → "that's your restaurant, live" (opener, 0 permissions).
- W2 — claim on the spot + owner taps Confirm on a pocket test order (the activation event — gate on
  verifying the owner-alert actually fires; the courier out-of-app signal gap is the one net-new
  build the hub review already flagged).
- W3 — owner sends themselves the unfurling link → it doubles as their first distribution act
  (Facebook page button, not IG bio — Statcounter VERIFIED Albania is 95.6% Facebook by referral).

**The 30-second owner math (pricing lens, say it out loud in lek):** 10 orders/day × 1,500 lek ×
25% = **112,500 lek/mo to Wolt vs 2,900 lek flat** — "your first eight orders pay the month." Build
only the savings-counter to show it; nothing else.

**Commitment stack (pricing lens, cited behavioral evidence, adds zero friction):** printed QR =
foot-in-door; owner finishes the build + taps publish = IKEA effect (+63% WTP); "8 of 10 steps
done" = endowed-progress (34% vs 19% completion); their own test order; link-in-bio.

**Localization edge (market lens, A/B-graded):** add **Italian + Polish** locales — Poland +55.8%
tourists cluster on **Golem**, which sits beyond Wolt's 4km per-venue radius (a segment Wolt
literally cannot serve). This is the one place a new locale directly opens demand.

## 4. Ranked lever table (reconciled, 90-day EV)

| Rank | Lever | 90d EV | Gate / note |
|---|---|---|---|
| 1 | Walk-in demo-claim (July lull) | +€1,155 | blocked on `/claim` fix; = the survey |
| 2 | Referral from venue #1 | +€325/seed | gated on 1 activated venue; manufacture the loop |
| 3 | Menu auto-import as the wow | rider | `demo-from-wolt.mjs` reach multiplier |
| 4 | Concierge onboarding | rider | time-boxed ≤10 venues, never charged |
| 5 | Warm WhatsApp follow-up | +€500 | flips sign on reply rate — test with 10 msgs first |
| — | Instagram DM cold | −€510 | warm-up only |
| — | Paid ads | −€280 | park |
| — | Wait for September (do nothing now) | −€800..1,500 | **reject** — July is the survey window |

## 5. Two market facts the repo didn't know (surface, don't act blindly)

- **Fiscalization moved:** Albania's POS/e-invoicing mandate hit coastal bar-restaurants **May 30,
  2026 (passed)**, everyone by Dec 31, 2026, with 24h auto-penalties. The repo's G1 fiscal blocker
  stands — but this is also the adoption *vehicle*: e-invoicing is Albanian SMBs' #1 digital tool
  (58–77% adoption). A compliance-safe wedge via a certified fiscal provider is A/B-graded.
- **The informal ordering layer** (Instagram DM / WhatsApp / phone) is where a large share of
  Albanian food ordering actually happens — the wedge dowiz intercepts — but the exact share is
  **UNVERIFIED**; the July walk-ins should measure it.

## 6. bebop — the passive rider (not a campaign)

bebop's binding constraint isn't marketing, it's that **no stranger can install it today** (npm
`bebop-agent` 404, README's "no Rust" false, `bin/bebop` needs a clone + cargo). But the
zero-friction wedge is nearly free: the **keyless demo is ~80% already true**
(boot/recall/node/govern/ledger run deterministic with no API key). Verdict:
- **Fold "make-it-trialable" (install one-liner + prebuilt release binary + package the keyless
  demo) into the G07/G08 parking-capture session** — the one where the uncommitted crypto gets
  committed and the memory corpus bootstrapped. Hours of work, inherited, so bebop is trial-ready
  when attention returns without spending the operator-weeks G07 assigns to dowiz P0.
- **Keep AGPL** (never triggers for a locally-run CLI; neutral-to-positive trial evidence). No CLA.
- **Park the delivery-protocol thread** — `delivery/` is empty and its own cold-start plan needs a
  working dowiz with real venues. Hold any Show-HN/PH campaign to the 2026-07-25 review.

## 7. The skip-list (all four lenses agree: do NOT do these now)

TMA intake, WhatsApp Cloud API intake, Viber, video pipeline (WS4), dedicated demo app, Better Auth
mid-funnel, payments/fiskalizimi *code*, cold email/calls, paid ads, Instagram-cold as primary,
messenger-transport (per the hub review — not until demand shows), any bebop marketing campaign.

## 8. The single decision this synthesis asks for

Everything above resolves to one operator action: **do the July walk-in mornings** (starting after
the `/claim` fix), with the demo-QR cards and the 30-second lek math, on the venues already on Wolt.
The stack has out-run the market test by a month; this is the cheapest possible way to learn the one
thing no amount of engineering can answer — whether a real Durrës owner will use it.

*Synthesis produced 2026-07-11 from four parallel read-only research lenses. Companion program
(local-first decentralized hub, `docs/design/local-first-hub-2026-07-11/`) synthesized separately.*
