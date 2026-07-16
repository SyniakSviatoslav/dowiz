# DOWIZ — ПЛАН ДИЗАЙНУ ВСІХ ІНТЕРФЕЙСІВ v1.0
## Дизайн-мова «Sea & Sheet» на фізичному двигуні: client · owner · courier, local-first, кросплатформно, мультимодально
### Синтез: філософія Dowiz × референс-артефакт × повний інвентар фіч × field-UI двигун × pgrust/DOD/egui/wgpu. Критерій — когерентність, реалізовність локально, збереження кожної наявної фічі.

> **Що це.** Один канонічний план дизайну для КОЖНОГО інтерфейсу dowiz (client/owner/courier), який стоїть
> **на плечах** сьогоднішнього плану field-UI двигуна ([FIELD-UI-ENGINE-PLAN](../field-ui-engine/FIELD-UI-ENGINE-PLAN.md)).
> Дизайн-мова — це **шар досвіду**, який реалізується двигуном: море = поле `M Ü+Γ U̇+c²LU=S`; переходи =
> ζ-демпфована motion; feedback = функція Гріна; brand content = SDF-рендер шар; дані = Rust-native
> local-first event-log (у дусі pgrust). Усе рендериться **локально, без серверів**; готове до web, laptop,
> AR-окулярів; керується дотиком, голосом, жестами.
>
> **Джерела.** 4 інвентарі коду (client+courier фічі, owner+admin фічі, хаб/стейт/замовлення+реактивність,
> дизайн-система) + 2 дослідження (мультимодальність+кросплатформність, операціоналізація дизайн-мови) +
> референс-артефакт «Artë Pasta: arrive→choose→receive». Конспекти: [design-reports/INDEX.md]. Поточний
> стан і кожна наявна фіча звірені на диску.
>
> **Незламний контракт.** Жодна реалізована фіча старого інтерфейсу не втрачається (майстер-чеклісти §7).
> Money ніколи не tween. Local-first: render-loop ніколи не торкається сервера. Гібрид: DOM лишається для
> a11y-mirror / text-input / SSR-menu (це архітектура, не тимчасовість).

---

## 0. ГОЛОВНА ІДЕЯ І П'ЯТЬ ІСТИН

**Головна ідея (доведена дослідженням дизайн-мови).** Кожен інтерфейс dowiz — це рівно дві речі, покладені
одна на одну:

```
┌─────────────────────────────────────────────────────────────┐
│  BRAND CONTENT LAYER — «Sheet / Папір»  (brand-owned)        │
│  меню, картки, форми, таблиці, слова, ціни, РІШЕННЯ           │
│  рендериться SDF-content-шаром двигуна (crisp text/shapes)   │
├─────────────────────────────────────────────────────────────┤  ← spectral edge (шов, dowiz-signature)
│  DOWIZ AMBIENT LAYER — «Sea / Море»  (dowiz-owned, tinted)   │
│  arrival · transitions · tracking · feedback · focus         │
│  рендериться ПОЛЕМ двигуна (M Ü+Γ U̇+c²LU=S)                 │
└─────────────────────────────────────────────────────────────┘
```

**Море — це поле двигуна.** Референс-артефакт уже це довів: WebGL2 Gerstner-хвильове море, чия **амплітуда
росте і колір іде terracotta→gold зі станом замовлення**, dive-перехід із рефракцією/каустиками, ripples на
дотик — усе локально, без сервера. Це буквально `field-UI` теза, реалізована. Наш план бере це і робить
**універсальним для всіх ролей і всіх брендів**.

### П'ять істин синтезу

1. **Двигун уже існує на 40% і море вже реалізоване.** `particle-cloud.js` = DOD ring з event→physics VOCAB
   (order→amber, delivered→gold, failed→blood-swirl); референс-артефакт = повне хвильове море зі
   state-driven амплітудою/кольором. Дизайн — це **дротування** цих активів до всіх екранів + добудова
   SDF-content-шару.

2. **Стан замовлення вже локальний і вже мапиться на фізику.** Order state machine (10 станів) дзеркалена в
   `channel.js` → браузер валідує transition **без сервера**; VOCAB уже мапить статус→колір/енергію/swirl.
   Terracotta→gold «море дозріває зі станом» = пряме продовження наявного коду.

3. **Дизайн-система перелічувана → GPU-таблиця токенів; owner торкається лише 5.** Opinionated software:
   один ідеально-відкалібрований варіант. Owner задає accent/ink/paper/type-pair/radius; усе інше
   (spectral/sea/field/money/motion/grid/status) — dowiz-fixed. Один accent → 4 когерентні місця (Sheet
   direct / Sea tint / spectral re-derive / backdrop) без ручного тюнінгу.

4. **Усе — це field impulse.** Кожна модальність (дотик/голос/жест/погляд/контролер) на кожній платформі
   нормалізується в один `Intent` у `FieldPos(u,v)`-просторі; `field.apply(&intent)` = ОДИН code path.
   Саме це робить мультимодальність когерентною і кросплатформність write-once.

5. **Чесна межа — гібрид, і money священні за конструкцією.** Поле НЕ тримає exact-align (measure-zero),
   crisp-text (band-limited), money/selection (discrete) → constraint solver + SDF + kernel state. Money
   живе на Sheet у `<Money>` (integer з kernel, немає tween-пропу — count-up структурно недосяжний). DOM
   лишається для a11y-mirror/text-input/SSR-menu.

---

## 1. ЯК ЦЕ СТОЇТЬ НА FIELD-UI ДВИГУНІ (мапа дизайн→двигун)

Кожен дизайн-концепт реалізується конкретним примітивом двигуна. Це не дві системи — це **одна**: двигун
рендерить, дизайн-мова диктує, що саме.

| Дизайн-концепт (Sea & Sheet) | Примітив двигуна (field-UI) | Блюпринт двигуна |
|---|---|---|
| **Море (ambient field)** | Поле `M Ü+Γ U̇+c²LU=S`, damped-wave + heat-kernel | FE-04 (particle→wgpu), FE-07/08 |
| **Амплітуда/колір ростуть зі станом** | `S(t)` source = OrderStatus; TIDE motion (ζ>1) | FE-08 motion field + reactivity |
| **Dive-перехід** | ζ=1 critical motion + Green's impulse + shader composite | FE-08 + FE-04 |
| **Sheet-rise** | FLUID (ζ≈0.8) spring на позиції панелі | FE-08 |
| **Feedback (ripple/bloom/shake)** | Функція Гріна `U=G∗S`, event→source vocab | FE-10 |
| **Focus (hero, emphasis)** | Потенційна яма `V(x)`; scale/brightness = readouts | FE-11 |
| **Drill-down layout / split-pane** | Spectral embedding φ₂φ₃ + force layout | FE-07 + FE-12 |
| **Sheet (brand content, crisp)** | SDF shape pipeline + MSDF text | FE-05 + FE-06 |
| **`<Money>` (never tween)** | field↔state boundary + kernel integer | FE-09 |
| **Токени (accent/spectral/…)** | GPU token table, per-frame UBO | FE-05 |
| **Дані local-first** | Rust-native event-log (pgrust-дух) + fold replay | новий (§4) |
| **Мультимодальний ввід** | `Intent`/`FieldPos` + `InputSource` trait | новий (§5) |
| **Battery / lazy-render** | render-on-settle = resonator Converged | FE-14 |
| **DOD / fixed-timestep** | SoA store + accumulator DT=0.02 | FE-02 + FE-03 |
| **Zero-copy рендер** | Float32Array view + writeBuffer | FE-01 |
| **A11y / forms hybrid** | прихований DOM semantic mirror + `<input>` overlay | FE-15 |

**Ключ:** дизайн-блюпринти цього документа (DZ-*) **не дублюють** engine-блюпринти (FE-*) — вони їх
**споживають** і додають дизайн-семантику (що море робить для замовлення, як виглядає Sheet бренду, які
екрани яких ролей). Виконавці роблять FE-* (двигун) → потім DZ-* (досвід поверх нього).

---

## 2. ДВОШАРОВА АРХІТЕКТУРА — BACKBONE КОГЕРЕНТНОСТІ

### 2.1 Правило приналежності (запам'ятати)

| Питання | Так → | Ні → |
|---|---|---|
| Ambient / physical / transition / tracking / feedback? | **МОРЕ** (dowiz-fixed physics, brand-tinted) | — |
| Content / слово / ціна / форма / рішення? | — | **SHEET** (brand-authored) |
| Motion між станами або у відповідь на дію? | **МОРЕ** (поле несе енергію) | — |
| Число, особливо money? | **SHEET, і воно НІКОЛИ не рухається** (kernel value, presented not interpolated) | — |
| Шов між ними? | **SPECTRAL EDGE** (dowiz signature, brand-tinted) | — |

### 2.2 Море (Layer A) — dowiz IP/moat

Одне неперервне поле під контентом на **кожному** екрані кожної ролі кожного бренду, tinted до venue accent.
Володіє: **arrival** (Act 1 entrance), **transitions** (dive/sheet-rise/page), **tracking** (амплітуда+колір
зі станом), **feedback** (Green's function: tap→ripple, success→heat bloom, error→high-λ shake,
loading→sustained source), **focus** (одне `V(x)` драйвить scale/brightness/blur/saturation як readouts).

### 2.3 Sheet (Layer B) — venue world

Непрозора поверхня що піднімається над Морем і тримає рішення. Володіє: **content** (меню/картки/форми/
таблиці/слова), **brand identity** (accent/ink/paper/type/radius), **decision** (tabs/chips/stepper/checkout).

### 2.4 Spectral edge — шов

Один dowiz-підпис: `--spectral` oklch градієнт terracotta→accent→gold, re-derived per brand. Верхній край
кожного Sheet, ring-wipe кожного dive, progress-thread tracking. Пришвидшується коли surface **attending**
(active/loading/awaiting input). Єдиний dowiz-mark на brand Sheet.

### 2.5 Money-guarantee (архітектурна, не конвенція)

Межа field↔state двигуна (FE-09) означає: Море **презентує** kernel-decided integer-cent і **не може**
інтерполювати. Money живе на Sheet у `<Money>` компоненті (`--font-mono` tabular-nums), у якого **немає
tween-пропу** — count-up структурно неможливий. Enforced 3 способи: field↔state boundary + `<Money>` no-prop
+ ESLint no-number-anim on price. **Це виправляє 4 legacy money-tween порушення (ClientLayout/EarningsPage/
DashboardPage/AnalyticsPage) за конструкцією.**

---

## 3. РЕАКТИВНІСТЬ: ІНТЕРФЕЙС ВІДПОВІДАЄ НА НАМІР/ПОДІЮ/СТАН/СЕРЕДОВИЩЕ

Море — не декор; це **державний осцилограф** досвіду. Кожен сигнал стає source-збуренням поля. Повний
каталог (з інвентаря хаба):

### 3.1 INTENT (дія користувача/жест/голос)
| Сигнал | Field-відповідь |
|---|---|
| toggle modifier | локальний ripple у cart-locus |
| add to cart | ingest-pulse до cart-attractor + cart-pill exact-value snap (НЕ count-up) |
| change qty | маса cart-регіону ∝ subtotal |
| submit order | **order_created amber burst** (energy 1.0, burst 1.4) |
| pointer/point (finger/gesture) | **live repulsion** = moving Dirichlet test particle у well |
| voice command | resolver → той самий Navigate/Select що finger → поле анімує ідентично |

### 3.2 EVENT (доменні події — terracotta→gold гачок)
Пряме продовження наявного VOCAB. Замінити discrete 24-particle burst на **continuous field, чиї
color/energy/swirl = f(OrderStatus)**:
| OrderStatus | Field |
|---|---|
| Pending/Confirmed/Preparing/Ready | ember drift (energy 0.3, slow — «очікування») |
| InDelivery / PickedUp | teal courier stream (swirl 1.6 = tangential courier motion) |
| Delivered | **gold bloom** (burst 1.8 — terminal success) |
| Rejected / Cancelled | blood turbulence (swirl 3.4) |
| illegal transition | hard red recoil shock |
| anomaly count (owner) | field agitation ∝ anomaly |
| channel attribution (owner) | per-channel attractor mass ∝ count |

### 3.3 STATE (реактивні стори)
Field-global `energy` scalar = **температура поля** (найважливіша ручка): cart/shift/availability модулюють
energy + region mass. Kernel event-log = source of truth (replay `fold_transitions` локально).

### 3.4 ENVIRONMENT (середовище — 3 gaps добудувати)
| Сигнал | Стан | Field-відповідь |
|---|---|---|
| reduced-motion | ✅ є | Море → static tinted gradient, стан легібельний via pills/color/text |
| dpr / resize | ✅ є | field resolution adapt |
| `?ch=` channel | ✅ є | order.channel attribution |
| **online/offline** | ❌ **добудувати** | field syncs (online) vs pure-local (offline) — indicator у Морі |
| **open-hours / time-of-day** | ❌ **добудувати** | ambient field brightness/dormancy (закрито = calm dark sea) |
| **GPS / courier location** | ❌ **добудувати** | marker kinematics = tangential field flow along route |
| device/orientation/battery | auto-sleep idle | lazy-render-on-settle (FE-14) |

**Ці три gaps — additive:** listener `navigator.onLine`, open-hours модель (керує ambient яскравістю Моря),
geolocation feed (courier marker kinematics). Жоден не існує сьогодні; усі — нова робота (§DZ-блюпринти).

---

## 4. LOCAL-FIRST ДАНІ (Rust-native, дух pgrust) — РЕНДЕР НІКОЛИ НЕ ТОРКАЄТЬСЯ СЕРВЕРА

Підтверджено інвентарем: render-path уже local-first (Astro SSG + in-browser WASM kernel; order create/advance
не торкаються мережі). План закриває залишок:

- **Event-log локально** (дух pgrust = Rust-native infrastructure): kernel event-sourced core уже є; персистити
  event-log client-side (OPFS+SQLite-WASM браузер / native SQLite rusqlite — вже bundled у server crate) і
  **replay `fold_transitions` на завантаженні** реконструює канонічний стан без round-trip.
  - > ⚠ CORRECTED (operator, 2026-07-16): dowiz does NOT use SQLite as an architectural choice. The spectral/sqlless
    > approach — content-addressed `BlockStore` + JSONL `FileEventStore` (`kernel/src/backup.rs`, `kernel/src/event_log.rs`)
    > — is the MAIN storage/retrieval path in dowiz's own kernel/engine, with **pgrust as the uniform SQL-fallback/backup
    > target, not SQLite**. The `OPFS+SQLite-WASM браузер / native SQLite rusqlite` phrasing here (and the recap
    > "local-first OPFS-SQLite" in §"Головні числа" below) is superseded: the client event-log persists via the
    > content-addressed BlockStore + JSONL event-log pattern over OPFS (a pgrust-backed table only if the shape is
    > genuinely relational), replayed by `fold_transitions` — never SQLite-WASM/rusqlite as the store engine.
- **Меню/scene-graph/cart/order-in-progress** = 100% локальні, резидентні в пам'яті, field читає щотіка
  (money-поле, well-глибини).
- **Сервер = async out-of-band sync peer**, не render-dependency: submission/payment/dispatch за **outbox**,
  що drain'иться коли `navigator.onLine`. UI повністю інтерактивний офлайн; лише final commit чекає connectivity.
- **Транспорт live-updates:** канонічний dowiz не має (poll-only); bebop `WssTransport` готовий+тестований
  (hybrid Ed25519+ML-DSA-65 gate, channel-binding anti-replay), iroh QUIC = drop-in stub. Field підписується на
  `Transport::recv()` потік signed order-event frames → кожен verified → `fold` step → physics burst.
- **CRDT sync (пізніше, cross-device):** Automerge 2.0 (Rust ~10× швидший) — offline-first, converges без
  координатора. ⚠️ money-merge обережно (naive schema → double-charge).

---

## 5. МУЛЬТИМОДАЛЬНІСТЬ І КРОСПЛАТФОРМНІСТЬ (одна абстракція)

### 5.1 Усе — `Intent` у `FieldPos`

```rust
pub struct FieldPos { u: f32, v: f32, w: f32 }  // [0,1]², w=panel-normal (AR)
pub enum Intent { Point{p,strength,id} | Impulse{p,force,amp} | Select{target,p}
                | Navigate{axis: Dive|Surface|Lateral} | Scrub{p,delta} | Command{verb,object,slots} }
pub trait InputSource { fn poll(&mut self, ctx:&FrameCtx) -> SmallVec<[Intent;4]>; fn kind()->Modality; fn confidence_floor()->f32; }
// InputRouter.tick → field.apply(&intent) — ОДИН code path, physics ідентична всім модальностям
```

### 5.2 Модальності (усе on-device, local-first)
- **Дотик/pointer** (winit/canvas) → screen→FieldPos → Point/Impulse/Scrub.
- **Голос** (local): wake-word (Porcupine ~1MB) → Moonshine (sub-second English) / Web Speech `processLocally`
  / whisper.cpp (multilingual sq/en/uk fallback) → **command grammar** (не free dictation) → Intent::Command
  → resolver → Navigate/Select. Web Worker pipeline, ніколи main thread. «покажи пасту»→Dive(pasta),
  «додай cacio e pepe»→Select(dish) well-collapse, «оформити»→Dive(checkout).
- **Жести** (MediaPipe on-device, 21 landmarks): point=Point (Dirichlet particle), pinch=Select (well-collapse),
  open-palm push=Impulse (amplitude), swipe=Navigate. Gate як wake-word (battery). AR headset: WebXR Hand
  Input direct (25 joints).
- **Погляд/pinch** (Vision Pro) → gaze-ray→Point, pinch→Select.

**Multimodal fusion trivial:** голос «cacio e pepe» (Command) + finger point (Point) на той самий регіон →
одна well → **reinforce**. Це виграш абстракції.

### 5.3 Кросплатформність (write once → any device incl. glasses)
- **SHARED (100% Rust):** field solver + DOD SoA + fixed-timestep + Intent + WGSL shaders + wgpu one API.
- **PLATFORM-SPECIFIC:** windowing (winit desktop; iOS embed wgpu у native view; Android surface-loss handle
  Suspended/Resumed; web canvas; XR no window), input backends, a11y (parallel hidden DOM semantic tree —
  non-optional WCAG), persistence.
- **AR/spatial:** field → off-screen wgpu texture → curved/cylindrical panel floating XR («меню в просторі»),
  той самий shader+DOD+1 vertex transform, `FieldPos.w`=panel-normal (crests pop toward user). Input=ray:
  hand-ray/gaze-ray → intersect panel → hit (u,v)=FieldPos (ідентично 2D pointer!). money = pinned mono panel.
- **Ship order:** flat web (WebGPU+WebGL2 fallback) + native desktop → voice+gestures → native OpenXR Quest →
  WebXR-WebGPU коли XRGPUBinding lands (draft червня 2026). ⚠️ НЕ блокувати на WebGPU-XR binding.

---

## 6. ТРИ АКТИ — УНІВЕРСАЛЬНИЙ SHELL ДЛЯ ВСІХ РОЛЕЙ

Клієнтська подорож arrive→choose→receive — не клієнтська фіча, а **shell-граматика dowiz**, яку успадковують
усі три ролі. Три акти = три URL-стани = drill-down story з робочою кнопкою «назад».

| | Act 1 — ARRIVE (Море спокійне) | Act 2 — CHOOSE (Sheet піднімається) | Act 3 — ACT/RECEIVE (Море дозріває) |
|---|---|---|---|
| **CLIENT** `/s/:slug` | Hero над хвильовим морем; top items (lazy path Pareto 80/20) | Menu Sheet: category tabs+counts + search + sort chips (price/protein/kcal) + allergen filter; dish → bottom-sheet(mobile)/modal(desktop) з ingredients+nutrition+modifier-groups+stepper | `/s/:slug/track/:order`: море дозріває зі станом (амплітуда+terracotta→gold), step pills, honest ETA range, courier map = поле |
| **OWNER** `/admin` | Dashboard «pulse» — поле Є бізнесом: кожне замовлення = Green's ripple, volume = амплітуда, anomaly = agitation | `/admin/menu`, `/admin/orders`: content Sheet, **split-pane** (nav left / content right) для щільних екранів, accordion progressive disclosure | `/admin/orders/:id`: дія над замовленням → поле ripples state change; venue accent tints Sea |
| **COURIER** `/courier` | Shift/queue — море спокійне, task_assigned = один incoming ripple + ping | Task Sheet: pickup/dropoff timeline, великі `--tap-courier/critical` targets, 60s offer countdown | Navigate/deliver: **МОРЕ Є КАРТОЮ** — CourierTrack поле bound to status; амплітуда/колір travel з delivery progress; SwipeToComplete |

**Універсальні nav-інваріанти:** split-pane для щільності (φ₂φ₃ spectral / 2-col collapse <bp-tablet);
progressive disclosure accordion ≤7 (Hick's); breadcrumbs + URL-state + working-back (load-bearing); кожен
рівень винагороджує клік (category показує top items + brand voice); hybrid служить обом типам (lazy top-items
Act1, curious drills).

---

## 7. ПЕР-РОЛЬ ЕКРАНИ: КОЖНА ФІЧА ЗБЕРЕЖЕНА, ПЕРЕВИРАЖЕНА В SEA&SHEET

Це контракт «не втратити жодну фічу». Кожен екран старого інтерфейсу → як він живе в новій мові. Повні
майстер-чеклісти — у [design-reports конспекті]; тут — мапа екран→акт→sea/sheet-трактування.

### 7.1 CLIENT (13 фіч-груп)
- **Storefront shell** → Sheet chrome: per-tenant palette derive (5 owner tokens → Sea tint + Sheet + spectral
  + backdrop), logo, SunlightToggle, CurrencySwitcher(+EUR), LanguageSwitcher gated. Sticky cart bar = Sheet
  pill з `<Money>` (НЕ AnimatedNumber — snap).
- **MenuPage** → Act 2 Sheet: hero (Google rating+reviews, geo ETA "~N min", StateChip open/closed/busy,
  closed/busy banners) над Морем; category tabs scroll-spy + Chef's Picks ✦; search/sort/allergen-filter
  persisted; responsive staggered grid = **SPREAD** diffusion from tap (items не pop, дифундують); no-results
  reset. **Reactivity:** category select → Sheet re-diffuse; venue closed → Море calm dark.
- **Product detail** → Sheet bottom-sheet(mobile)/modal(desktop): rich media gallery (ADR-0002 lazy+gated),
  cinematic reveal = Green's bloom, kcal/macro/taste-axes/nutrition-grid/allergen-list/ingredients, **modifier
  groups** (radio/checkbox/select/quantity, required/min/max, price deltas), qty stepper, add-to-cart live price
  + toast + cart bounce (ripple) + haptic. Card = tappable, shared-element image morph.
- **Cart engine** → local-first: localStorage per-location versioned + cross-tab sync + dedupe + reconcileToMenu
  (re-price drifted). Feeds field money region.
- **CheckoutPage** → Act 2 Sheet (progressive disclosure): delivery/pickup tabs; contact (name/phone
  Albanian→E.164); optional messenger; entrance photo R2; MapWithPin + "My Location"; entrance/apartment/notes;
  dropoff chips; **cash** amount+change+tip; summary (subtotal/fee-or-"calculated"/tax/tip/`<Money>` total/wide
  ETA/client-mirror ADR-0005); NutritionRing; Wikipedia city-fact; draft persist; place order (idempotency +
  **OTP flow** send/verify/intent-hash + full error matrix MIN_ORDER/NOT_DELIVERABLE/CASH_TOO_LOW/hard_block +
  fallback phone) + success checkmark + push subscribe + privacy notice.
- **OrderStatusPage** → Act 3 **the Sea matures**: status fetch + tracking-link ?t= exchange + **live WS**
  (route/courier-position/status terminal-lock/message) + 30s watchdog; CourierLiveMap tweened+rotated marker
  (bearing) = поле flow along route; **honest ETA range** (never single/0); OrderProgress stepper
  (delivery/pickup branch, timestamps, active pulse) = step pills на Морі; share-my-location; call/message
  courier; terminal CTAs; rating stars+feedback + Google review invite; MessageThread; offline/WS banner.
  **Reactivity core:** OrderStatus → Море амплітуда+terracotta→gold + swirl (§3.2).

### 7.2 COURIER (7 сторінок + shell)
- **Shell** → BottomTabBar Tasks/Earnings/History/Shift; full-bleed delivery/login. Море спокійне на shift.
- **LoginPage / CourierInvitePage** → Sheet forms (email/pw error-shake; invite redeem role-aware + 16-char
  code + validity states).
- **ShiftPage** → Act 1: live HH:MM:SS timer, start/end (Море energy raise on start), on/off pulsing dot,
  today's stats grid, messenger save.
- **TasksPage** → Act 1→2: assignment fetch + real online-status + **WS task_assigned → один ripple + ping +
  dedupe**; accept→delivery/reject optimistic-restore; TaskCard **60s offer countdown + auto-decline** +
  pickup→dropoff timeline. Online/offline empty states.
- **DeliveryPage** → Act 3 **Море Є КАРТОЮ**: live GPS 12s heartbeat + CourierLiveMap (courier/dest/client pins
  + route) = field flow; WS client_location + mid-delivery cancel banner; drop-off card; entry-photo
  thumb→fullscreen; tip; cash breakdown; call/message; mark-picked-up; cash-collected; **SwipeToComplete**
  (keyboard, resets-on-failure) → delivered = **gold bloom**; celebration.
- **EarningsPage** → Sheet: `<Money>` today/week/month (НЕ count-up), payout history + StatusBadge.
- **HistoryPage** → Sheet: completed list (locale date sq/en/uk, 5-star, feedback), empty/error.

### 7.3 OWNER/ADMIN (16 сторінок)
- **Shell** → auth guard + entry-redirect (new→onboarding/draft→activation/published→dashboard); 11-nav
  (sidebar collapsible + mobile bottom-4 + More-sheet-7); logout; currency/lang/sunlight; DEV Flow-Test
  easter-egg.
- **Dashboard/Orders** → Act 1 «business pulse»: 5 KPI (Revenue = `<Money>` НЕ count-up); order search+filter+
  sort+Live/History+CSV+copy-link; **OrderCard** (shortId/timeline-deltas/OTP-verified/reputation-signal/
  masked/rating+feedback/overdue/hollow-guard) з lifecycle actions (Accept→CONFIRMED/Reject→CANCELLED danger-
  confirm/Mark-Preparing/Mark-Ready/Assign-Courier→IN_DELIVERY) = **кожна дія ripples поле**; preset messaging;
  readiness checklist 7; new-order banner + ping + haptic; **WS dashboard** (order.created/status
  regression-guarded merge + PII claim-check debounced refetch, courier position/shift); live courier map
  (NO-PRESENCE ADR-0006). **Reactivity:** new order = Green's ripple, volume = amplitude, anomaly = agitation.
- **MenuManager** → Act 2 split-pane: category CRUD + product CRUD (name/price/prep-time/description/stock/
  taste-5-axes/photo≤5MB) + availability toggle (86/stop-list) + RecipeEditor BOM + MediaManager rich +
  KitchenBusyToggle + MenuScheduleEditor + AI import wizard (upload-modes→preview→commit). ⚠️ **wire dormant
  AllergenEditor publish-gate + ReadinessIndicator** (attestation "cannot publish until allergens declared").
- **Analytics** → Sheet: period 7d/30d; 4 KPI (Revenue `<Money>`); revenue bar chart; top-products drill-down;
  ingredient consumption + reorder; order heatmap; delivery geo-map; CSV. (charts = Sheet content, animate but
  money static.)
- **CRM** → Sheet: customer list + search/sort; contact reveal (audit-gated PII) + redacted CSV; high-value
  badge; customer detail (prefs/orders/heatmap).
- **Promotions** → Sheet: CRUD (code/type/value/min-order/valid-window/max-uses); active toggle;
  Active/Inactive/Expired/Scheduled; usage badge.
- **Branding** → Sheet **the token editor** (критично розширити): наразі 3/10 tokens → додати font-picker +
  radius/spacing + theme presets + skin selector; live phone-frame postMessage preview; auto-generate;
  Google/social. **Owner торкається лише 5 T1 tokens** (accent/ink/paper/type-pair/radius) — усе інше derived
  (§8).
- **Settings** → Sheet: store details; delivery config + zone map; working hours (→ open-hours env signal §3.4);
  language; Telegram (QR/deep-link/targets/test + category preference-centre); fallback phone; delivery
  pause/resume.
- **Couriers** → Sheet: list + active-count + invite (role/16-char-code); detail (earnings/deliveries/shifts);
  order timeline; CSV; live map no-presence. ⚠️ reconcile courier `rating` read vs NO-COURIER-SCORING.
- **Onboarding/Activation** → Act sequence: menu-first (upload PDF/photo → AI parse → pre-fill → claim
  authed/Telegram); activation **trinity gate** (menu/notifications/fulfillment) + pickup toggle + publish
  (gated canPublish + missing[] + live/draft preview + inline edit).
- **Auth** (Login/AuthCallback) → Sheet: email+pw/Telegram/Google/dev; session-expired banner; token storage.
- **Supplies** → Sheet: library CRUD (⚠️ reconcile localStorage-only → local-first event-log §4).
- **FlowTest** (DEV) → lifecycle runner harness.

**⚠️ Redesign flags (з інвентаря — вирішити в блюпринтах):** money-tween → `<Money>` (4 legacy sites);
dormant allergen publish-gate → wire; branding 3/10 → full token editor; supplies localStorage → local-first;
no-presence + rating-read → NO-COURIER-SCORING reconcile; OrderCard no Delivered/Rejected button (courier/API).

---

## 8. ДИЗАЙН-СИСТЕМА: ТОКЕНИ, ζ-МОТИОН, КОМПОНЕНТИ

### 8.1 Токени — 3 tiers (owner торкається лише T1)
- **T1 BRAND-OWNED (5):** `--brand-primary` accent (+auto hover/strong-AA/light), `--brand-text/-muted` ink
  (AA validated), `--brand-bg/surface/raised` paper, `--brand-font-heading/body` type-pair (2 max), `--brand-radius`.
  Hick's law для owner: presets `.theme-*` cover 80%, 5 tokens = power-user.
- **T2 DOWIZ-FIXED (never overridable):** `--spectral` oklch (terracotta→accent→gold) + speeds 6s/attend-1.4s;
  `--sea-backdrop` (color-mix brand-bg 12% #060402) + `--sea-tint`=primary; `--field-c/gamma/mass` (NEW engine
  params як tokens); **`--font-mono` (NEW — real gap!)** + `--money-ink`/`--price-red` #C21A1F (hue-shift only,
  role locked); `--ease-snap` ζ=1 (0.32,0.72,0,1) / `--ease-tide` ζ>1 (0.37,0,0.63,1) / existing ease-out/soft;
  springs motion.ts = (ω,ζ); `--space` 4px / `--text` 8-step / `--tap` 44/48/56 / `--z` / `--safe` / `--status`
  10×4 (DOWIZ-FIXED — lifecycle identical every brand) / `--chart`.
- **T3 DOWIZ INTERNAL BRAND:** product's own chrome (marketing/login/settings/owner-tool-frame) = Warm
  Cosmo-Noir default (#d69a3d/#061b1a/#f5efe5) + data-skin=paper. Rule: **Sea tints active venue; Sheet wears
  content-owner brand.** Один accent → 4 placements (Sheet direct / Sea tint / spectral re-derive / backdrop)
  coherent zero manual.

### 8.2 ζ-словник (єдине джерело feel, ніколи raw cubic at call site)
SNAP ζ=1 (default UI, taps/toggles/hero) · FLUID ζ≈0.8 (entrances/sheet-rise) · PLAYFUL ζ≈0.5 (RARE delight,
cart-success) · **TIDE ζ>1 (THE SEA — drift/amplitude-growth/terracotta→gold, ambient never-settles)** · SPREAD
heat-kernel (reveals/theme-swap, items diffuse from tap). Standing law: **«don't be shy animation, but money
never tweens»** (Sea generous, Sheet numbers frozen); air > expensive animation; exit faster than enter;
reduced-motion first-class.

### 8.3 Компонент-граматика (tiny vocab repeated, strict palette)
field-backdrop (Sea) · sheet (26px spectral-edge grip) · spectral-edge · grip · card (one idea, cardEntry crisp
never bounce) · stepper · chip · pill (`--status-*` only) · **`<Money>`** (mono tabular kernel-integer, NO tween
prop) · tab-bar (drives SPREAD) · ripple/particle (2 renderers 1 field source). Palette discipline: **no new hue
per screen**. Air law: negative space = component (min pad space-4 card / space-6 sheet / space-8 section).

### 8.4 9 coherence-правил (mechanically checkable)
(1) Sea always dowiz; (2) spectral edge threads every flow; (3) money sacred (mono+tabular+integer+never-tween+
never-round, 3-way enforced); (4) 3-second hierarchy (one hero deepest well); (5) air ratios + limited palette;
(6) ζ governs all motion; (7) consequence ≤--motion-instant network-indep; (8) drill-down story + reversible
URL-state; (9) reduced-motion never loses meaning.

---

## 9. ФАЗИ ПОБУДОВИ (кожна = робоча система + gate)

Двигун (FE-*) — передумова; дизайн (DZ-*) поверх. Beachhead = канонічний `web/` (Astro/Svelte+WASM). Порядок:

- **Ф0 — Двигун-фундамент (FE-01..06):** zero-copy міст, SoA store, fixed-timestep, particle→wgpu, SDF+tokens,
  MSDF text. **GATE:** референс-море (Gerstner wave shader) рендериться через wgpu zero-copy; один Sheet-екран
  (Storefront card) pixel-точно.
- **Ф1 — Sea & Sheet backbone (DZ-1..3):** two-layer shell (`<Shell>` три акти всі ролі), token 3-tiers +
  `<Money>` + ESLint no-tween-price, spectral edge. **GATE:** dive+sheet-rise transitions; money snap ніколи
  tween (RED 4 legacy sites); brand accent → 4 placements coherent.
- **Ф2 — Поле-реактивність (DZ-4..6):** OrderStatus→Море (terracotta→gold amplitude/swirl), Green's feedback,
  potential-well focus, local-first event-log + replay. **GATE:** статус advance → море дозріває; add-cart →
  ripple + money snap; закрито → calm dark sea; offline → pure-local render.
- **Ф3 — Пер-роль екрани (DZ-7..9):** client menu/checkout/track, courier flow, owner dashboard/menu/... — усі
  фічі з майстер-чеклістів. **GATE:** кожна фіча з чеклісту присутня; WS live tracking; OTP; SwipeToComplete;
  owner lifecycle actions ripple поле.
- **Ф4 — Мультимодальність + кросплатформ (DZ-10..12):** Intent/FieldPos + InputSource, voice (Moonshine+
  grammar), gestures (MediaPipe), a11y-mirror + input-overlay, WebGL2+native fallback. **GATE:** voice "add X"
  = same field response as tap; screen-reader reads mirror; WebGL2-only device renders; AR panel (ray→FieldPos).

---

## 10. ПІДСУМКОВИЙ ПАСПОРТ

**Система:** дизайн-мова «Sea & Sheet» для всіх інтерфейсів dowiz на фізичному field-UI двигуні. Море (поле
двигуна, dowiz-owned brand-tinted) несе arrival/transitions/tracking/feedback/focus; Sheet (SDF-content,
brand-owned) тримає content/identity/decision; spectral edge = шов. Local-first (render ніколи не торкається
сервера), кросплатформно (web/laptop/AR через одну Rust-codebase), мультимодально (touch/voice/gesture/gaze →
один Intent).

**Несуча основа (стоїть на):** field-UI двигун (M Ü+Γ U̇+c²LU=S, ζ=1 critical easing, Green's feedback,
potential-well focus, spectral layout, DOD SoA, zero-copy, egui+wgpu, hybrid boundary) + фізика (particle-cloud,
damped wave) + pgrust-дух (Rust-native local-first event-log) + DOD + egui/wgpu.

**Несуча дизайн-логіка:** two-layer Sea&Sheet backbone; 3-tier tokens (owner 5); ζ-motion vocab; three-act
universal shell; meaningful-agency (one hero/3-sec, instant field consequence, presets 80% + power-user, Hick's
grouping); 9 coherence rules; money-guarantee architectural.

**Головні числа:** owner touches 5 tokens; ζ=1 snap / ζ>1 tide (Sea); terracotta→gold зі станом; DT=0.02;
30fps design budget; spectral 6s/attend-1.4s; local-first OPFS-SQLite; `<Money>` mono tabular never-tween;
three acts = three URL states.

**Обсяг:** ~40% фізики + море вже в коді; двигун FE-01..17 = передумова; дизайн DZ-блюпринти = досвід поверх;
кожна наявна фіча (client/courier/owner майстер-чеклісти) збережена і перевиражена; 3 env-gaps
(online-offline/open-hours/GPS) + branding token editor + dormant allergen-gate = нова робота.

**Ключовий інваріант.** Море завжди dowiz; spectral edge threads every flow; money священні й нерухомі;
консеквенція felt ≤ один frame; drill-down = story reversible; reduced-motion ніколи не втрачає сенс. Дизайн —
шар досвіду; двигун — фізика; kernel — істина; бренд — папір. Одна система через ролі, бренди й платформи —
бо фізика, шов і священне число ніколи не міняються; міняється лише папір бренду.

---
*Кінець плану. Версія 1.0. Блюпринти виконавцям — у BLUEPRINTS-DOWIZ-INTERFACES.md. Стоїть на
docs/design/field-ui-engine/. Синтез: 4 інвентарі коду + 2 дослідження + референс-артефакт; критерій —
когерентність + local-first realizability + збереження кожної фічі.*
