# Dynamic menu ("Menu Mutator") and four resilience/insight items: law, evidence, an honest design

Lane **W-MUTATOR** (Opus). Research and design only. Written 2026-10-03 against `main fba9ee0f`.
The box was under a production deploy for the whole lane. The lane therefore ran **no Bash, no Grep, no git, no cargo/node/gates**, and used only Read, WebSearch and WebFetch. Every code claim below comes from a file that was **read**. A claim that needs a tree-wide search is marked **VERIFY-GREP**; the checklist in §8 gathers them for after "BOX FREE".

**Tags used in this document:**
- **CITED-CODE** `path:line`: read in this lane.
- **SOURCE**: an external page fetched or searched on **2026-10-03**. Its URL is given. "snippet" means that only a search-engine summary was available.
- **CALC**: arithmetic done here from stated inputs.
- **GUESS**: my estimate. Treat it as a hypothesis.

---

## 0. Відповідь оператору (одна сторінка)

**Коротко:** «мутатор меню» у запропонованому вигляді будувати **не можна і не варто**. Чесна версія — можна, дешево, і вона корисна.

**Що з пропозиції правда, а що ні (перевірено по дереву):**
- **«Профіль на пристрої»** — правда як патерн. Вітрина вже без акаунта: історія замовлень, адреси й фільтр алергенів живуть у браузері (`state.js:250-253`, `:314-320`).
- **«Вмикач на заклад»** — механізм уже є: `loc.features`, тобто прапорець прапорцем (`state.js:157-161`).
- **«Обчислюється за мікросекунди на edge»** — арифметика справді мікросекундна. Але робити її **на edge** — неправильне місце. BN2 щойно прибрав читання меню з Worker-а: меню й манифест ідуть із R2/CDN (`hubdo/publish.rs`, `store/shell.js`). Персональне ранжування на Worker-і повернуло б запит на кожен перегляд, а ліміт Free — 100 000 запитів на добу. Правильне місце — **браузер**.
- **«DAG-рушій рахує ваги»** — неправда. DG8 рахує предикати (недоступно, алерген, fsm_ok…), а не ваги. Ранжування цілком влазить у чисту функцію в `dowiz-hub`, яка виконується під час публікації.
- **«Bebop-бінарна нормалізація»** — напівправда. Бінарні блоки `.dwb` (`menu_prices`, `names`) уже публікуються (`publish.rs:53`), але ваг у них немає.

**Право й етика** (детально в §2):
- В Албанії сьогодні діють Закон 124/2024 про персональні дані (дзеркало GDPR), Закон 9902/2008 про захист споживачів і ДРМ 434/2018 про маркування харчових продуктів.
- EU AI Act, DSA та Omnibus в Албанії **не діють**. Проте проєкт албанського закону про ШІ, повністю узгодженого з AI Act, з 29.05.2026 на громадському обговоренні.
- Висновок для сигналів і підштовхувань:
  - **НЕ можна:** персональна ціна чи бонус за профілем; тригер «вагання»; швидкість скролу; «уся інформація, яку можна зібрати»; приховування страви через **виведену** (не заявлену гостем) алергію; «популярне сьогодні», яке не виміряне.
  - **Можна:** погода, час, свята, події, які вводить власник; «використати першим» для партії, що спливає, після схвалення власником і з чесним підписом; «найчастіше замовляють за тиждень» з реальним числом і порогом; фільтр алергенів **за явним вибором гостя** (він уже є: `dw_avoid`).
  - **Можна лише з явною згодою (opt-in):** пам'ять смаку, яка ніколи не покидає телефон.
- Інваріант «нікого не оцінюємо»:
  - Вектор смаку **на пристрої** ранжує **страви** для гостя, а не гостя, тому його не порушує.
  - Серверний профіль або сегмент гостя порушує.
  - «Знеособлений» KPI персоналу порушує: псевдонім — це все одно людина. Подієвий облік рівня закладу вже існує (`exceptions/alert.rs`, `fold.rs`).

**Чи працює це взагалі** (§3):
- Найкращий польовий доказ: у ресторані в Пекіні показ «топ-5 страв» підняв попит на них на **13–20 %** (Cai, Chen, Fang, AER 2009). Позиція першою чи останньою в списку дає до **2×** популярності (Dayan & Bar-Hillel 2011).
- DoorDash, Uber Eats і Wolt **не публікують** чисел приросту від персоналізації.
- Мета-аналіз «нуджів» після поправки на упередження публікацій **не знаходить ефекту** (Maier et al., PNAS 2022).
- Один заклад на ~20–60 замовлень на день (GUESS з W-DEPTH):
  - ефект +15 % на страву з 10 % часткою замовлень він виміряє за **~320 днів** при поділі 50/50 (CALC);
  - +10 % до середнього чека — за **~29 днів** (CALC, CV=0.6 GUESS).
- Тобто приріст **майже не вимірюваний**, і звіт мусить так і казати.

**Що будувати** (§4):
- **Шар 0** — контекст без персональних даних, однаковий для всіх. Окрема смуга «Для сьогодні» над меню, ≤ 6 страв, кожна з причиною («дощ — супи», «шеф радить», «найчастіше за тиждень: 23»). Порядок меню закладу **не ламаємо**: власник його розставив свідомо (`menu.js:210-211`). Правила пише власник, з прев'ю і поясненням «чому ця страва перша».
- Погода — **MET Norway** (безкоштовно, CC BY 4.0, комерційне використання дозволене). **Open-Meteo безкоштовний лише некомерційно**, комерційний план від $29 на місяць.
- Дані тягне нічний cron раз на добу на заклад і кладе у `context.json` на CDN. Додаткових запитів до Worker-а на візит — **0**.
- **Шар 1** — пам'ять смаку лише на телефоні: opt-in, стирається одним дотиком, нікуди не надсилається.
- **Шар 2** (серверний профіль) — **не будувати**.

**Решта пунктів:**
- **(a)** CRDT **не потрібен**: DO — єдиний писар, черга намірів з idempotency-ключами вже є в кур'єра й офіціанта. Найцінніше:
  - «серцебиття кухні» з автопаузою, щоб онлайн-замовлення не падали в порожнечу, коли в закладі зник інтернет;
  - ДБЖ на роутер.
  Фіскалізація Албанії дозволяє офлайн-чек із відправкою протягом **48 годин**.
- **(b)** Див. `2026-10-03-external-deps-and-aggregator.md`: Wolt — єдиний, Glovo в Албанії немає з 2024.
- **(c)** Сторожа маржі — це 1–2 дні коду поверх наявних цін постачань і собівартості. Без рецептів (на живих закладах 0) вона мовчить.
- **(d)** Аномалії вже рахуються на рівні закладу. Бракує «скасування після друку рахунку» і тижневого дайджесту. Персональних KPI — ні.

**Рішення оператора** — у §7.

---

## 1. Ground truth in the tree

### 1.1 What each idea would build on

| Input / piece | What exists | Where (CITED-CODE) | State |
|---|---|---|---|
| Storefront menu render | Cards are built once and then only mutated; a sort reorders nodes; `pop` = **the venue's own order** ("a restaurant arranges its menu deliberately and we should not overrule it") | `workers/api/public/store/menu.js:1-22, 192-226` (comment at `:210-211`) | LANDED |
| Venue-set "popular" flag | Tag `popular` draws a sakura flag on the card. It is **a tag the venue types**, not a measured fact | `store/menu.js:35, 112` | LANDED. **Legal finding §2.4 (row MR0)** |
| Sort options | `pop` (venue order), `low`, `high`, `az`. No contextual sort | `store/menu.js:43, 212-215` | LANDED |
| Published menu on R2/CDN (BN2) | The DO writes content-addressed objects plus `manifest.json` (`max-age=30`). Blocks `menu_prices`, `names` (`.dwb`). The recipes block `bom` is **not public** | `workers/api/src/hubdo/publish.rs:1-53` | LANDED (the `CDN` binding may still be commented in `wrangler.toml`, per `publish.rs:27-29`: VERIFY-GREP) |
| Storefront CDN reader | Reads the manifest. Falls back to the Worker route, never blank. **Puts the clock back in the browser** (`clockAt`): open/closed is derived client-side from published hours | `workers/api/public/store/shell.js:1-120` | LANDED. The same pattern serves context ranking: publish data, derive the view in the browser |
| Device copy (BN3A: `lib/blocks.js`, IndexedDB `dowiz.blocks.v1`) | **Not found.** `workers/api/public/lib/blocks.js` does not exist. The privacy registry's `BROWSER` list (which the personal-data gate forces to be complete) has no `dowiz.blocks.*` row | `privacy/registry/outside.rs:119-182` | IN FLIGHT or unmerged (VERIFY-GREP `blocks.v1`) |
| Device-side stores today | `dw_orders` (order history, "NO ACCOUNT, and that is the design"), `dw_addrs`, `dw_avoid` (allergen avoid list), carts; admin `dowiz.replica.v1` (with Name/Phone/Address); courier `dowiz.outbox`; room `dowiz.room.outbox` | `store/state.js:250-320`; `registry/outside.rs:119-182` | LANDED |
| Stock lots and expiry | Lots with expiry, FEFO draw order, reconciliation to the shelf | `crates/dowiz-hub/src/stock/lots.rs:1-80` | LANDED. **Live venues hold 0 recipes and 0 supplies** (memory `dowiz-stock-ledger-works-but-is-off`; W-DEPTH §0) |
| Stamped cost, food cost per dish, supplier price points and ‰ change | `portionCost`, `cogs`, `margin`, `foodCostPm` per dish; `prices[].points` + `changePm` per supply; reorder hint `REORDER_BELOW_DAYS=3` | `workers/api/src/services/analytics/kitchen/report.rs:16-35, 37-133` | LANDED (inert without recipes) |
| Allergens | EU-14 three-way `Declaration` (Undeclared / None / Contains). An unknown code is refused. "An undeclared dish cannot be put on sale" | `crates/dowiz-hub/src/allergens.rs:1-80` | LANDED. The DG8 fixture found **86 undeclared allergens** on the sushi fixture (memory `dowiz-quality-queue-2026-09-30`) |
| Allergen display | The dish sheet: "Allergens are the one line that must never be silent: three states, three treatments" | `store/dish.js:1-11` | LANDED |
| Allergen filter | `dw_avoid`. `hiddenBecause()` hides `contains` **and `undeclared`** when the guest has chosen what to avoid | `store/state.js:292-327` | LANDED in state. Whether it is wired into `applyFilters` is not visible in `menu.js` (VERIFY-GREP `hiddenBecause`). Note `menu.js:21-22`: "allergens are not this storefront's to declare (operator decision, 2026-09-18)" |
| Analytics | Pure fold, **"NO ANALYTICS STORE"**, windows of 7 or 30 days; `top_products` (top 8), `by_hour[24]`, `by_channel` | `services/analytics/fold.rs:1-62` | LANDED; P2 (daily cube) approved 10-03 |
| Venue-level exception register | Kinds `void_after_kitchen`, `comp`, `late_amendment`, `refund`, `cash_outside_till`, `pay_out`, `over_short`. "nothing is totalled or ordered PER PERSON: `by` is a name on a row" | `workers/api/src/exceptions/fold.rs:1-34` | LANDED |
| Exception alert | A count per venue per period crosses the owner's threshold (default 3) and goes to the owner's own Telegram chat. "A COUNT PER VENUE-PERIOD, NEVER PER PERSON" | `workers/api/src/exceptions/alert.rs:1-75` | LANDED |
| "The DAG" | DG1-DG10 on the bebop side. DG8 = a Datalog rule layer for `unavailable, allergen, fsm_ok, courier_may, personal` (memory `dowiz-quality-queue-2026-09-30`, merged 8300b51b). W-BOTTLENECK I1: "rules (Datalog) yes, bytecode no" | memory; `docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md:51` | No ranking or weight rule exists (VERIFY-GREP in the DG8 rule file) |
| DO alarm (`hubdo/timer.rs`) | One alarm per venue object, for the outbox, the eBills till link and the fiscal queue (`SEND_ENABLED` gate). The nightly cron re-arms a lost alarm | `workers/api/src/hubdo/timer.rs:37-58, 186-203` | LANDED. A context publish fits the **nightly** run, not the alarm |
| Nightly cron | `crons = ["17 3 * * *"]` | cited by W-EXTRES (`wrangler.toml:177`) | LANDED |
| Replica / changes-since | Catch-up window in the object; a whole-image write clears it (memory `dowiz-redteam-2026-09-21`); admin replica `dowiz.replica.v1` | memory; registry | LANDED |
| Worker CPU limit | Workers **Free**, 10 ms CPU. 622 kills at exactly 10 000 µs. The minute cron is over 10 ms on 656 of 1440 runs | memory `dowiz-costs-free-plan-and-bebop`; operator "stay on Free" (`ROADMAP-2026-09-22.md:63`) | Binding constraint |
| Request budget | 1,250 Worker requests a day, 14.7 DO requests per Worker request (MEASURED 2026-10-01 by W-BOTTLENECK, `…bottlenecks….md:25`). DO 100 000/day account-wide | same doc `:66-67` | Binding |
| Personal-data registry and gate | Every store, host and browser key needs a row, or `tools/gates/personal-data.sh` fails. Lawful basis enum cites **Law 124/2024 Art. 7(1)** | `workers/api/src/privacy/registry.rs:1-150`; `tools/gates/personal-data.sh:1-27` | LANDED. Any new key (`dowiz.taste.v1`) or host (`api.met.no`) must add a row |
| Consent | Unticked box every time; a wording id proves the sentence read; marketing consent only beside a phone | `store/consent.js:1-60` | LANDED. Reuse this pattern for any consent, never a banner |
| No-scoring gate | Pairs PERSON × JUDGEMENT (`customer_rating`, `guest_tier`, `staff_rank`…, plus bare `reputation`/`vip`) in the product path | `tools/gates/no-scoring.sh:35-63` | LANDED. It does **not** catch `taste`, `affinity`, `propensity`, `segment`, `ltv`, `price_sensitivity` (row GATE-1) |
| Per-venue feature flag | `on(name)`: a flag is ON unless `loc.features[name] === false` | `store/state.js:154-161` | LANDED. **Careful:** the default is ON, so a new `menuRank` flag must be read as opt-in (§4.6) |

### 1.2 The pasted claims, checked

| Claim in the proposal | Verdict | Why |
|---|---|---|
| "Computed in microseconds at the edge" | **Half true, wrong place** | Ranking 165 dishes by ≤ 20 rules is microseconds of arithmetic (GUESS: about 3 300 predicate checks). Doing it **per guest at the Worker**, though, means a Worker request per view: it undoes BN2 (menu reads now go to the CDN), it defeats the 30 s edge cache, and it spends the Free daily caps. Done in the **browser**, it is just as fast and free. |
| "Bebop binary normalisation" | **Half true** | Real binary `.dwb` blocks for `menu_prices` and `names` are published (`publish.rs:53, 83-85`); decoding the DG7 block takes 8 µs (W-BOTTLENECK `:27`). There is no weight block. |
| "The DAG engine computes weights" | **False today** | DG8 computes boolean predicates (above). No weights or scores anywhere in it (VERIFY-GREP). A rule like `featured(d) :- uses(d,s), expiring(s)` *could* be a Datalog rule, but nothing requires the DAG. |
| "Privacy-first, profile kept on the device" | **True as a pattern** | The storefront already keeps history, addresses and the avoid list only on the device (`state.js:250-320`). Contradicted by the same proposal's "all the information about them that can be collected" (§2.4). |
| "Per-venue on/off setting" | **True, the mechanism exists** | `loc.features` (`state.js:157-161`), with the default-ON caveat. |
| "Most ordered today" | **Not measurable honestly today** | At 20-60 orders/day (GUESS), "today" at 12:00 is 2-6 orders. The fold can give a **7-day** top with counts (`fold.rs:53`). |
| "Hide or swap dangerous dishes" | **Only on the guest's explicit choice** | That already exists (`dw_avoid`). Inferred allergies are illegal to rely on and unsafe (§2.3). |
| "Auto-generated event packs" | **No data source** | There is no free, reliable, legal feed of events in Durrës or Tirana (§4.8). Packs are the owner's to compose. |
| "Expiring stock drives placement" | **True in the engine, inert in the data** | Lots and expiry exist; live venues have 0 supplies and 0 recipes. |

---

## 2. Law and ethics

### 2.1 Which law binds an Albanian venue today, and which is best practice

| Instrument | Binds a Durrës venue today? | Source |
|---|---|---|
| **Albania Law 124/2024 "On personal data protection"**, adopted 19 Dec 2024, Official Gazette no. 9 of 17 Jan 2025, in force 1 Feb 2025. Some articles apply from **17 Jan 2027**: Art. 29(3), 31, 32, 35, 36, 64, 65, 67(2)/(3)/(5) per Art. 101(2), snippet. Aligned with GDPR and Directive 2016/680. Regulator: Commissioner for the Right to Information and Personal Data Protection (IDP). Fines up to ALL 2 bn or 4 % of turnover | **YES** | [IDP text (sq)](https://idp.al/wp-content/uploads/2025/03/Law-no.124-2024.pdf), [IDP text (en)](https://idp.al/wp-content/uploads/2025/04/Law-no.124-2024-DP.pdf) (both PDFs fetched but compressed, not machine-readable here); [DLA Piper Albania](https://www.dlapiperdataprotection.com/?t=law&c=AL); [KPMG Albania, 2025-02](https://kpmg.com/al/en/insights/2025/02/new-law-on--personal-data-protection-.html); [Clym summary](https://www.clym.io/regulations/law-no-1242024-on-personal-data-protection-albania) (snippet for the delayed articles) |
| Law 124/2024 article map, per DLA Piper: Art. 7 lawful bases (Art. 7(1)(f) legitimate interest); Art. 8 consent conditions, parental consent under 16; Art. 9 special categories; Art. 19 and Art. 46 the right to object, including to direct marketing **and the profiling related to it** (Art. 46(4)); Art. 46(3) explicit consent for sensitive data. **The automated-decision article (the GDPR Art. 22 equivalent) and the principles article (the Art. 5 equivalent) were not confirmed by number.** | — | [DLA Piper](https://www.dlapiperdataprotection.com/?t=law&c=AL). The tree's registry cites Art. 7(1) and Art. 41(3)(b) (`registry/outside.rs:31`) |
| **Albania Law 54/2024 "On electronic communications"**, in force 20 Dec 2024. DLA Piper says cookies are governed here, not in Law 124 (Art. 163 location data, Art. 165 unsolicited communications). **Whether it has an ePrivacy Art. 5(3) equivalent (consent to store or read on the terminal) is UNVERIFIED**: the official .docx was fetched and saved at `/root/.claude/projects/-root/2f7d739d-…/tool-results/webfetch-1791015004520-yeuxte.docx` but could not be unzipped without Bash | probably | [infrastruktura.gov.al .docx](https://www.infrastruktura.gov.al/wp-content/uploads/2025/07/Law-no.-54-date-30.06.2024_On-electronic-communications-in-the-Republic-of-Albania.docx); [DLA Piper](https://www.dlapiperdataprotection.com/?t=law&c=AL) |
| **Albania Law 9902/2008 "On consumer protection"** (misleading practices). July 2023 amendments: fines up to 2 % of annual turnover (snippet) | **YES** | [Law 9902 (en)](https://www.avokatipopullit.gov.al/media/manager/website/reports/LAW%20No%209902,%20dated%2017.4.2008%20ON%20CONSUMER%20PROTECTION.pdf); [Tirana Times](https://www.tiranatimes.com/tougher-fines-for-consumer-abuse-_112210/) |
| **Albania DCM (VKM) no. 434 of 11.07.2018 "On food labelling and information of consumers"**: a **partial** approximation of Reg. 1169/2011; effective two years after its publication (OG no. 106, 19.07.2018), i.e. July 2020 | **YES** | [Deloitte Albania Legal News, July 2018](https://www.deloitte.com/content/dam/Deloitte/al/Documents/tax/Deloitte%20Albania_Legal%20News_July%202018.pdf) (snippet); the FAOLEX record returned HTTP 403 |
| **Albania fiscalisation Law 87/2019**: an offline receipt is allowed without the NIVF and must be sent within **48 h** of the connection loss | **YES** (item a) | [fiscal-requirements.com](https://www.fiscal-requirements.com/news/2937-fiscalization-in-albania-what-happens-if-the-communication-or-connection-with-the-tax-administration-is-interrupted) (snippet); [PwC, Law on fiscalization](https://www.pwc.com/al/en/Law_on_fiscalization.pdf) |
| **Albania draft law "On Artificial Intelligence"**, published for public consultation by NAIS on **29 May 2026**, "fully aligned" with Reg. 2024/1689, including its prohibited practices | **Not yet; coming** | [Boga & Associates newsletter](https://www.bogalaw.com/pdf/Newsletter%20-%20The%20Draft%20Law%20On%20Artificial%20Intelligence%20in%20Albania.pdf) (snippet) |
| **GDPR** (Reg. 2016/679) | **No, for the venue**: Art. 3(2) reaches data subjects *who are in the Union*, and a tourist eating in Durrës is not. **Possibly yes for dowiz** if the dowiz operator is established in the EU (Art. 3(1)), even as a processor. **Operator decision D7** | [EUR-Lex ELI](https://eur-lex.europa.eu/eli/reg/2016/679/oj); Art. 22 text read on [gdpr-info.eu](https://gdpr-info.eu/art-22-gdpr/) |
| **ePrivacy Directive 2002/58, Art. 5(3)** and **EDPB Guidelines 2/2023 on its technical scope**, v2.0 adopted **7 Oct 2024**: storage of any duration or size counts, including JS/local storage | No (EU); best practice | [EDPB 2/2023 v2 PDF](https://edpb.europa.eu/system/files/2024-10/edpb_guidelines_202302_technical_scope_art_53_eprivacydirective_v2_en_0.pdf); [Hunton summary](https://www.hunton.com/privacy-and-cybersecurity-law-blog/edpb-adopts-guidelines-on-scope-of-eprivacy-directive) |
| **EU AI Act** (Reg. 2024/1689). **Art. 5 applies from 2 Feb 2025** (Art. 113(a)). Art. 5(1)(a) bans "subliminal … or purposefully manipulative or deceptive techniques" that materially distort behaviour and cause significant harm; 5(1)(b) bans exploiting vulnerabilities due to "age, disability or a specific social or economic situation" | No (Albania); the Albanian draft copies it | [artificialintelligenceact.eu Art. 5](https://artificialintelligenceact.eu/article/5/); [EUR-Lex ELI](https://eur-lex.europa.eu/eli/reg/2024/1689/oj) |
| **Commission Guidelines on the AI-system definition** (Feb 2025, non-binding): "systems based solely on rules defined by natural persons" and basic data processing fall **outside** the definition | best practice | [Commission](https://digital-strategy.ec.europa.eu/en/library/commission-publishes-guidelines-ai-system-definition-facilitate-first-ai-acts-rules-application); [Covington](https://www.insideprivacy.com/artificial-intelligence/european-commission-guidelines-on-the-definition-of-an-ai-system/) |
| **UCPD** 2005/29 + **Commission UCPD Guidance 2021/C 526/01** (17 Dec 2021): §4.2.7 covers data-driven personalisation and dark patterns | No (EU); Albanian Law 9902 has the misleading-practice core | [Commission UCPD page](https://commission.europa.eu/law/law-topic/consumer-protection-law/unfair-commercial-practices-and-price-indication/unfair-commercial-practices-directive_en); [Covington on dark patterns](https://www.insideprivacy.com/eu-data-protection/the-eu-stance-on-dark-patterns/) |
| **Consumer Rights Directive Art. 6(1)(ea)**, added by **Omnibus 2019/2161** (applies from 28 May 2022): the trader must say when "the price was personalised on the basis of automated decision-making". Recital 45: this does not cover dynamic pricing without personalisation | No (EU); best practice | [EUR-Lex 2019/2161](https://eur-lex.europa.eu/eli/dir/2019/2161/oj); [Addleshaw Goddard](https://www.addleshawgoddard.com/en/insights/insights-briefings/2020/competition/the-eu-omnibus-directive--time-to-prepare-for-strengthened-consumer-laws/) |
| **EDPB Guidelines 03/2022 on deceptive design patterns**, v2.0 adopted **14 Feb 2023** | best practice | [EDPB](https://www.edpb.europa.eu/our-work-tools/our-documents/guidelines/guidelines-032022-deceptive-design-patterns-social-media_en) |
| **DSA Art. 25**: "Providers of online platforms shall not design … their online interfaces in a way that deceives or manipulates"; 25(2) excludes practices already covered by the UCPD or GDPR. Section 3 (including Art. 25) does not apply to **micro and small** platforms (Art. 19; my reading of the Regulation's structure, VERIFY) | No (Albania); dowiz is a micro enterprise anyway | [DSA Art. 25 text](https://www.eu-digital-services-act.com/Digital_Services_Act_Article_25.html); [EUR-Lex ELI](https://eur-lex.europa.eu/eli/reg/2022/2065/oj) |
| **CJEU C-634/21 SCHUFA**, 7 Dec 2023: a score is itself an Art. 22 "decision" when a third party's conduct depends decisively on it | best practice | [Bird & Bird](https://twobirds.com/en/insights/2023/global/key-takeaways-from-the-schufa-case-of-the-cjeu) |
| **CJEU C-184/20 OT**, 1 Aug 2022: data that **indirectly** reveal a special category are Art. 9 data | best practice (Albania mirrors Art. 9) | [Covington: "special category data by inference"](https://www.insideprivacy.com/eu-data-protection/special-category-data-by-inference-cjeu-significantly-expands-the-scope-of-article-9-gdpr/) |
| **WP29 Guidelines on automated decisions (WP251rev.01)**: differential pricing can be "similarly significant" if "prohibitively high prices effectively bar someone" | best practice | [EC newsroom, WP251](https://ec.europa.eu/newsroom/article29/items/612053) (snippet) |
| **Reg. 1169/2011 (FIC)**. Art. 14(2): for non-prepacked food sold at a distance, the Art. 44 particulars must be **available before the purchase is concluded**. Art. 44(1)(a): allergen particulars (Art. 9(1)(c)) are **mandatory** for non-prepacked food | No (EU); Albania's VKM 434/2018 is the partial equivalent | [EUR-Lex ELI](https://eur-lex.europa.eu/eli/reg/2011/1169/oj) (the EUR-Lex page did not render for the fetcher; text read on the [UK mirror of Art. 14](https://www.legislation.gov.uk/eur/2011/1169/article/14) and [Art. 44](https://www.legislation.gov.uk/eur/2011/1169/article/44)) |
| **WP29 Opinion 2/2017 on data processing at work**: employee consent is "highly unlikely" to be a valid basis; monitoring must be necessary, proportionate and transparent | best practice (item d) | [SCL summary](https://www.scl.org/6946-data-processing-at-work-updated-guidance/) |
| **Platform Work Directive 2024/2831**, Art. 7: an absolute ban on automated processing of emotional state, private conversations and similar; transposition by **2 Dec 2026** | No; best practice for courier and staff analytics | [EU-OSHA](https://osha.europa.eu/en/legislation/directive/directive-20242831eu-platform-work); [Taylor Wessing](https://www.taylorwessing.com/en/insights-and-events/insights/2024/11/radar---eu-platform-work-directive-brings-in-new-protections-for-platform-professionals) |
| Albania's EU status: candidate since 2014; **all six negotiating clusters open as of 17 Nov 2025**; talks are expected to end by 2027 | context | [European Western Balkans, 2025-11-17](https://europeanwesternbalkans.com/2025/11/17/albania-opened-the-last-remaining-cluster-with-the-eu/) |

**The rule this lane recommends:** design to the **EU standard now**. Three reasons:
1. Albania's data law is already a GDPR mirror.
2. The AI law is a copy of the AI Act in consultation.
3. Accession is the declared direction.

Anything that would be illegal in the EU should be treated as **not built**. Exploiting the gap would buy a short window and a rewrite.

### 2.2 Allergens: may a menu HIDE a dish because of an inferred allergy?

**No.** There are four independent reasons.
1. **Information duty.** The FIC requires allergen information to be *available before purchase* (Art. 14(2) + 44(1)(a)). Hiding a dish neither informs nor guarantees anything. The duty is met by **showing** the three-state declaration, which the dish sheet already does (`dish.js:10-11`).
2. **Special-category data.** An inferred allergy is health data under Art. 9 (C-184/20 extends Art. 9 to inferences). Law 124 Art. 9 mirrors this. Processing it needs explicit consent, and a guess made from scroll or order behaviour is neither consented nor accurate.
3. **Safety.** An inference has false negatives. A guest who is shown a "filtered" menu will believe it is safe, which is exactly the failure `allergens.rs:3-9` exists to prevent. Hiding by inference converts a guess into a safety claim.
4. **Autonomy.** The guest may be ordering for someone else.

**What is lawful and already built:** filter at the guest's **explicit** choice (`dw_avoid`), hiding `contains` **and `undeclared`** (`state.js:292-327`). This is the strict reading of `allergens.rs`. A "swap" suggestion ("dishes without sesame in this category") is fine **under that explicit choice**. VERIFY-GREP that the filter is wired; `menu.js:21-22` says the opposite.

**Open defect:** the DG8 fixture counted 86 undeclared allergen declarations. While those exist, the explicit filter hides most of the menu for an allergic guest. The answer is to declare the allergens, not to filter more softly (row MR0).

### 2.3 Every proposed signal and nudge: verdict

Legend:
- **OK**: lawful in Albania today and under the EU standard.
- **OK-OPTIN**: lawful only with explicit, separate, withdrawable opt-in.
- **NOT OK**: refused, on law, ethics or invariant grounds.

| # | Signal or nudge | Verdict | Reasoning |
|---|---|---|---|
| S1 | Weather and temperature at the venue | **OK** | Not personal data. Same for everyone. |
| S2 | Time of day and weekday | **OK** | Already used for open/closed (`shell.js:79-93`). |
| S3 | Albanian public holidays | **OK** | A static list (§4.8). |
| S4 | Local events (football, concerts), entered by the owner | **OK** | Owner-entered facts. |
| S5 | Expiry or surplus driven "use first" placement | **OK, with conditions** | Waste reduction is legitimate. The label must be honest: "Chef's pick", never "fresh" or "popular". Only lots within their **use-by** date may be pushed, and never a raw-fish lot near its limit (safety; HACCP). The owner approves every suggestion. |
| S6 | Prep time (kitchen load) | **OK** | Already shown (`cookingMin`, `menu.js:101`). Raising quick dishes at peak is honest if labelled ("ready in 10 min"). |
| S7 | "Most ordered **today**" | **NOT OK as worded** | At this volume the claim is noise, and a statement presented as fact must be true (Law 9902 misleading practices; UCPD art. 6). Replace it with S7′. |
| S7′ | "Most ordered **this week** at {venue}: N times" | **OK** | Computed by the fold over 7 days, excluding cancelled and test orders, shown only when N ≥ 10 (GUESS threshold, owner-tunable), with the number visible. The live probe checks it against the analytics fold. |
| S8 | The venue's own `popular` tag (exists) | **NOT OK as is** | It is an owner-typed **popularity claim** with no measurement behind it (`menu.js:112`). Either rename it to "Venue's pick" in all four languages or derive it from S7′ (row MR0, decision D5). |
| S9 | Scroll velocity, dwell on sections | **NOT OK** | Behavioural micro-tracking. Sending it out is profiling and needs consent (Law 124 Art. 7/8; ePrivacy 5(3) under EDPB 2/2023). Keeping it on the device is legally grey, but it buys nothing measurable (§3) and it is surveillance by design. Refused on ethics and minimisation grounds. |
| S10 | "Hesitation" between two dishes triggers a nudge | **NOT OK** | It targets a moment of indecision to change the choice: emotional steering, which the EDPB 03/2022 catalogue and the UCPD guidance treat as a dark pattern. Under the AI Act it is not clearly prohibited for food, because the bar is "significant harm", but a hesitation trigger attached to a **price** is S12. |
| S11 | "All the information that can be collected" | **NOT OK** | It is the negation of data minimisation and purpose limitation (GDPR Art. 5(1)(b)(c); Law 124's principles article, number unconfirmed). It is also the negation of `personal-data.sh`, which exists to make every store justify itself. |
| S12 | A per-person price or bonus from the profile | **NOT OK** | Personalised pricing on willingness to pay. EU: must be disclosed (CRD 6(1)(ea)). It can be an Art. 22 "similarly significant" effect (WP251). It **scores a participant**: the invariant (§2.4). Albania: risks Law 9902 and discrimination complaints. A **uniform** rule ("everyone gets the 10th roll free", the existing stamp card) is fine. |
| S13 | A uniform promotion visible to everyone (happy hour, event pack price) | **OK** | Not personalised. The price shown is the price charged (kernel re-derivation, `shell.js:20-22`). |
| S14 | Badges ("chef's pick", "vegetarian", "spicy") | **OK if true** | Recipe-derived tags are exact (W-DEPTH U9). |
| S15 | Auto-generated event packs | **OK if owner-approved** | A pack is a menu item with a price and must pass the money law. "Auto-generated" means *suggested to the owner*, never published unseen. |
| S16 | Hide or swap dishes by **inferred** allergy | **NOT OK** | §2.2. |
| S17 | Filter by allergens the **guest chose** | **OK** | Exists (`dw_avoid`). The choice stays on the device. |
| S18 | Guest history and taste vector, **on device only**, opt-in, wipeable, never sent | **OK-OPTIN** | Arguably exempt from ePrivacy 5(3) as "strictly necessary for a service explicitly requested", but treated as opt-in anyway. It can reveal religion or health (always vegetarian, never pork), so it is never sent and it is shown to the guest ("what this phone remembers"). |
| S19 | Guest history and taste vector, **server-side** | **OK-OPTIN legally, but NOT recommended** | Needs explicit consent separate from marketing, the right to object (Law 124 Art. 19/46(4)), a registry row, an erasure path, and probably a DPIA (Law 124 DPIA articles, some delayed to 2027). It creates the profile the invariant warns against and adds nothing that S18 does not (§3). |
| S20 | Countdown timers, "only 2 left!", fake scarcity | **NOT OK unless literally true** | UCPD Annex I no. 7 (false limited availability) and misleading-action rules. "Sold out" is already shown truthfully from stock (`menu.js:111`). |

### 2.4 The "no scoring of any participant" invariant

The invariant is "trust is a signed capability, never a score" (`CLAUDE.md`; `no-scoring.sh:1-45`). The gate's own header names what makes something a score: "the **direction of the gaze**" (`no-scoring.sh:17-20`).

- **A per-guest taste vector *on the device*.** It answers "which dish suits this person", so the gaze is on **dishes**. Nothing about the person is computed where anyone else can see it, compared with anyone, or used to grant or deny anything. It **does not violate** the invariant under four conditions:
  1. it never leaves the device;
  2. it never sets a price or eligibility;
  3. it never produces a class, segment or tier of guests;
  4. it can only reorder within the "For today" strip, never hide.
- **The same vector *on the server*, keyed by phone.** It becomes a property of a person that the venue can read and act on: the first step to "VIP", "big spender" or "price-insensitive". **Violates the spirit.** The gate would not catch it (names like `guest_taste`, `customer_affinity`, `propensity`, `ltv` are not in its pattern), hence row GATE-1.
- **A staff anomaly KPI, "anonymised".** A pseudonym is still a person (GDPR recital 26), and a per-person anomaly rate is a score whatever it is called. **Violates.** The compliant version already exists:
  - venue-level counts per period (`alert.rs:5-9`);
  - signed rows that name who did each act, which is accountability for an act and not a rating of a person (`fold.rs:3-8`).

  Extend it (§5d); do not personalise it.

---

## 3. Evidence that ranking and personalisation work

### 3.1 What is published

| Study or source | Setting | Effect size | Quality note |
|---|---|---|---|
| **Cai, Chen, Fang 2009**, "Observational Learning: Evidence from a Randomized Natural Field Experiment", *AER* 99(3):864-882 | Restaurant chain in Beijing, 2 weeks, Oct 2006 | Showing the **top-5 most popular dishes** raised their demand by **13-20 %**. No significant pure-saliency effect. Stronger for infrequent customers. Satisfaction up | Randomised field experiment. The best evidence for a **truthful** "most ordered" badge. [NBER w13516](https://www.nber.org/papers/w13516); [RePEc](https://econpapers.repec.org/article/aeaaecrev/v_3a99_3ay_3a2009_3ai_3a3_3ap_3a864-82.htm) |
| **Dayan & Bar-Hillel 2011**, "Nudge to nobesity II: Menu positions influence food orders", *JDM* 6(4):333-342 | Lab and a real café | Items **first or last** in their category were **up to twice as popular** as the same item in the middle | Field component, small. [JDM](https://dlab.sauder.ubc.ca/sjdm/journal/11/11407/jdm11407.html) |
| **Yang 2012**, eye movements on menus, *IJHM* | Eye-tracking, mock menus | People read menus **sequentially, like a book**. **No "sweet spot"**; a "sour spot" exists | Refutes the classic menu-engineering placement folklore. [NRN summary](https://www.nrn.com/menu-trends/study-counters-prevailing-restaurant-menu-theories) |
| **Cadario & Chandon 2020**, meta-analysis of 96 field experiments (299 effects), *Marketing Science* | Healthy-eating nudges | Cohen's **d = 0.23** overall; cognitive 0.12, affective 0.24, behavioural 0.39 | [INSEAD Knowledge](https://knowledge.insead.edu/marketing/which-healthy-eating-nudges-work-best) |
| **Maier et al. 2022**, *PNAS*, "No evidence for nudging after adjusting for publication bias" | Re-analysis of the Mertens et al. nudge meta-analysis | **No evidence** of an overall nudge effect after a robust Bayesian correction for publication bias | A strong reason to treat any promised uplift as a hypothesis. [PMC9351501](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC9351501/) |
| Wansink lab (Cornell) | Menu labels, portions | Widely quoted figures (e.g. "+27 % from descriptive labels") come from this lab | **17 retractions; misconduct finding of 27 Sep 2018.** Do not cite its numbers. [Retraction Watch](https://retractionwatch.com/2018/09/19/jama-journals-retract-six-papers-by-food-marketing-researcher-brian-wansink) |
| **DoorDash** engineering, 2018 | Store feed with embeddings | "+25 % click-through vs a most-popular baseline" for earlier personalisation work | Snippet only: the blog returned HTTP 403. Store-level ranking, not dishes inside one venue. [DoorDash careers blog](https://careersatdoordash.com/blog/personalized-store-feed-with-vector-embeddings/) |
| **Uber Eats** engineering, 10 Sep 2018 | Restaurant recommendations | "significant business metric lifts", **no numbers published** | [Uber blog](https://www.uber.com/blog/uber-eats-recommending-marketplace/) |
| **Wolt**, Algorithmic Transparency (operations as of Feb 2024) | Venue ranking | Ranking by distance, opening hours, time of day, impressions and conversion per city, collaborative filtering with random IDs. First-time users get **non-personalised** ranking. **No opt-out documented. No uplift numbers** | [press.wolt.com](https://press.wolt.com/en-WW/237306-algorithmic-transparency-consumers/) |
| **Lu, Ge, Mao 2024** (HBR, "When Gig Work Meets Extreme Weather") | One Chinese delivery platform | Heat waves raise **order volume** (snippet: +9.1 % orders/hour in a heat wave; couriers work 6 % longer) | Demand **volume**, not which dishes. [Tongji SEM](https://sem.tongji.edu.cn/semen/25714.html); [EBSCO record](https://www.ebsco.com/articles/earth-and-atmospheric-sciences/1ce7f88f-6bd7-5095-8ade-0e916d4ad90e/when-gig-work-meets-extreme-weather) |
| Weather and online food ordering (*Kybernetes* 2020, K-05-2020-0322) | Online catering orders | Orders rise with rainfall and temperature (snippet) | [DOI via Unpaywall](https://unpaywall.org/10.1108%2FK-05-2020-0322) (not read in full) |

**What the evidence supports:**
- A **truthful popularity signal** works: 13-20 %, randomised.
- **Position** works: up to 2×, but owners already control position through `sortOrder`.
- **Weather changes order volume.** No study found shows that weather-driven *re-ranking of dishes inside one venue* raises revenue. **GUESS**: soups up in rain and cold drinks up in heat is plausible, but unmeasured for sushi in Durrës.
- Platform personalisation works **at platform scale**, across stores. None of the big platforms publishes a number for in-venue dish ranking.

### 3.2 What one small venue can and cannot learn (CALC)

The sample-size rule for α = 0.05 (two-sided) and power 0.8 is n ≈ 16σ²/δ² per arm ([Kohavi rule of thumb, via arXiv 2305.16459](https://ar5iv.labs.arxiv.org/html/2305.16459)). Volume assumed: 20-60 orders/day (GUESS, W-DEPTH §2.3).

| Metric | Baseline | Effect to detect | Orders per arm | Total (50/50) | Days at 40/day | Days at 20/day |
|---|---|---|---|---|---|---|
| Featured dish attach rate | 10 % of orders | +15 % relative (Cai-sized), δ = 1.5 pp | 16·0.09/0.000225 = **6 400** | 12 800 | **320** | 640 |
| Featured dish attach rate | 10 % | +30 % relative, δ = 3 pp | 1 600 | 3 200 | 80 | 160 |
| Average order value | CV 0.6 (GUESS) | +10 % | 16·0.36/0.01 = 576 | 1 152 | **29** | 58 |
| Average order value | CV 0.6 | +5 % | 2 304 | 4 608 | 115 | 230 |
| Visit → order conversion | — | — | **not measurable** without counting visits, which BN2 removed from the Worker. A beacon would cost requests and needs ePrivacy consent | — | — | — |

**Honest consequences:**
1. A single venue can detect only **large** effects (≥ 10 % AOV within about a month, or ≥ 30 % on one dish within a quarter).
2. A 10 % holdout (90/10) needs about 5× longer than 50/50 for the same power. At one venue it is **impractical**: use **50/50 alternating per page load**, not per person (no identifier needed).
3. Order-level metrics are conditional on ordering. If the strip changes *who* orders, AOV can move for reasons unrelated to the dishes. The report must say so.
4. Seasonality (summer tourists in Durrës) dwarfs these effects. Arms must run **concurrently**, never "before and after".
5. Pooling across venues is the only way to real power. With 3 live venues it is not available.
6. **Cold start:**
   - Layer 0 needs **no data**: rules plus weather.
   - "Most ordered this week" needs a week of orders and the N ≥ 10 threshold.
   - Expiry suggestions need recipes and lots (0 today).
   - Layer 1 needs ≥ 3 past orders on that phone before it changes anything (GUESS threshold).

---

## 4. Design: the honest version

### 4.1 Shape: an additive strip, not a reshuffle

`menu.js:210-211` encodes a product decision worth keeping: the venue's order is deliberate and is not overruled. The ranking therefore produces **a "For today" strip above the categories**:
- at most 6 dishes;
- each with a **reason chip**;
- a sort option "For today" beside `pop/low/high/az`, which the guest chooses.

The categories below stay in the venue's own order. Three consequences:
- Nothing is ever hidden by ranking (only by the guest's explicit allergen choice).
- The guest always sees why ("why am I seeing this").
- One tap returns to the venue's order, and the choice is remembered on the device (`dw_rank_off`, a registry row).

### 4.2 Layer 0: context-only, the same for everyone, zero personal data

**Data flow.**

```
                     (nightly 03:17 UTC cron, per venue, in the venue's runner)
 api.met.no ──1 GET──► forecast for venue lat/lon (48 h hourly)
 static holiday list (owner-editable) ─┐
 owner events (console)               ─┼─► dowiz_hub::rank::context(rules, dishes, facts) [PURE]
 analytics fold: 7-day dish counts     ─┤        │
 stock fold: lots expiring ≤ 2 d,      ─┘        ▼
   approved by owner ("use first")       context.json  { v:1, hours:[...48], picks:[...], rules_v }
                                                 │  content-addressed, named in manifest.json (BN2)
                                                 ▼
                         cdn.dowiz.org/v/<slug>/<k64>.json  (immutable)  +  manifest (max-age 30)
                                                 │
 guest browser: shell.js reads manifest → context → picks the CURRENT hour by venue clock
               → evaluates the published rule outcomes for that hour → renders the strip
```

- **Weather is fetched by the Worker once a day, never by the guest's browser.** MET's terms discourage direct browser use and require caching. The guest's IP reaches no third party.
- **The rule evaluation is in Rust (`dowiz-hub`, pure, tested)** at publish time, producing per-hour **outcomes**: for each of the 48 hours, the ≤ 6 dish ids and their reason codes. The browser only *selects the hour*. It does no ranking logic of its own beyond that, so the logic stays single-sourced in the kernel side (MANIFESTO C1/C2: deterministic, no float).
- **A rule** is owner-authored and closed-vocabulary. It is not an AI system under the Commission's definition guidelines, which matters once the Albanian AI law lands. The shape is: `WHEN <condition> THEN raise <category|tag|dish> [LIMIT n]`.
  - Conditions: `rain ≥ x mm/h`, `temp ≥ / ≤ t°C`, `hour in [a,b)`, `weekday in {…}`, `holiday`, `event(name)`, `prep ≤ m min`.
  - Built-in sources: `chef_pick` (owner-approved expiry suggestions), `top_week(N≥10)`.
- **Integers only**, MANIFESTO C2: temperature in tenths of °C, precipitation in tenths of mm.
- **Fail-open:** context missing, stale (> 36 h) or malformed → no strip, the venue's order, no error. This mirrors `shell.js:11-14`.

**CPU and requests on Workers Free:**

| Step | Cost | Basis |
|---|---|---|
| MET fetch | 1 subrequest per venue per day | Nightly runner, already one invocation per venue |
| Parse MET compact JSON (~20-40 KB, GUESS) | ≈ 0.25-0.5 ms CPU | CALC from the MEASURED 6.33 ms per 538 KB JSON parse (W-BOTTLENECK `:26-27`) ≈ 12 µs/KB. GUESS until BN8 measures it |
| Fold 7-day counts + expiring lots + rules × 48 hours × 165 dishes | ≈ 0.1-1 ms (GUESS) | The 7-day fold already runs for analytics. The rules pass is about 48 × 3 300 checks |
| R2 writes | 1-2 per venue per day (context + manifest) | Against 1 M Class A/month free (`publish.rs:13-14`) |
| Guest visit | **0 Worker requests, 0 DO requests**; 1 CDN read (immutable, cached) | BN2 path |
| Browser | Selecting the hour and reordering ≤ 6 nodes: well under 1 ms | Same mechanism as `applyFilters` |

The nightly runner is a separate invocation per venue, so the 10 ms is per venue. **Measure it with BN8** before turning the feature on anywhere. If the fold is too heavy, split it into two runs (facts first, then rules).

### 4.3 Layer 1: on-device taste memory (opt-in)

- **Storage:** IndexedDB key `dowiz.taste.v1` per venue slug. It needs a `BROWSER` registry row; the gate forces this.
- **Content:**
  - counts per **tag** and **category** of dishes the guest actually ordered on this phone, from the order confirmation they already receive;
  - exponential decay with a 60-day half-life (GUESS);
  - no timestamps finer than a day;
  - no dish-view, scroll or dwell data, ever.
- **Effect:** it may reorder **within** the Layer 0 strip and add at most 2 dishes the guest has ordered before ("Again?"). It never hides anything and never touches a price.
- **Opt-in:**
  - an unticked toggle in the storefront's settings sheet: "Remember what I like on this phone";
  - default OFF;
  - offered after the second completed order, never before.
  - The pattern is `consent.js`: never pre-ticked, never remembered as ticked when off.
- **Transparency:** "What this phone remembers" lists the tag counts in plain words. **Forget** wipes the key in one tap.
- **Never sent.** Two tests enforce this:
  1. a JS test asserts no `fetch`/`sendBeacon` body contains the key's content;
  2. a gate refuses `navigator.sendBeacon` and visibility/scroll-to-network code under `public/store/` (row GATE-2).
- **Cost:** 0 Worker, 0 DO; a few KB of IndexedDB.
- **Data-protection note:** the counts can reveal religion or health. Because they never reach the controller, the processing stays under the guest's control. The privacy notice still says it exists (the registry row drives the notice).

### 4.4 Layer 2: server-side profile, with consent — recommended NOT to build

The evidence (§3) gives no reason to expect it to beat Layer 1 at a single venue. It adds:
- a profile store;
- a consent purpose;
- an erasure and export path;
- a DPIA;
- a new attack surface (the red team of 2026-09-21 found three public read+write families);
- a pull toward scoring.

**Re-entry condition:** ≥ 20 venues pooled, the operator decides, a DPIA is written, and the "never a price, never a tier" rules are encoded as gates first.

### 4.5 UI (four languages; sq and ru are drafts to be reviewed by a native speaker)

**Owner console** (Settings → Menu → "For today"). It is one screen, kept "as simple as possible":
- **Switch:** "Show a 'For today' strip" (default OFF).
- **Rules list:** each rule shows a plain-language sentence and a toggle. Starter rules are offered, never auto-enabled: "Rain → soups and hot dishes", "Above 28 °C → cold drinks", "Lunch 12-15 → sets", "Most ordered this week".
- **Use first** (from expiring lots, when recipes exist): suggestions with **Approve / Skip**.
- **Preview:** a time slider over the next 48 h shows the strip as a guest would see it, with "why is this dish first" listing the rule and the fact (e.g. "rain 1.2 mm/h at 13:00, MET Norway").
- **Measurement card** (§4.7).

**Guest storefront:**
- strip title;
- reason chips;
- an "i" that opens the "Why this order?" sheet;
- "Show the venue's order" (turns it off on this phone);
- Layer 1 toggle and Forget.

| key | sq | en | uk | ru |
|---|---|---|---|---|
| rk_title | Për sot | For today | Для сьогодні | На сегодня |
| rk_why | Pse kjo renditje? | Why this order? | Чому такий порядок? | Почему такой порядок? |
| rk_whyBody | Renditur sipas motit, orës dhe zgjedhjes së {venue}. E njëjtë për të gjithë. | Arranged by the weather, the time and {venue}'s choices. The same for everyone. | Підібрано за погодою, часом і вибором {venue}. Однаково для всіх. | Подобрано по погоде, времени и выбору {venue}. Одинаково для всех. |
| rk_off | Shfaq renditjen e lokalit | Show the venue's order | Показати порядок закладу | Показать порядок заведения |
| rk_rain | Bie shi | It's raining | Дощ | Дождь |
| rk_hot | Ditë e nxehtë | Hot day | Спекотно | Жарко |
| rk_chef | Zgjedhja e kuzhinierit | Chef's pick | Шеф радить | Шеф советует |
| rk_top | Më e porositura këtë javë: {n} | Most ordered this week: {n} | Найчастіше за тиждень: {n} | Чаще всего за неделю: {n} |
| rk_quick | Gati për {m} min | Ready in {m} min | Готово за {m} хв | Готово за {m} мин |
| rk_mem | Mbaj mend çfarë më pëlqen në këtë telefon | Remember what I like on this phone | Пам'ятати мої смаки на цьому телефоні | Запоминать мои вкусы на этом телефоне |
| rk_memWhat | Çfarë mban mend ky telefon | What this phone remembers | Що пам'ятає цей телефон | Что помнит этот телефон |
| rk_forget | Harro | Forget | Забути | Забыть |
| rk_ownerSwitch | Shfaq shiritin "Për sot" | Show a "For today" strip | Показувати смугу «Для сьогодні» | Показывать полосу «На сегодня» |
| rk_ownerWhyFirst | Pse kjo pjatë është e para | Why this dish is first | Чому ця страва перша | Почему это блюдо первое |
| rk_attrib | Moti: MET Norway (CC BY 4.0) | Weather: MET Norway (CC BY 4.0) | Погода: MET Norway (CC BY 4.0) | Погода: MET Norway (CC BY 4.0) |

ASCII quotes only inside JS strings (COMMON-RULES 11). The table uses typographic quotes for reading only.

### 4.6 The per-venue switch

- Setting `feature.menuRank`, **default OFF**.
- **Caveat:** `state.js:157-161` treats an absent flag as ON. The storefront must therefore read this flag as opt-in, `features.menuRank === true`, through a new helper `onOptIn(name)`. A test pins that an absent `features` block shows no strip.
- When the switch is OFF, the publish writes no `context` and the shell renders exactly as today.
- Layer 1 additionally requires the guest's own opt-in.

### 4.7 Measurement: the holdout and honest uplift reporting

- **Arm assignment:** each **page load** draws `ctx` or `base` 50/50 in the browser (`crypto.getRandomValues`; client JS, not the kernel). It is not stored and carries no identifier.
- **The order body** carries `menuView: { v:1, arm:"ctx"|"base", rules_v:int }`. That is all; no list of dishes seen. This is order content, already covered by the `OrderContent` registry rows. VERIFY that the strict-body gate (`crate::body`, `deny_unknown_fields`) gets the field added on purpose.
- **The fold** (pure, beside `fold.rs`) gives per arm: orders, AOV, and the attach rate of dishes that were in the strip in that hour (recomputable from `context` + `rules_v`).
- **The report** is in plain words and **never claims a win without power**:
  - "ctx: 412 orders, AOV 1 830 L; base: 398 orders, AOV 1 790 L; difference +2.2 % (95 % CI −4.1 % … +8.5 %). Not enough orders to tell. About 740 more orders are needed to detect a 5 % change." (CALC, as in §3.2)
  - When the interval excludes 0, it says "likely", never "proved".
  - It always prints the arms' sizes.
  - It warns when the arms ran in different seasons.
- **The owner can stop the experiment** and serve `ctx` to everyone. The report then stops claiming anything ("no comparison running").

### 4.8 Weather, holidays, events: sources and terms

| Source | Terms | Use |
|---|---|---|
| **MET Norway Locationforecast** (`api.met.no`) | **CC BY 4.0**, attribution required; **identifying User-Agent mandatory** (app plus contact); ≤ 20 req/s; **must cache**, honour `Expires`, use `If-Modified-Since`; direct browser connections discouraged; commercial use not prohibited | **Primary.** One request per venue per day from the nightly runner. [MET terms](https://api.met.no/doc/TermsOfService) |
| **Open-Meteo** | Free API **non-commercial only** (< 10 000 calls/day). "Integrating our service into commercial products" is commercial. Paid "API Standard" plan; **$29/month** for 1 M calls per a search snippet. Data CC BY 4.0 | **Fallback only if the operator pays** (decision D4). [Open-Meteo terms](https://open-meteo.com/en/terms); [pricing](https://open-meteo.com/en/pricing) |
| **Holidays** | Nager.Date `/api/v3/PublicHolidays/2026/AL` (MEASURED: it answered with 14 rows) **omits both Bajrams** and Alphabet Day, which other lists include ([qppstudio](https://www.qppstudio.net/publicholidays2026/albania.htm), [Nager](https://date.nager.at/api/v3/PublicHolidays/2026/AL)). The Bajram dates depend on the lunar calendar and are announced yearly, and substitute days are decided by the Council of Ministers | A **static list in the tree** (`dowiz-hub`), seeded from the official decision, **editable by the owner** ("add or move a holiday"), and marked "check each year". No API dependency |
| **Football** | API-Football: free tier of 100 requests/day covers the Albanian Superliga (league 442, per a third-party summary); needs an account key | **Not recommended as a platform dependency.** It is a key, a quota and a third party for a nudge with no measured value. If the operator wants it, it follows the OpenRouter precedent: **the owner's own key** (decision D6). [API-Football guide](https://www.api-football.com/news/post/how-to-get-started-with-api-football-the-complete-beginners-guide); [thestatsapi](https://thestatsapi.com/blog/free-football-api-alternatives) |
| **Concerts and local events in Durrës/Tirana** | No free, reliable, legal API was found. Ticketing sites have no public API; scraping them is fragile and may breach their terms (GUESS) | **The owner adds events by hand**: name, date and time, optional pack. This is the honest answer |

### 4.9 Contracts and live probes (operator rule of 2026-10-03)

```
LINK rank.weather.upstream     producer: api.met.no  version: locationforecast/2.0 "compact"
  GET https://api.met.no/weatherapi/locationforecast/2.0/compact?lat=41.315&lon=19.445
      headers: User-Agent "dowiz.org/<ver> ops@<domain>" (MANDATORY), If-Modified-Since
  → 200 { type:"Feature", geometry:{coordinates:[lon,lat,alt]},
          properties:{ meta:{updated_at, units:{air_temperature:"celsius", precipitation_amount:"mm"}},
                       timeseries:[{ time:RFC3339, data:{ instant:{details:{air_temperature:number}},
                                     next_1_hours?:{ summary:{symbol_code}, details:{precipitation_amount} } } }] } }
     | 304 (use cache) | 203 (deprecated version: FAIL LOUD)
  terms: CC BY 4.0 attribution shown on storefront; cache per Expires
LINK rank.context (ours)       producer: nightly runner → R2   consumer: store/shell.js
  R2 v/<slug>/<k64>.json, named in manifest.json as "context"
  schema v1: { v:1, slug, day:"YYYY-MM-DD", tz:"Europe/Tirane", rules_v:int, source:{weather:"met.no", updated_at},
               hours:[ { h:"YYYY-MM-DDTHH", t10:int, p10:int, sym:str,
                         strip:[ { id:str, why:"rain"|"hot"|"cold"|"chef"|"top"|"quick"|"holiday"|"event"|"rule", n?:int } ] } ] }
  invariants: |strip| ≤ 6; every id is an available dish in the same generation's fragment;
              why=="top" ⇒ n ≥ threshold && n == fold7(id); no field names a person
LINK rank.rules (ours)         PUT /api/owner/menu/rank (owner, Place::of_authorised)
  req { enabled:bool, rules:[ { id, when:{ rain_ge10?, temp_ge10?, temp_le10?, hours?:[a,b], weekdays?:[0..6], holiday?:bool, event?:str, prep_le?:int },
                                 raise:{ category?|tag?|dish? }, limit:1..6, on:bool } ], top_threshold:int≥5 }
  resp 200 { rules_v:int, preview:{ hours:[...as above, next 48 h] } }
LINK rank.measure (ours)       order body field  menuView:{ v:1, arm:"ctx"|"base", rules_v:int }
  GET /api/owner/menu/rank/report?days=7|30 → { arms:{ ctx:{orders, aov, attach_pm}, base:{...} },
       diff_pm:int, ci_pm:[lo,hi], needed_orders:int|null, verdict:"too_few"|"likely_up"|"likely_down"|"no_difference" }
LINK rank.holidays (ours)      static table dowiz-hub::holidays::AL  + owner overrides in settings `rank.holidays`
```

**Live probes on `qa-durres.dowiz.org`** (never a real venue):
1. Set rules through the console API; read the preview back.
2. Trigger the nightly runner for qa-durres only; read the manifest → `context` from `cdn.dowiz.org`; validate it against schema v1; check that `top` counts equal the analytics fold.
3. Assert MET was called with the User-Agent (the runner logs the header it sent) and that a second run inside `Expires` sends `If-Modified-Since`.
4. Load the storefront headless; the strip renders with reason chips; "Show the venue's order" restores `pop`.
5. Place an order with `menuView` and read it back in the report.
6. Turn the switch off; the next publish has no `context` and the storefront shows no strip.
7. Clean up: delete the rules and remove the probe order through the existing cleanup path.

---

## 5. Items (a)-(d)

### (a) Offline-first: kitchen tablet and owner phone

**State in the tree:**
- Courier offline write queue `dowiz.outbox` with idempotency guards (ROADMAP §2 "Offline writes"); waiter room outbox `dowiz.room.outbox`.
- Admin read replica `dowiz.replica.v1`, which holds Name, Phone and Address (`registry/outside.rs:162`).
- BN3/BN3A device block replica: not on main (§1.1).
- DO = **single writer per venue**.
- Architecture blueprint verdict: **CRDT AGAINST**, except when "a second writer appears that cannot share an object — an offline POS till" (memory `dowiz-architecture-evolution-2026-09-22`; `DECISIONS.md` STATUS line, CRDT fence still in force).
- PRRO row 19 (Ukraine 36 h offline) is queued; Albania's equivalent is the 48 h rule.

**Is a CRDT needed?** **No.** Every write on the tablet is an *intent* that the DO either accepts or refuses (FSM, money law), so the right model is:
- a queue of intents with idempotency keys (already built for the courier and the room);
- plus a read replica.

A CRDT would let two offline devices both "win", which the money/order fence forbids.

**Can a kitchen run without the Worker?**

| Failure | What happens today | Honest design |
|---|---|---|
| **The venue's internet is down; the Worker and DO are up** | Online customers **keep ordering**; the kitchen tablet does not see them. The Telegram bell goes to the kitchen chat, which works if the phone is on mobile data (GUESS: VERIFY-GREP whether an unacknowledged-order alarm exists) | **Kitchen heartbeat + auto-pause (row OF1).** The DO records the last console contact. During opening hours, if an order stays unacknowledged for M minutes (default 5) **and** no kitchen contact happened in K minutes, it sends a Telegram alert to the venue's own chat (existing outbox), then sets the existing `deliveryPaused`, so the storefront shows "paused" (`shell.js:86-91`) instead of taking orders nobody sees. One alarm per pending order on the existing timer. This is the highest-value resilience row |
| **Power cut** | Tablet on battery; router dead | A **UPS for the router** (≈ €30-60, GUESS) plus the phone's hotspot. It is an ops recommendation, not code, and the cheapest real fix |
| **Walk-in cash sale with no network** | Not possible (memory: "full offline sale … absent today") | **Row OF3 = the PRRO row adapted to Albania:** a local sale intent queued with an idempotency key; a paper receipt without NIVF; synced and **fiscalised within 48 h** (Law 87/2019 rule above). It stays OFF while `SEND_ENABLED=false`. **Decision D8.** Stock is drawn when the intent reaches the DO, not before |
| **Card payment offline** | Impossible by nature: Stripe needs the network | Cash only while offline; the UI says so |
| **Cloudflare outage** | Everything stops | Rare; out of scope. The device replica keeps menus readable (BN3) |

**AX7 "shred before replicate":** the admin replica carries Name/Phone/Address to the device (`registry/outside.rs:162`). An offline kitchen tablet needs the **ticket**: first name and initial, items, notes. It does not need phone or full address unless the order is a delivery. That is the same cut the Telegram ticket already makes (`registry/outside.rs:31`). Row OF2 applies that cut to the kitchen replica.

**Risks:**
- an auto-pause that fires falsely during a quiet hour (mitigated by "unacknowledged order AND no contact");
- alarm DO requests against the 100k/day cap (one per order, small);
- staff learning a fallback they rarely use (a lesson in `dw_learn_`).

### (b) Omnichannel aggregator bridge

Settled by `docs/research/2026-10-03-external-deps-and-aggregator.md` §4; not redone here. Summary:
- **Wolt** is the only international aggregator in Albania (7 cities, including Durrës and Tirana).
- **Glovo** suspended its Albanian launch in April 2024.
- **Bolt Food** is absent.
- **Baboon** (local) has no API.
- Today dowiz has **manual entry** only (`command/aggregator.rs`, `admin/aggregator.js`); there is no Wolt adapter or mock.
- The real path:
  - dowiz applies to Wolt as a POS provider;
  - OAuth 2.0 self-service onboarding;
  - an HMAC webhook into a durable DO inbox;
  - the ALL minor-unit question to be settled in the sandbox.
- **Operator action:** submit the Wolt POS integration request (that doc §0).

The only link to the dynamic menu: a Wolt order counts in the 7-day "most ordered" fold. It must not double-count manual re-entries (the id is `<channel>-<external_id>`, so it is idempotent).

### (c) Price-drift watchdog: supplier price → recipe cost → menu price, with a margin floor

**State.** Every number needed is computed already:
- supplier price points and `changePm`;
- `portionCost`;
- `foodCostPm` per dish;
- `margin`.

(`kitchen/report.rs:62-67, 106-118`.) What is missing:
1. A **floor** setting per venue (and optionally per category): "food cost above X ‰" or "margin per portion below Y L".
2. An **alert on crossing** through the existing Telegram tell (`stock/tell.rs`, as `stock.low` does), once per crossing (the `alert.rs` level pattern).
3. A console line on the dish: "cost rose 12 % since 1 Sep; margin 38 % → 31 %; floor 35 %".

**Dependence.** It is silent until the venue has **recipes** (0 live) and **received prices**. The prices come from:
- manual receipts (exist);
- invoice OCR (W-DEPTH **P10**: tesseract.js `sqi`, deterministic parser, supplier aliases; approved 10-03);
- the Albanian e-invoice (P11).

It also depends on P1 (the starter pack and recipe skeletons) to have anything to compute.

**Design.**
- A pure check `margin_floor(dish_rows, floor)` beside `report.rs`.
- Run it at each goods receipt (the price changed) and nightly. It reports `uncosted` dishes rather than staying silent ("12 dishes have no recipe: no margin check").

**Risk:** an alert that fires on bad data (a mistyped price). Every alert therefore shows the receipt row it came from (traceability, W-DEPTH §1.4 point 7).

**Size:** S (1-2 days), after P1/P10.

### (d) Micro-KPIs and cash-abuse anomaly detection

**State.** The venue-level exception register (§1.1) already has seven kinds and an owner-thresholded Telegram alert, with **no per-person totals by construction** (`fold.rs:3-8`, `alert.rs:5-9`).

**Gaps:**
1. **"Cancelled after the bill or receipt was printed."** No such kind is in the list. Whether a bill-print event exists in the room log needs checking (VERIFY-GREP `bill` / `print` in `command/room*`). If it does, add `cancel_after_bill` and `discount_after_bill`.
2. **Manual discounts** outside `comp`: `discount_manual`, if staff can apply one (VERIFY).
3. A **weekly venue digest**: counts per kind against the venue's own previous 4 weeks ("voids after kitchen: 7 this week, usual 1-3"). The comparison is the venue to itself, never a person to a person.
4. **Platform-anonymised benchmarks across venues:** not possible with 3 venues (W-DEPTH §1.3) and not attempted.

**"Anonymised for staff" is rejected.** A per-person anomaly rate under a pseudonym is still personal data and still a score (§2.4). The rows already carry `by`: a signed act, which is accountability. That is the line OD-8 draws, and it stays.

**Labour-law angle:**
- Staff are data subjects (`Subject::Staff` and `Data::StaffId` exist in the registry).
- The basis is **legitimate interest or legal obligation** (the till and fiscal records), **not consent**: employee consent is "highly unlikely" to be free (WP29 2/2017).
- The staff privacy notice must say that exception rows name who signed and who receives the alert (the owner's chat).
- Retention follows the accounting records.
- There must be **no automated consequence**: an alert never suspends a login or deducts pay. A human reads it.
- Best practice from the Platform Work Directive Art. 7: no emotional or behavioural inference about staff.
- Albanian Labour Code specifics were **not verified** in this lane. That is a question for local counsel, not a code row.

---

## 6. Plan: lane-sized rows

Effort: XS ≤ ½ day, S ≤ 2 days, M ≤ 1 week (GUESS, lane-days on Opus).

Every row carries:
- a **contract** `tools/live-proof/contracts/<id>.json` (schema draft 2020-12; W-LIVE harness);
- a **live probe** on qa-durres;
- UI in **sq/en/uk/ru**;
- a **registry row** for any new store, key or host;
- RED→GREEN tests;
- the 10 ms / DO-cap budget measured with BN8.

**[DEC]** marks a row that needs an operator decision, not data. No row depends on the operator supplying photos, suppliers or raters.

| # | Row | Value ÷ effort ÷ risk | Files | Acceptance | Live probe (qa-durres) | Contract |
|---|---|---|---|---|---|---|
| **1** | **MR0: truth fixes.** (i) Rename the venue tag `popular` to "Venue's pick" in 4 languages, or back it with S7′ **[DEC D5]**. (ii) Owner console: count and list dishes with **undeclared** allergens and block "publish" of new undeclared dishes (the hub already refuses sale, `allergens.rs:10-11`). (iii) Wire or confirm the guest `dw_avoid` filter (VERIFY) | high ÷ XS ÷ low | `public/store/menu.js:112`, `store/i18n.js`, `admin/*` allergen view, `state.js:322-327` | Every card flag text is a claim the data supports; the undeclared count shows on the menu screen; avoid-filter test: a dish with `undeclared` is hidden when a filter is active | Toggle a dish's allergens on qa-durres; read the storefront card and the console count | `menu.flags.v1` |
| **2** | **GATE-1/2: invariant gates.** Extend `no-scoring.sh` with `(guest|customer|client|user|staff|courier)_(taste|affinity|propensity|segment|cohort|ltv|spend|wtp)`, `price_sensitivity`, `willingness_to_pay`. New `no-tracking.sh`: refuses `sendBeacon`, `scroll`/`IntersectionObserver` handlers that call `fetch`, and `dowiz.taste` inside any request body under `public/store/` | high ÷ XS ÷ low | `tools/gates/no-scoring.sh`, `tools/gates/no-tracking.sh` (+ `.prove.sh`) | RED on a planted `guest_affinity` and a planted beacon; GREEN on main | n/a (static gate); listed in run-all | `gate.no-tracking.v1` |
| **3** | **OF1: kitchen heartbeat + auto-pause** | very high ÷ S ÷ medium (false pauses) | `hubdo/timer.rs`, `cron/timer.rs`, `hubdo` placement path, `admin` settings (`alerts.kitchen.ack_min`, `auto_pause`), i18n | An unacknowledged order with no kitchen contact for K min alerts once; at M min it sets `deliveryPaused` with reason `kitchen_offline`; the storefront shows paused; acknowledging un-pauses only by the owner's tap (never silently) | Place an order on qa-durres with the console closed; observe the Telegram alert in the QA chat and the storefront `closedReason:"paused"`; reopen | `ops.kitchen-heartbeat.v1` |
| **4** | **MR1-MR3: Layer 0 strip** (context publish + owner rules + storefront strip + switch, default OFF) | high ÷ M ÷ low | new `crates/dowiz-hub/src/rank/{mod.rs, rules.rs, holidays.rs, tests.rs}`; `workers/api/src/cron/*` (nightly MET fetch, User-Agent); `hubdo/publish.rs` (a `context` object in the manifest); `store/shell.js` (select the hour), `store/menu.js` (strip + sort `today`), `store/i18n.js`; `admin/menu-rank.js` + i18n; registry `Host{api.met.no}` and `BROWSER dw_rank_off` | Strip ≤ 6 with reason chips; absent or stale context → today's exact render; `features.menuRank` absent → no strip; preview equals what the storefront shows for that hour (shared fixture) | §4.9 steps 1-7 | `rank.weather.upstream`, `rank.context.v1`, `rank.rules.v1`, `rank.holidays.v1` |
| **5** | **MR4: truthful "most ordered this week"** | high ÷ S ÷ low | `services/analytics/fold.rs` (7-day dish counts excluding cancelled/test), `rank` source `top_week` | Badge only when n ≥ threshold; n shown; equals the analytics pane's number | Place N probe orders on qa-durres; the badge appears with N; delete them; the badge disappears next publish | inside `rank.context.v1` (`why:"top"`, `n`) |
| 6 | **PD1: margin-floor alert** | medium ÷ S ÷ low (inert until recipes) | `services/analytics/kitchen/report.rs` (pure `margin_floor`), `services/operations/stock/tell.rs`, settings `alerts.margin.floor_pm`, dish line in `admin/ingredients*.js` | Crossing alerts once per level; uncosted dishes listed, not skipped | On qa-durres: create a supply, a recipe and a dish; receive at a higher price; read the alert in the QA chat and the dish line | `alerts.margin-floor.v1` |
| 7 | **MR6: measurement arms + honest report** | medium ÷ M ÷ low | `store/checkout.js` (`menuView`), `crate::body` allow-list, a new pure `rank/measure.rs`, `admin/menu-rank.js` report card | Arms 50/50 per page load (test with a seeded RNG); report CI and `needed_orders` match a fixture computed by hand; never prints "proved" | Place orders on both arms on qa-durres; read the report; its counts match | `rank.measure.v1` |
| 8 | **MR5: "use first" from expiring lots (owner approves)** | medium ÷ S ÷ medium (food safety wording) | `rank` source `chef_pick`, stock fold of lots ≤ 2 days with forecast use < lot left, owner Approve/Skip list | Never suggests a lot past use-by; never a raw-fish lot within 1 day of its date (owner-tunable); label "Chef's pick" only | Receive a lot expiring tomorrow on qa-durres; approve the suggestion; the strip shows it; consume the lot; it disappears | `rank.chef-pick.v1` |
| 9 | **AN1: exception kinds `cancel_after_bill`/`discount_after_bill` + weekly venue digest** (if the bill-print event exists, VERIFY) | medium ÷ S ÷ low | `exceptions/fold.rs`, `exceptions/alert.rs`, digest in the nightly, staff notice text in `privacy/notice.rs` | Counts per kind vs the venue's own 4-week range; no per-person aggregate (no-scoring gate GREEN) | Print a bill, then cancel, on qa-durres; a row appears; the digest lists it | `exceptions.digest.v1` |
| 10 | **OF2: kitchen replica cut** (AX7: ticket fields only, no phone or full address unless delivery) | medium ÷ S ÷ low | admin replica writer + registry row update | The replica for the kitchen role carries no phone for pickup or table orders | Read the kitchen login's replica on qa-durres and assert the fields | `replica.kitchen.v1` |
| 11 | **MR7: Layer 1 on-device taste memory (opt-in)** **[DEC D3]** | low-medium ÷ M ÷ medium | `store/taste.js` (new), `store/menu.js` (strip reorder), settings sheet, registry `BROWSER dowiz.taste.v1`, `no-tracking.sh` covers it | Default off; offered after the 2nd order; Forget wipes; a test proves no request body carries it | Headless browser on qa-durres: opt in, place 3 orders, see "Again?"; Forget; the strip returns to Layer 0 | `rank.taste-local.v1` (schema of the IndexedDB value, never on the wire) |
| 12 | **OF3: offline cash sale + Albanian 48 h fiscal path** (adapts the PRRO row) **[DEC D8]** | high for some venues ÷ L ÷ high (fiscal) | room/till outbox, fiscal queue (`SEND_ENABLED` stays false until the operator says otherwise) | Offline sale queued with a key; paper receipt marked "pa NIVF"; synced; fiscal queue holds it with a 48 h deadline | qa-durres with the network cut on the tablet (DevTools offline); reconnect; the sale lands once | `till.offline-sale.v1` |
| — | **NOT BUILT:** S9 scroll velocity, S10 hesitation trigger, S11 "collect everything", S12 per-person price or bonus, S16 inferred-allergy hiding, S19 server-side taste profile, platform-anonymised staff KPIs, auto-published event packs, scraped event feeds | — | — | Gates GATE-1/2 make the first four hard to add by accident | — | — |

**Sequencing.**
1. Rows 1-2 first: no dependencies, protect everything after them.
2. Then row 3, independent.
3. Then rows 4-5 together (one lane). Row 4 depends on BN2 being live with its `CDN` binding (VERIFY).
4. Rows 6 and 8 wait for the venue data (P1 starter pack and recipes).
5. Row 7 after row 4 has run for a week.
6. Row 11 only on decision D3.

---

## 7. Final verdict

**Build, layer by layer:**
- **Layer 0** (rows 4, 5, then 7 and 8): a "For today" strip from owner-written rules over weather (MET Norway), time, holidays (a static list), owner-entered events, a truthful 7-day "most ordered" with its number, and owner-approved "use first".
  - Published nightly to the CDN. 0 Worker requests per visit. Default OFF per venue.
  - It never reorders or hides the venue's menu.
- **Layer 1** (row 11, on decision): on-device taste memory. Opt-in, wipeable, never sent.
- **Layer 2:** not now (re-entry in §4.4).
- **Around it:** truth fixes (row 1), invariant gates (row 2), kitchen heartbeat (row 3), margin floor (row 6), exception digest (row 9).

**Do not build, and why:**

| Thing | Why not |
|---|---|
| Per-person price or bonus | Scores a participant (invariant); EU disclosure duty plus a possible Art. 22 effect; consumer-law risk in Albania |
| Hesitation trigger, scroll velocity | Manipulative design per EDPB 03/2022 and the UCPD guidance; tracking needs consent; unmeasurable benefit at this volume (§3.2) |
| "All information that can be collected" | Contradicts minimisation, purpose limitation and the personal-data gate's whole purpose |
| Hiding dishes by inferred allergy | Health data by inference (C-184/20); unsafe false negatives; the FIC information duty is met by showing, not hiding |
| "Most ordered today" | Untrue at 20-60 orders/day; misleading-practice risk |
| Staff KPI "anonymised" | A pseudonym is a person, so it is still scoring; the venue-level register already exists |
| Per-guest ranking "at the edge" | Undoes BN2, spends the Free caps, gives nothing the browser cannot do |
| Server-side taste profile | Adds a profile, a DPIA and an attack surface for no evidenced gain |

**Top 5 rows:**
1. MR0 truth fixes;
2. GATE-1/2;
3. OF1 kitchen heartbeat + auto-pause;
4. MR1-MR3 Layer 0 strip;
5. MR4 truthful weekly top.

**Рішення оператора (українською):**
- **D1.** Чи вмикати смугу «Для сьогодні» взагалі? Рекомендація: так, але за замовчуванням ВИМКНЕНО для кожного закладу; власник вмикає сам.
- **D2.** Підтвердити, що НЕ будуємо: персональні ціни й бонуси, тригер «вагання», швидкість скролу, «зібрати все», приховування за виведеною алергією, серверний профіль смаку, «знеособлені» KPI персоналу.
- **D3.** Шар 1 (пам'ять смаку лише на телефоні, opt-in) — будувати чи відкласти? Рекомендація: відкласти, доки Шар 0 не пропрацює місяць.
- **D4.** Погода: MET Norway (безкоштовно, з атрибуцією) чи платний Open-Meteo (від $29 на місяць)? Рекомендація: MET Norway.
- **D5.** Тег «popular», який власник ставить вручну: перейменувати на «Вибір закладу» чи рахувати з реальних замовлень? Зараз це неперевірене твердження на картці.
- **D6.** Футбол і події: лише вручну власником (рекомендація) чи API-Football з власним ключем власника?
- **D7.** Де юридично зареєстрований оператор dowiz? Якщо в ЄС, GDPR діє для dowiz як процесора напряму, і DPIA та реєстр стають обов'язком, а не практикою.
- **D8.** Офлайн-продаж готівкою з фіскалізацією протягом 48 годин (албанське правило): адаптувати рядок ПРРО під Албанію? Відправка у податкову лишається вимкненою, доки ви не скажете інакше.
- **D9.** Аномалії персоналу лише на рівні закладу, з підписаними рядками, без персональних KPI — підтвердити як остаточне.

---

## 8. Deferred checks (run after "BOX FREE", read-only)

1. `grep -rn "hiddenBecause\|state.avoid" workers/api/public/store`: is the explicit allergen filter wired into the card render? (§2.2, row 1)
2. `grep -rn "blocks.v1\|lib/blocks" workers/api/public`: BN3A state. (§1.1)
3. `grep -n "CDN" workers/api/wrangler.toml`: is BN2's R2 binding live? (row 4 dependency)
4. `grep -rn "unacknowledged\|ack_min\|kitchen.*alert" workers/api/src`: does an unacknowledged-order alert already exist? (row 3)
5. `grep -rn "bill.*print\|printed" workers/api/src/command`: a bill-print event for `cancel_after_bill`. (row 9)
6. `grep -rln "weight\|rank" <DG8 rule file>`: confirm the DAG has no weights. (§1.2)
7. Unzip the saved Law 54/2024 .docx (`unzip -p … word/document.xml`) and search "terminal" / "pajisje fundore": the Albanian ePrivacy 5(3) equivalent. (§2.1)
8. `grep -rn "features\." workers/api/src | grep -i default`: how a venue's `features` block is built (default-ON caveat, §4.6).
9. Extract the Law 124/2024 English PDF text (`pdftotext`, if installable) to pin the article numbers for principles, automated decisions and DPIA. (§2.1)

## 9. Sources (all accessed 2026-10-03)

**Albanian law and regulators:**
- Albanian Law 124/2024 — https://idp.al/wp-content/uploads/2025/03/Law-no.124-2024.pdf ; https://idp.al/wp-content/uploads/2025/04/Law-no.124-2024-DP.pdf ; https://www.dlapiperdataprotection.com/?t=law&c=AL ; https://kpmg.com/al/en/insights/2025/02/new-law-on--personal-data-protection-.html ; https://www.clym.io/regulations/law-no-1242024-on-personal-data-protection-albania ; https://www.iapp.org/news/a/albania-s-personal-data-protection-law-a-legal-framework-harmonized-with-the-gdpr
- Albanian Law 54/2024 — https://www.infrastruktura.gov.al/wp-content/uploads/2025/07/Law-no.-54-date-30.06.2024_On-electronic-communications-in-the-Republic-of-Albania.docx
- Albanian Law 9902/2008 — https://www.avokatipopullit.gov.al/media/manager/website/reports/LAW%20No%209902,%20dated%2017.4.2008%20ON%20CONSUMER%20PROTECTION.pdf ; https://www.tiranatimes.com/tougher-fines-for-consumer-abuse-_112210/
- Albanian VKM 434/2018 — https://www.deloitte.com/content/dam/Deloitte/al/Documents/tax/Deloitte%20Albania_Legal%20News_July%202018.pdf
- Albanian fiscalisation (offline, 48 h) — https://www.fiscal-requirements.com/news/2937-fiscalization-in-albania-what-happens-if-the-communication-or-connection-with-the-tax-administration-is-interrupted ; https://www.pwc.com/al/en/Law_on_fiscalization.pdf
- Albanian draft AI law — https://www.bogalaw.com/pdf/Newsletter%20-%20The%20Draft%20Law%20On%20Artificial%20Intelligence%20in%20Albania.pdf
- EU accession status — https://europeanwesternbalkans.com/2025/11/17/albania-opened-the-last-remaining-cluster-with-the-eu/

**EU law, guidance and case law:**
- GDPR — https://eur-lex.europa.eu/eli/reg/2016/679/oj ; Art. 22 https://gdpr-info.eu/art-22-gdpr/
- ePrivacy and EDPB 2/2023 — https://eur-lex.europa.eu/eli/dir/2002/58/oj ; https://edpb.europa.eu/system/files/2024-10/edpb_guidelines_202302_technical_scope_art_53_eprivacydirective_v2_en_0.pdf ; https://www.hunton.com/privacy-and-cybersecurity-law-blog/edpb-adopts-guidelines-on-scope-of-eprivacy-directive
- AI Act Art. 5 — https://artificialintelligenceact.eu/article/5/ ; https://eur-lex.europa.eu/eli/reg/2024/1689/oj
- AI-system definition guidelines — https://digital-strategy.ec.europa.eu/en/library/commission-publishes-guidelines-ai-system-definition-facilitate-first-ai-acts-rules-application ; https://www.insideprivacy.com/artificial-intelligence/european-commission-guidelines-on-the-definition-of-an-ai-system/
- UCPD and its guidance — https://commission.europa.eu/law/law-topic/consumer-protection-law/unfair-commercial-practices-and-price-indication/unfair-commercial-practices-directive_en ; https://www.insideprivacy.com/eu-data-protection/the-eu-stance-on-dark-patterns/
- Omnibus / CRD 6(1)(ea) — https://eur-lex.europa.eu/eli/dir/2019/2161/oj ; https://www.addleshawgoddard.com/en/insights/insights-briefings/2020/competition/the-eu-omnibus-directive--time-to-prepare-for-strengthened-consumer-laws/
- EDPB 03/2022 — https://www.edpb.europa.eu/our-work-tools/our-documents/guidelines/guidelines-032022-deceptive-design-patterns-social-media_en
- DSA Art. 25 — https://www.eu-digital-services-act.com/Digital_Services_Act_Article_25.html ; https://eur-lex.europa.eu/eli/reg/2022/2065/oj
- SCHUFA C-634/21 — https://twobirds.com/en/insights/2023/global/key-takeaways-from-the-schufa-case-of-the-cjeu
- C-184/20 — https://www.insideprivacy.com/eu-data-protection/special-category-data-by-inference-cjeu-significantly-expands-the-scope-of-article-9-gdpr/
- WP251 — https://ec.europa.eu/newsroom/article29/items/612053
- FIC 1169/2011 — https://eur-lex.europa.eu/eli/reg/2011/1169/oj ; https://www.legislation.gov.uk/eur/2011/1169/article/14 ; https://www.legislation.gov.uk/eur/2011/1169/article/44
- WP29 2/2017 — https://www.scl.org/6946-data-processing-at-work-updated-guidance/
- Platform Work Directive — https://osha.europa.eu/en/legislation/directive/directive-20242831eu-platform-work ; https://www.taylorwessing.com/en/insights-and-events/insights/2024/11/radar---eu-platform-work-directive-brings-in-new-protections-for-platform-professionals

**Evidence on ranking and nudging:**
- Cai, Chen, Fang 2009 — https://www.nber.org/papers/w13516 ; https://econpapers.repec.org/article/aeaaecrev/v_3a99_3ay_3a2009_3ai_3a3_3ap_3a864-82.htm
- Dayan & Bar-Hillel 2011 — https://dlab.sauder.ubc.ca/sjdm/journal/11/11407/jdm11407.html
- Yang 2012 — https://www.nrn.com/menu-trends/study-counters-prevailing-restaurant-menu-theories
- Cadario & Chandon — https://knowledge.insead.edu/marketing/which-healthy-eating-nudges-work-best
- Maier et al. 2022 — https://www.ncbi.nlm.nih.gov/pmc/articles/PMC9351501/
- Wansink retractions — https://retractionwatch.com/2018/09/19/jama-journals-retract-six-papers-by-food-marketing-researcher-brian-wansink
- DoorDash — https://careersatdoordash.com/blog/personalized-store-feed-with-vector-embeddings/ (403; snippet)
- Uber Eats — https://www.uber.com/blog/uber-eats-recommending-marketplace/
- Wolt — https://press.wolt.com/en-WW/237306-algorithmic-transparency-consumers/
- Lu, Ge, Mao 2024 — https://sem.tongji.edu.cn/semen/25714.html ; https://www.ebsco.com/articles/earth-and-atmospheric-sciences/1ce7f88f-6bd7-5095-8ade-0e916d4ad90e/when-gig-work-meets-extreme-weather
- Weather and online ordering — https://unpaywall.org/10.1108%2FK-05-2020-0322
- Sample-size rule — https://ar5iv.labs.arxiv.org/html/2305.16459

**Data sources for Layer 0:**
- MET Norway terms — https://api.met.no/doc/TermsOfService
- Open-Meteo — https://open-meteo.com/en/terms ; https://open-meteo.com/en/pricing
- Holidays — https://date.nager.at/api/v3/PublicHolidays/2026/AL ; https://www.qppstudio.net/publicholidays2026/albania.htm
- API-Football — https://www.api-football.com/news/post/how-to-get-started-with-api-football-the-complete-beginners-guide ; https://thestatsapi.com/blog/free-football-api-alternatives
