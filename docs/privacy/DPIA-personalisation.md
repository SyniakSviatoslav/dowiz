# DPIA: guest taste scoring ("personalisation"), 2026-10-04

Written by lane W-MR0 (rows MR7 and MR8) after two operator decisions of 2026-10-04:
1. "смак і поведінку гостя треба оцінювати ... на пристрої і сервері".
2. A ruling the same day: personalisation is AUTOMATIC with NO checkbox. The lawful basis is
   legitimate interest, and the guest can stop it in one tap.

DECISIONS.md D0 is amended for exactly this. This file is three things:
- the Art. 35 GDPR assessment of that processing (Albanian Law 124/2024 has the matching article);
- the legitimate-interest balancing test;
- the record of what remains open.

**No lawyer has reviewed this file.** On 2026-09-24 the operator authorised shipping legal texts
without one (docs/privacy/LEGAL-DECISIONS-2026-09-24.md).

## 1. What is processed, where, and why

| | On the guest's phone (MR7) | At the venue's server (MR8) |
|---|---|---|
| Default | ON; "Remember what I like on this phone" turns it off in one tap | ON for a guest who gave a phone; "We remember your taste · turn off" (menu footer, order page) objects in one tap |
| Store | IndexedDB `dowiz.taste.v1`, per venue | the venue's `people` image, kind `taste`, beside the customer card, under the phone's pseudonymous key |
| Inputs | dishes ordered, dish cards opened, add-then-remove, dwell per dish and category, scroll depth per category, hour of day, referrer/UTM at first visit, device class (phone/tablet/desktop from width and pointer only); every time stored is a day | the guest's own orders at this venue (dish tags and categories); and, with an order, the device's aggregated vector `taste_sync`: at most 12 tags and 12 categories as integers 0..1000, with no events, no dish ids, no referrer and no device |
| Output | a taste vector and an intent guess that re-order a "For you" strip and add up to 2 "Again?" dishes | top tags and categories (60-day half-life) and a segment: new, regular, at risk or lapsed, by recency and frequency only, with the rule in words |
| Recognition | none needed | only by what the guest gave: the phone typed at checkout, or an order link the phone holds. **Never by fingerprinting** (`tools/gates/no-tracking.sh` rule 5 refuses canvas/audio read-back and device-surface reads) |
| Who sees it | the guest | the venue's owner (customer card, segment counts) and the guest (order page) |
| Retention | until the guest taps Forget or turns it off | 12 months after the last order (**GUESS, operator to confirm**); an objection or an erasure deletes it at once |

Purpose: suggest dishes. **Excluded:** a price, a discount, a promotion, an eligibility or a refusal.
`tools/gates/no-scoring.sh` refuses those uses. It allows guest-taste names only in
`workers/api/public/store/` and `workers/api/src/services/customers/taste.rs`, and refuses those files the
day they name a price, discount, promo, eligibility or refusal. Personalised pricing is not done here;
doing it would trigger the disclosure duty of the Omnibus Directive (EU) 2019/2161.

## 2. Lawful basis: legitimate interest, Art. 6(1)(f), Recital 47

**Balancing test (the three-part test of EDPB Guidelines 1/2024 on legitimate interest):**

1. *Interest.* The venue wants to show a returning guest the dishes they like, as a waiter who knows
   them would. The interest is lawful, clear and present.
2. *Necessity.* The profile holds aggregated tags and categories and a recency/frequency segment. It
   holds no raw events and no free text. It is kept under a pseudonymous key and only at the venue the
   guest ordered from. A smaller profile would not let the venue suggest anything.
3. *Balance.*
   - **Reasonable expectations (Recital 47):** a regular guest of a restaurant expects to be
     remembered. The relationship is a direct client one, and the guest gave their phone to order.
   - **Data:** low-risk. It is food preference, not health data. An allergy inferred from taste is
     never stored as a fact and never hides a dish.
   - **Effect:** none on price, eligibility or service. It only re-orders suggestions, so there is
     no Art. 22 decision.
   - **Safeguards:** the objection is one tap, visible on every menu and order page, and it deletes
     what was kept. The guest sees everything held. The data never leaves the venue, and there is no
     fingerprinting.

   On those facts the guest's interests do not override the venue's. **The result is legitimate
   interest, with the objection as the safeguard.**

**Notice (Art. 13(1)(c),(d), Art. 21(4)).** The privacy notice names the purpose, the basis and the
one-tap objection in sq/en/uk/ru (`workers/api/src/privacy/notice/`). The footer line itself is the
"explicitly brought to the attention" of Art. 21(4).

**ePrivacy Art. 5(3): the honest residual risk.** The on-device profile is information stored on the
guest's terminal. Art. 5(3) exempts storage "strictly necessary" for a service the user explicitly
requested. A "For you" strip is arguably not strictly necessary, and legitimate interest is **not** a
basis under Art. 5(3). The design relies on these facts:
- the storage is first-party;
- it never leaves the phone raw;
- the guest sees all of it and wipes it in one tap;
- the guest can turn it off.

A regulator could still read Art. 5(3) as requiring prior consent for this storage. **This lane does not
claim the device half is ePrivacy-compliant on legitimate interest.** It records the risk for the
operator's decision. The conservative fallback is one line: make the device switch default OFF again in
`workers/api/public/store/taste-device.js` (`isOn`).

## 3. Screening against the EDPB criteria (WP248 rev.01)

| Criterion | Hit? | Why |
|---|---|---|
| 1 Evaluation or scoring, incl. profiling | **YES** | taste profile and segment |
| 2 Automated decision with legal or similar effect | no | only re-orders suggestions; never hides, prices or refuses (no Art. 22 decision) |
| 3 Systematic monitoring | **partial** | menu behaviour is observed, on the phone only; it leaves the phone only as the aggregated vector |
| 4 Sensitive data | no by design | inferred allergies are not stored as health data |
| 5 Large scale | no | per venue, that venue's own guests |
| 6 Matching or combining datasets | no | one venue's orders plus the guest's own phone summary; nothing across venues |
| 7 Vulnerable subjects | no | guests ordering food (the 16-year line of LEGAL-DECISIONS §7 holds) |
| 8 Innovative technology | no | |
| 9 Prevents a right or a service | no | ordering works the same after an objection |

One criterion hits clearly and one partly. Because the processing is on by default, this full DPIA is
written rather than a screening.

## 4. Risks and measures

| Risk | Measure | Where |
|---|---|---|
| The guest does not know | footer line on every menu and on the order page, plus the privacy notice | `workers/api/public/store/taste-device.js` (`rememberLine`), `workers/api/public/store/taste-venue.js` |
| The objection is hard or does not stick | one tap; filed in the consent log as a `personalisation` withdrawal (`Method::Objection`); the profile is deleted in the same request; the phone stops sending; a later `taste_sync` is refused 400; a phone without an order link sends `taste_off` with its next order | `workers/api/src/services/customers/taste_routes.rs`, `crates/dowiz-hub/src/consent.rs` (`objected`) |
| Behaviour data leaks to the server | only the aggregated vector is sent; the gate refuses beacons, scroll/visibility telemetry, raw profile keys in a body, and `taste_sync` without `notObjected()` on its line | `tools/gates/no-tracking.sh` rules 1-4 |
| A guest is recognised against their will | recognition only through the phone they type or an order link they hold; fingerprint surfaces refused | `tools/gates/no-tracking.sh` rule 5 |
| The score is used to price or refuse | the gate refuses those names in the scorer files | `tools/gates/no-scoring.sh` |
| It is kept forever | absent 12 months after the last order. **The physical sweep (P6) is not built** | `workers/api/src/services/customers/taste.rs` |
| The segment reads as a judgement | 4 segments by recency/frequency, with the rule shown; no tier, no spend | same |

Residual risk: low for the server half. For the device half, the ePrivacy Art. 5(3) reading in §2 is
the open risk.

## 5. Data-subject rights

- **Access and export (Art. 15, 20):** `GET /api/order/:id/taste`; the order page shows it.
- **Objection (Art. 21):** "turn off" in the footer, or "Turn off and delete" on the order page. The
  objection is absolute for this purpose and needs no reason.
- **Erasure (Art. 17):** the owner's forget on the customer card removes the profile, and the erasure
  also withdraws the purpose.
- **Art. 22:** no decision with legal or similarly significant effect is taken on the score.

## 6. Open

1. Confirm retention: 12 months is a GUESS.
2. The P6 sweep, which physically deletes expired profiles, is not built.
3. Decide on the ePrivacy Art. 5(3) residual risk for the device half (§2).
4. No "turn it back on" is offered after an objection. The hub accepts a re-grant act (`Given`
   naming the personalisation sentence), but no screen sends one.
5. Recognition by a phone typed at checkout is NOT built as a lookup that returns a profile. Anyone
   could type a neighbour's number and read their taste. Recognition uses the order link instead
   (see the lane verdict).
6. Staff and courier personal KPIs need their own D0 amendment. This lane did not do them.

## W-SENSE addendum (2026-10-04): taste, texture, aroma, context, mood, history, offers

Operator ask of 2026-10-04: dishes describe their taste (six axes 0-5), texture and aroma; guests are
profiled on the same axes. What changes in this assessment, and what does not:

| | Device | Server |
|---|---|---|
| New inputs | the declared axes of the dishes already counted (no new behaviour signal); the venue's moment at order time (band, weekday/weekend, weather AT THE VENUE) | the same axes of the guest's own orders; the venue's moment at placement; the device may add `taste_sync.sense` (closed vocabulary, integers 0..1000) |
| New outputs | ranking by match with the guest's axes, with the moment, with the session's mood; "because you often pick smoky + crispy"; "your taste over the year" | `sense`, `because`, `contexts`, `when` (weekday x band counts), `history` (12 monthly snapshots) in the guest's own view and on the owner's card |
| Mood | one-tap chips, held in page memory only; never stored, never sent (`tools/gates/no-tracking.mjs` rule 6) | none |
| Location | none: the context route takes no query; the weather is asked for the VENUE's coordinates, rounded to ~1 km (`tools/gates/no-tracking.mjs` rule 7; `services/venue/context.rs` tests) | none |
| Device context | which wallet the browser offers (Apple Pay / Payment Request), to order the pay buttons; a yes/no, never stored or sent; no fingerprint surface is read | none |

**Owner analytics.** The segment builder answers COUNTS, a six-month trend and the busiest
weekday x band, never a list of people (`services/customers/taste/builder.rs`). Its keys are the
closed sensory vocabulary only: no allergen, no health word can be a segment (tested), so no
special-category inference is targeted. Allergens only ever filter or warn for the guest's own safety.

**Personalised offers (Omnibus 2019/2161, GDPR 21/22).** An offer to a taste segment is a campaign
(segment `taste`): (1) it reaches only guests with a MARKETING consent on the campaign's channel
(today WhatsApp; the consent fold mints the witness, `campaigns/audience.rs`); (2) its words are
written by the server -- "Personalised offer", the dish, the dish's PUBLIC menu price -- and the
owner's own text and numbers are discarded; (3) a taste offer carries no promo code (a code is a
reduction; a reduction chosen by taste would be personalised pricing) -- refused 400; (4) nothing
reads a segment to set a price, a fee or a refusal (`tools/gates/no-scoring.sh`; the order total of
a guest in a segment equals the public price, tested). A loyalty reward on top is allowed by the
operator but is not built.

**Retention, rights.** Unchanged: the new fields live inside the one `taste` record (12 months after
the last order; objection, export via `GET /api/order/:id/taste`, erasure with the card).

**Residual risks added.** (a) Monthly snapshots make a year of taste visible to the owner on the
guest card -- mitigated by the one-tap objection deleting all of it and by showing the guest the
same history; (b) the weather call reveals the venue's location to Open-Meteo (public anyway; no
guest data in the call); (c) the sushi starter defaults are typical public profiles, not the
venue's recipe -- shown as a draft the owner must review and save.
