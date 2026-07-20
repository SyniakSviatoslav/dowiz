# DOWIZ ІНТЕРФЕЙСИ — БЛЮПРИНТИ ДИЗАЙНУ
## Implementation-ready робочі одиниці для агентів-виконавців

> Похідний від [DOWIZ-INTERFACES-PLAN.md](DOWIZ-INTERFACES-PLAN.md). Стоїть **на плечах** двигуна
> [BLUEPRINTS-FIELD-UI.md](../field-ui-engine/BLUEPRINTS-FIELD-UI.md) (FE-01..17) — DZ-блюпринти **споживають**
> FE-примітиви, не дублюють. Кожен DZ — самодостатня одиниця: точний scope, наявний код, дизайн-семантика,
> прив'язка до FE, gate. Джерела: 4 інвентарі коду + 2 дослідження + референс-артефакт. Поточний стан звірено.

## 0. КОНТРАКТ ВИКОНАВЦЯ

1. **Не втратити жодну фічу.** Кожен екран несе повний майстер-чекліст (client/courier/owner). Gate падає
   якщо будь-яка фіча зникла.
2. **Money never tweens.** `<Money>` (integer з kernel, no tween prop) скрізь; ESLint no-number-anim-on-price.
3. **Local-first.** Render/input loop ніколи не торкається сервера; server = async sync peer за outbox.
4. **Гібрид чесний.** DOM лишається для a11y-mirror/text-input/SSR-menu — архітектура, не тимчасовість.
5. **Двигун перший.** DZ-* залежать від FE-* (двигун). Виконувати FE-хвилі 0-1 перед DZ-хвилею 1.
6. **Coherence rules (9)** з плану §8.4 — жоден DZ не сміє порушити.
7. **RED→GREEN.** Кожен DZ має falsifiable gate.

## 1. ХВИЛІ І ЗАЛЕЖНОСТІ

```
ПЕРЕДУМОВА: engine FE-01..06 (zero-copy, SoA, fixed-timestep, particle→wgpu, SDF+tokens, MSDF text)
ХВИЛЯ 1 (Sea&Sheet backbone) ─────────────────────────────
  DZ-01 <Shell> три-акт two-layer (all roles)   dep FE-04/05/08
  DZ-02 token 3-tiers + <Money> + ESLint         dep FE-05/09
  DZ-03 spectral edge + dive/sheet-rise           dep FE-08
ХВИЛЯ 2 (поле-реактивність) ──────────────────────────────
  DZ-04 OrderStatus→Море (terracotta→gold)        dep FE-08/10 + hub
  DZ-05 Green's feedback vocab (all events)        dep FE-10
  DZ-06 local-first event-log + replay + env-signals dep §4
ХВИЛЯ 3 (пер-роль екрани — усі фічі) ─────────────────────
  DZ-07 CLIENT (menu/detail/checkout/track)        dep DZ-01..06
  DZ-08 COURIER (shift/tasks/delivery/earnings)    dep DZ-01..06
  DZ-09 OWNER (dashboard/menu/analytics/…16)       dep DZ-01..06
ХВИЛЯ 4 (мультимодальність + кросплатформ) ───────────────
  DZ-10 Intent/FieldPos + InputSource + voice/gesture  dep FE-*
  DZ-11 a11y-mirror + input-overlay hybrid         dep FE-15
  DZ-12 cross-platform (WebGL2/native/AR panel)    dep FE-16
```

---

## ХВИЛЯ 1 — SEA & SHEET BACKBONE

### DZ-01 — `<Shell>`: три-акт two-layer граматика (всі ролі)
**Depends** FE-04/05/08 · **Est** L

**SCOPE:** новий shared `<Shell>` (Rust/wgpu + DOM-fallback) — двошарова граматика (Море+Sheet+spectral) з
трьома актами (arrive/choose/receive) для client/owner/courier.

**CURRENT (звірено):** 2 стеки (legacy React apps/web + канонічний Astro/Svelte web/); particle-cloud = Море
seed; референс-артефакт = повна три-акт реалізація (hero→sheet-rise→sea-develops). Немає уніфікованого shell.

**WHY:** [план §6] три акти = shell-граматика ВСІХ ролей (3 URL states drill-down + working-back). Backbone
когерентності.

**TARGET:** `<Shell>` рендерить: (A) field-backdrop (Море, FE-04, z-under, `--sea-tint` over `--sea-backdrop`);
(B) Sheet (SDF content FE-05, rises FLUID ζ≈0.8, 26px top corners `--brand-radius`, spectral edge rim, grip);
(C) act state machine arrive→choose→receive = 3 URL states. Assignment rule (план §2.1) enforced: ambient/
transition/tracking/feedback→Море; content/word/price/decision→Sheet. Split-pane variant для owner density.

**GATE:** три акти = 3 URL з working-back state-intact; Море під кожним екраном; Sheet rises over Море (не
slide); reduced-motion → Море static gradient, стан легібельний.

**ACCEPTANCE:** ☐ two-layer (Море+Sheet) ☐ 3 акти 3 URL ☐ assignment rule ☐ split-pane owner ☐ reduced-motion.
**OUT OF SCOPE:** per-role content (DZ-07/08/09); не дублювати FE-04/05 (споживати).

### DZ-02 — Token 3-tiers + `<Money>` + ESLint no-tween
**Depends** FE-05/09 · **Est** M · 🔴 money red-line

**SCOPE:** GPU token table (3 tiers), `<Money>` component, ESLint rule.

**CURRENT (звірено):** tokens.css перелічувано (11 brand+40 status+8 semantic); **немає `--font-mono` token**
(gap — tabular-nums без token); branding редагує ЛИШЕ 3/10 tokens; 4 legacy money-tween sites
(ClientLayout:154/EarningsPage/DashboardPage:421/AnalyticsPage:262 via AnimatedNumber/CountUpPrice).

**WHY:** [план §8.1, §2.5] owner touches 5 tokens (opinionated); money architectural guarantee.

**TARGET:** T1 brand-owned (5: accent/ink/paper/type-pair/radius, presets 80%); T2 dowiz-fixed (spectral/sea/
field/`--font-mono` NEW/`--money-ink`/`--ease-snap`/`--ease-tide`/space/text/tap/status/chart); T3 internal
Cosmo-Noir. Один accent → 4 placements (Sheet/Sea-tint/spectral/backdrop). **`<Money>`**: `--font-mono`
tabular-nums `--money-ink`, renders kernel integer-cent, **NO tween prop exists**, never rounded. ESLint:
ban number-animation on price nodes.

**GATE:** RED — spроба tween money → ESLint error / `<Money>` has no anim path; 4 legacy sites → `<Money>`
snap (RED before: count-up; GREEN after: integer jump). Owner changes accent → 4 placements update coherent.

**ACCEPTANCE:** ☐ 3 tiers ☐ owner touches 5 ☐ `--font-mono` added ☐ `<Money>` no-tween ☐ ESLint rule ☐ 4
legacy fixed. **OUT OF SCOPE:** 🔴 не чіпати kernel money-логіку (лише presentation).

### DZ-03 — Spectral edge + dive/sheet-rise transitions
**Depends** FE-08 · **Est** M

**SCOPE:** spectral edge (seam) + 3 cinematic transitions.

**CURRENT (звірено):** референс-артефакт має `--spectral` oklch + dive shader (u_dive plunge/ring-wipe/
refraction/chromatic) + sheet-rise + sea-develops. Наявний код = reference, не generalized.

**WHY:** [план §2.4, §4] spectral = ЄДИНИЙ dowiz mark на brand Sheet; transitions = ζ-motion.

**TARGET:** `--spectral` re-derived per brand (terracotta→accent→gold), top rim every Sheet + ring-wipe dive +
tracking thread, faster коли attending (6s→1.4s). DIVE (ζ=1 anchored on pressed button, ring-wipe+refraction+
chromatic, Sheet revealed THROUGH sea; reduced→scroll); SHEET-RISE (FLUID gentle, grip drag-down returns);
SEA-DEVELOPS (DZ-04). Bind FE-08 motion + FE-10 Green's.

**GATE:** dive anchored on button (RED off-center); spectral on every sheet rim + speeds up attending; reduced
→ scroll not plunge.

**ACCEPTANCE:** ☐ spectral per-brand re-derive ☐ dive/sheet-rise ζ-driven ☐ attending speedup ☐ reduced-motion.
**OUT OF SCOPE:** sea-develops-with-state (DZ-04).

---

## ХВИЛЯ 2 — ПОЛЕ-РЕАКТИВНІСТЬ

### DZ-04 — OrderStatus → Море (terracotta→gold, sea-develops)
**Depends** FE-08/10 + hub · **Est** M

**SCOPE:** статус замовлення керує параметрами Моря (Act 3 tracking).

**CURRENT (звірено):** order_machine 10 states дзеркалено channel.js (браузер валідує БЕЗ сервера);
particle-cloud VOCAB вже мапить статус→{color,energy,swirl}; CourierTrack робить 24-particle burst on mount
(one-shot). Terracotta→gold = наявна VOCAB семантика (order amber → delivered gold).

**WHY:** [план §3.2] tracking = entrance field MATURING не progress bar — dowiz most distinctive moment.

**TARGET:** замінити discrete burst на **continuous field, color/energy/swirl = f(OrderStatus)**: Pending/
Confirmed/Preparing/Ready=ember drift energy0.3; InDelivery/PickedUp=teal swirl1.6 (courier motion); Delivered=
gold bloom burst1.8; Rejected/Cancelled=blood swirl3.4; illegal=red recoil. Амплітуда росте + колір travels
terracotta→gold + waves turn зі станом (TIDE ζ>1). Local (channel.js validate). Step pills + honest ETA range
(never single/0) на Sheet.

**GATE:** статус advance → амплітуда jump + колір shift (RED: static); illegal transition → red recoil; money
на tracking = `<Money>` snap; local validate no server call.

**ACCEPTANCE:** ☐ continuous f(OrderStatus) ☐ terracotta→gold ☐ swirl per courier/failure ☐ step pills+ETA ☐
local no-server. **OUT OF SCOPE:** general feedback (DZ-05).

### DZ-05 — Green's-function feedback vocab (усі події)
**Depends** FE-10 · **Est** M

**SCOPE:** уніфікований feedback: кожна дія/подія = field source.

**CURRENT (звірено):** particle-cloud VOCAB (5 events); legacy per-component feedback (framer whileTap/toast/
haptic). Reactivity catalog (hub report) = повний INTENT/EVENT list.

**WHY:** [план §3, §5 field-ui] ОДНЕ Green's механізм замість per-component feedback код.

**TARGET:** event→source vocab: tap→δ ripple (ζ0.6), add-cart→ingest pulse + cart-pill `<Money>` snap, success/
delivered→Gaussian HEAT bloom, error/reject→high-λ shake, loading→sustained source, order-placed→amber burst,
anomaly→agitation. Particles = tracers seeded ∝|∇U| advected U̇. Ripples+particles = 2 renderers 1 field.
Reduced-motion → static state legible.

**GATE:** одна дія = один field impulse (RED: per-component code); tap ripple expanding front; error shake
decays; money snap never tween in feedback.

**ACCEPTANCE:** ☐ event→source vocab ☐ unified (no per-component) ☐ 2 renderers 1 field ☐ money snap. **OUT OF
SCOPE:** input modalities (DZ-10).

### DZ-06 — Local-first event-log + replay + env-signals
**Depends** план §4 · **Est** L

**SCOPE:** local persistence + 3 env-signal gaps.

**CURRENT (звірено):** render local-first (Astro SSG + WASM kernel, order-flow no network); persistence
server-side но UI не POST; supplies localStorage-only; env gaps online-offline/open-hours/GPS не існують.

**WHY:** [план §4, §3.4] render ніколи не торкається сервера; field реагує на середовище.

**TARGET:** event-log local (OPFS+SQLite-WASM браузер / native SQLite; дух pgrust Rust-native) + replay
`fold_transitions` on load = канонічний стан no round-trip. Outbox pattern (submission/payment behind outbox
drains online). ENV signals: `navigator.onLine` listener (field syncs vs pure-local + indicator у Морі),
open-hours model (Settings working-hours → ambient Море brightness/dormancy, closed=calm dark), geolocation
feed (courier marker kinematics = field flow along route). Reconcile supplies localStorage → event-log.

> ⚠ CORRECTED (operator, 2026-07-16): the **TARGET** above previously named `OPFS+SQLite-WASM браузер / native SQLite` as the event-log store. dowiz does NOT use SQLite as an architectural choice. The spectral/sqlless approach — content-addressed `BlockStore` + JSONL `FileEventStore` (`kernel/src/backup.rs`, `kernel/src/event_log.rs`) — is the MAIN storage/retrieval path in dowiz's own kernel/engine, with **pgrust as the uniform SQL-fallback/backup target, not SQLite**. Corrected TARGET: the local event-log persists via the content-addressed BlockStore + JSONL event-log pattern over OPFS (a pgrust-backed table only if the shape is genuinely relational), replayed by `fold_transitions` on load — never SQLite-WASM/native-SQLite as the engine.

**GATE:** offline → pure-local render (RED: server call); reload → replay reconstructs state; closed-hours →
Море calm dark; GPS → courier field flow.

**ACCEPTANCE:** ☐ local event-log + replay ☐ outbox drains online ☐ online/offline signal ☐ open-hours ambient
☐ GPS kinematics ☐ supplies reconciled. **OUT OF SCOPE:** CRDT cross-device sync (deferred).

---

## ХВИЛЯ 3 — ПЕР-РОЛЬ ЕКРАНИ (усі фічі майстер-чеклістів)

### DZ-07 — CLIENT (menu/detail/checkout/track)
**Depends** DZ-01..06 · **Est** XL

**SCOPE:** усі client-екрани в Sea&Sheet, кожна фіча збережена (master checklist client).

**CURRENT (звірено):** MenuPage/CheckoutPage/OrderStatusPage + ClientLayout + components (повний checklist у
конспекті).

**TARGET (кожна фіча → sea/sheet):**
- **Shell:** per-tenant palette (5 tokens→Sea+Sheet+spectral+backdrop), logo, Sunlight/Currency(+EUR)/Language
  gated, sticky cart bar = `<Money>` pill (НЕ AnimatedNumber).
- **Menu Sheet (Act 2):** hero (Google rating+reviews, geo ETA, StateChip open/closed/busy + banners) над
  Морем; category tabs scroll-spy + Chef's Picks ✦; search/sort(price/protein/kcal)/allergen-filter persisted;
  grid = SPREAD diffusion from tap (не pop); no-results reset. Category select → Sheet re-diffuse; closed →
  Море calm dark.
- **Detail Sheet (bottom-sheet/modal):** rich media (ADR-0002 lazy+gated), reveal = Green's bloom, kcal/macro/
  taste-axes/nutrition/allergen-list/ingredients, **modifier groups** (radio/checkbox/select/quantity req/min/
  max price-deltas), qty stepper, add-to-cart live `<Money>` + toast + cart ripple + haptic. Card tappable +
  shared-element morph.
- **Cart:** local-first (localStorage per-location versioned + cross-tab + dedupe + reconcileToMenu).
- **Checkout Sheet (progressive disclosure):** delivery/pickup tabs; contact (name/phone Albanian→E.164);
  messenger; entrance photo R2; MapWithPin + My-Location; entrance/apartment/notes; dropoff chips; cash amount+
  change+tip; summary (subtotal/fee-or-calculated/tax/tip/`<Money>` total/wide-ETA/client-mirror); NutritionRing;
  Wikipedia fact; draft persist; place order (idempotency + **OTP** send/verify/intent-hash + full error matrix
  + fallback phone) + success + push + privacy.
- **Track (Act 3 sea-matures):** DZ-04 OrderStatus→Море + status fetch + tracking-link ?t= + live WS (route/
  courier-position/status terminal-lock/message) + 30s watchdog + CourierLiveMap tweened+rotated marker (=field
  flow) + honest ETA range + OrderProgress stepper (delivery/pickup branch) + share-location + call/message +
  terminal CTAs + rating+feedback+Google-invite + MessageThread + offline banner.

**GATE:** кожна фіча master-checklist присутня (RED: enumerate missing); money `<Money>` snap; OTP flow; WS
live; local menu render.

**ACCEPTANCE:** ☐ повний client checklist ☐ modifier groups ☐ OTP ☐ WS tracking ☐ money snap ☐ local-first.
**OUT OF SCOPE:** owner/courier.

### DZ-08 — COURIER (shift/tasks/delivery/earnings/history)
**Depends** DZ-01..06 · **Est** L

**TARGET (master checklist courier):**
- **Shell:** BottomTabBar Tasks/Earnings/History/Shift; full-bleed delivery/login; dev-mode.
- **Login/Invite:** email/pw error-shake; invite redeem (role-aware Courier/Dispatcher, 16-char code, validity
  states).
- **Shift (Act 1):** live HH:MM:SS timer, start/end (Море energy raise on start), on/off pulsing dot, today's
  stats grid, messenger save.
- **Tasks (Act 1→2):** assignment fetch + real online-status + WS task_assigned → **один ripple + ping +
  dedupe**; accept→delivery/reject optimistic-restore; TaskCard **60s countdown + auto-decline** + pickup→
  dropoff timeline; online/offline empty.
- **Delivery (Act 3 Море=карта):** live GPS 12s heartbeat + CourierLiveMap (courier/dest/client pins + route)
  = field flow; WS client_location + mid-delivery cancel; drop-off card; entry-photo→fullscreen; tip; cash
  breakdown; call/message; mark-picked-up; cash-collected; **SwipeToComplete** (keyboard, resets-on-failure) →
  delivered=gold bloom; celebration.
- **Earnings:** `<Money>` today/week/month (НЕ count-up), payouts + StatusBadge.
- **History:** completed (locale date sq/en/uk, 5-star, feedback).

**GATE:** повний courier checklist; 60s auto-decline; SwipeToComplete never-fake-success; GPS field flow; money
snap. **OUT OF SCOPE:** ⚠️ NO-COURIER-SCORING reconcile (окремо).

### DZ-09 — OWNER/ADMIN (dashboard/menu/analytics/…16 pages)
**Depends** DZ-01..06 · **Est** XL

**TARGET (master checklist owner):**
- **Shell:** auth guard + entry-redirect (new→onboarding/draft→activation/published→dashboard); 11-nav (sidebar
  + mobile bottom-4 + More-7); logout; currency/lang/sunlight; DEV Flow-Test.
- **Dashboard (Act 1 business pulse):** 5 KPI (Revenue=`<Money>` НЕ count-up); search/filter/sort/Live-History/
  CSV/copy-link; **OrderCard** (shortId/timeline-deltas/OTP-verified/reputation/masked/rating+feedback/overdue/
  hollow-guard) + lifecycle actions (Accept→CONFIRMED/Reject→CANCELLED danger-confirm/Mark-Preparing/Mark-Ready/
  Assign→IN_DELIVERY) = **кожна дія ripples поле**; preset messaging; readiness 7; new-order banner+ping+haptic;
  **WS dashboard** (order.created/status regression-guarded merge + PII claim-check debounced refetch + courier
  position/shift); live map NO-PRESENCE. New order = Green's ripple, volume = amplitude, anomaly = agitation.
- **MenuManager (Act 2 split-pane):** category+product CRUD (name/price/prep-time/stock/taste-5/photo≤5MB) +
  availability toggle + RecipeEditor BOM + MediaManager rich + KitchenBusy + MenuSchedule + AI import wizard.
  ⚠️ **wire dormant AllergenEditor publish-gate + ReadinessIndicator**.
- **Analytics:** period 7d/30d; 4 KPI (Revenue `<Money>`); revenue bar; top-products drill-down; ingredient
  consumption+reorder; heatmap; geo-map; CSV.
- **CRM:** list+search/sort; contact reveal (audit PII)+redacted CSV; high-value badge; detail (prefs/orders/
  heatmap).
- **Promotions:** CRUD (code/type/value/window/max-uses); active toggle; states; usage badge.
- **Branding (⚠️ full token editor):** розширити 3/10 → font-picker+radius/spacing+presets+skin; owner touches
  5 T1; live phone-frame postMessage; auto-generate; Google/social.
- **Settings:** store details; delivery+zone map; **working hours → open-hours env signal (DZ-06)**; language;
  Telegram (QR/deep-link/targets/test + category preference-centre); fallback phone; delivery pause/resume.
- **Couriers:** list+active-count+invite (role/16-char); detail (earnings/deliveries/shifts); timeline; CSV;
  live map. ⚠️ rating-read vs NO-COURIER-SCORING reconcile.
- **Onboarding/Activation:** menu-first (upload→AI parse→pre-fill→claim); **trinity gate** (menu/notifications/
  fulfillment) + pickup toggle + publish (canPublish + missing[] + preview + inline edit).
- **Auth/Supplies/FlowTest:** email+pw/Telegram/Google/dev; supplies (reconcile localStorage→event-log);
  FlowTest DEV harness.

**GATE:** повний owner checklist; lifecycle actions ripple поле; money `<Money>`; WS dashboard merge; wire
allergen-gate; branding full editor; split-pane density. **OUT OF SCOPE:** NO-COURIER-SCORING policy (reconcile
separately).

---

## ХВИЛЯ 4 — МУЛЬТИМОДАЛЬНІСТЬ + КРОСПЛАТФОРМ

### DZ-10 — Intent/FieldPos + InputSource + voice + gesture
**Depends** FE-* · **Est** L

**SCOPE:** unified input abstraction + voice + gesture backends (all on-device local).

**TARGET:** `FieldPos(u,v,w)` + `Intent` enum (Point/Impulse/Select/Navigate/Scrub/Command) + `InputSource`
trait; InputRouter.tick→field.apply (ОДИН code path). Backends: PointerSource; **VoiceSource** (wake-word
Porcupine → Moonshine/Web-Speech-processLocally/whisper multilingual → command grammar → Command → resolver →
Navigate/Select; Web Worker never main thread; deictic "покажи пасту"→Dive, "додай X"→Select); **HandCameraSource**
(MediaPipe 21-landmarks: point=Point Dirichlet, pinch=Select well-collapse, palm-push=Impulse, swipe=Navigate;
confidence_floor gate). Multimodal fusion (voice+point same well reinforce).

**GATE:** voice "add X" = same field response as tap (RED: divergent path); pinch = well-collapse; multimodal
reinforce; all on-device no server.

**ACCEPTANCE:** ☐ Intent/FieldPos/InputSource ☐ voice command-grammar local ☐ MediaPipe gesture ☐ fusion ☐
on-device. **OUT OF SCOPE:** AR (DZ-12).

> **FRAMING-КОРЕКЦІЯ (операторська директива 2026-07-18) — intent-interface = несуча UX-філософія, НЕ
> "optional integration".** Оператор: критично важливо, щоб власнику/клієнту/кур'єру не доводилось довго
> думати, вникати, гуглити чи постійно клацати — саме ТОМУ існують локальний агент + голосовий контроль +
> інтерфейс намірів, рендерений через фізику як продовження/відображення бекенду. Це корекція РАМКИ, не
> послідовності:
>
> 1. **Секвенування НЕ рухається.** Імплементація DZ-10 лишається Хвиля 4 / Phase-9b (master roadmap P38b
>    DoD-3: «DZ-10 voice + gesture remain at Phase-9b priority — pulling them forward is a scope violation,
>    not initiative») — голос не збудуєш раніше, ніж існує order-flow, яким керувати.
> 2. **Але архітектурно голос — НЕ периферійна фіча.** Touch, voice, gesture — три вхідні методи ОДНОГО
>    механізму: будь-яка модальність → `Intent{pos: FieldPos, magnitude}` → `S`-імпульс оператора
>    `M·Ü+Γ·U̇+c²·L·U=S` (IP-05: INTENT→`S`-імпульс у FieldPos, `InputSource` trait уніфікує
>    миша/тач/голос/жест/AR-промінь; IP-07: одночасні джерела = суперпозиція `S₁+S₂`, не «останній виграв»).
>    Див. [BLUEPRINTS-INTEGRATION-PORTS.md](../integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md) IP-05/IP-07.
> 3. **Це і є причина існування P38 та P40.** WebGPU field-render (P38a/P38b) рендерить UI як фізико-польове
>    відображення стану бекенду САМЕ ТОМУ, що така поверхня відповідає на НАМІР (тап, голосова команда,
>    жест — той самий field impulse), замість змушувати користувача навігувати конвенційне меню-дерево;
>    local-agent tool loop (P40) — той самий intent-канал у текстовій формі. Intent-паттерн присутній з дня
>    один: `Intent`/`FieldPos`/`InputSource` — це P38b DoD-1 (перша хвиля коду), ще ДО голосу; VoiceSource
>    пізніше лише ДОДАЄ backend до вже несучого механізму.
> 4. **Наслідок для виконавців (діє з дня один):** жоден DZ/FE не сміє будувати input-шлях, що працює лише
>    для pointer'а — кожен вхід іде через `Intent` (ОДИН code path; gate DZ-10 вже це вимагає). «Optional»
>    у Phase-9b означає «пізніше», НЕ «збоку».

### DZ-11 — A11y semantic mirror + input overlay (hybrid)
**Depends** FE-15 · **Est** L

**SCOPE:** hybrid DOM для a11y + text-input (non-optional WCAG).

**CURRENT (звірено):** legacy a11y DOM-based (role/aria-live/sr-only); canvas = zero semantic DOM; AccessKit no
web backend 2026.

**TARGET:** parallel hidden transparent DOM semantic mirror (dishes as real `<button>`, role/aria-label/tabindex,
reconcile per-frame from field widget list); transparent `<input>` overlay для forms (IME/autofill/mobile-kbd,
keep type=email/tel); SSR /s/:slug menu stays DOM. Document permanent losses (Ctrl+F/translate).

**GATE:** screen-reader reads mirror (RED: canvas invisible); form accepts typed input+autofill; keyboard nav.
**ACCEPTANCE:** ☐ semantic mirror ☐ input overlay ☐ SSR menu DOM ☐ reconcile per-frame. **OUT OF SCOPE:** public
SEO page migration (stays SSR).

### DZ-12 — Cross-platform (WebGL2 fallback / native / AR panel)
**Depends** FE-16 · **Est** L

> **Registered as P97 (2026-07-20).** The AR-panel content below (the "Track 2" spatial-computing
> readiness half of dowiz's AR/VR strategy) is now consolidated, alongside P38-rev §12.2, §17.5,
> and the spatial-storefront-voice-hub synthesis's Lane B, into
> [`BLUEPRINT-P97-ar-vr-spatial-interface-2026-07-20.md`](../BLUEPRINT-P97-ar-vr-spatial-interface-2026-07-20.md)
> §3. This unit's text is preserved unaltered below (append-only) — P97 is the canonical
> cross-reference for AR/VR going forward, not a duplicate.

**TARGET:** WebGPU+WebGL2 fallback (requestAdapter success); native desktop (winit/wgpu); AR = field→off-screen
texture→curved panel floating XR (`FieldPos.w`=panel-normal, ray→panel→FieldPos same as 2D pointer, money=pinned
mono panel). Native OpenXR Quest first; WebXR-WebGPU коли XRGPUBinding lands. iOS embed wgpu native view; Android
surface-loss handle.

**GATE:** WebGL2-only device renders (RED: assumes WebGPU); AR panel ray→FieldPos = same Intent as pointer;
native desktop window. **ACCEPTANCE:** ☐ WebGL2 fallback ☐ native desktop ☐ AR curved panel ☐ same Intent flat+
spatial. **OUT OF SCOPE:** WebXR-WebGPU binding (deferred until ships).

---

## ДОДАТОК A — DZ → FE (двигун) → ПЛАН МАПА

| DZ | Що | Споживає FE | План § |
|----|-----|------------|--------|
| 01 | Shell 3-act two-layer | FE-04/05/08 | §2/§6 |
| 02 | tokens + Money | FE-05/09 | §8.1/§2.5 |
| 03 | spectral + transitions | FE-08 | §2.4 |
| 04 | OrderStatus→Море | FE-08/10 | §3.2 |
| 05 | Green's feedback | FE-10 | §3 |
| 06 | local-first + env | §4 | §3.4/§4 |
| 07 | CLIENT | DZ-01..06 | §7.1 |
| 08 | COURIER | DZ-01..06 | §7.2 |
| 09 | OWNER | DZ-01..06 | §7.3 |
| 10 | multimodal input | FE-* | §5 |
| 11 | a11y hybrid | FE-15 | §5.3 |
| 12 | cross-platform | FE-16 | §5.3 |

## ДОДАТОК B — ІНВАРІАНТИ (жоден DZ не порушує)

1. Не втратити жодну фічу (master checklists client/courier/owner).
2. Money `<Money>` mono+tabular+integer+never-tween+never-round (4 legacy fixed).
3. Sea always dowiz; spectral edge threads every flow.
4. Local-first: render/input loop ніколи не сервер (outbox async).
5. Гібрид: DOM a11y-mirror/input-overlay/SSR-menu.
6. ζ governs all motion (SNAP UI / TIDE Sea); no raw cubic.
7. Consequence ≤--motion-instant network-indep.
8. Drill-down story reversible (URL-state working-back).
9. Owner touches 5 tokens; coherence by construction (cross-brand ~95% shared).
10. One Intent all modalities all platforms (field.apply one path).

## ДОДАТОК C — НАРАТИВНО-КІНЕМАТОГРАФІЧНА ЯКІСТЬ: ДЕ ВОНА ЖИВЕ І ЩО ВОНА ОЗНАЧАЄ (2026-07-18)

> Операторський запит: «бренд-кастомізація у межах наративного кінематографічного інтент-хвильового
> інтерфейсу — який має бути саме досвідом взаємодії з користувачем». Це НЕ нова робоча одиниця і НЕ
> декоративний шар поверх механічно-функціонального shell — це явне іменування ЯКОСТІ, якою Море вже
> мусило бути (не просто "ambient field", а ambient field з наративним/кінематографічним pacing), плюс
> перевірка, що 5-токенна межа Sheet цим запитом не порушується. Читати разом із DZ-01/03/04/05 та
> [BLUEPRINT-P38](../CORE-ROADMAP-2026-07-17/BLUEPRINT-P38-webgpu-render-engine.md) §11.

### C.1 Вердикт: наратив/кінематографія = Sea. Sheet лишається 5 токенів. Шостого токена НЕМАЄ.

Аргумент, не постулат:

1. **Правило приналежності вже вирішило.** План §2.1: «Motion між станами або у відповідь на дію →
   МОРЕ». Pacing — це і Є motion між станами, розгорнутий у часі. Драматургія (яка подія отримує
   паузу, яка — швидкий зріз) = темпоральна форма arrival/transitions/tracking/feedback — всі чотири
   Sea-owned (план §2.2). Кінематографічність не СИДИТЬ у Sea — вона з правила приналежності ВИПЛИВАЄ.
2. **Темп уже структурно brand-agnostic.** ζ-словник (план §8.2), `--ease-snap/tide`,
   `--spectral-speed` 6s→attend 1.4s, `--status` lifecycle — все T2 DOWIZ-FIXED «never overridable»
   (план §8.1). Цей додаток нічого не переносить із Sheet у Sea — він називає якість, яку T2 вже
   структурно захищає, точно як сам оператор `M·Ü+Γ·U̇+c²·L·U=S` brand-agnostic.
3. **Кросбрендова легібельність = moat.** Клієнт, що вивчив мову замовлення в одній venue, мусить
   читати її в кожній («lifecycle identical every brand», T2 `--status`). Кінограматика працює так
   само: зріз і затримана склейка означають те саме в кожному фільмі; студія обирає палітру й акторів,
   не значення монтажу. Бренд, якому дозволили «зробити драматичніше», зламав би словник для всіх.
4. **Чесність стану.** Біти дуги прив'язані до реальних доменних подій (kernel fold, local validate).
   Бренд-ручка темпу дозволила б розіграти напругу, якої стан не має (fake urgency), або приглушити
   провал — та сама причина, чому money never tweens архітектурно, а не конвенційно (план §2.5).

**Steelman розглянуто і відхилено:** «спокійний fine-dining vs гучний fast-food — нехай бренд обирає
темп». Відповідь: бренд-регістр уже входить через 5 токенів — hue/chroma акценту, type-pair, radius
перефарбовують ОДИН і той самий рух (теплий terracotta читається інакше, ніж холодний slate, на
ідентичній хвилі); а темп уже модулюється СЕМАНТИЧНО, не брендово: attending 6s→1.4s, energy scalar,
open-hours calm-dark — ручки СТАНУ, не смаку. Шостий токен «drama/pacing» відкрив би arms race уваги
(та сама причина відмови від per-component overrides — Hick's для owner) і зруйнував би «coherence by
construction ~95% shared» (інваріант B-9). **Вердикт: NO шостому токену.** Кінематографічна якість —
медіум, не фіча з меню бренду. Наслідок для DZ-09 Branding: full token editor НЕ отримує поля
motion/drama/pacing; спроба додати = порушення інваріантів B-6/B-9 і gate DZ-02 (owner touches 5).

### C.2 Наративна дуга замовлення — pacing beats на наявній механіці

Життєвий цикл замовлення — не flat progress bar, а дуга з бітами. Кожен біт = вже-наявне джерело
VOCAB (DZ-04/05) плюс явна темпоральна вказівка. Інструменти — тільки наявні: ζ-пресети
(`motion.rs:66-76` snappy/fluid/playful + TIDE/SPREAD план §8.2), `heat_kernel_delay` (стагер),
settle gate FE-14 (тиша, вимірювана), spectral attending speed (urgency), energy scalar (гучність
партитури). Жодного нового примітива — хореографія наявних джерел.

| Біт | Подія | Field (наявний VOCAB) | Темп (нове ІМЕНУВАННЯ, не нова механіка) |
|---|---|---|---|
| Inciting | order placed | amber burst 1.4 | SNAP-удар МИТТЄВО (інваріант B-7: consequence ≤ `--motion-instant` — підтвердження на Sheet ніколи не чекає); далі **held beat**: Море ~600-900ms осідає БЕЗ нових імпульсів — вага рішення = тиша ПІСЛЯ підтвердження, не затримка підтвердження |
| First answer | confirmed | ember drift 0.3 | одна низька хвиля FLUID — тихе запевнення, НЕ фанфари; світ відповів |
| Long middle | preparing / ready | ember drift 0.3 | найтихіший відрізок дуги: TIDE ζ>1 ambient, settle gate активний (0 rAF wake-ups) — впевнена нерухомість; терпіння чесне, бо ETA — range, не єдине число |
| Tempo cut | courier assigned / picked up | teal swirl 1.6 | зміна регістру = зміна КІНЕМАТИКИ (реальний рух кур'єра, GPS flow DZ-06), не додаткова анімація; spectral attending пришвидшується 6s→1.4s |
| Tracking shot | in transit | field flow along route | камера = focus well на маркері кур'єра (FE-11); ETA падає → attending speed росте, НЕ мигання/пульсація |
| Climax | delivered | **gold bloom burst 1.8** | ЄДИНИЙ максимум амплітуди успішної гілки — зароблений попередньою стриманістю; потім видих: поле → calm, settle gate забирає кадр (denouement = тиша) |
| Tragedy (чесна) | rejected / cancelled | blood turbulence 3.4 | без пом'якшення І без соромлення користувача; після shake — recovery beat 300-600ms тиші перед наступним CTA |

**Амплітудний бюджет (falsifiable, у межах gate DZ-04):** на успішній гілці жоден проміжний біт не
гучніший за Delivered — max amplitude per state монотонно накопичується до клаймаксу. Held beat
present (поле без нових source-імпульсів N ms після order_created burst) — перевіряється тим самим
event-log'ом DZ-06. Той самий шаблон успадковують courier (task offered = один incoming ripple → run
→ SwipeToComplete gold bloom, DZ-08) і owner (order created ripple → lifecycle actions → settled,
DZ-09) — три ролі, одна драматургія, бо дуга належить домену, не ролі.

### C.3 Кінематографічний grade — на НАЯВНІЙ OKLCH-механіці, нуль нових токенів

Наявне: `--spectral` oklch terracotta→accent→gold (план §2.4), `--sea-backdrop` = color-mix(brand-bg
12% #060402), `--sea-tint` = brand-primary, terracotta→gold color travel (DZ-04), open-hours
calm-dark (DZ-06). Це ВЖЕ grade: tracking-дуга і є color grade, розгорнутий у часі.

Розширення (T2, dowiz-fixed): **grade = пара скалярів (ΔL, ΔC) поверх Sea-кольорів як f(біт)**.
OKLCH тут не випадковість, а причина: L і C зсуваються рівномірно БЕЗ спотворення brand-hue
(перцептивна рівномірність) — регістр змінюється, бренд лишається собою. Establishing (Act 1) — ледь
притемнено/десатуровано; confirmed — теплий підйом L; delivered — gold lift (вже є); closed — calm
dark (вже є). Імплементація = ті самі color-mix відсотки + VOCAB кольори; жодного нового brand-токена.
**Межа: grade живе ТІЛЬКИ на Sea (під Sheet).** Sheet ink/paper НІКОЛИ не graded — контраст контенту
(AA validated, T1) не сміє їздити на драматургії: гроші/слова/рішення читаються однаково в кожному біті.

### C.4 Мова камери — що безкоштовне, що чесно коштує

Безкоштовне (мапиться 1:1 на наявні примітиви; лише іменування):
- **Establishing shot** = Act 1 arrive: full-bleed Море, hero = найглибший well (3-second hierarchy,
  план §8.4-4) — це DZ-01, назване кінотерміном.
- **Cut / dissolve** = dive (ring-wipe, anchored on pressed button) / sheet-rise (DZ-03) — це і є
  вся монтажна граматика; інших «склейок» не існує і не треба.
- **Close-up / rack focus** = V(x) potential well (FE-11) + його readouts scale/brightness/blur/
  saturation (план §2.2); переміщення well = rack focus.

Чесно коштує (НЕ будувати зараз; окреме costed-рішення, якщо колись знадобиться):
- **Full-scene depth-of-field** — post-process blur pass поверх кадру (новий render target поверх
  P38a G3) — не покривається single-well blur readouts. Named cost, not free.
- **3D-камера / dolly / parallax** — поле 2D (`FieldPos{u,v}`; `w` = AR panel-normal, НЕ глибина
  сцени, DZ-12). Будь-який perspective transform = нова система координат і новий render шлях.
  REJECTED for now; splatting-арка — окрема і НЕ є цим.

### C.5 Стриманість — несуча дисципліна, не примітка

«Кінематографічний» НЕ означає максималізм чи attention-grabbing. Прочитання для виконавця, прямим
текстом, щоб «cinematic» ніколи не читалося як «додай анімації скрізь»:

1. **Кінематографічність — критерій ВИДАЛЕННЯ так само, як додавання.** Рух, що не сидить на
   іменованому біті дуги (C.2) або в словнику feedback (DZ-05), — кандидат на видалення. Idle-декор
   порушує settle gate (FE-14: 0 rAF wake-ups settled — тиша ВИМІРЮВАНА, не побажання).
2. **Частотне правило:** дії, які роль виконує десятки/сотні разів на день (courier accept, owner
   lifecycle actions, keyboard-ініційовані) — SNAP або без анімації взагалі; delight-біти лише на
   рідкісних термінальних моментах (delivered, first-run). Earnings/money — ніколи (вже закон, B-2).
3. **Клаймакс заробляється тишею.** Gold bloom вражає РІВНО тому, що preparing був майже нерухомий;
   якщо все «кінематографічне» — ніщо не кінематографічне. Амплітудний бюджет C.2 — механічна форма
   цієї тези.
4. **Exit швидший за enter; recovery space після помилки; reduced-motion — перший клас:** дуга
   РОЗПОВІДАЄТЬСЯ і статично (pills/color/text, інваріант B-9 плану §8.4-9) — наратив не залежить
   від руху, рух лише його виконує.

Це узгоджено з brand-voice canon (Warm Cosmo-Noir): noir — це передусім тінь і пауза; іронія — у
стриманості, не в спецефектах. Голос dowiz у ЧАСІ = ця драматургія; Sheet-токени лише вдягають її.

### C.6 Прив'язка (нуль нових одиниць, нуль нових токенів)

Цей додаток НЕ додає DZ-13. Він нормативно уточнює наявні одиниці: DZ-01 (акти = establishing/arc
shell), DZ-03 (dive/rise = єдина монтажна граматика), DZ-04 (дуга C.2 + амплітудний бюджет + grade
C.3 — gates розширюються тут, не в новій одиниці), DZ-05 (біти = Green's sources з темповою
вказівкою), DZ-08/09 (успадкування дуги per-role), DZ-09 Branding (no pacing field — C.1). Інтент-
рамка DZ-10 без змін: біт відповідає на Intent однаково для touch/voice/gesture — кінематографічна
якість = ЯК поле відповідає на намір у часі, і саме тому вона живе у двигуні, а не в скіні.

---
*Кінець блюпринтів. 12 робочих одиниць DZ-01..12, 4 хвилі, стоять на engine FE-01..17. Джерело:
DOWIZ-INTERFACES-PLAN.md + 4 інвентарі коду + 2 дослідження + референс-артефакт. Критерій — когерентність +
local-first + збереження кожної фічі + falsifiable gate. Автор синтезує; виконують агенти.*