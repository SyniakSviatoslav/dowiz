# DeliveryOS — MVP-доповнення: сенсорна шина, ручні мости та шви під North Star

> Це вхідний бриф для Тріадної Ради (design-time). North Star (P1–P7 автопілот) — ціль під ріст, НЕ предмет цього документа. Тут — рівно те, що додаємо на MVP-етапі: покращує ручний сервіс сьогодні + закладає сенсори/шви під майбутню автономію + не потребує обсягу даних, якого MVP ще не має.

## 0. Принципи етапу (контракт, не стиль)
1. observe-don't-control на незводимому (готування/дорога недетерміновані → спостерігати інтервал, не контролювати). Контроль лише де спільний ресурс — кухонна черга.
2. Cold-start: евристика спершу, дані потім. ~30 замовлень/день. Кожне число осмислене в день 1 з нульовою історією, тихо затягується.
3. Поппер-інваріант: жодної метрики без критерію фальсифікації (§8).
4. range-never-point до клієнта: внутрішньо точне, клієнту завжди інтервал (навіть «1–2 хв»).
5. Агенти декларують приватно (kitchen prep, courier timing) — система синтезує ОДНЕ клієнтське число.
6. Людина вирішує консеквентне (скасування/повернення/переофер/тавро) — лише сигнал+флаг, жодних авто-покарань.
7. North Star ≠ now-build: не будувати калібрувальні петлі / авто-рішення / рекурсивний BOM-рантайм / батч-оптимізацію зараз — закладати під них ШВИ.

## 1. TIER 1 — Сенсорна шина (негайно; покращує ручний сервіс, незакривано ретроактивно)
- 1.1 Логування timestamps подій у append-only order_status_history (confirmed_at, courier_geofence_enter[раз/завдання], picked_up, delivered_at, опц. geofence_enter_customer) + збереження показаного клієнту promised_window на ордері (історична істина, незмінна). Acceptance: кожен перехід → рядок з ts; promised_window_lo/hi записані при confirm і не міняються; geofence_enter рівно раз; відновлювані тривалості prep/road/dwell.
- 1.2 Нормалізований baseline доставки: на delivered_at рахувати route_distance_m + expected_delivery_min (road-distance × швидкість, БЕЗ запуску роутера) у delivery_trace; аналітика кур'єра нормалізована (факт/baseline), не сира → не карати за важкий маршрут. Виправляє зміщення виміру під autonomy-тест І кур'єр-вимір одним примітивом.
- 1.3 Воронко-інструментація: funnel_events (menu_view/add_to_cart/checkout_start/checkout_abandon) + показане ETA-вікно на момент події; анонімно, session-scoped, нуль PII понад потрібне. Робить неспостережний контрфакт (втрачений клієнт через задовге ETA) спостережним.
- 1.4 Абсолютний кеп вікна: клієнтська обіцянка ніколи > locations.eta_cap_min; впирання сигналиться власнику (не тихо). Неспростовне зовнішнє гальмо padding-creep.

## 2. TIER 2 — Ручні мости (цінність зараз; флоу той самий, коли калібрування дозріє)
- 2.1 Ручний countdown готовності кур'єру: prep_time(агрегат позицій, дефолт max=паралельна кухня) → кур'єр бачить відлік + нудж «виїзди» коли countdown − travel_eta − margin ≤ 0; кур'єр володіє моментом виїзду (оверайд не блокується); dispatch_margin_min конфіг (хилить прибути раніше — простій дешевий, холодна їжа дорога).
- 2.2 BackgroundWarning + heartbeat-флаг кур'єра: превентивне попередження у фоні; heartbeat-gap > поріг → delivery_flags + сповіщення власнику; система НЕ авто-перепризначає (переофер = тап власника).
- 2.3 Денний чеклист доступності (MVP-форма P5): бінарний is_available + опц. products.stock_remaining (NULL=безлім; int=денний кап); вичерпано/непідтверджено = недоступно з причиною-хінтом; нуль регресії при NULL. Поведінкова прокладка до BOM; фінальна доступність пізніше = min(derived-з-інгредієнтів, ручний-кап).
- 2.4 Проактивний апдейт затримки + WISMO-трекінг (ручний P6, найбільший клієнт-ROI): клієнт-вікно (не точка) на оформленні під кепом; стадії схлопують вікно вручну (confirmed→cooking countdown; picked_up «їде ~Y»; GPS-близько «прибуває ~Z» — все одно діапазон); проактивне сповіщення коли зсув > material_shift_min (конфіг) з опцією зв'язку кур'єр/власник; жива карта лише в доставці; WS впав → статус-сторінка+телефон; збій (прибрана позиція) → авто-сповіщення з дією. Клієнт-паддінг = «авіаційна OTP»: вікно містить реальність otp_target_pct% (петля під вимір, cold-start = широке).

## 3. TIER 3 — Інваріанти та гігієна (ретро дорого; закладай зараз навіть ручними числами)
- 3.1 Агенти декларують приватно — система синтезує клієнтське число (анти-гейм до того як петлі дадуть кухні стимул роздувати ETA).
- 3.2 Атомарний декремент stock_remaining У ТІЙ САМІЙ транзакції ордера (status-guarded UPDATE + ідемпотентність + серверна ціна). SQL: UPDATE products SET stock_remaining = stock_remaining - $qty WHERE id=$pid AND (stock_remaining IS NULL OR stock_remaining >= $qty) RETURNING; 0 рядків → недоступно → відхилити рядок. Нема oversell на останній порції (тест гонки → рівно один успіх).
- 3.3 Шви БД зараз, рантайм потім (схема повна, код ні).

## 4. Поверхня атаки авто-confirm — мінімальний захист зараз (OTP ВІДКЛАДЕНО, шов)
1. Velocity-ліміти (phone+IP, конфіг пороги). 2. One-tap abort готування власнику (обрізає збиток до 1 порції). 3. no_show-репутація soft-gate на порозі (не блок, людино-оверайдний; customers.no_show_count є). Прийнятно без OTP: cash-on-delivery (нема платіж-шахрайства) + velocity + no_show + abort; найгірше = 1 змарнована порція; не повертати фрикшн усім.

## 5. Cold-start-пом'якшення
Кухонний-тип prep-пріор: дефолти prep_time за категорією (піца ~12–18хв) як baked-in доменний конфіг (НЕ крос-тенант пул — пресет); власник перекриває.

## 6. Консолідовані шви схеми (node-pg-migrate, forward-only, RLS ENABLE+FORCE у тій самій міграції, гроші integer)
### 6.1 NOW-рантайм:
- ALTER products ADD stock_remaining integer (NULL=безлім).
- ALTER orders ADD promised_window_lo_min int, promised_window_hi_min int.
- ALTER delivery_trace ADD route_distance_m int, expected_delivery_min int.
- ALTER locations ADD eta_cap_min int NOT NULL DEFAULT 90, dispatch_margin_min int DEFAULT 5, material_shift_min int DEFAULT 5, otp_target_pct int DEFAULT 90.
- CREATE TABLE funnel_events (id, location_id FK, session_ref text, event_type text, shown_eta_lo_min int, shown_eta_hi_min int, created_at) + index(location_id, created_at DESC) + RLS tenant_isolation+FORCE.
- order_status_history (append-only) уже закладено — несе всі §1.1 timestamps; якщо нема типу під courier_geofence_enter → додати в enum.
### 6.2 SEAM (закладаються зараз, рантайм FLAT/manual, активуються під North Star):
- CREATE TABLE ingredients (id, location_id, name, kind text DEFAULT 'raw' ['raw'|'intermediate'], is_batch_made bool, unit text, current_stock numeric, tracking_mode text DEFAULT 'untracked', waste_pct numeric DEFAULT 0, reset_cadence text, last_set_at, created_at) + RLS.
- CREATE TABLE recipe_components (id, location_id, parent_kind text ['product'|'ingredient'], parent_id uuid [POLYMORPHIC — products.id або ingredients.id], ingredient_id FK ingredients, qty_per_parent numeric, unit text, created_at) + index + RLS. Дерево-фаза відкладено (DAG без циклів, memoize, кап глибини).
- Батч-вузол MVP: intermediate з ручним current_stock; конкурентна коректність спільного інгредієнта = один вузол (атомарний guard на ньому; наївний flatten переплатив би під гонкою). Рецепти посилаються на ВУЗОЛ → апгрейд manual→derived міграційно-вільний.
- courier_sequence (уже в orders) — шов під батч-послідовність P3.

## 7. ПОЗА СКОУПОМ (свідомо НЕ будувати; за швом): калібрувальні петлі P7, батч-оптимізація P3, рекурсивна BOM-деривація, будь-яке авто-рішення (авто-призначення/батч/переофер/диспетч), smart backpressure (зараз ручний busy_mode ×2), demand-forecast, phone-OTP.

## 8. Фальсифікація під North Star (збирати зараз щоб тест був можливий)
- 8.1 Autonomy-ставка (роутабельне-проти-незводиме): з §1.2 розкласти похибку на роутабельну (vs road-distance нижня межа, не пояснена локальним знанням, БЕЗ роутера) і незводиму (трафік/паркінг/поріг). Засторога: ефективна автономія (об'їхав відомий затор) НЕ роутабельний розрив → нормалізувати на складність, не сирий час. Названі фальсифікатори заздалегідь.
- 8.2 Padding-creep гальмо: дуальна метрика — кожна надійність-метрика (BTR=promised_window vs факт) парується зі швидкісною контр-метрикою (кидання vs довжина вікна §1.3); авто-петлі рухають лише variability(σ), strategic(консервативність)=ручка власника.
- 8.3 Кур'єр-вимір: нормалізувати на road-distance baseline зони/години (§1.2); рейтинг = owner-advisory, ніколи авто-покарання.

## 9. Відкриті North-Star-питання (не розв'язуються зараз, не загубити): координація курсів усередині ордера (max(prep) холодить фрі); батч×готівкова експозиція (courier_cash_ledger); скасування після авто-confirm (cash нема рефанд-рейок); корельовані шоки (дощ-птн-20:00 крос-параметрно); refused-goods петля назад у інвентар/економіку; backpressure проти виручки.

## 10. Пріоритет
Ранжир: 1) §1.1 timestamps+promised_window (паливо P1/P2/P7, ретро не відновити). 2) §2.4 проактивний P6 (найбільший клієнт-ROI). 3) §1.2 нормалізований baseline (лагодить зміщення). 4) §2.3 чеклист. 5) §6 шви (найвища незворотність).
Послідовність: спершу §6 шви + §1.1 timestamps → §1.2/1.3/1.4 (виправити зміщення/контрфакт до брудних даних) → §2 ручні мости → паралельно §3/§4/§5.
DoD-гейт: кожен ордер повний timestamp-слід+promised_window; кожна доставка нормалізований baseline; воронка логується+кеп; кур'єр countdown+нудж+оверайд; heartbeat-флаг; чеклист+атомарний декремент без oversell; клієнт range-never-point+проактивний апдейт; velocity+abort+no_show; BOM-шви flat нуль регресії; нуль авто-рішень/петель/рекурсивного BOM.
