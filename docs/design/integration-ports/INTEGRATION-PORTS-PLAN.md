# dowiz / bebop2 — Integration Ports + Reactive Interface — PLAN

> **Дата:** 2026-07-13 · **Тип:** дослідження → аналіз → синтез → план (НЕ код)
> **Стоїть на:** [[rust-engine-rewrite-arc-2026-07-13]] (RW-01..12, растовий двигун) ·
> [[dowiz-interfaces-design-arc-2026-07-13]] (Sea & Sheet, DZ-01..12) ·
> [[field-ui-engine-arc-2026-07-13]] (field-UI оператор, FE-01..17) ·
> [[hydraulic-loop-v2-arc-2026-07-13]] (кібернетичний контур) · kernel/bebop2 PQ ·
> DOD · pgrust · wgpu · egui.
> **Мета фінального вигляду:** dowiz/bebop2 = **хаб-меш децентралізованих локальних нодів
> з постквантовою крипто-безпекою**, кожен нод суверенний, будь-яка інтеграція — це **порт**,
> ніколи пряме втручання в кор.

---

## 0. Керівний закон (governing law) — сказано оператором дослівно

> **«Кор незмінний — будь-які інтеграції це порти до моєї системи і протоколу, ніколи із прямим втручанням.»**

Цей закон **уже механічно забезпечений трьома композованими воротами в коді** (не декларація —
компільована реальність). Наш план не винаходить його — він **розширює словник можливостей адитивно**
і проводить кожну інтеграцію через ці ворота:

| Ворота | Що гарантує | Де вже в коді |
|---|---|---|
| **Wire-gate** | Кожен вхідний кадр перевіряється гібридним PQ-підписом перед тим, як торкнутися логіки | `proto-cap/src/hybrid_gate.rs` (`HybridGate::RequireBoth`), `signed_frame.rs` |
| **Law-gate** | Кожна зміна стану проходить через `assert_transition` машини станів; нелегальний перехід = `409`, не мовчазна порча | `kernel/src/order_machine.rs`, `apply_event` у `server/` |
| **Money-gate** | Гроші — тільки `i64` за типом; кор **не винаходить жодного числа** | `kernel/src/money.rs` |

**Наслідок для архітектури** (три речі, і тільки три):
1. **Не чіпати кор.** Kernel (`order_machine`/`analytics`/`money`/`domain` з `NO-COURIER-SCORING`) —
   чиста decide/fold Law над event-log. Ніколи не rewritten — лише re-consumed.
2. **Обгорнути кор anti-corruption фасадом.** `KernelFacade` — єдиний дозволений шлях; адаптер
   інтеграції **ніколи не імпортує `dowiz-kernel`** → компіляційний firewall (спроба = build error).
3. **Провести кожну інтеграцію через словник можливостей.** Розширювати `scope.rs` (Resource × Action) —
   **тільки адитивно**; кожна інтеграція тримає `SignedFrame`-мандат, звужений до рівно того, що обіцяла
   картка. **Немає токена**, який виражає мутацію грошей чи скоринг кур'єра — кор-guard.

Одна фраза, що містить усю безпеку інтеграцій: **можливість (capability), а не ключ-носій (bearer);
звуження (attenuation), не розширення; перевірка офлайн, не дзвінок у центр.**

---

## 1. Дві осі цього плану

Завдання має дві переплетені осі — обидві живуть на межі кора, обидві реактивні:

- **Вісь A — Реактивний інтерфейс.** Інтерфейс + анімація стану, що реагує на **наміри/сигнали
  користувача** (незалежно від джерела: дотик/голос/жест/погляд), на **середовище**, на **фізичні
  оптимізації** та на **власні внутрішні системи девайса**. Динамічна зміна інтерфейсу з
  intent/event/state/environment.
- **Вісь B — Порти/мости інтеграцій.** Кожна інтеграція = порт до системи і протоколу. Вибір
  інтеграції — на розсуд клієнта, «як краще їм»: (1) мої готові рішення, (2) зручні кастомайзовані,
  (3) для девелоперів/агентів. Покрити всі типи з'єднань: web, MCP, iframe, embed-link, API, порти,
  агентські системи, сповіщення, бекапи, порти для embedded/vector/RAG/agentic-RAG, інтеграції для
  великих платформ (хостинг, веб-білдери), мости для соцмереж/месенджерів. + Entropy-порт (ANU QRNG)
  + чесний квантовий шум.

Обидві осі — **один і той самий математичний апарат**, що об'єднує всі попередні арки: критичне
демпфування ζ=1 (гідравлічний резонатор = field-UI easing = governor). Реактивний інтерфейс —
це **оператор поля як API-контракт**; порт інтеграції — це **можливість як API-контракт**. Один
event-log годує обидва.

---

## 2. Вісь A — Реактивний інтерфейс: оператор поля як контракт

### 2.1 Головна ідея — «operator-as-API-contract»

Field-UI двигун (FE-01..17) керується **одним оператором**:

```
M·Ü + Γ·U̇ + c²·L·U = S
```

Реактивність = **прив'язати кожен вхідний сигнал до одного з 8 параметрів цього оператора**, і більше
ні до чого. Це і є контракт: інтерфейс не має «if voice then…, if gesture then…» розгалужень — усі
джерела зводяться до змін тих самих 8 чисел. 8 параметрів-«ручок»:

| Параметр | Фізичний зміст | Що ним керує |
|---|---|---|
| `S` | джерело/збудження (forcing) | **наміри** (тап/голос/жест/погляд → імпульс у точку) |
| `Γ` | демпфування | енергетичний бюджет (батарея/термал → більше Γ = спокійніше) |
| `M` | інерція/маса | клас девайса (слабкий → важче поле, менше руху) |
| `c²` | жорсткість/швидкість хвилі | стан замовлення (PENDING→DELIVERED піднімає амплітуду) |
| `g` | «гравітація» фокусу | potential-well (куди тягне увагу) |
| `φ` | фаза/колір | семантика стану (terracotta→gold OKLCH) |
| `A_max` | стеля амплітуди | prefers-reduced-motion / QualityGovernor |
| `Q` | добротність/резонанс | середовище (offline/open-hours) |

### 2.2 Таксономія сигнал → параметр (чотири класи джерел)

- **INTENT** (намір користувача, ≤100 мс відгук): тап/клік/голос/жест/погляд → **все = імпульс `S`
  у `FieldPos`**. `InputSource` trait: миша/тач нативно; голос локально (Whisper/Moonshine/Web-Speech);
  жест MediaPipe; AR-промінь → `FieldPos`. **Мультимодальність = суперпозиція** `S = S₁ + S₂` (голос
  «більше гострого» + палець на страві складаються в одне збудження — не конфлікт, а сума).
- **EVENT** (подія системи/протоколу): `order.*` з event-log → зміна `c²`/`φ` (стан піднімає море,
  зсуває колір). Green's-function feedback (FE-10) — брижі відповіді.
- **STATE** (внутрішній стан застосунку): фокус/drill-down → potential-well `g` (FE-11), spectral
  layout `φ₂φ₃` (FE-07/12).
- **ENVIRONMENT** (середовище + девайс): online/offline, години роботи, GPS-близькість, **власні
  внутрішні системи девайса** (термал, батарея, мережа, видимість вкладки) → `Γ`/`A_max`/`Q`.

### 2.3 Реакція на власні внутрішні системи девайса (фізичні оптимізації)

Це те, що робить інтерфейс **чесно живим на будь-якому залізі** — від ноута до окулярів. Кожен сигнал
має **чесну доступність 2026** (не обіцяй те, чого API не дає):

| Сигнал девайса | API (2026) | Дія на оператор | Статус |
|---|---|---|---|
| Термал/тиск CPU | Compute Pressure API | ↑`Γ`, ↓`A_max`, ступінь QualityGovernor | Chromium-only → **тільки reduce-work** |
| Батарея | Battery Status API | нижче порогу → ↑`Γ`, менше частинок | Chromium-only → enhancement |
| Мережа | Network Information API | slow → менше вантажу, degrade-closed | Chromium-only → enhancement |
| Видимість вкладки | Page Visibility API | приховано → pause поля (0 роботи) | **універсально** |
| Час кадру | rAF frame-time | > бюджет → ступінь вниз | **універсально** (головний сигнал) |
| Reduce-motion | `prefers-reduced-motion` | `A_max→0`, миттєві переходи | **універсально** |
| Wake Lock | Screen Wake Lock API | тримати екран під час трекінгу | добре підтримано |

**Правило чесності:** універсальні сигнали (frame-time, prefers-*, Page Visibility) — **основа** governor;
Chromium-only (Compute Pressure/Battery/Network) — **тільки покращення reduce-work**, ніколи не єдина
опора. Мертві/моторошні (Ambient Light Sensor, Idle Detection API) — **обходимо** (годинник замість сенсора
світла; Page Visibility замість детекції простою).

### 2.4 QualityGovernor — граційна деградація (один драбинчастий контролер)

Замість «works або не works» — драбина, керована frame-time budget:

```
Q3 WebGPU compute (metaball/reaction-diffusion, повне поле)
  ↓ (frame > budget АБО термал/батарея)
Q2 фрагментний шейдер (WebGL2, спрощене поле)
  ↓
Q1 CSS-пружини (linear() springs, View Transitions same-doc)
  ↓
Q0 статика (prefers-reduced-motion / дуже слабкий девайс)
```

Гроші **ніколи не туляться (tween)** на жодному ступені — вони на межі field↔state (FE-09), поза полем.

### 2.5 «Круті трюки» з мережі — відібрані (і відхилені)

**Взяті** (добре підтримані, семантичні): OKLCH terracotta→gold перехід кольору стану; CSS `linear()`
пружинні easing; View Transitions для same-doc переходів; WebGPU compute metaball/reaction-diffusion як
«море»; potential-well фокус; Green's-function брижі; spectral drill-down.
**Відхилені** (dead/creepy/несумісні): Ambient Light Sensor (моторошно + deprecated), Idle Detection API
(privacy-invasive), device-specific хаки, що ламають a11y або money-never-tween.

---

## 3. Вісь B — Порт-доктрина: гексагональна архітектура межі

### 3.1 Що таке порт (і чим він НЕ є)

**Порт = адаптер на межі, що робить рівно одне з двох:**
- **READ-side (outbound):** підписується на **проєкцію** event-log, яку йому дозволено бачити
  (`order.completed`, `channel` ledger), і штовхає похідні дані назовні.
- **WRITE-side (inbound):** подає **підписаний намір (signed intent)** у ворота можливостей (той самий
  шлях `POST /api/orders` / `apply_event`, що повертає `409` на нелегальний перехід). Ворота валідують,
  **кор вирішує. Порт ніколи не вирішує.**

Порт **ніколи не імпортує `dowiz-kernel`** (компіляційний firewall). Порти **деградують закрито**
(мертвий WhatsApp ніколи не блокує замовлення — cash-on-delivery стоїть сам). Порти **відкликаються в
один клік**.

> **Застереження про слово «port»:** rust-engine-rewrite вживає «port» = портування TS→Rust. Тут port =
> гексагональний адаптер межі. У коді назвати недвозначно: `bridges/` або `ports::external`.

### 3.2 Граматика можливості (scope) — маппиться на bebop PQ-ідентичність

```
scope = {
  tenant:    venue_id                 // ніколи не cross-venue
  direction: in | out
  actions:   [render, create_order, read_projection, upload_conversion,
              notify, sync_catalog, export, backup]
  data:      [menu, order.status, order.completed(hashed), channel_ledger, loyalty_state]  // явний allowlist
  caveats:   [expires_at, rate<=N/min]  // ТІЛЬКИ звужують (attenuation)
  class:     publishable | secret
  sig:       ML-DSA-65(venue_identity) ⊕ Ed25519   // гібрид
}   // перевіряється ОФЛАЙН vs AnchorRoster
```

- **Publishable (`pk_…`):** безпечно в client HTML (embed-ключ). Тільки низькоризикові публічні дії
  (render menu, create pending order, subscribe push-token). **Не може** читати дані клієнта.
  Anti-abuse = per-venue origin-allowlist + per-key rate-limit, server-side.
- **Secret (`sk_…`):** тільки server-side (warehouse sink, conversion uploader). Тримає бекенд venue
  або dowiz-hosted worker від імені venue.

**Немає grant** для мутації грошей чи скорингу кур'єра — це не токени, які port-модель може **виразити**
(кор-guard). Це модель **макарунів/biscuit**: caveats **тільки звужують**, математично гарантовано токен
ніколи не розширює власну владу, перевіряється **офлайн** (без дзвінка в центр) — ідеально для
децентралізованого мешу.

### 3.3 Три рівні поставки (кожен порт — на одному чи кількох)

| Рівень | Хто | Що робить | Поставка |
|---|---|---|---|
| **🟢 T1 Ready / one-click** | будь-який власник | Click «Connect», авторизувати, готово. dowiz хостить worker, мінтить capability. | Managed, zero config |
| **🟡 T2 Customizable** | hands-on власник / їхня веб-людина | Свій домен, свої шаблони, вибір каналів/полів, вказати свій warehouse. | Settings UI + guided fields |
| **🔧 T3 Developer / agent** | розробник або AI-агент | Сирі capability-токени, REST + webhook + **MCP** surface, SDK. Будує що завгодно в межах scope. | Docs + token issuer + MCP server |

---

## 4. Таксономія портів (усі типи з'єднань — одна таблиця)

Кожен тип з'єднання, який назвав оператор, = один порт-шаблон над тим самим event-log:

| Тип з'єднання | Порт | Directon | Scope (actions · data) | Рівень | Блюпринт |
|---|---|---|---|---|---|
| **Embed / iframe / embed-link** | H — Storefront embed | in | `render, create_order · menu` (pk_) | T1·T2 | IP-13 |
| **Web / REST API** | усі порти мають REST-грань | in/out | per-scope | T2·T3 | IP-03 |
| **MCP** | MCP-server port | in/out | tools→scopes | T3 | IP-08 |
| **Webhook / event stream** | універсальний out | out | `read_projection · order.*` (PQ+HMAC) | T3 | IP-19 |
| **Realtime / notifications** | F — Messaging/notify | in/out | `notify · order.status` | T1·T2·T3 | IP-15 |
| **Backups** | Backup port (encrypted CRDT) | out | `backup · event-log` | T1·T2·T3 | IP-11 |
| **Analytics / data-storage** | E — Data-export | out (RO) | `read_projection, export` | T1·T2·T3 | IP-12 |
| **Embedded / vector / RAG / agentic-RAG** | Corpus port | out (RO) | `read_projection · corpus` | T3 | IP-09 |
| **Agentic systems** | Agent-as-capability | in/out | delegated sub-scope | T3 | IP-08 |
| **Marketing / conversion** | B — Attribution upload | out | `upload_conversion · order.completed(hashed)` | T1·T2·T3 | IP-14 |
| **Catalog / social** | G — Catalog-sync | out | `sync_catalog · menu` | T1·T2·T3 | IP-16 |
| **Hosting / web-builders** | H (shared) | in | `render, create_order · menu` | T1·T2 | IP-13 |
| **Matcher / mesh dispatch** | Matcher (deterministic, no-one-owns) | — | pure, entropy-independent | — | (bebop `matcher.rs`) |
| **Entropy / QRNG** | Entropy port (SeedPool) | in (seed only) | OS floor + advisory sources | T1·T2 | IP-17·18 |

---

## 5. Ключові інтеграції по сферах (як ціла продуктова команда)

Один event-log, багато вихідних портів. Marketing-attribution + analytics-export + loyalty — **той самий
READ** (scoped projection `order.completed`). Cash + local-first = **перевага, не пробіл**: увесь ad-світ
2026 переїхав на server-side/offline conversion-upload — саме те, що продукує локальний event-log зі
збереженим click-id + hashed-phone.

| Сфера | Ключова платформа | Порт | Що робить власник | Чесний виріз |
|---|---|---|---|---|
| **Маркетинг** | Google Business Profile | A/B | dowiz-лінк як «Preferred» ordering link (2026: pure redirect, order на dowiz не Google, zero code) | — |
| **Маркетинг** | Meta CAPI · Google Data Manager · TikTok Events | B | «Connect IG/TikTok/Google» OAuth | Google→Data-Manager-API **15Jun2026 cutover**, 63-day window; Meta retired 7d/28d Jan2026; PII SHA-256 hashed |
| **Маркетинг** | `?ch=` first-party stamp | native spine | нічого — автоматично (вже: `Storefront.svelte:93`) | — |
| **Продажі** | Storefront/QR/deep-links + promo attribution | H | друк QR, share link | «Sales»=conversion+repeat, cash no-Stripe |
| **Продажі** | Loyalty (Apple/Google Wallet pass) | C | увімкнути punch-card | **Build-in-house** — уже маєш order-ledger; wallet-pass = єдиний outward port |
| **Продажі** | Google reviews (GBP Reviews API v4) | D | post-order review nudge | +0.1 star≈+1% covers; API gated (60+ days) |
| **Продажі** | *Higgsfield* | **немає порту** | опційна порада «зроби promo-clip» | Manual AI-video tool, не порт (credits volatile) |
| **Продажі** | *Apollo* | **виключено** | не показується власникам | Category error — B2B lead-gen; тільки internal GTM для dowiz-компанії |
| **Аналітика/дані** | Google Sheets / CSV | E | «Download» / «Send to Sheets» | 80% case |
| **Аналітика/дані** | Postgres / Supabase sink | E | вставити свій connection string | Replica, не master |
| **Аналітика/дані** | pgrust local-first store | E (джерело істини) | нічого — дані на ноді | Local-first anchor |
| **Аналітика/дані** | Webhook / stream / MCP | E | видати read-token | Read-only enforced by cap-class |
| **Месенджери** | **Telegram** (вже в dowiz) | F | увімкнути; клієнт тапає `t.me`/скан QR | **Primary engine** — $0, повний push+OTP |
| **Месенджери** | WhatsApp App / Cloud API | F | App: link number / API: guided signup | Utility templates, per-message pricing 1Jul2025 |
| **Соцмережі** | Instagram (Messaging + catalog) | F+G | connect IG Pro | Push **неможливий** (deprecated 27Apr2026) → discovery + catalog + in-window reply |
| **Соцмережі** | Meta product catalog | G | auto-sync menu | IG product-tagging + Advantage+ ads |
| **Соцмережі** | Viber (опційно) | F | тільки якщо Viber-heavy | ~€100/mo floor → skip by default |
| **Соцмережі** | TikTok | funnel→A/B | постити щодня, лінк у біо | Discovery only (push API-impossible) |
| **Хостинг** | Plain HTML / WP / Shopify / Wix / Tilda / Webflow / Squarespace / Framer | H | вставити один рядок, або install app | Один `<script>` covers all; marketplace apps лише де ріжуть friction |

**Adapter-honesty правило:** порт рекламує per-channel capability, тому UI **ніколи не пропонує** «push
order updates on Instagram» (не може). Telegram=full push; WhatsApp-API=utility templates; IG/TikTok=reply-in-window-only.

---

## 6. Entropy-порт + ANU QRNG + чесний квантовий шум (фінальний вигляд: PQ-меш)

### 6.1 PQ-меш — фінальний вигляд dowiz/bebop2

Меш **суверенних локальних нодів**, без привілейованого сервера. Нод-суверенність = (1) локальний
entropy-пул (`EntropyRng` ChaCha20 DRBG fail-closed reseed@1MiB, `rng.rs:475`; `compile_error!` якщо немає
entropy-провайдера — прод-keygen ніколи не шипиться без ентропії); (2) гібридна PQ-ідентичність, вся з
пулу через `*_from_entropy()`: Ed25519 + ML-DSA-65 (FIPS 204, q=8380417) + ML-KEM-768 (FIPS 203, q=3329);
(3) local-first дані (ledger на девайсі, мережа = підписані наміри, не істина). Транспорт: ML-KEM-768
encaps → XChaCha20-Poly1305 AEAD; Argon2id at-rest; SignedFrame Ed25519⊕ML-DSA-65 над канонічним TLV;
HybridGate RequireBoth; AnchorRoster genesis-frozen UCAN-subset; matcher.rs **чистий детермінований** —
жоден центральний диспетчер.

### 6.2 Entropy-порт — правило «mix NEVER replace» (уся безпека в одному реченні)

> **OS getrandom = ОБОВ'ЯЗКОВА ПІДЛОГА. Кожне інше джерело — ADVISORY, тільки ДОДАЄ, ніколи не
> gate/replace.**

Чому mix, не replace (3 реальні причини):
1. **Ніколи не довіряй одному джерелу.** ANU може бути MITM'нутий/rate-limited/логований/неправильний.
   Змішування = слабке як **найслабша ланка в ХЕШІ всіх** = сильне як **найсильніша вціліла**.
2. **OS getrandom уже CSPRNG-grade, завжди локальний** (NIST SP 800-90 default+fallback).
3. **Hash-combining МОНОТОННИЙ по ентропії.** `H(os‖qrng‖…)` непередбачуваний, якщо **будь-який** вхід
   непередбачуваний; додавання джерела **ніколи не зменшує** OS-підлогу.

Механізм (адитивний, ще НЕ в дереві): `EntropySource` trait `{name()→&str для UI, is_local()→bool
(девайс-HW vs мережевий beacon), poll(out)→Result (best-effort, timeout→пул проходить БЕЗ нього)}`;
`SeedPool{os: mandatory floor, extra: Vec<Box<dyn EntropySource>>}`; `seed()→Result` **fail-closed тільки
на OS-підлозі**: (1) `os.fill?` обов'язково; (2) domain-sep concat `"bebop2/seed-pool/v1"‖os‖кожне-джерело-
що-відповіло-вчасно` (мертвий beacon **пропускається**, не фатально); (3) **SHA3-512 екстрактор**
(`hash.rs:352`) — вихід сильний як найсильніший. QRNG викликається **тільки на seed+reseed**, ніколи
per-byte/per-frame/render-loop.

### 6.3 ANU QRNG — простий інтерфейс для «додай свій квантовий генератор»

ANU API 2026 (перевірено): legacy `qrng.anu.edu.au` (free no-key, **виводиться з експлуатації**) +
поточний `api.quantumnumbers.anu.edu.au` (`x-api-key` header, AWS-hosted, active). Запит
`GET ?length=32&type=uint8`; безкоштовний ключ на `quantumnumbers.anu.edu.au` + AWS usage-priced.

**3-крокове людське онбординг** (Settings → Security → Entropy sources):
- **Крок 1 «Отримати безкоштовний квантовий ключ»** — кнопка відкриває `quantumnumbers.anu.edu.au`
  реєстрацію в новій вкладці; одне поле вставити `x-api-key`.
- **Крок 2 «dowiz тестує наживо»** — на вставку тягне 16 квантових байт, людською мовою:
  «✓ ANU beacon reachable — 41ms — sample: 3f a1 08…»; ключ у OS keychain, **ніколи не в plaintext-config,
  ніколи не логується**.
- **Крок 3 «Готово»** — постійний chip «Entropy: OS ✓ + ANU quantum ✓», один тумблер on/off, tooltip:
  «ключі завжди захищені device-ентропією; коли ANU доступний — ще й підмішуємо квант; якщо повільно/офлайн
  — нічого не ламається, тихий fallback».
- **Failure UX (чесно):** beacon down на reseed → chip сіріє «ANU quantum (paused — using device entropy)»,
  без модалки/помилки/блоку. Ніколи не гірше за нод без QRNG.

### 6.4 Квантовий шум як шифрування — чесна відповідь (real vs marketing)

- **Реально:** Y-00/AlphaEta (Yuen-2000) — фізичний quantum-noise stream cipher (оптична модуляція, shot
  noise Born's-rule, ~160-201 Gbit/s демо); QKD; QRNG. **Не досяжно в софті** (marketing-пастка): Y-00/QKD
  потребують **спеціального оптичного заліза** (coherent transceivers/photon detectors/фізична shot-noise
  fiber). Web-app/телефон **не може** робити physical-layer quantum-noise шифрування. QRNG-seeded ChaCha/AES,
  назване «quantum encryption», = **перебільшення**.
- **Чесна пропозиція dowiz:** **quantum-SEEDED** висока-ентропія keying **поверх** FIPS PQ, defense-in-depth,
  ніколи не замінюючи: (1) quantum-seeded ключі — **шипимо це** (QRNG-байти мішаються в DRBG-seed, кожен
  ключ успадковує квантову ентропію; конфіденційність тримається на PQ/AEAD-математиці, але випадковість
  квантово-недетермінована); (2) опційний QRNG-derived extra-keystream — belt-and-suspenders (ChaCha20 XOR
  поверх ciphertext або nonce/subkey fold, той самий fail-closed пул, **не** заміняє FIPS PQ).
- **Marketing-правило:** кажемо **«quantum-SEEDED»**, не «quantum-encrypted». Захищувана заява: **«ключі
  згенеровані справжньою квантовою випадковістю поверх постквантово-захищеної криптографії»** —
  правдиво/перевірювано/захищувано.

### 6.5 Pluggable «додай свій квантовий генератор» = `EntropySource` trait

OS getrandom (local ✓ MANDATORY floor завжди) · ANU QRNG (beacon ✗ advisory best-effort skip-if-down) ·
Local HW QRNG (USB/PCIe/on-chip local ✓ advisory low-latency) · Other beacons (NIST Randomness Beacon
local ✗ advisory). Усе комбінується SHA3-512 `SeedPool::seed`. Додати джерело = `impl EntropySource` +
register. **Гарантії конструкцією:** fail-closed (тільки OS-підлога валить seed), monotone (нове джерело
тільки додає), visible (UI перелічує «OS ✓ + ANU quantum ✓ + local HW ✓»), local-vs-network явно
`is_local()`. Якість ентропії **ніколи не рекламується на дроті** (leaking «хто має кращу ентропію» =
targeting-сигнал). Кращий рандом **захищає тебе, не привілеює** — matcher детермінований, entropy-independent,
fancy-QRNG нод не має жодної економічної переваги. Суверенність збережена.

---

## 7. Онбординг + made-for-humans документація (усе — для людини)

### 7.1 Директорія інтеграцій — по СФЕРАХ, не по вендорах

In-app hub (2026: 5+ інтеграцій → нижчий churn, вища WTP), організований по сферах:
`Notifications · Backups · Analytics & Reporting · Marketing & Automation · Ordering Channels ·
Developer/Custom`. Кожна **картка** = проста мова, ніколи config-first:
- **What** (одне речення) + **Why you'd want it**
- **Capability in plain words:** «Can READ your menu and order status. Cannot touch money. Cannot see
  customer phone numbers.»
- **3-step setup** (нумеровано, 1 скріншот на крок)
- **Tier badge:** 🟢 Ready · 🟡 Customizable · 🔧 Dev-agent

### 7.2 Прозора scoped-згода (grant) + відкликання (revoke)

Consent-екран (2026 least-privilege, Google-Workspace-Marketplace/HighLevel-OAuth патерн): перелічує
**кожен** grant явно, попереджає на sensitive, одна кнопка **Connect**. На Connect dowiz **мінтить**
scoped PQ-capability = `SignedFrame` під ML-DSA-65 venue-ідентичністю, перевіряється **офлайн** vs
AnchorRoster. «Connected integrations» список: кожен рядок показує live-capability простими словами +
last-activity + **Revoke** (= додати revocation-caveat / прибрати anchor з AnchorRoster → SignedFrame
перестає перевірятися mesh-wide).

### 7.3 Made-for-humans doc-стандарт (Diátaxis: tutorial+how-to hybrid)

Кожна картка — фіксований шаблон. **Веди людським результатом; API/config — за collapsible «For
developers».** Review-правило: **жодна картка не починається з config-поля, API-ключа чи акроніма.**

```md
# <Plain title = outcome, e.g. "Get a text when an order comes in">
**Who it's for:** <one line>
**What it does:** <one sentence, no jargon>
**Why you'd want it:** <benefit in venue's words>
**What it can and can't touch:** ✅ read order status  🚫 cannot touch money  🚫 cannot see phones
## Set it up in 3 steps
1. <action> <screenshot>  2. <action> <screenshot>  3. <action> <screenshot>
**It's working when:** <concrete observable success>
## If something's off  — <top 3 problem → fix>
<details><summary>For developers</summary> event names, payload schema, scopes, signature </details>
```

### 7.4 Automation-конектори (Zapier / n8n / Make)

Webhook/event-порт **Є** універсальним конектором (~8000 apps, без per-app інженерії). Triggers-outbound:
`order.*` → any URL, **PQ-signed (ML-DSA-65) + HMAC-SHA256** per endpoint. Actions-inbound: cap-guarded
endpoints через kernel `decide`. Effort-порядок: **n8n сьогодні zero-build** (Webhook node free/core, self-
hostable = local-first fit) → Zapier published app → Make module. Автоматизація тримає **тільки**
`WebhookOut(order.*)` — не може читати menu/PII. Least-privilege переживає no-code зручність.

---

## 8. Порядок побудови (хвилі) — additive, кор недоторканий

| Хвиля | Що | Блюпринти | Ризик |
|---|---|---|---|
| **W0 — Firewall + словник** | KernelFacade anti-corruption; адитивне розширення `scope.rs` Resource×Action; InboundPort/OutboundPort traits; провести все через HybridGate | IP-01·02·03·04 | 🔴 red-line (capability/auth) — human gate |
| **W1 — Реактивне ядро** | Operator-as-contract 8 params; signal→param taxonomy; QualityGovernor драбина; multimodal S₁+S₂ | IP-05·06·07 | середній (UI) |
| **W2 — Landing capture + Embed** | Два landing-jobs (gclid/fbclid/ttclid + hashed phone поруч з `?ch=`); Port H single `<script>` embed | IP-13·14 | низький — розблоковує весь маркетинг |
| **W3 — Notify + Messaging** | notify crate (event→outbox→fan-out→adapters); Port F Telegram formalize + WhatsApp App | IP-10·15 | середній |
| **W4 — Data-export + Backup** | Port E CSV/Sheets/Postgres RO; Backup port encrypted CRDT mesh | IP-11·12 | 🔴 backup crypto — human gate |
| **W5 — MCP + Agentic-RAG** | MCP-server port; Corpus port (pgvector+BM25+rerank); agent-as-capability | IP-08·09 | середній |
| **W6 — Entropy + QRNG** | SeedPool + EntropySource trait (mix never replace); ANU 3-step onboarding | IP-17·18 | 🔴 crypto red-line — human gate |
| **W7 — Directory + Docs + Automation** | Directory-by-sphere UI; made-for-humans template; consent/revoke; Zapier/n8n/Make | IP-19·20 | низький |
| **W8 — Marketing scale + Social** | Port B conversion-upload (Data-Manager 15Jun2026); Port C loyalty wallet; Port D reviews; Port G catalog-sync; marketplace apps | IP-14·16 | низький, opportunistic |
| **W-RED** | Core-untouched тест + R1-R6 (кожен reachable red→green) | IP-21 | обов'язковий gate кожної хвилі |

**Big-bang заборонено.** Island-by-island, canonical `web/` beachhead, keep-running. Кожна хвиля закрита
тільки коли її RED-тест red→green (VERIFIED-BY-MATH, Mandatory Proof Rule).

---

## 9. Узгодження з попередніми планами (усе клеїться)

- **DOD / rust / wasm / kernel:** порти живуть у растовому двигуні (`dowiz-engine` workspace RW-01),
  ніколи не імпортують kernel-crate. Реактивний оператор = FE-01..17 field-двигун. Entropy-порт — у
  `kernel/src/pq/` сусідстві, additive до `rng.rs`.
- **pgrust:** аналітичний порт (E) читає pgrust-проєкцію на ноді — «your data on your machine»; warehouse
  sink = replica.
- **wgpu / egui:** QualityGovernor Q3=WebGPU compute над wgpu-полем (RW-11 view→wgpu-field); реактивні
  сигнали керують оператором, оператор рендериться wgpu.
- **Sea & Sheet (DZ-01..12):** порти постачають дані в SHEET (menu/prices/status); реактивність рухає МОРЕ
  (ambient field). Money на межі field↔state — ніколи не tween, на жодному ступені governor.
- **bebop2 PQ:** усі мандати = SignedFrame Ed25519⊕ML-DSA-65; транспорт ML-KEM-768→XChaCha20; HybridGate
  RequireBoth; AnchorRoster UCAN-subset; matcher детермінований. Ці плани **не додають нової крипти** —
  вони **споживають** наявну + додають **тільки** Entropy-порт (mix, fail-closed).

---

## 10. RED-контракти (кожен має досяжний червоний шлях — без false-green)

| # | Гарантія | RED (має падати до) | GREEN (проходить після) |
|---|---|---|---|
| **R0** | **Core-untouched** | Адаптер інтеграції імпортує `dowiz-kernel` → компілюється | Build FAILS (компіляційний firewall) |
| **R1** | Offline-queue survives outage | Kill network mid-delivery без outbox → подія втрачена | Outbox PENDING, redeliver on reconnect, idem→exactly-once |
| **R2** | PQ-signed delivery verifiable | Flip 1 byte ML-DSA-65 sig → receiver приймає | Receiver REJECTS; assert |
| **R3** | **Capability isolation (anti-exfil)** | Порт з `EmitNotification`/`WebhookOut(order.*)` читає menu/phone → succeeds | Authz DENIES, торкається нічого; assert 403 |
| **R4** | Attenuation can't widen | Attenuated cap просить ширший scope → granted | Verify FAILS; math narrowing-only |
| **R5** | Backup tamper refused | Flip 1 byte ciphertext → restore partial | Poly1305 fails → restore REFUSES |
| **R6** | Deterministic replay | Corrupt один event → different-accepted state | Replay bit-identical; reduceAnomalies flags illegal via order_machine guard |
| **R7** | **Entropy fail-closed** | OS getrandom fails, ANU up → seed succeeds (fallback to beacon) | Seed FAILS (тільки OS-підлога рятує; beacon ніколи не заміняє) |
| **R8** | Entropy monotone | Додати мертвий beacon → seed падає/слабшає | Seed проходить БЕЗ нього, сила незмінна |

R3 — головний тест довіри: **інтеграція не може ексфільтрувати за межі своєї capability.** Оскільки
capability — підписаний, офлайн-перевірюваний, attenuation-only `SignedFrame`, зведений до явного grant-enum
без money/PII-токена, спроба ексфільтрації **не має валідного шляху** — і RED-тест доводить, що deny
реальний, не припущений.

---

## 11. Одна фраза résumé

**Кор незмінний. Кожна інтеграція — capability-scoped порт над одним event-log, що деградує закрито,
відкликається в один клік, перевіряється офлайн, і не може виразити того, чого не обіцяла. Реактивний
інтерфейс — той самий оператор поля, куди наміри/події/стан/середовище/девайс входять як 8 чисел.
Ентропія тільки підмішується, ніколи не замінює OS-підлогу. Квантове — seeded, не encrypted, і ми це
кажемо чесно. Усе — для людини.**
