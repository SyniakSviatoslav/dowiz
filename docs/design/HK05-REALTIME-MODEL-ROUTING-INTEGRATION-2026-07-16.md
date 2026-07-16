# HK-05/HK-09 — реальний стан і інтеграція real-time model-tier routing (2026-07-16)

> **Важлива примітка перед усім іншим:** три скріншоти, які ви описали ("Tiered Swarm
> Architecture", "Industrial AI Pipeline", "Concurrency Scaling metrics") — це **не
> зовнішня індустріальна концепція**. Це буквально екрани вже наявних документів
> цього ж репозиторію: `docs/design/SWARM-QUANT-BLUEPRINT-2026-07-15.md` +
> `docs/design/hermes-kernel-rewrite-2026-07-15/{REWRITE-PLAN,BLUEPRINTS}.md`.
> Цифри збігаються дослівно: **3.93× @N=4, 7.51× @N=8** — той самий benchmark
> (`tools/telemetry/swarm_proof.py`, реально існує й запускається). Хтось (інша модель,
> що рецензувала скріншоти) описав вам назад вашу ж власну систему, не впізнавши її.
> Це не применшує якість опису — Architect/Executor/Verifier, Router/Planner/Worker/
> Evaluator, HK-05 dynamic-complexity-routing — усе точно назване. Але "інтеграція
> цього в openbebop/dowiz" — неправильна постановка: воно вже **живе** в
> `tools/telemetry/` цього репо і в окремому крейті `hermes-kernel`. Реальне питання —
> **що з цього справді wired, а що лише скомпільовано й чекає на дріт**, і саме на це
> нижче — перевірена, не здогадана, відповідь.

---

## 1. Де що фізично лежить (3 репо, не 2)

Досі в розмові фігурували два репо — `/root/dowiz` і `/root/bebop-repo` (openbebop).
Цей HK-05/HK-09 апарат живе в **третьому**: `/root/hermes-agent-kernel-rewrite/
hermes-kernel/` — окремий Rust-крейт, що є "kernel" для **самого агентного
інструментарію** (Hermes), не для продукту DeliveryOS. `dowiz/tools/telemetry/
governance.sh` — це тонкий bash-міст (`gov_kern`), що пайпить JSON у скомпільований
бінарник `hermes-kernel` і читає JSON назад. Сам REWRITE-PLAN.md це прямо визнає:
"Principles and component mapping reuse patterns already proven... in this same
operator's own **openbebop and dowiz** codebases" — тобто причинність зворотна до
тієї, що напрошується зі скріншотів: патерн (kernel=чистий/детермінований,
harmonic-centrality) **народився в dowiz/openbebop і був перенесений в Hermes**, а
не навпаки.

## 2. Що реально ПОБУДОВАНО і ПРОТЕСТОВАНО (перевірено читанням коду, не документів)

| Компонент | Файл | Статус |
|---|---|---|
| `harmonic_centrality(n, edges)` | `dowiz/kernel/src/harmonic.rs` | ✅ Побудовано, 10+ тестів, wasm-wired (`wasm.rs::harmonic_centrality_js`), власний коментар прямо каже "uses for HK-05/HK-06 model routing" |
| `TaskFeatures`, `Complexity{Simple,Moderate,Complex}`, `classify_complexity()` | `hermes-kernel/kernel/src/routing.rs` | ✅ Побудовано — дешева евристика над сигналами препромпту (довжина, tool-chain, scope) |
| `rank_models_for_bucket(bucket, history, available)` | `hermes-kernel/kernel/src/routing.rs` | ✅ Побудовано — **буквально викликає `harmonic_centrality`**: будує граф root→model-вузли з кількістю ребер ∝ success-rate моделі в цьому complexity-bucket, ранжує за centrality |
| `ev`, `ev_wave`, `kelly_fraction`, `ruin_prob`, `ev_route_select`, `lane_size`, `pid_parallelism`, `Recalibrator`, `Budget`, `jury_aggregate` | `hermes-kernel/kernel/src/control.rs` | ✅ Побудовано, 13 тестів |
| CLI-диспетчер: `op_classify_complexity`, `op_rank_models`, `op_gov_route`, `op_gov_lane`, `op_gov_decide`, `op_gov_precedent` | `hermes-kernel/cli/src/main.rs` | ✅ Побудовано — усі оп'ять диспетчеризуються, бінарник реально це вміє |
| `swarm_proof.py` (3.93×/7.51× benchmark) | `dowiz/tools/telemetry/swarm_proof.py` | ✅ Реальний, запускний скрипт — цифри не вигадані |
| `Orchestrator::dispatch` (DRAFT→CERTIFIED gate для loop-карток) | `dowiz/kernel/src/loops.rs` | ✅ Побудовано й протестовано (BP-20) — але це оркестрація **build/verify-петель**, не LLM-tier dispatch; суміжний, не той самий примітив, що Router/Planner/Worker/Evaluator зі скріна 2 |

## 3. Що НЕ wired — конкретний, фальсифікований розрив

`gov_route()` у `governance.sh` (реальний, живий шлях, яким сьогодні користуються
агентні сесії) фолдить `track_record.jsonl` **лише за `task_type`**, будує
`(model, p, v, cost)` через python-евристику й передає в `op_gov_route`. Він:

- **НЕ викликає `op_classify_complexity`** — вхідна задача ніколи не класифікується
  за складністю перед routing-рішенням.
- **НЕ фолдить track-record за `(complexity_bucket, model)`** — тобто
  `rank_models_for_bucket`, попри те що повністю побудований і використовує
  harmonic-centrality, **ніколи не викликається з живого коду**.
- **НЕ викликає `op_gov_lane`/`lane_size`/`pid_parallelism`** — кількість
  паралельних executor-лейнів (N=4, N=8 у benchmark) сьогодні **фіксована руками**,
  не підлаштовується динамічно під arrival-rate/service-time telemetry, яку
  `tools/telemetry/lib.sh::resource_sample()`/`bench_run()` вже збирають.

Тобто: **весь обчислювальний двигун для "автоматичного real-time балансування між
рівнями моделей" уже написаний, протестований і компілюється в бінарник — бракує
рівно одного кроку: `governance.sh` має його викликати.** Це не дослідницьке
питання, це wiring-задача з чіткою межею.

## 4. Пряма відповідь на ваше питання

> *"Як ви плануєте використовувати ці естіймейти (з 29304) для автоматичного
> балансування навантаження між різними рівнями моделей у реальному часі?"*

Механізм уже спроєктований точно так, як питання й передбачає — потрібно лише
з'єднати три вже готові шматки в `governance.sh`:

1. **На вході кожної задачі** — викликати `op_classify_complexity` над
   `TaskFeatures` (довжина повідомлення, передбачена глибина tool-chain,
   присутність математики/багатофайлового скоупу) → отримати `Complexity`
   bucket. Дешево, без виклику моделі (feature extraction над уже складеним
   промптом).
2. **Ранжування моделей для цього bucket** — фолдити `track_record.jsonl` за
   `(bucket, model)` замість поточного лише-`task_type`, викликати
   `op_rank_models` (`rank_models_for_bucket` → harmonic centrality над
   success-rate-графом) → впорядкований список кандидатів **для цього конкретного
   поєднання складність×модель**, не глобальний leaderboard.
3. **EV-гейт + розмір лейну в реальному часі** — top-ranked модель іде через уже
   робочий `op_gov_route` (EV = p·v−(1−p)·c, reject якщо ruin>cap), а
   `op_gov_lane`/`lane_size(arrival_rate, service_time, u_target)` +
   `pid_parallelism(error, kp)` визначають **скільки паралельних executor-лейнів
   відкрити прямо зараз**, підживлюючись живою arrival-rate/service-time
   телеметрією з `resource_sample()` — це і є "реальний час": лейн ширшає, коли
   черга зростає й executor встигає, звужується під ruin-cap коли ні.

Результат — той самий фіксований N=4/N=8 benchmark стає **адаптивним**: bucket
"simple" отримує вузький дешевий лейн з дешевою моделлю, bucket "complex" —
ширший лейн з дорожчим architect-executor, і ширина лейну сама дихає разом з
навантаженням, а не задається константою в коді запуску.

**Конкретний наступний крок** (не дослідження — implementation, невеликий):
розширити `gov_route()`/`gov_lane()` у `governance.sh` трьома викликами вище,
писати новий стовпець `bucket` у `track_record.jsonl` (зворотно сумісно —
відсутність поля = `Simple` за замовчуванням), і підтвердити RED→GREEN: та сама
задача, класифікована як `Complex`, повинна отримати інший (ширший/дорожчий)
маршрут, ніж класифікована як `Simple` — це фальсифіковний тест, який або
проходить, або ні.

## 5. Чи це стосується самого продукту dowiz/openbebop?

Чесно: **сьогодні ні, і форсувати зв'язок було б тим самим overclaiming'ом, який
цей проєкт свідомо відкидає** (та сама дисципліна, що відхилила штучний
TimesFM↔Gaussian-Splatting зв'язок у попередньому дослідженні). HK-05/HK-09 —
це **інструментарій розробки** (routing рішень для агентних сесій, що працюють
над кодом dowiz/openbebop), не delivery-платформна фіча. Продукт dowiz сьогодні
не запускає власних LLM-агентів у проді, яким потрібен би був цей роутер.

Єдиний реальний, не притягнутий місток: `bebop2` вже має `bebop mcp` (JSON-RPC
сервер, що виставляє bebop-можливості як MCP tools) і `ActionContract`/
`ChannelAdapter` capability-scoped порти. Якби dowiz колись **запустив власну
AI-агентну фічу в проді** (наприклад, AI-асистент для owner-панелі чи
підтримки), EV-driven tiered routing (Architect/Executor/Verifier + harmonic-
centrality ranking) — це вже готовий, перевірений патерн для вибору моделі
під капотом **того ж MCP-порту**, а не нова архітектура. Але це умовний,
майбутній хук, не сьогоднішня інтеграційна робота — вписувати його в roadmap
зараз без реальної AI-фічі в продукті було б передчасним скоупом.

---

**Джерела (перевірено читанням коду, не документів):** `dowiz/kernel/src/harmonic.rs`,
`dowiz/kernel/src/wasm.rs:710-720`, `dowiz/kernel/src/loops.rs` (BP-20),
`dowiz/tools/telemetry/{governance.sh,swarm_proof.py,lib.sh}`,
`hermes-kernel/kernel/src/{routing,control}.rs`, `hermes-kernel/cli/src/main.rs`,
`dowiz/docs/design/{SWARM-QUANT-BLUEPRINT-2026-07-15,SWARM-GOVERNANCE-DESIGN,
CONCURRENCY-ANALYSIS-2026-07-11}.md`, `dowiz/docs/design/hermes-kernel-rewrite-
2026-07-15/{REWRITE-PLAN,BLUEPRINTS,AUDIT}.md`.
