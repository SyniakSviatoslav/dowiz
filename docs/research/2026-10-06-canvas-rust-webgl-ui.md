# Повна заміна HTML на canvas: Rust + WebGL рендерить усе — дослідження R-CANVAS

Lane R-CANVAS, 2026-10-06, model Fable (session 264e2624, продовження після смерті main 11753ff4). Дослідницький лейн: код продукту не змінено; прототипи лише в scratchpad `canvas-lab/`. Кожне число — **FRESH** (команда, rc, шлях) або **RECORDED** (джерело + умови); оцінки позначені **ESTIMATE**.

Запит оператора: "запусти дослідження на повну заміну усього html на canvas де раст з webgl усе рендерить сам, яка б це не була вітрина, щоб досягнути ідеальної кросплатформеності, швидкодії й єдиного інтерфейсу без js, html". Поправка того ж дня: **"ніде немає бути DOM"** — нуль DOM на кожній поверхні; варіанти "Rust-генерований DOM" та "status quo + canvas місцями" зняті; "canvas лише для окремих поверхонь" допустимий тільки як крок міграції.

## 0. Десять рядків для оператора

1. **Можливо.** Збудовано прототип екрана меню (165 страв, uk+sq, ціна в лек, смакові смужки, чіпи алергенів, фото-плейсхолдер) — один `<canvas>`, Rust володіє версткою, станом, введенням і малюванням; без wasm-bindgen, без std, без залежностей.
2. **Вага на дроті (FRESH): 12,1 KB gzip разом** (wasm 9,1 KB + лоадер 2,1 KB + html 0,5 KB) проти **155 KB gzip** JS-шелу + **42 KB** CSS сьогоднішньої вітрини. Фреймворк egui/eframe для того самого екрана — **1 140 KB gzip** (у 94 рази більше), Flutter CanvasKit — 1,5 MB brotli.
3. **Старт (FRESH, headless Chromium 153 на A78, програмний растр): перший кадр 81–135 ms** від запуску лоадера, компіляція+інстанціація wasm 3,9 ms у Node. Прокрутка: **1,7 ms p50 CPU на кадр** при dpr 2 (Canvas2D-шлях), 0,6 ms (WebGL-шлях, лише CPU-частина). Сьогоднішня вітрина на тому самому стенді: інтервал кадрів **33 ms p50, 107 зі 120 кадрів довші за 33 ms**.
4. **"Без JS" недосяжний до кінця: мінімум — 2 050 B gzip лоадера** (wasm не можна завантажити без JS: ESM-інтеграція wasm — Stage 3, не відвантажена). Один `<script>` у `<head>`, не в `<body>`.
5. **Нуль DOM у `<body>` = 1 елемент `<canvas>`** — досяжно на всіх поверхнях для: платежів (Stripe Checkout редирект), карти (растрові тайли малює Rust), push/PWA/deep links/share (JS API без елементів), пошуку/копіювання/зуму (власні), тура learn (ідентифікатори вузлів сцени).
6. **Два місця без no-DOM маршруту сьогодні.** (а) **Введення тексту в Safari та Firefox**: EditContext (без прихованого поля) є лише в Chromium 121+; інакше — **1 тимчасовий `<input>` поки поле у фокусі** (eframe робить саме так — виміряно: `CANVAS,SCRIPT,INPUT`). Той самий елемент дає автозаповнення й менеджери паролів. (б) **Скрінрідери**: AccessKit для web "заплановано", AOM virtual nodes не відвантажено, ariaNotify (Chrome 140+) лише озвучує — єдиний робочий шлях (Flutter, Figma) — **дзеркальний DOM**, будований лише коли увімкнено: 1 постійна прихована кнопка + ≈3 вузли на картку за запитом.
7. **Закон.** EAA (2019/882) діє для e-commerce з 2025-06-28; мікропідприємства-постачальники послуг звільнені (ст. 4(5)) — більшість закладів, але не всі, і не платформа. Без дзеркального DOM вітрина на canvas не відповідає WCAG 2.1 AA / EN 301 549. Це не юридична порада.
8. **SEO:** статична HTML-відповідь Worker лише краулерам — "legacy workaround" за Google (ризик cloaking при розбіжності). Це DOM для машин, не наш застосунок; або відмовитись від індексації меню (QR-трафік). OG-мета для прев'ю лишаються в `<head>`.
9. **Вартість (ESTIMATE): 90–130 лейн-днів** на всі поверхні (≈ 21 000 рядків JS сьогодні), з яких 15–25 — власний набір віджетів (текстове поле з виділенням/IME, списки, аркуші, форми, графіки, растрова карта, камера). Попередня оцінка Leptos (DOM залишався) була 45–60.
10. **Рекомендація:** один Rust-рушій сцени, два бекенди (Canvas2D як базовий — системні шрифти, якість тексту як у HTML, вимірюваний на цьому боксі; WebGL2 — для Sea/графіків/карти, той самий текстовий конвеєр через атлас). **Перший лейн — room/kitchen board** (персонал, без платежів, без SEO, без споживчого EAA, майже без тексту) з гейтом "елементів у body = 1" та тестом втрати контексту; 6–10 лейн-днів.

## 1. Фронтенд сьогодні (FRESH, 2026-10-06)

Виміряно з дерева `workers/api/public` (`find`/`wc`, node zlib gzip-9, `brotli -q 11`) і з живого QA-хабу (read-only, headless Chromium 153, 390×844, dpr 2).

| Поверхня | JS файлів / байт / рядків | Шелл на дроті (raw / gzip / brotli) | Елементів у `<body>` (статично / живий рендер) |
|---|---|---|---|
| store (вітрина) | 44 / 362 240 / 6 204 | 59 модулів за sw.js: 380 382 / **155 448** / 131 943; CSS 176 947 / 41 831 / 35 217; index.html 5 387 / 2 189 | 34 / **200** (224 всього, 21 у head, 1 canvas = Sea) |
| admin (консоль) | 137 / 1 189 122 / 14 039 | статичне замикання 11 мод.: 163 058 / 58 779 / 49 763 | 22 / **52** на екрані входу (48 запитів, 499 231 B) |
| courier | 5 / 112 622 | 12 мод.: 164 043 / 61 272 / 52 004 | 31 / **63** (38 запитів, 336 903 B, 1 canvas) |
| room | 24 / 169 150 | 4 мод.: 42 982 / 16 194 / 13 866 | 21 / **43** (58 запитів, 513 434 B) |
| platform (хаб, лендінг) | 3 / 70 565 | 5 мод.: 156 588 / 59 481 / 53 565 | 254 / dowiz.org **328** (21 запит, 1 378 147 B з фільмом) |
| lib (спільне, вкл. MapLibre) | 55 / 1 690 969 | — | — |

Живий storefront qa-durres: **69 запитів, 569 659 B розкодованих тіл** (58 js = 382 883; 5 css = 176 947; html 6 325), кодування на дроті zstd, 3,3 s до networkidle. sw.js 7 716 B (shell-кеш 59 модулів), kit/sw.js 5 160. Якорів `data-tour`: 109. CSP у `public/_headers` (4 836 B) уже містить `'wasm-unsafe-eval'` (W-OCR, 2026-10-04) — wasm на canvas не потребує змін CSP. Гейти UI: ui-adoption, ui-reach, tap-size, langs, sw-shell, learn, no-tracking, icons.mjs, design_gate.py (HTML + linked CSS). Мови: sq/en/uk/ru (рядкові таблиці i18n.js 32,5 KB на вітрині). Офлайн: network-first, шелл у кеші SW.

Тулчейн на боксі: rustup 1.96.1 + wasm32-unknown-unknown (піни репо); wasm-bindgen 0.2.129 і binaryen 133 (aarch64) завантажено в scratchpad сесії; Playwright chromium_headless_shell-1243 є. **WebGL на боксі малює порожнечу** (пам'ять `webgl-renders-nothing-on-the-box`) — підтверджено й тут: контекст створюється, `readPixels` повертає нулі, `isContextLost() = true`.

## 2. Частина A — стан мистецтва: хто живе на canvas і чим платить (CITED)

| Система | Що робить | Чим платить | Джерело |
|---|---|---|---|
| **Flutter web (CanvasKit / Skwasm)** | весь UI на одному canvas; HTML-рендерер знято | CanvasKit 1,54–2,26 MB brotli, Skwasm 1,21–1,86 MB; доступність — **DOM-оверлей** `<flt-semantics-host>`/`<flt-semantics>` з ARIA, вимкнений за замовчуванням "з міркувань продуктивності", вмикається невидимою кнопкою "Enable accessibility" | docs.flutter.dev/ui/accessibility/web-accessibility; startdebugging.net/2026/09/canvaskit-vs-skwasm-for-flutter-web-in-2026 |
| **Figma** | WebGL-полотно, C++/wasm | "Mirror DOM" синхронний з дизайном для скрінрідерів; прихований textarea для IME | figma.com/blog/building-accessibility-into-a-canvas-based-product |
| **Google Docs** (з 2021) | canvas-рендер документа | обіцяє незмінну підтримку AT — реалізовано власним a11y-деревом у DOM | workspaceupdates.googleblog.com/2021/05/Google-Docs-Canvas-Based-Rendering-Update.html |
| **egui / eframe** (Rust) | immediate-mode UI, web через glow (WebGL2) | **2,19 MB raw / 1,14 MB gzip** для нашого екрана (FRESH); web-доступність = `web_screen_reader` (синтез мови), не AccessKit; "cannot search an egui web page"; сам створює прихований `<input>` | docs.rs/eframe; виміряно тут |
| **AccessKit** | a11y-абстракція для Rust-тулкітів | адаптери Windows/macOS/Unix/Android/iOS; **web: "Planned adapters: web (for applications that render their own UI elements to a canvas)"** — немає | github.com/AccessKit/accesskit README |
| **maplibre-rs** | Rust/wgpu векторні тайли | "no Text, Labels, Symbols, Raster"; web потребує WebGPU nightly | maplibre.org/maplibre-rs/docs/book |
| **Makepad, Slint, Iced, Vello, Bevy UI, GPUI** | власні рендерери (десктоп-перші) | web-доступність ніде не вирішена без DOM; Vello = WebGPU (Android: Chrome 121+ на Android 12+ з Adreno 600+/Mali-G78+, compat mode у Chrome 146) | developer.chrome.com/blog/new-in-webgpu-121; cinevva 2026-03-10 chrome-146-webgpu-compatibility |
| **html-in-canvas** (WICG) | малювати DOM-елементи в canvas (`drawElementImage`) | вимагає DOM-дітей canvas; лише Chrome Canary за прапорцем | github.com/WICG/html-in-canvas |

Платформні API, що замінюють DOM (усі — JS-функції без елементів): EditContext (IME на canvas: **Chrome/Edge 121+, Firefox і Safari — ні**, caniuse wf-edit-context; MDN: "The DOM element can be any element, including a `<div>` or a `<canvas>`"), ariaNotify (Chrome 140+, лише оголошення), Clipboard API, Payment Request/ApplePaySession, WebOTP і Contact Picker (Chromium Android), Credential Management (Chromium), Local Font Access (Chrome 103+, не Safari/Firefox), OffscreenCanvas (Safari 17+ повністю, 16.4+ без WebGL у worker), `roundRect` (Safari 16.4+, Firefox 112+), `import source` для wasm (TC39 Stage 3, не відвантажено).

Ринки: Android 12+ (поріг WebGPU) — Албанія ≈ 73 % мобільних (16: 35,7 %, 14: 16,9 %, 13: 14,5 %, 15: 12,0 %, 12: 8,3 %), Україна ≈ 60 % з помітним хвостом старих версій (6.0 — 13 % у червні 2026) — statcounter, липень 2026. **WebGL2 покриває практично всі телефони останніх п'яти років** — тому WebGL2, не WebGPU, є базовим GPU-шляхом; WebGPU лише як прогресивне покращення.

## 3. Частина B — жорсткі обмеження: вердикт, no-DOM маршрут, нередукований мінімум

Визначення для гейта: **"DOM поверхні" = елементи всередині `<body>` у стані спокою.** `<head>` (meta/link/title/script) — метадані документа, не UI; їх рахуємо окремо й чесно.

| # | Обмеження | No-DOM маршрут | Нередукований мінімум | Вердикт |
|---|---|---|---|---|
| B1 | **"Без JS"** | wasm завантажується лише з JS; `import source` wasm — Stage 3, не в браузерах | **1 `<script type=module>` у head, 2 050 B gzip** (FRESH, esbuild-мініфікований лоадер: 4 386 raw / 2 050 gz / 1 844 br; разом з обробниками подій, Canvas2D-викликами й WebGL-програмою) | розв'язно ціною 2 KB JS, який ніколи не росте |
| B2 | **Введення тексту, IME, віртуальна клавіатура** | `canvas.editContext = new EditContext()` — Chromium 121+ (Chrome Android так; **iOS Safari, Firefox — ні**) | **1 тимчасовий `<input>`/`<textarea>` поза екраном, лише поки поле у фокусі** (так робить eframe: виміряно `CANVAS,SCRIPT,INPUT`; так робить Figma) | розв'язно для Chromium; для Safari/Firefox = 1 елемент на час введення |
| B3 | **Автозаповнення, менеджери паролів, SMS-код** | WebOTP `navigator.credentials.get({otp})` і Contact Picker — Chromium Android; Credential Management (паролі) — Chromium; passkeys/WebAuthn — усюди без DOM | той самий тимчасовий `<input autocomplete=…>` з B2 на Safari/Firefox; телефон гостя — WebOTP/Contact Picker на Android, вручну на iOS | розв'язно ціною гіршого UX на iOS (без автозаповнення адреси/телефону, якщо без `<input>`) |
| B4 | **Платежі: карта, 3DS, Apple Pay, Google Pay** | **Stripe Checkout (hosted, редирект)**: "Customers enter their payment details in a fully-featured payment page … via a redirect to a Stripe-hosted page"; Apple/Google Pay, 3DS, Link — на сторінці Stripe; `success_url` повертає до нас (`return_url` уже є у `store/checkout.js`) | **0** у нашій сторінці; зникає й `js.stripe.com` із CSP. Втрата: оплата всередині аркуша; +1 редирект; Elements-таби зникають | розв'язно; в майбутньому ApplePaySession (JS API, 0 DOM) всередині застосунку |
| B5 | **Карта (MapLibre, геокодер)** | растрові тайли: `fetch` → `createImageBitmap` → `drawImage`/текстура — малює Rust; геокодер Nominatim = JSON | **0**; векторні тайли з підписами неможливі (maplibre-rs без тексту) → потрібне **растрове** джерело: tile.openstreetmap.org забороняє "heavy use" застосункам без дозволу OWG → платний провайдер або власний кеш у R2 (вартість — відкрите питання) | розв'язно ціною X = провайдер растрових тайлів + ≈5 лейн-днів на панорамування/зум/маркери/маршрут |
| B6 | **SEO та прев'ю посилань** | OG/`<meta>` у head (прев'ю читають head); для краулерів — окрема статична HTML-відповідь Worker за user-agent, згенерована з тих самих Rust-даних | **0 у body**; ≈10–20 елементів у head; друга відповідь = DOM для машин, не наш застосунок. Google: "no longer recommends… a workaround and not a long-term solution"; cloaking-ризик, якщо зміст розходиться | розв'язно ціною ≈3 лейн-днів (рендер меню в HTML уже планувався як C3) або відмова від індексації меню |
| B7 | **Cloudflare Free, 10 ms CPU** | рендер на клієнті; Worker віддає 3 статичні файли; HTML для краулерів — ≈1–2 ms на 165 карток (ESTIMATE за C3) | — | не обмежує |
| B8 | **Перший кадр і розмір на 3G** | FRESH: 12,1 KB gz усього; перший кадр 81–135 ms на боксі | на 3G (≈ 50 KB/s) ≈ 0,25 s завантаження проти ≈ 4 s сьогоднішніх 197 KB gz | краще за сьогодні на порядок |
| B9 | **Батарея/GPU/пам'ять на слабких телефонах** | Canvas2D-шлях: растр браузера (GPU-прискорений у Chrome Android); WebGL2: 1 draw call, 1 текстура-атлас 2048² RGBA = **16 MB VRAM** (R8 = 4 MB — наступний крок) | пам'ять wasm 2,75 MB (статичні буфери), JS-купа 10–14 MB (FRESH); **на телефоні не виміряно** | невиміряно тут; гейт на реальному телефоні обов'язковий |
| B10 | **Втрата WebGL-контексту** (фон, GPU reset, забагато контекстів) | `webglcontextlost` → `preventDefault`, `webglcontextrestored` → перестворити програму/буфери/атлас; сцена в Rust незалежна від GL — перемалювати 1 кадр | — | розв'язно; тест із `WEBGL_lose_context` у гейт |
| B11 | **Пошук на сторінці, виділення/копіювання, зум, масштаб шрифту** | власний пошук; виділення малює Rust, копіювання — `navigator.clipboard.writeText` (жест); pinch — `touch-action:none` + масштаб сцени; зум браузера = зміна dpr → перемалювання; системний масштаб шрифту — читати `font-size` кореня (`getComputedStyle(document.documentElement)`) | **0 елементів** | розв'язно ціною ≈3 лейн-днів; Ctrl+F браузера не працюватиме (як у egui, Figma) |
| B12 | **Доступність (скрінрідери, EAA)** | **немає**: AccessKit web — "planned"; AOM virtual nodes — не відвантажено; ariaNotify — лише озвучення; html-in-canvas — Canary і сам є DOM | **дзеркальний DOM за запитом**: 1 постійна прихована кнопка "увімкнути доступність" (патерн Flutter) + ≈3 вузли/картку при увімкненні (меню ≈ 500). Без нього — не WCAG 2.1 AA, отже не EN 301 549 / EAA для не-мікро закладів | **не розв'язно без DOM**; мінімум 1 елемент постійно, N на вимогу |
| B13 | **Камера/QR, завантаження фото, експорт файлів** | Chromium: `MediaStreamTrackProcessor`/`ImageCapture`, `showOpenFilePicker` — 0; експорт — відповідь Worker з `Content-Disposition` — 0 | Safari/Firefox: 1 тимчасовий `<video>` для декодування потоку, 1 тимчасовий `<input type=file>` | розв'язно; тимчасово 1 елемент поза Chromium |
| B14 | **Telegram-віджет логіну (admin)** | редирект на oauth.telegram.org | 0 (сьогодні віджет = iframe) | розв'язно |
| B15 | **Гейти** | див. §5.3 — кожен має заміну на дереві сцени | — | розв'язно, ≈5–8 лейн-днів |

**Підсумок B:** ціль "1 `<canvas>` у body" досяжна на всіх поверхнях у Chromium; у Safari/Firefox додається **1 тимчасовий елемент під час введення тексту/камери/файлу**; для скрінрідерів — **1 постійний + N за запитом**. Це і є чесний нередукований мінімум.

## 4. Частина C — прототипи й виміри (FRESH)

### 4.1 Що збудовано (`canvas-lab/`, лише scratchpad)

- **menucanvas** — `#![no_std]`, нуль залежностей, cdylib; ручний ABI з 8 імпортів (`c2d_rect`, `c2d_text`, `c2d_clip/unclip`, `txt_measure`, `gl_raster`, `gl_frame`, `host_log`) і 9 експортів (`init/resize/frame/pointer/wheel/set_lang/stats/…`). Rust: дані 165 страв (згенеровано з `design/dubin-sushi-menu.json`, 59 страв × 3), верстка карток (1 або 2 колонки), перенос слів за виміряною шириною, еліпсис, прокрутка з інерцією, хіт-тест кнопки "+", кошик, перемикач uk/sq. Два бекенди: **mode 0 = Canvas2D** (браузер формує й растеризує текст системними шрифтами); **mode 1 = WebGL2** — Rust будує буфер вершин (заокруглені прямокутники — тесельовані віяла, градієнти — пер-вершинні кольори), текст — **атлас слів, растеризованих хостом на OffscreenCanvas 2D** (системні шрифти, без завантаження шрифтів, без DOM), 1 програма, 1 draw call. Лоадер `web/loader.js` — єдиний JS. Збірка: `cargo +1.96.1 build --release --target wasm32-unknown-unknown` **1,17 s**; `wasm-opt -Oz` з явними прапорцями фіч (`--all-features` у binaryen 133 видає кодування, яке V8 відкидає: "Invalid import kind 127").
- **eguimenu** — той самий екран на eframe 0.33.3 (glow + default_fonts, wasm-bindgen =0.2.129); холодна збірка через slot.sh **2 m 05 s**.
- Стенд: Playwright + chromium_headless_shell (Chromium 153.0.8010.12), 390×844, isMobile, програмний растр на ядрах 4–6 (A78). Файли результатів: `canvas-lab/menucanvas/web/m{0,1}d{1,2,3}.json`, `shot-mode0-dpr{1,2,3}.png`, `canvas-lab/diag.out`.

### 4.2 Розмір (байти; `gzip -9`, `brotli -q 11`)

| Артефакт | raw | gzip | brotli |
|---|---|---|---|
| **menucanvas.wasm** (після wasm-opt -Oz) | 25 971 | **9 087** | 7 893 |
| loader.js (ручний, читабельний) | 6 338 | 2 566 | 2 269 |
| loader.min.js (esbuild 0.28.1) | 4 386 | **2 050** | 1 844 |
| index.html (1 canvas + 1 script) | 681 | 479 | 309 |
| **Разом на дроті (zero-DOM меню, 165 страв у wasm)** | 32 950 | **≈ 12 100** | ≈ 10 500 |
| eguimenu_opt.wasm (eframe glow, -Oz) | 2 189 883 | 1 140 710 | 987 263 |
| eguimenu.js glue (bindgen) / мініфікований | 69 910 / 32 664 | 10 622 / 8 688 | 8 940 / 7 425 |
| Сьогоднішній шелл вітрини (59 модулів) + CSS | 557 329 | **197 279** | 167 160 |
| RECORDED: Leptos CSR меню (2026-10-01 §5.2) | 137 985 + glue | 60 818 + 5 700 | — |
| RECORDED: Flutter CanvasKit / Skwasm (startdebugging.net 2026-09) | — | — | 1 540 000–2 260 000 / 1 210 000–1 860 000 |

### 4.3 Час (headless Chromium 153, програмний растр; ms)

| Випадок | fetch+compile+instantiate | перший кадр від старту лоадера | CPU 1-го кадру (холодні виміри тексту) | прокрутка 120 кадрів: p50 / p90 / p99 / max (CPU у `frame`) | drag p50 | перемикання мови (1-й / 2-й кадр) | пам'ять wasm / JS-купа |
|---|---|---|---|---|---|---|---|
| Canvas2D dpr 1 | 47,6 | **105,2** | 54,7 | 3,1 / 3,9 / 15,8 / 21,3 | 2,5 | 4,1 / 2,0 | 2,75 MB / 10 MB |
| Canvas2D dpr 2 | 86,5 | **134,5** | 42,8 | **1,7 / 2,1 / 4,9 / 7,2** | 1,3 | 2,5 / 1,7 | 2,75 MB / 10 MB |
| Canvas2D dpr 3 | 41,0 | **81,0** | 34,2 | 1,7 / 2,3 / 3,5 / 5,9 | 1,4 | 2,5 / 1,5 | 2,75 MB / 14,3 MB |
| WebGL2 dpr 1 (ANGLE/SwiftShader; буфер порожній, контекст "lost") | 80,0 | 134,5 | 46,6 | **0,6 / 1,2 / 1,8 / 2,0** (лише CPU: 4 566 вершин/кадр) | 0,5 | 3,3 / 0,3 | 2,75 MB / 10 MB |
| WebGL2 dpr 2 | 97,9 | 165,8 | 59,4 | 0,6 / 1,3 / 2,6 / 2,8 | 0,5 | 4,2 / 0,3 | атлас: полиці 294 px з 2 048 |
| egui dpr 1 (`__ready`) | — | 413 (до готовності eframe) | — | rAF-каденція p50 16,7 / max 216 | — | — | — |
| Node 22 `WebAssembly.compile`+instantiate, медіана з 7 | menucanvas **3,92**; egui 20,17 | | | | | | |

Піксельний доказ: Canvas2D dpr 1 — 283 614 з 329 160 пікселів не кольору паперу (намальовано; скріншоти `shot-mode0-dpr{1,2,3}.png` показують картки, uk/sq-текст, смужки, чіпи, навігацію); WebGL — `readPixels` = нулі, `isContextLost() = true` на всіх трьох наборах прапорців (`--use-gl=swiftshader`, `angle/swiftshader`, без прапорців) — **GPU-сторона на боксі невимірювана**, як і записано в пам'яті.

Каденція кадрів тим самим зондом (`cadence.mjs`: 1 wheel 40 px на rAF, 120 кадрів, dpr 2; що відчуває користувач — верстка+малювання+растр разом):

| Сторінка | p50 | p90 | p99 | max | кадрів > 33 ms | елементів у body |
|---|---|---|---|---|---|---|
| **Сьогоднішня вітрина, живий qa-durres** | **33,3** | 50,0 | 83,3 | 83,4 | **107 / 120** | 200 |
| **menucanvas Canvas2D** | **16,7** | 16,8 | 66,7 | 150,1 | **8 / 120** (1 long task 134 ms) | 2 (canvas + script; script іде в head у цілі) |
| menucanvas WebGL2 (порожній буфер, CPU-частина) | 16,7 | 16,7 | 16,8 | 16,8 | 0 / 120 | 2 |
| eguimenu (WebGL2, порожній буфер) | 16,7 | 16,7 | 16,8 | 16,8 | 0 / 120 | 3 (canvas, script, прихований input від eframe) |

Caveat: 16,7 ms = vsync headless (60 Hz), тобто прототипи встигають у кожен кадр; 8 довгих кадрів Canvas2D за 120 — один GC/растр-стрибок (long task 134 ms), WebGL/egui рівні, але їхній буфер тут порожній. Застереження до каденції: стенд без GPU — HTML-сторінка тут позбавлена GPU-композитора, який на телефоні робить прокрутку плавною без JS; число показує вартість верстки/растру 200 елементів на програмному растрі, а не те, що гість побачить на телефоні. Порівняння чесне лише між прототипами.

### 4.4 Якість тексту й що прототип НЕ показує

- Canvas2D: текст формує й растеризує браузер системним шрифтом (`system-ui`) — та сама якість, що в HTML, без завантаження шрифтів; uk (кирилиця) і sq (ë, ç) намальовані коректно (скріншоти). Кернінг у реченні — через перенос по словах (сума ширин слів), різниця візуально нульова.
- WebGL2: слова растеризуються на OffscreenCanvas у dpr-масштабі й малюються текстурованими квадратами; при цілочисельних позиціях ідентично Canvas2D, при дробових — легке розмиття (LINEAR). Атлас RGBA 2048² = 16 MB; при dpr 3 і двох мовах полиці займуть ≈ 900 px — запас є, але R8-атлас і кеш по гліфах (потрібна власна шейпінг-бібліотека: rustybuzz ≈ 300–500 KB wasm) — наступний крок, якщо WebGL стане базовим.
- Не виміряно тут: GPU-час кадру, перший кадр на реальному телефоні, батарея, поведінка при втраті контексту на Android, якість тексту WebGL-шляху на око, IME на iOS, прокрутка пальцем на екрані. Усе це — гейт на телефоні ([[live-proof-rule]]).

## 5. Частина D (за поправкою) — план нуль-DOM по поверхнях

### 5.1 Архітектура (одна на всі поверхні)

```
index.html  = <head> (meta, manifest, OG, 1 <script type=module>)  +  <body><canvas></body>
loader.js   = 2 KB: instantiate, розмір/ dpr, події → wasm, Canvas2D або WebGL2 бекенд, атлас, втрата контексту
app.wasm    = Rust: дані (блоки хаба через bebop-wasm), стан, верстка, віджети, текст, хіт-тест, сцена → команди
sw.js       = 3 файли шелу (замість 59 модулів)
crawl       = Worker: HTML-меню для краулерів (той самий Rust-рендер даних; C3)
a11y        = дзеркальний DOM за запитом (1 кнопка постійно)
```

Один рушій сцени, два бекенди: **Canvas2D — базовий** (працює всюди, вимірюваний гейтами на боксі, системні шрифти, 1,7 ms/кадр), **WebGL2 — для Sea, графіків аналітики, карти та як режим на сильних телефонах** (той самий текстовий конвеєр). Суто WebGL-збірка допустима, але втрачає очі headless-гейта на цьому боксі й вимагає телефон для кожної перевірки.

### 5.2 Поверхня за поверхнею (ESTIMATE лейн-днів; порядок = рекомендований)

| # | Поверхня | Що дає | Що коштує / ризик | Нередукований DOM | Лейн-дні |
|---|---|---|---|---|---|
| 0 | **Рушій + віджети** (текстове поле з виділенням/IME/EditContext, список з віртуалізацією, аркуш, тост, форма, таби, графік, растрова карта, камера, чат) + гейт "body = 1" + тест втрати контексту | основа всього | найбільший ризик: текстове поле на iOS | — | 15–25 |
| 1 | **room / kitchen board** (персонал) | доказ на живому хабі; без платежів, SEO, споживчого EAA, майже без тексту | tour-якорі → id вузлів | 1 | 6–10 |
| 2 | **courier** | карта на растрових тайлах, маршрут, камера для QR | провайдер тайлів; `<video>` на iOS під час сканування | 1 (+1 тимч. iOS) | 8–12 |
| 3 | **store: меню, кошик, трекінг** | 12 KB замість 197 KB; офлайн-шелл з 3 файлів | дзеркальний DOM для AT; HTML для краулерів | 1 (+1 a11y-кнопка; +N за запитом) | 15–20 |
| 4 | **store: checkout** | Stripe Checkout редирект; телефон/адреса — WebOTP/Contact Picker на Android, `<input>` тимчасово на iOS | втрата оплати в аркуші | 1 (+1 тимч.) | 4–6 |
| 5 | **admin (консоль)** 14 039 рядків, найбільше форм | єдиний інтерфейс | форми/таблиці/завантаження фото/експорти; найдорожча | 1 (+1 тимч.) | 30–40 |
| 6 | **platform + landing + learn** (254 елементи, фільм, waitlist) | — | лендінг — SEO-критичний: краулерний HTML обов'язковий | 1 | 5–8 |
| 7 | **гейти** (§5.3) | — | — | — | 5–8 |
| | **Разом** | | | | **≈ 90–130** |

Що **не має зламатися** на кожному кроці: платежі (live-proof на qa-durres, Stripe test), тур learn (109 якорів → id вузлів, гейт learn рахує їх у сцені), офлайн (sw-shell = 3 файли), no-tracking (без змін), ru/uk/sq/en (гейт langs стає `match` у Rust — компілятор вимагає всі чотири), push (без змін — SW), встановлення PWA (manifest у head без змін).

### 5.3 Pass marks (вимірювані)

| Гейт | Сьогодні | Заміна в нуль-DOM | Поріг |
|---|---|---|---|
| **dom-count (новий)** | — | headless на кожній поверхні у спокої: `document.body.querySelectorAll('*').length` | **= 1**; тимчасовий елемент дозволений лише поки поле у фокусі (тест фокусує й знімає фокус: знову 1) |
| wire budget (новий) | шелл 155 KB gz | wasm + loader + html gzip | ≤ 60 KB gz на поверхню (прототип 12 KB; запас на віджети/дані) |
| first frame (новий) | — | headless dpr 2, програмний растр | ≤ 300 ms (прототип 135) |
| frame CPU (новий) | — | 120 wheel-кадрів, p99 CPU у `frame` | ≤ 8 ms (прототип 4,9) |
| context-loss (новий) | — | `WEBGL_lose_context.loseContext()` → `restoreContext()` → піксельний diff кадру | ідентичний кадр |
| design gate (HTML+CSS) | рахує правила/контраст | тест дерева сцени в `cargo test`: контраст тексту ≥ 4,5, відступи, шкала типографіки | без регресу |
| tap-size | DOM-прямокутники ≥ 44 px | прямокутники хіт-тесту в сцені ≥ 44 px | 0 порушень |
| langs | рядкові таблиці JS | вичерпний `match` по `Lang` у Rust + тест, що кожен рядок має 4 мови | компіляція |
| learn | `data-tour` у HTML | id вузлів сцени; тур малює Rust | 109 якорів знайдено |
| ui-reach / ui-adoption | grep маршрутів у JS | grep маршрутів у Rust UI | без регресу |
| sw-shell | 59 модулів | 3 файли | список = збірка |
| icons | CSS-маски data-URL | SVG-шляхи → `Path2D` (Canvas2D) / растр в атлас (WebGL) | кожна іконка малює ≥ 1 піксель |
| a11y (новий) | — | з увімкненим дзеркальним DOM: axe-core 0 critical; скрінрідер-прохід на телефоні | 0 critical |
| phone (новий, ручний) | — | один реальний Android ≤ 2 GB RAM + один iPhone: перший кадр, прокрутка, IME, оплата | ≤ 1 s перший кадр на 3G |

### 5.4 Найменший перший лейн — W-CANVAS1 (6–10 лейн-днів)

Перенести `menucanvas` у `crates/dowiz-canvas` (Canvas2D-бекенд, WebGL2 за прапорцем `?gl=1`), виростити з нього **room/kitchen board** на живих даних хаба (блоки через `crates/bebop-wasm`, сокет статусів), відвантажити на qa-durres поруч зі старою сторінкою (`/room/canvas/`), додати гейти dom-count, first-frame, frame-CPU, context-loss, перенести тур-якорі room у id вузлів; підтвердити на одному Android-телефоні ([[live-proof-rule]]). Вихід лейна: виміряна таблиця §4.3 для room на телефоні + рішення, чи WebGL2 стає базовим.

## 6. Чого не вдалося виміряти тут (і що це означає)

1. **GPU-кадр WebGL2** — бокс малює порожнечу; маємо лише CPU-частину (0,6 ms). На телефоні треба виміряти `requestAnimationFrame`-каденцію та `EXT_disjoint_timer_query_webgl2`.
2. **Телефон**: перший кадр на 3G, батарея, пам'ять, IME iOS, pinch — не вимірювались. Числа §4.3 — програмний растр на A78, відносні.
3. **Якість тексту WebGL-шляху на око** — лише Canvas2D-скріншоти.
4. **SEO-поведінка** краулерного HTML — не тестувалась (немає індексації QA-хабу).
5. **EAA** — обсяг звільнень і хто є "постачальником послуги" (заклад чи платформа) — питання для юриста, не цього звіту.
6. Каденція сьогоднішньої вітрини (33 ms) — на стенді без GPU, тобто завищена відносно телефона.

## 7. Джерела, на які спирався найбільше

- EditContext: caniuse.com/wf-edit-context; developer.mozilla.org/en-US/docs/Web/API/EditContext_API; blogs.windows.com/msedgedev/2024/02/13/custom-web-editing-experiences-with-editcontext/
- Flutter web a11y: docs.flutter.dev/ui/accessibility/web-accessibility; розміри рендерерів: startdebugging.net/2026/09/canvaskit-vs-skwasm-for-flutter-web-in-2026/; docs.flutter.dev/platform-integration/web/renderers
- Figma: figma.com/blog/building-accessibility-into-a-canvas-based-product/; Google Docs: workspaceupdates.googleblog.com/2021/05/Google-Docs-Canvas-Based-Rendering-Update.html
- AccessKit README: github.com/AccessKit/accesskit; egui/eframe docs: docs.rs/eframe; ariaNotify: groups.google.com/a/chromium.org/g/blink-dev/c/QCtWzIPgcCY (Intent to Ship); AOM: wicg.github.io/aom/spec
- html-in-canvas: github.com/WICG/html-in-canvas
- EAA: ccpc.ie/business/enforcement/accessibility/european-accessibility-act-guidelines-for-microenterprises/; e-include.eu/web-accessibility/european-accessibility-act/
- Google dynamic rendering: searchengineland.com/google-no-longer-recommends-using-dynamic-rendering-for-google-search-387054; developers.google.com/search/docs/crawling-indexing/javascript/dynamic-rendering
- Stripe Checkout: docs.stripe.com/payments/checkout
- maplibre-rs: maplibre.org/maplibre-rs/docs/book/; OSM tile policy: operations.osmfoundation.org/policies/tiles/
- WebGPU Android: developer.chrome.com/blog/new-in-webgpu-121; Chrome 146 compat mode: app.cinevva.com/news/2026-03-10-chrome-146-webgpu-compatibility
- Android-версії: gs.statcounter.com/android-version-market-share/mobile/albania, …/ukraine
- Context loss: developer.mozilla.org/en-US/docs/Web/API/HTMLCanvasElement/webglcontextlost_event
- OffscreenCanvas/roundRect/Local Font Access: caniuse.com/offscreencanvas; caniuse.com/wf-canvas-roundrect; developer.mozilla.org/en-US/docs/Web/API/Local_Font_Access_API
- wasm ESM/source-phase imports: github.com/tc39/proposal-source-phase-imports; developer.mozilla.org/docs/Web/JavaScript/Reference/Operators/import/source
- Попереднє дослідження: docs/research/2026-10-01-cloudflare-free-cost-and-rust-web.md §5 (Leptos: 137 985 raw / 60 818 gz)

## 8. Відкриті питання оператору

1. **Скрінрідери:** чи приймаємо "дзеркальний DOM за запитом" (1 прихована кнопка постійно + вузли лише коли увімкнено)? Без нього нуль-DOM вітрина не відповідає WCAG/EAA для закладів, що не є мікропідприємствами.
2. **Safari/Firefox:** 1 тимчасовий `<input>` поки гість вводить текст (IME + автозаповнення) — єдиний нередукований елемент; приймаємо, чи відмовляємось від автозаповнення на iOS?
3. **Stripe:** редирект на hosted Checkout (0 DOM у нас) замість оплати всередині аркуша — ок?
4. **Карта:** растрові тайли (платний провайдер або власний кеш у R2) замість векторного MapLibre — ок, і який бюджет на тайли?
5. **SEO меню:** окрема HTML-відповідь для краулерів (за Google — застарілий обхідний шлях) чи відмова від індексації меню (QR-трафік)? Лендінг dowiz.org у будь-якому разі потребує краулерного HTML.
6. **Базовий бекенд:** Canvas2D як базовий + WebGL2 для ефектів/карти/графіків (рекомендація), чи суто WebGL2 (тоді кожна перевірка — на телефоні)?
7. **Порядок:** перший лейн — room/kitchen board; підтвердити, і чи запускати W-CANVAS1 зараз (3 лейни вже зайняті).
8. **Бюджет:** 90–130 лейн-днів на всі поверхні — підтвердити старт або обмежити вітриною + персоналом (≈ 50–70).
