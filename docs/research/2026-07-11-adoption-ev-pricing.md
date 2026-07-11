# Adoption levers under zero-friction economics — EV model, pricing paths, owner math

> **Date:** 2026-07-11 · **Lens:** EV model + zero-friction economics · **Status: RESEARCH ONLY —
> this file is the only artifact created; no code, no config, no repo state touched.**
> Repo grounding: `DeliveryOS-Business-Value-Sort.md` (Tier-1 wedge / Tier-5 "pilot as measured
> funnel"), `docs/design/gap-blueprints-2026-07-11/G11-business-validation-week.md` (funnel state,
> pre-committed GREEN/RED), `G07-program-spine-arbiter.md` §scorecards (distance-to-value),
> memory `crypto-payments-build-2026-06-30.md` (payments = cash-first; crypto dark behind flags).
> Binding constraint accepted as given: **no money asked up front, or vendors won't try.**

**Assumption tags used on every number:**
- **[S]** sourced — external benchmark, cited in §7.
- **[R]** repo-verified — from a repo doc that itself verified it.
- **[G]** guess — operator-honest estimate; range given; flagged for testing.
- **[D]** derived — arithmetic on tagged inputs.

---

## 0. The one structural fact the model must respect

At this stage dowiz has **zero validated demand** (G11: no real order, no claimed venue, 12 shadow
demos [R]). Therefore the dominant value term of any adoption lever this quarter is **not
revenue — it is information**: a decisive GREEN (real venue, real order) or a decisive RED
(pre-committed N contacts, zero claims) both redirect hundreds of operator-hours. The EV model
below carries this as an explicit **VOI (value-of-information) term** so it can't be smuggled in
or ignored. Revenue terms are still computed honestly — they are small at N=1–5 venues, and the
table says so.

---

## 1. The EV model

### 1.1 Formula

```
EV(lever, 90d) = Reach × p_try × p_act × V_act        (commercial term)
              + P_decisive × VOI                       (information term)
              − (OperatorHours × w) − BuildCost        (cost term)
```

- **Reach** — venues the lever can put the offer in front of within 90 days.
- **p_try** — probability a reached venue *tries* (claims its demo / starts onboarding).
- **p_act** — probability a trying venue *activates* (publishes + ≥1 real non-operator order
  ≤30 days — exactly G11's GREEN definition [R]).
- **V_act** — expected commercial value of one activated venue (below).
- **P_decisive** — probability the lever produces a clean GREEN or clean RED against G11's
  pre-committed metrics inside the window. A lever that fails *ambiguously* (silence, no data)
  scores low here even if cheap.
- **w** — operator-hour cost.

### 1.2 Parameter ledger (every input, one line each)

| Param | Value | Tag | Basis |
|---|---|---|---|
| Horizon | 90 days | [G] | matches G11 week + two follow-up waves |
| w (operator hour) | €20/h | [G] | opportunity-cost proxy for a solo founder-operator; results shown so you can re-weight |
| Subscription price point | €29/mo ≈ 2,900 lek | [R]/[G] | strategy doc's flat $19/39/59 [R: Business-Value-Sort header]; €29 = blended mid; 1€≈100 lek [G: mid-2025 rate ≈97–99, rounded for owner math] |
| p_pay (activated free venue → paying within 12 mo) | 0.25 (range 0.10–0.40) | [G] | anchored [S]: sales-assisted freemium converts 5–7% avg, 10–15% top-quartile (ChartMogul/FirstPageSage); concierge + personal relationship + day-1 price anchor assumed to beat top quartile; **untested — the single most important pricing unknown** |
| Monthly logo churn once paying | 4%/mo | [S] | SMB SaaS norm 2–4%/mo (good) to 3–7%/mo (typical micro-SMB) |
| 12-mo revenue per paying venue | €281 | [D] | 29 × Σ(0.96^t, t=0..11) = 29 × 9.68 |
| Referral bonus per happy activated venue | +€30 | [D]/[G] | 3 named intros [G] × 50% try [G, anchored S: referred customers convert ~4×] × 60% act [G] × p_pay × €281 × 0.5 haircut |
| **V_act** | **€100** | [D] | 0.25 × 281 + 30 ≈ 100 |
| **VOI** (first decisive GREEN/RED) | **€2,000** | [G] | conservative proxy: a decisive answer redirects ≥100 operator-hours of the next quarter (≥€2,000 at w) — G07 shows the whole program is currently allocating on this unvalidated assumption [R] |
| VOI₂ (repeatability signal — does a *channel* compound?) | €500 | [G] | second-order; only referral loop and WhatsApp can earn it |

### 1.3 Ranked lever table (90-day EV)

| # | Lever | Reach | p_try | p_act | Exp. activations | P(venue #1 ≤30d) | Commercial € | Info € | Cost € | **EV €** |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | **Pre-built demo claim, walk-in (G11 motion)** | 30 (12 demos + 18 new; Wolt-Durrës has 50+ venues [R]) | 0.25 [G: 0.05–0.50; D2D cold 2–5%, warm/demo-led 20–30% [S]] | 0.50 [G: concierge; 70% of churn is first-90-days, TTFV<7d halves it [S]] | 3.75 | **0.80** [D: 1−0.875¹²] | 375 | 1,800 [D: 0.9×2,000] | 1,020 [D: 30 visits×1.25h + 3h/claimed + 3h prep, ×€20] + ~€0 build (one-line `/claim` fix + QR script already in G11 [R]) | **+1,155** |
| 2 | **Referral loop from venue #1** (gated on ≥1 happy venue) | 3 intros/seed [G] | 0.50 [G; referred convert ~4×, 84% of B2B buying starts with a referral [S]] | 0.60 [G] | 0.9/seed | n/a (post-#1) | 90/seed | 300 [D: 0.6×VOI₂... plus repeatability] | 65/seed | **+325/seed** (≈ +260 unconditional now [D: ×0.8]) |
| 3 | **Menu auto-import as the wow** (rider on #1 for the 18 non-demo venues) | — | ×1.3 [G] | ×1.2 [G] | +1.2 | ↑ | +120 | faster decisive | ~0 operator; import exists per repo (photo/PDF menu import Tier-1 [R] — verify build state before relying on it) | **+120–250** |
| 4 | **Free pilot concierge** (rider; the p_act=0.5 and p_pay=0.25 above already assume it) | — | — | 0.3→0.5 [G, anchored S: onboarding <7d TTFV → ~50% lower churn; sales-assisted ≈2× self-serve conversion] | +1.5 vs self-serve | ↑ | +150 | keeps RED *diagnosable* (you see where the funnel dies) | 3h/claimed venue | **positive ≤10 venues; time-box it — negative at >30** |
| 5 | **WhatsApp outreach with demo link** | 60 [G] | 0.05 [D: 15% reply [G; WhatsApp open 95–98%, avg reply 4%, S-Europe up to 50% [S]] × 33% claim-if-reply [G]] | 0.40 [G] | 1.2 | ~0.5–0.7 | 120 | 700 [D: 0.35×2,000 — silence is *ambiguous*, not falsifying] | 320 | **+500** (best as scheduler/warm-up for #1, not standalone) |
| 6 | **Instagram content** | ~5 owner-touches/90d [G — page from zero; FB/IG reach 43% of AL population but organic-new-page reach is tiny [R/S]] | 0.05 [G] | 0.40 [G] | 0.1 | ~0.05 | 10 | ~0 (no decisive signal) | 520 [D: 2h/wk×13] | **−510 as acquisition** → demote to trust-hygiene: one credible page, 2h once, so owners who vet you find a face [R: trust research T4] |
| 7 | **Paid ads (Meta)** | 1,500–3,000 clicks on €300 [G; low-CPC market, global avg CPC $1.14, emerging markets far lower [S]] | ≈0 owner-targeting precision (Meta can't reliably target "restaurant owner, Durrës", audience ≈50–200 people) [G] | — | 0–0.4 | ~0.05 | ≤40 | 100 | 420 | **−280 → PARK** (revisit later as *customer* demand-gen sold to venues, a different product) |
| 8 | **Waiting-for-September** | — | — | — | −2.2 forgone [D] | 0 by construction | −690 [D: 60d of displaced #1 EV] | −(answer delayed a quarter) | +120 re-scrape stale demos [D] | **−800 to −1,500 → REJECT** (see sensitivity — even halved summer p_try beats waiting) |

**Reading the table:** #1 is the only lever that is EV-positive *even in its pessimistic branch*
(below), because the pre-committed RED is itself worth the walking. #2–#4 are riders/successors of
#1, not alternatives. #5 supports #1. #6–#8 are negative now.

### 1.4 Sensitivity — which assumption dominates each lever (test this first)

| Lever | Dominant assumption | Swing | What it does to EV | Cheapest test |
|---|---|---|---|---|
| Demo-claim walk-in | **p_try (claim rate)** | 0.05 → 0.50 | activations 0.75 → 7.5; EV **+855 → +2,280** [D: commercial 75→750; VOI holds at 1,800 as long as the pre-committed N contacts are completed — a clean 0/10 is a *decisive* RED] | **First 5 walk-ins.** This is literally G11's pre-committed N=10 contacts / 5 venues [R]. The EV risk is not low claim-rate; it is *not completing the walks* (G07 revealed-preference: zero outreach sessions to date [R]) |
| Demo-claim walk-in (2nd) | p_act (claim→real order) | 0.25 → 0.70 | EV +690 → +1,530 [D] | Concierge the first claim within 72h; instrument the funnel digest (G11 5.4) |
| Referral loop | **Existence + happiness of venue #1** (binary gate) | 0 → 1 | EV 0 → +325/seed | Ask every activated owner for ONE named intro at the moment of first-order joy (peak-end) |
| Menu-import wow | Import actually works at-the-table in <10 min | works/doesn't | ×1.3 try multiplier → 0 (a failed live import at the table is *negative* trust) | One rehearsal: import a real Durrës paper menu photo end-to-end before using it in a pitch |
| Concierge rider | Hours per venue stay ≤3h | 3h → 8h | flips negative past ~6h/venue [D] | Log hours on venue #1; decide at venue #3 |
| WhatsApp outreach | **Reply rate** | 4% (global avg) → 30% (S-Europe warm) | EV **−45 → +1,100** [D] | 10 messages with the OG-card demo link (staging carries the rich unfurl; prod does not [R]) before scaling to 60 |
| Instagram | Owner-reach of a zero-follower page | — | stays negative in all 90d branches | none worth paying for now |
| Paid ads | Owner-targeting precision | — | stays negative | none |
| Wait-for-September | "Owners too busy in peak season to talk" | halves summer p_try to 0.125 | walk-in EV still **+970** [D: 30×0.125×0.5×100 + 1,800 − 1,020] — waiting loses in its own best case; September also *adds* Wolt entrenchment (in Durrës since 03/2025, growing [R]) and 60 more days of demo-data staleness | Do morning visits (10:00–12:00), avoid rush hours — capture the pro-September argument without the delay |

---

## 2. Pricing paths — converting a zero-friction trial without breaking the promise

### 2.1 Benchmarks (what the free-first winners actually charge)

| Company | Free thing | Where the money is | Take/price | Relevance |
|---|---|---|---|---|
| **GloriaFood** | Full ordering system, unlimited orders, 0% commission, forever | Paid add-ons: online payments, sales website, branded apps, done-for-you setup (~$29/mo each); white-label reseller program | flat add-on subs | **Closest comparable.** Proves free-forever core + paid convenience add-ons is viable in exactly this vertical [S] |
| **Square / SumUp** | Free software, cheap/subsidized hardware | Payments take-rate | 1.69–1.75% per transaction; SumUp One €29/mo drops it to 0.99%; ~65% of SumUp revenue is transaction fees [S] | Monetize the *money flow*, never the software |
| **Toast** | $0 starter software | Payments >> SaaS (per its S-1); 2.49%+15¢ (paid plan) or 3.09%+ (pay-as-you-go); most restaurants end at $150–300/mo with add-ons [S] | hybrid | Shows free-entry + escalating take works — and how "free" quietly becomes expensive (reputation cost to copy) |
| **Wave** | Free accounting/invoicing | Card processing 2.9%+$0.60, payroll $25+$6/employee, Pro $19/mo | payments + payroll | Monetize adjacent money services. Cautionary: its 2024 shift to a paid tier after "free forever" years drew backlash [S] — don't re-promise what you'll unpromise |
| **WhatsApp Business** | Free app for the SMB | API per-conversation fees at scale | usage fees | Free until you're big enough to feel it |
| **Aggregator anchor (what dowiz undercuts)** | — | Commission on every order | Wolt 20–30% + platform fee [S]; Albania-specific rates undisclosed; Baboon (local, 1,000+ venues) undisclosed; industry 15–30% [R/S] | The enemy number the pitch is priced against |

### 2.2 Three candidate paths, scored

Scoring: **Trial friction** (0 = none, 5 = kills the trial) · **12-mo EV/venue** [D, at p_pay and
churn from §1.2] · **Wedge integrity** (does it keep "never a % of your food revenue" literally
true?) · **Ops complexity**.

| Path | Mechanics | Friction | 12-mo EV/venue | Wedge integrity | Ops | Verdict |
|---|---|---|---|---|---|---|
| **A — Free up to N orders/mo, then flat** | Free forever ≤100 orders/mo [G threshold ≈ 3–4 orders/day — below it the venue isn't getting real value anyway]; above → flat 1,900/3,900/5,900 lek/mo (maps the existing $19/39/59 [R]). Price list public from day 1; pilot venues grandfathered 90 days | **0** — nothing asked until the product has demonstrably paid for itself; the threshold *is* the success signal | ~€70 [D: 0.25×281] | **Perfect** — flat, never a %; the 30-sec math stays honest | Low: an order counter + a conversation (no billing infra needed for venue #1–10; invoice manually) | **PRIMARY.** Converts exactly and only on success; GloriaFood-proven shape with a flat-sub twist |
| **B — Free while cash-only; margin on online payments when they arrive** | Cash orders (77% of market [R]) free forever. When prepaid launches (crypto vertical is built-dark [R: crypto-payments memory]; cards later), take 1.9–2.9% *of the payment*, Square/Wave-style | **0** — monetizes a convenience the venue opts into per-order | €10–30 [D: low share of prepaid early; grows with card adoption] | **Good** — it's a payment-processing fee, not a commission; must be *messaged* as such | Medium-high: PSP/fiscalization posture, refund SLA (ADR-0017 launch gate is NEEDS-HUMAN [R]) | **RIDER, later.** Do not lead with it; bolt on when payments go live |
| **C — Free product, paid concierge/hardware** | Software free forever; charge one-time: done-for-you menu entry (~5,000 lek), branded QR kit prints, thermal-printer resale, priority-support sub | **0 on software, but…** it puts a price on the exact lever (concierge) that drives activation — friction lands where it hurts most | €20–50 one-time | OK | High: caps at operator hours; anti-scales | **PARTIAL ONLY.** Sell extras opportunistically, but never gate onboarding help behind payment during the pilot phase |

**Recommended path: A primary, B rider, C extras.** And one promise-preserving rule from the Wave
lesson [S]: **publish the price list on day 1 while charging nothing.** "Pilot falas — çmimi
standard 2,900 lek/muaj" anchors the future price (no free→paid shock), signals seriousness
(free-with-a-price reads as a gift; free-without-a-price reads as worthless), and keeps the trial
promise intact because the *trial* was never promised to be the *product forever* — only the
try was promised to be free.

**Honest caveat:** the G11 GREEN event validates *demand for the product*, not *willingness to
pay*. p_pay=0.25 [G] stays a guess until the first invoice. Schedule the price conversation at
day 30 of venue #1, with the savings counter (§3) as the evidence on the table.

---

## 3. The own-couriers math — 30 seconds, in numbers an owner recognizes

**Model venue (all [G], mid-range Durrës):** X = 10 delivery orders/day through an aggregator;
Y = 1,500 lek average basket (anchor: standard dinner in Albania runs ~1,900–3,500 lek with drink
[S]; a delivery basket is lighter; Durrës is the country's cheapest restaurant city [S]);
commission 25% (mid of Wolt's 20–30% [S]); 30 open days/mo; 1€ ≈ 100 lek [G].

| | per order | per month | per year |
|---|---|---|---|
| Aggregator keeps (25%) | **375 lek** | **112,500 lek (≈ €1,125)** | **1,350,000 lek (≈ €13,500)** |
| dowiz flat | — | **2,900 lek (≈ €29)** | 34,800 lek (≈ €348) |
| Payback | — | subscription = commission on **8 orders** → **less than one day's deliveries pays the whole month** | — |

Grid for the pocket (commission cost per month at 25%):

| orders/day → | 5 | 10 | 20 |
|---|---|---|---|
| basket 1,200 lek | 45,000 lek/mo | 90,000 | 180,000 |
| basket 1,500 lek | 56,250 | **112,500** | 225,000 |
| basket 2,000 lek | 75,000 | 150,000 | 300,000 |
| dowiz flat, as % of that | 5–6% | 2–3% | ~1% |

**The 30-second script (say it, don't show slides):**

> *Albanian:* „Sa porosi del në ditë me Wolt? … Dhjetë? Me mesatare 1,500 lekë, Wolt-i të mban 375
> lekë për secilën — **112 mijë lekë në muaj**. Ti ke kurierët e tu — pse paguan komision për ta?
> Ky është kanali YT: linku yt, klientët e tu, kurierët e tu — **0% komision**. Kushton 2,900 lekë
> në muaj — **tetë porositë e para e paguajnë**. Deri në 100 porosi në muaj është falas — pa kartë,
> pa kontratë. Dyqani yt është tashmë gati — shikoje.“ *(turn the phone around → their live demo)*
>
> *English:* "How many delivery orders a day on Wolt? Ten? At a 1,500-lek average they keep 375 lek
> each — 112,000 lek a month. You have your OWN couriers — why pay commission on them? This is
> YOUR channel: your link, your customers, your couriers — 0% commission. It costs 2,900 lek a
> month; your first eight orders cover it. Under 100 orders a month it's free — no card, no
> contract. Your shop is already built — look."

**Honesty box (keeps the pitch unfalsifiable-proof):** the aggregator also *brings* demand — the
honest claim is not "quit Wolt", it is **"move your regulars and your Instagram followers to your
own link."** Conservative version: if only **half** the volume shifts, the venue still nets
~53,000 lek/mo ≈ **640,000 lek/yr (≈ €6,400)** [D] against a 2,900 lek/mo flat fee. Multi-homing
is the norm; dowiz wins the direct channel, not the war.

---

## 4. Risks of free — and cheap commitment devices that add no friction

### 4.1 Risks, sized

| Risk | Size | Mitigation |
|---|---|---|
| **No-commitment churn** — free users ghost silently; 70% of SaaS churn lands in the first 90 days [S]; micro-SMB churns 3–7%/mo even when paying [S] | HIGH | Commitment-device stack (§4.2); activation defined as the *owner's own* actions (test order, publish tap), never operator-performed |
| **Freeloader venues** — claim, consume concierge hours, never publish or stay forever under the free threshold | MED | Concierge explicitly time-boxed (3h/venue, first 10 venues); below-threshold venues cost ~€0 marginal and still work as referral assets + social proof; threshold in Path A converts exactly the ones getting real value |
| **"Free = worthless" perception** — zero price can read as zero quality; the owner never *decides* anything, so owns nothing | MED | Day-1 public price list ("pilot falas, çmimi 2,900 lek") — free becomes a *gift with a stated value*; ask for non-monetary payment: a Google review, one named referral, the QR on the counter |
| **Support cost of free** — 12 venues × 1–2h/mo [G] of "my photo is wrong / hours changed" | MED, grows linearly | Self-serve admin exists [R]; batch fixes; the Viber/WhatsApp support line is also the trust channel (trust research T4 [R]) — count it as marketing, cap it per venue |
| **Free trial validates demand, not pricing** | STRUCTURAL | Named above (§2.2): day-30 price conversation with the savings counter as evidence; p_pay is the top pricing unknown to retire |
| **Unconsented-demo exposure** during outreach | already tracked | G11 §5 risk list + five substitutes hold [R]; decline honored same-day — not duplicated here |

### 4.2 Commitment devices — behavioral evidence, zero added friction

Each device costs the venue nothing in money and *increases* their sunk micro-investment — the
mechanism the free tier is missing.

1. **The venue prints and displays the QR** (counter tent / door sticker). A small public act of
   adoption: foot-in-the-door — agreeing to a small request reliably raises compliance with the
   larger one later (Freedman & Fraser 1966 [S]); public + effortful = Cialdini consistency. The
   QR kit is already S-effort per G11 3.1 [R].
2. **The owner finishes the build.** The pre-built demo is deliberately ~80% done; the owner
   uploads their own hero photo, picks the accent color, and **taps publish themselves**. IKEA
   effect: people value what they part-built ~63% higher (Norton, Mochon & Ariely 2012 [S]) — and
   the effect *requires completion*, which is exactly what the demo-claim flow provides.
3. **Endowed progress framing.** Present the demo as "your shop is already 8 of 10 steps done" —
   endowed progress roughly doubles completion (34% vs 19%, Nunes & Drèze 2006 car-wash study
   [S]). The shadow demo IS pre-stamped progress; say it out loud, show a progress bar.
4. **The owner's own test order from their own phone** — the "aha" (already the activation event
   in the trust research T-onboarding [R]); labor + immediate proof; instrument it.
5. **Link in their Instagram bio** — public, visible commitment in front of *their* audience;
   costs 60 seconds; also the first real distribution act.
6. **Savings counter in admin** ("Wolt would have kept 41,250 lek this month") — loss-aversion
   framing converts retention into the day-30 price conversation; this is Business-Value-Sort
   Tier-5 "make ROI visible — double down" [R], and it's cheap (order data already exists).
7. **A named go-live date + day-7 check-in** — implementation intentions; costs one calendar entry.

---

## 5. Top-5 recommendations (ranked)

1. **Execute the G11 walk-in demo-claim week now, in July, mornings.** Highest EV (+€1,150 [D]);
   EV-positive even at a 5% claim rate because the pre-committed N=10-contacts RED is itself worth
   €1,800 of redirected effort [D]. The dominant risk is non-execution, not rejection (G07
   revealed-preference: zero outreach sessions to date [R]). Rejecting "wait for September"
   is part of this: waiting loses even under its own best assumption (§1.4).
2. **Adopt Path A pricing and publish it on day 1 while charging nothing:** free forever ≤100
   orders/mo → flat 1,900/3,900/5,900 lek/mo; pilot venues grandfathered 90 days; Path B
   (payment-margin on prepaid) reserved as a rider for when payments launch; never gate concierge
   behind money during the pilot. This converts on success only and keeps "never a % of your food
   revenue" literally true.
3. **Stack the commitment devices into the concierge choreography:** owner's own test order →
   owner uploads photo/picks color → owner taps publish → QR printed and placed on the counter →
   link in bio → day-7 check-in. Every step is evidence-backed (§4.2), free, and converts the
   free trial's biggest weakness (no skin in the game) into sunk micro-investment.
4. **Say the 30-second math (§3) at every table, and build nothing new for it** except the savings
   counter (Tier-5, cheap [R]) — it is simultaneously the pitch, the retention loop, and the
   evidence for the day-30 price conversation with venue #1.
5. **The moment venue #1 is happy, ask for one named intro** ("cilit pronar t'ia tregoj?") —
   referral EV ≈ +€325/seed [D], referred SMBs convert ~4× [S]; run 10 WhatsApp demo-link messages
   as a reply-rate probe (its EV flips on that one number, §1.4); spend €0 on Instagram content
   and paid ads until ≥5 referral asks have been made.

---

## 6. What would change this analysis

- **Claim rate from the first 5 walk-ins** (re-rank everything with real p_try — the
  Business-Value-Sort's own instruction to re-sort on pilot data [R]).
- **First price conversation outcome** (retires p_pay=0.25 [G], the largest pricing unknown).
- **Albania-specific aggregator commission** becoming public (sharpens §3 from 25% [S-range] to a
  named local number an owner can verify).
- **Concierge hours/venue** exceeding ~6h (flips lever #4 negative early).

## 7. Sources

**External:**
- ChartMogul SaaS Conversion Report — freemium avg 5.6%, sales-assisted 5–7%/10–15%: https://chartmogul.com/reports/saas-conversion-report/ · First Page Sage freemium benchmarks: https://firstpagesage.com/seo-blog/saas-freemium-conversion-rates/
- Wolt merchant fees (20–30% + platform fee): https://blog.menuviel.com/wolt-fees-and-commissions-for-restaurants/ · https://explore.wolt.com/en/deu/merchant/learning-center/wolt-merchant-fees-and-commissions
- GloriaFood free model + paid add-ons: https://www.gloriafood.com/why-is-gloriafood-free · https://www.gloriafood.com/pricing
- SumUp economics (1.69%; ~65% of revenue = transaction fees; One €29/mo → 0.99%): https://sacra.com/c/sumup/ · https://www.joinstored.com/blogs/sumup-pricing-explained · Square 1.75%: https://www.mobiletransaction.org/square-vs-izettle-vs-sumup/
- Toast pricing (free starter, 2.49%+15¢ / 3.09% PAYG; payments > SaaS per S-1): https://www.posusa.com/toast-pos-pricing/ · https://merchantinsiders.com/blogs/toast-fees/
- Wave freemium (2.9%+$0.60; payroll; Pro $19; paid-model rollout backlash): https://en.wikipedia.org/wiki/Wave_Financial · https://betakit.com/after-years-of-providing-free-accounting-and-invoicing-software-wave-rolls-out-paid-model/
- Referral benchmarks (4× conversion, 37% longer retention, 16% higher LTV — Wharton/Schmitt-Skiera-Van den Bulte; 84% of B2B buying starts with referral): https://faculty.wharton.upenn.edu/wp-content/uploads/2013/05/Schmitt_Skiera_VandenBulte_2013_Referrral_Programs_2.pdf · https://referral-factory.com/referral-marketing-statistics · https://www.demandsage.com/referral-marketing-statistics/
- Door-to-door / field-sales conversion (2–5% cold, up to 20–30% warm/managed): https://www.sunbasedata.com/blog/door-to-door-sales-success-rate · https://spotio.com/blog/door-to-door-sales/
- WhatsApp outreach (open ~95%, avg reply ~4%, S-Europe >50%): https://outreaches.ai/blog/cold-outreach-benchmarks · cold-email baseline: https://martal.ca/b2b-cold-email-statistics-lb/
- SMB SaaS churn (2–4%/mo good, 3–7%/mo typical micro-SMB; 70% of churn in first 90 days; TTFV<7d ≈ −50% churn): https://optif.ai/learn/questions/b2b-saas-churn-rate-benchmark/ · https://genesysgrowth.com/blog/saas-churn-rates-stats-for-marketing-leaders
- Foot-in-the-door (Freedman & Fraser 1966): https://pubmed.ncbi.nlm.nih.gov/5969145/ · IKEA effect (+63% WTP; completion boundary): https://www.hbs.edu/ris/Publication%20Files/11-091.pdf · https://en.wikipedia.org/wiki/IKEA_effect · Endowed progress (34% vs 19%, Nunes & Drèze): https://www.coglode.com/nuggets/endowed-progress-effect
- Albania restaurant prices (Tirana dinner 1,900–3,500 lek; Durrës cheapest restaurant city): https://explorertom.com/en/prices-albania-cost/ · https://politiko.al/english/e-tjera/sa-kushton-nje-vakt-ne-restorant-me-i-liri-durresi-me-i-shtrenjti-jugu-i538726 (403 on fetch — headline only; re-verify figure locally)
- Meta ads CPC (global avg ~$1.14; no Albania-specific data found — targeting-precision, not CPC, is the binding problem): https://www.superads.ai/facebook-ads-costs/cpc-cost-per-click

**Repo:** `DeliveryOS-Business-Value-Sort.md` · `docs/design/gap-blueprints-2026-07-11/G11-business-validation-week.md` · `G07-program-spine-arbiter.md` · `docs/research/2026-07-03-albania-gap-analysis.md` (R7 lek model, R10 competition) · `docs/research/food-delivery-market-brief-2026.md` · `docs/research/2026-07-03-trust-and-likeability.md` (T4/T7, onboarding choreography) · `docs/research/2026-07-04-customer-distribution-channels.md` (QR/WhatsApp/IG state) · memory `crypto-payments-build-2026-06-30.md` (payments dark, cash-first).

---

*Research-only session. One file created (this one); working tree otherwise left as found on
`feat/paleo-dinosaur-digs`.*
