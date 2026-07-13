# dowiz / bebop2 — Integration Ports + Reactive Interface — BLUEPRINTS

> **Дата:** 2026-07-13 · Супроводжує [INTEGRATION-PORTS-PLAN.md](./INTEGRATION-PORTS-PLAN.md).
> 21 одиниця (IP-01..21). Кожна: **Мета · Межа (що чіпаємо / НЕ чіпаємо) · Форма · RED-контракт ·
> Рівень поставки**. НЕ код — блюпринт. Кор недоторканий; усе адитивно.
>
> Умовні позначки: 🔴 = red-line (crypto/auth/money/capability → human gate); 🟢/🟡/🔧 = рівні T1/T2/T3.

---

## Група 1 — Firewall межі (W0)

### IP-01 · KernelFacade — anti-corruption шар
- **Мета:** єдиний дозволений шлях у кор. Жоден адаптер інтеграції ніколи не викликає `kernel::decide`
  напряму.
- **Межа:** ЧІПАЄМО — новий `facade` crate у `dowiz-engine` workspace. НЕ ЧІПАЄМО — `kernel/*`
  (re-consumed, не rewritten).
- **Форма:** `KernelFacade` експонує рівно два методи: `submit_intent(SignedFrame) -> Result<Vec<Event>,
  Reject>` (write-side, проходить wire→Law→money ворота) і `read_projection(Scope) -> Projection` (read-
  side, тільки дозволена проєкція). Адаптер бачить **тільки** фасад; `Cargo.toml` адаптера **не має**
  `dowiz-kernel` у deps.
- **RED (R0):** тест-крейт, що додає `dowiz-kernel` у deps адаптера і викликає `decide` → **build MUST
  FAIL**. Компіляційний firewall доведений тим, що заборонений код не компілюється.
- **Рівень:** інфраструктура (передумова всіх портів). 🔴

### IP-02 · Розширення словника можливостей (`scope.rs`) — адитивне
- **Мета:** дати новим інтеграціям точні grant'и, не змінюючи наявні.
- **Межа:** ЧІПАЄМО — `proto-cap/src/scope.rs` `Resource`/`Action` закриті enum'и (розширюємо варіантами).
  НЕ ЧІПАЄМО — семантику наявних; **не додаємо** варіанта для money-mutation чи courier-score (їх не має
  існувати).
- **Форма:** Resource += `Menu 0x05 · Order 0x06 · Analytics 0x07 · Customer 0x08 · Corpus 0x09 ·
  Backup 0x0A · Loyalty 0x0B`. Action += `Render · CreateOrder · ReadProjection · UploadConversion ·
  Notify · SyncCatalog · Export · Backup`. Кожна пара Resource×Action = один вираз scope. Каверзи
  (`expires_at`, `rate<=N`) **тільки звужують**.
- **RED (R4):** attenuated capability просить Resource/Action поза своїм піддеревом → `verify` MUST FAIL.
  Тест доводить, що enum-розширення не відкрило шлях розширення влади.
- **Рівень:** інфраструктура. 🔴

### IP-03 · Порт-трейти InboundPort / OutboundPort
- **Мета:** один контракт, N адаптерів. Web/REST-грань кожного порту.
- **Межа:** ЧІПАЄМО — новий `ports` (або `bridges`) crate. НЕ ЧІПАЄМО — кор, фасад.
- **Форма:** `trait OutboundPort { fn required_scope(&self)->Scope; fn on_projection(&self, p:&Projection);
  }` (підписка на read-проєкцію) · `trait InboundPort { fn required_scope(&self)->Scope; async fn
  submit(&self, f:SignedFrame)->Outcome; }` (подача signed intent через фасад). REST-грань = тонкий axum-
  хендлер, що перекладає HTTP↔SignedFrame, і **нічого більше** (кор вирішує).
- **RED:** InboundPort з scope `Notify` намагається `submit` order-mutation → фасад повертає `Reject`,
  стан кора незмінний; assert.
- **Рівень:** T2·T3 інфраструктура.

### IP-04 · Наскрізна маршрутизація через HybridGate
- **Мета:** кожен вхідний кадр будь-якого порту проходить wire→Law→money ворота — без винятків.
- **Межа:** ЧІПАЄМО — інтеграційний шлях подачі. НЕ ЧІПАЄМО — `hybrid_gate.rs` (RequireBoth уже правильний).
- **Форма:** `submit_intent` фасаду обгортає `HybridGate::check(frame)` (PQ Ed25519⊕ML-DSA-65) → потім
  Law-gate (`assert_transition`) → потім money-gate (i64 by type). Будь-який fail = `Reject`, ніколи fake-
  pass; transitional `ClassicalUntilPqAudit` дозволений тільки явно й видимо.
- **RED (R2):** кадр з валідним Ed25519 але битим ML-DSA-65 → `RequireBoth` MUST reject (не «один з двох
  ок»). Assert HybridIncomplete/PqVerifyFailed.
- **Рівень:** інфраструктура. 🔴

---

## Група 2 — Реактивне ядро (W1)

### IP-05 · Operator-as-API-contract — 8 параметрів
- **Мета:** усі вхідні сигнали зводяться до змін 8 чисел оператора `M·Ü+Γ·U̇+c²·L·U=S`; нуль
  if-джерело розгалужень.
- **Межа:** ЧІПАЄМО — новий `reactive` модуль поверх field-двигуна (FE-01..17). НЕ ЧІПАЄМО — сам оператор
  (споживаємо, не переписуємо).
- **Форма:** `struct FieldParams { S, Γ, M, c², g, φ, A_max, Q }`. Кожен сигнал (§2.2 плану) = чиста
  функція `Signal -> Δ FieldParams`. INTENT→`S`-імпульс у FieldPos; EVENT→`c²`/`φ`; STATE→`g`;
  ENVIRONMENT/DEVICE→`Γ`/`A_max`/`Q`. `InputSource` trait уніфікує миша/тач/голос/жест/AR-промінь → усі
  дають `Intent{pos:FieldPos, magnitude}`.
- **RED:** подати два джерела на один кадр (голос + палець) → результуюче `S` = сума обох імпульсів
  (суперпозиція), не «останній виграв»; assert на числі.
- **Рівень:** T1 (вбудовано в двигун).

### IP-06 · QualityGovernor — драбина граційної деградації
- **Мета:** «works на будь-якому залізі» через єдиний драбинчастий контролер, керований frame-time.
- **Межа:** ЧІПАЄМО — governor-модуль. НЕ ЧІПАЄМО — money-boundary (гроші не туляться на жодному ступені).
- **Форма:** стани Q3 WebGPU-compute → Q2 фрагментний-шейдер → Q1 CSS-linear()-springs+View-Transitions →
  Q0 статика. Тригери переходу вниз: frame-time > budget (універсально), Compute Pressure/Battery
  (Chromium-only, тільки reduce-work), `prefers-reduced-motion`→Q0. Page Visibility hidden → pause (0
  роботи). Гістерезис, щоб не «блимати» між ступенями.
- **RED:** підняти synthetic frame-time > budget N кадрів → governor MUST спуститися ступінь; money-елемент
  MUST лишитися без tween на всіх ступенях; assert обидва.
- **Рівень:** T1.

### IP-07 · Мультимодальний вхід = суперпозиція джерел
- **Мета:** будь-яке джерело (touch/voice/gesture/gaze) → один `Intent`; кілька одночасно = `S₁+S₂`.
- **Межа:** ЧІПАЄМО — thin-shell input-адаптери (15 Web APIs, RW-09). НЕ ЧІПАЄМО — оператор.
- **Форма:** локальні провайдери, нуль-сервер: голос Whisper/Moonshine/Web-Speech (локально); жест
  MediaPipe; погляд/AR-промінь → FieldPos. Кожен емітить `Intent`; двигун складає в `S`. a11y-дзеркало
  DOM + text-input як рівноправні джерела (hybrid, DZ).
- **RED:** e2e — голосова команда «більше гострого» + тап на страві в межах 100мс → одна дія створює один
  order-intent з обома сигналами складеними; Playwright assert на DOM-результаті (toBeVisible/toContainText).
- **Рівень:** T1 (кросплатформно: web/laptop/AR-окуляри).

---

## Група 3 — Web / MCP / Agentic / RAG порти (W5)

### IP-08 · MCP-server порт + agent-as-capability
- **Мета:** один MCP-порт відкриває весь екосистему агентам; агент = носій делегованої sub-capability.
- **Межа:** ЧІПАЄМО — новий MCP-сервер-адаптер (T3). НЕ ЧІПАЄМО — кор (tools мапляться на scopes через
  фасад).
- **Форма:** MCP 2026 (spec finalized 2026-07-28, stateless core, OAuth 2.1 resource-server, stdio +
  Streamable-HTTP). `tools/resources/prompts` кожен прив'язаний до `Scope`: `read_menu`→`Menu·Render`,
  `place_order`→`Order·CreateOrder`, `read_orders`→`Analytics·ReadProjection`. Агент отримує **звужену
  делеговану** capability (attenuation-only) — sub-agent не може перевищити батьківський scope. A2A/AP2
  для agent-payments — але cash-модель означає, що payment-tool не існує (money-gate).
- **RED (R3+R4):** MCP-агент зі scope `Menu·Render` викликає `place_order` → deny; делегує собі ширший
  scope → verify fails. Assert обидва.
- **Рівень:** 🔧 T3.

### IP-09 · Corpus-порт — embedded / vector / RAG / agentic-RAG
- **Мета:** дати RAG/agentic-RAG системам read-only проєкцію (menu/orders/policies) як корпус, без доступу
  до PII/money.
- **Межа:** ЧІПАЄМО — Corpus-адаптер (out, read-only). НЕ ЧІПАЄМО — event-log (тільки читає проєкцію).
- **Форма:** agentic-RAG hybrid — pgvector (embeddings) + BM25 (lexical) + rerank. Корпус = scoped
  проєкція (`menu`, `order.completed(hashed)`, публічні policies) на pgrust-ноді. Read-token `sk_` scoped
  `Corpus·ReadProjection` — літерально не може мутувати. Embeddings рахуються локально (local-first);
  export у зовнішній vector-store = копія, яку власник обрав.
- **RED:** corpus-token намагається `submit` або читати `Customer` → deny; assert read-only by cap-class.
- **Рівень:** 🔧 T3.

---

## Група 4 — Notifications + Realtime (W3)

### IP-10 · Notify crate — event→outbox→fan-out
- **Мета:** одна подія кора → багато каналів, за однією capability, local-first, ніколи не блокує render.
- **Межа:** ЧІПАЄМО — новий `notify` crate. НЕ ЧІПАЄМО — kernel, push.js/messenger.ts (reuse як adapters).
- **Форма:** `kernel emits order.*` (sync local) → `NotificationPort.tail(event_log)` (holds
  `EmitNotification`) → routing-table match (venue-owned, part of event-log; wildcards `order.*` +
  per-event override) → **OUTBOX** (local durable: `{event_id, channel, recipient, payload, status:
  PENDING|SENT|FAILED|DEAD, attempts, next_retry_at, idem_key}`) → async worker → fan-out dispatcher →
  per-channel adapters. Retry exp-backoff+jitter → DLQ(DEAD) surfaced-in-UI-never-silently-dropped; dedupe
  `idem_key=H(event|channel|recipient)` + Idempotency-Key header; offline outbox drains on reconnect, reuse
  `/api/healthz` ratchet (boot-grace+latched-storm).
- **RED (R1):** kill network mid-delivery → подія лишається PENDING, redeliver on reconnect, exactly-once
  by idem. Assert.
- **Рівень:** T1·T2·T3 інфраструктура.

### IP-15 · Port F — Messaging / messenger адаптери
- **Мета:** order-status push + OTP наружу, customer-msg → owner-inbox всередину; чесність per-channel.
- **Межа:** ЧІПАЄМО — ChannelAdapter реалізації. НЕ ЧІПАЄМО — notify-ядро (IP-10).
- **Форма:** `trait ChannelAdapter { channel_id; required_scope; async deliver(&SignedFrame,&Config)->
  Outcome; idempotency_key }`. Адаптери: **Telegram (primary, $0, повний push+OTP, reuse messenger.ts)**;
  WhatsApp Business App (manual) / Cloud API (utility templates, per-message pricing 1Jul2025); WebPush
  (reuse push.js/sw.js); SMS/Email (provider). **Adapter-honesty:** scope рекламує per-channel capability
  → UI ніколи не пропонує «push on Instagram/TikTok» (API-impossible); IG/TikTok=reply-in-window-only.
  Albania stack: Telegram primary + WhatsApp second + IG catalog/discovery.
- **RED (R2):** flip 1 byte ML-DSA-65 підпису доставки → адаптер-отримувач REJECTS; assert.
- **Рівень:** 🟢 T1 Telegram · 🟡 T2 WhatsApp Cloud · 🔧 T3 raw webhook+MCP.

---

## Група 5 — Backups + Analytics (W4)

### IP-11 · Backup-порт — encrypted, venue-owned, replayable
- **Мета:** «твої дані — твої»; backup = event-log; restore = replay через той самий fold → bit-identical.
- **Межа:** ЧІПАЄМО — новий `backup` crate + BackupSink адаптери. НЕ ЧІПАЄМО — event-log, fold (детермінований).
- **Форма:** event-log → serialize → **XChaCha20-Poly1305 encrypt (key = venue's own, derived from
  passphrase/identity-key; dowiz НІКОЛИ не тримає)** → opaque ciphertext. Sinks (`trait BackupSink{async
  put(&Ciphertext,cfg)}`): Download / S3-R2 / Google-Drive (усі бачать тільки ciphertext) / their-Postgres-
  pgrust (plaintext тільки якщо вони тримають key — їхня БД, їхній вибір). Cross-device: Automerge CRDT доку
  над event-log, PQ-encrypted sync frames (ML-KEM-768→XChaCha20→ML-DSA-65 sig, verified vs AnchorRoster —
  тільки власні девайси merge). Scheduled local worker (`daily 03:00→[download,s3]`). **Меш = бекап**
  (кожен нод — повна репліка).
- **RED (R5):** flip 1 byte ciphertext → Poly1305 auth fails → restore REFUSES (never loads corrupt); +
  (R6) corrupt один event → replay MUST NOT дати accepted-state, reduceAnomalies flags via order_machine.
- **Рівень:** 🟢 T1 Download/Drive · 🟡 T2 S3/own-PG · 🔧 T3 raw. 🔴 (crypto)

### IP-12 · Port E — Data-export / analytics stream (read-only)
- **Мета:** tenant-scoped egress; venue OWNS numbers; dowiz ніколи не gatekeeper.
- **Межа:** ЧІПАЄМО — export-адаптер. НЕ ЧІПАЄМО — event-log (тільки читає pgrust-проєкцію на ноді).
- **Форма:** scope `Analytics·{ReadProjection,Export}` secret, **READ-ONLY by cap-class** (export-token
  літерально не може мутувати). T1 «Download my data»→CSV + «Send to Google Sheets» (one-click OAuth,
  dowiz keeps updated). T2 «Stream to my Postgres/Supabase» (paste connection-string, held encrypted,
  scoped write-only-their-tables, choose projections+cadence). T3 raw read-only query + webhook + MCP.
  Усі 3 читають pgrust-проєкцію → дані не форсуються з venue-заліза; export = копія за вибором власника.
- **RED (R3):** export-token намагається `submit`/мутувати → deny; читає `Customer`-scope поза grant → deny.
- **Рівень:** 🟢 T1 · 🟡 T2 · 🔧 T3.

---

## Група 6 — Marketing / Sales / Social порти (W2, W8)

### IP-13 · Port H — Embed / storefront (iframe + embed-link + hosting)
- **Мета:** «встав мій storefront у наявний сайт» — один рядок покриває всі web-білдери.
- **Межа:** ЧІПАЄМО — embed loader + cross-origin iframe. НЕ ЧІПАЄМО — order-core (усередині iframe,
  той самий kernel-шлях).
- **Форма:** `<script src="cdn.dowiz.app/embed.js" data-embed-key="pk_venue_..." async>` → loader інжектить
  **cross-origin sandboxed iframe** (different origin, тому `allow-scripts`+`allow-same-origin` НІКОЛИ не
  співіснують — комбо вбиває sandbox), `sandbox="allow-scripts allow-forms allow-popups"`, tight
  Permissions-Policy, `frame-ancestors` CSP. postMessage resize+order-callbacks зі **strict `event.origin`
  allowlist** обабіч. Multi-tenancy: `pk_` embed-key safe-in-HTML, server-side bound-to-origin-allowlist +
  rate-limited. Cash → iframe ніколи не тримає payment-secrets (нижчий surface vs Stripe). Per-builder:
  один `<script>` first (plain-HTML/WP/Shopify/Wix/Tilda/Webflow/Squarespace/Framer), потім marketplace
  apps лише де ріжуть friction (Shopify/Wix/WP.org/Webflow).
- **RED:** publishable `pk_` з чужого origin → server-side reject (origin-allowlist); `pk_` намагається
  read customer-data → deny (publishable class не має grant). Assert.
- **Рівень:** 🟢 T1 (marketplace app) · 🟡 T2 (snippet).

### IP-14 · Port B — Marketing attribution / conversion-upload
- **Мета:** cash-order attribution — один shape для Meta/Google/TikTok; local event-log = ідеальне джерело.
- **Межа:** ЧІПАЄМО — landing-capture jobs + upload-адаптер. НЕ ЧІПАЄМО — kernel (тільки читає
  order.completed проєкцію).
- **Форма:** (1) landing capture: grab `gclid`/`fbclid`/`ttclid` поруч з наявним `?ch=`, keep SHA-256
  hashed phone/email fallback → store on order in event-log. (2) on `order.completed` → server-side
  conversion upload keyed by click-id+hashed-contact → 3 тонкі адаптери: Meta CAPI (Dataset), Google Data
  Manager API, TikTok Events API. **Time-sensitive:** Google→Data-Manager-API **15Jun2026 cutover** (legacy
  Ads-API blocked), 63-day window; Meta retired 7d/28d Jan2026 (upload promptly); PII SHA-256 hashed
  (EMQ≥7/10); share event_id для browser-pixel dedup.
- **RED (R3):** upload-порт (scope `upload_conversion·order.completed(hashed)`) намагається читати raw
  phone → deny (тільки hashed у проєкції); assert no-raw-PII leaves node.
- **Рівень:** 🟢 T1 OAuth · 🟡 T2 own-Dataset · 🔧 T3 raw webhook.

### IP-16 · Port G — Catalog-sync + Port C loyalty + Port D reviews
- **Мета:** menu→Meta catalog (IG tagging/ads); wallet-pass loyalty; post-order review nudge.
- **Межа:** ЧІПАЄМО — три тонкі out-адаптери. НЕ ЧІПАЄМО — kernel; loyalty-логіка build-in-house (уже
  маємо order-ledger).
- **Форма:** **G Catalog-sync** (`sync_catalog·menu`): push menu → Meta Commerce Manager catalog → IG
  product-tagging (5/image) + Advantage+ ads; WhatsApp product-messages читають той самий catalog. **C
  Loyalty** (`notify·loyalty_state`): issue Apple/Google Wallet pass (no app-install, lifts adoption),
  update points on `order.completed` — **єдиний outward port**, решта in-house. **D Reviews**
  (`read_projection,notify·order.completed,review`): post-order Google-review nudge (deep-link+QR, no API,
  T1) + optional GBP Reviews API v4 list/reply у owner-dash (T2/T3, gated 60+ days). **Виключено:** Higgsfield
  (manual AI-video tip, не порт), Apollo (B2B, internal GTM only, ніколи в UI).
- **RED:** catalog-порт (scope `sync_catalog·menu`) намагається читати order-PII → deny; assert.
- **Рівень:** 🟢 T1 · 🟡 T2 · 🔧 T3.

---

## Група 7 — Entropy + QRNG (W6) 🔴

### IP-17 · SeedPool + EntropySource trait — «mix never replace»
- **Мета:** підмішувати додаткові джерела ентропії в DRBG-seed без ослаблення fail-closed OS-підлоги.
- **Межа:** ЧІПАЄМО — additive до `kernel/src/pq/rng.rs` (widen seeding→pool). НЕ ЧІПАЄМО — `Entropy` trait
  fail-closed контракт (`fill` entirely-or-Err), `compile_error!` guard, DRBG hot-path.
- **Форма:** `trait EntropySource { fn name()->&str; fn is_local()->bool; fn poll(out:&mut [u8])->Result }`
  (best-effort, timeout→пул проходить без). `struct SeedPool { os: MandatoryFloor, extra: Vec<Box<dyn
  EntropySource>> }`; `seed()->Result` fail-closed **тільки на OS**: (1) `os.fill?` обов'язково; (2)
  domain-sep concat `"bebop2/seed-pool/v1"‖os‖кожне-що-відповіло`; (3) **SHA3-512 екстрактор** (`hash.rs:352`).
  `EntropyRng::new/reseed` — одно-рядкова зміна `entropy_provider().fill` → `pool.seed()`. QRNG **тільки
  на seed+reseed**, ніколи per-byte/frame/render.
- **RED (R7+R8):** (R7) OS getrandom fails, ANU up → `seed()` MUST FAIL (beacon ніколи не заміняє підлогу).
  (R8) реєструвати мертвий beacon → `seed()` проходить БЕЗ нього, сила незмінна (monotone). Assert обидва.
- **Рівень:** інфраструктура. 🔴 crypto red-line — human gate.

### IP-18 · ANU QRNG — 3-крокове людське онбординг
- **Мета:** «додай власний квантовий генератор» простим, чесним, made-for-humans інтерфейсом.
- **Межа:** ЧІПАЄМО — Settings→Security→Entropy UI + ANU-адаптер (impl EntropySource, is_local()=false).
  НЕ ЧІПАЄМО — SeedPool (просто register нове джерело).
- **Форма:** ANU 2026 = `api.quantumnumbers.anu.edu.au` (`x-api-key`, `GET ?length=32&type=uint8`; free key
  at `quantumnumbers.anu.edu.au`; legacy `qrng.anu.edu.au` retiring). 3 кроки: **(1)** «Get free quantum
  key» (кнопка→signup нова вкладка; одне поле paste x-api-key); **(2)** «dowiz tests live» (тягне 16 байт,
  «✓ ANU beacon reachable — 41ms — sample: 3f a1 08…»; ключ у **OS keychain, ніколи plaintext/logs**);
  **(3)** «Done» (chip «Entropy: OS ✓ + ANU quantum ✓», тумблер, human tooltip). **Failure UX:** beacon
  down → chip сіріє «ANU quantum (paused — using device entropy)», без модалки/помилки/блоку. Quantum bytes
  **не** raw key → входять у SeedPool хешовані з OS. **Marketing rule:** «quantum-SEEDED», не «quantum-
  encrypted» (§6.4 плану — Y-00/QKD потребують оптичного заліза, софт не може).
- **RED:** e2e Playwright — вставити фейковий ключ → chip показує статус чесно; вимкнути мережу під час
  reseed → chip «paused», нод продовжує генерувати ключі (fail-closed to OS). Assert DOM.
- **Рівень:** 🟢 T1 (paste key) · 🟡 T2 (add other beacon/local-HW). 🔴

---

## Група 8 — Directory + Docs + Automation (W7)

### IP-19 · Automation-конектори (Zapier / n8n / Make) + універсальний webhook
- **Мета:** webhook/event-порт як універсальний конектор до ~8000 apps, без per-app інженерії.
- **Межа:** ЧІПАЄМО — WebhookAdapter (out) + inbound cap-guarded endpoints. НЕ ЧІПАЄМО — kernel.
- **Форма:** Triggers-outbound `order.*`→any URL, **PQ-signed (ML-DSA-65) + HMAC-SHA256** per endpoint
  («New Order»/«Order status changed»). Actions-inbound cap-guarded через kernel `decide`. Effort-порядок:
  **n8n zero-build today** (Webhook node free/core, self-hostable=local-first) → Zapier published app →
  Make module. Автоматизація тримає **тільки** `WebhookOut(order.*)` — не читає menu/PII.
- **RED (R3):** Zap з `WebhookOut(order.*)` намагається read menu/customer → deny; flip HMAC → receiver
  rejects. Assert.
- **Рівень:** 🔧 T3 (dev/agent).

### IP-20 · Integration directory UI + made-for-humans doc-стандарт + consent/revoke
- **Мета:** browsable по СФЕРАХ; згода прозора scoped; відкликання в один клік; док—для людини.
- **Межа:** ЧІПАЄМО — directory UI (owner) + doc-template + consent-мінтер. НЕ ЧІПАЄМО — capability-крипта
  (mint = SignedFrame під ML-DSA-65, вже є).
- **Форма:** hub по сферах (`Notifications·Backups·Analytics·Marketing·Ordering·Developer`). Картка =
  What+Why + **capability-in-plain-words** («Can READ menu+order status. Cannot touch money. Cannot see
  phones») + 3-step (1 screenshot/крок) + tier badge. Consent-екран (least-privilege): перелічує кожен
  grant, warns on sensitive, one Connect → мінтить scoped `SignedFrame` (verified offline vs AnchorRoster).
  «Connected» list: live-capability-plain-words + last-activity + **Revoke** (drop anchor→stops verifying
  mesh-wide). Doc-template: outcome-first, «It's working when…», troubleshooting, «For developers»
  collapsed. **Review-правило:** жодна картка не починається з config/API-key/акроніма. UX: time-to-first-
  value first, progressive disclosure, segment-specific paths.
- **RED:** consent на «read order status» → мінтований frame має РІВНО цей scope (не ширше); Revoke →
  наступний запит того порту fails verify. Assert scope-exactness + revoke-effect.
- **Рівень:** 🟢 T1 (owner UI).

---

## Група 9 — RED-suite (W-RED, gate кожної хвилі)

### IP-21 · Core-untouched + capability-isolation test-suite
- **Мета:** довести, що керівний закон механічний, не декларація; кожна гарантія має досяжний red→green.
- **Межа:** ЧІПАЄМО — новий test-крейт + Playwright e2e. НЕ ЧІПАЄМО — нічого продакшн.
- **Форма:** зібрати R0-R8 в один gate, кожен red-first:
  - **R0** адаптер імпортує kernel → build FAILS (IP-01).
  - **R1** offline-queue survives (IP-10).
  - **R2** PQ-signed delivery verifiable (IP-04/15).
  - **R3** capability isolation anti-exfil — **головний** (IP-08/12/14/19).
  - **R4** attenuation can't widen (IP-02/08).
  - **R5** backup tamper refused (IP-11).
  - **R6** deterministic replay (IP-11).
  - **R7** entropy fail-closed (IP-17).
  - **R8** entropy monotone (IP-17).
- **RED:** кожен рядок має відомий стан, де він падає до фіксу; жоден не `expect(true)`/skip/inflated-
  timeout (test-integrity red-lines). Ledger-row у `docs/regressions/`.
- **Рівень:** обов'язковий gate. 🔴

---

## Зведення: блюпринт → хвиля → сфера

| BP | Назва | Хвиля | Сфера/тип | Рівень | Red-line |
|---|---|---|---|---|---|
| IP-01 | KernelFacade firewall | W0 | інфра | — | 🔴 |
| IP-02 | scope.rs розширення | W0 | інфра | — | 🔴 |
| IP-03 | Port-трейти | W0 | web/REST | T2·T3 | — |
| IP-04 | HybridGate routing | W0 | інфра | — | 🔴 |
| IP-05 | Operator-as-contract | W1 | реактивність | T1 | — |
| IP-06 | QualityGovernor | W1 | реактивність | T1 | — |
| IP-07 | Multimodal superposition | W1 | реактивність | T1 | — |
| IP-08 | MCP + agent-capability | W5 | MCP/agentic | T3 | — |
| IP-09 | Corpus RAG-порт | W5 | vector/RAG | T3 | — |
| IP-10 | Notify crate | W3 | notifications | T1·T2·T3 | — |
| IP-11 | Backup port | W4 | backups | T1·T2·T3 | 🔴 |
| IP-12 | Data-export port E | W4 | analytics | T1·T2·T3 | — |
| IP-13 | Embed/storefront H | W2 | hosting/embed | T1·T2 | — |
| IP-14 | Attribution upload B | W2·W8 | marketing | T1·T2·T3 | — |
| IP-15 | Messaging F | W3 | messengers | T1·T2·T3 | — |
| IP-16 | Catalog/loyalty/reviews G·C·D | W8 | social/sales | T1·T2·T3 | — |
| IP-17 | SeedPool entropy | W6 | entropy | інфра | 🔴 |
| IP-18 | ANU QRNG onboarding | W6 | entropy | T1·T2 | 🔴 |
| IP-19 | Automation Zapier/n8n/Make | W7 | webhook | T3 | — |
| IP-20 | Directory+docs+consent | W7 | onboarding | T1 | — |
| IP-21 | RED-suite R0-R8 | W-RED | gate | — | 🔴 |

**Інваріант усіх 21:** кор недоторканий (R0 доводить компіляційно); кожна capability scoped/signed/офлайн-
перевірювана/attenuation-only/revocable; гроші ніколи не туляться; ентропія mix-never-replace; квантове
seeded-не-encrypted; усе для людини.
