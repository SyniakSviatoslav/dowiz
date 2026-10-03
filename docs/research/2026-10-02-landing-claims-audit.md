# Landing claims audit — every sentence on dowiz.org against the code

Lane W-PUBUI, 2026-10-02. Scope: `workers/api/public/platform/landing-words.js` (4 languages) and the
Ukrainian defaults plus literal text in `workers/api/public/platform/index.html`. Follows
`docs/research/2026-10-02-system-integration-check.md` (the syscheck, which found the PQ tile false;
commit 27831f32 fixed PQ). Same rule as that commit: say what is true today, keep the section, do not
delete the claim.

Tags: MEASURED (a GET or command run 2026-10-02, quoted), CITED (`file:line` in the tree the lane was cut
from), SYSCHECK (the matrix row in the syscheck report). LIVE means wired, enabled, no missing secret, and
used by a live venue today.

## What was measured live (read-only GETs, 2026-10-02)

| probe | result |
|---|---|
| `GET https://sushi-durres.dowiz.org/api/public/locations/sushi-durres/menu` | 200, 88351 B. `location.name` = "Dubin & Sushi"; `supportedLocales` = sq, en, uk; `payments` = `{card:false, cash:true, applePay:false, googlePay:false, crypto:[]}`; `stripePublishableKey: null`; `telegramBot: null`; 165 dishes: `imageUrl` on 77, `nutrition.kcal` on 163, `allergens` non-empty on **0**, `ingredients` on 73 |
| same for `dubin-sushi` | 200, 79806 B, identical shape and counts |
| `GET https://sushi-durres.dowiz.org/api/public/rates` | 200: base ALL, rates for EUR and USD (`ppm`) |
| `GET https://cdn.dowiz.org/v/{sushi-durres,dubin-sushi}/manifest.json` | **404** both: no published menu on the CDN yet |
| `GET https://sushi-durres.dowiz.org/sw.js` | 200, 7026 B, carries `CDN_CACHE` (3 hits) |
| `GET https://api.github.com/repos/SyniakSviatoslav/dowiz` | 200: the repository is public |

## Counts

| | TRUE | OVERSTATED | FALSE | rows |
|---|---|---|---|---|
| the 20 feature tiles (a1t..a20t) | 5 | 9 | 6 | 20 |
| every other factual sentence | 19 | 5 | 15 | 39 |
| **total** | **24** | **14** | **21** | **59** |

The advantages paragraph said "nineteen already run in a live venue; the twentieth waits for one key".
After the rewrite: **thirteen** tiles run in a live venue today (a1 a2 a3 a4 a5 a7 a8 a9 a10 a14 a17 a19
a20) and **seven** are built and wait for a key, a setting or the venue's own data (a6 card -> Stripe;
a11 -> the venue's bot; a12 and a13 -> the venue's model endpoint; a15 -> recipes and supplies; a16 -> the
seal key, and not on a live path; a18 -> the venue's S3 bucket). Each of those seven lines now says what
it waits for. a7, a10 and a14 are wired and enabled but syscheck rows 16, 17 and 19 are C-U (never
exercised on a live delivery); they are counted live because nothing they need is missing.

## The 20 tiles

| key | claim (EN, before) | code path | verdict | why |
|---|---|---|---|---|
| a1 | 0% commission, $50/month, no delivery tariff | no commission or tariff code exists (pricing is a commercial term; no billing code) | TRUE | an offer, not a mechanism |
| a2 | own domain and brand | `workers/api/src/hubstore.rs:241` `slug_of_host`: a venue is `<slug>.<PLATFORM_HOST>` only; no custom-domain path; brand `services/venue/brand.rs` | **FALSE** (domain) | a subdomain of dowiz.org, not the venue's domain. Brand part true |
| a3 | installs as an app; the menu opens even offline | `public/store/install.js`; `public/sw.js:3-8` network first, shell only; menu offline only via a published CDN manifest (`sw.js:100-111`) | OVERSTATED | MEASURED manifests 404, so offline shows the shell and the venue's phone, not the menu |
| a4 | four languages; prices in the currency the customer pays in | `public/lib/langs.js` (4); `store/nav.js:14,78` reading currency; `/api/public/rates` | OVERSTATED | prices can be READ in EUR/USD; payment is in the venue's currency (ALL). Venues list 3 menu languages; the interface speaks 4 |
| a5 | photos, calories, allergens, ingredients, recipes; never grey placeholders | `store/dish.js`; menu fields | OVERSTATED | MEASURED photos 77/165, allergens 0/165; recipes are never shown to customers |
| a6 | card or cash, promo codes, confirmation within a minute | `storefront.rs:1069` Stripe; SYSCHECK row 10 | **FALSE** (card) | MEASURED `payments.card:false`, `stripePublishableKey:null` on both venues |
| a7 | arrival time computed from the route | `live_eta.rs:237,253,263,276` all `eta::straight_line_m`; `crates/dowiz-core/src/eta.rs:204,291` "deliberately NOT presented as road distance" | OVERSTATED | straight-line distance plus kitchen time plus the courier's last position |
| a8 | storefront, phone, WhatsApp, Instagram, partner APIs — one queue | `services/ordering/channel.rs:23,25` WhatsApp/Instagram "once it places orders (none today)"; `:30` "until a partner API exists" | **FALSE** | messaging channels go to the inbox; marketplace orders are typed in from a tablet (`services/orders/aggregator.rs:1-4`); till sales imported (eBills) |
| a9 | owner console on a phone | `public/admin/**` | TRUE | |
| a10 | route, statuses, cash and proof of delivery | `public/courier/screens.js:161-164` cash count; `courier/app.js:172` map | OVERSTATED | no proof-of-delivery artefact (photo/code); no turn-by-turn route |
| a11 | every order lands in your bot within seconds | `notify.rs:57` venue `notify.telegram.token`; SYSCHECK row 5 C-U | OVERSTATED | only after the venue connects its own bot; which venues have one is UNVERIFIED |
| a12 | dowiz drafts posts; you publish to Telegram and Instagram | `services/engagement/posts.rs:153` drafts via `assist::ask`; `verdict.rs:66-72,117` publishes | OVERSTATED | drafting needs the venue's model endpoint (`assist.rs:86` `ai.enabled`, off by default) and channel tokens |
| a13 | local AI on your own server; nothing is sent away | `assist.rs:16,86`; `privacy/registry/outside.rs:50` "the venue chooses the endpoint … It receives order id, status, total, items …" | **FALSE** | order data IS sent to the endpoint the venue names; off until set. True part: never a customer's name or phone |
| a14 | bookings with a pass, chat, wallet | `lib.rs` booking routes (SYSCHECK row 18 C+P), row 19 C-U | TRUE | |
| a15 | stock deducted per order; when it runs out the dish hides itself | `crates/dowiz-hub/src/stock/log.rs:177-191` fail-closed reservation; no auto-hide anywhere (`grep -rn "sold out\|auto.*unavailable"` -> 0 code hits) | **FALSE** | an order the shelf cannot cover is refused; no dish hides; 0 recipes / 0 supplies on both venues (memory 09-27 wipe) |
| a16 | ML-KEM-768 and ML-DSA-65 pass NIST ACVP in CI; not live | syscheck §3 | TRUE | fixed by 27831f32; unchanged |
| a17 | integers, double entry, refunds net to zero | `tools/gates/float-money.sh` in CI (`ci.yml:88` run-all); `hubdo/refund.rs:1-3` log + stock only | OVERSTATED | integer minor units are true and gated; double entry and "refunds net to zero" are kernel properties not on the live refund path (no card refund exists, SYSCHECK row 11) |
| a18 | backups to S3 every night; MCP server | `cloud.rs:50-55` per-venue `cloud.s3.*`; `mcp/tools.rs:57` | OVERSTATED | the copy runs only for a venue that set a bucket (UNVERIFIED which); MCP is live |
| a19 | every node keeps its own database; cloud optional; no middleman | `CLAUDE.md` crate map: the Worker is the only server since 2026-10-01; one Durable Object per venue (`hubstore.rs:265` `id_from_name(&self.venue)`) | **FALSE** | everything runs on Cloudflare; couriers and customers keep no database. True part: each venue's store is its own |
| a20 | open source AGPL-3.0; no courier ratings | `LICENSE`; `workers/api/Cargo.toml:5`; `tools/gates/no-scoring.sh`; MEASURED repo public | TRUE | |

## Every other factual sentence

| key | claim (EN, before) | verdict | evidence / why |
|---|---|---|---|
| title | your venue's own app, 0% commission | TRUE | |
| h1 | your venue, your app, zero commission | TRUE | |
| lede | own app on its own domain — menu, payment, courier live | **FALSE** | domain (a2), payment = cash only (a6) |
| advP | nineteen run live; the twentieth waits for one key | **FALSE** | thirteen live, seven waiting (above) |
| filmP | describes the film | TRUE | |
| productH | your colours, menu, customers; order in a few taps | TRUE | |
| chip | 8–12 min · courier on the way | TRUE | a mock-up on a phone frame |
| phoneCap | Dubin & Sushi, Durrës — a live venue | TRUE | MEASURED `location.name` = "Dubin & Sushi" |
| s1k | own domain | **FALSE** | a2 |
| s1h | the venue's app, not the platform's | TRUE | |
| s1p | own domain …; opens the menu even offline | **FALSE** | a2, a3 |
| s2h | the chef's photos, calories, allergens | TRUE | capability exists (fields, `store/dish.js`) |
| s2p | card or cash, confirmation within a minute; every dish a photo, calories, allergens, never a placeholder | **FALSE** | a6, a5 |
| s3h | they watch it come | TRUE | |
| s3p | … more repeat orders | OVERSTATED | an outcome nobody measured |
| engineH | an engine that needs no middleman | TRUE | no aggregator between venue and customer |
| engineP | open delivery protocol, local AI, no server everything stops without | **FALSE** | a13, a19 |
| p1h | one hub for every order | TRUE | |
| p1p | WhatsApp, Instagram, partner APIs in one log; immutable log | **FALSE** | a8. The append-only log and the forbidden-transition error are true |
| p2h | arrival time from the road | OVERSTATED | a7 |
| p2p | ETA from the route; no address ends up in someone else's logs | **FALSE** | a7; `public/store/address.js:30` reverse-geocodes the pin at `nominatim.openstreetmap.org`. Tiles: `store/track-map.js:16` OpenFreeMap |
| p3h, p3p | PQ, stated precisely | TRUE | 27831f32 |
| p4h | Local AI. Nothing is sent away. | **FALSE** | a13 |
| p4p | on your own server; money, states and SIGNATURES by deterministic code | OVERSTATED | endpoint is the venue's choice; no signature is live (syscheck P3/P4: `actor_pubkey` zeros) |
| p5h | a protocol, not a platform | OVERSTATED | it runs as one platform today |
| p5p | every node — venue, courier, customer — keeps its own database; cloud optional | **FALSE** | a19 |
| statement | … no server that everything stops without | **FALSE** | the Worker is that server |
| termsH, priceP, t2p | flat $50, 100% of each order | TRUE | offer |
| t1p | aggregators take a share and the customer | TRUE | external market claim, not audited against code |
| t2f1 | own domain and an app without a store | **FALSE** | a2 |
| t2f2 | four languages, payments, courier live | **FALSE** | payments = cash only |
| t2f3 | data stays in the venue | **FALSE** | a19 |
| closeP, joinOk | leave an email; one letter from a person | TRUE | SYSCHECK row 32 C+P |
| fNote | decentralised, local-first, post-quantum-ready protocol | OVERSTATED | not decentralised or local today; "post-quantum-ready" is the gate's allowed word (`tools/gates/pq-words.sh`) |
| marquee (index.html) | storefront · phone · WhatsApp · Instagram · API · one order log | **FALSE** | a8; now storefront · phone · marketplaces · till · one order log (`mq4`, `mq5`) |

## Before -> after (EN and UK; sq and ru carry the same meaning)

| key | EN before | EN after | UK after |
|---|---|---|---|
| lede | … its own app on its own domain — menu, payment, courier live on the map — … | … its own app at its own address — menu, orders, the courier on the map — … | … власний додаток на власній адресі — меню, замовлення, кур'єр на мапі — … |
| advP | Nineteen already run in a live venue; the twentieth is built, verified and waiting for one key. | Thirteen already run in a live venue; seven are built and wait for a key, a setting or the venue's own data — each line says which. | Тринадцять уже працюють у живому закладі; сім зібрані й чекають на ключ, налаштування чи дані самого закладу — у кожному рядку сказано, на що. |
| a2t/a2p | Own domain and brand / The venue's address, logo, colours and seal | Own address and brand / The venue's own address (name.dowiz.org), logo, colours and seal | Власна адреса і бренд / Своя адреса (назва.dowiz.org), логотип, кольори й печатка закладу |
| a3p | … the menu opens even offline. | … offline, it opens the venue's page with its phone number. | … без зв'язку відкривається сторінка закладу з його телефоном. |
| a4t/a4p | the customer's currency / prices in the currency the customer pays in | Four languages, three currencies / prices can be read in lek, euro or dollars — payment is in the venue's currency | Чотири мови, три валюти / ціни можна читати в леку, євро чи доларах — оплата у валюті закладу |
| a5p | Photos, calories, allergens, ingredients and recipes — never grey placeholders. | Photos, calories, allergens and ingredients on every dish the venue fills in. | Фото, калорії, алергени й склад — на кожній страві, яку заповнив заклад. |
| a6p | Card or cash, promo codes, a confirmation within a minute. | Cash, promo codes, and the venue's confirmation on the customer's screen. Card payment is built and switches on once Stripe is connected. | Готівкою, промокоди, підтвердження закладу на екрані клієнта. Оплата карткою зібрана й увімкнеться, щойно підключать Stripe. |
| a7p | … arrival time is computed from the route. | … computed from the courier's position, the distance and the kitchen time. | … рахується з його місця, відстані й часу кухні. |
| a8p | Storefront, phone, WhatsApp, Instagram and partner APIs — one queue … | Storefront, orders taken by phone or from a marketplace tablet, sales from the till — one queue … | Вітрина, замовлення з телефону й з планшета маркетплейсу, продажі з каси — одна черга … |
| a10p | Route, statuses, cash and proof of delivery | Map, statuses and the cash count at the door | Мапа, статуси й підрахунок готівки біля дверей |
| a11p | Every order lands in your bot within seconds … | Connect your own bot and every order lands in it … | Підключіть власного бота — і кожне замовлення приходить у нього … |
| a12p | dowiz drafts posts … | With an assistant connected, dowiz drafts posts … | З підключеним помічником dowiz пише пости … |
| a13t/a13p | Local AI assistant / … on your own server. Nothing is sent away. | AI assistant on your model / … through the model the venue chooses; it never sees a customer's name or phone. Off until you set one. | ШІ-помічник на вашій моделі / … через модель, яку обирає заклад; імені й телефону клієнта не бачить. Вимкнений, доки ви її не вкажете. |
| a15p | Stock is deducted with every order; when it runs out, the dish hides itself. | Every order reserves the ingredients its recipe names; an order the shelf cannot cover is refused. Starts working once the venue enters recipes and supplies. | Кожне замовлення резервує інгредієнти з рецептури; замовлення, яке склад не покриває, відхиляється. Працює, щойно заклад внесе рецептури й поставки. |
| a17p | Integers, double entry, refunds that net to zero — no floating point. | Every amount is a whole number of the smallest unit; a CI gate refuses floating point near money. | Кожна сума — ціле число найменших одиниць; гейт у CI не пропускає плаваючу кому поруч із грошима. |
| a18p | Backups to S3 every night; … | A full copy goes to your S3 bucket every night once you connect one; … | Щоночі повна копія йде у ваше S3-сховище, щойно ви його підключите; … |
| a19t/a19p | Your data stays in the venue / Every node keeps its own database; the cloud is optional, there is no middleman. | Each venue's data kept apart / Every venue has its own store, never pooled with another's; the nightly copy goes to your own S3. | Дані кожного закладу окремо / Кожен заклад має власне сховище, ніколи не змішане з іншими; нічна копія — у ваше S3. |
| s1k/s1p | own domain / … on your own domain … and opens the menu even offline. | own address / An address of your own … no store. | власна адреса / Власна адреса … без магазину. |
| s2p | Menu, cart, card or cash, and a confirmation within a minute. … never a grey placeholder. | Menu, cart, cash payment, promo codes and the venue's confirmation on screen. Every dish carries the photo, calories and allergens the venue fills in. | Меню, кошик, оплата готівкою, промокоди й підтвердження закладу на екрані. Кожна страва — з фото, калоріями й алергенами, які заповнив заклад. |
| s3p | … Fewer "where is my order" calls, more repeat orders. | … Fewer "where is my order" calls. | … Менше дзвінків «де моє замовлення». |
| engineP | … open delivery protocol: one order log, post-quantum-ready cryptography, local AI, and no server that everything stops without. | … open code: one order log per venue, post-quantum primitives checked against the NIST vectors, and an assistant on the model you choose. | … відкритий код: один журнал замовлень на заклад, постквантові примітиви, перевірені тест-векторами NIST, і помічник на моделі, яку обираєте ви. |
| p1p | Storefront, phone, WhatsApp, Instagram, partner APIs — everything lands in one log. … the courier one route … immutable log … | Storefront, orders taken by phone or from a marketplace tablet, sales from the till — everything lands in one log. … the courier their deliveries … WhatsApp and Instagram messages arrive in one inbox. … append-only log … | Вітрина, замовлення з телефону й з планшета маркетплейсу, продажі з каси — … кур'єр — свої доставки … Повідомлення з WhatsApp та Instagram — в одній скриньці. … журналі, що лише доповнюється … |
| p2h | Arrival time from the road, not a guess | Arrival time from where the courier is | Час прибуття — з того, де кур'єр |
| p2p | … ETA computed from the route. … no address or order number ends up in someone else's logs. | … ETA computed from their position, the distance and the kitchen time. … no order number leaves dowiz, and an address pin is looked up through OpenStreetMap's open geocoder. | … ETA рахується з його місця, відстані й часу кухні. … номер замовлення не виходить за межі dowiz, а точку адреси розпізнає відкритий геокодер OpenStreetMap. |
| p4h | Local AI. Nothing is sent away. | AI on the model you choose | ШІ на моделі, яку обираєте ви |
| p4p | … on your own server … Money, states and signatures … | … through the model endpoint the venue sets — its own server if it likes — and never receives a customer's name or phone. … Money and order states … | … через модель, яку вказує заклад — хоч на власному сервері, — і ніколи не отримує імені чи телефону клієнта. … Гроші й стани замовлень … |
| p5h | A protocol, not a platform | Open code, not a black box | Відкритий код, а не чорна скринька |
| p5p | Every node — venue, courier, customer — keeps its own database; the cloud is optional. | Each venue's data lives in its own store, never pooled with another's, and its nightly copy can go to your own S3 bucket. | Дані кожного закладу живуть в окремому сховищі, ніколи не змішаному з іншими, а нічна копія може йти у ваше власне S3. |
| statement | … No server that everything stops without. | … Every line of the code open to read. | … Кожен рядок коду відкритий. |
| t2f1/t2f2/t2f3 | Own domain … / Four languages, payments, courier live / Data stays in the venue | Own address … / Four languages, orders, courier on the map / Each venue's data kept apart | Власна адреса … / Чотири мови, замовлення, кур'єр на мапі / Дані кожного закладу окремо |
| fNote | a decentralised, local-first, post-quantum-ready delivery protocol | a delivery platform with post-quantum-ready primitives | платформа доставки з постквантово готовими примітивами |
| marquee | WhatsApp · Instagram · API | marketplaces · the till (`mq4`, `mq5`) | маркетплейси · каса |

## Outside this lane's files, said so it is not lost

* `public/store/i18n.js` `installBody` ("The menu opens with one tap, even offline") makes the a3 claim on the
  storefront itself; it becomes true once a venue's manifest is published and cached (BN2 + W-BN3A's sw.js).
  Not edited here.
* When F16 lands (seal key) and Stripe keys are set, a6, a16 and a18 move to the live column; the counts
  in advP must be re-derived then, from this table.
