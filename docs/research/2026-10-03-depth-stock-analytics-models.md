# Depth over breadth: stock better than Poster, analytics better than Toast, and which free models are worth it

Lane **W-DEPTH**. Research and design only. Written 2026-10-03 against `main` 34aa2bc0. No compute was run: no cargo, node, python, gates or model downloads. Every external fact carries its URL. The access date is **2026-10-03** unless another date is given.

**Labels used in this document:**
- **cited**: the page was fetched and the fact is on it.
- **snippet**: only a search-engine summary was available, because squareup.com, square.com and pos.toasttab.com return HTTP 403 to the fetcher and iiko.ru refused the connection.
- **GUESS**: my own estimate, not a measurement.

The operator's brief, verbatim: "Ширина замість глибини ... Погано, необхідно дослідити конкурентів та покращити наявний сервіс шліфуванням - зокрема кращим складським обліком аніж Poster, та кращою аналітикою за Toast - huggingface моделі варто дослідити для цього, можуть бути безкоштовні моделі що вирішать або покращать усі сервіси".

The binding rule of 2026-10-03 (`live-proof-rule`): a row is proven only by a LIVE probe on `qa-durres.dowiz.org`, through the real Worker, DO and R2 and any real external service. Each row also needs a written contract, meaning a schema, a version and a description. Mocks are the regression layer only.

---

## 0. The finding that reorders everything

**The stock engine already has more depth than Poster. What is missing is data, history and a few owner-facing reports.**

- **The engine.** `crates/dowiz-hub/src/stock*` is about 5,100 lines of tested, deterministic ledger. It covers:
  - weighted-average cost (WAC);
  - lots with expiry and first-expiry-first-out (FEFO);
  - semi-finished products with production acts;
  - separate cleaning and cooking losses;
  - signed write-offs with reasons;
  - stocktake sessions with drift;
  - returns from the door and till voids;
  - a cost stamp at the moment of sale.

  Section 1 compares this against Poster line by line.
- **The data.** The live venues hold **165 dishes, 0 recipes and 0 supplies** (memory `dowiz-stock-ledger-works-but-is-off`, re-checked by W-SYSCHECK on 2026-10-02). dubin's inventory was wiped on operator order (b35c8402) for "the new person" to fill in.
- **What follows.** An owner today sees none of the depth. The biggest single lever is **getting a venue's shelf filled in under an hour**: a starter nomenclature, recipe skeletons, a photo of the invoice, a count by voice. More ledger features come after that.

**The analytics engine is honest but shallow and short-sighted:**
- The owner pane offers **7 or 30 days only** (`workers/api/src/services/analytics/fold.rs:57-62`). It shows one window with no comparison, and no weekday-by-hour view.
- The kitchen numbers allow up to 62 days (`services/analytics/kitchen.rs:32`). However, the hot order log keeps only **30 days** (`workers/api/src/hubstore.rs:955`, `HOT_KEEP_MS`). `fold_kitchen` reads the hot log only (`workers/api/src/hubdo/reads.rs:59-72`).
- **Probable defect (verify live, row P2):** for days 31-62 the stock side, which is the stock log and is not rotated, shows draws and receipts while the sales side shows zero. The food-cost and theoretical-versus-recorded lines would then be wrong for the older half of a 62-day window.
- **Forecasting is impossible today.** No surface keeps more than 30 days of sales in a form a 10 ms request can read.

---

## 1. Competitors in depth

The two helper research passes are summarised here, with their sources. Vendor names are used as references only.

### 1.1 Who is even in the Albanian market

| Vendor | Sells in Albania? | Albanian UI? | Source |
|---|---|---|---|
| Toast | **No.** Markets are US, CA, IE and UK, and the availability article also names AU. xtraCHEF and Benchmarking are **US-only**. | No | https://support.toasttab.com/en/article/Toast-Product-Availability-in-Australia-Canada-Ireland-and-the-United-Kingdom (updated 2026-09-15, cited) |
| Square | **No.** Card acceptance covers US, CA, AU, JP, UK, IE, FR and ES. | No | https://squareup.com/help/gb/article/4956 (snippet) |
| Poster | No Albanian site found. Locales are EN, ES, KZ, RU, RU-KZ and UK. | **No** | https://joinposter.com/en (cited) |
| Syrve | Unverified. It claims 50+ countries, and its locales are EN-GB, IT and EN-AE. | Unverified | https://www.syrve.com/en-gb/syrve-pos-for-restaurants (cited) |
| Lightspeed K | Unverified. Insights cover "North America, Australia, and New Zealand". | Unverified | https://k-series-support.lightspeedhq.com/hc/en-us/articles/7339064083739-Magic-Menu-Quadrant (cited) |
| easyPos (local) | Yes. Certified for Albanian fiscalisation. Offers warehouse, "composed articles" and P&L. | sq/en/it | https://easypos.al/llms-full.txt (cited) |

**Consequence.** "Better than Toast" is a product bar, not a market fight. No one dowiz's venues can buy from offers Toast's analytics in Albanian, Ukrainian and Russian. The real local alternatives are certified fiscal tills such as easyPos and Logic (https://www.logic.al/en/clients/). They have shallow stock and no AI.

### 1.2 Inventory: feature by feature

The dowiz column cites the tree at 34aa2bc0. "Gap" is what dowiz lacks, or "—" when dowiz is at parity or better.

| Capability | Poster (Склад) | Syrve / iiko | MarketMan | Toast xtraCHEF / IQ Inventory | Lightspeed K | Square | **dowiz today (file:line)** | Gap |
|---|---|---|---|---|---|---|---|---|
| Goods receipt, manual | yes [P1] | yes | yes | yes | yes | yes | `POST /api/owner/stock/received`, with price, supplier, invoice, lot and expiry (`stock/meta.rs:1-14`, `stock/cost.rs:4-10`); a multi-line invoice sheet (`public/admin/ingredients-count.js:60`) | — |
| Receipt from an invoice photo | Postie AI, Business+ plan [P2] | AI Invoice Scanner, "18+ languages" [S1] | yes, 50 scans on Starter [M1] | yes [T1] | partners only | through MarketMan [Q2] | **none** | **yes**, row P10 |
| Receipt from e-invoice / EDI | **no EDI mentioned** [P2] | unverified | EDI, capped by plan [M1] | EDI [T1] | unverified | unverified | none. Albania mandates B2B e-invoices as UBL XML through the CIS with a NIVF since 2021-07-01 [AL1] | **an opportunity no global vendor can take**, row P11 |
| Gross/net/yield on a recipe | **one loss % per ingredient** [P3] | cold and hot losses (earlier report, iiko.help; not reachable today) | portioned costs | Yield + Consumable portion (earlier report) | unverified | unverified | **two losses, clean `cleanPm` and cook `cookPm`, per supply, plus net/out per line**; the measured yield of each batch can become the default in one tap (`stock/event.rs:111-114`, `public/admin/ingredients.js:178-179`) | — (dowiz is deeper) |
| Semi-finished, production act | preparations, manufacture, butchery [P4] | yes | unverified | batch prep (IQ Inv.) | production batches [L1] | through MarketMan | **ПФ cards expanded exactly, `Cooked`/`Made` act with weighed output, a carry of fractional draws** (`stock/act.rs:1-14`, `stock/carry.rs:1-24`, `stock/basket.rs:1-8`) | — |
| Costing | **weighted average** [P5] | unverified | unverified | **FIFO + lots** (IQ Inv., limited release) [T2] | unverified | unverified | WAC as a fold; **cost stamped on the order line at sale** (`stock/cost.rs:12-25`); lots in FEFO order (`stock/lots.rs:1-15`); no FIFO COGS view | small: an optional FIFO-by-lot report |
| Multiple storages and transfers | **yes**, console and register [P6] | unverified | unverified | sub-locations "not yet supported" [T2] | transfers between locations [L2] | through MarketMan | **one shelf per venue; no storage, no transfer event** | **yes**, row P12 (bar / kitchen / freezer) |
| Deduction on sale | by station [P7] | yes | from POS | item-level 86 only, natively [T3] | recipes | Square Recipes (beta) [Q1] | reserve at placement, consume at PREPARING, release on cancel; **refuse a new order on a counted shortfall (the automated 86)** (`stock.rs:15-19`) | — (dowiz is stronger: refusal at checkout, not after) |
| Modifiers draw stock | yes ("ingredient inside a modifier", earlier report) | unverified | unverified | **"recipe modifiers not yet supported"** [T2] | unverified | through MarketMan | **no**: options carry no BOM (`crates/dowiz-hub/src/modifiers.rs` has no BOM path; R13 open) | **yes**, row P8 |
| Write-off with reasons | configurable reasons [P8] | yes | waste reports | configurable reasons [T2] | waste | waste | fixed reasons Spoiled/Dropped/Unsold/Returned/StaffMeal, **signed by a named person**, valued at the WAC of the moment (`stock/event.rs:7-15`, `public/admin/ingredients-count.js:101`) | — |
| Stocktake | full/partial, barcode, checklist [P9] | guided mobile counts [S1] | mobile, partial [M2] | phone counts offline [T4] | counts | — | session counts, one decision, drift shown live while typing (`public/admin/ingredients-count.js:20-40`) | an offline count and a **voice count** (row P14) |
| Theoretical vs actual | count difference plus a movement report, **no named AvT** [P10] | variance | AvT on every plan [M1] | AvT, needs 2 counts [T5] | discrepancy report [L2] | through MarketMan | **recipes-say and log-says shown side by side** with drift value (`services/analytics/kitchen/report.rs:5-7, 81-90`), but **no per-count-window "unexplained" figure or % of revenue** | **yes**, row P4 |
| Par / reorder / purchase order | **"Poster doesn't create inventory purchases automatically"** [P11] | automatic purchase plan [S2] | par + suggested POs [M3] | par + order guide [T4] | reorder points + POs [L1] | through MarketMan | days of cover and a reorder hint from average daily use (`kitchen/report.rs:16-35`: below 3 days, buy 7 days); `lowAt` floor; **no supplier entity, lead time or order list** | **yes**, row P5 |
| Forecast-driven ordering | Postie AI "forecasts purchases" (3rd-party) [P12] | AI sales prediction [S2] | Smart Ordering [M1] | no | no | through MarketMan | none | **yes**, rows P5 and P6 |
| Lots / expiry | **not found** | unverified | expiry (3rd-party) | lots (IQ Inv.) | unverified | unverified | **lots with expiry, FEFO, a daily `stock.expiring` alert** (`stock/lots.rs`, `services/operations/stock/tell.rs:9`) | — (dowiz is ahead of Poster) |
| Low-stock alert | **email only**, hourly or at shift close [P13] | — | — | — | — | email/dashboard [Q3] | **Telegram groups**: `stock.low` on crossing, `stock.expiring` daily, received/wasted (`stock/tell.rs:5-9`) | — (dowiz is ahead) |
| Supplier records, price history | supplier records; price history unverified [P14] | price-list comparison [S1] | price-change report | vendor management | suppliers [L1] | — | supplier as free text on a receipt (`stock/view.rs:141-153`); **per-supply price points and change ‰** (`kitchen/report.rs:106-118`) | a supplier **card** (contact, days, lead time), row P5 |
| Allergens / nutrition | not found | — | per ingredient | fixed per-country lists (IQ Inv.) | — | — | EU-14 three-way on the dish (`crates/dowiz-hub/src/allergens.rs`); kcal/protein/fat/carbs derived from the recipe (`workers/api/src/recipe.rs`) | — |
| HACCP / raw-fish record | not found | — | — | — | — | — | lot fields exist; a freezing-treatment field was proposed in 09-26 §2.1 and is not built | small, row P13 |
| Languages | EN/ES/KZ/RU/UK, **no sq** | unverified | EN/DE/FR/ES/HE/ZH (3rd-party) | EN | — | — | **sq/en/uk/ru** everywhere | — |
| Price | Starter 540 ₴/mo **has no warehouse**; Mini 1,080, Business 1,530 (photo invoices), Pro 2,160 ₴/mo (annual rates) [P1] | £49-99 per till per month [S3] | $249-449/mo [M1] | xtraCHEF "from $149" (3rd-party) | inventory in Essential $189 [L3] | add-on $99/location [Q2] | included | — |

**Sources for §1.2:**
- [P1] https://joinposter.com/ua/pricing
- [P2] https://knowledge-base.joinposter.com/en/what-is-a-supply-import-and-what-are-the-ways-to-do-it
- [P3] https://knowledge-base.joinposter.com/en/how-to-specify-gross-and-net-weight-manually
- [P4] https://knowledge-base.joinposter.com/en/how-to-add-preparations ; https://knowledge-base.joinposter.com/en/what-is-manufacture-and-how-to-use-it ; https://knowledge-base.joinposter.com/en/what-are-butcheries-and-how-to-work-with-them
- [P5] https://knowledge-base.joinposter.com/en/how-the-cost-is-calculated-in-poster
- [P6] https://knowledge-base.joinposter.com/en/when-to-use-multiple-storage-locations-and-how-to-work-with-them ; https://knowledge-base.joinposter.com/en/how-to-make-a-transfer-between-storage-locations
- [P7] https://knowledge-base.joinposter.com/en/how-to-configure-stock-deductions
- [P8] https://knowledge-base.joinposter.com/en/manage-write-off-reasons
- [P9] https://knowledge-base.joinposter.com/en/how-to-run-an-inventory-check ; https://knowledge-base.joinposter.com/en/how-to-run-an-inventory-check-using-a-barcode-scanner
- [P10] https://knowledge-base.joinposter.com/en/how-to-read-the-ingredients-movements-report
- [P11] https://knowledge-base.joinposter.com/en/how-to-prepare-a-month-end-report
- [P12] https://dev.ua/en/news/postie-ai-asistant-1747215140 (3rd-party)
- [P13] https://knowledge-base.joinposter.com/en/how-to-set-up-low-stock-alert
- [P14] https://knowledge-base.joinposter.com/en/how-to-add-a-supplier
- [S1] https://www.syrve.com/en-gb/blog/how-syrve-works-13-ai-invoice-scanner (2026-06-12) ; https://www.syrve.com/en-gb/solution/back-of-house/inventory-control-software
- [S2] https://syrve.com/en-ae/solution/back-of-house/restaurant-forecasting-software
- [S3] https://www.syrve.com/en-gb/pricing
- iiko 8.3 auto-ordering (3rd-party): https://vc.ru/585788-vyshla-novaya-versiya-iiko-83
- [M1] https://www.marketman.com/pricing
- [M2] https://MarketMan.com/platform/restaurant-inventory-management-software
- [M3] https://www.marketman.com/platform/restaurant-purchasing-software-and-order-management (snippet)
- [T1] https://support.toasttab.com/en/article/xtraCHEF-by-Toast (updated 2026-07-07)
- [T2] https://support.toasttab.com/en/article/Toast-IQ-Inventory-FAQ (updated 2026-09-27)
- [T3] https://support.toasttab.com/en/article/86-an-Item
- [T4] pos.toasttab.com inventory pages (snippet)
- [T5] https://support.toasttab.com/en/article/xtraCHEF-Get-Started-With-Actual-vs-Theoretical-Analysis-Reports (updated 2026-01-29)
- [L1] https://k-series-support.lightspeedhq.com/hc/en-us/articles/4407517428891
- [L2] https://k-series-support.lightspeedhq.com/hc/articles/20537741099419
- [L3] https://www.lightspeedhq.com/pos/restaurant/pricing/
- [Q1] https://squareup.com/help/us/en/article/8629-beta-track-ingredient-costs-with-square-recipes (snippet)
- [Q2] https://squareup.com/gb/en/press/square-restaurant-inventory-marketman (snippet)
- [Q3] https://squareup.com/us/en/point-of-sale/features/inventory-management (snippet)
- [AL1] https://invopop.com/coverage/albania.md ; https://www.cleartax.com/al/albania-e-invoicing (secondary). Whether a BUYER can pull received e-invoices by API from the CIS is **unverified**.

### 1.3 Analytics: feature by feature

| Capability | Toast | Square | Lightspeed | Poster | Syrve / iiko | **dowiz today** | Gap |
|---|---|---|---|---|---|---|---|
| Sales by day, hour and dish | Sales Summary by date, weekday, time of day and daypart; CSV/XLS export [T6] | dashboard (snippet) | Item Popularity, Scorecard [L4] | by weekday and hour [P15] | OLAP, "200+ metrics" (snippet) [S4] | **7 or 30 days**, by day, by hour (24 bars), top 8 dishes, channel and fulfilment (`services/analytics/fold.rs:36-54`, `public/admin/more.js:225-240`) | windows, comparison, weekday×hour, export |
| Compare with a previous period | Toast Now: vs last week or last year (snippet) | — | — | — | — | **none** | **yes**, row P2 |
| Food cost %, margin per dish | through xtraCHEF | through MarketMan | — | P&L | yes | **per dish and per day, stamped at the cost when sold**; "uncosted" and "unmodelled" counted honestly (`kitchen/report.rs:37-69, 120-128`) | — (dowiz is stronger than Toast native) |
| Menu engineering (profit × popularity) | product mix only; no classic matrix found | not found | **Magic Menu Quadrant = popularity × RETENTION, not profit** [L5] | **ABC analysis** (revenue/profit/qty) [P16] | ABC/XYZ [S4] | **none** (no quadrant in the tree; grep for `plowhorse`/`quadrant` hits only unrelated files) | **yes**, row P3. **None of the six vendors documents the classic Kasavana–Smith matrix.** |
| Waste, losses, yields | xtraCHEF | MarketMan | — | write-off export | — | waste by reason and value, clean and cook loss in grams, measured vs default yield, supplier price trend (`kitchen/report.rs:105-118`) | — |
| Forecast | projected sales to scheduling partners; "30%-40% discrepancy is normal" [T7] | MarketMan "forecasting ingredient needs" [Q2] | — | Postie (3rd-party) | AI forecast + purchase plan [S2] | **none** | **yes**, row P6 |
| Benchmark vs peers | **Toast Benchmarking**: peer groups by cuisine, service and price band; US only [T8] | — | Benchmarks and Trends (US, 2024, headline only) | — | — | none, and **not honestly possible**: 2 venues | **cannot match** (§5) |
| Natural-language AI | **Toast IQ**: questions, confirmed actions, "For you" feed; US, limited in UK/IE/CA; data goes to a 3rd-party LLM [T9] | **Square AI**, US open beta 2025-06-03 [Q4] | **Lightspeed AI**, 2026-01-09 [L6] | Postie AI [P15] | not found | **assistant on the venue's own endpoint**: redacted facts, OpenAI-compatible, off by default (`workers/api/src/assist.rs:1-15, 42-60, 91-97`); graph retrieval over dishes, ingredients and orders (`services/engagement/assist.rs:28-66`); **its facts carry live orders and the graph, not the numbers of the analytics panes** (`assist.rs:130-161`) | **yes**, row P9 |
| Anomaly alerts | "For you" feed of trends [T9] | — | — | — | — | exception alerts (voids, comps, till over/short) to Telegram (`exceptions/alert.rs`, per the 09-26 TG report) | sales/stock anomaly digest, row P4 |
| Customers | — | — | retention quadrant | "Who are my regulars?" [P15] | — | masked list, reveal with a written reason and a log, CSV (`public/admin/more.js:242-275`) | — |
| Mobile | Toast Now app | phone dashboard | — | Poster Boss | Syrve Dashboard | **the console is a phone PWA** | — |

**Sources for §1.3:**
- [T6] https://support.toasttab.com/en/article/Sales-Summary-Report (updated 2026-05-07)
- [T7] https://support.getsling.com/en/articles/8614345-projected-sales-forecasting-powered-by-toast (3rd-party)
- [T8] https://support.toasttab.com/article/Toast-Benchmarking-Overview (updated 2026-03-25)
- [T9] https://support.toasttab.com/en/article/Toast-IQ-Overview (updated 2026-09-22)
- [Q4] https://squareup.com/us/en/press/square-ai-open-beta (snippet)
- [L4] https://k-series-support.lightspeedhq.com/hc/en-us/articles/18235324645531
- [L5] https://k-series-support.lightspeedhq.com/hc/en-us/articles/7339064083739-Magic-Menu-Quadrant
- [L6] https://www.lightspeedhq.com/news/lightspeed-commerce-launches-lightspeed-ai-a-new-ai-powered-intelligence-layer-for-retail-and-hospitality/
- [P15] https://joinposter.com/en/tour/analytics
- [P16] https://knowledge-base.joinposter.com/en/how-abc-analysis-works-in-poster
- [S4] https://www.syrve.com/en-ae/solution/above-store/restaurant-analytics-software

### 1.4 What would make dowiz CLEARLY better, not just at parity

The target user is a small Albanian sushi/delivery venue, run from a phone, in sq/en/uk/ru.

1. **Data in, in minutes.** Poster charges for photo invoices on its Business plan. dowiz can offer five ways in:
   - a sushi starter pack: about 40 supplies with sensible yields, and recipe skeletons matched to the menu's dish names;
   - a photo of the invoice;
   - **the Albanian e-invoice the supplier is already legally obliged to issue**: structured UBL with a NIVF [AL1];
   - a count by voice;
   - a count the kitchen does on its own login.

   Nobody in §1.2 can read Albanian e-invoices, because nobody in §1.2 is in Albania.
2. **The refusal at checkout.** Poster deducts after the sale. dowiz refuses the order that cannot be made (`stock.rs:15-19`). For delivery, that is the difference between a refund and a happy customer. Keep it, and make it visible as lost-sales numbers (row P8).
3. **Raw-fish compliance as a by-product.**
   - Lots, FEFO and expiry already exist.
   - Albania requires an "L" lot code on food labels from 2026-01-01. Source: the 09-26 report §2.1, https://albaniandailynews.com/news/new-food-security-measures-introduced-denaj
   - Reg. 853/2004 requires a freezing treatment for fish eaten raw.
   - Adding the freezing record to the lot gives a HACCP export no POS here has (row P13).
4. **Menu engineering done right and explained in the owner's language.** It is the classic profit × popularity matrix on the **stamped** cost, which no vendor in §1.3 documents. Each quadrant gets one actionable sentence, from 4-language templates rather than a model.
5. **Honest AvT ("unexplained loss") per count window, valued in lek.** The digest goes to the kitchen's Telegram group, not to an email nobody reads (Poster's alerts are email only [P13]).
6. **A forecast that says how wrong it was.** Every prep and order suggestion shows its own back-tested error over the last 4 weeks. Toast's partner says 30-40 % error is "normal" [T7] and does not show the venue its own.
7. **Every number is replayable.** The fold and the stamps are the audit, and nothing is a stored counter. No vendor can show "this figure came from these records". dowiz can, and that is the honest answer to "better analytics than Toast": not more charts, but numbers the owner can trust and trace.

---

## 2. Free models

### 2.1 Where a model can run (the four places)

| Place | Facts | Limits for us |
|---|---|---|
| **(a) Cloudflare Workers AI** | **10,000 neurons/day free**, reset 00:00 UTC; $0.011 per 1,000 beyond that on Workers Paid only (https://developers.cloudflare.com/workers-ai/platform/pricing/, cited). Available on Free per a 3rd-party summary (https://costbench.com/software/llm-api-providers/cloudflare-workers-ai/free-plan/); the pricing page says "free allocation for anyone". Rate limits: ASR and translation 720/min, text generation 300/min, embeddings 3,000/min (https://developers.cloudflare.com/workers-ai/platform/limits/). | **The allowance is per ACCOUNT, shared by every venue**, and stays FREE per the 2026-10-01 decision, so each venue needs a daily cap. Calling the binding costs the Worker almost no CPU, because inference happens on Cloudflare's GPUs and the 10 ms is CPU, not wall time (GUESS for our exact overhead; measure it in the row's probe). **No `[ai]` binding exists today** (`workers/api/wrangler.toml`). |
| **(b) Owner's browser** (transformers.js / ONNX Runtime Web) | WebGPU has been on by default in Chrome 121+ on Android 12+ with Qualcomm/ARM GPUs (https://developer.chrome.com/blog/new-in-webgpu-121?hl=en, cited). Chrome 146 adds an ES 3.1 "compatibility mode" (https://app.cinevva.com/news/2026-03-10-chrome-146-webgpu-compatibility, secondary). WASM vs WebGPU on embeddings: 507 vs 46 ms on a desktop (https://huggingface.co/spaces/Xenova/webgpu-embedding-benchmark/discussions/1). | Assume no WebGPU on cheap phones. On WASM, only models under about 100 M parameters are practical (GUESS). It costs nothing and data stays on the device. |
| **(c) Offline batch on the operator box** | ARM/proot, about 1.6 GB free RAM, no GPU (CLAUDE.md, freellm note). | The box is not a production service. It suits **evaluations** and one-off back-fills, not a feature a venue depends on. |
| **(d) The venue's own endpoint** | Already built: `ai.enabled` + `ai.endpoint` (https only) + `ai.model`, OpenAI chat shape, facts always redacted (`workers/api/src/assist.rs:84-97, 42-60`). | Quality is whatever the venue chose. It is the only route today to strong Albanian free text. |

Neuron cost per model, from the pricing page (cited), with what 10k neurons/day buys (my arithmetic):

| Model | Neuron cost | 10k neurons/day buys |
|---|---|---|
| llama-3.1-8b-instruct-fp8 | 34,868 per M output tokens | ≈ 290k output tokens |
| llama-3.2-3b-instruct | 30,475 per M output tokens | ≈ 330k output tokens |
| m2m100-1.2b | 31,050 per M tokens | ≈ 320k tokens |
| whisper-large-v3-turbo | 46.63 per audio minute | ≈ 214 min |
| bge-m3 | 1,075 per M tokens | ≈ 9.3M tokens |

The catalogue (https://developers.cloudflare.com/workers-ai/models/, cited) lists:
- **LLMs:** eurollm-9b-it, gemma-4-26b-a4b-it, qwen3-30b-a3b-fp8, mistral-small-3.1-24b, gpt-oss-20b/120b
- **Image-to-text:** llava-1.5-7b
- **Embeddings:** embeddinggemma-300m and qwen3-embedding-0.6b

### 2.2 Licence traps (all cited on the model cards)

The following are **not usable commercially**:

| Model | Licence | Source |
|---|---|---|
| NLLB-200 | CC-BY-NC | https://huggingface.co/facebook/nllb-200-distilled-600M |
| Aya Expanse | CC-BY-NC | https://huggingface.co/CohereLabs/aya-expanse-8b |
| Moirai-2 | CC-BY-NC | https://huggingface.co/Salesforce/moirai-2.0-R-small |
| Qwen2.5-VL-3B | Qwen Research licence | https://huggingface.co/Qwen/Qwen2.5-VL-3B-Instruct/blob/main/LICENSE |
| tabularisai multilingual sentiment | CC-BY-NC | https://huggingface.co/tabularisai/multilingual-sentiment-analysis |

Two more carry conditions:
- **MobileCLIP** uses the apple-amlr licence, and its commercial terms are unclear.
- **Llama 3.2** officially supports **no sq, uk or ru**, and its licence requires a "Built with Llama" notice (https://huggingface.co/meta-llama/Llama-3.2-1B-Instruct).

### 2.3 Use case by use case: one recommendation each, and whether arithmetic wins

**Data volume (GUESS from the live venues):** about 20-60 orders/day, about 165 dishes, most of them selling 0-3 portions a day. A per-dish hourly series is almost all zeros.

| # | Use case | Candidates (licence · size · cited accuracy · where) | **Recommendation** | Does a deterministic method beat the model at one venue's volume? |
|---|---|---|---|---|
| U1 | Demand forecast per dish × hour × weekday | Chronos-Bolt tiny/mini/small/base: Apache-2.0, 9/21/48/205 M, beats statistical baselines on 27 datasets (https://huggingface.co/amazon/chronos-bolt-small). Chronos-2 and chronos-2-small: Apache-2.0, 120 M / 28 M, GIFT-Eval MASE win rate 79.8 % vs seasonal naive 10.1 % (https://huggingface.co/amazon/chronos-2 ; https://arxiv.org/pdf/2510.15821, secondary). TimesFM-2.5: Apache-2.0, 200 M (https://huggingface.co/google/timesfm-2.5-200m-pytorch). TTM-r2: Apache-2.0, about 1 M, but **needs 512+ context points** (https://huggingface.co/ibm-granite/granite-timeseries-ttm-r2). Lag-Llama: Apache-2.0, 2.45 M. Browser: community Chronos-Bolt ONNX exports are **encoder-only**, not forecasters (https://huggingface.co/light-curve/chronos-bolt-tiny). | **Deterministic, in the Rust fold, ships now.** For the venue × hour-band total: the median of the last 4-8 same weekdays. For each dish: its share of the venue total, with **TSB** (Teunter–Syntetos–Babai) smoothing for intermittent dishes. A back-test (MASE vs seasonal-naive) is shown to the owner. **Chronos-2-small as an offline challenger on the box (c)** only, once a venue has ≥ 12 weeks of rolled-up history, and it is promoted only if it beats the baseline's MASE by ≥ 10 % on two venues. | **Yes, today.** No study tested these models on few-week, per-dish-hourly, intermittent counts. A hierarchical-Bayesian TSB beats Croston/ARIMA on intermittent retail data (https://arxiv.org/abs/2511.12749v1). Foundation models were "at par" for demand forecasting (https://mlanthology.org/neuripsw/2024/puvvada2024neuripsw-critical). With about 30 days of hot history and zeros everywhere, the model has nothing to learn from that the median does not already use. |
| U2 | Reorder points and par levels | none needed | **Arithmetic.** par = forecast use over (lead time + order cycle) + safety stock; safety = z · σ(daily use) · √lead time, with z = 1.65 by default (GUESS default, owner-tunable). order = par − available − on order. This replaces the fixed "below 3 days buy 7" in `kitchen/report.rs:16-35`. | **Yes.** It is a formula over the forecast; there is nothing for a model to do. |
| U3 | Anomaly / shrinkage on the ledger | none needed | **Arithmetic.** Per count window: unexplained = (opening + received − closing) − theoretical − recorded waste, in grams and lek and as ‰ of revenue. Flag above 3 % of revenue (the industry threshold quoted in the 09-26 report from https://get.apicbase.com/food-cost-variance/), or the item's drift beyond 2σ of its own past windows. | **Yes.** A 2-venue platform has no labelled anomalies to train on, and the arithmetic is explainable. |
| U4 | Waste prediction | none needed | **Arithmetic over lots.** For each open lot: forecast use before its expiry vs the quantity in the lot. The surplus is "will expire unused" → "use first / put on special" in the daily `stock.expiring` digest. | **Yes.** |
| U5 | Menu engineering | Workers AI or the venue endpoint for prose | **Arithmetic** (Kasavana–Smith: popularity vs 70 % of the average share, margin vs the weighted average contribution margin, on stamped cost) plus **one template sentence per quadrant** in 4 languages. The LLM explanation is opt-in through (d). | **Yes.** The templates are reviewed once by a native speaker, and a 1-4 B model's Albanian is unverified (U6). |
| U6 | Natural-language Q&A over the venue's data, sq/en/uk/ru | Qwen3 0.6-4 B: Apache-2.0, lists **Albanian, Ukrainian, Russian** (https://qwenlm.github.io/blog/qwen3/). Gemma 4 E2B/E4B: Apache-2.0, "140+ languages", per-language Albanian not confirmed (https://ai.google.dev/gemma/docs/core). EuroLLM-9B: Apache-2.0, on Workers AI, ru/uk but **no Albanian** (https://huggingface.co/utter-project/EuroLLM-9B-Instruct). Phi-4-mini: MIT, ru/uk, **no sq**. TildeOpen-30B covers sq, but at 30 B it is a reference point only. **No sub-5 B model publishes an Albanian score.** | **Two layers.** (1) A **closed set of about 25 intents** answered deterministically in all 4 languages: "how much did X sell", "food cost last week", "what will run out", "worst margin dish". Matching is by keyword lexicon, and the answer is a pre-computed number with a link to the pane. (2) Anything else goes to **(d) the venue's endpoint**, as now. The analytics numbers are added to its FACTS. **Workers AI (a) is an operator-switchable default**, `@cf/qwen/qwen3-30b-a3b-fp8` (GUESS pick: it covers sq per Qwen3's language list), with a per-venue daily cap. | **For the top 25 questions, yes.** Deterministic answers are exact and instant, and identical in Albanian. The model earns its place only on the long tail, and only through a model the operator has tested on 20 Albanian questions. |
| U7 | Invoice / receipt OCR → goods receipt | Tesseract `sqi`: Albanian traineddata packaged (https://pkgs.alpinelinux.org/package/v3.21/community/aarch64/tesseract-ocr-data-sqi); tesseract.js runs in the browser (support for sqi there is GUESS). PaddleOCR PP-OCRv5 latin: **lists Albanian**, about 2 M recogniser (https://www.paddleocr.ai/latest/en/version3.x/algorithm/PP-OCRv5/PP-OCRv5_multi_languages.html). Florence-2: MIT, 0.23/0.77 B, OCR task, transformers.js. Qwen2-VL-2B: Apache-2.0, DocVQA 90.1. SmolVLM2: Apache-2.0, DocVQA 80. Granite-Docling: English only. llava-1.5-7b on Workers AI. **None of the VLMs is verified on ë/ç.** | **First: the Albanian e-invoice (row P11).** It is structured, so it needs no OCR, if the buyer can retrieve it (operator ask Q3). **Second: photo → tesseract.js `sqi` in the owner's browser → a deterministic line parser** (qty, unit, price, total, numbers in `1.200,00` form reusing `dowiz_hub::import` price parsing) → **matching to the venue's supplies by learned aliases** (supplier text → supply id, remembered after the first confirmation) → the owner confirms each line. A VLM through (d) is an optional "read it for me" button. | **Mostly yes.** The hard part is the match and the parse, not the pixels, and the match is a table the venue teaches once per supplier. OCR is worth it; a large VLM is not (GUESS: verify on 10 real Durrës invoices, operator ask Q4). |
| U8 | Review / feedback sentiment | cardiffnlp xlm-roberta sentiment: licence not shown, trained on 8 languages without sq. tabularisai: NC. Workers AI distilbert-sst-2: English only. | **Deterministic topic tags**, not sentiment: a 4-language lexicon (cold, late, missing, spicy, portion, packaging...), aggregated **per dish and per week only**. | **Yes.** The `no-scoring` rule (`tools/gates/no-scoring.sh`; `services/orders/feedback.rs:22`, "NO STARS, NO SCORE, NOT ON ANYONE") forbids a per-order score that could be joined to a courier. Topics per dish are actionable; a sentiment number is not. |
| U9 | Dish photo quality and tagging | SigLIP2-base: Apache-2.0, 0.4 B, multilingual text tower (https://huggingface.co/google/siglip2-base-patch16-224). nateraw/food: Apache-2.0, Food-101 89 %, but **"sushi" is a single class** (https://huggingface.co/nateraw/food). Laplacian-variance blur (https://pyimagesearch.com/2015/09/07/blur-detection-with-opencv/). | **Quality: arithmetic in the browser on upload** (Laplacian variance, clipped-histogram exposure, resolution, aspect). **Tagging: no model.** The dish already has a category, a recipe and allergens; tags derive from the recipe (salmon → "salmon", raw fish → "raw"). | **Yes.** Recipe-derived tags are exact, and a 0.4 B CLIP on a phone is slow and cannot tell nigiri from maki reliably (GUESS). |
| U10 | Menu translation into 4 languages | M2M100 418M/1.2B: **MIT**, covers sq/uk/ru (https://huggingface.co/facebook/m2m100_418M); **on Workers AI** as m2m100-1.2b. Opus-MT en-sq: Apache-2.0, Tatoeba BLEU 46.5 (https://huggingface.co/Helsinki-NLP/opus-mt-en-sq), browser-light. MADLAD-400-3B: Apache-2.0, too large for (b) and (c). NLLB: NC, excluded. FLORES eng→sqi chrF++ was **not extracted** (unverified). | **Workers AI m2m100-1.2b (a) as a DRAFT** that the owner sees side by side and edits. It is never published unreviewed, and the dish name is kept untranslated by default (a sushi name is a proper noun). One call per edited text, cached in `content_i18n`. | **No.** Translation is a real model win: there is no deterministic alternative except a human. Albanian quality is unverified, so the draft is reviewed (operator ask Q5). |
| U11 | Voice ordering and voice stock counts | Whisper on Workers AI: 41-47 neurons/min. **Albanian WER: no FLEURS figure** (https://models.handy.computer/languages/sq). uk FLEURS large-v3 6.3 / turbo 7.3; ru 5.0 / 5.9 (https://models.handy.computer/languages/uk ; …/ru). Albanian fine-tunes: whisper-large-v2 WER 25.2 on Kaggle-Albanian (https://huggingface.co/rishabhjain16/whisper_l2_to_kaggle_sq, secondary). Vosk: ru/uk small, **no Albanian** (https://alphacephei.com/vosk/models). Moonshine tiny-uk 27 M, FLEURS 18.3. Today: the browser's Web Speech API (`public/lib/voice.js:21`), which on Chrome sends audio to Google's servers (GUESS from Chrome's documented behaviour; the offline-voice research lane owns this). | **Keep the deterministic grammar** (`crates/dowiz-hub/src/voice.rs:1-27`). **Add a stock-count verb** ("salmon two kilo three hundred" → count proposal, confirmed by tap). Speech-to-text stays the existing path; Workers AI whisper-turbo is a measured option *after* 20 real Albanian kitchen recordings are scored. | **The grammar beats a model** for what was meant (the module's own argument). For what was said there is no deterministic alternative; but for Albanian there is **no measured number at all**, so it cannot be promised. |

**Summary of where models earn their place:** translation drafts (U10), invoice OCR (U7, small OCR rather than a VLM), and the long tail of natural-language questions (U6, through the venue's endpoint or an opt-in Workers AI default). Everything else is arithmetic, and arithmetic that the owner can trace is the actual edge over Toast.

---

## 3. Plan: lane-sized rows, polish first, ordered by owner value ÷ effort

**Effort scale:** XS ≤ ½ day, S ≤ 2 days, M ≤ 1 week, L > 1 week (GUESS, lane-days on Opus).

**Every row has:**
- **Contract:** `tools/live-proof/contracts/<id>.json` with `{id, version, producer, consumer, request_schema, response_schema, external_api_version|null, description}`. The JSON Schema is draft 2020-12. W-LIVE owns the harness (`tools/live-proof/**`), which is **not in the tree at 34aa2bc0**, so these rows depend on W-LIVE landing first.
- **Probe:** `tools/live-proof/probes/<id>.mjs` against `https://qa-durres.dowiz.org`. It uses the QA owner and kitchen logins in `/root/.dowiz_owner`, never a real venue. It validates the live response against the schema and cleans up through the UI or API.
- **UI in sq/en/uk/ru:** every new key lands in `public/admin/i18n.js` (sq/en/uk), `i18n-ru.js`, or the screen's own `*-i18n.js`, with a placeholder and an explanation per field (W-APPLE rule).
- **Rules:** the no-backend-without-ui rule; the one-image gate; one object turn per request; 10 ms CPU, so folds stay O(window) on the DO; the coverage ratchet.

**[OP]** marks a row that needs operator input.

### Phase A: polish what exists (high value, small effort)

**P1. Shelf in an hour: the sushi starter pack + menu-matched recipe skeletons** (value 5, effort S)

- **What it does.**
  - A "Start stock" sheet offers a curated pack of about 40 supplies (rice, nori, salmon, tuna, avocado, cream cheese, soy, wasabi, ginger, sesame, boxes, chopsticks...). Each has a unit, a `cleanPm`/`cookPm` default, `kind` and category, and names in 4 languages.
  - It proposes recipe skeletons by matching dish names to pack lines with a deterministic keyword table, e.g. "Philadelphia" → salmon, cream cheese, rice, nori. Grams are left empty, to be confirmed by the owner.
  - Everything goes through the existing writers: `POST /api/owner/supplies/import` with the CSV built client-side, and `recipes/import` with `?apply=1` after the preview (`services/catalogue/import/bulk.rs:33-41`).
- **Files:**
  - new `public/admin/stock-start.js`, `stock-start-i18n.js`, `stock-start-pack.js` (data);
  - the entry button in `public/admin/ingredients.js`;
  - the matcher, pure and tested: `stock-start-logic.js` + `.test.mjs`.
- **Acceptance:**
  - on an empty venue, 3 taps produce ≥ 30 supplies and ≥ 20 recipe drafts;
  - the preview shows every unmatched dish;
  - nothing is written before Apply;
  - a re-run is idempotent (same ids).
- **LIVE probe** (`stock-start.mjs` on qa-durres):
  1. open with a stocktake to zero (the trap in `dowiz-stock-ledger-works-but-is-off`);
  2. apply the pack;
  3. `GET /api/owner/stock` shows the pack ids;
  4. a dish with a confirmed skeleton shows `weightG` and `cost` derived on the **storefront** menu read with `?fresh=1`;
  5. clean up with retire + sweep.
- **Contract:** `stock.start.v1`. Request: CSV bodies, the existing import schema. Response: `{applied, warnings[], supplies[], recipes[]}`.
- **[OP]** the operator or the venue chef corrects the pack's default grams and loss percentages once (Q1).

**P2. Analytics that look back and compare: a durable daily sales cube + the 30-day defect** (value 5, effort M)

- **What it does:**
  - A per-venue **daily cube**: for each local day, `{orders, revenue, by_hour[24], by_channel, dishes: {id: [qty, revenue, stamped_cogs]}}`.
  - It is written at rotation (`hubstore.rs:950-980`) for each day leaving the hot log, and back-filled once from archives.
  - It is held in the venue DO as a pointer-free, checkpoint-style record. It is **a cache with a verifier, not a source**, the same law as `stock/checkpoint.rs:18-22`: a `verify` route refolds a sample day from its archive and compares byte for byte.
  - The owner pane gains:
    - windows of 7 / 30 / 90 / 365 days;
    - **the same period before it** and the **same weekday over the last 4 weeks**, as deltas;
    - a weekday × hour heat grid;
    - CSV export.
  - The kitchen fold reads the cube for days outside the hot log, which fixes the probable 31-62-day defect.
- **Tension, stated:** `fold.rs:3-5` says "NO ANALYTICS STORE". The cube keeps that law in spirit, because it is derived, verified and rebuildable. It is needed because a 10 ms request cannot read 60 R2 archives. The operator should know this (Q2).
- **Files:**
  - `services/analytics/{fold.rs, handler.rs, cube.rs (new, pure), cube/tests.rs}`;
  - `hubdo/reads.rs` (fold_analytics, fold_kitchen);
  - `hubstore.rs` (rotation writes the cube);
  - `public/admin/more.js` (openAnalytics);
  - i18n.
- **Acceptance:**
  - RED→GREEN test: a 62-day kitchen window over a fixture whose orders are half archived shows sales on day 40 (RED today if the defect is real);
  - `cube(day) == fold(archive(day))` for every fixture day;
  - CPU: one request folds ≤ 365 cube rows. Log the measured µs on a 365-day fixture in `docs/measurements`.
- **LIVE probe** (`analytics-cube.mjs`):
  1. place 3 orders on qa-durres;
  2. read `/api/owner/analytics?days=7`;
  3. assert `compare.prev` and `byWeekdayHour` exist and validate;
  4. call the verifier route on yesterday and expect `equal: true`.

  Rotation itself can only be proven live after 30 days, so the probe also calls a QA-only "rotate now" switch, guarded to the QA hub.
- **Contract:** `analytics.owner.v2`. It adds `compare{prev, weekdayAvg}`, `byWeekdayHour[7][24]` and `days ∈ {7,30,90,365}`. It is versioned because the existing shape grows (v1 fields kept).

**P3. Menu engineering matrix** (value 5, effort S; depends on P2 for windows > 30 days, works on 30 days without it)

- **What it does:** the kitchen numbers gain a `menu` block.
  - **Popularity:** a dish's share of portions, compared with 70 % of 1/N.
  - **Profit:** its stamped contribution margin per portion, compared with the weighted average.
  - **Quadrant:** star / plowhorse / puzzle / dog.
  - **Action:** one action key per quadrant, e.g. a puzzle gets "move it up the menu, rename, add a photo"; a plowhorse gets "raise price by X, or cut the portion cost of Y", where Y is the most expensive recipe line.
  - Dishes without a recipe go in "unknown margin" and are never forced into a quadrant.
- **Files:**
  - `services/analytics/kitchen/{menu.rs (new, pure), menu/tests.rs}`, `kitchen.rs`;
  - `public/admin/kitchen-view.js` (2×2 grid of dish chips, tap → dish sheet);
  - `kitchen-i18n.js` (4 × 4 template sentences, reviewed by a native speaker for sq and ru).
- **Acceptance:** a fixture with known prices, costs and sales lands each dish in the textbook quadrant, and an uncosted dish is "unknown".
- **LIVE probe** (`menu-matrix.mjs`): create 4 QA dishes with recipes and costs, place orders shaped to put one in each quadrant, read `/api/owner/analytics/kitchen?days=1`, assert the quadrants, clean up.
- **Contract:** `analytics.kitchen.v2` (adds `menu[]`).

**P4. Unexplained loss per count window (honest AvT) + the weekly Telegram digest** (value 4, effort S)

- **What it does:**
  - Between two stocktake sessions, per supply: `actual = opening + received − closing`; `theoretical` = sales draws; `explained` = waste + prep loss; `unexplained = actual − theoretical − explained`. Each is shown in base units, in lek at WAC, and as ‰ of the window's revenue.
  - Rows are ordered by lek. Flag a row when the window is > 30 ‰ (3 %) of revenue, or when the item's drift exceeds 2σ of its own past windows.
  - The `stock.digest` Telegram event goes to the groups that chose it, using the existing routing (`stock/tell.rs`). It carries the top 5, in the group's language.
- **Files:**
  - `services/analytics/kitchen/{avt.rs (new), avt/tests.rs}`, `report.rs`;
  - `services/operations/stock/tell.rs` (new kind);
  - `public/admin/kitchen-view.js`, `telegram-view.js` (new checkbox);
  - i18n.
- **Acceptance:** the fixture from the 09-26 report §4 A4 gives known numbers; a window without two counts says "count twice to see this" and shows no zero.
- **LIVE probe** (`avt.mjs`):
  1. count salmon 1000;
  2. receive 500;
  3. sell 2 dishes (2 × 40 g);
  4. waste 20;
  5. count 1300;
  6. assert `unexplained = 1000 + 500 − 1300 − 80 − 20 = 100` g, valued;
  7. trigger the digest to the QA Telegram group and read it back through the Bot API `getUpdates` of the QA bot. This is a real external service, so the contract pins Bot API version 7.x.
- **Contract:** `analytics.avt.v1`, `telegram.stock_digest.v1`.

**P5. Suppliers as cards, par levels and the order list you send from your phone** (value 5, effort M) **[OP]**

- **What it does:**
  - A supplier card: name, phone/WhatsApp, Telegram, delivery weekdays, lead days, order cutoff, language. Each supply gets a default supplier and a pack size.
  - Par uses U2's formula on P6's forecast. Until P6 lands, it uses the average daily use already computed (`kitchen/report.rs:27-35`).
  - "Order list" groups suggested quantities by supplier, rounded up to packs. The owner edits and taps **Share** (`navigator.share`, as in the courier invite of W-URGENT), which sends a text in the supplier's language.
  - Sending writes an `ordered` marker on the stock log, so "on order" subtracts from the next suggestion and the receipt closes it.
  - The legacy fixed hint (`REORDER_BELOW_DAYS = 3`, `report.rs:18`) is replaced.
- **Files:**
  - `crates/dowiz-hub/src/stock/{event.rs (+`Ordered{item,qty,supplier,po}`, never moves the shelf), ledger.rs}`;
  - the checkpoint codec (+ the new field in its fixed order);
  - `services/operations/suppliers.rs` (new);
  - `public/admin/suppliers.js`, `order-list.js`, i18n.
- **Acceptance:**
  - the old-image rule: every log written before folds byte-identical (checkpoint verify);
  - par/order arithmetic is unit-tested;
  - a receipt naming the PO closes the open quantity.
- **LIVE probe** (`order-list.mjs`): create a supplier, set lead 1 day, record use over 3 days, read `/api/owner/stock/order-list`, assert the quantity and rounding, post `ordered`, receive against it, assert "on order" = 0.
- **Contract:** `stock.suppliers.v1`, `stock.order_list.v1`.
- **[OP]** the real suppliers, their delivery days, lead times and pack sizes for a pilot venue (Q6).

**P6. Prep list and forecast (K14), with its own error shown** (value 4, effort M; best after P2)

- **What it does:**
  - **Forecast:** per weekday × hour band (open–14, 14–18, 18–close), the median of the last 4-8 same weekdays for the venue total, and each dish's TSB share. Expand the dish forecast through recipes into ПФ and raw, using the existing `prep::expand` (`crates/dowiz-hub/src/prep.rs:168`).
  - **Prep list for today:** forecast ПФ use − ПФ on hand (`stock/basket.rs:77`, `ready_micro`) + bookings × average basket. This is K14 of the kitchen report, `2026-09-26-kitchen-role.md:464`.
  - **Error:** MASE of the last 4 weeks vs seasonal-naive, shown as "usually off by ±n portions".
- **Files:**
  - `crates/dowiz-hub/src/forecast.rs` (new, pure: median, TSB, MASE) + tests;
  - `services/analytics/forecast.rs`;
  - `public/kitchen/` prep screen (existing kitchen hub) + `public/admin/kitchen-view.js`;
  - i18n.
- **Acceptance:** on synthetic series (weekly seasonality + noise + intermittent dishes), MASE is ≤ seasonal-naive. A dish with < 3 weeks of history says "learning" instead of a number.
- **LIVE probe** (`prep-forecast.mjs`): it needs history, so it uses QA back-fill. It writes a 6-week synthetic order history to qa-durres through the real order route (paced under the Free daily caps: ≤ 300 orders, GUESS budget; check against the DO request cap before running). It then reads `/api/kitchen/prep?day=`, asserts the shape and that the forecast equals the median computed in the probe from what it placed, and checks that the error field exists.
- **Contract:** `kitchen.prep_forecast.v1`.

**P7. Expiry-aware waste prediction in the daily digest** (value 3, effort XS; after P6)

- **What it does:** for each open lot, compare the forecast use until expiry with the quantity left. A surplus goes into the existing `stock.expiring` event as "won't be used in time: N g → use first / special".
- **Files:** `services/operations/stock/tell.rs`, `stock/lots.rs` reader (no new records), i18n.
- **Acceptance:** a lot of 1 kg expiring tomorrow with a forecast use of 300 g reports 700 g.
- **LIVE probe:** receive a QA lot with expiry tomorrow, trigger the daily digest on the QA hub, read it back from the QA Telegram group.
- **Contract:** `telegram.stock_expiring.v2`.

**P8. Lost sales and extras that draw stock** (value 3, effort S)

- **What it does:**
  - **A13:** every `OutOfStock` refusal at checkout appends a non-shelf `refused{item, dish, at}` record. It is fold-neutral, rate-limited to 1 per item per 10 minutes, and carries no personal data. It is reported as "orders lost to stock-outs" with lek.
  - **R13:** a modifier option gains an optional `bom`, and the chosen options' lines join the reservation.
- **Files:**
  - `storefront.rs` (placement refusal path);
  - `crates/dowiz-hub/src/{modifiers.rs, stock/event.rs}`;
  - `services/analytics/kitchen/report.rs`;
  - `public/admin/menu-edit.js` (option recipe line), i18n.
- **Acceptance:** "extra salmon" reserves 20 g more; a refused basket yields exactly one `refused` row; old images fold the same.
- **LIVE probe:** count a QA supply to 30 g, order a dish needing 40 g, expect 409 + a `refused` row in `/api/owner/analytics/kitchen`; then add an option with 20 g of the supply and assert the reservation.
- **Contract:** `stock.refused.v1`, `catalog.modifier_bom.v1`.

### Phase B: the AI layer, only where it wins

**P9. The assistant reads the numbers: closed intents in 4 languages + analytics FACTS** (value 4, effort M) **[OP]**

- **What it does:**
  - About 25 intents (sales of X, food cost, margin, what runs out, prep for tomorrow, unexplained loss, best/worst dish, compare with last week) are answered **without any model**:
    - a keyword lexicon in sq/en/uk/ru selects the intent;
    - the number comes from the same folds as P2–P6;
    - the answer is a template sentence plus a link that opens the pane.
  - Unmatched questions go to the existing endpoint path (`assist.rs`). The FACTS gain a compact `numbers` block (7-day totals, top/bottom dishes, low stock, AvT top 3), redacted as today.
  - **Optional:** a Workers AI binding as the "no endpoint configured" default, `@cf/qwen/qwen3-30b-a3b-fp8`, a GUESS pick. It is capped per venue (e.g. 300 neurons/day, GUESS; 10k ÷ ~30 venues) and refuses loudly when the cap is spent.
- **Files:**
  - `services/engagement/{assist.rs, assist/intents.rs (new, pure), assist/intents/tests.rs}`;
  - `workers/api/wrangler.toml` (`[ai] binding = "AI"`, only if the operator approves);
  - `public/admin/assistant*.js`, i18n.
- **Acceptance:**
  - 25 intents × 4 languages = 100 fixture questions route to the right intent;
  - the deterministic answer equals the pane's number;
  - the endpoint path is unchanged.
- **LIVE probe** (`assist-intents.mjs`): ask 8 questions (2 per language) on qa-durres and compare with `/api/owner/analytics*`. If the AI binding is approved: one long-tail question goes to Workers AI, the response is validated, and the neuron counter moves.
- **Contract:** `assist.owner.v2`, with `external_api_version: "workers-ai/@cf/qwen/qwen3-30b-a3b-fp8"`.
- **[OP]**:
  - (a) whether to add a Workers AI binding to the FREE account, given that the 10k neurons/day is shared by all venues;
  - (b) 20 real Albanian owner questions, and a native speaker to grade the answers (Q7).

**P10. Invoice photo → goods receipt** (value 4, effort M) **[OP]**

- **What it does:**
  - The delivery sheet gains "From a photo". tesseract.js with `sqi+eng` runs in the browser and is lazy-loaded only on that tap (size: GUESS, a few MB). Verify that tesseract.js loads `sqi` traineddata, and pin the tessdata file in `public/lib/ocr/` (no CDN at runtime, to keep the CSP). A deterministic parser turns lines into `{text, qty, unit, unit_price, total}`, reusing the hub's number parsing.
  - **Supplier aliases**: a supplier's line text maps to a supply id, remembered on confirmation and stored on the supplier card from P5.
  - The owner confirms each line, and the existing multi-line receipt writes them.
  - An optional "read with my AI" button sends the image to the venue's endpoint (d), if it is a vision model.
- **Files:**
  - `public/admin/receipt-photo.js` + `-logic.js` + tests;
  - `public/lib/ocr/*` (vendored, licence file alongside);
  - `services/operations/suppliers.rs` (aliases);
  - i18n.
- **Acceptance:**
  - a fixture set of 10 real invoice photos (operator-supplied) reaches ≥ 80 % of lines correct before confirmation (GUESS target);
  - the second invoice from the same supplier needs ≥ 50 % fewer edits.
- **LIVE probe** (`receipt-photo.mjs`): a Playwright/Chrome-headless drive of the real console on qa-durres uploads a fixture invoice, confirms, and reads `GET /api/owner/stock` for the priced receipt with supplier and doc.
- **Contract:** `stock.receipt_from_photo.v1`. The OCR is client-side; the contract covers the receipt request built from it.
- **[OP]** 10 photographed supplier invoices from Durrës, with personal data blacked out (Q4).

**P11. Albanian e-invoice import (purchase side)** (value 5 if possible, effort M) **[OP], research spike first**

- **What it does:** received B2B e-invoices (UBL 2.1 with NIVF) for the venue's NIPT become draft receipts, matched by supplier NIPT and line aliases. The existing `ebills` client is a strict path allow-list (`ebills/client.rs:4-14`), and any new path is added as a typed `Path`.
- **Spike (XS, research only):** can a buyer retrieve received invoices from the CIS or the e-Fatura portal by API with the venue's credentials, or only by manual download? Only the issuer side is documented in the sources found [AL1].
- **Fallback:** the owner uploads the XML or PDF they received by email, and the UBL is parsed deterministically. The parse is exact, with no OCR.
- **LIVE probe:** import a real QA-owned e-invoice. A real e-invoice needs a real NIPT, so this is **NEEDS-KEY** until the operator supplies one.
- **Contract:** `stock.einvoice_import.v1`, `external_api_version: "UBL 2.1 / AL CIS"`.

**P12. Storages and transfers: kitchen / bar / freezer** (value 3, effort L)

- **What it does:**
  - The new event `Moved{item, qty, from, to, by}` keeps one fold per (item, storage).
  - The sale draws from the storage of the station, following Poster's rule [P7] that the default is the storage that last received.
  - The freezer storage is where the raw-fish freezing record (P13) lives.
- **Effort is L** because every fold (ledger, cost, lots, carry, checkpoint codec) gains a storage dimension. The old-image rule maps every old record to the default storage.
- **Files:** `crates/dowiz-hub/src/stock/**`, `services/operations/stock/**`, `public/admin/ingredients*.js`, i18n.
- **LIVE probe:** receive into "freezer", move 500 g to "kitchen", sell, and assert the per-storage levels.
- **Contract:** `stock.storages.v1`.

**P13. Raw-fish freezing record and HACCP export** (value 3, effort S; after or alongside P12)

- **What it does:** `frozen{at, hours, temp_c}` or `supplierTreated` on a lot; a CSV export of lot → orders and order → lots, plus the freezing log.
- **Files:** `stock/meta.rs` (an extra key, old-image safe), `services/operations/stock/view.rs`, `public/admin/ingredients.js`, i18n.
- **LIVE probe:** receive a lot with a freezing record, place an order, download the export, find the order under the lot.
- **Contract:** `stock.haccp_export.v1`.
- **[OP]** does the venue freeze in-house or buy pre-treated (Q8).

**P14. Voice stock count** (value 3, effort S)

- **What it does:** the kitchen grammar (`services/engagement/voice/kitchen.rs:1-10`) gains `count <supply> <quantity>` in 4 languages ("salmon two kilo three hundred", "salmon 2,3 kg"). It produces a signed proposal; one tap adds the line to an open count session. It uses the existing speech path.
- **Acceptance:** a grammar test table of 20 utterances × 4 languages; ambiguity becomes a question.
- **LIVE probe:** POST the transcript to `/api/voice` as the QA kitchen login, confirm the token, and read the count session. Speech-to-text itself is not part of the proof; the transcript is the input.
- **Contract:** `voice.kitchen_count.v1`.

**P15. Menu translation drafts** (value 3, effort S; needs the same Workers AI binding as P9) **[OP]**

- **What it does:** in the dish editor, "Translate" calls the Worker, which calls `@cf/meta/m2m100-1.2b` for the missing languages. Drafts appear editable and are **never auto-published**; names are kept by default. Each call is capped per venue.
- **Files:** `catalog_edit.rs` / the i18n write path, `public/admin/menu-edit.js`, i18n.
- **LIVE probe:** translate a QA dish description en→sq/uk/ru with the real Workers AI and validate the response shape. A native speaker grades 20 sq drafts once, outside the gate.
- **Contract:** `catalog.translate_draft.v1`, `external_api_version: "workers-ai/@cf/meta/m2m100-1.2b"`.

**P16. Photo quality check on upload; feedback topics per dish** (value 2, effort XS each)

- **Photo check:** Laplacian variance, exposure and resolution are computed in the browser before upload (`public/admin/menu-edit.js`); a warning is shown, never a block.
- **Feedback topics:** a 4-language lexicon over `feedback.text`, aggregated per dish per week in the kitchen numbers. This is **never per order or per courier**, and the no-scoring gate must stay green.
- **LIVE probe:**
  - leave feedback "the rice was cold" on a delivered QA order;
  - read the topic count on that dish;
  - upload a blurred fixture photo and assert the warning.
- **Contracts:** `catalog.photo_check.v1`, `analytics.feedback_topics.v1`.

### Not recommended (with the condition to re-open)

- **Foundation time-series models in production.** Re-open when a venue has ≥ 12 weeks of cube history and Chronos-2-small beats the P6 baseline by ≥ 10 % MASE in an offline evaluation on the box.
- **An on-device LLM in the owner's browser** (Qwen3 / Gemma-4-E2B over WebGPU). Re-open if the offline-voice lane shows that WebGPU is present on the venues' actual phones, *and* the Albanian answers score acceptably.
- **A sentiment score.** It conflicts with no-scoring. Topics (P16) replace it.
- **CLIP auto-tagging.** Recipe-derived tags are exact.
- **Peer benchmarking.** It is meaningless with 2 venues. Re-open at ≥ 30 venues of similar type, k-anonymous with k ≥ 5.

### Order by value ÷ effort

| Rank | Row | Value | Effort | [OP] |
|---|---|---|---|---|
| 1 | P1 starter pack | 5 | S | Q1 (grams) |
| 2 | P3 menu matrix | 5 | S | — |
| 3 | P4 unexplained loss + digest | 4 | S | — |
| 4 | P2 cube + compare + 30-day fix | 5 | M | Q2 (cube vs "no store") |
| 5 | P5 suppliers, par, order list | 5 | M | Q6 |
| 6 | P7 expiry waste | 3 | XS | — |
| 7 | P8 lost sales + modifier BOM | 3 | S | — |
| 8 | P14 voice count | 3 | S | — |
| 9 | P6 prep forecast | 4 | M | — |
| 10 | P9 assistant intents | 4 | M | Q7 |
| 11 | P10 invoice photo | 4 | M | Q4 |
| 12 | P11 e-invoice (spike first) | 5? | M | Q3 |
| 13 | P15 translation drafts | 3 | S | Q5 |
| 14 | P16 photo + topics | 2 | XS | — |
| 15 | P13 HACCP freezing | 3 | S | Q8 |
| 16 | P12 storages | 3 | L | — |

**Lane cut (3 lanes, file-disjoint, GUESS):**

| Lane | Rows | Main areas |
|---|---|---|
| L-α | P1 → P5 | `admin/stock*`, `suppliers*`, `crates/dowiz-hub/src/stock/event.rs` |
| L-β | P3 → P4 → P7 | `services/analytics/kitchen/*`, `stock/tell.rs`, `kitchen-view.js` |
| L-γ | P2 → P6 | `services/analytics/{fold,cube,forecast}`, `hubstore` rotation, `more.js` |

P8, P9, P10, P11, P14 and P15 follow when a lane frees up.

---

## 4. Where dowiz cannot win, honestly

- **Benchmarks against peers** (Toast Benchmarking, Lightspeed Trends). dowiz has no network of comparable venues.
- **Hardware, payroll and labour analytics** (sales per labour hour). There is no time-clock data.
- **Certified fiscal till status in Albania**, which easyPos and Logic hold. dowiz imports from the till (ebills), and its fiscal send is OFF by operator decision (memory `dowiz-fiscal-send-and-loyalty-authorised`).
- **EDI with large distributors**, which MarketMan and xtraCHEF have. The Albanian e-invoice (P11) is the only comparable path, and it is unverified.
- **Albanian AI quality.** No small open model publishes an Albanian score, and Albanian speech recognition has no measured WER. Toast has no Albanian at all, but dowiz cannot *prove* good Albanian AI either. It can only prove its deterministic answers.
- **Scale of forecasting data.** At one venue's volume no ML method beats a well-made median by much, and the vendors' AI forecasts rest on network-wide data dowiz does not have.

---

## 5. Verdict

**The top 5 polish rows:**
1. **P1**, the starter pack. It turns on an engine that is already deeper than Poster's.
2. **P3**, the menu-engineering matrix. No vendor reviewed documents it.
3. **P4**, unexplained loss per count, sent to Telegram.
4. **P2**, the durable daily cube with comparisons. It also fixes the probable defect where the kitchen window beyond 30 days has no sales side.
5. **P5**, suppliers, par levels and an order list shared from the phone. Poster does not create purchases at all.

**The 3 model uses worth it:**
1. Menu translation drafts with Workers AI m2m100 (MIT), always reviewed by the owner.
2. Invoice OCR with tesseract.js `sqi` in the browser, plus a deterministic parser and learned supplier aliases.
3. Long-tail questions through the venue's own endpoint, or an opt-in Workers AI Qwen3, behind 25 deterministic intents.

**Not worth it now:**
- Time-series foundation models; a median plus TSB is enough at this volume.
- Sentiment scoring; it conflicts with the no-scoring rule.
- CLIP tagging.
- An in-browser LLM.
- Promising Albanian speech recognition before it has been measured.

**Where dowiz beats Poster and Toast:**
- Albanian/Ukrainian/Russian throughout.
- Refusal at checkout rather than deduction after the sale.
- Two-stage losses and weighed production acts.
- Lots with FEFO and expiry, and a raw-fish HACCP trail.
- Cost stamped at the moment of sale.
- Telegram group alerts instead of email.
- A classic menu-engineering matrix.
- Honest AvT in lek.
- Forecasts that show their own error.
- Every number traceable to its records.
- Free.

**Where dowiz cannot win:** peer benchmarks, labour analytics, fiscal certification, distributor EDI, and Albanian AI with measured quality.

**Questions for the operator (in Ukrainian):**
1. **Q1.** Хто з кухарів один раз перевірить стартовий набір із ~40 інгредієнтів суші: грами в рецептах і відсотки втрат при чищенні та варінні?
2. **Q2.** Чи дозволяєте ви щоденний «куб продажів»? Це похідний кеш з перевіркою, і саме він дає аналітику довшу за 30 днів і порівняння з минулим тижнем. Він відступає від правила «жодного окремого сховища аналітики».
3. **Q3.** Чи має заклад доступ до порталу e-Fatura / CIS під своїм NIPT? Чи можна там забирати отримані від постачальників е-рахунки (вручну або через API)? Потрібен один реальний NIPT для перевірки.
4. **Q4.** Будь ласка, надішліть 10 фото реальних накладних від постачальників у Дурресі, із закритими персональними даними. На них ми перевіримо розпізнавання.
5. **Q5.** Чи вмикаємо Cloudflare Workers AI на безкоштовному акаунті? Ліміт 10 000 нейронів на день спільний для всіх закладів. Він потрібен для чернеток перекладу меню і, за бажанням, для асистента.
6. **Q6.** Дані реальних постачальників для пілотного закладу: хто вони, в які дні возять, за скільки днів треба замовляти, розмір упаковки.
7. **Q7.** Потрібні 20 справжніх запитань власника албанською та носій мови, який оцінить відповіді асистента.
8. **Q8.** Рибу для сирих страв ви заморожуєте самі, чи купуєте вже оброблену? Від цього залежить, які поля журналу HACCP додавати.
