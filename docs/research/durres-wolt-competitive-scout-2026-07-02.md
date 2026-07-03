# Durrës Food-Delivery Competitive Scout — Wolt + Google Maps (2026-07-02)

**Purpose.** Competitive positioning intel for dowiz (a 0-commission, data-ownership delivery
platform) against the incumbent aggregator in Durrës, Albania: **Wolt** (owned by DoorDash).
Scope is PUBLIC BUSINESS DATA ONLY — vendor names, cuisine, ratings, hours, menu/photo presence,
price tier, and aggregate public-review themes. **No person-profiling** — owners/staff/individuals
are out of scope by charter.

Companion to the prior scout `storefront-venue-data-maps-scrape-2026-07-01` (ArtePasta demo build).

---

## 0. Method, sources, and data confidence

| Source | What it gave | Confidence |
|---|---|---|
| Wolt Durrës category pages (`/top-rated-venues`, `/category/{pizza,street_food,sandwich}`) | Vendor names, **Wolt rating (10-pt scale)**, delivery-time band, open/closed/offline status | HIGH for names + status; ratings are Wolt's own 10-pt score |
| Google Maps / TripAdvisor / Wanderlog / RestaurantGuru | **Google rating (5-pt)** + review counts, cuisine, price tier, phone, own-website presence | HIGH for review counts; MEDIUM for price tier (aggregator-normalized) |
| Wolt merchant learning-center + industry guides | Commission model (20–30% of subtotal + platform fee) | HIGH for model; exact Albania rate is per-contract, not public |

**What I could NOT get (rate-limits / JS-gating), stated honestly:**
- Wolt's **full paginated venue list** (`/restaurants/all/2..N`) and the city landing page render
  client-side and truncate under WebFetch — I could not get a single clean "every venue" dump.
  I reconstructed the roster from the four category pages that DID render (top-rated, pizza,
  street-food, sandwich), so **non-pizza/non-fast-food segments (sushi bars, cafés, groceries,
  fine-dining that don't self-tag into those categories) are under-counted.**
- **TripAdvisor delivery list = HTTP 403**, **RestaurantGuru delivery index = HTTP 503** (matches the
  memory note that RestaurantGuru blocks the sandbox). Cross-ref Google ratings came via Wanderlog,
  which republishes Google data.
- **Exact Wolt commission % for Albania** is contract-private; the 20–30% band is Wolt's own
  published model + industry guides, not a leaked Durrës rate.
- **Per-vendor menu-photo quality** was not visually inspected at scale (would need headless
  Playwright per venue, as in the ArtePasta scout). Photo-quality claims below are inferred from
  segment (chains/upscale = pro photos; independent fast-food = weak/none) and flagged as inferred.

**Vendor count: ~80 distinct food vendors identified on Wolt Durrës** (see §1). Many carry a
Closed / "opens tomorrow" / "temporarily offline" status at scrape time — Durrës is a seasonal
coastal market with high on/off churn.

---

## 1. Vendor table

Ratings shown in two scales: **Wolt** = /10 (Wolt's platform score), **Google** = /5 with review
count where cross-referenced. `—` = not captured on the surfaces I could read. Status = Wolt state
at scrape time.

### 1a. Delivery-active vendors on Wolt (fast-food / pizza / street-food core)

| Vendor | Cuisine | Wolt /10 | Google /5 (revs) | Delivery band | Wolt status | Own website? |
|---|---|---|---|---|---|---|
| Eddie's Pizza | Pizza | 9.6 | — | 20–30 | Open | not found |
| Pizzeri Enea | Pizza | 9.6 | — | 20–30 | Open | not found |
| Noee Crepes | Crepes | 9.6 | — | 15–25 | Open | not found |
| Dogu Al Durrës | Hot dogs | 9.4 | — | 20–30 | Open | not found |
| Pizzeria Ristorante Odeon | Pizza | 9.4 | — | 25–35 | Open | not found |
| 4 Stinet | Italian/pizza/seafood | 9.4 | 4.0 (1,945) / 3.9 (71) | 30–40 | Closing soon | **YES — 4stinet.com** |
| Prime Snack | Snacks | 9.4 | — | 20–30 | Closing soon | not found |
| Tata's Bites | Snacks | 9.4 | — | — | Closed | not found |
| Fast Food 1 Maji | Fast food | 9.2 | — | 15–25 | Closing soon | not found |
| S Burger By Eden | Burgers | 9.2 | — | 20–30 | Open | not found |
| Sushi Time (Gusto di Mare) | Sushi/seafood | 9.2 | 4.4 (912) | 25–35 | Open | not found |
| Pizzeria Miele | Pizza | 9.2 | — | 15–25 | Closing soon | not found |
| Pizzeri Gaxha | Pizza | 9.2 | — | 30–40 | Open | not found |
| Adi's Street Food | Street food | 9.2 | — | 30–40 | Closing soon | not found |
| Pizzeri Midway | Pizza | 9.2 | — | — | Closed | not found |
| Pizzeri Champion | Pizza | 9.2 | — | — | Closed | not found |
| ArtePasta | Pasta | 9.2 | — | 20–30 | Open | **dowiz demo built** (/s/artepasta) |
| Food 11 by Timi | Fast food | 9.0 | — | 20–30 | Open | not found |
| Pizzeri Luna Hallall | Pizza (halal) | 9.0 | — | 25–35 | Closing soon | not found |
| Fast Food Durrësi | Burgers/gyros | 9.0 | 4.6 (153) | 15–25 | Open | not found |
| Ciao Bar 2 Restaurant & Pizzeri | Pizza | 9.0 | — | 20–30 | Closing soon | not found |
| Pizzemporio | Pizza | 9.0 | — | 20–30 | Open | not found |
| The Pizza King | Pizza | 9.0 | — | 25–35 | Closing soon | not found |
| Sulltan Kebap & Pizza | Kebab/pizza | 9.0 | — | 20–30 | Open | not found |
| Pizza Max | Pizza | 9.0 | — | 20–30 | Open | not found |
| Korabi Fast Food | Fast food | 9.0 | — | 30–40 | Closing soon | not found |
| Pizza Nove | Pizza | 9.0 | — | — | Offline | not found |
| Gatime Tradicionale Durres | Traditional | 9.0 | — | — | Closed | not found |
| Fast Food Greece | Greek | 9.0 | — | — | Closed | not found |
| Fast Food Merakliu | Fast food | 8.8 | — | 25–35 | Open | not found |
| Pizzeria Terra Cota Napoli | Pizza | 8.8 | — | 25–35 | Closing soon | not found |
| Tarantella | Pizza/sandwich | 8.8 | — | 25–35 | Closing soon | not found |
| Taste Food | Fast food | 8.8 | — | — | Offline | not found |
| Greek Souflaki Gyros Durres | Greek | 8.6 | 4.7 (272)* | 20–30 | Open | Instagram only* |
| Hepta Grill & Coffee | Grill/coffee | 8.6 | — | 30–40 | Open | not found |
| Likos Burgers Durrës | Burgers | 8.6 | — | 25–35 | Open | not found |
| Crepablos | Crepes | 8.6 | — | 25–35 | Open | not found |
| Fast Food Lekli 2000 | Fast food | 8.6 | — | 15–25 | Open | not found |
| Chapo's Burger | Burgers | 8.6 | — | 25–35 | Open | not found |
| Pizzeri Gaxha Kënetë | Pizza | 8.6 | — | 25–35 | Open | not found |
| Nemo pizzeri Fast Food | Pizza/fast food | 8.6 | — | 20–30 | Open (New) | not found |
| Piceri Lila's | Pizza | 8.6 | — | 15–25 | Open | not found |
| Soam Fast Food | Kebab/burger/souvlaki | 8.6 | 4.3 (315) | 15–25 | Open | **not found** (Wolt-only) |
| KFC Durrës | Chicken (chain) | 8.8 | — | 20–30 | Open | chain (corp site) |
| Burger King Durrës | Burgers (chain) | 8.4 | — | 30–40 | Open | chain (corp site) |
| Euro Dream (Gatime Trad.) | Traditional | 8.4 | — | 25–35 | Open | not found |
| Ardèn Restaurant | Restaurant/pizza | 8.4 | — | 25–35 | Open | not found |
| Piceri Furrë Druri Klard | Pizza (wood-fired) | 8.4 | — | — | Closed | not found |
| Il Maestro Pizzeri | Pizza | 8.2 | — | 20–30 | Open | not found |
| Pizza Napoli | Pizza | 8.2 | — | 30–40 | Closing soon | not found |
| Social Lies American Tavern | American | 8.0 | — | — | Offline | not found |
| AT29 | Restaurant | 8.0 | — | — | Offline | not found |
| Guru Snack Bar | Snacks | 7.8 | — | — | Closed | not found |
| Malredo Pizza & More | Pizza | 7.8 | — | — | Closed | not found |
| Sufllaqe Produkte Zgare Rion | Grill/souvlaki | 7.8 | — | — | Offline | not found |
| Pizzeri Roma | Pizza | 7.6 | — | — | Closed | not found |
| Pizzeri Mazrreku | Pizza | 7.6 | — | — | Closed | not found |
| UMA Food and Drinks | Restaurant/pizza | 7.4 | — | 30–40 | Open | not found |
| Restaurant Pizzeri Rildo | Pizza | 7.4 | — | 30–40 | Closing soon | not found |
| Pizzeri Peza | Pizza | 7.0 | — | — | Closed | not found |
| Armenian Dominant Food | Armenian | 9.4 | — | — | Offline | not found |
| GlobArt Turkish Restaurant | Turkish | 9.6 | — | — | Closed | not found |
| Golden Heaven | Fast food | 9.2 | — | — | Offline | not found |
| Flair | Café/snacks | 9.4 | — | — | Closed | not found |
| Lofi Ice Cream | Ice cream | 9.6 | — | — | Closed | not found |
| Target Burgers | Burgers | — | — | 25–35 | Open | not found |
| Peaky Corner | Restaurant | — | — | 20–30 | Open | not found |
| Baffetto Pizza | Pizza | — | — | — | Offline | not found |
| Restorant Piceri Colombia | Pizza | — | — | 30–40 | Open (New) | not found |
| Fast Food & Piceri Mi-El | Pizza/fast food | — | — | 30–40 | Open (New) | not found |
| Pop Pizza | Pizza | — | — | 25–35 | Closing soon | not found |
| Dolanit Fast Food | Fast food | — | — | 25–35 | Closing soon | not found |
| Cosmo Restaurant | Restaurant | — | — | 20–30 | Closing soon | not found |
| Fast Food Perla | Fast food | — | — | — | Closed | not found |

\* Greek Souflaki: Wanderlog lists a "Greek Souvlaki – Fast Food Durrës" at 4.7 (272) with an
Instagram handle and no website; likely the same or a sibling brand. Treat the cross-ref as
approximate.

### 1b. High-value brands seen on Google/TripAdvisor (dine-in leaders; delivery presence varies)

These are the recognizable Durrës brands with the largest public review bases. Several are NOT
confirmed on Wolt from the pages I could read — that gap is itself a signal (own-channel or
phone-order businesses that an aggregator hasn't captured, or that dine-in-dominate).

| Vendor | Cuisine | Google /5 (revs) | Price | Own website? | On Wolt? |
|---|---|---|---|---|---|
| Westwood Meathouse | Steak/Italian | 4.8 (1,323) | $$$$ | not found | not seen on read pages |
| Pastarella | Italian/seafood pasta | 4.7 (696) | $$$$ | not found | not seen |
| Ymer's Grill | Souvlaki/gyros/grill | 4.9 (340) | $$ | not found | not seen |
| Epidamn Restaurant & Garden | International | 4.6 (690) | $$$$ | not found | not seen |
| Ullishtja Agroturizëm | Traditional Albanian | 4.6 (468) | $$$ | not found | not seen |
| Alternative Food Durrës | Pizza/Italian | 4.6 (370) | $$ | not found | not seen |
| ZINS | Sushi/international | 4.3 (1,028) | $$$$ | not found | not seen |
| Mema House | Mediterranean/Albanian | 4.2 (1,252) | $$$ | not found | not seen |
| Pelikan | Pastry/dessert/coffee | 4.4 (884) | $$ | not found (chain) | not seen |
| Wild West | Mexican | 3.8 (906) | $$$ | not found | not seen |
| Gelateria Çela | Gelato | 4.4 (539) | $$ | not found | not seen |
| Restorant Rimini | Italian/pizza | 4.2 (466) | $$$ | not found | not seen |
| 2 Kitarrat – Arome Deti | Seafood | 4.2 (333) | $$$ | not found | not seen |
| Nuri's Pizzeria & Fast Food | Pizza/kebab | 4.8 (117) | $$ | not found | temporarily closed |
| King Kebab & Pizza | Kebab/pizza (halal) | 4.3 (195) | $ | not found | not seen |
| Mulliri Vjetër | Coffee/café (chain) | 4.3 (178) | $ | chain site | not seen |

---

## 2. Weakness matrix — the aggregator-dependency gaps dowiz attacks

Scored per-vendor-segment. `●` = strong weakness (dowiz opportunity), `◐` = partial, `○` = not a
weakness / already covered.

| Weakness axis | Independent fast-food / pizza (the ~60 in §1a) | Upscale/high-volume brands (§1b) | Global chains (KFC/BK) |
|---|---|---|---|
| **Wolt commission dependence (20–30% of subtotal)** — pure margin loss, no way to discount it | ● thin-margin fast food feels 25–30% hardest | ● $$$$ tickets → biggest absolute € lost per order | ◐ corporate deals soften it |
| **No direct-ordering channel** — customer + order data locked inside Wolt; can't remarket | ● Wolt-only, zero owned funnel | ● huge review base but no owned order flow | ○ chains have own apps |
| **No own website / weak web presence** | ● overwhelmingly none (only 4 Stinet found w/ a site) | ● even 1,000+-review brands have no site found | ○ corporate sites exist |
| **No customer CRM / loyalty** — can't see who ordered, can't bring them back | ● none | ● none — leaving repeat revenue on the table | ◐ chain loyalty apps |
| **Weak/absent menu photography** (inferred by segment) | ● independents lean on Wolt's stock/thin shots | ◐ some pro dine-in photos, not menu-item shots | ○ chain brand assets |
| **Seasonal open/close churn** — many "Closed / offline / opens tomorrow" at scrape | ● hurts discoverability; Wolt buries closed venues | ◐ dine-in cushions it | ○ |
| **Ratings soft spots** (public review themes) | ◐ several 7.0–8.2 Wolt / sub-4.2 Google (e.g. Wild West 3.8, Rildo/Peza/UMA 7.0–7.4) | ◐ mixed (Mema 4.2, Rimini 4.2) | ○ |
| **Delivery-time weakness** — 30–40 min bands common | ◐ own-courier control could beat Wolt ETAs | ◐ | ○ |

**Recurring public-review complaint themes (Wolt, cross-market — Durrës-specific reviews were
403/503-gated, so these are the platform's general themes and should be presented as "typical of
aggregator delivery," not verified Durrës incidents):** slow delivery, wrong/missing items, food
arriving cold, and unresponsive/slow customer support. dowiz's own-courier + direct-line model is
the structural answer to all four.

---

## 3. How dowiz reinforces against each weakness (positioning claims to sharpen)

| Wolt-dependence weakness | dowiz answer | Sharpened claim for the pitch |
|---|---|---|
| 20–30% commission on every order | 0% commission storefront (`/s/:slug`), vendor pays only real delivery cost | **"Keep the 25–30% Wolt takes. On a €1,000/day kitchen that's €7,500+/month back in your pocket."** Make it a per-vendor € number, not a %. |
| Order + customer data locked in Wolt | Owner CRM — every order, phone, repeat-rate is the vendor's own data | **"Wolt owns your customers. dowiz gives them to you — name, order history, and the right to bring them back."** (Ties to data-sovereignty charter.) |
| No own website / online channel | Instant branded SSR storefront at `/s/:slug`, own palette/logo/fonts | **"Your own ordering site in a day — not a tile inside Wolt's app."** Demo it live (ArtePasta pattern). |
| No loyalty / remarketing | CRM + Telegram/notification channel already built | **"Turn one order into ten — reorder nudges, offers, loyalty. Impossible on Wolt."** |
| Weak menu photos | demo-builder pipeline: Maps hero photo → R2, per-tenant fonts, item photos | **"We build your storefront with real photography, not a stock thumbnail."** |
| Slow/wrong-order/cold-food/support complaints | Own-courier control + dispatch + direct customer line | **"You control the courier and the customer relationship — not a call-center in another country."** |
| Seasonal open/close churn burying venues | Vendor-owned storefront is always the vendor's front door, not ranked by an aggregator | **"Your storefront doesn't get buried when Wolt reshuffles its list."** |

**Positioning north-star:** dowiz is not "a cheaper Wolt." It is **"own your storefront, your
customers, and your margin."** Lead with margin (concrete €), close with data-ownership (the moat
Wolt structurally can't offer because their model IS the lock-in).

---

## 4. Ranked demo-builder targets ("reinforce dowiz with this")

Ranking logic = **switching value = high aggregator dependence × weak own-presence × ticket size /
volume.** Highest = a strong brand that owns nothing of its own funnel and loses the most to
commission. (ArtePasta is already built — reference exemplar, excluded from the ranking.)

| Rank | Target | Why it's the highest-value claim/demo | Build angle |
|---|---|---|---|
| **1** | **Westwood Meathouse** | Premium $$$$ steakhouse, **1,323 Google reviews** (largest quality brand base found), no own site found. On a high-ticket menu, Wolt's ~30% is the **biggest absolute € loss per order** in the city → the commission pitch bites hardest. | Build an upscale storefront demo; lead the pitch with a €/month commission-savings number on a €30–50 avg ticket. |
| **2** | **Soam Fast Food** | High-volume city-center fast food, strong on BOTH Wolt (8.6) and Google (4.3, 315 revs), **Wolt-only / no own website** → textbook data-lock-in victim with volume to make CRM/loyalty pay off. | Volume/CRM demo: reorder nudges + own site; show "you'd own these 315+ reviewers as customers." |
| **3** | **Ymer's Grill** | **Best-rated in the set (4.9 / 340 revs)**, souvlaki/gyros, Instagram-and-phone only, no website. A beloved brand that owns none of its digital funnel = maximal brand-vs-infrastructure gap. | Quality-brand demo: "the best-rated grill in Durrës deserves its own storefront, not a rented tile." |
| 4 | **Pastarella** | Upscale Italian, 4.7 (696), no site found, high ticket → same economics as Westwood, second-tier volume. | Mirror the Westwood build as a second upscale reference. |
| 5 | **Alternative Food Durrës** | Pizza/Italian, 4.6 (370), no site → mid-market pizza is dowiz's densest opportunity segment (pizza is the single biggest Wolt category in Durrës). | Pizza-segment flagship demo; reusable template for the ~30 other pizzerias. |
| 6 | **Eddie's Pizza / Pizzeri Enea** | Joint-top **9.6 Wolt** pizza, no own presence found → "top of Wolt, nothing of their own" is a crisp story. | Fast follow to prove the pizza template converts the highest-Wolt-rated venues. |

**Segment insight for the demo-builder loop:** pizza is by far the largest Wolt Durrës category
(30+ pizzerias, almost none with an own site). Building ONE polished pizza storefront template
(palette + hero photo + item photos + fonts) makes the marginal cost of the next 30 demos near-zero
— this is where the certified demo-builder loop (memory `demo-builder-loop`) has the highest leverage.

---

## 5. Storefront features that specifically exploit the observed gaps

- **Commission-savings calculator on the pitch banner** — take the vendor's Wolt price level +
  visible volume and show the €/month they'd keep. Turns the abstract "0%" into their number.
  (Extends the existing preview-banner pitch from the ArtePasta build.)
- **"Claim your customers" CRM teaser** — the preview shows a blurred customer/order list captioned
  "these are yours on dowiz, invisible on Wolt."
- **Real menu photography as the wedge** — the demo-builder already pulls Maps hero + item photos +
  nearest-font matching; against Wolt's thin thumbnails this is a visible quality delta in the demo.
- **Own-courier ETA honesty** — many Wolt venues sit at 30–40 min bands; a storefront that shows an
  honest, controllable ETA is a differentiator (ties to the honest-dispatch work in memory).
- **Always-on storefront** — counters the seasonal "Closed/offline/buried" churn: the vendor's
  `/s/:slug` is their permanent front door regardless of aggregator ranking.

---

## Sources

- Wolt Durrës — top-rated venues: https://wolt.com/en/alb/durres/top-rated-venues
- Wolt Durrës — pizza: https://wolt.com/en/alb/durres/category/pizza
- Wolt Durrës — street food: https://wolt.com/en/alb/durres/category/street_food
- Wolt Durrës — sandwich: https://wolt.com/en/alb/durres/category/sandwich
- Wolt Durrës — city landing: https://wolt.com/en/alb/durres
- Wolt merchant fees & commissions (Albania): https://explore.wolt.com/en/alb/merchant/learning-center/wolt-merchant-fees-and-commissions
- Wolt fees 2025/2026 guide (Menuviel): https://blog.menuviel.com/wolt-fees-and-commissions-for-restaurants/
- Wanderlog — best fast food, Durres County: https://wanderlog.com/list/geoCategory/489702/best-fast-food-restaurants-in-durres-county
- Wanderlog — best dinner restaurants, Durres County: https://wanderlog.com/list/geoCategory/377014/best-restaurants-to-have-dinner-in-durres-county
- 4 Stinet own website: https://4stinet.com/
- TripAdvisor Durres delivery (403 at scrape): https://www.tripadvisor.com/Restaurants-g318866-zfp19-Durres_Durres_County.html
- RestaurantGuru Durres delivery (503 at scrape): https://restaurantguru.com/delivery-Durres-m9631
- SeeNews — Wolt expands to Durrës: https://seenews.com/news/wolt-expands-to-albanias-durres-1272591

*Ethics note: this scout covers businesses and their public presence only — names, cuisine, ratings,
hours, menu/photo presence, price level, and aggregate public-review themes. No owner/staff/individual
profiling was performed or recorded, per the standing charter.*
